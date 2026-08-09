use std::time::{Duration, Instant};

use russh::{ChannelMsg, client};

use super::{SshClientHandler, SshShellHandle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ShellKind {
    Bash,
    Zsh,
    Fish,
    PosixSh,
    Unknown,
}

impl ShellKind {
    fn from_name(name: &str) -> Self {
        let value = name.to_ascii_lowercase();
        if value.contains("fish") {
            Self::Fish
        } else if value.contains("zsh") {
            Self::Zsh
        } else if value.contains("bash") {
            Self::Bash
        } else if value.contains("sh") {
            Self::PosixSh
        } else {
            Self::Unknown
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SshShellIntegrationPhase {
    Normal,
    WaitInitial,
    Suppressing,
}

pub(super) struct SshShellIntegrationState {
    phase: SshShellIntegrationPhase,
    pending_script: Option<Vec<u8>>,
    ready_marker: Vec<u8>,
    legacy_ready_marker: Option<Vec<u8>>,
    suppress_started_at: Option<Instant>,
    suppressed: Vec<u8>,
}

impl SshShellIntegrationState {
    pub(super) fn new(
        pending_script: Option<Vec<u8>>,
        ready_marker: Vec<u8>,
        legacy_ready_marker: Option<Vec<u8>>,
    ) -> Self {
        let phase = if pending_script.is_some() {
            SshShellIntegrationPhase::WaitInitial
        } else {
            SshShellIntegrationPhase::Normal
        };
        Self {
            phase,
            pending_script,
            ready_marker,
            legacy_ready_marker,
            suppress_started_at: None,
            suppressed: Vec::new(),
        }
    }

    pub(super) fn is_normal(&self) -> bool {
        self.phase == SshShellIntegrationPhase::Normal
    }

    pub(super) fn is_suppressing(&self) -> bool {
        self.phase == SshShellIntegrationPhase::Suppressing
    }

    pub(super) fn should_inject_on_initial_delay(&self) -> bool {
        self.phase == SshShellIntegrationPhase::WaitInitial && self.pending_script.is_some()
    }

    pub(super) async fn inject(&mut self, channel: &mut russh::Channel<client::Msg>) {
        let Some(script) = self.pending_script.take() else {
            return;
        };
        if channel.data_bytes(script).await.is_ok() {
            self.phase = SshShellIntegrationPhase::Suppressing;
            self.suppress_started_at = Some(Instant::now());
        } else {
            self.phase = SshShellIntegrationPhase::Normal;
            self.suppress_started_at = None;
        }
    }

    fn timeout_expired(&self) -> bool {
        self.phase == SshShellIntegrationPhase::Suppressing
            && self
                .suppress_started_at
                .is_some_and(|started_at| started_at.elapsed() > Duration::from_secs(30))
    }

    fn force_normal(&mut self) {
        self.phase = SshShellIntegrationPhase::Normal;
        self.suppress_started_at = None;
        self.pending_script = None;
        self.suppressed.clear();
    }

    pub(super) async fn filter_output(
        &mut self,
        bytes: &[u8],
        channel: &mut russh::Channel<client::Msg>,
    ) -> Vec<u8> {
        match self.phase {
            SshShellIntegrationPhase::Normal => strip_ssh_ready_markers(
                bytes,
                &self.ready_marker,
                self.legacy_ready_marker.as_deref(),
            ),
            SshShellIntegrationPhase::WaitInitial => {
                let visible = strip_ssh_ready_markers(
                    bytes,
                    &self.ready_marker,
                    self.legacy_ready_marker.as_deref(),
                );
                self.inject(channel).await;
                visible
            }
            SshShellIntegrationPhase::Suppressing => {
                if self.timeout_expired() {
                    self.force_normal();
                    return strip_ssh_ready_markers(
                        bytes,
                        &self.ready_marker,
                        self.legacy_ready_marker.as_deref(),
                    );
                }
                self.suppressed.extend_from_slice(bytes);
                if self.suppressed.len() > 64 * 1024 {
                    let keep_from = self.suppressed.len().saturating_sub(4096);
                    self.suppressed.drain(..keep_from);
                }
                if let Some(after_ready) = bytes_after_ssh_ready_marker(
                    &self.suppressed,
                    &self.ready_marker,
                    self.legacy_ready_marker.as_deref(),
                ) {
                    let after_ready = after_ready.to_vec();
                    self.force_normal();
                    strip_ssh_ready_markers(
                        &after_ready,
                        &self.ready_marker,
                        self.legacy_ready_marker.as_deref(),
                    )
                } else {
                    Vec::new()
                }
            }
        }
    }
}

pub(super) async fn detect_ssh_shell_type(
    handle: &SshShellHandle,
    timeout_ms: u64,
) -> Option<ShellKind> {
    let timeout_ms = timeout_ms.clamp(100, 60_000);
    let command = r#"printf '%s\n' "$SHELL"; ps -p $$ -o comm= 2>/dev/null || true"#;
    tokio::time::timeout(Duration::from_millis(timeout_ms), async {
        let channel = match handle {
            SshShellHandle::Dedicated(handle) => {
                open_ssh_exec_channel(handle, command).await.ok()?
            }
            SshShellHandle::Multiplexed(handle) => {
                let handle = handle.lock().await;
                open_ssh_exec_channel(&handle, command).await.ok()?
            }
        };
        let output = collect_ssh_exec_stdout(channel).await;
        let kind = ShellKind::from_name(output.trim());
        (kind != ShellKind::Unknown).then_some(kind)
    })
    .await
    .ok()
    .flatten()
}

async fn open_ssh_exec_channel(
    handle: &client::Handle<SshClientHandler>,
    command: &str,
) -> anyhow::Result<russh::Channel<client::Msg>> {
    let channel = handle.channel_open_session().await?;
    channel.exec(true, command.as_bytes().to_vec()).await?;
    Ok(channel)
}

async fn collect_ssh_exec_stdout(mut channel: russh::Channel<client::Msg>) -> String {
    let mut output = String::new();
    while let Some(message) = channel.wait().await {
        match message {
            ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
                output.push_str(&String::from_utf8_lossy(&data));
            }
            ChannelMsg::Close | ChannelMsg::Eof => break,
            _ => {}
        }
    }
    let _ = channel.close().await;
    output
}

pub(super) fn build_ssh_ready_marker(session_id: &str) -> String {
    format!("\x1b]7777;NyaTermReady:{session_id}\x07")
}

pub(super) fn build_legacy_ssh_ready_marker(ready_marker: &str) -> Option<String> {
    let inner = ready_marker.strip_prefix("\x1b]")?.strip_suffix('\x07')?;
    let session_id = inner.strip_prefix("7777;NyaTermReady:")?;
    Some(format!("\x1b]7777;DflyReady:{session_id}\x07"))
}

fn ssh_ready_printf(marker: &str) -> String {
    marker
        .replace('\\', "\\\\")
        .replace('\x1b', "\\033")
        .replace('\x07', "\\007")
        .replace('\'', "'\\''")
}

pub(super) fn ssh_shell_injection_script(shell: ShellKind, ready_marker: &str) -> Option<String> {
    let ready = ssh_ready_printf(ready_marker);
    match shell {
        ShellKind::Bash => Some(format!(
            concat!(
                " NYATERM_PRUNE_HISTORY=1;",
                " export NYATERM_INJ=1;",
                " NYATERM_LAST_HISTCMD=\"${{HISTCMD-}}\";",
                " __nyaterm_host(){{ hostname 2>/dev/null || printf localhost; }};",
                " __nyaterm_prune_history(){{",
                " [ -n \"${{NYATERM_PRUNE_HISTORY:-}}\" ] || return 0;",
                " unset NYATERM_PRUNE_HISTORY;",
                " local hline;",
                " hline=\"$(HISTTIMEFORMAT= history 1 2>/dev/null || true)\";",
                " case \"$hline\" in",
                " (*NYATERM_PRUNE_HISTORY*|*NYATERM_INJ*|*__nyaterm_prompt*|*NyaTermReady*)",
                " if [[ \"$hline\" =~ ^[[:space:]]*([0-9]+) ]]; then",
                " history -d \"${{BASH_REMATCH[1]}}\" 2>/dev/null || true;",
                " fi",
                " ;;",
                " esac;",
                " NYATERM_LAST_HISTCMD=\"${{HISTCMD-}}\";",
                " }};",
                " __nyaterm_emit_command(){{",
                " local histcmd=\"${{HISTCMD-}}\";",
                " if [ -n \"$histcmd\" ] && [ \"${{NYATERM_LAST_HISTCMD-}}\" != \"$histcmd\" ]; then",
                " NYATERM_LAST_HISTCMD=\"$histcmd\";",
                " local cmd; cmd=\"$(fc -ln -1 2>/dev/null)\";",
                " if [ -n \"$cmd\" ] && command -v base64 >/dev/null 2>&1; then",
                " local b64; b64=\"$(printf '%s' \"$cmd\" | base64 | tr -d '\\r\\n')\";",
                " printf '\\033]7777;NyaTermCommand:%s\\007' \"$b64\";",
                " fi;",
                " fi;",
                " }};",
                " __nyaterm_prompt(){{",
                " __nyaterm_prune_history;",
                " __nyaterm_emit_command;",
                " printf '\\033]7;file://%s%s\\007' \"$(__nyaterm_host)\" \"$PWD\";",
                " }};",
                " __nyaterm_install_prompt(){{",
                " local decl;",
                " decl=\"$(declare -p PROMPT_COMMAND 2>/dev/null || true)\";",
                " if [[ \"$decl\" =~ ^declare\\ -[^[:space:]]*a[^[:space:]]*\\ PROMPT_COMMAND= ]]; then",
                " local f;",
                " for f in \"${{PROMPT_COMMAND[@]}}\"; do",
                " [ \"$f\" = __nyaterm_prompt ] && return 0;",
                " done;",
                " PROMPT_COMMAND=(__nyaterm_prompt \"${{PROMPT_COMMAND[@]}}\");",
                " else",
                " case \"${{PROMPT_COMMAND-}}\" in (*__nyaterm_prompt*) ;; (*)",
                " PROMPT_COMMAND=\"__nyaterm_prompt${{PROMPT_COMMAND:+; $PROMPT_COMMAND}}\" ;; esac;",
                " fi;",
                " }};",
                " __nyaterm_install_prompt;",
                " printf '{}'\n",
            ),
            ready
        )),
        ShellKind::Zsh => Some(format!(
            concat!(
                " fc -p /dev/null 2>/dev/null\n",
                " export NYATERM_INJ=1;",
                " __nyaterm_host(){{ hostname 2>/dev/null || printf localhost; }};",
                " __nyaterm_emit(){{ printf '\\033]7;file://%s%s\\007' \"$(__nyaterm_host)\" \"$PWD\"; }};",
                " __nyaterm_preexec(){{",
                " if [ -n \"$1\" ] && command -v base64 >/dev/null 2>&1; then",
                " local b64; b64=\"$(printf '%s' \"$1\" | base64 | tr -d '\\r\\n')\";",
                " printf '\\033]7777;NyaTermCommand:%s\\007' \"$b64\";",
                " fi;",
                " }};",
                " autoload -Uz add-zsh-hook 2>/dev/null || true;",
                " typeset -ga precmd_functions preexec_functions;",
                " [[ \" ${{precmd_functions[*]}} \" == *\" __nyaterm_emit \"* ]] || precmd_functions+=(__nyaterm_emit);",
                " [[ \" ${{preexec_functions[*]}} \" == *\" __nyaterm_preexec \"* ]] || preexec_functions+=(__nyaterm_preexec);",
                " fc -P 2>/dev/null\n",
                " printf '{}'\n",
            ),
            ready
        )),
        ShellKind::Fish => Some(format!(
            concat!(
                " set fish_private_mode 1 2>/dev/null\n",
                " set -gx NYATERM_INJ 1;",
                " function __nyaterm_emit --on-event fish_prompt;",
                " printf '\\033]7;file://%s%s\\007' (hostname) $PWD;",
                " end;",
                " function __nyaterm_preexec --on-event fish_preexec;",
                " if test -n \"$argv[1]\"; and command -sq base64;",
                " set -l b64 (printf '%s' \"$argv[1]\" | base64 | tr -d '\\r\\n');",
                " if test -n \"$b64\";",
                " printf '\\033]7777;NyaTermCommand:%s\\007' \"$b64\";",
                " end;",
                " end;",
                " end;",
                " set -e fish_private_mode 2>/dev/null\n",
                " printf '{}'\n",
            ),
            ready
        )),
        ShellKind::PosixSh | ShellKind::Unknown => None,
    }
}

pub(super) fn bytes_after_ssh_ready_marker<'a>(
    bytes: &'a [u8],
    ready_marker: &[u8],
    legacy_ready_marker: Option<&[u8]>,
) -> Option<&'a [u8]> {
    find_subsequence(bytes, ready_marker)
        .map(|index| &bytes[index + ready_marker.len()..])
        .or_else(|| {
            legacy_ready_marker.and_then(|marker| {
                find_subsequence(bytes, marker).map(|index| &bytes[index + marker.len()..])
            })
        })
}

pub(super) fn strip_ssh_ready_markers(
    bytes: &[u8],
    ready_marker: &[u8],
    legacy_ready_marker: Option<&[u8]>,
) -> Vec<u8> {
    let mut output = strip_one_marker(bytes, ready_marker);
    if let Some(marker) = legacy_ready_marker {
        output = strip_one_marker(&output, marker);
    }
    output
}

fn strip_one_marker(bytes: &[u8], marker: &[u8]) -> Vec<u8> {
    if marker.is_empty() {
        return bytes.to_vec();
    }
    let mut output = Vec::with_capacity(bytes.len());
    let mut rest = bytes;
    while let Some(index) = find_subsequence(rest, marker) {
        output.extend_from_slice(&rest[..index]);
        rest = &rest[index + marker.len()..];
    }
    output.extend_from_slice(rest);
    output
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

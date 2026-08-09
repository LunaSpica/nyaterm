use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use russh::{ChannelMsg, client};

use super::{SshClientHandler, SshShellHandle};

const READY_MARKER_PREFIX: &str = "7777;NyaTermReady:";
const COMMAND_MARKER_PREFIX: &str = "7777;NyaTermCommand:";
const LEGACY_READY_MARKER_PREFIX: &str = "7777;DflyReady:";
const LEGACY_COMMAND_MARKER_PREFIX: &str = "7777;DflyCommand:";
const MAX_OSC_BUF: usize = 64 * 1024;
const SUPPRESSED_OUTPUT_LIMIT: usize = 64 * 1024;

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

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(super) struct SshIntegrationOutput {
    pub(super) visible: Vec<u8>,
    pub(super) cwd_paths: Vec<String>,
    pub(super) accepted_commands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OscResult {
    pub(super) visible: Vec<u8>,
    pub(super) visible_after_ready: Vec<u8>,
    pub(super) cwd_paths: Vec<String>,
    pub(super) ready: bool,
    pub(super) accepted_commands: Vec<String>,
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
    stripper: OscStripper,
    suppress_started_at: Option<Instant>,
    suppressed_visible_bytes: usize,
}

impl SshShellIntegrationState {
    pub(super) fn new(
        pending_script: Option<Vec<u8>>,
        ready_marker: String,
        legacy_ready_marker: Option<String>,
    ) -> Self {
        let phase = if pending_script.is_some() {
            SshShellIntegrationPhase::WaitInitial
        } else {
            SshShellIntegrationPhase::Normal
        };
        Self {
            phase,
            pending_script,
            stripper: OscStripper::new(&ready_marker, legacy_ready_marker.as_deref()),
            suppress_started_at: None,
            suppressed_visible_bytes: 0,
        }
    }

    pub(super) fn is_normal(&self) -> bool {
        self.phase == SshShellIntegrationPhase::Normal
    }

    pub(super) fn is_suppressing(&self) -> bool {
        self.phase == SshShellIntegrationPhase::Suppressing
    }

    pub(super) fn is_waiting_initial(&self) -> bool {
        self.phase == SshShellIntegrationPhase::WaitInitial
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
            self.force_normal();
        }
    }

    pub(super) fn force_normal_after_timeout(&mut self) -> SshIntegrationOutput {
        let flushed = self.stripper.flush();
        self.force_normal();
        let _ = flushed;
        SshIntegrationOutput::default()
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
        self.suppressed_visible_bytes = 0;
    }

    pub(super) fn filter_output(&mut self, bytes: &[u8]) -> SshIntegrationOutput {
        match self.phase {
            SshShellIntegrationPhase::Normal => self.stripper.push(bytes).into_output(),
            SshShellIntegrationPhase::WaitInitial => self.stripper.push(bytes).into_output(),
            SshShellIntegrationPhase::Suppressing => {
                if self.timeout_expired() {
                    return self.force_normal_after_timeout();
                }
                let result = self.stripper.push(bytes);
                let cwd_paths = result.cwd_paths;
                let accepted_commands = result.accepted_commands;
                if result.ready {
                    let visible = result.visible_after_ready;
                    self.force_normal();
                    SshIntegrationOutput {
                        visible,
                        cwd_paths,
                        accepted_commands,
                    }
                } else {
                    self.suppressed_visible_bytes = self
                        .suppressed_visible_bytes
                        .saturating_add(result.visible.len())
                        .min(SUPPRESSED_OUTPUT_LIMIT);
                    SshIntegrationOutput {
                        visible: Vec::new(),
                        cwd_paths,
                        accepted_commands,
                    }
                }
            }
        }
    }
}

impl OscResult {
    fn into_output(self) -> SshIntegrationOutput {
        SshIntegrationOutput {
            visible: self.visible,
            cwd_paths: self.cwd_paths,
            accepted_commands: self.accepted_commands,
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
        let channel = open_ssh_exec_channel(handle, command).await.ok()?;
        let output = collect_ssh_exec_stdout(channel).await;
        let kind = ShellKind::from_name(output.trim());
        (kind != ShellKind::Unknown).then_some(kind)
    })
    .await
    .ok()
    .flatten()
}

pub(super) async fn build_ssh_shell_integration_script(
    handle: &SshShellHandle,
    shell: ShellKind,
    ready_marker: &str,
    cwd_follow_mode: super::SftpCwdFollowMode,
    timeout_ms: u64,
) -> Option<Vec<u8>> {
    match cwd_follow_mode {
        super::SftpCwdFollowMode::Off => None,
        super::SftpCwdFollowMode::ShellIntegration => {
            ssh_shell_injection_script(shell, ready_marker).map(String::into_bytes)
        }
        super::SftpCwdFollowMode::RcFile => {
            match install_remote_shell_integration(handle, shell, timeout_ms).await {
                Ok(()) => activation_script(shell, ready_marker).map(String::into_bytes),
                Err(_error) => {
                    ssh_shell_injection_script(shell, ready_marker).map(String::into_bytes)
                }
            }
        }
    }
}

async fn open_ssh_exec_channel(
    handle: &SshShellHandle,
    command: &str,
) -> anyhow::Result<russh::Channel<client::Msg>> {
    match handle {
        SshShellHandle::Dedicated(handle) => open_ssh_exec_channel_on_handle(handle, command).await,
        SshShellHandle::Multiplexed(handle) => {
            let handle = handle.lock().await;
            open_ssh_exec_channel_on_handle(&handle, command).await
        }
    }
}

async fn open_ssh_exec_channel_on_handle(
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

async fn exec_remote_command(
    handle: &SshShellHandle,
    command: &str,
    timeout_ms: u64,
) -> anyhow::Result<String> {
    let timeout_ms = timeout_ms.clamp(100, 60_000);
    tokio::time::timeout(Duration::from_millis(timeout_ms), async {
        let mut channel = open_ssh_exec_channel(handle, command).await?;
        let mut output = String::new();
        let mut exit_status = None;
        while let Some(message) = channel.wait().await {
            match message {
                ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
                    output.push_str(&String::from_utf8_lossy(&data));
                }
                ChannelMsg::ExitStatus {
                    exit_status: status,
                } => exit_status = Some(status),
                ChannelMsg::Close | ChannelMsg::Eof => break,
                _ => {}
            }
        }
        let _ = channel.close().await;
        match exit_status.unwrap_or(0) {
            0 => Ok(output),
            status => anyhow::bail!("remote command exited with status {status}: {output}"),
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!("remote command timed out"))?
}

pub(super) fn build_ssh_ready_marker(session_id: &str) -> String {
    format!("\x1b]7777;NyaTermReady:{session_id}\x07")
}

pub(super) fn build_legacy_ssh_ready_marker(ready_marker: &str) -> Option<String> {
    let inner = marker_inner(ready_marker);
    let session_id = inner.strip_prefix(READY_MARKER_PREFIX)?;
    Some(format!("\x1b]{LEGACY_READY_MARKER_PREFIX}{session_id}\x07"))
}

fn ready_printf(marker: &str) -> String {
    marker
        .replace('\\', "\\\\")
        .replace('\x1b', "\\033")
        .replace('\x07', "\\007")
        .replace('\'', "'\\''")
}

pub(super) fn ssh_shell_injection_script(shell: ShellKind, ready_marker: &str) -> Option<String> {
    let ready_osc = ready_marker
        .replace('\x1b', "\\033")
        .replace('\x07', "\\007");

    match shell {
        ShellKind::Bash => Some(format!(
            concat!(
                " NYATERM_PRUNE_HISTORY=1;",
                " NYATERM_READY_PENDING=1;",
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
                " if [ -n \"${{NYATERM_READY_PENDING:-}}\" ]; then",
                " unset NYATERM_READY_PENDING;",
                " printf '{}';",
                " fi\n",
            ),
            ready_osc,
        )),
        ShellKind::Zsh => Some(format!(
            concat!(
                " fc -p /dev/null 2>/dev/null\n",
                " NYATERM_READY_PENDING=1;",
                " export NYATERM_INJ=1;",
                " __nyaterm_host(){{ hostname 2>/dev/null || printf localhost; }};",
                " __nyaterm_emit(){{",
                " printf '\\033]7;file://%s%s\\007' \"$(__nyaterm_host)\" \"$PWD\";",
                " }};",
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
                " if [ -n \"${{NYATERM_READY_PENDING:-}}\" ]; then",
                " unset NYATERM_READY_PENDING;",
                " printf '{}';",
                " fi\n",
            ),
            ready_osc,
        )),
        ShellKind::Fish => Some(format!(
            concat!(
                " set fish_private_mode 1 2>/dev/null\n",
                " set -g NYATERM_READY_PENDING 1;",
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
                " if set -q NYATERM_READY_PENDING;",
                " set -e NYATERM_READY_PENDING;",
                " printf '%s' '{}';",
                " end\n",
            ),
            ready_osc,
        )),
        ShellKind::PosixSh | ShellKind::Unknown => None,
    }
}

pub(super) fn activation_script(shell: ShellKind, ready_marker: &str) -> Option<String> {
    let ready = ready_printf(ready_marker);
    match shell {
        ShellKind::Bash => Some(format!(
            " NYATERM_PRUNE_HISTORY=1; NYATERM_READY_PENDING=1; export NYATERM_INJ=1; export NYATERM_READY_MARKER=\"$(printf '{}')\"; [ -r \"$HOME/.config/nyaterm/shell-integration.bash\" ] && . \"$HOME/.config/nyaterm/shell-integration.bash\"; __nyaterm_install_prompt 2>/dev/null; if [ -n \"${{NYATERM_READY_PENDING:-}}\" ]; then unset NYATERM_READY_PENDING; printf '%s' \"${{NYATERM_READY_MARKER-}}\"; fi\n",
            ready
        )),
        ShellKind::Zsh => Some(format!(
            " fc -p /dev/null 2>/dev/null\n NYATERM_READY_PENDING=1; export NYATERM_INJ=1; export NYATERM_READY_MARKER=\"$(printf '{}')\"; [ -r \"$HOME/.config/nyaterm/shell-integration.zsh\" ] && . \"$HOME/.config/nyaterm/shell-integration.zsh\"; __nyaterm_install_prompt 2>/dev/null; fc -P 2>/dev/null\n if [ -n \"${{NYATERM_READY_PENDING:-}}\" ]; then unset NYATERM_READY_PENDING; printf '%s' \"${{NYATERM_READY_MARKER-}}\"; fi\n",
            ready
        )),
        ShellKind::Fish => Some(format!(
            " set fish_private_mode 1 2>/dev/null\n set -g NYATERM_READY_PENDING 1; set -gx NYATERM_INJ 1; set -gx NYATERM_READY_MARKER (printf '{}'); if test -r \"$HOME/.config/nyaterm/shell-integration.fish\"; source \"$HOME/.config/nyaterm/shell-integration.fish\"; end; __nyaterm_install_prompt 2>/dev/null; set -e fish_private_mode 2>/dev/null\n if set -q NYATERM_READY_PENDING; set -e NYATERM_READY_PENDING; printf '%s' \"$NYATERM_READY_MARKER\"; end\n",
            ready
        )),
        ShellKind::PosixSh | ShellKind::Unknown => None,
    }
}

pub(super) fn persistent_script(shell: ShellKind) -> Option<&'static str> {
    match shell {
        ShellKind::Bash => Some(BASH_PERSISTENT_SCRIPT),
        ShellKind::Zsh => Some(ZSH_PERSISTENT_SCRIPT),
        ShellKind::Fish => Some(FISH_PERSISTENT_SCRIPT),
        ShellKind::PosixSh | ShellKind::Unknown => None,
    }
}

fn persistent_script_path(shell: ShellKind) -> Option<&'static str> {
    match shell {
        ShellKind::Bash => Some("$HOME/.config/nyaterm/shell-integration.bash"),
        ShellKind::Zsh => Some("$HOME/.config/nyaterm/shell-integration.zsh"),
        ShellKind::Fish => Some("$HOME/.config/nyaterm/shell-integration.fish"),
        ShellKind::PosixSh | ShellKind::Unknown => None,
    }
}

fn rc_file_path(shell: ShellKind) -> Option<&'static str> {
    match shell {
        ShellKind::Bash => Some("$HOME/.bashrc"),
        ShellKind::Zsh => Some("$HOME/.zshrc"),
        ShellKind::Fish => Some("$HOME/.config/fish/conf.d/nyaterm-shell-integration.fish"),
        ShellKind::PosixSh | ShellKind::Unknown => None,
    }
}

pub(super) const MANAGED_BLOCK_START: &str = "# >>> nyaterm shell integration >>>";
pub(super) const MANAGED_BLOCK_END: &str = "# <<< nyaterm shell integration <<<";

pub(super) fn rc_managed_block(shell: ShellKind) -> Option<String> {
    let source_path = persistent_script_path(shell)?;
    let body = match shell {
        ShellKind::Bash | ShellKind::Zsh => format!(
            "if [ -r \"{}\" ]; then\n  . \"{}\"\nfi",
            source_path, source_path
        ),
        ShellKind::Fish => format!(
            "if test -r \"{}\"\n  source \"{}\"\nend",
            source_path, source_path
        ),
        ShellKind::PosixSh | ShellKind::Unknown => return None,
    };
    Some(format!(
        "{MANAGED_BLOCK_START}\n{body}\n{MANAGED_BLOCK_END}"
    ))
}

fn sh_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn remote_install_command(shell: ShellKind) -> Option<String> {
    let script = persistent_script(shell)?;
    let block = rc_managed_block(shell)?;
    let script_path = persistent_script_path(shell)?;
    let rc_path = rc_file_path(shell)?;
    Some(format!(
        r#"set -eu
script_path={script_path}
rc_path={rc_path}
mkdir -p "$HOME/.config/nyaterm"
case "$rc_path" in */*) mkdir -p "${{rc_path%/*}}" ;; esac
script_tmp="${{script_path}}.tmp.$$"
cat > "$script_tmp" <<'NYATERM_SCRIPT_EOF'
{script}
NYATERM_SCRIPT_EOF
if [ ! -f "$script_path" ] || ! cmp -s "$script_tmp" "$script_path"; then
  mv "$script_tmp" "$script_path"
else
  rm -f "$script_tmp"
fi
block_tmp="${{script_path}}.block.$$"
cat > "$block_tmp" <<'NYATERM_BLOCK_EOF'
{block}
NYATERM_BLOCK_EOF
rc_tmp="${{rc_path}}.tmp.$$"
start={start}
end={end}
if [ -f "$rc_path" ] && grep -F "$start" "$rc_path" >/dev/null 2>&1 && grep -F "$end" "$rc_path" >/dev/null 2>&1; then
  NYATERM_BLOCK_FILE="$block_tmp" awk -v start="$start" -v end="$end" '
    $0 == start {{
      if (!done) {{
        while ((getline line < ENVIRON["NYATERM_BLOCK_FILE"]) > 0) print line
        close(ENVIRON["NYATERM_BLOCK_FILE"])
        done=1
      }}
      skip=1
      next
    }}
    $0 == end {{ skip=0; next }}
    !skip {{ print }}
    END {{
      if (!done) {{
        if (NR > 0) print ""
        while ((getline line < ENVIRON["NYATERM_BLOCK_FILE"]) > 0) print line
      }}
    }}
  ' "$rc_path" > "$rc_tmp"
else
  if [ -f "$rc_path" ]; then
    cat "$rc_path" > "$rc_tmp"
    if [ -s "$rc_tmp" ]; then printf '\n' >> "$rc_tmp"; fi
  else
    : > "$rc_tmp"
  fi
  cat "$block_tmp" >> "$rc_tmp"
fi
if [ ! -f "$rc_path" ] || ! cmp -s "$rc_tmp" "$rc_path"; then
  if [ -f "$rc_path" ] && [ ! -f "$rc_path.nyaterm.bak" ]; then
    cp "$rc_path" "$rc_path.nyaterm.bak" 2>/dev/null || true
  fi
  mv "$rc_tmp" "$rc_path"
else
  rm -f "$rc_tmp"
fi
rm -f "$block_tmp"
"#,
        script_path = script_path,
        rc_path = rc_path,
        script = script,
        block = block,
        start = sh_single_quote(MANAGED_BLOCK_START),
        end = sh_single_quote(MANAGED_BLOCK_END),
    ))
}

async fn install_remote_shell_integration(
    handle: &SshShellHandle,
    shell: ShellKind,
    timeout_ms: u64,
) -> anyhow::Result<()> {
    let Some(command) = remote_install_command(shell) else {
        anyhow::bail!("no persistent shell integration available for {shell:?}");
    };
    exec_remote_command(handle, &command, timeout_ms)
        .await
        .map(|_| ())
}

const BASH_PERSISTENT_SCRIPT: &str = concat!(
    "# nyaterm shell integration v1\n",
    "__nyaterm_host(){ hostname 2>/dev/null || printf localhost; }\n",
    "__nyaterm_prune_history(){\n",
    "  [ -n \"${NYATERM_PRUNE_HISTORY:-}\" ] || return 0\n",
    "  unset NYATERM_PRUNE_HISTORY\n",
    "  local hline\n",
    "  hline=\"$(HISTTIMEFORMAT= history 1 2>/dev/null || true)\"\n",
    "  case \"$hline\" in\n",
    "    (*NYATERM_PRUNE_HISTORY*|*NYATERM_INJ*|*__nyaterm_install_prompt*|*NyaTermReady*)\n",
    "      if [[ \"$hline\" =~ ^[[:space:]]*([0-9]+) ]]; then history -d \"${BASH_REMATCH[1]}\" 2>/dev/null || true; fi\n",
    "      ;;\n",
    "  esac\n",
    "}\n",
    "__nyaterm_emit_command(){\n",
    "  local histcmd=\"${HISTCMD-}\"\n",
    "  if [ -n \"$histcmd\" ] && [ \"${NYATERM_LAST_HISTCMD-}\" != \"$histcmd\" ]; then\n",
    "    NYATERM_LAST_HISTCMD=\"$histcmd\"\n",
    "    local cmd; cmd=\"$(fc -ln -1 2>/dev/null)\"\n",
    "    if [ -n \"$cmd\" ] && command -v base64 >/dev/null 2>&1; then\n",
    "      local b64; b64=\"$(printf '%s' \"$cmd\" | base64 | tr -d '\\r\\n')\"\n",
    "      printf '\\033]7777;NyaTermCommand:%s\\007' \"$b64\"\n",
    "    fi\n",
    "  fi\n",
    "}\n",
    "__nyaterm_prompt(){\n",
    "  __nyaterm_prune_history\n",
    "  __nyaterm_emit_command\n",
    "  if [ -n \"${NYATERM_READY_PENDING:-}\" ]; then unset NYATERM_READY_PENDING; printf '%s' \"${NYATERM_READY_MARKER-}\"; fi\n",
    "  printf '\\033]7;file://%s%s\\007' \"$(__nyaterm_host)\" \"$PWD\"\n",
    "}\n",
    "__nyaterm_install_prompt(){\n",
    "  NYATERM_LAST_HISTCMD=\"${HISTCMD-}\"\n",
    "  local decl\n",
    "  decl=\"$(declare -p PROMPT_COMMAND 2>/dev/null || true)\"\n",
    "  if [[ \"$decl\" =~ ^declare\\ -[^[:space:]]*a[^[:space:]]*\\ PROMPT_COMMAND= ]]; then\n",
    "    local f\n",
    "    for f in \"${PROMPT_COMMAND[@]}\"; do [ \"$f\" = __nyaterm_prompt ] && return 0; done\n",
    "    PROMPT_COMMAND=(__nyaterm_prompt \"${PROMPT_COMMAND[@]}\")\n",
    "  else\n",
    "    case \"${PROMPT_COMMAND-}\" in (*__nyaterm_prompt*) ;; (*) PROMPT_COMMAND=\"__nyaterm_prompt${PROMPT_COMMAND:+; $PROMPT_COMMAND}\" ;; esac\n",
    "  fi\n",
    "}\n"
);

const ZSH_PERSISTENT_SCRIPT: &str = concat!(
    "# nyaterm shell integration v1\n",
    "__nyaterm_host(){ hostname 2>/dev/null || printf localhost; }\n",
    "__nyaterm_emit(){\n",
    "  if [ -n \"${NYATERM_READY_PENDING:-}\" ]; then unset NYATERM_READY_PENDING; printf '%s' \"${NYATERM_READY_MARKER-}\"; fi\n",
    "  printf '\\033]7;file://%s%s\\007' \"$(__nyaterm_host)\" \"$PWD\"\n",
    "}\n",
    "__nyaterm_preexec(){\n",
    "  if [ -n \"$1\" ] && command -v base64 >/dev/null 2>&1; then\n",
    "    local b64; b64=\"$(printf '%s' \"$1\" | base64 | tr -d '\\r\\n')\"\n",
    "    printf '\\033]7777;NyaTermCommand:%s\\007' \"$b64\"\n",
    "  fi\n",
    "}\n",
    "__nyaterm_install_prompt(){\n",
    "  autoload -Uz add-zsh-hook 2>/dev/null || true\n",
    "  typeset -ga precmd_functions preexec_functions\n",
    "  [[ \" ${precmd_functions[*]} \" == *\" __nyaterm_emit \"* ]] || precmd_functions+=(__nyaterm_emit)\n",
    "  [[ \" ${preexec_functions[*]} \" == *\" __nyaterm_preexec \"* ]] || preexec_functions+=(__nyaterm_preexec)\n",
    "}\n"
);

const FISH_PERSISTENT_SCRIPT: &str = concat!(
    "# nyaterm shell integration v1\n",
    "function __nyaterm_emit\n",
    "  if set -q NYATERM_READY_PENDING\n",
    "    set -e NYATERM_READY_PENDING\n",
    "    printf '%s' \"$NYATERM_READY_MARKER\"\n",
    "  end\n",
    "  printf '\\033]7;file://%s%s\\007' (hostname) $PWD\n",
    "end\n",
    "function __nyaterm_preexec\n",
    "  if test -n \"$argv[1]\"; and command -sq base64\n",
    "    set -l b64 (printf '%s' \"$argv[1]\" | base64 | tr -d '\\r\\n')\n",
    "    if test -n \"$b64\"\n",
    "      printf '\\033]7777;NyaTermCommand:%s\\007' \"$b64\"\n",
    "    end\n",
    "  end\n",
    "end\n",
    "function __nyaterm_install_prompt\n",
    "  functions -e __nyaterm_emit_event 2>/dev/null\n",
    "  functions -e __nyaterm_preexec_event 2>/dev/null\n",
    "  function __nyaterm_emit_event --on-event fish_prompt\n",
    "    __nyaterm_emit\n",
    "  end\n",
    "  function __nyaterm_preexec_event --on-event fish_preexec\n",
    "    __nyaterm_preexec $argv\n",
    "  end\n",
    "end\n"
);

pub(super) struct OscStripper {
    buf: Vec<u8>,
    ready_inner: Vec<u8>,
    legacy_ready_inner: Option<Vec<u8>>,
}

impl OscStripper {
    pub(super) fn new(ready_marker: &str, legacy_ready_marker: Option<&str>) -> Self {
        Self {
            buf: Vec::new(),
            ready_inner: marker_inner(ready_marker).into_bytes(),
            legacy_ready_inner: legacy_ready_marker.map(|marker| marker_inner(marker).into_bytes()),
        }
    }

    pub(super) fn push(&mut self, chunk: &[u8]) -> OscResult {
        self.buf.extend_from_slice(chunk);
        if self.buf.len() > MAX_OSC_BUF && find_subsequence(&self.buf, b"\x1b]").is_none() {
            return OscResult {
                visible: std::mem::take(&mut self.buf),
                visible_after_ready: Vec::new(),
                cwd_paths: Vec::new(),
                ready: false,
                accepted_commands: Vec::new(),
            };
        }

        let mut visible = Vec::new();
        let mut visible_after_ready = Vec::new();
        let mut cwd_paths = Vec::new();
        let mut ready = false;
        let mut after_ready = false;
        let mut accepted_commands = Vec::new();

        loop {
            let Some(esc_pos) = find_subsequence(&self.buf, b"\x1b]") else {
                if after_ready {
                    visible_after_ready.extend_from_slice(&self.buf);
                }
                visible.extend_from_slice(&self.buf);
                self.buf.clear();
                break;
            };

            if after_ready {
                visible_after_ready.extend_from_slice(&self.buf[..esc_pos]);
            }
            visible.extend_from_slice(&self.buf[..esc_pos]);
            let rest = self.buf[esc_pos..].to_vec();
            let Some((end_idx, term_len)) = find_osc_terminator(&rest) else {
                self.buf = rest;
                if self.buf.len() > MAX_OSC_BUF {
                    visible.extend_from_slice(&self.buf);
                    self.buf.clear();
                }
                break;
            };

            let seq_end = end_idx + term_len;
            let seq = &rest[..seq_end];
            let inner = &rest[2..end_idx];
            if let Some(payload) = inner.strip_prefix(b"7;") {
                if let Some(path) = parse_osc7_payload(payload) {
                    cwd_paths.push(path);
                }
            } else if self.is_current_ready_marker(inner) {
                ready = true;
                after_ready = true;
            } else if inner.starts_with(READY_MARKER_PREFIX.as_bytes())
                || inner.starts_with(LEGACY_READY_MARKER_PREFIX.as_bytes())
            {
                // Private marker for another session. Strip it but do not mark ready.
            } else if let Some(command) = parse_command_marker(inner) {
                accepted_commands.push(command);
            } else {
                if after_ready {
                    visible_after_ready.extend_from_slice(seq);
                }
                visible.extend_from_slice(seq);
            }
            self.buf = rest[seq_end..].to_vec();
        }

        OscResult {
            visible,
            visible_after_ready,
            cwd_paths,
            ready,
            accepted_commands,
        }
    }

    pub(super) fn flush(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.buf)
    }

    fn is_current_ready_marker(&self, inner: &[u8]) -> bool {
        inner == self.ready_inner || self.legacy_ready_inner.as_deref() == Some(inner)
    }
}

fn marker_inner(marker: &str) -> String {
    let Some(rest) = marker.strip_prefix("\x1b]") else {
        return marker.to_string();
    };
    if let Some(inner) = rest.strip_suffix('\x07') {
        inner.to_string()
    } else if let Some(inner) = rest.strip_suffix("\x1b\\") {
        inner.to_string()
    } else {
        rest.to_string()
    }
}

fn find_osc_terminator(bytes: &[u8]) -> Option<(usize, usize)> {
    bytes
        .iter()
        .position(|byte| *byte == b'\x07')
        .map(|index| (index, 1))
        .or_else(|| find_subsequence(&bytes[2..], b"\x1b\\").map(|index| (index + 2, 2)))
}

fn parse_osc7_payload(payload: &[u8]) -> Option<String> {
    let payload = std::str::from_utf8(payload).ok()?;
    let after_scheme = payload.strip_prefix("file://")?;
    let path = if after_scheme.starts_with('/') {
        after_scheme.to_string()
    } else {
        let slash = after_scheme.find('/')?;
        after_scheme[slash..].to_string()
    };
    if path.is_empty() { None } else { Some(path) }
}

fn parse_command_marker(inner: &[u8]) -> Option<String> {
    let payload = inner
        .strip_prefix(COMMAND_MARKER_PREFIX.as_bytes())
        .or_else(|| inner.strip_prefix(LEGACY_COMMAND_MARKER_PREFIX.as_bytes()))?;
    let decoded = BASE64_STANDARD.decode(payload).ok()?;
    let command = String::from_utf8(decoded).ok()?;
    if command.is_empty() {
        None
    } else {
        Some(command)
    }
}

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;

    use super::{
        SshShellIntegrationPhase, SshShellIntegrationState, build_legacy_ssh_ready_marker,
        build_ssh_ready_marker,
    };

    fn shell_integration_state(session_id: &str) -> SshShellIntegrationState {
        let ready_marker = build_ssh_ready_marker(session_id);
        let legacy_ready_marker = build_legacy_ssh_ready_marker(&ready_marker);
        SshShellIntegrationState::new(
            Some(b"inject-script\n".to_vec()),
            ready_marker,
            legacy_ready_marker,
        )
    }

    fn mark_injection_sent(state: &mut SshShellIntegrationState) {
        state.phase = SshShellIntegrationPhase::Suppressing;
        state.pending_script = None;
        state.suppress_started_at = Some(Instant::now());
    }

    #[test]
    fn wait_initial_passes_first_output_and_keeps_injection_pending() {
        let mut state = shell_integration_state("session-1");

        let output = state.filter_output(b"Welcome to Ubuntu\r\nLast login: today\r\n");

        assert_eq!(
            output.visible,
            b"Welcome to Ubuntu\r\nLast login: today\r\n"
        );
        assert!(output.cwd_paths.is_empty());
        assert!(output.accepted_commands.is_empty());
        assert!(state.is_waiting_initial());
        assert!(state.should_inject_on_initial_delay());
    }

    #[test]
    fn suppressing_discards_visible_output_until_ready_marker() {
        let mut state = shell_integration_state("session-1");
        mark_injection_sent(&mut state);

        let before_ready = state.filter_output(b"echoed injection and stale prompt# ");

        assert!(before_ready.visible.is_empty());
        assert!(before_ready.cwd_paths.is_empty());
        assert!(before_ready.accepted_commands.is_empty());
        assert!(state.is_suppressing());
    }

    #[test]
    fn ready_marker_enters_normal_and_preserves_only_visible_after_ready() {
        let mut state = shell_integration_state("session-1");
        mark_injection_sent(&mut state);
        let command = BASE64_STANDARD.encode("git status");

        let output = state.filter_output(
            format!(
                "old prompt# \x1b]7;file://host/home/user\x07\
                 \x1b]7777;NyaTermCommand:{command}\x07\
                 \x1b]7777;NyaTermReady:session-1\x07prompt# "
            )
            .as_bytes(),
        );

        assert_eq!(output.visible, b"prompt# ");
        assert_eq!(output.cwd_paths, vec!["/home/user".to_string()]);
        assert_eq!(output.accepted_commands, vec!["git status".to_string()]);
        assert!(state.is_normal());
    }

    #[test]
    fn tauri_parity_keeps_motd_discards_injection_noise_and_shows_final_prompt() {
        let mut state = shell_integration_state("session-1");
        let mut visible = Vec::new();

        let initial = state.filter_output(b"Welcome to Ubuntu\r\nLast login: today\r\n");
        visible.extend_from_slice(&initial.visible);
        assert!(state.is_waiting_initial());

        mark_injection_sent(&mut state);
        let stale_prompt = state.filter_output(b"root@host:~# ");
        visible.extend_from_slice(&stale_prompt.visible);

        let ready = state.filter_output(b"\x1b]7777;NyaTermReady:session-1\x07root@host:~# ");
        visible.extend_from_slice(&ready.visible);

        assert_eq!(
            visible,
            b"Welcome to Ubuntu\r\nLast login: today\r\nroot@host:~# "
        );
        assert_eq!(count_subsequence(&visible, b"root@host:~# "), 1);
        assert!(state.is_normal());
    }

    #[test]
    fn initial_delay_can_still_inject_when_remote_outputs_nothing() {
        let state = shell_integration_state("session-1");

        assert!(state.should_inject_on_initial_delay());
        assert!(state.is_waiting_initial());
    }

    #[test]
    fn suppressing_timeout_discards_buffered_output_and_enters_normal() {
        let mut state = shell_integration_state("session-1");
        mark_injection_sent(&mut state);
        state.suppress_started_at = Some(Instant::now() - Duration::from_secs(31));

        let output = state.filter_output(b"stale prompt# ");

        assert!(output.visible.is_empty());
        assert!(output.cwd_paths.is_empty());
        assert!(output.accepted_commands.is_empty());
        assert!(state.is_normal());
    }

    fn count_subsequence(haystack: &[u8], needle: &[u8]) -> usize {
        haystack
            .windows(needle.len())
            .filter(|window| *window == needle)
            .count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeServiceStatus {
    Ready,
    Porting,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrationCapability {
    pub area: &'static str,
    pub status: NativeServiceStatus,
    pub note: &'static str,
}

#[derive(Debug, Clone)]
pub struct NativeServices {
    capabilities: Vec<MigrationCapability>,
}

impl NativeServices {
    pub fn new() -> Self {
        Self {
            capabilities: vec![
                MigrationCapability {
                    area: "GPUI window shell",
                    status: NativeServiceStatus::Ready,
                    note: "Native window, layout, navigation, and migration dashboard are active.",
                },
                MigrationCapability {
                    area: "Configuration model and store",
                    status: NativeServiceStatus::Ready,
                    note: "Saved connection/group schema, credential password, SSH key and OTP hydration, known_hosts, redb table layout, and native redb backup import/export are compatible with the Tauri backend.",
                },
                MigrationCapability {
                    area: "OTP",
                    status: NativeServiceStatus::Ready,
                    note: "The standalone nyaterm-otp crate is part of the new workspace, with native TOTP/HOTP auto-fill support for SSH keyboard-interactive prompts.",
                },
                MigrationCapability {
                    area: "Local PTY",
                    status: NativeServiceStatus::Ready,
                    note: "A Tauri-free portable-pty session manager is available for local terminals.",
                },
                MigrationCapability {
                    area: "Terminal screen model",
                    status: NativeServiceStatus::Ready,
                    note: "The native GPUI terminal renders a Rust vte-backed screen model for printable text, cursor movement, line/display erase, SGR filtering, baseline wrapping, and legacy-compatible keyword highlights.",
                },
                MigrationCapability {
                    area: "Telnet / Raw TCP",
                    status: NativeServiceStatus::Ready,
                    note: "The native session manager supports TCP sessions with basic Telnet IAC negotiation and NAWS.",
                },
                MigrationCapability {
                    area: "Serial",
                    status: NativeServiceStatus::Ready,
                    note: "The Tauri-free session manager opens serial ports, streams output, writes input, and closes reader threads cleanly.",
                },
                MigrationCapability {
                    area: "SSH terminal baseline",
                    status: NativeServiceStatus::Ready,
                    note: "The native session manager can open saved-password, runtime-password, none-auth, private-key, certificate, keyboard-interactive, or ProxyJump SSH PTY shells with OTP auto-fill, known_hosts checks, GPUI prompt decisions, write, resize, output, and close events.",
                },
                MigrationCapability {
                    area: "SSH credentials and proxying",
                    status: NativeServiceStatus::Ready,
                    note: "ProxyJump is active. Native SSH multiplex handles can share an authenticated target/jump chain across exec, SFTP, and local/remote/dynamic tunnel adapters without per-operation disconnects. Legacy tunnel profiles load from redb, and the GPUI Tunnels panel can open/close local direct-tcpip, remote forwarded-tcpip, and dynamic SOCKS5 native adapters. SSH shell sessions can request X11 forwarding and proxy X11 channels to the local display with legacy-compatible cookie rewriting.",
                },
                MigrationCapability {
                    area: "SFTP and transfers",
                    status: NativeServiceStatus::Ready,
                    note: "A native russh-sftp adapter supports authenticated list/download/upload operations with streamed file and directory IO, optional SSH multiplex handle reuse, progress events, cancellation, cooperative pause/resume, overwrite/skip/rename/ask duplicate policies, GPUI duplicate decision prompts, and GPUI platform path pickers for upload files, upload directories, and download targets. Session recording is native as well: terminal input/output capture, control-sequence stripping, echo suppression, file recording, transcript save/search, legacy recording settings, and auto-start are wired through GPUI.",
                },
                MigrationCapability {
                    area: "Remote processes",
                    status: NativeServiceStatus::Ready,
                    note: "The GPUI Stats and Processes panels use native SSH exec channels to collect remote system snapshots, list remote processes, and issue TERM/KILL/renice actions through the same authentication, ProxyJump, known_hosts, prompt path, and optional multiplex handle reuse as SSH sessions. The Docker panel uses the same native SSH exec path for overview, images, volumes, networks, compose projects, container logs, container start/stop/restart actions, and system prune.",
                },
                MigrationCapability {
                    area: "Translation",
                    status: NativeServiceStatus::Ready,
                    note: "The native domain layer maps legacy provider language aliases, parses Google/Microsoft/DeepL/Baidu/Ali/Youdao responses, builds Baidu/Youdao/Ali signatures, and reads legacy plaintext translation credentials while saving secrets through the native master-key hierarchy. GPUI has a compact Translation page with provider switching and native HTTP execution off the render thread, plus Settings controls for target language, app IDs, secret replacement, and secret clear actions.",
                },
                MigrationCapability {
                    area: "Diagnostics and logs",
                    status: NativeServiceStatus::Ready,
                    note: "Native runtime creates JSONL diagnostics logs, the Settings panel can reveal the log directory through GPUI, and diagnostics archives include logs, manifest, and runtime snapshot data without Tauri commands.",
                },
                MigrationCapability {
                    area: "Cloud sync / AI",
                    status: NativeServiceStatus::Ready,
                    note: "The native domain layer can encode/decode legacy v3 portable snapshot redb payloads, zip compression, AES-GCM .nya encryption, and legacy Dragonfly decrypt fallback. Settings can export/import plaintext and encrypted .nya backups, edit and persist legacy-compatible cloud sync, quick-command, command-history, and AI provider/model settings with encrypted secrets, store legacy-shaped AI sessions/messages/audit logs, resolve AI models and credentials, discover OpenAI-compatible models through native HTTP, build and execute OpenAI-compatible, Anthropic, and Gemini Ask requests, stream Ask deltas for OpenAI-compatible, Anthropic, and Gemini providers, build AI and Agent prompts, redact sensitive context, parse command-card and Agent execute_command/final_answer model output, assess Agent command risk locally, build OpenAI-compatible/Anthropic/Gemini Agent tool-call requests, parse non-stream and streamed execute_command/final_answer tool calls, build and parse marker-based Agent command captures with exit codes, fuzzy-search history and quick commands, run encrypted cloud snapshot push/pull through a backend abstraction, protect non-forced cloud sync pull/push from conflicts, and show legacy-shaped sync history from diagnostics logs; local-directory, WebDAV, S3-compatible, Google Drive, OneDrive, Aliyun Drive, Gitee/GitHub snippet provider actions, GPUI cloud conflict resolution with explicit Force Push/Pull, quick-command panel insert/run, command-history panel insert/run/delete, command-search insert/run, AI model discovery, provider-native Ask streaming, foreground Agent observe/continue loops through the active terminal with marker capture for supported profiles, local/SSH Agent background execution, provider-native Agent streaming tool-call execution with JSON fallback, Agent tool-call stream status, compact Agent step timeline, Agent cancellation, command-card save/insert/run actions, and AI audit recording are wired into GPUI.",
                },
                MigrationCapability {
                    area: "WebView/Tauri plugins",
                    status: NativeServiceStatus::Ready,
                    note: "The GPUI Settings surface has a Tauri-free native update checker backed by GitHub release metadata. Native path pickers, path reveal, in-window prompts, local subprocess execution, and remote SSH exec cover the migrated dialog/process surface without Tauri commands, plugins, invoke handlers, or WebView dependencies. The Workspace right panel has a GPUI Command Center that replaces the legacy tray action surface for new connections, active sessions, cloud sync push/pull/history, settings, updates, and migration status; OS-level tray integration remains optional platform polish.",
                },
            ],
        }
    }

    pub fn capabilities(&self) -> &[MigrationCapability] {
        &self.capabilities
    }
}

impl Default for NativeServices {
    fn default() -> Self {
        Self::new()
    }
}

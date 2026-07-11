# NyaTerm Native Migration Plan

The repository root is now the Rust native workspace. The ignored `nyaterm-tauri/`
directory remains the source-of-truth input for migration until every service has
been ported or replaced.

## Architecture

- `crates/nyaterm-native`: GPUI application entry point and native desktop UI.
- `crates/nyaterm-domain`: UI-independent models, runtime paths, and service
  status contracts, plus the native redb-compatible connection store.
- `crates/nyaterm-migration`: inventory and legacy configuration loading helpers.
- `crates/nyaterm-session`: Tauri-free session services. Local PTY is backed by
  `portable-pty`; Telnet/Raw TCP uses native `TcpStream`; Serial uses
  `serialport`; SSH uses native `russh` behind the same service shape.
- `crates/nyaterm-terminal`: Rust terminal screen model backed by `vte`, used by
  the GPUI workspace instead of xterm.js/WebView rendering.
- `crates/nyaterm-otp`: copied standalone OTP crate from the Tauri source tree.
- `vendor/`: protocol dependencies copied from the Tauri source tree.

## Migration Boundary

The old Tauri app has substantial direct coupling to `tauri::AppHandle`,
`tauri::Emitter`, managed state, and plugin APIs inside `config`, `core`,
`storage`, and command modules. The native project therefore introduces a
service boundary first:

1. Replace `AppHandle` path access with `AppRuntime`.
2. Replace Tauri event emitters with a native event bus consumed by GPUI views.
3. Move protocol/session code behind services that can be tested without UI.
4. Attach GPUI terminal rendering to the session event stream.
5. Replace Tauri plugins with native crates or GPUI/platform equivalents.

## Current Native Coverage

- GPUI native window shell.
- Workspace, connection, tunnel, settings, transfer, and migration panels.
- Focusable GPUI terminal workspace with session output streaming and baseline
  keyboard input dispatch for printable characters, control keys, and common
  navigation escape sequences.
- Native terminal screen model with `vte` parsing for printable characters,
  carriage return/line feed/backspace/tab, cursor movement, line/display erase,
  SGR filtering, fixed-size screen snapshots rendered by GPUI, and
  legacy-compatible keyword-highlight settings/import with baseline native
  terminal highlighting.
- Legacy source inventory.
- Compatible saved connection/group schema.
- redb-compatible connection/group loading and persistence using the same
  `nyaterm.redb` tables and key prefixes as the Tauri backend.
- Legacy credential decryption for saved connection passwords, including
  `master.key` unwrap compatibility with NyaTerm and legacy Dragonfly wrapping
  prefixes plus portable key material.
- Legacy SSH key-store hydration for encrypted private key PEM, OpenSSH
  certificate, and key passphrase material.
- redb-compatible `known_hosts` repository with structured records, raw line
  preservation, OpenSSH hashed-host matching, legacy text-doc import, and
  changed-key detection.
- Native settings summary and SSH host-key policy persistence, preserving the
  legacy `settings/default` document shape.
- Local PTY service with create/write/resize/close and output/error/exit events.
- Telnet/Raw TCP service with create/write/resize/close, output/error/exit
  events, basic IAC negotiation, and NAWS resize payloads.
- Serial service with create/write/resize/close, port listing, output/error
  events, legacy-compatible port settings, and clean reader shutdown.
- SSH terminal baseline with native `russh` PTY shell creation for saved
  password, runtime password prompt, none auth, private-key, certificate auth,
  runtime key-passphrase prompt, keyboard-interactive verification, saved
  password keyboard-interactive fallback, and OTP auto-fill for TOTP/HOTP
  prompts; ProxyJump chains through direct-tcpip; known_hosts verification;
  GPUI host-key prompt decisions; plus write/resize/close and
  output/error/exit events.
- Native SSH tunnel service primitives for local direct-tcpip forwarding,
  remote forwarded-tcpip forwarding, and dynamic SOCKS5 forwarding, including
  authenticated SSH handle creation, listener lifecycle management, forwarded
  channel handling, SOCKS5 CONNECT parsing, and clean shutdown.
- Native SSH multiplex handles now keep a shared authenticated `russh`
  connection/runtime alive behind a typed handle. Remote exec, SFTP, and local,
  remote, or dynamic tunnel services can opt into that handle without
  disconnecting the shared target/jump chain after each operation, matching the
  legacy shared-handle adapter behavior while preserving the existing dedicated
  connection path.
- redb-compatible legacy tunnel profile loading from the `tunnels` table and a
  GPUI Tunnels panel with open/close actions for local, remote, and dynamic
  tunnel modes.
- Runtime directory resolution for installed and portable modes.
- Legacy OTP account store hydration/decryption, standalone OTP generation, and
  vendored protocol dependencies copied into the root workspace.
- Native SFTP service adapter using `russh-sftp` for authenticated
  list/download/upload operations over the same SSH configuration path, with
  streamed file/directory IO, cooperative pause/resume, cancellation controls,
  overwrite/skip/rename/ask duplicate policies, and progress callbacks for
  upload/download jobs.
- Transfers panel has native SFTP list/download/upload jobs wired through path
  inputs, a background queue, live transfer progress, pause/resume/cancel
  actions, duplicate policy controls, interactive duplicate decision prompts,
  result channel, and GPUI platform path pickers for upload files, upload
  directories, and download target directories.
- Settings panel has native GPUI config backup pickers for exporting and
  importing the legacy-compatible `nyaterm.redb` store, including pre-import
  validation, post-import store refresh, and retained safety backups of the
  replaced database.
- Settings panel can also export and import legacy v3 `.nya` portable
  snapshots in plaintext or AES-GCM encrypted form with a GPUI master-password
  prompt. The native domain bridge maps supported redb entities for sessions,
  credential records, SSH keys, OTP accounts, proxies, tunnels, tunnel groups,
  quick commands, command history, master-key tokens, and known_hosts
  while preserving the importing machine's local master-password setting.
- Native diagnostics logging writes JSONL files into the resolved runtime log
  directory. The Settings panel can reveal logs through GPUI and export a zip
  diagnostics bundle containing retained log files, `manifest.json`, and
  `runtime_snapshot.json`.
- Processes panel uses native SSH exec channels to list remote process
  snapshots and send TERM/KILL/renice actions. It reuses the same SSH
  authentication, ProxyJump, known_hosts, credential prompt, and OTP path as
  interactive SSH sessions, and the underlying service can run commands over a
  caller-provided SSH multiplex handle.
- Stats panel uses native SSH exec channels to collect the legacy-compatible
  remote system snapshot: host, uptime, OS, architecture, load average, CPU
  usage/per-core usage, memory, active network throughput, and mounted block
  devices.
- Translation now has native domain protocol coverage and a GPUI panel for the
  legacy provider set. The native runtime maps provider-specific language
  aliases, builds Baidu/Youdao/Ali signatures, reads legacy plaintext
  translation credentials and upgrades saved secrets through the native
  master-key hierarchy, sends Google, Microsoft, DeepL, Baidu, Ali, and Youdao
  requests through `zed-reqwest` off the render thread, parses detected source
  language and translated text without Tauri commands, reports missing
  credentials or provider errors explicitly, and exposes GPUI Settings controls
  for target language, provider app IDs, secret replacement, and secret clear
  actions.
- Native domain snapshot codec can round-trip legacy v3 portable snapshot redb
  payloads, zip compression, AES-GCM `.nya` encryption, and legacy Dragonfly
  decryption fallback. Plaintext and encrypted `.nya` backup import/export are
  wired into GPUI.
- Native cloud sync now has a legacy-shaped local-directory provider path for
  encrypted sync snapshots: `sync/current.redb.enc`, redb-backed
  `sync/latest.redb` revision pointers, per-revision snapshot files, master
  password encryption/decryption, push/pull state tracking, safety backups on
  pull, and conflict detection. The Settings panel exposes push/pull actions
  through a GPUI master-password prompt, shows recent sync history read from
  retained diagnostics JSONL logs, and displays conflict resolution controls
  with explicit Force Push / Force Pull actions instead of treating conflicts
  as ordinary failures.
- Cloud sync history logging now uses the Tauri-compatible
  `cloud_sync.history` / `entry` structured log shape, including trigger,
  provider, revision, duration, and success/conflict/failed statuses. Native
  local push/pull actions append entries without depending on Tauri
  observability.
- The cloud sync push/pull algorithm now targets a native `CloudSyncRemote`
  backend trait instead of direct filesystem IO. The checked-in local-directory
  backend preserves the current GPUI workflow, while the same snapshot,
  revision-pointer, state, and conflict logic is ready for WebDAV/Gist/Drive
  backend adapters.
- Gitee/GitHub Gist-style snippet storage now has a native dependency-free
  `SnippetRemote` core: cloud paths map to legacy `nyaterm-{base64url}.blob`
  filenames, blob payloads are standard-base64 encoded, and the remote can run
  the full encrypted push/pull algorithm through native provider actions.
- The native domain layer also carries dependency-free Gitee and GitHub Gist
  HTTP adapter semantics behind `SnippetHttpClient`: request URL/header/query
  construction, patch JSON bodies, Gitee `access_token` handling, GitHub API
  headers, truncated-file `raw_url` fallback, and retryable GitHub 409 update
  conflict detection are covered by unit tests. The GPUI app binds this adapter
  to the locked `zed-reqwest` client and exposes `Push Provider` / `Pull
  Provider` actions for `gitee_snippet`, `github_gist`, and `local_directory`,
  while unsupported providers fail explicitly instead of silently using a
  different backend.
- WebDAV provider actions are now wired through a native `CloudSyncRemote`
  implementation backed by `zed-reqwest`, with endpoint/root normalization,
  MKCOL directory creation, GET/PUT snapshot IO, Basic auth, and Digest
  `qop=auth` retry support for MD5 and SHA-256 challenges.
- S3-compatible provider actions are now wired through a native
  `CloudSyncRemote` backed by `zed-reqwest` and dependency-free SigV4 helpers:
  endpoint/bucket/root normalization, path-style and virtual-host style object
  URLs, access key/secret/session-token signing, GET/PUT snapshot IO, and
  offline coverage for canonical URLs and signed headers.
- Google Drive provider actions are now wired through a native
  `CloudSyncRemote` backed by Drive v3 HTTP calls: root/folder path resolution,
  OAuth access-token use with refresh-token retry, folder creation, media
  download, multipart file create, media file update, and GPUI controls for
  root/client/token fields.
- OneDrive provider actions are now wired through a native `CloudSyncRemote`
  backed by Microsoft Graph: path-addressed item lookup, folder creation,
  small-file content upload/download, OAuth access-token use with refresh-token
  retry, and GPUI controls for root/client/token fields.
- Aliyun Drive provider actions are now wired through a native
  `CloudSyncRemote` backed by Alipan open API calls: drive-type resolution,
  path lookup, folder creation, signed upload URL retrieval, single-part
  upload, upload completion, download URL retrieval, OAuth access-token use
  with refresh-token retry, and GPUI controls for root/drive-type/client/token
  fields.
- Native domain storage also reads and writes legacy-compatible cloud sync
  settings from `settings/doc/cloud-sync`, including WebDAV, S3, Gitee snippet,
  GitHub Gist, Google Drive, OneDrive, and Aliyun Drive configuration shapes.
  Provider secrets are merged through the legacy masked-secret sentinel and
  stored with the same `master.key` AES-GCM credential hierarchy.
- The Settings panel now includes native GPUI cloud sync configuration controls
  for enabled state, active provider, remote root, WebDAV endpoint/root/user/
  password, S3 endpoint/bucket/region/root/access-key/secret/session-token/URL
  style, Google Drive root/client/token fields, OneDrive root/client/token
  fields, Aliyun Drive root/drive-type/client/token fields, Gitee snippet
  endpoint/id/token, and GitHub Gist id/token. Saving uses the
  legacy-compatible redb settings document and preserves encrypted secrets
  unless a replacement value is typed.
- AI settings now have a native domain model matching the legacy schema v3
  shape: provider profiles, provider credentials, model items, custom actions,
  request defaults, mode, and agent command policy. Native storage reads and
  writes the embedded `settings/default.ai` document, normalizes legacy v2/v3
  data, encrypts API keys through the same `master.key` credential hierarchy,
  and preserves masked secrets on save.
- The GPUI Settings panel now exposes an AI configuration section for enabled
  state, active provider profile, Ask/Agent mode, agent command policy, model,
  local/SSH background execution, base URL, API key replacement, and stored AI
  session/message/audit counts.
- The AI Settings panel can discover OpenAI-compatible models through a native
  `zed-reqwest` HTTP binding against the active provider Base URL. Discovery
  runs off the GPUI thread, uses the configured request user-agent and bearer
  token when present, merges discovered model items by id, and keeps changes in
  the settings draft until saved.
- AI history and audit storage now use native `ConnectionStore` APIs for the
  legacy `settings/doc/ai-history` and `settings/doc/ai-audit` documents,
  including user/assistant message append, session listing/deletion, audit log
  append/list, legacy missing-reasoning defaults, and the same session/message/
  audit retention limits as the Tauri app.
- The AI prompt/redaction/parser layer is now native domain code: localized
  system prompts, terminal context prompt builders, agent observation prompts,
  sensitive-value redaction, command-card JSON parsing, `<think>` extraction,
  assistant text extraction, and reasoning-channel promotion are covered by
  unit tests.
- AI model resolution and discovery parsing are native domain code: request
  model/default-model selection, provider inference, credential validation,
  DeepSeek `-none` model-name mapping, OpenAI-compatible `/models` URL joining,
  response parsing, and discovery deduplication are covered by unit tests.
- OpenAI-compatible chat completion runtime foundations are now native:
  `/chat/completions` URL joining, request body construction with system
  prompt, bounded history, legacy prompt builder output, model-name mapping,
  non-stream response parsing, reasoning extraction, and a `zed-reqwest`
  native HTTP execution entry point are in place.
- Anthropic and Gemini Ask execution now have native non-stream adapters:
  Anthropic uses the Messages API with legacy prompt/history mapping and
  thinking/text response parsing; Gemini uses `generateContent` with system
  instruction, user/model history mapping, and thought/text response parsing.
- The Workspace right panel now includes a compact GPUI AI Ask surface. It
  sends native streaming Ask requests for OpenAI-compatible, Anthropic, and
  Gemini providers off the render thread, builds terminal context from the
  active SSH session and recent terminal output, applies legacy redaction before
  provider execution, persists user/assistant messages into the native AI
  history store, parses command-card output, and refreshes AI usage counters on
  completion.
- Parsed AI command cards are now visible in the GPUI Ask panel with title,
  risk label, command preview, explanation preview, and separate Save/Insert/Run
  actions. Save writes a legacy-compatible quick-command record into
  `settings/doc/quick-command` with AI source metadata and audit logging. Insert
  writes the command text to the active terminal session without submitting it;
  Run submits the command with a trailing newline. Insert and Run record native
  AI audit entries and refuse to run when no terminal session is active.
- The Workspace right panel now includes a compact Quick Commands surface backed
  by the legacy `settings/doc/quick-command` document. It loads saved commands
  and categories at startup, sorts by pinned/use count/update time, supports
  manual refresh, and can insert or run a saved command against the active
  native terminal while incrementing the legacy use counter.
- Command history now uses the legacy redb `command_history` table and key
  layout, including prompt cleanup, per-command de-duplication, use counts, and
  portable snapshot import/export. The Workspace right panel includes a compact
  Command History surface for refresh, insert, run, and delete actions; native
  submitted command buttons record history when they send a trailing newline.
- Command suggestions now have a native domain fuzzy-search foundation over
  command history and quick commands, returning the legacy-shaped
  `command/score/indices/source/display` result model. The Workspace right
  panel exposes this as a compact Command Search surface with Insert/Run actions
  for matched history or quick-command results.
- Session recording now has a native `nyaterm-session` recording manager that
  captures terminal input/output, strips terminal control sequences, suppresses
  common echoed input, writes timestamped/labeled or plain log files, preserves a
  bounded in-memory transcript, saves transcript snapshots, and searches
  captured history with literal/regex/whole-word options. GPUI wires Start,
  Stop, Save, transcript search, legacy recording settings, and auto-start paths
  into the active native session without touching Tauri commands.
- The settings, history, audit persistence, prompt, redaction, parser, model
  resolution, and discovery parsing side of the AI migration is now native.
  Live OpenAI-compatible model discovery and provider-native Ask streaming for
  OpenAI-compatible, Anthropic, and Gemini providers are wired into GPUI. Agent mode
  now uses the legacy execute_command/final_answer protocol prompts, parses the
  fallback JSON protocol, applies local command-risk assessment over model risk,
  and routes the proposed command through the existing GPUI command-card
  approval/insert/run path; ConfirmEach requires manual action, Smart can
  auto-run commands within the configured risk threshold, and Auto follows the
  saved policy. Foreground observe/continue loops now watch the active terminal
  after an Agent command runs. Supported POSIX/PowerShell/Cmd profiles use
  marker-based capture to hide command echo/markers and feed precise command
  output plus exit code into the next Agent step; send-only profiles retain the
  stabilized-output fallback. SSH connections persist the Agent execution
  profile, and when `agent_background_execution_enabled` is set, Agent command
  cards run through native local shell execution or SSH exec channels and feed
  stdout/stderr/exit code into the next Agent step without injecting text into
  the terminal. Local background execution preserves the active local session's
  working directory in `SessionInfo` and the Agent prompt context. Native
  OpenAI-compatible, Anthropic, and Gemini Agent requests now attach provider
  native `execute_command` and `final_answer` tool schemas, stream provider
  tool-call deltas, fold them into unified tool calls, surface Agent tool-call
  streaming status in GPUI, render a compact recent Agent step timeline, and
  prefer tool-call output before falling back to the legacy JSON protocol. The
  GPUI AI panel can cancel active Ask/Agent requests, foreground captures, and
  pending Agent observations by marking the worker generation stale and
  discarding late events.
- Native SSH shell X11 forwarding requests are wired through `russh`, including
  DISPLAY resolution, local X server TCP/Unix-socket fallback, `xauth` cookie
  hydration, MIT-MAGIC-COOKIE-1 rewrite, and GPUI terminal notices when the
  remote request or local display connection fails.
- Remote Docker management now has a native `nyaterm-session` service using the
  same SSH authentication, ProxyJump, known_hosts, and prompt path as remote
  processes. It ports the legacy Docker tabular and JSON parsers for overview,
  containers, images, volumes, networks, compose projects, container details,
  stats, logs, container actions, compose actions, and prune commands. GPUI
  exposes a dense Docker page with refresh, container start/stop/restart/logs,
  prune, resource counts, and recent log preview for the active SSH session.
- The Tauri updater plugin dependency now has a native GPUI replacement for
  update checks: `nyaterm-native` queries GitHub latest-release metadata off the
  render thread, compares numeric versions against `CARGO_PKG_VERSION`, and
  surfaces availability, release date, notes, and release URL in Settings.
- The legacy tray action surface now has a GPUI-native in-window replacement:
  the Workspace Command Center exposes new-connection navigation, active session
  focus, cloud sync push/pull/history entry points, settings, update checks, and
  migration status without Tauri tray/menu APIs.
- Native dialog/process replacement audit is clean for the new workspace: file
  and directory picking, path reveal, diagnostics export, config import/export,
  portable snapshots, transfer paths, recording paths, keyword-highlight import,
  SSH host-key prompts, credential prompts, duplicate prompts, cloud/snapshot
  password prompts, updater checks, local shell execution, remote SSH exec, and
  X11 `xauth` probing are handled through GPUI, Rust services, or explicit
  native subprocess calls without Tauri commands, plugins, invoke handlers, or
  WebView dependencies.



## UI Shell Parity Progress

- The native shell now keeps the terminal workspace permanently centered, matching the Tauri layout model of activity bars + left/right side panels around a fixed terminal surface.
- Activity bar items map to the legacy left/right panel set: File Explorer, Network, Security/Auth, Sync/Backup, Settings on the left; Saved Connections, AI Assistant, Active Sessions, Command History, Resource Monitor, Process Manager, Docker, plus Quick Commands / Command Send / Recording / Lock on the right.
- Side panels now host full native views instead of summary cards for transfers, tunnels, connections, remote stats/process/docker, AI, command history, recording, security/auth inventory, and sync history.
- Settings open as a full-screen native page with a Back action and keep the terminal workspace state when closed.
- Workspace chrome has been tightened toward the legacy look: GitHub Dark surface colors, tab strip above the terminal without an extra debug toolbar, empty-workspace quick actions, and denser connection list/editor UX in the right panel.

## Optional Platform Polish

- OS-level tray/menu integration is now optional platform polish because the
  tray actions have a GPUI Command Center substitute.

## UI shell parity (2026-07-11)

- Center remains terminal workspace; left/right side panels wrap it (no page-switch for panels).
- Side panels support drag-resize handles (left 160–720, default 256; right 200–720, default 288).
- Activity bar uses compact glyphs instead of two-letter codes.
- Security / Auth side panel: Keys + OTP tabs with list/add/edit/delete, OTP code generate+copy, file browse for keys.
- Domain storage: `save_ssh_key` / `delete_ssh_key` / `save_otp_entry` / `delete_otp_entry`.
- File Explorer inner header densified under shared panel chrome.

## Security + layout (continued)

- Security / Auth tabs: Keys, Passwords, Credentials, OTP with list/add/edit/delete.
- Passwords/Credentials reveal+copy via domain decrypt APIs.
- Domain: `SavedPassword` / `SavedCredential` + list/save/delete/decrypt APIs.
- Panel widths persisted to settings `ui.left_width` / `ui.right_width` (defaults 256/288).

## Secret unlock + panel layout persistence

- Security panel secret unlock footer (Lock/Unlock) with master-password dialog.
- Password/credential reveal, OTP code, and secret editors require unlock when master password is set.
- UI layout persistence now includes left/right widths, active panel ids, and collapsed flags under `ui.*`.
- Active panels restored on startup/import via `NavItem::from_persistence_id` (Tauri-compatible ids like `fileExplorer`).

## Activity bar layout parity

- Activity bar items are driven by persisted `ui.activity_bar_layout` zones (`left_top/left_bottom/right_top/right_bottom`) with Tauri-compatible ids.
- Right-click an activity item to reorder (Up/Down), move across zones, or toggle labels.
- Labels mode widens the bar and shows compact captions under glyphs.
- Layout is saved with panel widths/active panels via `save_ui_layout_settings`.

## Multi-open panel stack

- Settings > Appearance: toggle multi-open side panels (`appearance.panel_multi_open` / `ui.panel_multi_open`).
- In multi-open mode, activity bar clicks toggle panels into left/right stacks ordered by activity bar layout.
- AI Assistant remains exclusive overlay (does not join stack), matching Tauri `EXCLUSIVE_PANEL_IDS`.
- Stacked panels are vertically split with drag resize handles; weights persist in `ui.panel_stack_sizes`.
- Open panel lists persist as `ui.left_open_panels` / `ui.right_open_panels`.

## Workspace split resize + denser chrome

- Terminal dual-pane splits support drag-resize handles (H/V) and +/- ratio controls in the tab strip.
- File Explorer control strip and connection list/toolbars densified for side-panel use.
- Shared `transfer_input` height reduced to 36px for denser forms across panels.

## 2026-07-11 FE stack parity
- File Explorer now stacks browser (flex-1) + resizable File Transfer queue (default 180px, persisted as `ui.transfer_height`).
- Removed page-like transfer control chrome from FE; densified toolbar/path/list/queue to match Tauri AppPanelContent.

## 2026-07-11 Connections + FE chrome parity
- Saved Connections: Tauri-like filter strip with icon actions (+ / folder / temp SSH / sort / more menu for import-export).
- Connection rows densified to ~30px with double-click connect; group tree no longer card-framed.
- FE: hide toolbar/path when no SSH; Tauri-style empty states for offline/unsupported/empty/search-miss.
- Panel headers densified to 32px; security list rows slightly tighter.

## 2026-07-11 Connections context menus
- Right-click connection: Connect / Edit / Rename / Copy / Delete (multi-select connect when applicable).
- Right-click group: New connection / New folder / Open all / Rename / Delete.
- Row actions reveal on hover (or while selected), matching Tauri hover chrome.

## 2026-07-11 Recursive workspace splits
- Replaced dual-pane-only `WorkspaceSplitState` with recursive `WorkspacePaneNode` (Leaf | Split), matching Tauri `PaneNode` / `SplitPane`.
- Nested H/V splits: each split has id, direction, ratio; focused split resizes via handle and tab-strip −/+.
- Repeated H/V split on the active leaf grows a nested tree (duplicate session into new leaf).
- Unsplit collapses around the active leaf (closes sibling panes) instead of only supporting one level.
- Active leaf chrome ring + click-to-focus panes.

## 2026-07-11 Connections drag-and-drop
- Connection rows and group headers support GPUI drag/drop (same pattern as session tabs).
- Drop connection on another connection: reorder within/target parent (`sort_order` rewritten).
- Drop connection on group header: move into that group (append).
- Drop group on group: reorder among same parent or use list background to ungroup/root.
- Drop on list background: move connection/group to ungrouped root.
- Persist via domain `save_connection` / `save_group` with updated `group_id`/`parent_id` + sequential `sort_order`.

## 2026-07-11 Activity bar drag-and-drop
- Activity bar entries support GPUI `on_drag` / `on_drop` with `ActivityBarDragPayload` + drag preview.
- Drop on an item inserts before it; end-of-zone hit targets append.
- Cross left/right moves clear open-panel state on the source side (Tauri parity).
- Right-click context menu (move zones / up-down / labels) retained.

## 2026-07-11 Empty workspace + shell chrome
- Empty workspace matches Tauri `EmptyWorkspaceState`: large faded logo mark + primary-colored action rows with shortcut key chips.
- Actions: Temporary SSH Link, Open Chat, Show All Commands, Switch Terminal (resolved via keybindings).
- Panel headers densified to Tauri-like min-height with title + meta baseline layout.

## 2026-07-11 Title menubar + Active Sessions
- File / View / Terminal / Help are real dropdown menus (new session, import/export, zoom, sidebars, settings, splits, sync groups, clear, update check, about).
- Active Sessions is a dedicated dense panel (search strip + compact rows with type badge and icon actions) instead of workspace summary cards.

## 2026-07-11 AI panel + Command History density
- AI Assistant panel is no longer a stacked card + command search; full-height shell with mode toolbar (Ask/Agent), model label, new-chat / settings, scroll transcript (response + agent steps + command cards), and bottom composer.
- Command History matches Tauri: dense mono list with › prefix, click to insert, ▶ to run (no card chrome / 8-item cap).

## 2026-07-11 Remote panels densify (Docker / Process / Stats)
- Docker: side-panel shell with dense search toolbar + icon refresh/prune, flex list body, compact logs footer (removed page section_header + metric grid).
- Process Manager: dense search toolbar, compact sort strip, scrollable table (removed summary cards / page chrome).
- Resource Monitor: host summary toolbar + scrollable gauges/lists (removed page header + duplicate status refresh card).

## 2026-07-11 Docker container rows
- Containers list uses Tauri-like ~66px dense rows: left state accent, name + state badge, mono image/id line.
- Actions moved into ⋮ dropdown (Logs / Enter / Start / Stop / Restart / Kill / Remove) instead of a row of text buttons.
- Click row opens details; kill/remove still go through confirm dialog.

## 2026-07-11 Process overflow menu + full signals
- Process rows use ⋮ overflow menu (Copy PID / Copy Command / TERM / HUP / STOP / CONT / KILL) matching Tauri ProcessActionMenu.
- `process_menu_pid` tracks open menu; cleared on row select, refresh, and menu actions.
- Sort strip includes Process/Command in addition to CPU/Mem/PID/User.
- STOP/KILL still go through existing signal confirm panel.

## 2026-07-11 Docker Compose dense rows + menus
- Compose projects: ~74px dense rows with chevron expand, status pill, config path, ⋮ menu (Up/Restart/Down).
- Compose services: ~58px dense rows with status + container summary and ⋮ menu (Logs/Enter/Up/Stop/Restart).
- Removed wide inline action button strips; menus mirror container-row pattern.
- `docker_compose_menu_id` cleared on Docker tab change.

## 2026-07-11 AI chat message bubbles
- AI transcript now renders user/assistant bubbles (role label, reasoning block, streaming indicator) instead of a single preview card.
- In-memory `ai_chat_messages` + `ai_streaming_assistant_id` filled on Ask/Agent start, stream deltas, finish, cancel.
- Empty state guides enablement / model setup / start conversation (Tauri-like).
- New chat clears message list; command cards + agent steps remain below transcript.

## 2026-07-11 Screenshot shell fidelity
- Activity bar short labels now use Tauri-like panel titles (Files/Net/Auth/Sync/AI/…); labeled width ~52px.
- Side panel headers use compact `panel_title()` (Connections, Files, AI Assistant, …).
- Saved connection rows densified to ~44px with endpoint + relative last-used meta under the name.
- Empty workspace: larger greyscale logo mark, Temporary Link label, Kbd chips with "+" separators.
- AI panel: history popover + settings/new actions, setup empty state steps, composer mode switcher + send/stop icon (Tauri AIAssistantPanel layout).

## 2026-07-11 Shell assets + connection details
- Bundled monochrome SVG assets (`crates/nyaterm-native/assets/icons/*`) loaded via `NyaTermAssets` + `Application::with_assets`.
- Activity bar uses SVG icons (Files/Network/Auth/Sync/Settings/Connections/AI/…) with glyph fallback.
- Empty workspace and titlebar use NyaTerm cat-face logo mark (faded) instead of plain "N"/green square.
- Connection hover shows Tauri-like detail tooltip (type/host/user/last/desc).
- Connection drag preview densified with accent icon + label.

## 2026-07-11 Connection DnD indicators + AI execution menu
- Connection rows show blue before/after drop lines via `on_drag_move` + `connection_drop_target`.
- Drop supports before/after reorder (`move_connection_after`) and group inside highlight ring.
- AI toolbar adds agent command execution menu (Confirm each / Smart / Auto) with immediate persist.
- Title bar window controls use cleaner glyphs (– □ ×).

## 2026-07-11 AI history groups + connection multi-select
- AI history popover mirrors Tauri: search field, Clear All, date groups (Today/Yesterday/Last 7 Days/Earlier).
- Connection multi-select chrome: selection strip (Open/Copy/Delete/Clear) when selected_count > 0.
- Connection click modifiers: plain replace, Ctrl/Cmd toggle, Shift range across visible list order.
- Empty workspace action rows align label column + shortcut chips (grid-like).

## 2026-07-11 Shell chrome density
- PanelHeader meta is dynamic: Connections count, AI model label, Active Sessions count.
- Connection rows match Tauri single-line icon+name (~34px) with hover actions; details stay in tooltip.
- AI empty states: disabled / setup steps with Open AI Settings / start conversation.
- Activity bar uses tighter gap and refreshed monochrome SVG icons.

## 2026-07-11 Connections tree order
- Folders first, ungrouped connections after a thin separator (no "Ungrouped" header).
- Background click clears selection; row/group mousedown stops propagation.

## 2026-07-11 Connection icons + AI command cards
- Connection rows resolve Tauri-like icon keys (`server*`, distros, docker, …) via SVG assets under `assets/icons/conn/`.
- Group headers use folder icon + chevron (no "Ungrouped" label).
- AI command cards match Tauri layout: title/risk, mono command block, explanation/effect/rollback, Insert/Copy/Save/Run.

## 2026-07-11 AI markdown + remote density
- AI transcript: strip `<think>` tags, collapsible-like thought block, lightweight markdown (headings/lists/code/quotes).
- Agent steps: left accent status bar + mono detail.
- Process rows densified to ~34px; Docker container rows to ~52px.

## 2026-07-11 Network panel densify + scroll
- Fixed missing `network_meta_chip` (compact label/value meta strip under tabs).
- Network body: tab strip (34px) + meta chips (28px) + search (34px) + flex scroll list.
- Tunnel/Proxy group headers densified to ~30px with count badge; rows use py-2 denser layout.
- Tunnel rows drop noisy auto/local/public chip clutter (auto badge only when enabled).
- Proxy rows drop raw id line; show credentials/ProxyCommand summary instead.
- Network tab buttons restyled closer to Tauri TabsTrigger segment control.

## 2026-07-11 Files + Connections density pass
- File Explorer path bar min-height ~26px (Tauri PathBar); search field densified to 26px.
- Connection rows forced to single-line ~34px (was 44px) to match Tauri ConnectionItem py-1.5.
- Tooltip anchor adjusted for denser connection rows.
- Network group headers use chevrons + open/cmd counts (less chip noise).

## 2026-07-11 Network Tauri layout pass
- Network body restructured to Tauri order: scroll padding → segment Tabs → config label + New Group/New item → compact search → grouped list.
- Tunnel rows: StatusBadge + switch toggle + icon actions (Edit/Move/Delete).
- Proxy rows: 3-line stack + icon actions (Tauri ProxyRow).
- PanelHeader meta shows tunnel/proxy count and Files entry count.
- Files mid-action strip removed; Tauri-like footer with totals + cwd/send icons.
- Shared `icon_button` matches ghost icon-sm (no permanent border).

## 2026-07-11 Remote process density
- Process summary cards compacted; sort buttons h-24; process rows ~38px (Tauri PROCESS_ROW_HEIGHT).
- Connections search strip height aligned to Tauri h-9 area.

## 2026-07-11 SVG toolbar / AI chrome pass
- Bundled File Explorer toolbar icons under `assets/icons/fe/*` (new file/folder, upload/download, delete, nav, search, star, sync, paste).
- File Explorer toolbar + footer use SVG ghost buttons (Tauri h-7 icon-sm style) with active state for search/auto-sync.
- AI header actions (history/settings/new) and send/stop use `assets/icons/ai/*` SVG buttons.
- AI history search uses search SVG; Connections header actions use flash/folder/add/more SVG icons.
- Shared `svg` re-exported from view prelude for panel modules.
- Network row Edit/Move/Delete use `assets/icons/net/*` SVG actions.

## 2026-07-11 Remote + shell density pass
- Process Manager: removed duplicate sort column; flat sortable header strip; SVG refresh/more; no double table header.
- Docker toolbar: refresh/prune SVG icons.
- PanelHeader meta: Processes count + Docker container count.
- Empty workspace action: default text + hover primary (Tauri-like), max-width row.
- AI composer: Ask/Agent mode segment control; denser mode_button styling.
- File path bar favorite uses star SVG.

## 2026-07-11 Connections/AI/Docker chrome pass
- Connection hover actions + group actions use SVG (connect/edit/delete).
- Multi-select strip densified (30px, blue accent).
- Docker tabs: flat segment style like Network tabs.
- Stats refresh SVG; AI empty states use `icons/ai.svg`.
- Title bar height 36px; activity labels 8px; menu buttons denser.
- AI message bubbles tighter padding.

## 2026-07-11 Files upload menu + shell chrome pass
- File Explorer toolbar matches Tauri: New File/Folder, **Upload dropdown** (files/folder), Download, Delete, Go Up, Refresh, Search.
- Upload uses popover menu (`TransferBrowserUploadMenuState`) instead of two always-visible buttons.
- Search expands as absolute overlay on the toolbar strip (primary border), not a second batch-action row.
- Toolbar no longer duplicates footer cwd-sync / favorite / history controls.
- Footer densified (~28px) with item count + total file size + 24px cwd/send icons.
- Activity bar lock/recording use SVG assets (`lock.svg` / `record.svg`) instead of emoji/glyphs.
- Security / Auth: full-width 4-col segment tabs (Keys/Passwords/OTP/Credentials order) under PanelHeader; header meta shows active-tab count.

## 2026-07-11 Network modal dialogs
- Tunnel/Proxy editors, group editor, and delete confirms moved from inline banners to Tauri-like modal shells (`network_modal_shell` dimmed backdrop + centered card).
- Dialog footer uses Cancel/Save action row with top border (ActionFooter-like).
- Network panel root is `relative` so modals cover the full panel body.

## 2026-07-11 Connection dialogs + shared modal chrome
- Extracted shared `modal_dialog_shell` / `modal_dialog_footer` for Tauri Dialog-like overlays.
- Network modals now wrap the shared helpers.
- Connections: New/Edit connection, group editor, and delete confirms use centered modal shells instead of bottom-of-panel banners.
- Security list rows densified (tighter gap); tab strip padding matches Tauri `px-3 pt-3`.

## 2026-07-11 Quick Commands bottom chrome
- Bottom Quick Commands panel drops page-like title/metrics cards.
- Compact header strip: title + count, search field with SVG, denser view/sort mode chips, Add/Import.
- Category sidebar narrowed; command rows scroll in remaining height.

## 2026-07-11 History + status bar density
- Command History rows match Tauri mono list more closely (no play glyph; right-click runs, click inserts).
- Status bar densified to ~22px with 10px chips.

## 2026-07-11 Recording + Command Send densify
- Bottom Command Send: ~220px compact header strip, denser control/editor/footer chrome (Tauri SendCommandPanel shape).
- Recording: session search filter + dense session rows with SVG record/stop/save actions; header meta = session count.

## 2026-07-11 Sync / Backup History panel parity
- Side Sync/Backup panel drops page-like cards; Tauri-like status strip (dot + state + provider + refresh).
- Conflict card + Push/Pull/Settings strip; full scroll history list.
- History rows densified: status dot, kind/status colors, summary, trigger/provider/duration meta, expandable details.
- PanelHeader meta shows history count.

## 2026-07-11 Command Send labeled controls
- Bottom Command Send title row matches Tauri (title + session kind + target + Hide).
- Controls reworked into labeled bordered groups: Data, Mode, Count steppers, Interval steppers, serial EOL cycle.
- Editor uses mono placeholder; footer denser with Clear / Send / Send ↵.

## 2026-07-11 Settings shell densify
- Settings header densified to 36px with ghost Back control.
- Sidebar narrower (220px), uppercase category labels, soft primary nav items (no green borders).
- Content pane drops heavy outer card; group/title strip + scroll body like Tauri SettingsPage.

## 2026-07-11 Settings form density
- Shared `settings_form_section` / `settings_form_row` / `settings_switch` / `settings_choice_chip` primitives (Tauri SettingSection/Row/Switch).
- General: language chips + switch rows (startup restore / confirm close) instead of metric cards.
- Appearance: theme/font/X11 as form sections with chips and font-size steppers.
- Interaction: multi-open, clipboard/mouse switches, suggestions, encoding, word separators, tab mouse actions as dense rows.

## 2026-07-11 Terminal + Transfer settings form density
- Terminal General rewritten to SettingSection/Row switches + scrollback/keepalive steppers (no feature cards).
- Transfer top section: download path actions, ask-save switch, duplicate policy chips, queue snapshot row.

## 2026-07-11 Security + Search settings form density
- Security: master password status, screen lock switch/idle steppers, host-key policy chips.
- Search: buffer/history mode chips + case/regex/word switches; command-search catalog summary.

## 2026-07-11 AI / Translation / Config Backup form density
- AI General: enable/redaction/history switches, provider chips, credential inputs, agent mode chips, limit steppers.
- Translation: provider chips + per-vendor credential sections.
- Config Backup: export/import rows for JSON, .nya, encrypted .nya.

## 2026-07-11 Command Send progress + stop
- Mid-send progress card with completed/total units, current round, and percent bar.
- Send/Send↵ buttons become Stop while a multi-round send is active; cancel uses AtomicBool.

## 2026-07-11 AI Models + Cloud Sync header densify
- AI Models: compact model/credential rows with status dots; catalog summary + provider chips + credential grid (no metric cards).
- Cloud Sync header: enable switch, provider chips, remote root form rows.

## 2026-07-11 Cloud Sync body + remaining settings form density
- Cloud Sync provider fields (WebDAV/S3/Drive/OneDrive/Aliyun/Gitee/GitHub) wrapped in settings form sections.
- Sync actions (local/provider push-pull) and recent history use dense form sections.
- Diagnostics/Updates rewritten to form rows (export, logs, update check).
- AI Rules: max file size, agent step timeout, smart risk chips, dense action lists.
- Transfer Editor/Advanced/Recording rewritten to SettingSection/Row switches + steppers.

## 2026-07-11 Keybindings + Keyword Highlights densify
- Keyboard shortcuts: registry summary form section + per-category dense rows (key mono chip, status, Record/Save).
- Keyword highlights: enable/wrap switches + import row (no metric cards).

## 2026-07-11 Command Send multi-target
- Target chips: Current vs All compatible sessions (shell vs serial compatibility).
- Runtime fan-out writes each unit to resolved target session ids; header shows scope label.

## 2026-07-11 Command Send hex ASCII side pane
- Hex data mode shows mono editor + compact ASCII preview column (Tauri dual-pane step).

## 2026-07-11 Command Send sync-group targets
- `SendCommandTarget` supports Current / AllCompatible / Group(id).
- Group targets resolve enabled sync groups, skip paused members, filter by serial/shell compatibility.
- Target control shows Current, All, and up to 4 group chips with member counts.

## 2026-07-11 Process/Docker densify pass
- Process toolbar shows filtered/total/users + top CPU/MEM mono summary.
- Expanded process details compacted: inline chips, nice steppers, SIG buttons on one dense strip.
- Docker container rows tightened (48px, softer title weights).

## 2026-07-11 Command Send hex dual-pane polish
- Hex editor shows Tauri-style spaced uppercase pairs (double space every 4 bytes).
- Preview column: byte count, 4-byte guide marker summary, ASCII preview.

## 2026-07-11 Docker Compose row densify
- Project rows 74→60px, service rows 58→48px.

## 2026-07-11 Resource Monitor densify
- Gauge/summary cards compact: 44px ring value, dense mono, thinner padding.
- Stats body grids gap_2; system/load cards use compact dark chrome.

## 2026-07-11 Command Send infinite count + hex guide marks
- `send_command_count: Option<u32>` with ∞ (None); stepper 1→∞ and ∞→1.
- Infinite send loop until Stop; indeterminate progress pulse + round/unit labels.
- Hex editor overlay marks 4-byte group boundaries from formatted display.

## 2026-07-11 Process/Docker virtual windows
- Process list renders 80-row windows with Page up/down and hidden counts (Tauri virtual-list step).
- Search resets list offset; Docker containers cap first 60 with refine-search hint.

## 2026-07-11 Hex typing auto-format + process sort offset
- Shared `format_send_command_hex_display` in `send_command` crate module; hex key entry/backspace reformats pairs.
- Process sort toggles reset virtual list offset to top.

## 2026-07-11 Process/Docker wheel virtual lists + multi-line hex guides
- Process list: spacer virtual window + `on_scroll_wheel` updates `process_list_offset` (no native overflow alone).
- Docker containers: same pattern with `docker_list_offset`, search resets offset; row range footer.
- Hex Command Send: per-line 4-byte guide marks (`send_command_hex_guide_rows`) like Tauri `buildHexGuideRows`.

## 2026-07-11 File Explorer virtual list + process expanded height
- File Explorer: 30px-row spacer virtual window + wheel offset (`transfer_browser_list_offset`); search/sort/path reset.
- Process: spacer padding accounts for expanded details height when selected.
- Command History: dense rows with inline Run (right-click still runs).

## 2026-07-11 Hex editor chrome + Docker image window
- Hex editor: Tauri-like "HEX Editor" header strip + invalid flag; guide marks use `(group*13-1)` ch positions.
- Docker Images: first-80 window with refine-search footer; resource rows densified to 52px.

## 2026-07-11 Docker volumes/networks list windows
- Volumes and Networks tabs cap first 80 rows with refine-search footer (same pattern as Images).

## 2026-07-11 Process details fixed height for virtual list
- Expanded process details use fixed 112px shell so spacer math stays stable under selection.

## 2026-07-11 Docker container row Tauri height + meta
- Container rows restored to 66px with ports/status + created_at third line (Tauri density).
- Virtual list row slot 68px matches.

## 2026-07-11 Command Send default intervals
- Data/mode switches apply Tauri default intervals: Text/Line=1.00s, Char/Byte=0.02s, Hex/Packet=0.
- App default interval starts at 1.00s for Text/Line.

## 2026-07-11 Docker resource wheel virtual lists + hex guide scroll
- Images/Volumes/Networks: spacer + wheel virtual windows via `docker_resource_list_offset` (Compose keeps static scroll).
- Tab/search reset resource offset; range footer for large lists.
- Hex guides translate with `send_command_hex_scroll_y` on wheel (Tauri hexScroll.top step).

## 2026-07-11 Security Auth + Network densify
- Security Auth Keys/Passwords/Credentials/OTP: fixed-height dense rows with trailing actions (Tauri list density).
- Network panel scroll body padding/gap tightened (p_2/gap_2).

## 2026-07-11 Process table column alignment + accents
- Sort header uses the same 6-col grid as rows (Process/PID/CPU/Mem/User/menu) with numeric right-align.
- Process rows: left load/selection accent bar; mono numeric PID/CPU/Mem.
- Quick Commands / Recording panels densified slightly.

## 2026-07-11 Resource Monitor + Active Sessions densify
- Resource Monitor section titles + dense_capability_line mono values.
- Active Sessions rows fixed 44px height.

## 2026-07-11 Docker container details densify
- Details empty state, key/value lines, mounts, and networks use denser mono chrome matching panel system.

## 2026-07-11 Process responsive display modes
- `ProcessDisplayMode` from `right_panel_width` (compact <320, narrow <430, medium <540, else wide).
- Compact: 62px rows (command + PID/CPU), no table header.
- Narrow hides Mem; non-wide hides User; sort keys coerce when columns hidden.
- Details height scales by mode; virtual list row math follows mode.

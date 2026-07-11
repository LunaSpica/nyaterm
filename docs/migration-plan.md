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

## 2026-07-11 Process mode sort order + Sync history strip
- Apply ProcessDisplayMode sort-key coercion before sorting the filtered list.
- Sync Backup History status strip densified to 36px toolbar chrome.

## 2026-07-11 AI markdown GFM parity
- `parse_markdown_blocks` gains GFM pipe tables, multi-line quotes, thematic breaks (`---`), and `+` bullets.
- New `parse_inline_markdown` + `StyledText` highlights: bold/italic/bold-italic, inline code, links, strikethrough.
- AI transcript renderer shows bordered tables and denser list/code chrome closer to Tauri `MarkdownContent`.

## 2026-07-11 Quick Commands list/compact Tauri density
- Compact mode: 32px single-line rows (label + mono command), ghost icon actions, no card border stack.
- List mode: ~44px single-row stack (label over command) with trailing badge + icon actions (was multi-row card).
- Tile trailing actions use icon buttons; command list gap tightened to `gap_1`.

## 2026-07-11 Command Send hex dual-axis guide scroll
- Track `send_command_hex_scroll_x` + `send_command_hex_scroll_y` (Tauri `hexScroll.left/top`).
- Wheel handler consumes both axes; guide overlay translates with `-scroll_x/-scroll_y`.
- Clamp uses approximate viewport (5 lines × 48 chars) against formatted hex display size.

## 2026-07-11 Network tunnel/proxy row densify
- Tunnel/Proxy rows: px_2, gap_2, 12/11/10px type stack with overflow clip (Tauri side-panel density).
- Empty group states and config label use compact 11–12px chrome.

## 2026-07-11 AI command cards in transcript bubbles
- Assistant message bubbles render full `AICommandCardView`-like cards (title, risk, mono command, explanation/effect/rollback, Insert/Copy/Save/Run).
- Card actions resolve by card id across live `ai_command_cards` and historical message cards.
- Thought/reasoning content uses the shared markdown renderer; card chrome densified (`p_2`, taller command body).

## 2026-07-11 Connections tree virtual list
- Flatten expanded group tree into fixed-height rows (group header 28px, connection 34px, separator 10px).
- Spacer + wheel virtual window via `connection_list_offset` (viewport 36 + overscan 8).
- Search/sort/group expand reset offset; group headers use header-only section chrome so nested rows stay virtualized.

## 2026-07-11 AI Agent Step cards (Tauri AgentStepView)
- `AiAgentStepView` stores thought/command/observation sections inferred from upsert call sites.
- Steps render as collapsible cards: thought toggle, left-accent SHELL block, optional output expand, running footer.
- Expand state tracked in `ai_agent_thought_expanded` / `ai_agent_output_expanded` (cleared with step list).

## 2026-07-11 Appearance theme palette application
- `ThemePalette` for github-dark / github-light / catppuccin.
- Root shell + title bar apply palette bg/surface/border/text; font family follows appearance.font_family.

## 2026-07-11 Theme shell chrome expansion
- Panel headers take live `ThemePalette` (surface/border/text_muted).
- Status bar, activity bar, left/right side panels, and main surface use palette bg/surface/border.
- Terminal canvas font family/size follow appearance settings (`terminal_font_family` / `terminal_font_size`).

## 2026-07-11 ThemePalette Tauri field parity + workspace chrome
- Expand `ThemePalette` to Tauri surface tokens: section_header, hover, input, text_dimmed, success/warning/danger.
- Align github-light / catppuccin(-mocha) hex values with `nyaterm-tauri/src/lib/themes.ts`.
- Panel headers use section_header; tab strip, empty workspace, bottom panel, security-auth lists use palette.

## 2026-07-11 Activity chrome + Connections palette pass
- Activity bar entry buttons and title menus use live ThemePalette (accent/success/hover/muted).
- Saved Connections search strip, root surface, selection strip, and row hover/selected states use palette tokens (Tauri toolbar + ConnectionItem density preserved).

## 2026-07-11 AI Assistant shell palette + Connections extras
- AI panel root/action strip/composer borders and muted text follow ThemePalette.
- Connections more-menu and group-header hover/drop accents use palette tokens.

## 2026-07-11 Shared ThemePalette + major panel shells
- Extract `ThemePalette` to `crates/nyaterm-native/src/ui/theme.rs` for shared use.
- Shared components: `empty_panel`, `mode_button`, `icon_button`, `section_header` take live palette.
- Shell theming: Settings page, Docker/Process/Stats toolbars, Transfers root + path bar + queue, Quick Commands sidebar, Network/Tunnels surface.

## 2026-07-11 transfer_input + panel interior palette pass
- `transfer_input` takes live `ThemePalette` (border/input/text) across AI, Network, Process, Docker, Settings, Quick Commands, Transfers.
- Process/Docker/File Explorer/Tunnels toolbars deepen palette application; network tab chips themed.

## 2026-07-11 Settings form helpers ThemePalette
- `settings_form_section` / `settings_form_row` / `settings_switch` / `settings_choice_chip` / `settings_category_header` take live `ThemePalette`.
- All Settings tabs (workspace/AI/terminal/transfer/security/sync/translation) inject `let palette = self.theme_palette()` and pass it through.
- Free helper `ai_action_list` takes palette and themes action catalog rows (border/input/text/success).
- Choice-chip selected fill uses theme-aware accent tint (dark / light / catppuccin); switch on/off uses accent/border + hover tokens.

## 2026-07-11 Shared control ThemePalette expansion
- `small_button`, `capability_line`, `session_info_row` take live ThemePalette.
- Status bar controls (`status_bar_label` / `status_bar_button`), modal shell/footer, `inspector_card`, and `metric` themed.
- Resource monitor `dense_capability_line` and transfer `queue_metric` take palette tokens.
- Call sites across shell, overlays, transfers, remote panels, and settings inject `theme_palette()` (methods) or dark fallback (free helpers).

## 2026-07-11 Inspector + sidebar interior palette densify
- AI inspector interiors (transcript empty state, agent step cards, command cards, execution mode menu/history popover, message bubbles) replace github-dark hardcodes with ThemePalette.
- Right-side command center/search panels and disabled inspector panel themed.
- Left sidebar panels (connections/network/transfers/security auth/session rows/nav buttons) densify palette borders/text/surfaces.
- Quick Commands panel interior surfaces follow live theme.

## 2026-07-11 Remote Process/Docker/Stats interior palette densify
- Process table/menu/summary/detail chrome, resource gauge cards, and compact SVG action buttons use ThemePalette.
- Docker details lines/panels and tab bar/chips follow live theme.
- Stats view system cards and CPU core rows replace github-dark hardcodes.

## 2026-07-11 Transfers + Connections interior palette densify
- Transfer browser toolbar/footer buttons, path bar/history/favorites, entry rows, queue, and overlays (create/delete/move/editor/properties/context) map chrome hardcodes to ThemePalette.
- Connections view: rows, selection strip, editor/delete confirm panels, context menus, kind/toggle chips, and icon actions themed.
- Shared `view_widgets` connection/tunnel compact rows and tab actions densified; bottom send-command chips themed.
- Semantic status colors (job state dots, risk pills, close-red) intentionally retained.

## 2026-07-11 Full Tauri theme catalog + Network/shell densify
- Expand `theme_palette` to all 17 Tauri theme ids from `nyaterm-tauri/src/lib/themes.ts`.
- Interactive accent uses Tauri `link`/`focusRing` (github-light blue, not green brand accent).
- Settings Appearance lists every theme via `APPEARANCE_THEME_IDS` chips; legacy `catppuccin` normalizes to `catppuccin-mocha`.
- Densify Network tunnel/proxy rows/editors, title/status bar extras, terminal search/canvas chrome, session overlays, translation chips, and quick-command helper fields.

## 2026-07-11 Overlay/Docker/workspace residual palette densify
- Quick command editor/sync groups/recording/quick-switch overlays, session tab strip chips, host-key/credential prompts.
- Docker containers/compose menus & rows; Settings AI/workspace residual hardcodes; sidebar residual chrome.

## 2026-07-11 Terminal palette + residual chrome sweep
- `ThemePalette` gains `terminal_bg` / `terminal_fg` from Tauri terminal colors; terminal canvas uses them.
- `terminal_line_element` search/active match chrome takes palette; keyword highlight span bg uses surface.
- Broad residual densify: Settings terminal/transfer forms, overlays, Docker resource rows, transfer widgets, formatting status colors mapped where chrome-like, sidebar/session chrome.

## 2026-07-11 Sidebar helpers semantic chrome densify
- Quick command color tags map danger/success/accent/warning via palette (dark fallback for free helpers).
- Generic quick-command icons (terminal/code/server/folder/AI/bolt) use palette status colors; brand icons remain brand-colored.
- Sidebar residual security/session chrome (banners, unlock, nav hover) densified.
- Formatting docker/cloud-sync status colors prefer palette tokens (free helpers still dark-fallback until caller-themed).

## 2026-07-11 Prompt banners + Network status semantic densify
- Host-key/credential/snapshot prompt banners use warning/danger/input palette tokens.
- Tunnel/proxy status pills, switches, editor options, and section chrome densified.
- Process usage colors, transfer job status helpers, lock screen residual, and connection kind chips mapped.

## 2026-07-11 Live palette free helpers + wallpaper settings shell
- Status helpers (`docker_state_color`, cloud-sync status/kind colors, `quick_command_color`, network modal shell/footer) take live `ThemePalette` instead of hard github-dark fallbacks.
- Domain `AppSettingsSummary` loads/saves Tauri wallpaper fields: path, fit, image opacity, content opacity.
- Settings Appearance adds Background image section (browse/clear/fit/opacity steppers).
- Root shell layers wallpaper `img` under chrome with content opacity when a path is set.

## 2026-07-11 Shared chrome free helpers live ThemePalette
- `view_widgets` free helpers (logo/window controls, inspector status/compact rows, empty workspace, tab actions, stats bars, service status, markdown, toolbar SVG, connection type icon, compact connection/tunnel rows, cloud sync history row) take live `ThemePalette` as first arg.
- Process Manager free helpers (`process_table_*`, menus, resource gauge/summary cards, `usage_color`, compact remote SVG buttons) take live palette; Process/Stats views pass `self.theme_palette()`.
- Connections local icon/menu free helpers themed; stats CPU core rows use live palette tokens.
- Remaining dark fallbacks concentrated in Transfers/Tunnels/Docker free helpers and some settings/panels.

## 2026-07-11 Transfers Network Docker free helpers live ThemePalette
- Transfer browser toolbar/footer/entry rows/queue helpers, progress bar, and overlay menus take live `ThemePalette`.
- Tunnel/proxy section/editor/status free helpers take live palette; residual small_button dark fallbacks cleared in those modules.
- Docker tab/details/resources/compose/container free helpers use palette params or `cx_theme_palette(cx)`.

## 2026-07-11 Residual free-helper ThemePalette sweep
- Title menus, inspector disabled panels, bottom send-command chips/steppers themed.
- Connections editor chips/fields, Network icon actions, Settings AI/security/transfer/terminal free helpers themed.
- Sidebar security/session free helpers, quick-command icon/editor helpers and editor overlay choices themed.
- Zero remaining `theme_palette("github-dark")` hard fallbacks in `nyaterm-native` UI sources (brand/status hardcodes may remain intentionally).

## 2026-07-11 Residual chrome hardcode cleanup
- Map remaining github-dark shell tokens (border/bg/muted) in migration, process/workspace views, panel resize handles, terminal actions, tunnels residual hover, and bottom send field to live ThemePalette.

## 2026-07-11 Terminal ANSI SGR + wallpaper ObjectFit
- `nyaterm-terminal` cells store SGR style (fg/bg 0..=15, bold, reverse); CSI `m` applies ANSI/bright and 256-index 0..=15.
- `ThemePalette` gains `terminal_cursor`, `terminal_selection`, and full Tauri 16-color `terminal_ansi` for all 17 themes.
- Terminal canvas paints styled spans with palette-resolved ANSI colors (bold promotes 0..=7 to bright).
- Wallpaper layer uses GPUI `ObjectFit` (`cover`/`contain`/`fill`/`none`); Settings chips match Tauri cover/contain/stretch/tile (`fill` aliases stretch).

## 2026-07-11 Terminal cursor + chrome densify
- Terminal line painter draws block cursor from screen cursor_row/col using `terminal_cursor`/`terminal_bg`.
- Search match rows use `terminal_selection` instead of surface/hover chrome.
- Remove non-Tauri debug toolbar (Start Local/Probe/Dup/etc) from active session terminal surface; keep empty-session bootstrap actions.
- Drop bottom timestamps/gpu debug pills; tighter line stacking (no inter-line gap); search bar docks at top-right.

## 2026-07-11 Session tab strip densify
- Active tab top accent bar (Tauri-like 2px indicator).
- Tab active/idle backgrounds map to ThemePalette hover/bg instead of hardcoded dark blues.
- Residual bottom quick-command chrome tokens themed.

## 2026-07-11 Cursor style + blink settings
- Domain `AppSettingsSummary` loads/saves Tauri `appearance.cursor_style` (`block|underline|bar`) and `cursor_blink`.
- Settings Appearance adds Cursor section (style chips + blink switch).
- Terminal painter supports block / underline / bar caret approximations using theme cursor colors.
- Event pump toggles `cursor_blink_on` (~530ms half-period) so blinking matches focused terminal caret.

## Terminal text selection (native)

- Mouse drag selects cells on the visible VTE grid with theme `terminal_selection` highlight.
- Double-click selects a word; triple-click selects a line.
- `interaction_copy_on_select` copies on mouse-up when enabled.
- Copy / Select All / Paste Selected shortcuts prefer the live selection.
- Hit-testing uses approximate monospaced cell metrics (`font_size * 0.62` × 18px line height) and the painted terminal output bounds.

## Terminal SGR truecolor / underline

- `CellStyle` stores `fg_rgb` / `bg_rgb` (CSI 38/48;2) and `underline` (CSI 4/24).
- 256-color indices beyond 15 are approximated to the 16-color ANSI set.
- GPUI paints underline via `.underline()` on span elements; reverse video swaps truecolor/index pairs.

## Terminal context menu (native)

- Right-click on the terminal surface opens an anchored context menu (Tauri TerminalContextMenu).
- When `interaction_right_click_paste` is enabled, right-click pastes instead of opening the menu.
- Selection-aware items: Copy, Find (prefill), Translate, Ask AI, Paste Selected.
- Always available: Paste, Find, Clear Screen, Clear All, Select All, More Actions.

## Terminal scrollback viewport

- VTE screen keeps scrolled-off rows up to `terminal.scrollback_lines`.
- Mouse wheel adjusts per-session `scroll_offset` (0 follows live output).
- Snapshots/paint/selection/cursor use `viewport_snapshot(offset)`; cursor hides when scrolled away.

## Terminal keyboard scrollback

- `Shift+PageUp/PageDown` page through local scrollback without sending CSI 5~/6~ to the PTY.
- `Shift+Home/End` jump to oldest history / live bottom.
- `Ctrl+Shift+Up/Down` scroll one line.
- Typing while scrolled in history snaps back to the live bottom before sending input.

## Terminal buffer search over scrollback

- Buffer find searches absolute scrollback + live screen via `all_lines()`.
- Next/prev match reveals the absolute line by adjusting `scroll_offset`.
- Highlight mapping converts absolute indices into the current viewport window.

## Recent parity slices (GPUI)

- Terminal context menu: copy/find/paste/clear/select-all, online search engines
  from `search.custom_engines`, AI terminal actions, translate selection.
- Domain settings: `SearchEngineConfig` + load/save under `search.custom_engines`.
- Settings → Search: edit online search engines (add/remove/toggle menu/name/url).
- Terminal scrollback scrollbar thumb (drag + track click) with wheel/keyboard.

- Terminal line timestamps: VTE screen stamps first write per row (unix ms),
  scrollback retains stamps; GPUI gutter renders `[HH:MM:SS(.mmm)]` beside line numbers.

- Bracketed paste: track DECSET/DECRST `?2004` in the VTE screen and wrap
  clipboard paste with `ESC[200~` / `ESC[201~` when enabled.
- Context menu Open Link for selected http(s) or dotted host URLs.

- Action links: domain settings `terminal.action_links_*`, matcher module for
  URL/IPv4/host:port/archive, context-menu actions, Ctrl/Cmd-click default action,
  and Settings → Terminal toggles.

- Action link decorations: underline + accent color on matched viewport tokens.
- Alt-click opens multi-action menu; Ctrl/Cmd-click runs default action.

## 2026-07-11 Action link hover tooltip

- Hover over matched action-link tokens shows a Tauri-like tooltip: kind badge,
  value, Ctrl/Cmd+click default action preview, and Alt+click more-actions hint.
- Tooltip tracks cursor position, clears on leave/menu/selection drag, and stays
  hidden while context/action menus are open.
- 250ms show delay matches Tauri ActionLinkTooltip; pointer cursor appears while
  hovering a matched link (pending or visible).

## 2026-07-11 Translation dialog + menu clamp

- Terminal context-menu Translate items open a modal Translation dialog (source,
  provider badge, loading/error/result, copy/close) instead of navigating away
  from the terminal workspace to the Translation settings page.
- Context menu and action-link multi-action menu positions clamp to the last
  known window viewport size so they stay on-screen near edges.

## 2026-07-11 Search engine icons + action-link pointer

- Domain `SearchEngineConfig` gains optional `icon` (Tauri SEARCH_ICONS keys),
  load/save under `search.custom_engines[].icon`, defaults for Google/Bing/GitHub.
- Settings Search rows show a clickable icon chip (cycles icon keys); context-menu
  online-search items include a short icon prefix.
- Terminal surface uses pointer cursor while an action-link hover tooltip is active.

## 2026-07-11 Terminal glyph metrics + word separators + middle-click paste

- Selection/action-link hit-testing prefers GPUI TextSystem `ch_advance` for the
  configured terminal font (Tauri-style measured cell width), with fontSize*0.62
  fallback and painted 18px row height at the default 14px font.
- Double-click word selection uses `interaction.word_separators` (xterm
  wordSeparator semantics) instead of a hard-coded alnum set; default separators
  align closer to Tauri.
- Middle-click on the terminal surface pastes from the clipboard (xterm/Linux).

## 2026-07-11 Shift+click selection extend + clear on input

- Shift+left-click extends the existing terminal selection from its anchor and
  continues drag-extend (xterm-style).
- Sending terminal input (typing/paste/commands) clears the active selection.

## 2026-07-11 Dynamic terminal gutter widths

- Timestamp/line-number gutter column widths scale from measured cell metrics
  (0.85x gutter font) with the previous fixed 96/72/40 floors as minimums so
  hit-testing and paint stay aligned.

## 2026-07-11 Inline command suggestions

- Local keystroke draft tracks plain terminal input for fuzzy suggestions from
  command history + quick commands (`search_command_sources`).
- Popup near terminal cursor with history/quick badges, ↑↓/Enter/Tab/Esc, and
  click-to-run; respects min/max suggestion length settings and dismisses on
  menus/submit/tab-desync/control sequences.

## 2026-07-11 Terminal credential autofill

- Domain helpers port Tauri `credentialAutofill.ts` (ANSI/OSC strip, prompt
  extraction, username/password detection, custom regex match, default password
  fallback).
- GPUI panel near terminal cursor lists matching saved credentials with
  ↑↓/Enter/Esc/click; username fill arms a 60s pending auto-password send.
- Detection runs on active session output; session switch resets state; open
  panel / prompt mode suppresses inline command suggestions.

## 2026-07-11 Command suggestion history delete

- History-sourced command suggestion rows expose a delete control that removes
  the entry from redb command history and refreshes the live suggestion list
  (Tauri CommandSuggestions deleteHistory parity).

## 2026-07-11 Command suggestion fuzzy highlights

- Suggestion rows paint fuzzy match indices in accent/semibold spans, matching
  Tauri HighlightedCommand presentation for history and quick-command hits.

## 2026-07-11 Terminal input tracker for suggestions

- Domain `terminal_input_tracker` ports Tauri `terminalInputTracker` basics:
  insert/backspace/word-delete, cursor moves, tab desync, paste mode, and
  shell-prompt sanitization.
- Inline command suggestions now key off the tracker (`can_suggest_from_tracker`)
  instead of a simplified keystroke draft string.

## 2026-07-11 Command suggestion suppression + tab resync

- Domain `command_suggestion_suppression` ports Tauri interactive-program
  detection (vim/htop/less/sudo wrappers, `tail -f`, journalctl pager).
- Submitting a suppressing command hides suggestions until Ctrl+C or `q`.
- Tab-completion desync recovers via `resync_from_terminal_line` from the live
  terminal cursor line before the next non-tab keystroke.
- Enter records history from the tracker submission snapshot (pending entry)
  instead of only raw input bytes.

## 2026-07-11 Command suggestion 80ms debounce

- Fuzzy suggestion search is debounced 80ms after tracked input changes, with a
  generation counter cancelling stale timers (Tauri searchTimer parity).

## 2026-07-11 Tab actions near-cursor menu + search join fix

- Right-click tab actions open as a compact anchored menu at the pointer
  (Tauri TabContextMenu-like placement) instead of only a centered modal.
- Terminal buffer search rejoins lines with real newlines (broken join fixed).

## 2026-07-11 Character-level terminal find highlights

- Buffer search matches now include char column ranges and paint find decorations
  on the matched tokens (inactive selection bg, active warning/inverted span)
  instead of only tinting whole rows.
- Deep history search results show total/elapsed summary plus before/match/after
  context lines under each hit (Tauri TerminalSearchBar history densification).

## 2026-07-11 Compact tab actions layout

- Anchored tab actions menu uses a denser single-column layout with a compact
  header (session name/kind/id + Esc close) for closer Tauri TabContextMenu feel.

## 2026-07-11 Terminal search bar polish

- Buffer find status shows `1000+` at the match cap; prev/next controls are
  buffer-mode only, matching Tauri TerminalSearchBar behavior.

## 2026-07-11 Compact tab actions vertical menu + smart input selection

- Anchored tab actions menu now uses a Tauri TabContextMenu-like vertical item list
  (28px rows, separators, inline color swatches, disabled items) instead of dense
  card/grid action buttons. Expanded centered dialog retained for non-anchored open.
- Domain tracker gains `delete_terminal_input_range`, `build_move_input_cursor_data`,
  and `InputSelectionRange` helpers (Tauri `deleteTerminalInputRange` / move CSI).
- GPUI smart cursor selection: when a painted selection is fully contained in the
  tracked input line, Backspace/Delete removes the range, plain typing replaces it,
  and Left/Right collapse to edges without clearing via normal send path.
- Smart input click: mouse-up on the tracked command line repositions the shell
  cursor with CSI left/right moves; finishing a selection fully inside the input
  collapses the caret and clears the selection (Tauri mouse smart-cursor path).
- Paste into a smart input selection replaces the range (Tauri pasteText path).
- Terminal Enter now sends CR (`\r`) like xterm/Tauri instead of LF.
- Command suggestion popup densified to Tauri-like 380px width, uppercase
  compact header, and 11px mono rows.

## 2026-07-11 Disconnect keeps tab + Enter reconnect

- `SessionRuntimeMetadata.disconnected` keeps UI tab/order/metadata when the
  backend exits or the user chooses Disconnect (Tauri disconnected pane).
- Exited events no longer remove the tab; they stamp a disconnect banner and
  keep the buffer for reconnect.
- Enter (and tab Reconnect) recreate the session from `launch_config`, migrating
  custom name/color/history; Ctrl+D closes a disconnected tab.
- Tab strip uses danger accent for disconnected sessions; ordered_sessions
  synthesizes `SessionInfo` for disconnected ids.

## 2026-07-11 Selection preserve + disconnected chrome polish

- Non-smart terminal selections are no longer cleared on ordinary typing (Tauri
  custom key handler parity); smart input selections still clear themselves.
- Disconnected tabs show muted title (`· disconnected`), reduced opacity, danger
  accent, hide the live cursor, and a compact reconnect status strip.
- Reconnect writes a cyan `[Reconnecting…]` buffer line before recreating the
  backend session.

## 2026-07-11 Ctrl/Alt arrow CSI + expanded Disconnect

- Terminal input maps Ctrl+Arrow to CSI `1;5` and Alt+Arrow to CSI `1;3`, matching
  Tauri XTerminal word-navigation sequences; Alt+b/f/d emit ESC-letter word ops.
- Expanded (centered) Tab Actions dialog gains Disconnect (keep tab) alongside
  Reconnect/Close.


## 2026-07-11 Scroll-to-bottom, visual BEL, file-drop overlay

- When the viewport is scrolled into history (`scroll_offset > 0`), the active
  terminal shows a compact `↓ Live` FAB that jumps back to live output.
- BEL (`0x07`) sets a pending visual-bell flag on `TerminalScreen`; the GPUI
  surface flashes a light border overlay for ~200ms (event-pump ticks).
- External file drops (`gpui::ExternalPaths`) onto the terminal show a Tauri-like
  dashed overlay. Local sessions insert shell-quoted paths; SSH/Telnet/Serial
  report ZMODEM/SFTP guidance until the native ZMODEM pipeline lands.
- Domain helpers: `quote_local_path`, `format_local_terminal_drop_input`,
  `terminal_drop_overlay_copy`.

## 2026-07-11 OSC 0/2 window title → tab name

- `TerminalScreen` parses OSC 0/2 titles into `window_title` / `take_window_title`.
- GPUI stores titles in `session_dynamic_titles`; display name priority is
  custom rename → OSC title → backend session name.
- Titles migrate across reconnect restore maps and clear with session removal.

## 2026-07-11 Native ZMODEM core + GPUI interception

- Ported Tauri `core/zmodem.rs` into `nyaterm-session::zmodem` with `zmodem2`
  path dependency; detector/transfer unit tests pass.
- GPUI event pump intercepts raw session output via per-session
  `ZmodemDetector` / `ZmodemTransfer` before terminal paint.
- File-drop on SSH/Telnet/Serial/Raw queues upload paths, sends `rz\r`, and
  auto-accepts when remote ZMODEM upload headers are detected.
- Download detection opens a native folder picker; cancel aborts the transfer.
- Progress/complete/failed update `terminal_status` (transfer-list polish later).

## 2026-07-11 OSC 8 hyperlinks

- `TerminalScreen` tracks OSC 8 URI pool + per-cell indices; viewport snapshots
  expose `hyperlink_lines` with char column ranges.
- Terminal paint merges OSC 8 ranges into link underlines alongside action-links.
- Ctrl/Cmd-click opens http(s)/mailto OSC 8 URIs via the system opener.

## 2026-07-11 ZMODEM transfer jobs + OSC 133 shell integration

- ZMODEM Progress/Complete/Failed events upsert `TransferJobKind::ZmodemUpload`
  / `ZmodemDownload` rows with `SftpTransferProgress` so the transfer strip
  shows live percent; cancel routes through session ZMODEM state.
- `TerminalScreen` parses OSC 133 A/B/C/D shell-integration marks; command
  start/finish edges suppress and re-enable command suggestions (Tauri parity).

## 2026-07-11 OSC 7 CWD tracking

- `TerminalScreen` parses OSC 7 `file://` paths into `cwd` / `take_cwd`.
- GPUI stores per-session paths in `session_cwds`, shows them in session info,
  and when transfer-browser auto-sync-cwd is enabled, updates the browser path
  from OSC 7 without waiting for SFTP `pwd`.

## 2026-07-11 Native parity: sync action overlay + large-output protection

- Terminal panes now show a Tauri-style sync action chrome when the session is in an
  enabled sync group: Pause/Resume, Leave, Close Group (color-matched border).
- Large-output protection tracks per-session burst size, trims oversized chunks to the
  Tauri visible backlog cap, shows Overloaded/Recovered banners with skipped character
  counts, and recovers after a calm event-pump window (~3s).

## 2026-07-11 Terminal DEC modes: alternate screen + mouse reporting

- `nyaterm-terminal` now tracks DECSET 1049/47/1047 alternate screen (isolated from
  primary scrollback), 1000/1002/1003 mouse reporting, 1006 SGR encoding, and CSI s/u
  save/restore cursor.
- Native UI sends SGR/legacy mouse reports for click/wheel when reporting is active,
  matching xterm apps (vim/less/tmux) instead of local selection/scroll.

## 2026-07-11 ZMODEM SFTP conflict probe before rz

- Native ZMODEM uploads now probe the remote CWD via SFTP before sending `rz`,
  applying the transfer duplicate policy (ask/skip/overwrite/rename) like Tauri
  `probeAndResolveRemoteConflicts`. Probe failures skip detection and proceed.

## 2026-07-11 DECSTBM scroll region + origin mode + line ops

- `TerminalScreen` tracks DECSTBM (`CSI top;bottom r`) scroll margins and DECSET 6
  origin mode; partial-region scrolls no longer push rows into primary scrollback.
- CSI `S`/`T` scroll the region; CSI `L`/`M` insert/delete lines within the region.
- ESC `D`/`E`/`M` and C1 IND/NEL/RI drive index/reverse-index; ESC `7`/`8`/`c`
  cover DECSC/DECRC and RIS for tracked state.

## 2026-07-11 In-window Tab Windows multi-leaf layout

- Added `TerminalWindowNode` (leaf/split) modeling Tauri `tabWindows` multi-leaf tab
  groups, separate from per-session `WorkspacePaneNode` pane splits.
- Tab actions and workspace toolbar: **New Window Right/Below** detaches the active
  tab into a new leaf (requires ≥2 tabs in the source leaf); **Merge Windows**
  restores the flat global tab strip.
- Multi-leaf mode renders each leaf with its own mini tab strip + terminal canvas;
  the global strip is hidden while multi-leaf is active.

## 2026-07-11 Tab dock drag/drop zones

- Ported Tauri `TabDockDropOverlay` / leaf drag-over docking into native:
  `TabDockZone` detects center vs left/right/top/bottom from pointer position.
- `TerminalWindowNode::dock_tab` merges (center) or edge-splits the target leaf.
- Multi-leaf leaf content accepts `SessionTabDragPayload` with live drop overlay
  chrome and applies dock on drop; model unit tests cover zone detection and edge dock.

## 2026-07-11 Multi-leaf leaf tab strip densify

- Leaf mini-tabs now mirror global strip chrome: status/color accent, unread/disconnected
  cues, close button, right-click tab actions, and in-strip reorder via
  `place_tab_before` (same leaf) / move-to-before (across leaves).

## 2026-07-11 Multi-leaf window split resize

- Workspace split drag handles now resize `TerminalWindowNode` multi-leaf splits as
  well as per-tab `WorkspacePaneNode` splits (same divider chrome).

## 2026-07-11 Multi-leaf leaf strip + and drop cleanup

- Each multi-leaf tab strip has a `+` control to start a local session into the workspace.
- Clearing `terminal_window_drop` on mouse-up avoids sticky dock overlay chrome after drag cancel.

## 2026-07-11 Multi-leaf terminal_window_layout persistence

- Domain stores Tauri-compatible `ui.terminal_window_layout` via
  `RestorableTerminalWindowNode` (tab indexes + split ratios).
- Settings flag `general.startup_restore_window_layout` (default true) gates restore/save.
- Native multi-leaf mutations serialize layout; reconcile restores once sessions exist.

## 2026-07-11 open_tabs startup restore

- Domain persists Tauri-compatible `ui.open_tabs` (session type, connection id,
  custom name, tab color).
- When `startup_restore` is enabled, native sequentially reconnects saved tabs on
  launch, then restores multi-leaf `terminal_window_layout` if enabled.

## 2026-07-11 workspace_pane_layout persistence

- Domain stores native global pane splits as `ui.workspace_pane_layout`
  (`RestorableWorkspacePaneNode` with ordered open-tab indexes).
- Distinct from Tauri per-tab `RestorablePaneNode` trees and from multi-leaf
  `terminal_window_layout`.
- Native split/unsplit/resize/prune persist when startup restore + window layout
  restore are enabled; startup restores panes after open_tabs (and after multi-leaf
  if that wins and skips panes).

## 2026-07-11 Smart Split / Tile multi-leaf

- Ported Tauri `smartSplit` balanced binary tree (auto / horizontal / vertical).
- View menu + tab actions: Smart Split, Tile Horizontally/Vertically, Merge Windows.
- Applies full multi-leaf layout (one tab per leaf) and persists `terminal_window_layout`.

## 2026-07-11 Broadcast to All sessions

- Tauri `broadcastToAll` toggle: when enabled, terminal input fans out to every
  live session in addition to sync-group peers.
- Exposed via Terminal menu; status line reports enable/disable.

## 2026-07-11 View menu theme + language quick picks

- View menu lists appearance themes with checkmarks and English/中文 language
  toggles (Tauri Header theme/language submenus), plus existing zoom/sidebar/tile.

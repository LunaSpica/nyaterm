# GPUI Migration Status

This document records the current GPUI migration boundaries and debt in
`nyaterm-desktop`. Keep dynamic counts here instead of in `AGENTS.md`.

Last updated from the working tree on 2026-07-29.

## Current Metrics

| Metric | Current value | Notes |
| --- | ---: | --- |
| `NyaTermApp` fields | 21 | Counted from `features/app_state/mod.rs`; down from 585. The remaining fields are composition services and focused feature owners. |
| `impl NyaTermApp` blocks | 237 | Spread across 232 files under `crates/nyaterm-desktop/src`. |
| `#[path = "..."]` declarations in desktop | 0 | Cleared. Every directory is a real module; the boundary script fails on any new occurrence. |
| `use super::*` imports in desktop | 0 | Cleared in production and test modules; guarded crate-wide. |
| `features/prelude.rs` rough exported-token count | 0 | The transitional shared prelude is removed and guarded against reintroduction. |
| Entity Store structs | 3 | `WindowRuntime`, `StartupRestore`, `Overlay`. Each owns state the app does not. |
| Snapshot structs | 0 | Cleared. No store is a projection of `NyaTermApp` any more. |
| `replace_snapshot` methods | 0 | Cleared. |
| Store snapshot publish calls | 0 | `publish.rs` and the publish throttle are gone. |

Large files currently over 4,000 lines:

| File | Lines | Status |
| --- | ---: | --- |
| `crates/nyaterm-desktop/src/models/terminal.rs` | 4964 | Split candidate; coordinate with terminal render/runtime ownership. |
| `crates/nyaterm-desktop/src/features/terminal/terminal_surface_entity.rs` | 4516 | Split candidate; avoid hot-path regressions. |
| `crates/nyaterm-transport/src/lib.rs` | 4539 | Split by domain: SFTP, X11 forwarding, SSH tunnels and SSH authentication are out. What remains is the session manager, the four `TerminalTransport` impls and the SSH/serial/telnet session lifecycle. |
| `crates/nyaterm-core/src/storage.rs` | 4089 | Split by domain: config backup, keyword highlights, command history, known hosts, AI history, the secret vault, portable snapshots, app settings and cloud sync are out. About 1,450 lines of that are code; the rest is the test module. Schema compatibility is public contract. |

`core/ai.rs` was on this list at 4,032 lines; it is now 1,554 after being split
into `providers`, `agent`, `risk` and `settings`.

Other files currently over 2,000 lines include terminal runtime/view modules,
transport transfer protocol modules, and terminal GPUI painting modules. Treat
these as staged extraction candidates, not as formatting-only refactor targets.

## Completed

- Ordinary form, prompt and search inputs are real widgets, not label divs.
  `nyaterm-ui::TextField` owns the caret, selection, IME composition and
  clipboard for a box; `nyaterm-core::TextEdit` owns its editing model. Panels
  either own entities directly (the connection editor) or use the id-keyed
  registry in `features/text_inputs.rs`. Coverage includes settings and network
  editors, quick commands and send-command controls, security and SSH prompts,
  AI and sync fields, session overlays and filters, SFTP paths/search/dialogs/
  properties, terminal search, and keyword-highlight rules. The removed
  `transfer_input` helper is not a compatibility path.

  Full editing surfaces are separate by design: the terminal and paste review
  use terminal input routing, and `RemoteTextEditor` owns editor-specific
  selection, undo and command handling. The built-in transfer editor remains a
  dedicated editor-surface migration, not a registry form field. None of these
  should be replaced mechanically with a single-line registry field.

  Four GPUI behaviours make this migration go wrong in ways that are invisible
  until you drive the UI:

  - A surface that grabs focus on click (`.on_click(|| window.focus(&own))`)
    takes it straight back off whichever field the pointer landed on, because
    click follows mouse-down. Every converted dialog had one and every one had
    to lose it.
  - A wrapped field asks for its parent's height (`relative(1.)`). The box that
    hosts it needs a *definite* height and must not set `items_start`, or the
    percentage resolves against an indefinite value, the field lays out at zero
    height, and the box renders as an empty rectangle that cannot even be
    clicked.
  - `Div::text_ellipsis()` and `overflow_hidden()` do not stop text wrapping.
    Without `whitespace_nowrap()` a one-row label lays itself out as a column
    and shows whichever line lands in the row — a session tab read "ste", out of
    the middle of "System32".
  - A box has to be handed a width. It is content-sized by default, and an empty
    field's content is nothing, so it renders as a ~30px square that wraps its
    placeholder one or two characters per line. A flex column stretches its
    children, so most callers are fine; a `flex_none` slot (every
    `settings_form_row` control) and a plain block parent are not, and those
    call sites give the box an explicit width or a `flex_1` row of its own. A
    percentage width does not resolve against a block parent here — flex
    distribution does.

- Icon assets are no longer hand-drawn approximations. The migration had redrawn
  ~113 SVGs by hand in a thin-stroke style, while the pre-GPUI UI was
  overwhelmingly Material Design (140 `react-icons/md` imports against 41 lucide,
  and those 41 were shadcn internals) — which is why the two builds looked
  nothing alike. Icons are now vendored from their upstream sets through
  `scripts/icons.manifest` + `scripts/sync-icons.sh` and committed; see
  `docs/third-party-icons.md`.
- The monochrome/full-color split is explicit and enforced. `icons/**` goes
  through `mono_icon()`/`svg()` as an alpha mask, `color/**` through
  `color_icon()`/`img()` as a raster. This is what let the 33 official distro
  logos come back: previously 14 distros shared one tinted Tux, Kubernetes wore
  the Docker whale, and 14 services shared a tinted server rack, because `svg()`
  cannot carry more than one color.
- The icon lookup tables live in `features/icons/` (connection, quick-command,
  search-engine, file-kind, alias normalization, remote-system inference), split
  from the element construction in `features/view_widgets/icons.rs`. The
  two-letter text badges (`"DK"`, `"K8"`, `"UB"`) and the letter search-engine
  badges (`"G"`, `"GH"`) are gone, along with the duplicate badge table the
  terminal context menu carried.

- Workspace is already on resolver `3` for the Rust 2024 workspace.
- The migration dashboard, legacy source inventory and their dedicated
  workspace crate are removed. Default and development builds no longer carry
  a local legacy source-tree dependency.
- The three remaining Entity stores are authoritative owners: the window pump,
  startup restore and quick-switch overlay state are not writable mirrors of
  `NyaTermApp` or a FeatureState.
- The shell feature area is a real module tree. `features/shell` is declared as
  a normal `mod shell;` with its own `mod.rs`, `event_pump` and
  `keybinding_runtime` are directory modules, and `features/shell` no longer
  contains any `#[path]` declaration. Shell chrome exports reach the rest of
  `crate::features` through explicit re-exports in `features/shell/mod.rs`
  instead of twelve flattened `#[path]` module declarations.
- The session feature area is a real module tree. `features/session` is declared
  as `mod session;`, `session_runtime` is a directory module, and the prompt,
  auth and file-transfer session-state exports reach `crate::features` through
  explicit re-exports. Nesting immediately surfaced four prompt-drain methods
  that were only reachable because the module used to be a flat sibling; they
  are now declared `pub(in crate::features)` on purpose.
- The terminal feature area is a real module tree. `features/terminal` is
  declared as `mod terminal;`, and `terminal_runtime`, `terminal_surface`,
  `terminal_selection_runtime` and `terminal_context_menu_runtime` are directory
  modules, so seventeen further `#[path]` declarations are gone. Terminal
  internals are now addressed as `crate::features::terminal::terminal_runtime`
  rather than a top-level `crate::features::terminal_runtime`. Nesting also
  showed that nine prompt/terminal symbols no longer needed a `crate::features`
  level alias at all; those re-exports were removed.
- `features/mod.rs` contains no `#[path]` declaration at all. The AI, commands,
  settings, sync, transfers, remote and translation directories are real
  modules, together with their `ai_runtime`, `command_runtime`,
  `quick_command_runtime`, `security_runtime`, `settings_runtime`,
  `cloud_sync_runtime`, `transfer_jobs` and `remote_runtime` subtrees. Ten more
  `crate::features` level re-exports turned out to be unused once each consumer
  sat inside the owning subtree, and were removed.
- The `layout`, `panels`, `inspector`, `formatting` and `view_widgets` view
  areas are real module trees, including their nested `security_panel/panel`,
  `workspace/surface`, `quick_commands_panel/panel`, `send_command_bar`,
  `tab_actions_overlay` and `ai_widgets` subtrees.
- The complete twelve-module security presentation area under
  `layout/security_panel` and `layout/security_editors` now uses explicit
  imports instead of `use super::*`; both subtrees are guarded at zero wildcard
  imports. This is presentation import plumbing only: secret masking, unlock,
  delete confirmation, OTP refresh, and key/password/credential editor behavior
  are unchanged. Secret storage, serialization, and encryption formats are
  untouched.
- The rest of `features/layout` is explicit too: its remaining nineteen
  `use super::*` occurrences across activity bar, prompts, sidebar, sync
  history, title bar, workspace surfaces, and the workspace-menu tests are
  gone, so the complete layout tree is guarded at zero. Its seven directory
  entry modules are declaration-only and can no longer rebuild a shared import
  bucket. The empty `sidebar/panels.rs` module and its empty `impl NyaTermApp`
  were deleted. Compiler-confirmed cleanup also removed eleven shared-prelude
  exports and sixteen `crate::features` façade re-exports; activity-bar layout,
  prompt handling, session lists, sync-history actions, title menus, new-session
  menus, and tab interaction behavior are unchanged.
- The complete twenty-seven-module desktop terminal adapter tree now uses
  explicit imports, including its runtime, search, selection, context-menu,
  surface and large test modules. Forty `use super::*` occurrences are gone,
  all terminal files are guarded at zero, and the context-menu entry remains a
  declaration-only module instead of a shared import bucket. Compiler-confirmed
  cleanup also removed fifty-three shared-prelude exports and eleven terminal/
  formatting façade re-exports. This is desktop GPUI import-boundary cleanup:
  terminal parsing, snapshots and protocol handling in `nyaterm-terminal` are
  unchanged.
- The final twenty-five desktop `use super::*` imports are gone from the app
  composition root, construction/runtime-job modules, workspace views,
  workspace-tab implementation files and the remaining focused unit-test
  modules. The cleanup exposed the last consumers of `features/prelude.rs`;
  they now import their GPUI, core, transport, model and widget dependencies
  directly, so the transitional prelude was deleted rather than kept as an
  empty compatibility layer. A crate-wide guard now keeps both wildcard-import
  and shared-prelude debt at zero. Application construction, workspace layout,
  recording, credential matching, terminal frame processing, send-command and
  temporary-SSH-link behavior are unchanged.
- `#[path = "..."]` is gone from `nyaterm-desktop` and `nyaterm-terminal-gpui`.
  The `pages` tree, `http/cloud_sync` and `models/workspace_tabs` were the last
  holdouts; `pages/remote/docker` also stopped aliasing six sibling files and
  became a normal `docker/` directory. The per-directory guards collapsed into
  one crate-wide `check_no_matches`, so module paths now always match the
  directory layout and `pub(in ...)` bounds mean what they say.
- Quick command UI state is grouped into `QuickCommandFeatureState`, following
  the `ConnectionFeatureState` pattern: list, editor, dialogs, import and AI
  popover sub-structs, built from one `QuickCommandFeatureFocus` bundle.
  Twenty-seven `NyaTermApp` fields collapse into one, and `app_state` no longer
  imports any of the eleven quick command UI model types. This UI owner is now
  nested under the unified `CommandFeatureState` described below.
- Remote page state is grouped into `RemoteOpsFeatureState` with one struct per
  pane: `docker`, `process` and `stats`. The three panes turn out to share the
  same refresh bookkeeping (job id, owning session, pending flag, failure
  streak, last refresh instant), which the fifty-four prefixed `NyaTermApp`
  fields hid. Their job channels are created inside
  `RemoteOpsFeatureState::new` because construction was their only other use.
  A later ownership pass replaced the twenty-one repeated channel/lifecycle
  fields with three typed `RemoteJobState<Event>` values. Job admission now
  couples id/session ownership with `pending`, stale completion matching cannot
  clear a newer job, refresh failures and timestamps reset together on session
  switches, and the channels are private to the state module. Docker tab/search/
  confirm/details transitions, process filter/sort/selection/nice/signal state,
  Stats expansion, result collection and session-switch cleanup also execute on
  the pane owners. `NyaTermApp` retains SSH service launch, terminal status
  mirroring, active-session policy and GPUI notification.
- Security panel state is grouped into `SecurityFeatureState`: the four editors
  and their focus handles in `editors`, revealed passwords/credentials and
  generated OTP codes in `revealed`, and the master password prompt in
  `unlock`. The separate `screen_lock` child now owns the whole-application
  locked flag, password draft, focus, status and idle timer; its lifecycle
  transitions cannot mutate other app state. Secrets themselves still live in
  `nyaterm-core`; this is view/runtime state only, cleared through the same
  paths as before.
- Settings interaction state is grouped into `SettingsFeatureState`: custom
  search-engine row/editor menus, keyword-highlight editor expansion/focus,
  appearance menu and discovered font options, and keybinding search/recording
  state live in four focused child structs. Seventeen transient fields became
  one composition-root field. A later convergence pass moved the persisted
  `AppSettingsSummary`, `KeywordHighlightConfig`, staged master-password state,
  and global storage status into the same authoritative feature owner. Their
  save/load formats, encryption behavior, and storage paths are unchanged.
- Translation and native-update background state now have authoritative
  `TranslationFeatureState` and `UpdateFeatureState` owners. Eighteen app fields
  became two feature fields; each owner constructs and retains its own job
  channel together with pending/status/result and dialog state. Translation
  settings and the secret draft moved as one compatibility-sensitive unit,
  while their existing load/save, masking and fallback paths are unchanged.
  Native update runtime also moved out of the settings module into its own
  normal module tree, and both feature-specific job event types left the shared
  `runtime_jobs.rs` bucket. A later method-ownership pass made both channels
  private and moved job admission, bounded event draining, dialog transitions,
  translation input/secret routing, and the settings/secret-draft/target-language
  synchronization invariant onto those owners. `NyaTermApp` now retains the
  native HTTP/thread launch, persistence and clipboard adapters, terminal-status
  mirroring and GPUI notification only. The later encapsulation pass made every
  surviving field on both feature states and their job payloads private, moved
  persistence-time secret-draft merging behind `TranslationFeatureState`, and
  removed the duplicate translation `target_language` field so
  `TranslationSettings::target_language` is the only writable source.
- Cloud-sync configuration, compatibility state, history, conflicts, secret
  drafts and GitHub device-flow runtime now have one authoritative
  `CloudSyncFeatureState` owner with a focused `github` child. Fifteen app
  fields became one feature field, and the device-flow channel is constructed
  by that owner. Every owner field and the GitHub child are now private. Input
  routing, settings toggles/replacement, persistence-time secret-draft merging,
  job admission/completion/failure, conflict capture, history replacement and
  expansion, provider-menu state, and GitHub device-flow identity/cancellation/
  event transitions now stay on the feature state. Views use read-only accessors;
  `NyaTermApp` retains master-password/session policy checks, network and disk
  work, external-URL/clipboard adapters, terminal-status mirroring, GPUI
  notification and task coordination. Secret input still updates only
  `CloudSyncSecretDraft`; encrypted settings, serialized state/history and
  legacy state-document compatibility paths are unchanged.
- Recording and SSH-tunnel background resources now have focused
  `RecordingFeatureState` and `TunnelFeatureState` owners. Eleven app fields
  became two feature fields: the recording owner constructs the manager/write
  pipeline and retains active-count, deferred auto-start, busy/search and path
  prompt state, and all of those recording fields are now private. Recording
  action admission/completion, deferred auto-start, path-prompt admission,
  manager access and write-pipeline access execute through owner methods. The
  tunnel owner constructs its manager/job channel and owns pending-job
  admission and completion. Its manager, channel, pending list and nested
  tunnel/proxy catalog are now private. Catalog move/upsert/delete candidates
  are produced by the owner and become authoritative only through explicit
  commit methods after persistence succeeds. `NyaTermApp` remains responsible
  for GPUI prompts, notifications, session policy, `ConnectionStore` calls and
  job-result routing. Recording output behavior, tunnel transport execution and
  persisted tunnel configuration formats are unchanged.
- Live session runtime now has one top-level `SessionFeatureState` owner. The
  session manager and event bridge, command history/search/menu/busy state,
  active session/SSH/AI profile, order and runtime metadata, custom and dynamic
  titles, working directories, tab colors, ZMODEM/trzsz state and SSH
  multiplex handles moved from nineteen app fields to one feature field. The
  existing `SessionStartFeatureState` is nested as `SessionFeatureState::start`
  and still owns its worker channel, pending/failed maps, active selection,
  cancellation tombstones, pane replacement state, reconnect failures and
  deferred workspace split. Two more focused children now own the session
  interaction lifecycle: `SessionPromptState` constructs and retains the
  SFTP-duplicate, host-key and credential brokers, active SSH verification
  prompts, OTP provider and prompt focus; `SessionDialogState` owns tab
  actions, close-all/quit confirmation, rename, color, info, startup-command
  and temporary-SSH-link dialogs. This removes another thirty-four app fields.
  A later ownership pass made all ten prompt fields private. Broker admission,
  active-prompt resolution, credential and keyboard-interactive input, focus
  lifecycle, OTP preview/refresh state and prompt-id mismatch handling now run
  through `SessionPromptState` transitions and read-only getters. Mismatched
  decisions preserve the active prompt, and the existing distinction between
  manual OTP generation and periodic TOTP refresh remains unchanged.
  A later ownership pass made all twenty-four dialog fields private. Tab-action
  and submenu lifecycle, close-all/quit confirmation, rename validation, color
  and info close/open state, startup-command delay, and temporary-SSH draft/
  error cleanup now execute through owner transitions and read-only getters.
  `NyaTermApp` retains worker spawning, navigation, session policy, text-field
  setup, event routing, status updates and GPUI notifications. Credential
  storage, session creation, reconnect, terminal protocol and transport
  behavior are unchanged.
  The session-start ownership pass made all eleven surviving
  `SessionStartFeatureState` fields private. Fresh/reconnect registration,
  cancellation tombstones, event-result admission, active pending/failed
  selection, failure routing, reconnect errors and deferred workspace splits
  now update through owner transitions; renderers receive only read-only
  iterators and queries. The write-only `SessionPaneState`/`panes` projection
  was removed because no runtime or view ever read it. Worker spawning,
  connection-store updates, session registration and GPUI notification remain
  application-level coordination, and session/persistence formats are
  unchanged.
- Transfer state is grouped into `TransferFeatureState`. Seventy-eight fields
  turned out to be five separate things sharing one panel: the job `queue`, the
  SFTP `browser`, the file operation dialogs (`file_ops`), the built-in remote
  `editor`, and `external_sync` for handing a file to an outside editor, plus
  manual `paths` and `panel` chrome. Their lifetimes are unrelated, which the
  flat `transfer_*` prefix hid completely.
- The SFTP browser parity pass covers the six Tauri file-manager areas: toolbar
  commands and state, editable path/history/breadcrumb navigation, resizable
  sortable columns, range/additive selection and context actions, transfer
  progress and queue controls, and keyboard handling. The desktop state and
  pure helpers are covered by `nyaterm-desktop` tests. Real remote verification
  is complete against a disposable localhost OpenSSH server. The opt-in
  `sftp_service_round_trips_file_manager_operations` test covers directory and
  file creation, text read/write, properties, rename, upload, download,
  listing, and recursive cleanup. The live GPUI pass covered session restore,
  the browser toolbar and hidden-file toggle, path/history navigation, natural
  and column sorting, column resizing, additive selection, the context menu,
  and loading remote file properties. The transfer queue remains covered by
  desktop state tests, while its upload/download transport path is covered by
  the remote E2E test.
- AI state is grouped into `AiFeatureState`: provider `settings`, the `chat`
  composer and transcript, session `history`, model `discovery`, the agent
  `loop`, and `panel` chrome. Note that `SettingsDraftSnapshot` deliberately
  keeps its own `ai_settings` / `ai_model_draft` / `ai_base_url_draft` /
  `ai_secret_draft` fields with the old names; it is a separate snapshot type,
  and the rewrite was anchored on `self`/`this` so those were left alone.
- Terminal presentation state is grouped into `TerminalFeatureState`: `search`,
  `view` runtime, `input` focus and IME, inline command/credential `assist`,
  dedicated multi-line `paste` review editor, `selection` and mouse reporting,
  painted `layout` geometry, `menus`, paint caches, and the split/tab `windows`
  tree. The `assist` child owns the sixteen command tracker, suggestion popup,
  credential prompt detector and background matcher fields that previously
  lived directly on `NyaTermApp`; session-switch and settings reset operations
  now live on that child state. Action-link menu/tooltip state is part of
  `menus`, the terminal frame queue is part of `view`, and terminal palette and
  keyword-highlight caches are part of `paint`. `TerminalPasteReviewState` is
  still the dedicated full editing surface: it now owns draft normalization,
  UTF-8 cursor and selection transitions, vertical movement and IME reset, while
  `NyaTermApp` retains GPUI key routing, status updates and session sends. The
  line-by-line path still intentionally bypasses bracketed-paste framing.
  Parsing, snapshots and the wire protocol are untouched and stay in
  `nyaterm-terminal` and `nyaterm-transport`. `OverlaySnapshot` keeps its own
  `terminal_actions_open` / `terminal_context_menu_open` projection fields.
- `core/storage.rs` is split by domain rather than by type: `command_history`,
  `known_hosts`, `ai_history`, `vault` (SSH keys, OTP, passwords, credentials),
  `portable` (snapshots and config backups), `app_settings` and `cloud_sync`,
  on top of the earlier `config_backup` and `keyword_highlights`. Each took its
  table constants, record types, transaction helpers and domain logic with it.
  `known_hosts` also took `storage.rs`'s only uses of `base64`, `hmac` and
  `sha1`, and `app_settings` took twenty-nine `json_*` / `normalize_*` helpers
  that had no caller left outside it — that is the sign a seam is in the right
  place. Table names, key layouts, record shapes, document keys and the
  hashed-host matching rules are unchanged.
- `core/ai.rs` follows the same cut: `providers` (the three provider HTTP
  surfaces), `agent` (tool schemas, reply parsing, execution policy), `risk`
  (command risk classification) and `settings` (defaults, legacy migration,
  secret masking). `risk` is deliberately small and separate: it decides
  whether the agent may run a command unattended, so the pattern lists and
  escalation rules are worth reviewing on their own rather than buried in a
  4,000-line file. The `default_*` functions back serde attributes on types
  that stayed behind, so `ai.rs` imports them back by name -- a reminder that
  a serde-heavy type is not free to move away from its defaults.
- `transport/lib.rs` is split the same way: `sftp`, `x11`, `tunnel` and
  `ssh_auth`. Each carries its own domain types and unit tests, and each uses an
  explicit import list rather than a `use super::*` glob, so the split narrows
  visibility instead of just moving lines. The SFTP wire protocol, the X11
  cookie rewrite rules, the SOCKS5 handshake and the SSH authentication method
  order are unchanged.
- Send-command bar state is grouped into `SendCommandFeatureState`, split into
  the three phases the bar actually has: `composer` (payload and caret),
  `options` (how it is interpreted and delivered, plus the menus that set
  those), and `progress` (in-flight send, cancellation, counters).
- Sync-input groups now have one authoritative `SyncInputFeatureState` owner.
  The group collection, overlay/search/selection/delete state and broadcast-all
  flag moved together and all owner fields are now private. Views and command
  routing use read-only accessors, while group lifecycle, membership, pause,
  peer fan-out, broadcast toggles and disconnected-session cleanup execute on
  the owner. `NyaTermApp` retains session metadata/host queries, status messages,
  focus and redraw coordination. This state remains transient and changes no
  session or settings persistence format.
- `ShellFeatureState` now owns the complete cross-view window interaction
  cluster. Its `viewport`, `navigation`, `panels`, `chrome` and `workspace`
  children contain geometry bookkeeping, settings navigation/window state,
  side-panel stacks and resize lifecycles, title/tab menus and per-tab pane
  ownership respectively; the existing `bottom_panel` child still owns its
  mode, heights and drag state. Forty-seven top-level app fields became these
  five focused children, so `NyaTermApp` dropped from 102 fields to 55 without
  creating a writable mirror. Pure viewport, panel resize/stack resize, chrome
  menu and workspace ownership transitions execute on the child states.
  Render helpers remain on views, while `NyaTermApp` retains settings
  persistence, GPUI notification, terminal coordination and event routing.
- The remaining transient coordination tail was moved as one cohesive batch.
  `SessionFeatureState` now owns the pending transport-event queue and startup
  restore completion lifecycle; `StartupRestoreStore` owns only its queue,
  pending layouts and startup/load idempotence, so the former duplicated
  completion flag is gone. `CommandRuntimeState` owns the command persistence
  channel pair and pending-work admission, while shell diagnostics own their
  keyed throttle timestamps. Settings path/password prompts, About visibility
  and the remote editor window lifecycle now live under their existing
  settings, shell and transfer feature owners. Thirteen top-level fields became
  one new runtime owner plus focused children on existing states, taking
  `NyaTermApp` from 55 fields to 43 without changing persistence formats or
  background-worker behavior.
- Connection-owned catalogs are no longer eleven unrelated `NyaTermApp`
  fields. `ConnectionCatalogState` authoritatively owns saved connections,
  groups and runtime-discovered serial ports. Those fields are private; views
  consume read-only slices, load/clear/discovery replacement goes through owner
  methods, and recently-used updates cannot acquire a mutable catalog slice.
  Multi-selection reorder and group-cycle checks also live on the catalog.
  `SecurityFeatureState::catalog` owns SSH-key, OTP, password and credential
  catalogs without deriving `Debug`; and the private catalog inside
  `TunnelFeatureState` owns tunnel/proxy configs and groups alongside their
  existing runtime owner. The queued-saved-connection
  lifecycle and its request models moved from `app_state` to
  `SessionStartFeatureState`. Twelve old top-level fields became one focused
  top-level catalog plus children of existing owners, taking `NyaTermApp` from
  43 fields to 32. `ConnectionStore` remains the persistence implementation;
  table names, keys, serialized fields, encryption and fallback behavior did
  not change.
- Command state now has one top-level `CommandFeatureState` owner. Its
  `catalog`, `quick`, `history` and `runtime` children replace the separate
  quick-command collections, UI state, command-history collection and
  persistence runtime fields, taking `NyaTermApp` from 32 fields to 28. The
  owner constructs the command persistence worker and provides grouped
  load/clear transitions, while catalog writes replace commands and categories
  together. Quick-command serialization, command-history storage, portable-key
  handling and worker behavior are unchanged.
- The Entity Store projection layer is gone entirely, in two steps.

  First, the six domain stores (`Ai`, `CloudSync`, `Connections`, `RemoteOps`,
  `Settings`, `Transfer`) turned out to be write-only: outside `entities/` they
  appeared only in `app_shell::new`, and nothing read their snapshots. Every
  qualifying tick built six snapshot structs, compared them, and called
  `cx.notify()` for a reader that did not exist.

  Then `Workspace`, `Session` and `OverlaySnapshot` went the same way once the
  loop was traced end to end. `Workspace` and `Session` were read only by
  `published_core_store_snapshots_are_current`, which decided whether to
  republish them — a closed loop. `OverlaySnapshot` was a same-render
  round-trip: `Render` published it in its prologue and `overlay_host` read it
  back a few calls later, with a fallback that recomputed every field from the
  same `self` fields. `overlay_host` now computes those flags directly into a
  local `OverlayFlags`, which is what the fallback already did.

  `AppShell` never observed the stores — a comment there records that
  store-observe was amplifying each publish into an extra shell paint — so the
  `cx.notify()` in every publish had no subscriber either.

  A final audit removed `RuntimeStore` too: no consumer read it, while its
  `AppRuntime` duplicated the composition root and its native-service list only
  fed the retired migration dashboard. What remains owns something the app does
  not: `WindowRuntimeStore` (the window runtime pump), `StartupRestoreStore`
  (the restore queue) and `OverlayStore` (quick switch state, authoritative
  since the earlier migration).
- The connections UI state has started moving out of scattered `NyaTermApp`
  fields and into `ConnectionFeatureState`.
- The current connections state split separates list UI, import UI, editor
  draft/window state, group editor draft state, confirmations, and network page
  UI state.
- Quick switch overlay state is now owned by `OverlayStore`; the old
  `NyaTermApp` mirror fields were removed, and quick switch is no longer
  published through `OverlaySnapshot`. `QuickSwitchState` exposes read-only
  accessors; writes go through `OverlayStore` mutation methods.
- Shared prelude and desktop terminal facade reduction has started by moving the
  test-only terminal keyword highlight precompute helper to an explicit
  test-module import.
- `connection_runtime/groups.rs` no longer depends on the connection runtime
  wildcard import; its GPUI, app, model, and core dependencies are explicit.
- `shell/event_pump/helpers.rs` no longer depends on the event-pump wildcard
  import in production code; timing helpers now import only standard time/
  collection types and the transport event model they inspect.
- `shell/event_pump/planes.rs` no longer depends on the event-pump wildcard
  import; the runtime plane coordinator now names its GPUI, navigation,
  terminal paint counters and helper dependencies explicitly.
- `shell/event_pump/session_events.rs` no longer depends on the event-pump
  wildcard import; session-event draining now names its transport event, AI
  notice, terminal view and local drain-budget helpers explicitly. The stale
  wildcard allowance for the removed `event_pump/publish.rs` file is also gone.
- `shell/event_pump/mod.rs` no longer depends on the shell wildcard import or
  the helper wildcard import; the event-pump entrypoint now names its GPUI,
  prompt, text-input, navigation and timing-helper dependencies explicitly.
- `shell/global_shortcut_runtime.rs` no longer depends on the shell wildcard
  import; global shortcut routing now names its GPUI event types, app adapter,
  navigation models and shortcut matcher explicitly. `shortcut_matches` is no
  longer re-exported through `features/prelude.rs`.
- `shell/navigation_runtime.rs` no longer depends on the shell wildcard import;
  page/sidebar routing now names its GPUI context, app adapter and navigation
  models explicitly.
- `shell/keybinding_runtime/keybindings.rs` no longer depends on the parent
  keybinding wildcard import; shortcut recording now names its GPUI event
  types, storage facade and shortcut event conversion helper explicitly.
- `shell/keybinding_runtime` no longer carries any wildcard imports; the
  keyword-highlight runtime and its tests now name GPUI prompt/focus types,
  text input setup, storage and keyword-highlight models directly.
- `shell/panel_resize_runtime.rs` no longer depends on the shell wildcard
  import; resize handles and layout persistence now name their GPUI element
  traits, storage facade and navigation/layout models explicitly.
- `shell/activity_bar_runtime.rs` no longer depends on the shell wildcard
  import; activity chrome state and drag previews now name their GPUI render
  traits, app adapters, navigation models and icon helper explicitly.
- `shell/tab_mouse.rs` no longer depends on the shell wildcard import; tab
  drag previews, hover tooltips and mouse action routing now name their GPUI
  event/render traits, app adapter, formatting helpers and icon helper
  explicitly.
- `shell/quick_switch_runtime.rs` no longer depends on the shell wildcard
  import; quick switch routing now names its authoritative `OverlayStore`,
  query state, GPUI key/window types, text input setup and search helpers
  explicitly.
- `shell/panel_stack_runtime.rs` no longer depends on the shell wildcard import;
  panel stack coordination now names its GPUI element/event traits, app adapter,
  panel models, AI command mode, text input setup and preview helpers
  explicitly.
- `shell/appearance.rs` no longer depends on the shell wildcard import in
  production or tests; appearance settings now name their GPUI prompt/font
  APIs, storage facade, keyword-highlight merge helpers and tested font helpers
  explicitly.
- `shell/tab_windows_runtime.rs` no longer depends on the shell wildcard import;
  multi-leaf terminal window coordination now names its GPUI context,
  `ConnectionStore`, app adapter, formatting helper and terminal window models
  explicitly.
- `shell/workspace_runtime.rs` no longer depends on the shell wildcard import;
  workspace pane ownership and split-resize coordination now name their GPUI
  event/element traits, storage facade, UUID helper, app adapter and pane models
  explicitly.
- `features/shell/mod.rs` no longer imports the feature prelude for child
  modules. Shell child modules now carry their own explicit dependencies, while
  the module root only declares submodules and deliberate re-exports.
- `pages/transfers/helpers/editor.rs` no longer carries an unused helper
  wildcard import; the editor preview/search/permission helpers are pure local
  functions with no parent-module dependency.
- `pages/transfers/helpers/paths.rs` no longer depends on the transfer helper
  wildcard import; remote path helpers now name their column-width model and
  GPUI pixel type explicitly.
- `pages/transfers/helpers/properties.rs` no longer depends on the transfer
  helper wildcard import; property row builders and state construction now name
  their GPUI traits, SFTP entry model, transfer property state and shared
  permission-format helper explicitly.
- `pages/transfers/helpers/queue.rs` no longer depends on the transfer helper
  wildcard import; queue action controls, transfer job state helpers and their
  progress-ratio test now name GPUI traits, tooltip UI, transfer job models and
  transport progress types explicitly.
- `pages/transfers/helpers/job_row.rs` no longer depends on the transfer helper
  wildcard import; transfer job row rendering now names GPUI event/layout
  traits, transfer job models, shared transfer formatting helpers and queue
  progress helpers explicitly.
- `pages/transfers/helpers/browser.rs` no longer depends on the transfer helper
  wildcard import; browser search, sort-header rendering and natural SFTP entry
  ordering now name GPUI layout/event types, transfer browser sort models and
  SFTP entry/file-type dependencies explicitly.
- `pages/transfers/helpers` no longer contains any `use super::*`; the helper
  module root no longer imports its parent as a directory-level migration
  prelude, the transfers page parent no longer carries helper-only sort/job
  imports, and the architecture script now governs the whole helper directory.
- `pages/transfers/browser_columns.rs` no longer depends on the transfers page
  wildcard import; the column resize adapter now names only GPUI mouse/context
  events, `NyaTermApp`, and the browser sort-column model explicitly.
- `pages/transfers/browser_filter.rs` no longer depends on the transfers page
  wildcard import; file search filtering now names GPUI context/window/key
  events, text-input setup, SFTP entry data, sort-column state and helper
  sorting/status functions explicitly.
- `pages/transfers/browser_keys.rs` no longer depends on the transfers page
  wildcard import; browser keyboard routing now names GPUI key/window context,
  SFTP file type checks, shortcut matching and the search-text helper
  explicitly.
- `pages/transfers/browser_navigation.rs` no longer depends on the transfers
  page wildcard import; SFTP browser navigation/cache/favorite state now names
  GPUI context/window types, connection-store persistence, transfer navigation
  snapshots and path helpers explicitly.
- `pages/transfers/browser_selection.rs` no longer depends on the transfers
  page wildcard import; browser selection, context menu, delayed rename,
  clipboard, range selection and download dispatch code now names GPUI events,
  transfer UI state, SFTP entry types and path helpers explicitly.
- `pages/transfers/browser/helpers.rs` no longer depends on the browser module
  wildcard import; compact transfer toolbar/footer buttons now name GPUI
  element traits, tooltip construction, theme palette and app context
  explicitly.
- `pages/transfers/browser` no longer contains any `use super::*`; the browser
  module root is a normal module declaration, and the browser view names its
  GPUI element/event types, SFTP file-type checks, sort models, row/table
  helpers and compact toolbar helpers explicitly.
- `pages/transfers/entry_row.rs` no longer depends on the transfers page
  wildcard import; SFTP entry rows now name GPUI event/element traits, SFTP
  entry types, transfer column/rename state, path formatting helpers and
  transfer icon rendering explicitly, and the transfers parent imports its row
  helpers by name.
- `pages/transfers/queue.rs` no longer depends on the transfers page wildcard
  import; the transfer queue panel now names GPUI element/focus traits, transfer
  job state, text truncation, queue row/action helpers and chrome tooltip/header
  dependencies explicitly.
- The upload, favorites and context transfer browser overlays no longer depend
  on the transfers page wildcard import; their menu positioning, GPUI event and
  element traits, transfer prompt/path models and local menu helpers are named
  at the file boundary.
- The delete/move, unknown-file and properties transfer overlays no longer
  depend on the transfers page wildcard import; dialog sizing, path helpers,
  SFTP entry/type models, properties widgets and action buttons are imported
  explicitly at each overlay boundary.
- The transfer job menu/delete overlay and create-file/folder/symlink overlay no
  longer depend on the transfers page wildcard import; queue predicates,
  permission helpers, file-name validation, GPUI element traits and action
  button dependencies are explicit.
- `pages/transfers/properties.rs` no longer depends on the transfers page
  wildcard import, including its test module; properties runtime now names its
  SFTP attribute/service types, transfer job events, properties input model and
  path/permissions helpers explicitly.
- The transfer `file_ops` and `editor` subtrees no longer use wildcard imports.
  Their nine module/leaf boundaries now name GPUI contexts, transfer state and
  job models, SFTP services, editor helpers, path helpers and standard-library
  runtime types directly; the transfers page parent no longer carries those
  dependencies as an implicit prelude.
- The transfer path bar, its tests and the editor overlay now use explicit
  imports. This clears `use super::*` from the entire `pages/transfers` tree;
  the parent module now imports only the five GPUI items and app type used by
  its own transfer page composition, and the boundary script governs the whole
  tree.
- The remote process page and its `data`, `details`, `resources` and `table`
  modules no longer use wildcard imports. The process module entry point now
  names the page API it re-exports instead of flattening every child symbol,
  and the remote page parent no longer carries process-only models or GPUI
  input types.
- The Docker/Compose pages and adjacent remote stats page no longer use
  wildcard imports. Docker's two module layers expose named composition APIs;
  project, service, menu and status modules import each other directionally,
  while the top-level view sees only the panels and matchers it composes. The
  stats page imports `usage_color` from the process module directly, leaving
  `pages/remote/mod.rs` as a pure module entry point. All 13 wildcard imports
  were removed across roughly 3,800 lines, so the boundary script now governs
  the complete `pages/remote` tree.
- The complete settings page tree no longer uses wildcard imports. Its AI,
  security, translation, cloud-sync/backup, transfer, terminal and workspace
  modules name their GPUI traits, settings form helpers, text-input setup,
  theme, core and model dependencies directly. The nested AI and sync/backup
  `mod.rs` files are pure module entry points, and `settings/mod.rs` no longer
  carries child-only AI, translation, cloud-sync or widget imports. The
  boundary script governs all 27 Rust modules under `pages/settings` at zero
  `use super::*` imports. Cloud-sync provider fields, secret masking,
  validation and request/storage behavior are unchanged.
- The quick-command import runtime no longer uses wildcard imports or flattened
  child-module globs. Its dialog calls the source adapter directly; source
  parsing depends directionally on JSON, merge and helper modules; merge names
  the core quick-command compatibility models; and the seven existing tests
  import only the parser/merge API they exercise. `QuickCommandsConfig` also
  left `features/prelude.rs`. NyaTerm, WindTerm and Xshell input shapes, the
  4 MiB limit, merge behavior and storage path are unchanged.
- The complete `features/commands` tree is now free of wildcard imports. The
  command history, suggestion hot path, quick-command catalog/dialog/editor/run
  modules and their tests name their GPUI, core, model and sibling-helper
  dependencies directly; both runtime `mod.rs` layers are narrow composition
  entry points. Seven command-suggestion helpers also left
  `features/prelude.rs`, and the architecture script governs the whole subtree
  at zero `use super::*` imports. Command execution, history persistence,
  suggestion timing/search behavior and quick-command compatibility are
  unchanged.
- The complete `features/ai` tree is now free of wildcard imports. AI agent,
  chat, discovery, history, job and settings modules name their GPUI, core,
  transport, model and sibling-helper dependencies directly; all four nested
  `mod.rs` files are pure module entry points. Six AI/core/transport symbols
  also left `features/prelude.rs`, and internal job and settings helpers are
  imported from their owning modules instead of being flattened through the AI
  parent. The architecture script governs all 15 Rust modules under
  `features/ai` at zero `use super::*` imports. AI requests, streaming events,
  audit, risk decisions, model discovery, background execution and history
  storage behavior are unchanged.
- The complete `features/settings` runtime tree is now free of wildcard
  imports. Configuration backup and portable snapshot flows, diagnostics,
  updates, settings drafts, terminal/interaction/recording/transfer settings,
  search engines, and SSH key/password/credential/OTP security runtimes name
  their GPUI, core, transport, model and sibling-helper dependencies directly.
  The three runtime `mod.rs` layers no longer act as implicit preludes, and
  `export_diagnostics_archive` also left `features/prelude.rs`. The architecture
  script governs all 19 Rust modules under `features/settings` at zero
  `use super::*` imports. Persistence calls, validation order, secret masking,
  credential encryption, backup/snapshot compatibility and settings behavior
  are unchanged.
- The complete `features/inspector` tree is now free of wildcard imports. The
  AI ask composer, transcript, message bubbles, command cards, agent steps,
  history/execution overlays, command history and right-panel shell name their
  GPUI, core, model, formatting, widget and runtime dependencies directly. Both
  `mod.rs` layers are pure module entry points, the helper export is named, and
  the empty `impl NyaTermApp {}` placeholder was removed. Three AI core types
  and `svg_icon_button` also left `features/prelude.rs`. The architecture
  script governs all 12 Rust modules under `features/inspector` at zero
  `use super::*` imports. Inspector rendering, AI interaction and command
  execution behavior are unchanged.
- The complete 39-module `features/panels` tree is now free of `use super::*`
  imports. Thirty-six overlay, quick-command, send-command, tab-action,
  recording, sync-group and session panel modules name their GPUI, core,
  transport, model, widget and sibling-helper dependencies directly. The
  panels and tab-actions module entry points also use named helper imports
  instead of local glob chains. Eight low-frequency GPUI, transport,
  send-command, shortcut and widget symbols left `features/prelude.rs`, and the
  architecture script governs the whole subtree at zero parent-module
  wildcard imports while keeping module-entry helper imports named.
  Panel rendering, focus/keyboard routing, session actions, quick-command
  execution and send-command formatting/dispatch behavior are unchanged.
- The complete 16-module `features/session` tree is now free of
  `use super::*` imports, including the five nested test modules that previously
  relied on those wildcards. Session
  lifecycle and ordering, startup restore, prompt brokers, credential
  autofill, recording, temporary SSH links, and TRZSZ/ZMODEM runtime modules
  name their GPUI, core, transport, model, formatting and sibling dependencies
  directly. Twenty-three session-specific core, transport, standard-library
  and credential-autofill model symbols also left `features/prelude.rs`. The
  architecture script governs the whole subtree at zero parent-module wildcard
  imports and prevents those prelude exports from returning. SSH host-key and
  credential prompting, OTP lookup, credential autofill, session startup and
  restore, recording, and TRZSZ/ZMODEM behavior are unchanged.
- The complete 18-module presentation-support set under `features/icons`,
  `features/formatting` and `features/view_widgets` is now free of
  `use super::*` imports. Icon compatibility tests, markdown parser tests and
  GPUI view helpers name their exact data, theme, formatting, widget and GPUI
  dependencies. The formatting, view-widget and top-level feature façades also
  use named re-exports instead of glob re-exports; twenty-three low-frequency
  aliases no longer flatten into `crate::features` and remain available only
  through their owning module paths. The architecture script governs all three
  trees and prevents their module-entry glob re-exports from returning.
  Persisted connection/quick-command icon keys, icon resolution order, markdown
  parsing and rendering, cloud-sync history rows, and shared widget behavior
  are unchanged.
- The complete background-service runtime set under `features/remote`,
  `features/sync` and `features/translation` is now free of `use super::*`
  imports. Sixteen module/state/runtime files name their GPUI, core, transport,
  HTTP, model, formatting, widget and sibling dependencies directly; the three
  feature entry points and cloud-sync runtime entry point are pure module
  declarations, and the remote runtime no longer imports its helper module as
  a glob. `CloudSyncError` and `DockerService` also left the shared prelude,
  while four low-frequency job/helper aliases no longer flatten into the
  top-level features façade. The architecture script governs all three trees
  at zero parent-module wildcard imports. Remote Docker/process/stats jobs,
  cloud-sync provider, history and conflict behavior, persisted sync and
  translation formats, and translation requests/rendering are unchanged.
- The complete ten-module `http/cloud_sync` adapter tree is now free of
  `use super::*` imports, including its shared helpers, provider compatibility
  tests and nested GitHub Gist OAuth tests. WebDAV, S3, Google Drive, OneDrive,
  Aliyun Drive and snippet adapters name their core, HTTP, hashing, JSON,
  standard-library and sibling-helper dependencies directly. The module entry
  point now contains only module declarations and named provider/OAuth
  re-exports; provider-specific endpoint constants live with their adapters.
  The architecture script governs the subtree at zero wildcard imports and
  prevents broad helper/provider re-exports from returning. Provider setting
  fields, request signing, OAuth refresh/device flow, remote path construction,
  secret handling and cloud-sync wire behavior are unchanged.
- The complete eleven-module `features/transfers` runtime tree is now free of
  `use super::*` imports, including transfer-event state tests. Browser event
  reconciliation, native path prompts, transfer options, SFTP list/CWD jobs,
  progress throttling, queue selection and upload/download control name their
  GPUI, core, transport, model, formatting and sibling dependencies directly.
  Both transfer module entry points are pure declarations plus deliberately
  named public presentation exports; fourteen transfer-runtime-specific symbols
  left `features/prelude.rs`, and two internal formatting helpers no longer
  flatten through feature façades. The architecture script governs the whole
  runtime tree at zero wildcard imports and prevents helper globs from
  returning. SFTP duplicate resolution, retries, cancellation, progress,
  browser session isolation, path selection and transfer protocol behavior are
  unchanged.
- `connection_runtime/helpers.rs` no longer depends on the connection runtime
  wildcard import; its GPUI, app, model, and core dependencies are explicit.
- `connection_runtime/actions.rs` no longer depends on the connection runtime
  wildcard import and now routes deleted connection/group cleanup through
  `ConnectionFeatureState` methods.
- `connection_runtime/editor.rs` no longer depends on the connection runtime
  wildcard import; its GPUI, core, helper, app, and model dependencies are
  explicit.
- `shell/event_pump/bridge.rs` no longer depends on the shell/event-pump
  wildcard import; its app dependency is explicit.
- `shell/event_pump/helpers.rs` test coverage no longer depends on a nested
  wildcard import; the helper/constant surface used by those tests is explicit,
  and the file's guarded baseline is down to one remaining production wildcard.
- `connection_runtime` now lives under the `features/connections` module tree
  and uses normal `mod` declarations for helpers, actions, editor, and groups
  instead of `#[path = "..."]`.
- `connection_import_runtime` now lives under the normal
  `features/connections` module tree instead of being mounted from
  `features/mod.rs` with `#[path = "..."]`.
- The architecture boundary script now handles zero-match baselines correctly
  and locks the governed connections feature tree at zero `#[path = "..."]`
  declarations. It also locks the connection runtime parent, actions, editor,
  groups, and helpers at zero `use super::*` imports.
- Connections state now exposes semantic methods for import prompt state,
  connection editor draft/window lifetime, group editor lifetime, connection
  list search/sort/more-menu state and projections, selection, context menu
  lifetime, hover/drop target projections and cleanup, clear-all/delete/open
  confirmation lifetime, and list deletion cleanup. Pure state tests cover
  search key handling, sort cycling,
  menu close idempotence, click/toggle/range selection anchor rules, selection
  anchor cleanup, repeated drop-target updates, drop-position fallback, list
  runtime cleanup, deleted connection/group reference cleanup, editor close
  cleanup for draft secrets, popovers, and pending window state, editor
  icon/menu/password source/type/tab transitions, editor keyboard text input,
  new-group commit, toggle behavior, and proxy editor secret draft/error
  cleanup.
- `ConnectionListState` pure helper logic now lives in
  `features/connections/state/list_logic.rs`. The public
  `ConnectionFeatureState` list methods remain in `state.rs`, preserving the
  caller facade while keeping the list child private and reducing the monolithic
  state file by 368 lines of selection, search, sort, hover, drag/drop, and
  stale-reference cleanup helpers.
- Connection editor and connection group editor pure draft helper logic now
  lives in `features/connections/state/editor_logic.rs`. The public
  `ConnectionEditorFeatureState` and `ConnectionGroupEditorFeatureState`
  methods remain in `state.rs`, while editor lifecycle, menu, password source,
  tab, kind, toggle, keyboard input, path prompt, and group-name/error helpers
  are isolated behind the existing feature-state facade.
- `ConnectionFeatureState` list, import, editor, group-editor, confirmation,
  and network child state internals are private to the state module; governed
  production code enters through semantic `ConnectionFeatureState` methods
  instead of accessing child fields directly. The architecture boundary script
  rejects direct `connection_state.list.*`, `connection_state.import.*`,
  `connection_state.editor.*`, `connection_state.group_editor.*`,
  `connection_state.confirmations.*`, and `connection_state.network.*` access in
  governed features.
- Connection editor save-success UI cleanup now routes through
  `ConnectionFeatureState::finish_editor_save`. The runtime still owns
  validation, store persistence, reload, status text, and optional connection
  launch, while the feature state owns closing the editor, clearing popovers and
  pending window state, selecting the saved connection, and expanding its group.
- Connection editor runtime, window, inline-overlay lifecycle, menu keyboard
  handling, and editor panel rendering now use `ConnectionFeatureState` façade
  methods for draft, window, popover, menu highlight, title mode, and focus
  state. The `editor` child state and `ConnectionEditorFeatureState` type are
  private to the state module, and the architecture script guards governed
  features against reintroducing direct `connection_state.editor.*` access.
- Connection import runtime, overlay, and root rendering now use
  `ConnectionFeatureState` façade methods for dialog visibility, active path
  prompts, and focus handle access. The `import` child state is private, and
  the architecture script guards governed features against reintroducing direct
  `connection_state.import.*` access.
- Connection group editor name input and validation errors now route through
  `ConnectionFeatureState` methods instead of direct runtime draft mutation.
- Connection group editor runtime, modal view, page rendering, and snapshot
  publication now use `ConnectionFeatureState` façade methods for active draft,
  open state, and focus handle access. The `group_editor` child state is
  private, and the architecture script guards governed features against
  reintroducing direct `connection_state.group_editor.*` access.
- Connection delete, group-delete, group-open and clear-all confirmations now
  route through `ConnectionFeatureState` façade methods. The `confirmations`
  child state is private, so rendering and runtime actions no longer reach into
  `connection_state.confirmations.*` directly.
- Connection list refresh cleanup now runs after successful
  `refresh_store_from_runtime()` session reloads. Selection, range anchor,
  hover, pending hover, context menus, expanded groups, and drop target state
  are pruned against the loaded connection/group IDs through
  `ConnectionFeatureState` methods backed by its private list child.
- Selected-connection and visible-connection-id derivation now live behind
  `ConnectionFeatureState` and `state/list_logic.rs`. Callers that need those
  projections pass the persisted connection/group collections into
  `ConnectionFeatureState`; list filtering, sorting, expanded-group traversal
  and selection projection stay out of the app coordinator.
- Saved-connection group-tree derivation also lives behind
  `ConnectionFeatureState`; starting all connections in a folder and deciding
  whether the group context menu should show "open all" no longer duplicate the
  descendant traversal in `NyaTermApp` or page render code.
- Connection row hover intent, hover dismissal, and group hover transitions now
  route through `ConnectionFeatureState`, leaving the rows view to forward UI
  events instead of mutating transient list fields directly.
- Saved-group expansion now routes through `ConnectionFeatureState`; the
  architecture script guards against reintroducing direct list-child access in
  governed connections code.
- Network page UI state now sits behind `ConnectionFeatureState` façade methods
  for tab/menu/move-picker state, expanded-section reads, delete and group
  confirmations, tunnel/proxy editor lifetime, group/tunnel/proxy editor input,
  focus/error/cycle/toggle transitions, and deleted item reference cleanup. The
  `network` child state and `NetworkFeatureState` type are private to the state
  module; tunnel runtime actions, Network page rendering, and panel-count
  projection no longer access `connection_state.network.*` directly.
- `ConnectionFeatureFocus` no longer allocates an unused
  `network_group_editor` focus handle. Network group editing uses the text-input
  registry path instead of a dedicated modal focus handle, so keeping one in the
  connection focus bundle was a stale migration artifact.
- `NetworkFeatureState` pure helper logic now lives in
  `features/connections/state/network_logic.rs`. The public semantic methods
  remain in `state.rs`, while 485 lines of menu, move-picker, group editor,
  tunnel editor, proxy editor, and stale-reference cleanup helpers are isolated
  behind the existing feature-state facade.
- Network group deletion now routes stale UI cleanup through
  `NetworkFeatureState`: matching expanded sections, group dialogs, and
  deleted child tunnel/proxy menu, move-picker, delete-confirmation, and editor
  references are cleared after the existing persistence operation succeeds.
- `tunnel_runtime` now lives under the normal `features/tunnels` module tree,
  its helpers/actions/groups/tunnel editor/proxy editor files use normal `mod`
  declarations, and all files in that runtime submodule use explicit imports
  instead of `use super::*`.
- `features/prelude.rs` no longer re-exports tunnel transport config/mode types
  for desktop feature modules that now import them directly.
- Network page view modules now live under the normal
  `features/pages/tunnels` module tree. The parent, proxy, tunnel, and leaf
  modules use normal `mod` declarations and explicit imports instead of
  `#[path = "..."]` or `use super::*`.
- `features/prelude.rs` no longer re-exports Network page UI model types; state,
  runtime, panel chrome, and view modules import those models directly.
- `scripts/check-architecture-boundaries.sh` locks the governed Network page
  view tree at zero `#[path = "..."]` declarations and zero `use super::*`
  imports.
- Connections page view/editor modules now live under the normal
  `features/pages/connections` module tree. The parent, editor, connection
  editor sections, view, and leaf modules use normal `mod` declarations and
  explicit imports instead of `#[path = "..."]` or `use super::*`.
- Connection list interaction modules now live fully under the normal
  `features/connections/connections` module tree. The drag payload/rendering
  parent and the DnD, menu, and selection action modules are governed at zero
  `use super::*` imports.
- The `features/pages/mod.rs` module entry no longer imports the whole parent
  prelude and is governed at zero `use super::*` imports.
- Governed connections runtime/page test modules now use explicit imports
  instead of indented `use super::*`; the architecture script now catches both
  top-level and nested wildcard imports in governed scopes.
- `features/prelude.rs` no longer re-exports the connection page UI model types
  that are now imported directly by the connections page tree.
- `scripts/check-architecture-boundaries.sh` locks the governed connections
  page tree at zero `#[path = "..."]` declarations and zero `use super::*`
  imports.
- `scripts/check-architecture-boundaries.sh` also rejects direct
  `connection_state.list.*` access in governed features. Selection writes and
  governed list-state reads now go through `ConnectionFeatureState` methods,
  including connection page rendering, context menus, root more-menu dismissal,
  and event pump sideband projections.
- `.github/workflows/architecture-boundaries.yml` runs the architecture boundary
  script on pull requests and pushes to `main`, so governed debt checks have a CI
  entry point.
- `nyaterm-desktop` crate root no longer re-exports `NyaTermApp`; external
  workspace consumers should use the narrower `AppShell` entry point or the
  explicit module path.
- `nyaterm-transport/src/session_types.rs` now owns the public session
  info/kind/event, drain stats, session error, and `TerminalTransport` trait
  definitions. `nyaterm-transport/src/lib.rs` re-exports the same names to
  preserve the public facade while reducing the transport root's mixed
  type/runtime responsibility.
- `nyaterm-transport/src/sftp_transfer_types.rs` now owns SFTP transfer
  summary/progress types, buffer/retry option clamps, duplicate-policy parsing,
  duplicate request/decision types, and their pure tests. `lib.rs` re-exports
  the same public names and keeps the actual SFTP transfer loops, retry
  execution, conflict resolution, and protocol operations in place.
- Five low-frequency transport helpers were removed from
  `features/prelude.rs` and imported explicitly at their call sites:
  `SFTP_TRANSFER_CANCELLED`, `SftpTransferDirection`, `RemoteStatsService`,
  `open_ssh_multiplex_handle`, and `run_local_command`. This narrows import
  reach without changing runtime behavior.
- Nine low-frequency core keyword-highlight, search-engine, mouse-report, and
  terminal-resize exports were also moved out of `features/prelude.rs` and
  imported explicitly by their owning modules. The terminal change is import
  plumbing only; parser, snapshot, input, and resize behavior are unchanged.
- Six additional low-frequency AI, cloud-sync, and diagnostics core types were
  moved out of `features/prelude.rs` and imported by their owning modules. This
  keeps background job and diagnostics dependencies explicit without changing
  job execution or data formats.
- Ten low-frequency AI redaction, model-output parsing, and agent command
  helpers were moved out of `features/prelude.rs` and imported explicitly by
  the AI runtime/job modules. This is import plumbing only; AI job execution,
  audit persistence, terminal command capture semantics, and model-output
  parsing behavior are unchanged.
- Nineteen low-frequency cloud-sync local snapshot/history helpers,
  snippet backends, and native HTTP remote adapters were moved out of
  `features/prelude.rs` and imported explicitly by the sync/app-state modules.
  This is import plumbing only; cloud sync execution, backup compatibility,
  serialized fields, history format, and remote protocol behavior are
  unchanged.
- Six low-frequency AI model discovery/chat HTTP facade helpers and model ID
  helpers were moved out of `features/prelude.rs` and imported explicitly by
  the AI runtime/job modules. This is import plumbing only; AI request
  execution, model discovery behavior, saved AI settings fields, and chat
  history persistence are unchanged.
- Four additional AI context, session-history, audit-request, and model
  discovery merge symbols were moved out of `features/prelude.rs` and imported
  explicitly by the AI/app-state/quick-command modules. This is import plumbing
  only; AI context construction, audit writes, session history loading, and
  model discovery merge behavior are unchanged.
- Three low-frequency GitHub Gist auth UI model types were moved out of
  `features/prelude.rs` and imported explicitly by app-state and cloud sync auth
  runtime modules. This is import plumbing only; GitHub Gist auth event
  handling, token draft updates, and sync provider behavior are unchanged.
- Two low-frequency cloud sync prompt UI model types were moved out of
  `features/prelude.rs` and imported explicitly by app-state, settings draft,
  config runtime, and cloud sync prompt modules. This is import plumbing only;
  secret-draft masking/clearing, conflict prompt behavior, cloud sync execution,
  serialized fields, encryption, and backup compatibility are unchanged.
- Five low-frequency translation settings/result/UI draft symbols were moved
  out of `features/prelude.rs` and imported explicitly by app-state, settings,
  runtime-job, and translation modules. This is import plumbing only;
  translation settings persistence fields, secret-draft handling, and
  translation job execution are unchanged.
- The low-frequency search-engine editor field UI model was moved out of
  `features/prelude.rs` and imported explicitly by app-state and settings
  runtime modules. This is import plumbing only; terminal search-engine
  settings persistence and editor behavior are unchanged.
- The low-frequency settings-tab UI model was moved out of
  `features/prelude.rs` and imported explicitly by app-state, settings page,
  security unlock, cloud sync, AI inspector, and panel-stack modules. This is
  import plumbing only; active settings tab selection, settings navigation, and
  settings persistence behavior are unchanged.
- Five low-frequency activity-bar and active-session menu UI model types were
  moved out of `features/prelude.rs` and imported explicitly by app-state,
  shell, sidebar, and activity-bar modules. This is import plumbing only;
  persisted activity-bar setting fields and chrome interaction behavior are
  unchanged.
- The low-frequency right-panel focus UI model was moved out of
  `features/prelude.rs` and imported explicitly by app-state, shell navigation,
  panel stack, activity bar, and terminal actions overlay modules. This is
  import plumbing only; right-panel selection, recording focus, and chrome
  interaction behavior are unchanged.
- Three low-frequency terminal action-link tooltip/menu UI model types were
  moved out of `features/prelude.rs` and imported explicitly by app-state and
  terminal selection runtime modules. This is import plumbing only; terminal
  parser, snapshot, protocol, selection matching, and action-link interaction
  behavior are unchanged.
- Two low-frequency diagnostics path prompt UI model types were moved out of
  `features/prelude.rs` and imported explicitly by app-state and diagnostics
  runtime modules. This is import plumbing only; diagnostics archive export
  options, path prompt behavior, and archive contents are unchanged.
- Three low-frequency remote Docker UI model types were moved out of
  `features/prelude.rs` and imported explicitly by app-state, remote runtime,
  and remote page modules. This is import plumbing only; Docker command
  execution, SSH transport, and Docker overview/action behavior are unchanged.
- Three low-frequency remote process UI model types were moved out of
  `features/prelude.rs` and imported explicitly by app-state, remote runtime,
  and remote page modules. This is import plumbing only; remote process polling,
  signal dispatch, sorting behavior, and SSH transport behavior are unchanged.
- Six low-frequency layout resize UI model types were moved out of
  `features/prelude.rs` and imported explicitly by app-state, root layout, and
  shell resize runtime modules. This is import plumbing only; panel sizing,
  bottom-panel/transfer resize, workspace split resize, layout persistence, and
  drag interaction behavior are unchanged.
- The low-frequency bottom-panel mode UI model was moved out of
  `features/prelude.rs` and imported explicitly by app-state, layout, panel,
  and shell runtime modules. This is import plumbing only; quick-command and
  command-send visibility, panel height persistence, and shortcut behavior are
  unchanged.
- The low-frequency main-mode UI model was moved out of `features/prelude.rs`
  and imported explicitly by app-state, settings-window, root, session,
  terminal UI routing, and shell navigation modules. This is import plumbing
  only; workspace/page switching, snapshot labels, session focus routing, and
  settings-window behavior are unchanged.
- Three low-frequency chrome menu UI model types were moved out of
  `features/prelude.rs` and imported explicitly by app-state, title-bar menu,
  and tab-actions overlay modules. This is import plumbing only; title menu
  opening, submenu positioning, tab action menus, and session action behavior
  are unchanged.
- The low-frequency quick switch item UI model was moved out of
  `features/prelude.rs` and imported explicitly by the quick switch runtime and
  overlay modules. This is import plumbing only; `OverlayStore` quick switch
  ownership, item filtering, selected-index clamping, and launch behavior are
  unchanged.
- Three low-frequency keyword-highlight settings/import UI model types were
  moved out of `features/prelude.rs` and imported explicitly by app-state,
  settings, and keybinding runtime modules. This is import plumbing only;
  keyword-highlight settings persistence fields, import path-prompt handling,
  and rule import/merge behavior are unchanged.
- Two low-frequency recording path-prompt UI model types were moved out of
  `features/prelude.rs` and imported explicitly by app-state, recording runtime,
  and recording panel modules. This is import plumbing only; recording manager
  start/stop behavior, transcript save behavior, and selected path handling are
  unchanged.
- Two low-frequency quick-command import path-prompt UI model types were moved
  out of `features/prelude.rs` and imported explicitly by app-state,
  quick-command import runtime, and quick-command import overlay modules. This
  is import plumbing only; quick-command import parsing, merge behavior, saved
  quick-command fields, and persistence calls are unchanged.
- Four low-frequency recording pipeline and terminal-history search UI model
  types were moved out of `features/prelude.rs` and imported explicitly by
  app-state and terminal search runtime modules. This is import plumbing only;
  recording writer lifecycle, history-search request polling, and terminal
  search result handling are unchanged.
- Six low-frequency quick-command menu/dialog UI state types were moved out of
  `features/prelude.rs` and imported explicitly by app-state, quick-command
  dialog runtime, and quick-command panel/overlay modules. This is import
  plumbing only; quick-command editor opening, delete confirmations, details
  placement, category rename/delete state, and row/category menu behavior are
  unchanged.
- Two low-frequency quick-command view/sort UI mode types were moved out of
  `features/prelude.rs` and imported explicitly by app-state, quick-command
  runtime helpers/catalog, and quick-command panel modules. This is import
  plumbing only; persisted quick-command UI setting strings, sort ordering, view
  layout selection, and settings save behavior are unchanged.
- Two low-frequency transfer path prompt UI model types were moved out of
  `features/prelude.rs` and imported explicitly by app-state, transfer path
  runtime, and trzsz runtime modules. This is import plumbing only; native path
  picker options, prompt status strings, selected-path handling, and upload and
  download dispatch behavior are unchanged.
- Eight low-frequency transfer browser transient UI state types were moved out
  of `features/prelude.rs` and imported explicitly by app-state and the transfer
  page modules. This is import plumbing only; browser column width defaults,
  drag selection, context/favorites/upload menus, pending rename, per-session
  cache, and transfer browser interaction behavior are unchanged.
- Six low-frequency transfer file-operation UI state types were moved out of
  `features/prelude.rs` and imported explicitly by app-state and the transfer
  page modules. This is import plumbing only; new file/folder/symlink dialogs,
  rename state, properties editing draft, unknown-file prompts, and transfer
  file-operation behavior are unchanged.
- Five low-frequency transfer operation UI state types were moved out of
  `features/prelude.rs` and imported explicitly by app-state, transfer runtime,
  and transfer page modules. This is import plumbing only; transfer delete/move
  confirmations, job menu/delete prompts, external-sync prompts, and transfer
  operation behavior are unchanged.
- The low-frequency transfer editor workspace UI state type was moved out of
  `features/prelude.rs` and imported explicitly by app-state and the transfer
  page module. This is import plumbing only; editor tab tracking, active-tab
  selection, and remote editor lifecycle behavior are unchanged.
- The low-frequency transfer input focus UI model was moved out of
  `features/prelude.rs` and imported explicitly by app-state, transfer path
  runtime, terminal input adapter, and the transfer page module. This is import
  plumbing only; transfer path focus selection and text-entry behavior are
  unchanged.
- Two low-frequency transfer browser sort UI models were moved out of
  `features/prelude.rs` and imported explicitly by app-state and the transfer
  page module. This is import plumbing only; browser sort column selection,
  default direction, and entry ordering behavior are unchanged.
- Three low-frequency snapshot password prompt/store status UI models were
  moved out of `features/prelude.rs` and imported explicitly by app-state,
  settings, sync prompt, layout prompt, and settings page modules. This is
  import plumbing only; .nya snapshot prompt flow, store status strings, and
  config/cloud-sync execution behavior are unchanged.
- Two low-frequency sync-input and multi-line-paste UI draft models were moved
  out of `features/prelude.rs` and imported explicitly by app-state,
  sync-input, terminal paste, and paste overlay modules. This is import
  plumbing only; sync group membership behavior, paste normalization, and
  multi-line paste confirmation behavior are unchanged.
- Two low-frequency config path prompt UI models were moved out of
  `features/prelude.rs` and imported explicitly by app-state and settings
  config runtime modules. This is import plumbing only; config backup/import
  path prompt state, selected-path handling, and backup/import execution
  behavior are unchanged.
- Two low-frequency AI prepared-request/message-menu UI models were moved out
  of `features/prelude.rs` and imported explicitly by app-state, AI ask,
  transfer event, and AI message-menu modules. This is import plumbing only; AI
  request preparation, file-action handoff, and message context-menu behavior
  are unchanged.
- Four low-frequency AI settings/action editor UI models were moved out of
  `features/prelude.rs` and imported explicitly by app-state, AI settings
  runtime, terminal input, and AI settings view modules. This is import
  plumbing only; AI profile fields, action editing, credential editing, and
  request user-agent input behavior are unchanged.
- Two low-frequency command suggestion UI state models were moved out of
  `features/prelude.rs` and imported explicitly by app-state and command
  suggestion runtime modules. This is import plumbing only; command suggestion
  matching, rendering state, and terminal input tracking behavior are
  unchanged.
- The low-frequency AI detected-error UI state model was moved out of
  `features/prelude.rs` and imported explicitly by app-state, AI ask, and
  session event-pump modules. This is import plumbing only; terminal error
  detection, AI analysis request preparation, and notice dismissal behavior are
  unchanged.
- Four low-frequency quick-command editor and variable-prompt UI models were
  moved out of `features/prelude.rs` and imported explicitly by app-state,
  quick-command runtime, and quick-command overlay modules. This is import
  plumbing only; quick-command editing, variable parsing, prompt submission,
  and command execution behavior are unchanged.
- Eleven low-frequency security panel/editor UI models were moved out of
  `features/prelude.rs` and imported explicitly by app-state, security runtime,
  security panel, editor view, and panel-stack modules. This is import plumbing
  only; credential, SSH key, OTP, password storage formats, secret handling,
  unlock flow, and delete/save execution behavior are unchanged.
- Two low-frequency credential-autofill suggestion/pending UI state models were
  moved out of `features/prelude.rs` and imported explicitly by app-state and
  credential-autofill runtime modules. This is import plumbing only; prompt
  detection, match selection, saved credential storage, and autofill dispatch
  behavior are unchanged.
- Two low-frequency terminal context-menu UI state models were moved out of
  `features/prelude.rs` and imported explicitly by app-state and terminal
  context-menu runtime modules. This is import plumbing only; menu placement,
  submenu selection, terminal selection text handling, and terminal parser or
  protocol behavior are unchanged.
- The low-frequency terminal paint policy UI model was moved out of
  `features/prelude.rs` and imported explicitly by terminal view I/O runtime.
  This is import plumbing only; terminal parsing, snapshots, protocol logic,
  and paint policy resolution behavior are unchanged.
- The low-frequency terminal performance overlay UI model was moved out of
  `features/prelude.rs` and imported explicitly by terminal surface rendering
  and scroll helpers. This is import plumbing only; overlay state propagation,
  render-degradation notices, terminal parsing, snapshots, and protocol logic
  are unchanged.
- The low-frequency terminal performance mode UI model was moved out of
  `features/prelude.rs` and imported explicitly by terminal view I/O and
  action-link selection modules. This is import plumbing only; render
  degradation state, terminal parsing, snapshots, and protocol logic are
  unchanged.
- The low-frequency terminal protocol state model was moved out of
  `features/prelude.rs` and imported explicitly by terminal view I/O, terminal
  surface entity, and terminal buffer tests. This is import plumbing only;
  protocol-state derivation, terminal parsing, snapshots, and protocol behavior
  are unchanged.
- The low-frequency terminal selection model was moved out of
  `features/prelude.rs` and imported explicitly by app-state, terminal
  selection runtime, and terminal surface modules. This is import plumbing
  only; selection range construction, pointer selection updates, terminal
  parsing, snapshots, and protocol behavior are unchanged.
- The low-frequency terminal cell-position model was moved out of
  `features/prelude.rs` and imported explicitly by terminal selection metrics,
  selection runtime, and terminal surface tests. This is import plumbing only;
  cell mapping, selection range construction, terminal parsing, snapshots, and
  protocol behavior are unchanged.
- The low-frequency terminal frame buffer-text event model was moved out of
  `features/prelude.rs` and imported explicitly by the terminal buffer runtime.
  This is import plumbing only; frame event production, terminal parsing,
  snapshots, and protocol behavior are unchanged.
- The low-frequency terminal frame output event model was moved out of
  `features/prelude.rs` and imported explicitly by the terminal buffer runtime.
  This is import plumbing only; output frame coalescing, frame application,
  terminal parsing, snapshots, and protocol behavior are unchanged.
- The low-frequency terminal frame output submission model was moved out of
  `features/prelude.rs` and imported explicitly by the terminal buffer runtime.
  This is import plumbing only; output submission construction, terminal frame
  pipeline behavior, terminal parsing, snapshots, and protocol behavior are
  unchanged.
- The low-frequency terminal frame search event model was moved out of
  `features/prelude.rs` and imported explicitly by the terminal buffer runtime.
  This is import plumbing only; search frame application, terminal parsing,
  snapshots, and protocol behavior are unchanged.
- The low-frequency terminal frame snapshot event model was moved out of
  `features/prelude.rs` and imported explicitly by the terminal buffer runtime.
  This is import plumbing only; snapshot frame application, terminal parsing,
  snapshots, and protocol behavior are unchanged.
- The low-frequency terminal frame search key model was moved out of
  `features/prelude.rs` and imported explicitly by terminal search runtime and
  terminal buffer runtime. This is import plumbing only; search request
  construction, search result matching, terminal parsing, snapshots, and
  protocol behavior are unchanged.
- The low-frequency terminal frame pipeline model was moved out of
  `features/prelude.rs` and imported explicitly by app-state construction and
  app-state field definitions. This is import plumbing only; pipeline spawning,
  event delivery, terminal parsing, snapshots, and protocol behavior are
  unchanged.
- The low-frequency terminal UI output tail cap was moved out of
  `features/prelude.rs` and imported explicitly by the terminal buffer runtime.
  This is import plumbing only; output tail sizing behavior, terminal parsing,
  snapshots, and protocol behavior are unchanged.
- The low-frequency terminal search result currency helper was moved out of
  `features/prelude.rs` and imported explicitly by the terminal search runtime,
  terminal buffer runtime, and shell event pump. This is import plumbing only;
  search result acceptance, event delivery, terminal parsing, snapshots, and
  protocol behavior are unchanged.
- The low-frequency terminal snapshot geometry helper was moved out of
  `features/prelude.rs` and imported explicitly by the terminal buffer and
  view-I/O runtimes. This is import plumbing only; retained snapshot acceptance,
  terminal parsing, snapshots, and protocol behavior are unchanged.
- The low-frequency terminal expensive-interaction gating helper was moved out
  of `features/prelude.rs` and imported explicitly by terminal action-link and
  canvas rendering modules. This is import plumbing only; interaction gating,
  terminal parsing, snapshots, and protocol behavior are unchanged.
- The low-frequency terminal action-link matcher key helper was moved out of
  `features/prelude.rs` and imported explicitly by terminal action-link, canvas,
  buffer, and view-I/O modules. This is import plumbing only; matcher-key
  generation, terminal parsing, snapshots, and protocol behavior are unchanged.
- The low-frequency terminal frame event model was moved out of
  `features/prelude.rs` and imported explicitly by app-state and terminal buffer
  runtime modules. This is import plumbing only; event queue ownership, terminal
  parsing, snapshots, and protocol behavior are unchanged.
- Two low-frequency session runtime model types were moved out of
  `features/prelude.rs` and imported explicitly by app-state and session
  runtime modules. This is import plumbing only; session event draining,
  metadata registration, frame pipeline routing, and session ordering behavior
  are unchanged.
- The low-frequency startup command request model was moved out of
  `features/prelude.rs` and imported explicitly by app-state, session runtime,
  session lifecycle/dialog, and terminal startup-command modules. This is
  import plumbing only; startup command validation, scheduling delay,
  duplicated-session launch, and terminal input dispatch behavior are
  unchanged.
- The low-frequency startup command action UI model was moved out of
  `features/prelude.rs` and imported explicitly by app-state, session dialog,
  global shortcut, and tab/session overlay modules. This is import plumbing
  only; duplicate/multiplex action selection, dialog labels, startup-command
  validation, and dispatch behavior are unchanged.
- The low-frequency workspace smart-split UI model was moved out of
  `features/prelude.rs` and imported explicitly by the title menu and
  tab-window runtime. This is import plumbing only; smart split mode selection,
  tab tiling, active-tab preservation, and workspace layout persistence are
  unchanged.
- The low-frequency workspace tab split edge UI model was moved out of
  `features/prelude.rs` and imported explicitly by the tab-window runtime. This
  is import plumbing only; tab split layout mutation, workspace pane
  persistence, and terminal window layout persistence are unchanged.
- The low-frequency workspace tab dock-zone UI model was moved out of
  `features/prelude.rs` and imported explicitly by app-state, workspace page,
  and tab-window runtime modules. This is import plumbing only; drag/drop zone
  detection, tab docking, split layout mutation, and terminal window layout
  persistence are unchanged.
- The low-frequency terminal search-mode UI model was moved out of
  `features/prelude.rs` and imported explicitly by app-state, event-pump,
  terminal search, context-menu, surface, and view I/O modules. This is import
  plumbing only; search-mode selection, result routing, search refresh, and
  terminal paint behavior are unchanged.
- The low-frequency terminal window tree UI model was moved out of
  `features/prelude.rs` and imported explicitly by app-state, workspace page,
  tab-window runtime, action-link routing, and terminal buffer modules. This is
  import plumbing only; terminal-window layout persistence, split-tab routing,
  active-tab selection, and terminal buffer behavior are unchanged.
- The low-frequency workspace split-direction UI model was moved out of
  `features/prelude.rs` and imported explicitly by app-state, startup restore,
  tab actions, workspace runtime, tab-window runtime, and terminal buffer
  modules. This is import plumbing only; workspace split persistence, startup
  restore, split-tab commands, and terminal buffer behavior are unchanged.
- The low-frequency workspace pane tree UI model was moved out of
  `features/prelude.rs` and imported explicitly by app-state, startup restore,
  workspace runtime, workspace page, and terminal buffer modules. This is
  import plumbing only; workspace pane ownership, layout persistence, startup
  restore, and terminal buffer behavior are unchanged.
- The low-frequency workspace split compatibility alias was moved out of
  `features/prelude.rs` and imported explicitly by app-state. This is import
  plumbing only; workspace pane ownership, split layout persistence, and split
  mutation behavior are unchanged.
- The low-frequency send-command control-focus UI model was moved out of
  `features/prelude.rs` and imported explicitly by app-state, send-command
  runtime, and send-command bar controls. This is import plumbing only; control
  focus selection, count/interval input syncing, and send-command dispatch
  behavior are unchanged.
- The low-frequency send-command target UI model was moved out of
  `features/prelude.rs` and imported explicitly by app-state, send-command
  runtime, and send-command bar controls. This is import plumbing only; target
  selection, group-target labeling, compatible-session filtering, and command
  dispatch behavior are unchanged.
- The low-frequency send-command line-ending UI model was moved out of
  `features/prelude.rs` and imported explicitly by app-state, send-command
  runtime, and send-command bar state/controls. This is import plumbing only;
  line-ending selection, preview labeling, command-unit construction, and
  dispatch behavior are unchanged.
- The low-frequency send-command mode UI model was moved out of
  `features/prelude.rs` and imported explicitly by app-state, send-command
  runtime, and send-command bar state/controls. This is import plumbing only;
  mode selection, data-type coercion, count/interval defaults, and command-unit
  construction behavior are unchanged.
- The low-frequency send-command data-type UI model was moved out of
  `features/prelude.rs` and imported explicitly by app-state, send-command
  runtime, and send-command bar editor/state/controls. This is import plumbing
  only; text/hex selection, preview formatting, parsing, and command-unit
  construction behavior are unchanged.
- `nyaterm-terminal-gpui` no longer publicly glob-re-exports its `images`,
  `keywords`, and `paint` implementation modules. The existing named facade
  exports remain public; sibling modules still use the helpers internally.
- `nyaterm-core/src/storage/config_backup.rs` now owns the config backup info
  type and schema-neutral file validation/copy/write helpers. `storage.rs`
  re-exports `ConfigBackupInfo` and keeps redb validation, table definitions,
  serialized records, encryption paths, and portable snapshot application in
  the existing storage facade.
- `nyaterm-core/src/storage/keyword_highlights.rs` now owns keyword highlight
  import JSON parsing, rule normalization, and merge accounting. `storage.rs`
  still owns settings load/save persistence, so redb table names, settings
  field paths, encryption, backup, and legacy fallback behavior are unchanged.
- The legacy source checkout boundary is gone from both runtime and asset
  tooling. NyaTerm-owned full-color logos are committed canonical project
  assets marked `keep` in the icon manifest, while fetched icon sets retain
  their pinned upstream mappings. The architecture script rejects any return
  of the retired dashboard, inventory crate or local source checkout.

## Migrating

Connections are the active architecture convergence target.

Current ownership map:

| Area | Current owner | Kind | Notes |
| --- | --- | --- | --- |
| Saved connections | Private catalog in `NyaTermApp.connection_catalog` | Persisted domain state | Views receive read-only slices; grouped load/clear, copy refresh and recently-used replacement enter through `ConnectionCatalogState` methods. |
| Connection groups | Private catalog in `NyaTermApp.connection_catalog` | Persisted domain state | Same compatibility boundary and method-only owner as saved connections. |
| SSH keys | Private catalog in `NyaTermApp.security` | Secret-adjacent persisted catalog | The security feature is authoritative; consumers use a read-only slice and the catalog has no `Debug` implementation. |
| OTP entries | Private catalog in `NyaTermApp.security` | Secret-adjacent persisted catalog | Consumers use a read-only slice; do not log secrets or widen Debug exposure. |
| Saved passwords/credentials | Private catalog in `NyaTermApp.security` | Secret-bearing persisted catalogs | Credential autofill and security settings use read-only owner APIs; grouped store loads and clears replace all four catalogs together. |
| Serial ports | Private catalog in `NyaTermApp.connection_catalog` | Runtime/discovered state | Replaced through the catalog after session-manager discovery and never persisted. |
| Tunnel/proxy configs | Private catalog in `NyaTermApp.tunnel_state` | Persisted network config | Views use read-only slices; pure move/upsert/delete candidates and successful commits stay on `TunnelFeatureState`. The Network page UI remains separate transient state. |
| Queued saved-connection starts | `NyaTermApp.session.start` private queue | Transient session-start state | Admission, duplicate detection, draining and runtime cadence reads go through `SessionStartFeatureState` methods. |
| List search/sort/hover/selection/DnD | `NyaTermApp.connection_state` private list child | Temporary UI state | Runtime and rendering enter through `ConnectionFeatureState` methods; state is not persisted except sort setting remains synced to settings as before. |
| Connection import dialog | `NyaTermApp.connection_state` private import child | Temporary UI/runtime prompt state | File import still runs through existing runtime paths; runtime and rendering enter through `ConnectionFeatureState` methods. |
| Connection editor | `NyaTermApp.connection_state` private editor child | Editing draft/window UI state | Runtime key handling, window lifecycle, rendering popovers, and sideband projection use `ConnectionFeatureState` methods; draft remains separate from saved connection data. |
| Group editor | `NyaTermApp.connection_state` private group-editor child | Editing draft UI state | Draft remains separate from saved groups; runtime and rendering enter through `ConnectionFeatureState` methods. |
| Delete/open confirmations | `NyaTermApp.connection_state` private confirmations child | Temporary UI state | Rendering and runtime actions enter through `ConnectionFeatureState` methods; persisted data changes only after existing confirm actions run. |
| Network page UI | `NyaTermApp.connection_state` private network child | Temporary UI/editor state | Page rendering, editor focus, confirm/editor draft reads, and panel-count projection use `ConnectionFeatureState` methods. Persisted configs live in the private `TunnelFeatureState` catalog. |

This round changed desktop-side state ownership, UI state plumbing, and module
boundaries. Final reports should avoid broad statements that sound like no
persistence-related, terminal-adjacent, SSH/SFTP-adjacent, or transport-adjacent
files were touched. Use the narrower boundary below.

Recommended report wording: this round did not change persistence schemas or
compatibility contracts. redb table names, keys, serialized field names,
encryption formats, backup formats, and legacy fallbacks are unchanged. It also
did not change `nyaterm-terminal` parser/snapshot/protocol logic or
`nyaterm-transport` SSH/SFTP/transfer protocol execution logic.

For the final Chinese report, avoid broad behavior summaries. Use this wording
instead:
`没有改变持久化格式、表结构、序列化字段、加密或兼容逻辑；没有改变 nyaterm-terminal 的解析、快照或协议逻辑，也没有改变 nyaterm-transport 的 SSH/SFTP/传输协议执行逻辑。`

The terminal-adjacent desktop changes only route quick switch IME state through
the authoritative overlay Entity and add a test-only explicit import. That is a
desktop state ownership/UI adapter change, not a terminal parser, terminal
protocol, SSH/SFTP protocol, transfer protocol, or persistence-format change.
The tunnel/proxy runtime action files were touched only to route Network page
menu, editor, confirmation, and stale-reference UI state through
`ConnectionFeatureState` after the existing persistence operations succeed.
Tunnel/proxy config storage formats and transport execution paths continue to
use the existing behavior.

## Temporary Compatibility

- `nyaterm-store` remains a transitional persistence facade; storage
  implementation still lives in `nyaterm-core`.
- User-data compatibility readers remain in `nyaterm-core` beside legacy-format
  tests; they are independent of the removed source inventory/dashboard code.

## Architecture Debt

- `features/prelude.rs`, desktop `#[path = "..."]`, and desktop `use super::*`
  debt are cleared and guarded against reintroduction.
- `NyaTermApp` remains the dominant state owner. New state should move into a
  focused FeatureState or a deliberately authoritative Entity, not into new
  unrelated top-level fields.
- Connections module tree is still transitional. The current state grouping is
  real, and the connection runtime, connection list interaction, and
  connections page trees are now governed, but adjacent connections
  feature/runtime files still need staged cleanup.
- `core/storage.rs` and `transport/lib.rs` are down from 7,662 and 8,418 lines
  to 4,020 and 4,423. What is left in each is genuinely central: the store and
  its shared transaction/JSON/crypto helpers on one side, the session manager
  and session lifecycle on the other. Further extraction there would cut across
  real coupling rather than along a seam, and must still preserve public facade
  behavior and persistence/protocol compatibility.

## Forbidden To Add

- New `#[path = "..."]` declarations.
- New `use super::*` imports.
- New broad crate-root exports or replacement global preludes.
- New `NyaTermApp` fields without an ownership rationale.
- Dual authoritative state in both FeatureState and Entity Store.
- Migration inventory/dashboard code or dependencies on a local legacy source
  checkout.
- Undeclared persistence migrations, table/key renames, encryption prefix
  changes, or backup format changes.

Run `scripts/check-architecture-boundaries.sh` before review. The script keeps
historical baselines for the remaining governed rules, while desktop
`#[path]`, `use super::*`, and the shared feature prelude are enforced
crate-wide at zero. The GitHub Actions `Architecture Boundaries` workflow runs
this script for pull requests and pushes to `main`.

## Entity Ownership Migration

Do not migrate `ConnectionsStore` as the first authoritative Entity. Quick
switch overlay state has been migrated into `OverlayStore` as the first
authoritative Entity-owned UI state:

| Requirement | Quick switch / overlay candidate |
| --- | --- |
| Database format impact | None |
| Secret impact | None |
| Terminal hot-path impact | Limited to GPUI input adapter state reads/writes for the overlay; no terminal parser or renderer changes |
| Old field deletion | `quick_switch_open/query/marked_text/selected_index` removed from `NyaTermApp`; `quick_switch_focus` remains app-owned because it is a GPUI focus handle |
| Snapshot publish adjustment | Quick switch is no longer part of `OverlaySnapshot` publication |
| Main risk | Avoid reintroducing writable quick switch mirror fields in `NyaTermApp` or public writable `QuickSwitchState` fields |

Future overlay migrations should record the exact old write/read paths and
delete old fields in the same change that makes the Entity authoritative.

## Unwired Capabilities

`nyaterm-desktop` went from 104 dead-code warnings to 18. The 152 items removed
were the superseded migration render layer: the old `left_*_panel` /
`right_*_panel` tree that `panel_body`'s `*_view` dispatch replaced, the widget
helpers only it called, and the handlers only those widgets invoked.

The 18 that remain are deliberately kept, because they are not cruft. Each is a
capability that exists and in most cases is tested, but that nothing in the
product reaches. Deleting them would remove work and hide the gap; wiring them
up or dropping the feature is a product decision, not cleanup.

| Item | Evidence it is unfinished rather than dead |
| --- | --- |
| `TerminalWindowNode::split_tab_to_edge` + `SplitEdge` | Six tests in `models/tests_workspace.rs` cover splitting a tab to a named edge. No UI raises it. |
| `terminal_resize_geometry_for_bounds` (free fn + method) | Three tests pin the geometry maths. Nothing calls either form. |
| `credential_autofill_prompt_line_from_viewport` | Two tests cover prompt-line detection from a viewport. No caller. |
| `TerminalSurface::set_cursor_blink_visible` | One test. Cursor blink is driven another way today. |
| `apply_cursor_style` (`nyaterm-terminal-gpui`) | One test. Its only caller was the deleted `terminal_line_element`. |
| `SnapshotPasswordPromptKind::{Export, Import, CloudPush, CloudPull}` | 35 live match arms still handle these prompts; nothing raises one since the prompt entry points went. |
| `ConfigPathPromptKind::{Import, PortableExport}` | Same shape: the handling survives, the trigger does not. |
| `AiInputField::{BaseUrl, ApiKey}`, `TranslateInputField::TargetLanguage` | Editable fields the settings UI no longer routes to. |
| `SessionPaneState` payloads and its `Disconnected` variant | The pane state machine carries `request_id`, `name`, `kind`, `session_id` and `error` that no reader consumes. |
| `ActivityBarDragPayload::{zone, index}` | A drag carries where it came from; the drop handler ignores it. |
| `TerminalOutputRequest::RequestBufferText` | Buffer-text request variant with no producer. |

Treat this table as the migration's real remaining to-do list. It is more
useful than a dead-code count: a warning that stays at 18 and a warning that
creeps back to 104 mean very different things, and only the second is rot.

## Migration-Only Exit List

The legacy inventory crate, migration dashboard and local source checkout met
their removal conditions together. Compatibility readers required for existing
user data remain in `nyaterm-core`; only temporary API aliases remain to audit.

| Item | Current use | Default build | Removal condition | Replacement direction |
| --- | --- | --- | --- | --- |
| Temporary compatibility aliases | Migration convenience | Varies by module | All consumers use authoritative API | Remove alias and narrow exports |

## Suggested Order

The order below is deliberately different from earlier rounds. Narrowing
`features/prelude.rs` one symbol at a time produced little real encapsulation
while `features/mod.rs` still flattened every feature directory through
`#[path = "..."]`: a file could stop importing a symbol from the prelude and
still sit in the same crate-wide namespace, reachable from everywhere. Build the
real module tree first, then the remaining steps actually enforce something.

Items 1, 2, 4, 5, 6 and 7 are done. Item 3 remains active; what follows is the
honest remaining list.

1. Done. `#[path = "..."]` no longer appears in `nyaterm-desktop` or
   `nyaterm-terminal-gpui`, and a crate-wide guard keeps it that way.
2. Done. Desktop production modules and nested tests contain no
   `use super::*`; the crate-wide guard prevents the chain from returning. The
   compiler-confirmed final pass also removed `features/prelude.rs`, so modules
   cannot regain the same implicit dependency surface through a shared import
   bucket.
3. Largely done. `NyaTermApp` is down from 585 fields to 21, across eighteen
   feature-state structs. The latest cohesive cuts moved sixteen terminal
   command-assistance and credential-prompt fields into
   `TerminalFeatureState::assist`, then seventeen transient settings fields
   into four `SettingsFeatureState` children, then moved translation and native
   update channels, job state and dialogs into two authoritative feature-state
   owners, followed by cloud-sync configuration, secret drafts, history,
   conflict and GitHub device-flow state, recording and SSH-tunnel runtime
   resources with their job/UI lifecycle state, then the complete live session
   runtime with nested session-start, prompt and dialog ownership, then terminal
   presentation runtime followed by sync-input and screen-lock interaction
   lifecycles, and finally forty-seven window interaction fields under the
   shell's viewport, navigation, panel, chrome and workspace children. The
   latest batch then moved session event/restore coordination, command
   persistence runtime, diagnostic throttles, settings prompts, About state and
   remote-editor window ownership to focused states. The connection convergence
   batch then moved saved connection/group/serial catalogs, security catalogs,
   tunnel/proxy catalogs and queued saved-connection starts to their domain
   owners. The command convergence batch then unified the quick-command catalog
   and UI, command history and persistence worker under one owner. The
   migration-only exit batch then removed the inventory/service fields with the
   dashboard and its unused runtime store. The settings-catalog convergence
   batch then folded the last five direct compatibility settings/status fields
   into `SettingsFeatureState`, including pure tested ownership of staged
   master-password transitions. The remote-runtime convergence batch then
   unified Docker, process and host-stats job identity/channel/failure timing in
   typed state owned by each pane instead of twenty-one independently writable
   fields. The recording/sync-input encapsulation batch then made both owners
   method-only boundaries: fourteen previously writable fields became private,
   with recording job/prompt/pipeline transitions and sync-group reads routed
   through focused APIs. The tunnel encapsulation batch then made its runtime
   resources and four persisted catalogs method-only as well, moving pure
   catalog candidate construction and coupled group/member removal onto the
   owner while retaining the existing persistence adapter and formats. The
   connection-catalog encapsulation batch then made saved connections, groups
   and discovered serial ports private, routed all readers through slices, and
   moved grouped load/clear, recently-used replacement, multi-move ordering and
   group-cycle detection onto the owner. The security-catalog encapsulation
   batch then made SSH keys, OTP entries, saved passwords and credentials
   private, routed consumers through read-only slices, and grouped store refresh
   and failure clearing on `SecurityFeatureState` without widening
   secret-bearing `Debug` exposure. What remains at the composition root is
   stores, runtime and focused feature owners.
   Group by cohesion where a cluster exists; do not force the count down for
   its own sake.
   Method ownership is now moving too, which is what grouping the fields alone
   did not buy. The rule: if a method only reads and writes one feature state,
   it belongs on that state, and the `NyaTermApp` method becomes a forwarder
   that owns `cx.notify()`. That is enforced by the type system rather than by
   convention — a handler taking `&mut TransferBrowserState` cannot reach the
   session list no matter what a later edit tries. More than one hundred methods or
   self-contained transitions have moved this way across transfers, security,
   the shell, sync input, the send command bar, AI, quick commands, cloud sync,
   recording, session starts and terminal paste review;
   the transfer browser one made `TransferBrowserColumnResizeState` stop leaking
   into the page layer, while cloud sync made secret-field routing inaccessible
   outside its owner, and
   recording cleanup can no longer leave its manager, busy map and pipeline out
   of sync; recording action and path-prompt admission are atomic owner
   transitions, and sync-input views cannot mutate group/broadcast state while
   rendering. Tunnel/proxy views likewise cannot mutate persisted catalogs, and
   group/member removal commits together only after both existing store writes
   succeed. Connection views likewise cannot mutate saved catalogs or runtime
   serial discovery, and background recently-used updates can replace only an
   existing connection through the owner. Closing a pending session start
   cannot update its maps without
   also applying the active pending/failed fallback rules, and paste editing
   cannot mutate its UTF-8 cursor without also clearing stale selection/IME state.
   Shell viewport timing, panel drags, mutually-exclusive tab menus and pane
   ownership rebuilding now have the same property. Translation and native
   update job admission/event completion now also stay coupled to their own
   pending/status/result state. Their fields and job payloads are private, and
   translation settings replacement cannot desynchronize the secret draft or
   active target language because the duplicate target-language field is gone.
   Cloud sync now has the same method-only boundary: job-running, compatibility
   state, status and conflict are committed together on success or failure;
   settings drafts merge secret values on the owner; and GitHub device-flow
   events cannot mutate the token draft, gist id or cancellation identity from
   outside `CloudSyncFeatureState`.
   The three remote
   panes now share the same typed job lifecycle, so a stale Docker/process/stats
   event cannot clear the pending owner of a newer session job. Session dialogs
   now expose no writable fields at all: close-all confirmation clears tab menu
   ownership atomically, invalid rename submission keeps its dialog identity,
   and temporary-SSH close clears draft and error together. Those are the kinds
   of signals to look for.
   Two caveats worth keeping. Render helpers stay on the view even when they
   read one state — moving element construction onto a data struct trades one
   coupling for a worse one. And a method that reads a state plus `self.tr(...)`
   or a service is not a candidate; only move what is genuinely self-contained.
   The remaining `impl NyaTermApp` blocks are mostly this second kind.
4. Done. No store is a projection any more; the three that remain own real
   state. If a future domain wants Entity ownership, migrate it authoritatively
   rather than reintroducing a published read model.
5. Done for the two monoliths that motivated it. `core/storage.rs` and
   `transport/lib.rs` are split by domain rather than by individual type, which
   is what made the difference: pulling out a few pure type modules in earlier
   rounds barely moved the line count, whereas each domain module took its
   constants, record types, helpers, tests and crate dependencies with it.
   Table definitions, serialized records, encryption paths, backup formats,
   legacy fallback behavior, and the SSH/SFTP/X11 protocol paths are
   compatibility surface and stay unchanged. `models/terminal.rs` (4,995) and
   `terminal_surface_entity.rs` (4,524) are the next files of this size, but
   they are render hot paths and need a different approach than a domain cut.
   `core/ai.rs` is done too, down from 4,032 to 1,554.
6. Done. The final explicit-import pass removed `features/prelude.rs`; a
   crate-wide guard prevents a replacement shared feature prelude.
7. Done. The migration capability/service models had no consumer outside the
   retired dashboard, so their `nyaterm-core` crate-root exports and module were
   removed with that feature rather than kept as a compatibility facade.
8. Revisit `nyaterm-store` only after storage modules have clearer internal
   boundaries and consumers can move without changing persistence compatibility.

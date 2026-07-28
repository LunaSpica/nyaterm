# GPUI Migration Status

This document records the current GPUI migration boundaries and debt in
`nyaterm-desktop`. Keep dynamic counts here instead of in `AGENTS.md`.

Last updated from the working tree on 2026-07-28.

## Current Metrics

| Metric | Current value | Notes |
| --- | ---: | --- |
| `NyaTermApp` fields | 261 | Counted from `features/app_state/mod.rs`; down from 585, still transitional. |
| `impl NyaTermApp` blocks | 240 | Spread across 235 files under `crates/nyaterm-desktop/src`. |
| `#[path = "..."]` declarations in desktop | 0 | Cleared. Every directory is a real module; the boundary script fails on any new occurrence. |
| `use super::*` imports in desktop | 340 | Includes indented test-module imports; historical migration debt, do not add new occurrences. |
| `features/prelude.rs` rough exported-token count | 229 | Still a broad shared prelude; two hundred sixteen low-frequency transport/core/http/model/helper exports are now explicit imports. |
| Entity Store structs | 4 | `Runtime`, `WindowRuntime`, `StartupRestore`, `Overlay`. Each owns state the app does not. |
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
- `migration-dashboard` exists as an explicit desktop feature. Default desktop
  features are empty, so release/default builds do not enable the dashboard.
- Entity stores have a documented projection rule: `NyaTermApp` / FeatureState
  remains authoritative unless a specific migration explicitly changes that.
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
  imports any of the eleven quick command UI model types. The persisted
  `quick_commands` and `quick_command_categories` collections deliberately stay
  on `NyaTermApp`, as with connections.
- Remote page state is grouped into `RemoteOpsFeatureState` with one struct per
  pane: `docker`, `process` and `stats`. The three panes turn out to share the
  same refresh bookkeeping (job id, owning session, pending flag, failure
  streak, last refresh instant), which the fifty-four prefixed `NyaTermApp`
  fields hid. Their job channels are created inside
  `RemoteOpsFeatureState::new` because construction was their only other use.
- Security panel state is grouped into `SecurityFeatureState`: the four editors
  and their focus handles in `editors`, revealed passwords/credentials and
  generated OTP codes in `revealed`, and the master password prompt in
  `unlock`. Secrets themselves still live in `nyaterm-core`; this is the panel's
  view state only, cleared through the same paths as before.
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
  `view` runtime, `input` focus and IME, `selection` and mouse reporting,
  painted `layout` geometry, `menus`, and the split/tab `windows` tree. Parsing,
  snapshots and the wire protocol are untouched and stay in `nyaterm-terminal`
  and `nyaterm-transport`. `OverlaySnapshot` keeps its own
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

  What remains owns something the app does not: `RuntimeStore` (app runtime and
  native services), `WindowRuntimeStore` (the window runtime pump),
  `StartupRestoreStore` (the restore queue) and `OverlayStore` (quick switch
  state, authoritative since the earlier migration).
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
- The architecture script's local legacy-source-path allowlist now includes
  only the static icon vendoring script and manifest in addition to the existing
  migration inventory paths. These references are provenance for committed
  assets; default builds still must not depend on `./temp/nyaterm-tauri`.

## Migrating

Connections are the active architecture convergence target.

Current ownership map:

| Area | Current owner | Kind | Notes |
| --- | --- | --- | --- |
| Saved connections | `NyaTermApp.connections` | Persisted domain state | Loaded/refreshed from `ConnectionStore`; not duplicated inside `ConnectionFeatureState`. |
| Connection groups | `NyaTermApp.connection_groups` | Persisted domain state | Same compatibility boundary as saved connections. |
| SSH keys | `NyaTermApp.connection_ssh_keys` | Secret-adjacent persisted catalog | Keep out of broad UI state until a credential/security migration is tested. |
| OTP entries | `NyaTermApp.connection_otp_entries` | Secret-adjacent persisted catalog | Do not log secrets or widen Debug exposure. |
| Saved passwords/credentials | `NyaTermApp.connection_saved_passwords`, `connection_saved_credentials` | Secret-bearing persisted catalogs | Remain top-level for now due credential autofill and security settings consumers. |
| Serial ports | `NyaTermApp.connection_serial_ports` | Runtime/discovered state | Not persisted by this state grouping. |
| Tunnel/proxy configs | `NyaTermApp.tunnels`, `tunnel_groups`, `proxies`, `proxy_groups` | Persisted network config | UI overlay state moved under `connection_state.network`; config collections remain persisted domain state. |
| List search/sort/hover/selection/DnD | `NyaTermApp.connection_state` private list child | Temporary UI state | Runtime and rendering enter through `ConnectionFeatureState` methods; state is not persisted except sort setting remains synced to settings as before. |
| Connection import dialog | `NyaTermApp.connection_state` private import child | Temporary UI/runtime prompt state | File import still runs through existing runtime paths; runtime and rendering enter through `ConnectionFeatureState` methods. |
| Connection editor | `NyaTermApp.connection_state` private editor child | Editing draft/window UI state | Runtime key handling, window lifecycle, rendering popovers, and sideband projection use `ConnectionFeatureState` methods; draft remains separate from saved connection data. |
| Group editor | `NyaTermApp.connection_state` private group-editor child | Editing draft UI state | Draft remains separate from saved groups; runtime and rendering enter through `ConnectionFeatureState` methods. |
| Delete/open confirmations | `NyaTermApp.connection_state` private confirmations child | Temporary UI state | Rendering and runtime actions enter through `ConnectionFeatureState` methods; persisted data changes only after existing confirm actions run. |
| Network page UI | `NyaTermApp.connection_state` private network child | Temporary UI/editor state | Page rendering, editor focus, confirm/editor draft reads, and panel-count projection use `ConnectionFeatureState` methods. Tunnel/proxy configs remain in top-level persisted collections. |

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
- Entity stores are mostly read-model snapshots published from
  `NyaTermApp`/FeatureState by `event_pump/publish.rs`. Quick switch state is
  the first deliberately authoritative overlay Entity state.
- Some Entity stores expose mutation helpers for focused tests. Until a domain
  is explicitly migrated, these helpers are not proof that the Store is the
  production source of truth.
- Migration dashboard and legacy inventory remain development/migration aids.
  They must stay feature-gated when they depend on local legacy source paths.

## Architecture Debt

- `features/prelude.rs` still exports many business models, services, transport
  types, terminal helpers, and widgets. New or substantially edited modules
  should prefer explicit imports instead of relying on this prelude.
- `#[path = "..."]` and `use super::*` remain widespread. They are allowed as
  historical debt but should be removed from each area as that area is edited.
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
- Default-build dependencies on `./temp/nyaterm-tauri` or other local legacy
  source directories.
- Undeclared persistence migrations, table/key renames, encryption prefix
  changes, or backup format changes.

Run `scripts/check-architecture-boundaries.sh` before review. The script is
baseline-friendly: current historical debt is allowed, but additional
occurrences in the governed connections/network/event-pump scope fail. The
GitHub Actions `Architecture Boundaries` workflow runs this script for pull
requests and pushes to `main`.

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

| Item | Current use | Default build | Removal condition | Replacement direction |
| --- | --- | --- | --- | --- |
| `nyaterm-legacy` | Legacy inventory and compatibility support | Present as dependency; dashboard code is feature-gated | Migration dashboard no longer needs legacy source inventory | Keep only tested compatibility readers needed for user data |
| Migration Dashboard | Development migration tracking | Disabled by default feature set | GPUI migration inventory is complete and documented elsewhere | Static migration notes or external tooling |
| Legacy source path `./temp/nyaterm-tauri` | Inventory source root | Must not be required by default builds | Legacy inventory no longer used | Remove local-path scanning code |
| Projection-only Entity Stores | Read-model snapshots for UI domains | Enabled | Domain migrates to authoritative Entity ownership | Entity becomes source of truth or projection API is removed |
| Temporary compatibility aliases | Migration convenience | Varies by module | All consumers use authoritative API | Remove alias and narrow exports |

## Suggested Order

The order below is deliberately different from earlier rounds. Narrowing
`features/prelude.rs` one symbol at a time produced little real encapsulation
while `features/mod.rs` still flattened every feature directory through
`#[path = "..."]`: a file could stop importing a symbol from the prelude and
still sit in the same crate-wide namespace, reachable from everywhere. Build the
real module tree first, then the remaining steps actually enforce something.

Items 1, 3 and 4 are done. What follows is the honest remaining list.

1. Done. `#[path = "..."]` no longer appears in `nyaterm-desktop` or
   `nyaterm-terminal-gpui`, and a crate-wide guard keeps it that way.
2. Drop the `use super::*` chain in favor of explicit imports, starting with the
   leaf files of a directory and working up to its `mod.rs`. Now that each
   directory is a real module, this is what actually narrows visibility; before
   the module tree existed it only moved names between two flat namespaces.
   Note this one does not batch: every file needs its own dependency set
   resolved, so it is 355 small compiler-guided edits rather than one sweep.
3. Largely done. `NyaTermApp` is down from 585 fields to 284, across eight
   feature-state structs. What is left is a long tail — the biggest remaining
   domain is eighteen fields, and much of the rest is genuinely app-level
   (stores, runtime, services, persisted collections). Group by cohesion where
   a cluster exists; do not force the count down for its own sake.
   Method ownership is now moving too, which is what grouping the fields alone
   did not buy. The rule: if a method only reads and writes one feature state,
   it belongs on that state, and the `NyaTermApp` method becomes a forwarder
   that owns `cx.notify()`. That is enforced by the type system rather than by
   convention — a handler taking `&mut TransferBrowserState` cannot reach the
   session list no matter what a later edit tries. Twenty-two methods have
   moved this way across transfers, security, the send command bar, AI and
   quick commands; the transfer browser one made
   `TransferBrowserColumnResizeState` stop leaking into the page layer, which
   is the kind of signal to look for.
   Two caveats worth keeping. Render helpers stay on the view even when they
   read one state — moving element construction onto a data struct trades one
   coupling for a worse one. And a method that reads a state plus `self.tr(...)`
   or a service is not a candidate; only move what is genuinely self-contained.
   The remaining `impl NyaTermApp` blocks are mostly this second kind.
4. Done. No store is a projection any more; the four that remain own real
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
6. Reduce `features/prelude.rs` opportunistically while touching a module, not
   as standalone rounds. The remaining entries are genuinely shared types.
7. Tighten one more low-risk crate-root export after confirming there are no
   external workspace consumers. Prefer desktop/UI presentation crates before
   touching core or transport public APIs.
8. Revisit `nyaterm-store` only after storage modules have clearer internal
   boundaries and consumers can move without changing persistence compatibility.

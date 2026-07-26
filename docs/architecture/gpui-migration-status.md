# GPUI Migration Status

This document records the current GPUI migration boundaries and debt in
`nyaterm-desktop`. Keep dynamic counts here instead of in `AGENTS.md`.

Last updated from the working tree on 2026-07-26.

## Current Metrics

| Metric | Current value | Notes |
| --- | ---: | --- |
| `NyaTermApp` fields | 585 | Counted from `features/app_state/mod.rs`; still transitional and too broad. |
| `impl NyaTermApp` blocks | 236 | Spread across 233 files under `crates/nyaterm-desktop/src`. |
| `#[path = "..."]` declarations in desktop | 287 | Historical migration debt; do not add new occurrences. |
| `use super::*` imports in desktop | 355 | Includes indented test-module imports; historical migration debt, do not add new occurrences. |
| `features/prelude.rs` rough exported-token count | 230 | Still a broad shared prelude; two hundred fifteen low-frequency transport/core/http/model exports are now explicit imports. |
| Entity Store structs | 13 | Includes store handles/runtime stores and domain stores. |
| Snapshot structs | 9 | Workspace, session, overlay, settings, connections, transfer, AI, cloud sync, remote ops. |
| `replace_snapshot` methods | 9 | Entity stores are still primarily snapshot projections. |
| Store snapshot publish calls | 9 | Published from `features/shell/event_pump/publish.rs`. |

Large files currently over 4,000 lines:

| File | Lines | Status |
| --- | ---: | --- |
| `crates/nyaterm-transport/src/lib.rs` | 8418 | Split candidate; session type/event and SFTP transfer type/option extractions are complete; avoid behavior changes while extracting further pure modules. |
| `crates/nyaterm-core/src/storage.rs` | 7662 | Split candidate; schema-neutral config-backup and keyword-highlight import helper extractions are complete; schema compatibility is public contract. |
| `crates/nyaterm-desktop/src/models/terminal.rs` | 4995 | Split candidate; coordinate with terminal render/runtime ownership. |
| `crates/nyaterm-desktop/src/features/terminal/terminal_surface_entity.rs` | 4524 | Split candidate; avoid hot-path regressions. |
| `crates/nyaterm-core/src/ai.rs` | 4032 | Split candidate after storage/transport boundaries are clearer. |

Other files currently over 2,000 lines include terminal runtime/view modules,
transport transfer protocol modules, and terminal GPUI painting modules. Treat
these as staged extraction candidates, not as formatting-only refactor targets.

## Completed

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
- `connection_runtime/helpers.rs` no longer depends on the connection runtime
  wildcard import; its GPUI, app, model, and core dependencies are explicit.
- `connection_runtime/actions.rs` no longer depends on the connection runtime
  wildcard import and now routes deleted connection/group cleanup through
  `ConnectionListState` methods.
- `connection_runtime/editor.rs` no longer depends on the connection runtime
  wildcard import; its GPUI, core, helper, app, and model dependencies are
  explicit.
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
  `features/connections/state/list_logic.rs`. The public feature-state methods
  remain in `state.rs`, preserving the existing caller facade while reducing
  the monolithic state file by 368 lines of selection, search, sort, hover,
  drag/drop, and stale-reference cleanup helpers.
- Connection editor and connection group editor pure draft helper logic now
  lives in `features/connections/state/editor_logic.rs`. The public
  `ConnectionEditorFeatureState` and `ConnectionGroupEditorFeatureState`
  methods remain in `state.rs`, while editor lifecycle, menu, password source,
  tab, kind, toggle, keyboard input, path prompt, and group-name/error helpers
  are isolated behind the existing feature-state facade.
- `ConnectionFeatureState` child state internals are private to the state module;
  governed production code enters through semantic methods instead of accessing
  list/import/editor/group/confirmation/network fields directly.
- Connection editor save-success UI cleanup now routes through
  `ConnectionFeatureState::finish_editor_save`. The runtime still owns
  validation, store persistence, reload, status text, and optional connection
  launch, while the feature state owns closing the editor, clearing popovers and
  pending window state, selecting the saved connection, and expanding its group.
- Connection editor window and inline-overlay lifecycle checks now use
  `ConnectionEditorFeatureState` façade methods for active draft, window handle,
  pending state, modal ownership, title mode, and focus handle. The architecture
  script guards the root/window files against reintroducing direct
  `draft`/`window`/`window_open_pending` condition stitching.
- Connection editor runtime focus changes and the editor panel focus tracker now
  use `ConnectionEditorFeatureState::focus_handle()`. The architecture script
  guards governed editor runtime/view/projection paths against direct `focus`
  field reads.
- Connection import runtime, overlay, and root rendering now use
  `ConnectionImportState` façade methods for dialog visibility, active path
  prompts, and focus handle access. The architecture script guards those files
  against reintroducing direct `import_dialog_open`/`import_path_prompt`/
  `import_focus` field reads.
- Connection group editor name input and validation errors now route through
  `ConnectionGroupEditorFeatureState` methods instead of direct runtime draft
  mutation.
- Connection group editor runtime, modal view, page rendering, and snapshot
  publication now use `ConnectionGroupEditorFeatureState` façade methods for
  active draft, open state, and focus handle access. The architecture script
  guards those files against reintroducing direct `draft`/`focus` field reads.
- Connection list refresh cleanup now runs after successful
  `refresh_store_from_runtime()` session reloads. Selection, range anchor,
  hover, pending hover, context menus, expanded groups, and drop target state
  are pruned against the loaded connection/group IDs through
  `ConnectionListState`.
- Connection row hover intent, hover dismissal, and group hover transitions now
  route through `ConnectionListState`, leaving the rows view to forward UI
  events instead of mutating transient list fields directly.
- Saved-group expansion now routes through `ConnectionListState::expand_group`;
  the architecture script guards against reintroducing direct expanded-group
  insertion in governed connections code.
- Network page UI state now exposes semantic methods for tab/menu/move-picker
  state, expanded-section reads, delete and group confirmations, tunnel/proxy
  editor lifetime, group/tunnel/proxy editor input, focus/error/cycle/toggle
  transitions, and deleted item reference cleanup. Tunnel runtime actions and
  Network page rendering now use these methods instead of directly accessing
  `connection_state.network` fields for those transitions and projections.
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
- `scripts/check-architecture-boundaries.sh` also rejects direct mutating
  writes to `connection_state.list.selected_ids` and `last_selected_id` in the
  governed connections feature/page paths. Selection writes and governed
  list-state reads now go through `ConnectionListState` methods, including
  connection page rendering, context menus, root more-menu dismissal, and event
  pump sideband projections.
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
| List search/sort/hover/selection/DnD | `NyaTermApp.connection_state.list` | Temporary UI state | State is not persisted except sort setting remains synced to settings as before. |
| Connection import dialog | `NyaTermApp.connection_state.import` | Temporary UI/runtime prompt state | File import still runs through existing runtime paths. |
| Connection editor | `NyaTermApp.connection_state.editor` | Editing draft/window UI state | Runtime key handling, window lifecycle, rendering popovers, and sideband projection use state methods; draft remains separate from saved connection data. |
| Group editor | `NyaTermApp.connection_state.group_editor` | Editing draft UI state | Draft remains separate from saved groups. |
| Delete/open confirmations | `NyaTermApp.connection_state.confirmations` | Temporary UI state | Rendering and sideband projection now use state methods; persisted data changes only after existing confirm actions run. |
| Network page UI | `NyaTermApp.connection_state.network` | Temporary UI/editor state | Page rendering, editor focus, confirm/editor draft reads, and panel-count projection use state methods. Tunnel/proxy configs remain in top-level persisted collections. |

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
`NetworkFeatureState` after the existing persistence operations succeed.
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
- `core/storage.rs` and `transport/lib.rs` are still too broad to maintain
  comfortably. The first transport session type/event and SFTP transfer
  type/option extractions are complete, but further extraction must still
  preserve public facade behavior and persistence/protocol compatibility.

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

1. Replace the remaining `#[path = "..."]` declarations in
   `crates/nyaterm-desktop/src/features` with real nested modules, one feature
   directory per round. The files already live in the right directories, so this
   is mechanical and the compiler verifies every step. `features/shell` is done.
   Each converted directory gets a `check_no_matches` guard so the debt cannot
   return.
2. After a directory is a real module, drop its `use super::*` chain in favor of
   explicit imports, starting with the leaf files. Only then does the module
   boundary mean anything.
3. Split `NyaTermApp` along the pattern already validated by
   `ConnectionFeatureState`: grouped feature-state structs with private fields
   and semantic methods, covered by GPUI-free pure state tests. This is the
   highest-value remaining item; roughly 590 fields and 236 `impl NyaTermApp`
   blocks are the reason most desktop modules can reach most desktop state.
4. Decide the Entity Store question instead of leaving it half-migrated. Either
   move one or two more domains to authoritative Entity ownership, or delete the
   projection-only stores. Maintaining both a `NyaTermApp` field and a snapshot
   copy of the same state is the most expensive of the three options.
5. Continue extracting schema-neutral internal modules from `core/storage.rs`
   and schema/protocol-neutral modules from `nyaterm-transport/src/lib.rs`, by
   domain rather than by individual type. Table definitions, serialized records,
   encryption paths, backup formats, and legacy fallback behavior stay unchanged.
6. Reduce `features/prelude.rs` opportunistically while touching a module, not
   as standalone rounds. The remaining entries are genuinely shared types.
7. Tighten one more low-risk crate-root export after confirming there are no
   external workspace consumers. Prefer desktop/UI presentation crates before
   touching core or transport public APIs.
8. Revisit `nyaterm-store` only after storage modules have clearer internal
   boundaries and consumers can move without changing persistence compatibility.

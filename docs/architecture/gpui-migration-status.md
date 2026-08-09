# GPUI Migration Status

This document records the current GPUI migration boundaries and debt in
`nyaterm-desktop`. The native GPUI architecture is now the project baseline;
remaining entries here describe targeted parity, compatibility, ownership, and
cleanup work rather than a broad half-migrated application state. Keep dynamic
counts here instead of in `AGENTS.md`.

Last updated from the working tree on 2026-08-09.

## Tauri Main Parity Refreshes

- 2026-08-09 audited the active Tauri `main` delta from `f41e0d6d` to
  `b5900d70`. GPUI now carries the compatible quick-command export JSON shape,
  WindTerm quickbar escaped/real terminal-newline handling, cloud-sync
  `auto_pull_remote_changes` settings compatibility, cloud remote-check decision
  logic, the Sync Backup History push/pull shortcuts, persisted saved-connection
  expanded folder ids, and matching English/Simplified Chinese localization
  keys.
- The same Tauri range also changed pure React/WebView surfaces and a Tauri
  backend-only Claude Code subprocess runner. GPUI does not yet have the
  equivalent external Claude Code runtime, so the stdin-based invocation fix
  remains a targeted future AI parity task rather than WebView migration debt.
- Tauri saved-connections also added copy buttons inside the connection detail
  tooltip. GPUI already owns the detail tooltip surface, but the copy affordance
  remains a focused connection-list parity task.

## Current Metrics

| Metric | Current value | Notes |
| --- | ---: | --- |
| `NyaTermApp` fields | 20 | Counted from `features/app_state/mod.rs`; down from 585. The remaining fields are composition services and focused feature owners. |
| `impl NyaTermApp` blocks | 238 | Spread across 233 files. The additional block is the focused `view_io/input.rs` module boundary and adds no adapter method; method bodies and call sites, rather than the block count alone, remain the ownership check. |
| `#[path = "..."]` declarations in desktop | 0 | Cleared. Every directory is a real module; the boundary script fails on any new occurrence. |
| `use super::*` imports in desktop | 0 | Cleared in production and test modules; guarded crate-wide. |
| `use super::*` imports in terminal-GPUI | 0 | Cleared in production and test modules. The crate root is no longer a shared import bucket. |
| `features/prelude.rs` rough exported-token count | 0 | The transitional shared prelude is removed and guarded against reintroduction. |
| `cargo check -p nyaterm-app` desktop warnings | 0 | Cleared. The current cleanup either wired complete capabilities or removed superseded state and adapters. |
| Desktop clippy warnings | 5 | Stable Rust 1.97.1 reports four library warnings and one test-only warning. `cargo clippy --workspace --all-targets` still exits successfully. |
| Other workspace clippy warnings | 17 | Stable Rust 1.97.1 reports 9 core, 2 terminal, 5 UI and 1 terminal-GPUI warnings in unchanged source. Transport, OTP, store and app are clean. Cargo also reports the upstream `proc-macro-error2 v2.0.1` future-incompatibility notice. |
| Entity Store structs | 3 | `WindowRuntime`, `StartupRestore`, `Overlay`. Each owns state the app does not. |
| Snapshot structs | 0 | Cleared. No store is a projection of `NyaTermApp` any more. |
| `replace_snapshot` methods | 0 | Cleared. |
| Store snapshot publish calls | 0 | `publish.rs` and the publish throttle are gone. |

No Rust source file currently exceeds 3,000 lines. The largest production file
is the terminal desktop model at 2,647 lines, followed by terminal runtime
`buffer.rs` at 2,593. The former 4,000-line files now have these boundaries:

| Production file | Production lines | Test module lines | Notes |
| --- | ---: | ---: | --- |
| `crates/nyaterm-desktop/src/models/terminal.rs` | 2647 | 2270 | Frame-pipeline tests live in `models/terminal/tests.rs`; terminal cell and selection models remain in the production module. |
| `crates/nyaterm-desktop/src/features/terminal/terminal_surface_entity.rs` | 2475 | 2283 | The 54 focused tests live in `terminal_surface_entity/tests.rs`; the paint hot path and private entity boundary are unchanged. |
| `crates/nyaterm-transport/src/lib.rs` | 2547 | 990 | Crate-root session lifecycle tests live in `src/tests.rs` with explicit imports. Session configuration contracts, the bounded event queue, and SSH command/process execution live in focused modules; the crate root preserves their public facade. |
| `crates/nyaterm-core/src/storage.rs` | 1470 | 2618 | Compatibility tests live in `storage/tests.rs` with explicit imports. The facade, redb schema, encryption and fallback readers remain unchanged. |

Other large modules now keep their focused tests behind the same normal
child-module boundary:

| Production file | Production lines | Test module lines | Notes |
| --- | ---: | ---: | --- |
| `crates/nyaterm-desktop/src/features/terminal/terminal_runtime/view_io.rs` | 1606 | 869 | The 41 snapshot, scrolling, input-latency and key-encoding tests live in `view_io/tests.rs`; the 1240-line `view_io/input.rs` module owns keyboard, mouse, focus, wire-write, recording and encoding input adapters. |
| `crates/nyaterm-desktop/src/features/terminal/terminal_runtime/buffer.rs` | 2593 | 233 | The 10 frame-budget, search-apply, OSC52 and window-tab tests live in `buffer/tests.rs`; buffer application and surface notification logic remain together. |
| `crates/nyaterm-desktop/src/features/connections/state.rs` | 2106 | 1596 | The 60 list, editor and network owner-transition tests live in `state/tests.rs`; the focused `editor_logic`, `list_logic` and `network_logic` modules are unchanged. |
| `crates/nyaterm-desktop/src/features/transfers/state.rs` | 1354 | 788 | The 17 browser, queue, editor and external-sync owner tests live in `state/tests.rs`; the 621-line `state/browser.rs` and focused `state/editor.rs` module own their transitions while `TransferFeatureState` remains authoritative. |
| `crates/nyaterm-desktop/src/features/ai/state.rs` | 1788 | 669 | The 20 chat, settings, discovery, history and agent owner-transition tests live in `state/tests.rs`; the 967-line `state/settings.rs` module owns provider settings, model catalog, credential and custom-action transitions while `AiFeatureState` remains authoritative. |
| `crates/nyaterm-desktop/src/features/session/state.rs` | 2333 | 897 | The 16 restore, prompt, dialog and start-lifecycle owner tests live in `state/tests.rs`. |
| `crates/nyaterm-terminal/src/lib.rs` | 1966 | 1147 | The 75 terminal state-machine and snapshot tests live in `src/tests.rs` with explicit imports. |
| `crates/nyaterm-terminal/src/graphics.rs` | 1597 | 1289 | The 35 ingress, Kitty, iTerm2 and Sixel tests live in `graphics/tests.rs`; parser and graphics-state behavior remain in the production module. |
| `crates/nyaterm-core/src/cloud_sync.rs` | 1937 | 878 | The 19 remote, history, provider, conflict and encrypted snapshot tests live in `cloud_sync/tests.rs`; persistence and compatibility logic remain unchanged. |
| `crates/nyaterm-terminal-gpui/src/element.rs` | 1648 | 1168 | The 41 layout, shaping, row-cache and decoration tests live in `element/layout_cache_tests.rs`; GPUI layout and paint implementation remain together. |
| `crates/nyaterm-transport/src/trzsz.rs` | 1925 | 1214 | The 42 detector, protocol, download and upload engine tests live in `trzsz/tests.rs` with explicit imports. |

`core/ai.rs` was on this list at 4,032 lines; it is now 1,547 after being split
into `providers`, `agent`, `risk` and `settings`.

Other files currently over 2,000 lines include terminal runtime/view modules,
transport transfer protocol modules, and focused desktop feature state. Treat
these as staged extraction candidates, not as formatting-only refactor targets.

## Completed

- `gpui-component` infrastructure and ordinary-control migration are in place.
  The workspace uses vendored `gpui-component` `0.5.2` at
  `e1570bdc8fd2dc17d38cab09e74b1783bdf3b24b` and vendored Zed GPUI at
  `4aad57fd1f002f9feeea2b7fb6229ccbcd576cb1`. Dependency verification on
  2026-08-05 shows a single active path-based `gpui v0.2.2` from
  `vendor/zed/crates/gpui`. `nyaterm-ui` owns the component
  wrappers, theme bridge, and `NyaRoot`/`NyaWindowHandle` host aliases; desktop
  feature modules must continue importing NyaTerm wrapper types rather than
  `gpui_component` directly. The main window plus settings, connection editor,
  quick-command editor, remote editor, and external-sync child windows now open
  with `NyaRoot` as the first view layer while preserving typed window
  activation, stale-handle cleanup, pending-open guards, and close handlers.
  Startup, explicit theme changes, config imports and cloud-sync pulls keep
  `ThemePalette` authoritative by syncing component colors from the current
  NyaTerm palette after settings are loaded or changed.
  Phase 1 ordinary input migration is in place: the shared id-keyed
  `TextInputRegistry`, connection-list search, connection-editor fields, and
  connection group-name editor now own `nyaterm-ui::NyaInputState` and render
  `gpui-component` `Input` for ordinary form/search/prompt fields. The legacy
  `nyaterm-ui/src/text_field.rs` custom caret, hit-testing, selection and IME
  implementation has been removed; terminal input, paste review,
  `RemoteTextEditor` and other full editing surfaces keep their dedicated
  paths.
  The reusable button, switch, checkbox, radio, tab, select, menu, tooltip and
  dialog wrappers are exported only from `nyaterm-ui`. The common
  `small_button`, `mode_button` and settings `settings_choice_chip` helpers plus
  shared `dialog_action_button`
  now delegate to `nyaterm-ui::NyaButton`, the shared `svg_icon_button` and
  `modal_close_icon_button` helpers delegate to an asset-path based
  `nyaterm-ui::NyaIconButton`, and the tunnel row open/close control, shared
  settings switch helpers, Telnet option switches and keyword-highlight rule
  switches render controlled `nyaterm-ui::NyaSwitch` controls while
  feature/transport, connection-editor draft and settings state remain
  authoritative. The tunnel list and Telnet editor segmented tab controls now
  render through `nyaterm-ui::NyaTabs` while `NetworkTab` and
  `ConnectionEditorTelnetTab` remain authoritative. `nyaterm-ui::NyaSelectState`
  is now a real component-backed wrapper for string-valued selects, including
  optional NyaTerm font previews. The desktop `SelectRegistry` owns component
  popup/focus state while persisted settings own selected values. Appearance
  theme, terminal theme, contrast, wallpaper fit, font weight, cursor style,
  font-stack rows, and AI risk use this path; their manual menus and the shared
  settings `menu_open` mirror have been removed. Tunnel/proxy editor selectors,
  all four send-command selectors, and cloud-sync provider selection also use
  the registry. Their click-to-cycle/manual absolute menu implementations and
  pure-UI open flags were removed while draft, send-progress, and cloud-sync
  feature state remain authoritative for selected values.
  The title bar now hosts one `nyaterm-ui::NyaAppMenuBar` Entity instead of four
  independent dropdown triggers. The Entity is authoritative for active-menu,
  popup, and focus-restoration state; translated labels and the existing
  icon/shortcut/check-aware `NyaMenuItem` trees are resolved lazily from the
  application owner. Top-level hover switching, wrapping left/right navigation,
  Escape dismissal, and focus restoration are covered by component tests while
  the existing 40px custom window chrome remains authoritative for drag and
  platform window controls.
  Ordinary component dialogs now host CRUD, confirmation, import, translation
  result and update-check flows, including network group/tunnel/proxy dialogs,
  connection and quick-command confirmations, SFTP file-operation dialogs,
  Docker/process/security/AI/session confirmations, quick-command and
  connection import source pickers, translation results, and native update
  checks. Pure visual dialog state removed in the same passes includes import
  dialog open/focus fields, update dialog open state, AI confirmation booleans,
  close-all-session confirmation state, category rename focus and SFTP
  properties focus mirrors.
  Header-specific icon buttons, remote Docker tabs, settings sidebar tabs and
  terminal search's dedicated mode buttons remain custom until their broader
  domain passes.

- Ordinary form, prompt and search inputs are real widgets, not label divs.
  `nyaterm-ui::NyaInputState` delegates caret, selection, IME composition and
  clipboard behavior to `gpui-component::input::InputState` behind the
  `nyaterm-ui` wrapper boundary. Panels either own entities directly (the
  connection editor) or use the id-keyed registry in `features/text_inputs.rs`.
  Coverage includes settings and network editors, quick commands and
  send-command controls, security and SSH prompts, AI and sync fields, session
  overlays and filters, SFTP paths/search/dialogs/properties, terminal search,
  and keyword-highlight rules. The removed `transfer_input` helper is not a
  compatibility path.

  Full editing surfaces are separate by design: the terminal and paste review
  use terminal input routing, and `RemoteTextEditor` owns editor-specific
  selection, undo and command handling. The built-in transfer editor remains a
  dedicated editor surface, not a registry form field. None of these
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
- Domain child-window adapters no longer float at the `features/` root. The
  connection editor, quick-command editor and settings window now live under
  their owning runtime modules; the built-in remote editor, its dedicated text
  editing surface and the external-sync prompt window live under `transfers`.
  Their implementation types are no longer re-exported from `features/mod.rs`,
  and the architecture script rejects both the old root files and facade
  exports. Window admission, focus, close and editor behavior are unchanged.
- Shell runtime scheduling state is now shell-module-private. Root rendering,
  startup-restore persistence and terminal interaction/runtime adapters use
  grouped `ShellFeatureState` operations for paint counters, output pressure,
  latency timestamps, cursor/bell presentation, persistence dirty snapshots,
  resize throttling and coalesced scroll/selection repaint queues. The owner
  now performs queue admission, deduplication and sorted drain; the unused
  scroll-position repaint set, which had no writer, was removed. A multiline
  architecture guard rejects external `.shell.runtime` access and mutable
  runtime accessors. GPUI timers and repaint execution remain in their existing
  adapters. Persistence formats and terminal parser/protocol behavior are
  unchanged.
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
  nested privately under the unified `CommandFeatureState` described below.
  Views and runtime adapters use read-only queries and semantic transitions;
  toolbar/menu exclusion, editor and detached-window cleanup, category-delete
  reference cleanup, duplicate-variable synchronization, and import prompt
  admission can no longer be updated piecemeal outside the owner. Persistence,
  native window creation, text inputs, GPUI focus and rendering remain in their
  existing adapters.
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
  the pane owners. The complete follow-up encapsulation pass then made all three
  pane children and their implementation types private. Views now receive
  immutable per-render presentation values; menus, virtual-list offsets,
  Docker details/Compose caches and confirmations, process PID-scoped
  interaction, Stats data and every typed job transition enter through
  `RemoteOpsFeatureState`. `NyaTermApp` retains SSH service launch, terminal
  status mirroring, active-session policy and GPUI notification.
- Security panel state is grouped into `SecurityFeatureState`: the four editors
  and their focus handles in `editors`, revealed passwords/credentials and
  generated OTP codes in `revealed`, and the master password prompt in
  `unlock`. The separate `screen_lock` child now owns the whole-application
  locked flag, password draft, focus, status and idle timer. That child is
  private too: root rendering, global shortcuts, the event pump, settings and
  unlock adapters enter through read-only queries and atomic lifecycle methods
  on `SecurityFeatureState`. Secrets themselves still live in `nyaterm-core`;
  this is view/runtime state only, cleared through the same paths as before.
- Settings interaction state is grouped into `SettingsFeatureState`: custom
  search-engine row/editor menus, keyword-highlight editor expansion/focus,
  appearance menu and discovered font options, and keybinding search/recording
  state live in four focused child structs. Seventeen transient fields became
  one composition-root field. A later convergence pass moved the persisted
  `AppSettingsSummary`, `KeywordHighlightConfig`, staged master-password state,
  and global storage status into the same authoritative feature owner. Their
  save/load formats, encryption behavior, and storage paths are unchanged. The
  interaction children and the shared config/diagnostics/keyword-import/
  snapshot-password prompt child are now private. Search-row index/menu
  reconciliation, keyword expansion/edit lifecycle, appearance-menu exclusion,
  keybinding recording/search transitions and kind-matched prompt completion
  execute on `SettingsFeatureState`; views receive immutable presentation data
  and focus handles. A storage-status follow-up made the implementation type
  and backing child settings-module-private. Cross-domain persistence adapters
  now update its message/readiness through owner methods, rendering receives a
  borrowed immutable view, and store reopen replaces path/message/readiness as
  one transition. The next compatibility-catalog batch made
  `AppSettingsSummary`, `KeywordHighlightConfig` and staged master-password
  state settings-module-private as well. Cross-domain readers now use borrowed
  immutable access, appearance/keyword/browser preferences enter through
  semantic transitions, and the shell persists its complete UI layout through
  one typed owner update instead of piecemeal field writes. Filesystem prompts,
  persistence and GPUI notification stay in their existing adapters. A later
  summary-ownership batch made `AppSettingsSummary` private to
  `SettingsFeatureState` itself and moved roughly fifty general, diagnostics,
  interaction, terminal, remote-panel, recording, transfer and action-link
  edits onto semantic owner methods. Runtime adapters now borrow the summary
  read-only and retain only persistence, cross-feature synchronization, status
  and GPUI notification responsibilities. The following owner-closure batch
  made the keyword catalog, staged master-password state and storage status
  private to the `SettingsFeatureState` implementation too. Settings runtime
  adapters now replace keyword catalogs through the owner and update storage
  message/readiness atomically; no settings child remains directly writable.
  A cross-feature follow-up migrated all persistence outcome branches in
  connections, tunnels, commands, AI, shell, translation and transfers to the
  same atomic status transition and removed the standalone readiness setter.
  Message-only updates remain only for in-progress prompt/job presentation.
- Translation and native-update background state now have authoritative
  `TranslationFeatureState` and `UpdateFeatureState` owners. Eighteen app fields
  became two feature fields; each owner constructs and retains its own job
  channel together with pending/status/result workflow state. Translation
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
  Translation result and native update dialogs are now hosted by the component
  `NyaDialog`; translation keeps an open/result state because it suppresses
  terminal action-link UI while active, while update no longer stores a visual
  `dialog_open` flag.
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
  A later child-boundary closure made the start, prompt and dialog
  implementations session-module-private. Cross-domain views and coordinators
  now use `SessionFeatureState` queries and semantic actions instead of
  traversing `session.start`, `session.prompts` or `session.dialogs`; no child
  reference or mutable accessor is exposed.
  The session runtime-coordination encapsulation pass then made the manager,
  event bridge, restore lifecycle, pending event queue, per-session command
  history, active-session search/menu state and reconnect/disconnect busy map
  private. Runtime adapters now obtain only a shared manager handle or invoke
  bridge/queue methods on `SessionFeatureState`; they cannot mutate the bridge
  or its `VecDeque` directly. Command-history append, reconnect migration and
  global deletion are owner transitions, while starting a busy action closes
  the active-session menu atomically. Event processing, terminal updates,
  worker spawning, rendering and GPUI notification remain on `NyaTermApp` and
  its existing adapters.
  The following session catalog/presentation pass made order, runtime metadata,
  custom names, OSC titles, OSC working directories and tab colors private as
  one ownership cluster. Registration keeps order and metadata synchronized;
  tab moves, disconnect transitions, reconnect presentation migration and
  session removal now execute on `SessionFeatureState`. Removal atomically
  clears all six catalogs together with command history, busy state and the
  matching active menu before the application adapter cleans up terminal,
  workspace and transfer resources. Views and runtime adapters use read-only
  iterators or focused queries, while terminal effects use semantic title/CWD/
  color updates. Session persistence formats, terminal parsing and transport
  protocol behavior are unchanged.
  The active-session ownership pass then made selection private and migrated
  all readers and activation/clearing paths to focused owner methods. The
  active child stores only the authoritative session id; its SSH configuration
  and AI execution profile are derived from runtime metadata, so registration,
  replacement and removal cannot leave stale active-session configuration
  behind. Selecting an unknown id and clearing selection retain the existing
  `None`/`SendOnly` fallback behavior.
  The protocol-resource ownership pass then grouped the per-session ZMODEM and
  trzsz runtimes with SSH multiplex handles in a private
  `SessionProtocolRuntimeState`. Protocol adapters can create/query/drain only
  through focused `SessionFeatureState` methods; catalog removal now stops and
  removes both transfer runtimes in the same owner transition. ZMODEM/trzsz
  states explicitly stop their workers on drop, including whole-owner
  teardown. Multiplex reuse discards closed handles, and unreferenced handles
  are removed by the owner before a named background worker disconnects them;
  GPUI update callbacks no longer call the blocking multiplex `disconnect()`.
  Protocol parsing, frames, file-transfer behavior and transport types remain
  in their existing modules.
  A stronger method-level follow-up then found that the earlier block-level
  audit had missed owner-local session facades. Session display names,
  endpoints, SSH labels, disconnected/tooltip presentation, active command
  history, order movement, next-session selection, list/info/live-count reads
  and pending/failed start queries now execute directly on
  `SessionFeatureState`. Thirty-two owner-local `NyaTermApp` methods were
  removed across the session, transfer and sync-input clusters; registration,
  persistence, background work, GPUI notification and workspace coordination
  remain application adapters.
- Transfer state is grouped into `TransferFeatureState`. Seventy-eight fields
  turned out to be five separate things sharing one panel: the job `queue`, the
  SFTP `browser`, the file operation dialogs (`file_ops`), the built-in remote
  `editor`, and `external_sync` for handing a file to an outside editor, plus
  manual `paths` and `panel` chrome. Their lifetimes are unrelated, which the
  flat `transfer_*` prefix hid completely.
  The transfer-panel encapsulation pass then made its focus, height and resize
  state private and removed the write-only focused-endpoint marker. Runtime and
  views use `TransferFeatureState` queries and transitions; the owner now
  performs the complete height-drag calculation and clamps it to the existing
  60-600px range. Mouse events, terminal status, layout persistence and element
  construction remain in the application adapters.
  The following transfer-path pass made the manual remote/local endpoints,
  duplicate policy and shared native-path-prompt admission private as one
  child. Browser, session protocol and settings adapters now use
  `TransferFeatureState` queries and transitions; remote-path normalization and
  overlapping-prompt rejection plus kind-matched completion stay on the owner,
  while GPUI prompts, transfer jobs and compatibility settings persistence
  remain in their existing adapters.
  The file-operation pass made the complete `file_ops` child private too.
  Rename, move, delete, create-folder, create-file, create-symlink, properties
  and unknown-file dialogs now expose read-only snapshots and semantic
  transitions through `TransferFeatureState`. Deferred inline-rename focus is
  consumed only while its dialog is still active; property load/update/failure
  results are accepted only when both session and remote path still match, and
  closing a session removes only its properties dialog. GPUI focus, text-input
  registry cleanup, terminal/browser status and SFTP job launch remain in the
  application adapters.
  The external-sync pass made its prompt catalog, child-window index, pending
  window admissions and always-upload policy private. Active-session prompt
  selection, upload/ignore resolution and session cleanup are now atomic owner
  transitions, so consuming a prompt or closing its session also reconciles
  associated window tracking. GPUI window creation and activation, terminal
  status updates and SFTP upload work remain in the application adapters.
  The built-in editor pass made its workspace, tab menu, focus and detached
  window lifecycle private behind `TransferFeatureState`. Tab open/activation,
  dirty-close confirmation, discard, save completion/conflict reconciliation,
  session-scoped cleanup and window admission are owner transitions. The
  dedicated `RemoteTextEditor` continues to own selection, undo, IME and text
  commands through narrow active-tab access; GPUI window operations, SFTP jobs,
  rendering and status text remain in their adapters.
  The browser-ownership pass completed the transfer feature boundary by making
  the SFTP `browser` child and its implementation type visible only inside the
  transfers module. Other desktop domains now receive a borrowed, immutable
  presentation view and use named owner transitions for navigation/cache
  rollback, session restore/reset, history/favorites, search/sort, path editing,
  selection/rename, menus, resize state and auto-CWD timing. The borrowed view
  does not copy the listing or create writable mirror state. Typed transfer
  event reduction remains inside the owning transfers module, while GPUI
  rendering, focus, persistence calls and SFTP job launch stay in their existing
  adapters. Owner-level tests pin stable-navigation rollback, cache restoration
  cleanup/history clamping and history branch behavior.
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
  and loading remote file properties. The context menu now uses the
  `nyaterm-ui` wrapper around `gpui-component` and no longer keeps a parallel
  absolute-overlay state. Browser, file-operation, transfer and editor SFTP
  jobs reuse the active SSH multiplex handle; a newly connected or selected
  SSH session starts its initial listing without a manual refresh. The transfer
  queue remains covered by desktop state tests, while its upload/download
  transport path is covered by the remote E2E test.
- AI state is grouped into `AiFeatureState`: provider `settings`, the `chat`
  composer and transcript, session `history`, model `discovery`, the agent
  `loop`, and `panel` chrome. All six implementation children are now visible
  only inside `features::ai`; the rest of desktop uses read-only slices/queries
  and semantic owner transitions. Settings draft snapshot/restore, mutually
  exclusive menus and confirmations, external request preparation, model and
  mention index clamping, detected-error throttling, focus requests and Agent
  capture/reset execute on the owner. `SettingsDraftSnapshot` deliberately
  keeps its own `ai_settings` / `ai_model_draft` / `ai_base_url_draft` /
  `ai_secret_draft` fields with the old names because it is a separate snapshot
  type. GPUI focus/rendering, persistence calls and terminal-context collection
  remain in application adapters.
  A focused panel-ownership pass then made the `panel` child and
  `AiPanelState` implementation type private to `AiFeatureState` itself. The
  AI settings, history, discovery, chat-event and Agent runtime adapters now
  publish complete status text, clear the detected-error banner, and drive
  execution-menu/focus/error transitions through owner methods; no runtime can
  directly mutate `ai.panel.*`. Status wording, persistence outcomes, streaming
  reduction and Agent execution behavior are unchanged.
  The adjacent history/discovery ownership pass then made `history`,
  `discovery`, `AiHistoryState`, and `AiDiscoveryState` private to the parent
  owner. History job admission and stale-completion matching, list/load/delete/
  clear transitions, usage counters and the audit-write lock now have semantic
  owner APIs; transitions that also reset the active chat are atomic on
  `AiFeatureState`. Model-discovery admission/event draining and picker query/
  index/Escape transitions likewise no longer expose the child channel or
  mutable picker fields. Background storage/HTTP work, GPUI notification and
  settings persistence remain in application adapters. Architecture checks
  reject any return to direct `ai.history.*` or `ai.discovery.*` access. AI
  history formats, storage behavior, discovery merging and visible status text
  are unchanged.
  The next cohesive pass made the coupled `chat` and `agent` children private
  to `AiFeatureState` as well. Ask/Agent request admission, cancellation and job
  invalidation, prompt/mention transitions, stream and completion reduction,
  Agent step projection, observation polling, capture matching, background-job
  completion and continuation admission now execute through semantic owner
  transitions. Runtime adapters still collect terminal/session context, launch
  provider and local/SSH work, update GPUI text inputs and notifications, and
  publish persistence status. Typed effects carry only the follow-up work an
  adapter must coordinate; neither child nor its channel/cancellation handles
  are mutable from a runtime. Architecture checks reject direct
  `ai.chat.*`/`ai.agent.*` access and visibility regressions. Request payloads,
  stream/status wording, command execution policy and persisted AI history are
  unchanged.
  The settings ownership pass then made `settings` and `AiSettingsState`
  private to `AiFeatureState` itself. Provider/profile toggles, numeric policy
  bounds, pending-settings construction, masked-secret draft merging, model and
  credential catalog edits, custom-action edits, and discovery-result merging
  now execute through semantic owner operations. Adapters retain GPUI focus and
  text-input routing, persistence execution, model-discovery/provider calls and
  notification. The owner preserves the previous distinction between repairing
  a missing default model during model-list edits and leaving an explicitly
  absent default untouched during credential disable/removal. Architecture
  checks reject direct `ai.settings.*` access, visibility regressions and mutable
  child accessors. AI settings serialization, secret masking and persistence
  compatibility are unchanged.
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
  A later child-encapsulation batch made `search`, `input`, `paste`,
  `selection`, `layout`, `menus` and `paint` visible only inside
  `features::terminal`. Root, shell, session and panel adapters now use a
  borrowed paste-review view, one overlay-visibility projection, read-only
  focus/cache/geometry queries and semantic owner transitions. Session
  activation clears selection drag and action-link menu/tooltip state as one
  transition, while reconnect migrates painted surface bounds through the
  owner. A following split/tab batch made `windows` terminal-module-private and
  moved tree reconciliation, tab activation/reorder/docking, smart-split,
  restore/serialization, split ratios, reconnect id replacement and file-drop
  hover transitions onto `TerminalFeatureState`. Shell/session/page adapters
  retain GPUI notification, navigation, persistence scheduling and redb opens;
  they receive only immutable tree/drop projections and typed mutation
  results. A following inline-assistance batch made `assist`
  terminal-module-private and moved the command-suggestion and credential-
  autofill runtime modules under `features/terminal`. Cross-domain settings,
  session, history and root callers now use semantic reset/take methods or
  boolean visibility queries on `TerminalFeatureState`; no mutable-reference
  child accessor was introduced. The high-coupling `view` child remains the
  next terminal ownership boundary.
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
  those), and `progress` (in-flight send, cancellation, counters). A later
  encapsulation pass made all three children and their fields private. Views
  consume one immutable per-render presentation value; input normalization,
  menu exclusion, data/mode compatibility, progress accounting and cancel
  ownership execute on `SendCommandFeatureState`. Session targeting, terminal
  writes, GPUI focus, text-input synchronization and status text remain in the
  application adapters.
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
  mode, heights and drag state. All seven children, including diagnostics, are
  now visible only inside `features::shell`; other desktop modules use owner
  queries and semantic transitions. Settings-window admission/cleanup and
  embedded-page panel restoration, root-menu exclusion, connection-failure
  cleanup, mobile-panel closure, new-session submenu paths and pane-owner
  rebuilding now update related fields atomically. Forty-seven top-level app
  fields became these focused children, so `NyaTermApp` dropped from 102 fields
  to 55 without creating a writable mirror. Render helpers remain on views,
  while `NyaTermApp` retains settings persistence, GPUI notification, terminal
  coordination and event routing.
  A later status-ownership batch made the application-wide transient status
  private as well. More than nine hundred desktop reads and writes now use
  `ShellFeatureState::status` and `set_status`, so adapters can no longer replace
  the backing string directly; status contents, update timing and GPUI
  notification remain unchanged.
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
- Saved connections, groups and runtime-discovered serial ports now sit in the
  private catalog child of `ConnectionFeatureState`, alongside that feature's
  list, editor, confirmation, import and Network-page state. The former root
  `connection_catalog` field and its facade export are gone. Catalog replacement
  prunes stale list references in the same owner transition, and derived list,
  group-tree, group-expansion and SSH tunnel-editor queries no longer require
  callers to pass the owner's own collections back into it. `ConnectionStore`
  remains the persistence execution boundary and its compatibility contracts
  are unchanged.
- Command state now has one top-level `CommandFeatureState` owner. Its
  `catalog`, `quick`, `history` and `runtime` children replace the separate
  quick-command collections, UI state, command-history collection and
  persistence runtime fields, taking `NyaTermApp` from 32 fields to 28. The
  owner constructs the command persistence worker and provides grouped
  load/clear transitions, while catalog writes replace commands and categories
  together. A later encapsulation pass made `catalog`, `history` and `runtime`
  private: views receive slices or immutable `Arc` snapshots, worker admission
  and polling enter through the owner, and failed use-count persistence rolls
  back the optimistic catalog increment on the same state. Quick-command
  serialization, command-history storage, portable-key handling and worker
  behavior are unchanged.
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
  `ConnectionFeatureState` and `state/list_logic.rs`. Those projections read the
  feature's private catalog directly; list filtering, sorting, expanded-group
  traversal and selection projection stay out of the app coordinator and
  callers cannot supply a divergent collection.
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
- The former search-engine editor field model and its paired write-only row
  index were removed when the settings interaction owner was encapsulated.
  Expanded-row identity and the actual registry-backed text fields are the
  live editing state; no compatibility reader or persistence field used the
  deleted values.
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
- `nyaterm-terminal-gpui` no longer uses its crate root as a shared import
  bucket. Production modules and nested tests name their GPUI, core, terminal,
  type and sibling-helper dependencies explicitly, and the architecture guard
  keeps `use super::*` at zero. The root no longer re-exports GPUI/core/terminal
  dependency surfaces or imports implementation modules by glob; its terminal
  painting, input and keyword APIs remain named exports. Desktop now imports
  pure terminal cell helpers directly from `nyaterm-terminal`.
- The unused terminal-GPUI `apply_cursor_style` approximation and its isolated
  test are removed. Runtime cursor painting already uses the live terminal
  element path, and the deleted helper had no production caller.
- AI credential rows no longer retain the obsolete `credential_edit` marker or
  its three-field enum. Ordinary input entities own editing/focus, while
  `credential_secret_drafts` and the existing commit/persist path remain the
  authoritative save flow. Five related AI UI-local bindings and one stale
  master-password placeholder binding were also removed; credential formats,
  secret masking and persistence are unchanged.
- Construction-time focus handles and focus enums that ordinary input entities
  now own are removed from terminal, transfer, network, AI and security state.
  The remaining SSH-key focus enum still drives key/certificate picker routing.
  Process nice-value editing now uses its localized placeholder, and activity
  drag payloads carry only the tab identity consumed by the drop path.
- Workspace split tests now exercise the production `dock_tab` API directly;
  the obsolete `SplitEdge` adapter is gone. Terminal resize tests call the
  authoritative size/insets geometry function, credential prompt-line parsing
  is test-only, cursor visibility is asserted through real scroll behavior,
  and the unused convenience selection constructor is removed.
- Local settings now expose password-encrypted `.nya` export and import actions
  backed by the existing portable snapshot implementation. Import still
  rejects an unsaved settings draft or active/pending session, legacy
  unencrypted `.nya` import remains available through the connection import
  flow, and prompt admission no longer overwrites another active password
  prompt. Encryption prefixes, snapshot serialization and fallback decryption
  behavior are unchanged.
- The terminal frame worker no longer carries the producerless
  `RequestBufferText` command/event path. AI context continues to read the
  existing per-session UI buffer, and event-pressure tests use the live Search
  event as their deferred-work case. No terminal parser, snapshot or wire
  protocol behavior changed.
- A workspace clippy pass removed 127 structural diagnostics without adding
  lint allowances: adjacent guards now use Rust 2024 let-chains and GPUI update
  calls no longer bind unit results. Desktop lib warnings fell from 305 to 199;
  core fell from 39 to 26, terminal from 12 to 10, and transport from 16 to 10.
  The changes preserve condition evaluation order and error branches, including
  credential fallback, legacy settings parsing, known-host rendering, graphics
  storage, X11 cookie selection and transfer progress handling.
- A second bounded clippy pass removed another 77 desktop-lib diagnostics
  without adding lint allowances. Orphaned doc-comment spacing, needless tail
  returns and borrows, same-type conversions, copy clones, simple iterator
  predicates and integer clamp chains are now clean across all workspace
  targets. Desktop lib warnings fell from 199 to 122 and its test target from
  216 to 136 (122 duplicates); core fell from 26 to 22 and terminal from 10 to
  8, while transport remains at 10 and terminal-GPUI at 2. Two floating-point
  UI-width `manual_clamp` suggestions remain because replacing ordered
  `min`/`max` with `clamp` would change NaN behavior.
- A third expression and fixture pass reduced desktop lib warnings from 122 to
  71 and its test target from 136 to 72 (71 duplicates). Core fell from 22 to
  zero, terminal from 8 to 2, transport from 10 to 4, and bundled OTP from 4
  to zero; terminal-GPUI remains at 2. Named OSC 52 clipboard callback types
  replaced repeated dynamic-trait signatures, exact default enums use derived
  defaults, test fixtures initialize fields atomically, and duplicated UI/
  state branches were collapsed only after checking their evaluation order.
  The remaining desktop-lib diagnostics have no rustfix suggestions: they are
  parameter-object, large-enum, module-name or floating-point semantics design
  decisions rather than expression cleanup.
- A fourth ownership and parameter-object pass cleared the remaining workspace
  diagnostics without lint allowances. Desktop fell from 71 warnings to zero;
  terminal, transport and terminal-GPUI also reached zero, so
  `cargo clippy --workspace --all-targets` is project-warning-free. Typed render
  presentations now carry Docker, quick-command, tab-action, process, transfer,
  settings and sync-history inputs; terminal frame, scroll, mouse-report and
  suggestion geometry use dedicated requests. Connection cleanup now composes
  focused owner transitions instead of borrowed field bags, and
  `TerminalSurfaceFrameSnapshot` is the single frame-application contract.
  Persistence formats, terminal parsing and wire protocols are unchanged.
- The terminal surface entity's 2,283-line test module moved out of the
  production implementation file into the normal
  `terminal_surface_entity/tests.rs` child module. The 2,475-line entity keeps
  its private implementation boundary, all 54 focused tests keep direct access
  through Rust's child-module privacy, and no `#[path]`, shared prelude or
  runtime behavior was introduced. This removes the file from the 4,000-line
  debt list without touching the terminal paint hot path.
- The other three 4,000-line files now use the same normal test-module
  boundary. `models/terminal.rs` is 2,647 lines with its 86 frame/selection
  tests still passing; `nyaterm-transport/src/lib.rs` is 3,571 lines with all
  146 transport tests passing; and the compatibility-sensitive storage facade
  is 1,470 lines with all 183 core tests passing. The extracted tests use
  explicit imports rather than a shared wildcard prelude. No storage schema,
  serialized field, encryption prefix, fallback reader, PTY/SSH/Telnet/Serial
  behavior or terminal frame algorithm changed.
- The next embedded-test pass moved 118 desktop tests out of three production
  modules into normal child modules: 41 terminal view/input tests, 60
  connection owner-transition tests and 17 transfer owner-transition tests.
  `view_io.rs`, connection `state.rs` and transfer `state.rs` are now 2,828,
  2,106 and 2,715 lines respectively. The extracted modules use explicit
  imports, keep Rust child-module access to private helpers, and preserve all
  736 desktop tests without changing runtime ownership or GPUI behavior.
- `SessionEventQueue` moved from the transport crate root into a focused
  243-line private module. Its bounded output, drop accounting, blocking wake
  and budgeted drain behavior stay behind the same crate-root private facade,
  so SessionManager, X11 forwarding and the existing queue tests require no
  public API change. `nyaterm-transport/src/lib.rs` is now 3,339 lines.
- A final embedded-test pass moved 153 tests out of AI state, session state,
  the terminal crate root and the trzsz protocol module. The terminal and
  trzsz tests also replaced their inherited `use super::*` imports with exact
  symbol lists. All 736 desktop, 137 terminal and 147 transport tests still
  pass; terminal parsing, snapshots, SSH/X11 events and trzsz wire behavior are
  unchanged.
- The graphics and cloud-sync embedded test modules now use normal child-module
  boundaries too. Thirty-five graphics protocol tests moved to
  `graphics/tests.rs`, reducing the production module from 2,888 to 1,597
  lines. Nineteen cloud-sync compatibility and provider tests moved to
  `cloud_sync/tests.rs`, reducing that production module from 2,807 to 1,937
  lines. Both test modules use explicit imports; terminal protocol handling,
  remote provider behavior, encrypted snapshots, conflict handling, history
  compatibility and secret masking are unchanged.
- The terminal-GPUI element's 41 layout, shaping, row-cache and decoration
  tests moved from the middle of `element.rs` into the normal
  `element/layout_cache_tests.rs` child module. The production file is down
  from 2,820 to 1,648 lines, while the tests retain child-module access to its
  private layout and paint helpers with the same explicit imports. GPUI element
  layout, clipping, shaping, decoration and painting behavior are unchanged.
- The terminal runtime buffer's 10 focused tests moved from the production
  file tail into the normal `buffer/tests.rs` child module. `buffer.rs` is down
  from 2,830 to 2,593 lines, while frame drain budgets, search apply routing,
  surface/chrome notifications, OSC52 reply limits, local log escaping and
  terminal-window tab visibility retain direct focused coverage. Runtime frame
  application and terminal rendering behavior are unchanged.
- Terminal runtime input coordination moved out of `view_io.rs` into the
  focused 1,240-line `view_io/input.rs` module. It owns logical/key/raw input,
  per-session key protocol encoding, mouse and alternate-scroll reports, focus
  reports, wire-write recording, outgoing encoding synchronization, status
  updates and slow-input diagnostics. `view_io.rs` is down from 2,828 to 1,606
  lines and now concentrates on snapshots, retained scroll windows and surface
  paint synchronization. The existing 41 tests retain direct coverage through
  narrow test-only helper visibility; input, history, recording, IME, mouse,
  scroll and rendering behavior are unchanged.
- SSH command execution and remote process management moved together into a
  499-line `remote_process.rs` module. It owns the command output model, SSH
  exec channel collection, timeout runtime, local command adapter, process
  listing script and parser, and signal policy. The crate root re-exports the
  same public models, constants, service and helpers, while stats, Docker and
  SFTP keep using the shared crate-private execution facade. This reduces
  `nyaterm-transport/src/lib.rs` to 2,863 lines without changing SSH command or
  process behavior, and leaves no Rust source file above 3,000 lines.
- Local, Telnet, Serial and SSH session configuration contracts moved together
  into the private 340-line `session_config.rs` module. The crate root
  re-exports the same configuration, host-key and credential callback types,
  preserving every public path while reducing `lib.rs` from 2,863 to 2,547
  lines. `SshKeyAuthConfig` now has its own redacted `Debug` implementation and
  a direct regression test, so private key, certificate and passphrase values
  remain hidden even when the nested config is formatted independently.
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
- The method-ownership convergence pass now uses method bodies and call sites,
  not the number of `impl NyaTermApp` blocks, as its audit boundary. That
  stronger pass corrected an earlier over-broad completion claim and removed
  thirty-two owner-local session, transfer and sync-input facades. The 238
  remaining blocks still contain render/view construction, GPUI focus/window/
  notification work, persistence and service execution, or cross-feature
  coordination, so the block count is an inventory rather than a reduction
  target. The extra block since that audit is the `view_io/input.rs` production
  module split; it moved existing methods without adding a facade. The former
  feature-root constant bucket was also removed: terminal
  banner, AI agent timing, session policy, sync-group palette and tab
  presentation constants now live with their consumers. The architecture
  script enforces the exact twenty-field composition-root owner set and rejects
  reintroduction of the removed owner-local facades or root domain constants.

## Current Ownership

Feature-owner encapsulation is the established architecture boundary. New work
must preserve it; future ownership migrations should be driven by a concrete
cohesive cluster rather than an `impl NyaTermApp` count.

Current ownership map:

| Area | Current owner | Kind | Notes |
| --- | --- | --- | --- |
| Saved connections | Private catalog child in `NyaTermApp.connection_state` | Persisted domain state | `ConnectionFeatureState` is authoritative and exposes read-only slices plus semantic replacement/update operations; `ConnectionStore` remains the persistence boundary. |
| Connection groups | Private catalog child in `NyaTermApp.connection_state` | Persisted domain state | Catalog replacement and stale list-reference cleanup occur in one `ConnectionFeatureState` transition. |
| SSH keys | Private catalog in `NyaTermApp.security` | Secret-adjacent persisted catalog | The security feature is authoritative; consumers use a read-only slice and the catalog has no `Debug` implementation. |
| OTP entries | Private catalog in `NyaTermApp.security` | Secret-adjacent persisted catalog | Consumers use a read-only slice; do not log secrets or widen Debug exposure. |
| Saved passwords/credentials | Private catalog in `NyaTermApp.security` | Secret-bearing persisted catalogs | Credential autofill and security settings use read-only owner APIs; grouped store loads and clears replace all four catalogs together. |
| Secret unlock/reveal interaction | Private state in `NyaTermApp.security` | Secret-bearing transient UI state | Prompt admission, pending actions, success/failure, revealed values and lock cleanup enter through `SecurityFeatureState`; secret-bearing children have no `Debug` implementation. |
| Whole-application screen lock | Private state in `NyaTermApp.security` | Transient lock/input/idle state | Locked status, password draft, focus and idle timing are queried or changed only through `SecurityFeatureState`; storage verification, input widgets and GPUI focus remain in runtime/view adapters. |
| Quick-command catalog | Private catalog in `NyaTermApp.commands` | Persisted domain state | Views receive read-only slices/snapshots; successful storage operations replace commands and categories together through `CommandFeatureState`. |
| Quick-command UI | Private child in `NyaTermApp.commands` | Transient UI/window state | List filters and menus, editor/dialog drafts, import admission and AI prompt state are queried or changed only through `CommandFeatureState`; GPUI focus, windows, rendering and persistence stay in adapters. |
| Command history and persistence worker | Private state in `NyaTermApp.commands` | Persisted catalog plus background runtime | History snapshots, queue admission, event polling and idle checks enter through `CommandFeatureState`; failed optimistic use-count updates roll back on the owner. |
| Send-command composer/options/progress | Private children in `NyaTermApp.send_command` | Transient editor and send lifecycle | Views receive immutable presentation data; control edits, mutually-exclusive menus, data/mode defaults, progress counters and cancellation enter through `SendCommandFeatureState`. Session selection, terminal writes, GPUI/text-input routing and status remain in adapters. |
| Settings interaction and prompts | Private children in `NyaTermApp.settings` | Transient settings UI and prompt lifecycle | Search-engine rows, keyword-highlight editing, appearance menus, keybinding recording/search and config/diagnostics/import/password prompt admission enter through `SettingsFeatureState`; views use immutable presentation values and read-only focus/font access. Persistence, native filesystem prompts, text inputs and GPUI notification remain in adapters. |
| Application settings and keyword catalogs | State-private children in `NyaTermApp.settings` | Compatibility-sensitive persisted configuration plus staged master-password input | `AppSettingsSummary` and keyword-catalog mutations enter only through `SettingsFeatureState`; consumers borrow them immutably. Staged master-password changes use owner transitions and a non-`Debug` borrowed view. Serialization, encryption and persistence remain in `nyaterm-core` and existing adapters. |
| Global storage status | State-private child in `NyaTermApp.settings` | Runtime persistence health/presentation state | Persistence adapters update message/readiness atomically through `SettingsFeatureState`; rendering receives a borrowed immutable view, while store reopen replaces path/message/readiness together. Database work and compatibility handling remain in existing adapters and `nyaterm-core`. |
| AI settings/chat/history/discovery/agent/panel | Private children in `NyaTermApp.ai` | Persisted settings plus transient UI and background lifecycle | Desktop consumers use read-only slices/queries and semantic transitions; settings/profile/model/credential/action mutations, draft and secret merging, menu exclusion, confirmations, request/focus preparation, panel status/error ownership, picker clamping and Agent capture/reset enter through `AiFeatureState`. Persistence, provider/background execution, terminal-context collection, GPUI focus/rendering and notification remain in adapters. |
| Shell viewport/navigation/panels/chrome/workspace/runtime | Private state and children in `NyaTermApp.shell` | Transient GPUI composition, interaction, status and event-pump scheduling state | Other desktop modules use semantic shell operations for the private application-wide status line and GPUI event-pump/repaint/persistence scheduling. Menu exclusion, settings-window lifecycle, mobile panels, failure chrome, submenu paths, pane ownership, persistence dirty snapshots and coalesced terminal repaint admission remain `ShellFeatureState` operations. Persistence execution, rendering, GPUI windows/notification and terminal coordination remain in adapters. |
| Terminal interaction/presentation children | Terminal-module-private children in `NyaTermApp.terminal` | Per-session views/surfaces/frame queues plus search, focus/IME, paste review, inline command/credential assistance, selection/mouse, paint geometry, menus, caches and split/tab window ownership | Cross-domain adapters use immutable projections, typed frame-queue metrics and semantic lifecycle transitions. Session registration/removal, activation, reconnect output, discontinuity handling, render-cache invalidation and performance recovery now execute behind `TerminalFeatureState`; no desktop module outside `features/terminal` directly accesses `terminal.view`. GPUI notification, navigation, persistence execution, terminal parsing, snapshots, paint algorithms and protocol handling are unchanged. |
| Remote Docker/process/stats panes | Private children in `NyaTermApp.remote_ops` | Transient UI state plus typed background-event lifecycle | Views use immutable presentation values; menu exclusion, list-offset clamping, Docker details/Compose/confirmation cleanup, process PID-scoped cleanup, Stats expansion/data and job identity/failure timing enter through `RemoteOpsFeatureState`. SSH service launch, active-session policy, terminal status mirroring and GPUI notification remain in adapters. |
| Live session manager/event bridge | Private services in `NyaTermApp.session` | Runtime services | Callers receive a shared manager reference/handle and use bridge routing, drain and metrics methods; neither service field is writable outside `SessionFeatureState`. |
| Session restore/event queue | Private state in `NyaTermApp.session` | Transient runtime coordination | Restore completion is idempotent and pending transport events are counted, extended and popped only through owner methods; event interpretation stays in the event-pump adapter. |
| Session start/prompts/dialogs | Session-module-private children in `NyaTermApp.session` | Background admission plus transient prompt/dialog state | Cross-domain renderers and coordinators use `SessionFeatureState` queries and semantic actions, including pending/failed names, active failure and duplicate saved-start detection. Worker channels, prompt brokers and dialog drafts remain inside the session module; GPUI rendering and notification stay in adapters. |
| Session command history/search/menu/busy state | Private state in `NyaTermApp.session` | Transient per-session interaction state | Active-history snapshots/indexed reads, append/migration/deletion and active-menu/busy transitions are owner operations; beginning reconnect/disconnect closes the menu in the same transition. |
| Session catalog and presentation | Private state in `NyaTermApp.session` | Runtime catalog plus transient presentation | Order/metadata registration, whole-tree tab movement, disconnect marking, reconnect migration and removal are owner transitions. Session lists/info/live count, display names, endpoints, SSH labels, disconnected state and tab tooltips are owner queries; custom names, OSC titles/CWDs, tab colors, whole-workspace-tab lock state and transient tab drag targets use read-only queries and semantic updates. Persisted `ui.open_tabs[].locked` is restored into this owner and follows reconnect ID migration. |
| Active session selection and derived config | Private state in `NyaTermApp.session` | Transient selection over the runtime catalog | The active child stores only the session id; SSH config and AI execution profile are queried from private runtime metadata, and select/clear/remove transitions preserve the `None`/`SendOnly` fallback. |
| Session protocol resources | Private child in `NyaTermApp.session` | Per-session runtime resources | ZMODEM/trzsz maps and SSH multiplex handles are private; session removal stops protocol workers atomically, state drops stop remaining workers, and multiplex disconnect runs off the GPUI update path after owner reference checks. |
| Transfer panel interaction | Private child in `NyaTermApp.transfer` | Transient focus and resize state | Focus routing, height and resize state enter through `TransferFeatureState`; the write-only focused-endpoint marker was removed, and height-drag calculation and clamping are pure owner transitions while rendering and persistence stay in adapters. |
| Transfer endpoints and path prompts | Private child in `NyaTermApp.transfer` | Transient endpoints, compatibility-derived policy and prompt admission | Remote/local paths, duplicate policy and the shared native prompt slot enter through `TransferFeatureState`; normalization, single-prompt admission and kind-matched completion are owner transitions while GPUI prompts, jobs and settings persistence stay in adapters. |
| Transfer job queue | Private child in `NyaTermApp.transfer` | Typed background-event queue plus transient interaction state | Monotonic job-id allocation is called directly on `TransferFeatureState`; admission/removal, result-channel access, selection/menu/delete lifecycles, session-switch reset and session-scoped batch actions also enter through the owner. Cleanup prunes stale interaction references. Renderers receive a read-only slice, protocol adapters can update individual jobs without receiving the backing collection, and the event reducer temporarily extracts only its matched job while it coordinates browser/editor side effects. |
| Transfer SFTP browser | Transfers-module-private child in `NyaTermApp.transfer` | Listing, navigation/cache, history/favorites, selection, menus and viewport interaction | Other desktop domains receive a borrowed immutable presentation view; navigation rollback/session restore, history/favorites, search/sort, path editing, selection/rename, menu exclusion, resize and auto-CWD timing use named `TransferFeatureState` transitions. Typed transfer event reduction stays inside the owning transfers module; rendering, focus, persistence and SFTP launch stay in adapters. |
| Transfer file-operation dialogs | Private child in `NyaTermApp.transfer` | Transient dialog drafts, focus and property-operation lifecycle | Rename/move/delete/create/properties/unknown-file state enters through `TransferFeatureState`; deferred rename focus is consumed atomically, creation options update semantically, and property results require matching session/path ownership. Renderers receive read-only dialog state while GPUI focus, text inputs, status and SFTP launch remain in adapters. |
| Transfer external-editor sync | Private child in `NyaTermApp.transfer` | Transient prompts, child-window tracking and always-upload policy | Prompt admission/filtering/resolution, window admission/tracking and session cleanup enter through `TransferFeatureState`; consuming or dismissing a prompt reconciles its tracked window state. GPUI window operations, status updates and SFTP upload launch remain in adapters. |
| Built-in transfer editor | Private child in `NyaTermApp.transfer` | Editor workspace, tab/confirmation state and detached-window lifecycle | Tab admission/activation/close/discard, active-tab access, save result reconciliation, session cleanup, menu state and window admission enter directly through `TransferFeatureState`. `RemoteTextEditor` keeps its dedicated selection/undo/IME path through that narrow active-tab access; GPUI windows, SFTP work and rendering remain in adapters. |
| Serial ports | Private catalog child in `NyaTermApp.connection_state` | Runtime/discovered state | Replaced through `ConnectionFeatureState` after session-manager discovery and never persisted. |
| Tunnel/proxy configs | Private catalog in `NyaTermApp.tunnel_state` | Persisted network config | Views use read-only slices; pure move/upsert/delete candidates and successful commits stay on `TunnelFeatureState`. The Network page UI remains separate transient state. |
| Queued saved-connection starts | Session-module-private child in `NyaTermApp.session` | Transient session-start state | Admission, duplicate detection, draining and runtime cadence reads cross domain boundaries through `SessionFeatureState`; the queue remains owned by the nested start state. |
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

The terminal-adjacent desktop changes route quick switch IME state through the
authoritative overlay Entity and session event/manager access through the
private `SessionFeatureState` boundary. These are desktop state ownership/UI
adapter changes, not terminal parser, terminal protocol, SSH/SFTP protocol,
transfer protocol, or persistence-format changes.
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
- `NyaTermApp` is the composition root for two application services and eighteen
  focused feature owners. The architecture script checks that exact twenty-field
  set. New state should move into a focused FeatureState or a deliberately
  authoritative Entity, not into a new unrelated top-level field.
- The connections state, runtime, list interaction and child-window adapters
  now live under the normal `features/connections` tree and are governed.
  Connection views intentionally remain in the shared `pages` presentation
  tree; they enter state through `ConnectionFeatureState` rather than owning a
  second connection domain. Saved connections, groups and discovered serial
  ports are now a private child of that same owner, so no root-level catalog or
  caller-supplied catalog projection remains.
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
historical baselines for the remaining governed rules, while desktop and
terminal-GPUI `#[path]`/`use super::*` debt, the desktop shared feature prelude,
and the terminal-GPUI shared root import bucket are enforced at zero. The
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

`nyaterm-desktop` went from 104 dead-code warnings to 0. The first reduction
removed the superseded migration render layer: the old `left_*_panel` /
`right_*_panel` tree that `panel_body`'s `*_view` dispatch replaced, the widget
helpers only it called, and the handlers only those widgets invoked. The final
pass audited each remaining diagnostic against production call sites: complete
encrypted-backup behavior was connected to settings, one process label was
wired, and producerless adapters, transient focus state and test-only wrappers
were removed or scoped to tests.

There is no warning-backed unwired-capability list now. Future capabilities
must land with a production entry point, or remain outside production state
until that product path exists. Treat any new desktop dead-code warning as a
regression to classify, not as an accepted baseline.

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

Items 1 through 7 are done for the current convergence boundary. Item 8 remains
a deliberately deferred architectural decision.

1. Done. `#[path = "..."]` no longer appears in `nyaterm-desktop` or
   `nyaterm-terminal-gpui`, and a crate-wide guard keeps it that way.
2. Done. Desktop and terminal-GPUI production modules and nested tests contain
   no `use super::*`; crate-wide guards prevent the chain from returning. The
   compiler-confirmed final pass also removed `features/prelude.rs`, and the
   terminal-GPUI root no longer re-exports its dependency crates, so modules
   cannot regain the same implicit dependency surface through a shared import
   bucket.
3. Done for the current convergence boundary. `NyaTermApp` is down from 585
   fields to 20, across eighteen
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
   secret-bearing `Debug` exposure. The following security-editor lifecycle
   batch made the four mutually-exclusive editor drafts, QR-import admission
   and delete-confirmation state private too; input routing and editor
   open/finish/close transitions now enter through `SecurityFeatureState`,
   while GPUI focus, rendering, file decoding and persistence remain in their
   existing adapters. The secret-access lifecycle batch then made auth-tab,
   status, unlock prompt/pending-action state and revealed password,
   credential and OTP maps private. Unlock success/failure and secret cleanup
   are now owner transitions, while password verification, clipboard access
   and OTP generation remain in their existing adapters. The following screen
   lock encapsulation batch made the whole-application lock child private as
   well, replacing direct reads and writes across root rendering, shortcuts,
   the event pump, settings and the lock overlay with owner queries and atomic
   lifecycle transitions. Password verification, text-input ownership and GPUI
   focus remain in their existing adapters. The following command-data
   encapsulation batch made the quick-command catalog, command-history snapshot
   and persistence worker private, routed rendering and suggestion searches
   through read-only views, and coupled optimistic use-count updates with
   persistence-failure rollback on `CommandFeatureState`. Storage access,
   portable-key selection and GPUI notification remain in their existing
   adapters. The following quick-command UI encapsulation batch then made the
   nested list, editor, dialog, import and AI children private. Menu exclusion,
   editor/window closure, category-delete cleanup, variable synchronization and
   import admission now execute as owner transitions, while persistence, GPUI
   focus, native window creation, text inputs and rendering stay in adapters.
   The following session runtime-coordination batch made eight more
   `SessionFeatureState` fields private and routed manager/bridge/event-queue,
   history, search, menu and busy-state access through focused owner APIs.
   Reconnect/disconnect admission now couples busy registration with menu
   closure, and reconnect history migration cannot leave the old session id
   behind. The next session catalog/presentation batch then made six more
   fields private and moved registration/order synchronization, disconnect
   marking, reconnect presentation migration and atomic catalog removal onto
   `SessionFeatureState`. The active-session selection batch then made the
   remaining selection field private, routed roughly seventy-five desktop
   modules through owner queries/transitions, and removed the duplicate active
   SSH/AI caches: both values now derive from the private metadata catalog. The
   following protocol-resource batch made ZMODEM, trzsz and SSH multiplex maps
   a private child, coupled session removal with worker cleanup, added drop-time
   worker termination, and moved blocking multiplex disconnects onto named
   background workers after owner-controlled reference checks.
   The later session child-boundary batch made start, prompt and dialog
   implementation types session-module-private and migrated all cross-domain
   views and coordinators to `SessionFeatureState` methods. No child reference
   or mutable accessor was added.
   The following transfer-browser batch completed the transfer feature's child
   encapsulation: the browser implementation is transfers-module-private,
   cross-domain consumers receive a borrowed immutable view, and navigation,
   session cache, history/favorites, search/sort, path editing, selection/rename,
   menu and resize transitions moved onto `TransferFeatureState`.
   The settings storage-status batch then made the backing child and
   implementation type settings-module-private, moved cross-domain status
   writes behind owner transitions, and gave rendering a borrowed immutable
   view without changing storage execution or compatibility formats.
   The following settings compatibility-catalog batch completed child
   encapsulation for `SettingsFeatureState`: summary, keyword-highlight config
   and staged master-password state are now settings-module-private, all
   cross-domain reads are immutable, keyword/appearance/browser mutations use
   owner transitions, and UI layout persistence applies one typed atomic
   update. No mutable-reference accessor was introduced. The later settings-
   summary ownership batch then made the summary state-private as well and
   moved roughly fifty general, interaction, terminal, remote-panel, recording,
   transfer and action-link transitions off `NyaTermApp`; persistence execution,
   cross-feature synchronization and GPUI notification remain in adapters. The
   subsequent owner-closure batch made the keyword catalog, staged master-
   password state and storage status state-private too, migrated settings
   runtime writes onto semantic owner methods and tightened the architecture
   guard across the complete feature tree. No mutable child accessor was added.
   The following cross-feature batch then removed the standalone storage-ready
   setter and migrated every persisted success/failure outcome to one atomic
   message/readiness transition; message-only updates remain valid for progress
   states that deliberately preserve the last readiness result.
   The next terminal interaction batch made seven presentation children
   terminal-module-private: search, input, paste review, selection, layout,
   menus and paint caches. External views and coordinators now use immutable
   projections or semantic transitions; activation interaction cleanup and
   reconnect surface-bound migration execute atomically on
   `TerminalFeatureState`. The following terminal window batch made the
   split/tab child terminal-module-private and moved reconciliation, docking,
   smart-split, restore/serialization, ratio changes, reconnect id replacement
   and file-drop hover transitions onto the owner. The following inline-
   assistance batch made `assist` terminal-module-private, relocated its two
   runtime adapters into the terminal tree and moved cross-domain lifecycle
   operations behind `TerminalFeatureState`. The following terminal-view batch
   moved the application-wide status line and event-pump/repaint/persistence
   scheduling bookkeeping to the shell owner, then made `view` and its
   implementation type terminal-module-private. Cross-domain consumers now use
   typed queue metrics, immutable queries and session/render lifecycle
   transitions; direct external `terminal.view` access is zero.
   The following module-ownership batch moved all six domain-specific window
   and editor adapters out of the feature root into connections, commands,
   settings and transfers, and removed their broad root facade exports.
   The following shell-runtime encapsulation batch made its runtime child and
   implementation type shell-module-private, migrated all external direct
   access to grouped owner operations, and moved coalesced scroll/selection
   queue admission, deduplication and drain onto `ShellFeatureState`. The shell
   event pump keeps direct access inside its ownership boundary; GPUI timers,
   persistence execution and terminal repaint work remain in their adapters.
   The later shell-status batch made the application-wide status string private
   and migrated more than nine hundred reads and writes across the desktop tree
   to owner methods without changing the messages or their update cadence.
   The connection-owner unification batch then removed the last separate root
   catalog, nested saved connections, groups and serial discovery under
   `ConnectionFeatureState`, and coupled catalog replacement with list-reference
   cleanup while keeping `ConnectionStore` as the persistence executor.
   What remains at the
   composition root is stores, runtime and focused feature owners.
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
   into the page layer; the transfer-panel pass likewise made focus and
   height-resize state private, removed a write-only focused-endpoint marker,
   and moved the complete drag lifecycle onto `TransferFeatureState`; the next
   transfer-path pass made endpoints, duplicate policy and native prompt
   admission private, and removed the app-level remote-path normalization
   helper; the transfer-queue pass then made the typed result channel, backing
   job collection, selection, menus, delete confirmation and focus private,
   moving monotonic id allocation, admission, removal, session-scoped batch
   actions and interaction cleanup onto the owner while keeping protocol
   interpretation in its existing desktop adapters; the file-operation pass
   then made all eight dialog slots and focus handles private, moving deferred
   rename focus, creation-option edits, property result matching and
   session-scoped property cleanup onto the same owner; the external-sync pass
   then made prompt, child-window, pending-open and always-upload state private,
   coupling prompt resolution and session cleanup with window-tracking cleanup
   while leaving GPUI windows and SFTP uploads in their adapters; the built-in
   editor pass then made workspace/tab/menu/window state private and moved
   dirty-close, save-result, session-cleanup and window-admission transitions to
   the transfer owner while preserving the dedicated editor surface. The
   send-command encapsulation pass similarly made its composer, options and
   in-flight progress children private, replacing writable view/runtime access
   with owner transitions and a single per-render presentation value while
   leaving session routing and terminal writes in the app adapter. The settings
   interaction pass then made its five UI/prompt children private, coupling
   search-row deletion with index correction, mutually exclusive menus,
   keyword edit cleanup, keybinding recording state and kind-matched prompt
   completion on `SettingsFeatureState` while leaving persistence and native
   path prompts in adapters. The RemoteOps encapsulation pass then made all
   three Docker/process/Stats pane children private, routed views through
   immutable presentation values, and moved menu, scrolling, details/Compose,
   PID-scoped interaction and typed job transitions behind the domain owner.
   SSH launch and terminal-status mirroring remain app-level adapters. The AI
   encapsulation pass then restricted its six child states to the AI module,
   migrated roughly three hundred cross-feature reads and writes to owner APIs,
   and coupled settings snapshot/restore, overlay exclusion, confirmations,
   external request/focus preparation, detected-error throttling and Agent
   capture/reset transitions. The following Shell pass then restricted its
   seven children to `features::shell`, migrated roughly three hundred external
   reads and writes, and coupled settings-window cleanup, root menus,
   connection-failure chrome, new-session paths and pane-owner rebuilding on
   `ShellFeatureState`. GPUI windows/rendering, persistence, notification and
   terminal coordination remain adapters. Cloud sync
   made secret-field routing inaccessible outside
   its owner, and
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
   A later method-level audit corrected the earlier claim that the block-level
   review was sufficient: it found and removed thirty-two thin owner-local
   session, transfer and sync-input facades without changing the 237-block
   inventory. The remaining methods are view/render, GPUI, persistence, service
   or cross-feature adapters, and future audits must inspect method bodies and
   call sites rather than infer ownership from the block count. The exact
   composition-root field set, removed-facade names and absence of a root
   domain-constant bucket are now architecture checks.
4. Done. No store is a projection any more; the three that remain own real
   state. If a future domain wants Entity ownership, migrate it authoritatively
   rather than reintroducing a published read model.
5. Done for the original 4,000-line production files. `core/storage.rs` and
   `transport/lib.rs` were split by domain rather than by individual type, so
   constants, records, helpers, tests and dependencies moved together. The
   terminal model and surface entity instead moved their focused tests into
   normal child modules, preserving private access without disturbing their
   frame and paint hot paths. Table definitions, serialized records,
   encryption paths, backup formats, legacy fallback behavior, and the
   SSH/SFTP/X11 protocol paths remain unchanged. No Rust source file now
   exceeds 3,000 lines; further splits remain staged candidates only when a
   cohesive ownership, rendering or protocol boundary is found.
   `core/ai.rs` is done too, down from 4,032 to 1,547.
6. Done. The final explicit-import pass removed `features/prelude.rs`; a
   crate-wide guard prevents a replacement shared feature prelude.
7. Done. The migration capability/service models had no consumer outside the
   retired dashboard, so their `nyaterm-core` crate-root exports and module were
   removed with that feature rather than kept as a compatibility facade.
8. Revisit `nyaterm-store` only after storage modules have clearer internal
   boundaries and consumers can move without changing persistence compatibility.

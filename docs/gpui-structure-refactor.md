# GPUI Structure Refactor Plan

## Goal

Make `nyaterm-gpui` directory structure match a real module architecture:
eliminate dual-track empty shells, place code where the names claim it lives,
and keep behavior identical (pure module move / path rewrite).

## Success criteria

- No empty stub modules that only document “code lives elsewhere”.
- Single home for theme, terminal, widgets, HTTP adapters, and app features.
- `crate::ui` removed; public surface remains `AppShell` + `NyaTermApp`.
- `cargo check -p nyaterm-gpui` succeeds.
- No intentional behavior/logic changes.

## Target layout

```
crates/nyaterm-gpui/src/
  lib.rs
  app_shell/           # window root Entity, lifecycle
  entities/            # GPUI Entity stores (workspace/session/overlay/…)
  http/                # network adapters (ai, cloud_sync, translation, update)
  theme/               # ThemePalette + presentation tokens
  terminal/            # GPUI terminal painting/helpers
  widgets/             # reusable presentational components
  models/              # UI-only view models / local state types
  features/            # NyaTermApp + pages/panels/layout/runtimes
    mod.rs
    prelude.rs
    layout/
    pages/
    panels/
    *runtime*.rs       # keep runtime suffix; group stays flat under features for now
  action_links.rs      # pure helpers shared by models/features
  shortcuts.rs
  send_command.rs
  temporary_ssh_link.rs
```

## Explicit decisions

1. **Dissolve `ui/`** — it was a migration dump; not a lasting boundary.
2. **`features/` replaces `ui/view`** — owns `NyaTermApp` and all runtimes/pages/panels/layout.
3. **Top-level `theme` / `terminal` / `widgets` hold real code** — delete dual empty shells.
4. **HTTP adapters under `http/`** — `ai`, `cloud_sync`, `translation`, `update`.
5. **`models/` stays crate-level** — shared UI models used by features + terminal helpers; not domain models (`nyaterm-core`).
6. **`overlays/` removed as a fake boundary** — overlay UI stays in `features/panels` until Entity-driven overlay host is real.
7. **`views/` removed** — superseded by `features/`.
8. **Entity wiring deferred as follow-up** — stores in `entities/` stay; full notify migration of `NyaTermApp` fields is out of this structural pass.
9. **No file content splits** of 2k+ line files in this pass — only relocate modules.
10. **Visibility** — `pub(in crate::ui::view)` → `pub(in crate::features)`.
11. **External API** — `nyaterm_gpui::{AppShell, NyaTermApp}` unchanged for `nyaterm-app`.

## Non-goals (this pass)

- Splitting `app_state` / `models` into smaller files.
- Moving business logic into `nyaterm-core`.
- Completing Entity store adoption for settings/connections/transfers/ai.
- Changing GPUI render behavior or public UX.

## Implementation steps

1. Create `http/` and move `*_http.rs`.
2. Move `ui/theme.rs` → `theme/`, `ui/terminal.rs` → `terminal/`, `ui/components.rs` → `widgets/`.
3. Move `ui/models.rs` → `models/`, support helpers to crate root.
4. Move `ui/view` tree → `features/`.
5. Delete empty `views/`, `overlays/`, and residual `ui/`.
6. Rewrite `lib.rs` + bulk path/visibility updates.
7. `cargo check -p nyaterm-gpui` and fix remaining references.

## Follow-ups (not required for success of this pass)

- Group `features/*_runtime.rs` into feature folders (`features/session/`, `features/transfers/`, …).
- Wire remaining Entity stores through AppShell subscriptions.
- Split oversized files (`models`, connections page, sidebar, inspector).


## Phase 2 — Vertical feature folders (done)

Runtime and related modules under `features/` are grouped on disk while remaining
**flat Rust modules** via `#[path = "..."]` (same pattern as `panels.rs`):

| Folder | Contents |
|--------|----------|
| `session/` | session lifecycle/order/state, auth, prompts, restore, zmodem, recording |
| `terminal/` | terminal runtime/surface/selection/search/context menu, input, send-command |
| `shell/` | workspace chrome: activity bar, tabs, navigation, panels stack/resize, shortcuts |
| `connections/` | connection runtime + drag helpers |
| `transfers/` | transfer jobs/events/options/paths/widgets |
| `ai/` | AI chat/agent/jobs |
| `settings/` | settings/config/security/lock diagnostics/update |
| `remote/` | remote ops runtime |
| `tunnels/` | tunnel runtime |
| `sync/` | cloud sync runtime + providers |
| `commands/` | quick commands + command history runtime |
| `translation/` | translation runtime |

Still at `features/` root: `app_state`, `root`, `layout`, `pages`, `panels`,
`prelude`, `formatting`, `view_widgets`, `runtime_jobs`, `inspector`, `sync_input`.

Rationale: physical navigation without breaking `use super::*` / `pub(in crate::features)`.


## Phase 3 — `models/` split (done)

`models/mod.rs` was a 4k-line monolith. It is now:

| File | Responsibility |
|------|----------------|
| `terminal.rs` | terminal view/selection/performance/render cache |
| `session.rs` | session metadata, launch, paste, quick-switch |
| `navigation.rs` | nav items, activity bar, title menu, panel side |
| `network.rs` | tunnel/proxy editor state |
| `connections.rs` | connection editors, action links, suggestions |
| `remote.rs` | process/docker tabs + main/settings mode enums |
| `workspace.rs` | pane tree, tab windows, dock zones |
| `chrome.rs` | right focus, bottom panel, quick-command editor state |
| `transfers.rs` | transfer jobs + browser sort/columns |
| `security.rs` | key/otp/password/credential editors |
| `layout_state.rs` | panel resize state |
| `transfer_ui.rs` | transfer overlay/dialog state |
| `prompts.rs` | path/password/cloud/AI/translation prompt enums |
| `tests_workspace.rs` | workspace tree unit tests |

External path `crate::models::Foo` unchanged via `pub(crate) use ...::*`.


## Phase 4 — Entity stores + further splits (done)

### Entities module split
`entities/` is no longer a single file:

| File | Role |
|------|------|
| `runtime.rs` | `RuntimeStore` |
| `window_runtime.rs` | event-pump bootstrap |
| `startup_restore.rs` | open-tabs restore queue |
| `workspace.rs` / `session.rs` / `overlay.rs` | core shell snapshots |
| `domain.rs` | settings/connections/transfers/ai/cloud_sync/remote_ops snapshots |
| `handles.rs` | `UiStoreHandles` |
| `tests.rs` | unit tests |

### Domain store wiring
Placeholder unit structs replaced with snapshot stores (`replace_snapshot` + equality short-circuit).
`UiStoreHandles` now carries all domain store entities.
`AppShell` creates them, observes notify, and passes clones into `NyaTermApp`.
`publish_store_snapshots` projects live `NyaTermApp` state into every store each pump/render path.

### Models workspace split
`models/workspace.rs` → `workspace_pane.rs` + `workspace_tabs.rs`.


## Phase 5 — Large implementation splits (done)

### `layout/`
Former 3.2k `sidebar.rs` was a mixed bag. Now:
- `sidebar.rs` — left chrome / sessions / left panels
- `security_panel.rs` — security auth panel + private editor views
- `sync_history_panel.rs` — cloud sync history panel
- `view_helpers.rs` — shared presentational free helpers

### `pages/connections`
- `connections.rs` — `connections_view`
- `connections/list.rs` — list model, sort/filter, row chrome helpers

### `app_state/`
- `mod.rs` — `NyaTermApp` field struct
- `types.rs` — `TerminalRuntimeUiState`, `SessionPaneState`, `PendingSessionStart`
- `construct.rs` — `NyaTermApp::new`


## Phase 6 — Deeper implementation splits (done)

### `pages/connections/`
- `list.rs` — section model / sort / row chrome
- `view.rs` — main connections page + rows/search
- `editor.rs` — connection/group editors + delete confirms
- `menus.rs` — context menus

### `layout/`
- `security_panel.rs` / `security_editors.rs`
- `sync_history_panel.rs` / `view_helpers.rs` / `sidebar.rs`

### `inspector/`
- `ai_ask`, `commands`, `right_shell`, `right_domain`, `helpers`
- `ai_widgets/{transcript,agent,cards,history,messages}`

### `ai/ai_runtime/`
- `settings.rs` — profile/credential/action settings
- `chat.rs` — ask/chat/session/history runtime
- `helpers.rs` — builtin provider seeds

### `app_state/`
- `types` / `construct` / field struct module

## Phase 7 — Continued runtime / widget / HTTP splits (done)

Unblocked and extended structural splits (no intentional behavior changes).
Verified: `cargo check -p nyaterm-gpui -p nyaterm-app`.

### Compile unblocks from mid-split state
- `connections/connection_runtime/`: restored missing `ConnectionEditorToggle` +
  store helpers (`with_connection_store`, `persist_saved_connection`,
  `refresh_connection_auth_catalog`, …) after incomplete split
- `layout/workspace/`: shared `session_kind_icon_path` via `view_helpers`

### Feature runtime splits
| Module | Layout |
|--------|--------|
| `connections/connection_runtime/` | `helpers`, `editor`, `groups`, `actions` |
| `tunnels/tunnel_runtime/` | `helpers`, `tunnel_editor`, `proxy_editor`, `groups`, `actions` |
| `settings/security_runtime/` | `unlock`, `keys`, `otp`, `delete`, `passwords`, `credentials` |
| `terminal/terminal_runtime/` | `view_io`, `paste`, `sessions`, `scroll`, `buffer` |
| `terminal/terminal_surface/` | `helpers`, `canvas`, `chrome` |

### Layout chrome + widgets
| Module | Layout |
|--------|--------|
| `layout/` shell | `status_bar`, `title_bar`, `activity_bar`, `title_menu_helpers` (+ existing panels/workspace) |
| `view_widgets/` | `chrome`, `inspector_widgets`, `stats`, `rows`, `icons`, `markdown` |

### HTTP
| Module | Layout |
|--------|--------|
| `http/cloud_sync/` | `snippet`, `webdav`, `s3`, `google_drive`, `onedrive`, `aliyun`, `helpers`, `tests` |

### Conventions reinforced
- Prefer `#[path]` + flat module names so `use super::*` / `pub(in crate::features)` keep working
- Cross-sibling methods: `pub(in crate::features)` (private methods are not visible across split files)
- Shared free helpers: `pub(super)` + parent `use helpers::*`
- When splitting free-function files, keep doc comments attached to the correct item
- Glob re-export of `pub(super)` cannot raise visibility — public widgets use `pub(in crate::features)` in leaf modules + facade `pub(in crate::features) use …::*`


### Additional page/runtime splits (same phase)
| Module | Layout |
|--------|--------|
| `pages/settings/sync_backup/` | `backup_diagnostics`, `cloud_sync`, `helpers` |
| `pages/connections/view/` | `page`, `rows` |
| `pages/connections/editor/` | `connection`, `group_delete` |
| `ai/ai_runtime/chat/` | `jobs`, `settings_actions`, `discovery`, `history` |

### Still large / optional next splits
- Settings pages: `pages/settings/{ai,workspace,terminal}.rs` (~1.7–1.9k)
- `formatting.rs`, `quick_command_runtime.rs`, `terminal_selection_runtime.rs`
- `layout/workspace/surface.rs`, `terminal_surface/canvas.rs`, `layout/sidebar.rs`
- Optional: UI reads Entity snapshots directly (beyond `publish_store_snapshots` projection)

## Phase 8 — Settings pages + formatting + more runtimes (done)

Verified: `cargo check -p nyaterm-gpui -p nyaterm-app`.

### Settings pages
| Module | Layout |
|--------|--------|
| `pages/settings/ai/` | `section`, `models`, `rules`, `helpers` |
| `pages/settings/workspace/` | `general`, `appearance`, `interaction`, `keybindings` |
| `pages/settings/terminal/` | `general`, `search`, `keywords`, `helpers` |

### Shared helpers / widgets
| Module | Layout |
|--------|--------|
| `formatting/` | `labels`, `ai_history`, `connection_icons`, `markdown` |
| `layout/title_bar/` | `bar`, `menu` |
| `layout/workspace/surface/` | `tabs`, `menus`, `empty` |
| `panels/` | existing overlays + `send_command_bar` / `send_command_helpers` |

### Runtimes
| Module | Layout |
|--------|--------|
| `commands/quick_command_runtime/` | prior import/variables + `catalog`, `run`, `dialogs`, `editor`, `helpers` |
| `commands/command_runtime/` | `history`, `suggestions`, `helpers` |
| `settings/settings_runtime/` | `recording_transfer`, `terminal_remote`, `general_interaction`, `search_engines`, `helpers` |
| `remote/remote_runtime/` | `process`, `stats`, `docker`, `helpers` |
| `terminal/terminal_selection_runtime/` | `metrics`, `selection`, `action_links`, `smart_input`, `helpers` |


### Additional page/runtime splits (same phase)
| Module | Layout |
|--------|--------|
| `pages/settings/sync_backup/` | `backup_diagnostics`, `cloud_sync`, `helpers` |
| `pages/connections/view/` | `page`, `rows` |
| `pages/connections/editor/` | `connection`, `group_delete` |
| `ai/ai_runtime/chat/` | `jobs`, `settings_actions`, `discovery`, `history` |

### Still large (optional)
- `terminal_surface/canvas.rs` (~981, mostly one method)
- `title_bar/menu.rs` (~733, one dropdown builder)
- `pages/settings/sync_backup.rs`, connections `view`/`editor`, transfer pages
- `terminal/mod.rs`, `ai_runtime/chat.rs`, sidebar/security panels

## Phase 9 — Terminal module split + GPUI hardware-accel cleanup (done)

Verified: `cargo check -p nyaterm-gpui -p nyaterm-app`.

### `crates/nyaterm-gpui/src/terminal/`
Repaired incomplete split into a coherent paint stack (no behavior change):

| File | Role |
|------|------|
| `mod.rs` | shared imports + re-exports for submodules / crate API |
| `types.rs` | shared `TerminalHighlightSpan` |
| `element.rs` | `NyaTerminalElement` (`Element` impl) |
| `paint.rs` | line paint, bg flush, decorations, `terminal_line_element` |
| `ansi.rs` | ANSI → highlight spans |
| `keywords.rs` | keyword highlights + buffer search matches |
| `input.rs` | key bytes + screen bootstrap helpers |
| `tests.rs` | unit tests |

This matches the intended stack: **alacritty_terminal snapshot → GPUI custom Element**, with
`features/terminal/terminal_surface/{canvas,chrome}.rs` only as app chrome (scrollbar/search),
not a second renderer.

### GPUI settings
- Removed **Hardware acceleration** toggle from GPUI Terminal settings UI
- Removed `toggle_terminal_hardware_acceleration` runtime handler
- Core/storage field kept for settings-file compatibility with legacy/Tauri paths
  (unused by `NyaTerminalElement`)

### Other splits landed earlier in this phase wave
- `layout/sidebar/{shell,sessions,panels}`
- `panels/tab_actions_overlay/{overlay,compact,expanded}`
- `panels/quick_commands_panel/{panel,helpers}`
- `pages/transfers/editor/{open,lifecycle,input_sync,helpers}`

### Naming note
`terminal_surface/chrome.rs` = scrollbar + search bar (“UI chrome”), **not** Chromium / GPU.

## Phase 10 — Tunnel page split + cleanup (done)

Verified: `cargo check -p nyaterm-gpui -p nyaterm-app`.

### Warning cleanup
- Removed split-generated unused `use super::*` imports in formatting/settings/runtime helpers
- Narrowed terminal facade imports so private paint internals are not re-exported as crate API
- Removed stale `std::hash::Hasher` imports from models split files
- Fixed `ConnectionSection` visibility after the connections page split

### `pages/tunnels/tunnel/`
Former ~1k-line `tunnel.rs` is now a facade over focused free-function modules:

| File | Role |
|------|------|
| `sections.rs` | grouped tunnel list sections and move picker |
| `row.rs` | tunnel row, compact switch, action icons, status styling |
| `editor.rs` | tunnel editor modal and editor controls |
| `filters.rs` | tunnel search predicate |

## Phase 11 — Remote process, transfer settings, imports, and helper splits (done)

Verified:

- `cargo check -p nyaterm-gpui -p nyaterm-app`
- `cargo check -p nyaterm-gpui --tests`

### Page and runtime splits

| Module | Layout |
|--------|--------|
| `pages/remote/process/` | `data`, `table`, `details`, `resources` |
| `pages/settings/transfer/` | `files`, `editor`, `advanced`, `recording` |
| `commands/quick_command_runtime/import/` | `dialog`, `sources`, `json`, `merge`, `helpers`, `tests` |
| `pages/transfers/helpers/` | `paths`, `properties`, `editor`, `job_row`, `browser`, `queue` |

All four former 800–950 line files are now small facades. Existing call paths are preserved through
parent imports/re-exports, and cross-sibling visibility remains limited to the owning feature.

### Cleanup and test boundary repair

- Removed five unreferenced private transfer-settings helpers and their stale import
- Repaired cloud-sync provider test visibility that was lost in the Phase 7 provider split
- Provider-only test helpers remain scoped to `http::cloud_sync`; no new crate API was exposed
- Warning count reduced from 85 to 80 in the regular build (79 in the test build)

### Still large

- `pages/connections/editor/connection.rs` (~1,049; one large editor builder)
- `terminal/terminal_surface/canvas.rs` (~981; one rendering method)
- `panels/send_command_bar.rs` (~898; one panel builder)
- `pages/settings/sync_backup/cloud_sync.rs` (~863; one provider-heavy settings builder)

These need local builder/view-model extraction before another file-only split would be meaningful.

## Phase 12 — Protocol/provider builders + workspace tab model split (done)

Verified:

- `cargo check -p nyaterm-gpui -p nyaterm-app`
- `cargo check -p nyaterm-gpui --tests`
- `git diff --check`

### Connection editor builders

The former ~1,049-line connection editor method now keeps common fields and modal chrome in
`pages/connections/editor/connection.rs`, with protocol-specific fields in:

| File | Role |
|------|------|
| `connection/ssh.rs` | host/auth/key/OTP/proxy/jump/post-login fields |
| `connection/local.rs` | shell, arguments, working directory, and launch flags |
| `connection/telnet.rs` | host/port, backspace, raw TCP, and local echo fields |
| `connection/serial.rs` | port, baud, framing, and backspace fields |

The main builder is ~310 lines; the largest protocol builder is ~328 lines.

### Cloud-sync provider builders

`pages/settings/sync_backup/cloud_sync.rs` now owns provider selection, sync actions, conflict state,
and history. The seven provider credential forms moved to `cloud_sync/providers.rs`:
WebDAV, S3, Google Drive, OneDrive, Aliyun Drive, Gitee, and GitHub.

The main settings builder dropped from ~863 to ~584 lines. This extraction also removed historical
trailing whitespace that prevented standalone `rustfmt` from formatting the file.

### Workspace tab model

`models/workspace_tabs.rs` is now a ~98-line type facade over focused `TerminalWindowNode` impls:

| File | Role |
|------|------|
| `workspace_tabs/tree.rs` | construction, traversal, lookup, and leaf insertion |
| `workspace_tabs/mutation.rs` | removal, splitting, moving, and split ratios |
| `workspace_tabs/persistence.rs` | serialize/restore Tauri-compatible layouts |
| `workspace_tabs/docking.rs` | tab reorder, edge docking, and unique-tab normalization |

Cross-group helpers use parent-module visibility; the existing `pub(crate)` model API is unchanged.

### Next large modules

- `terminal/terminal_surface/canvas.rs` (~981; terminal interaction/render shell)
- `panels/send_command_bar.rs` (~898; one state-heavy panel builder)
- `panels/quick_commands_panel/panel.rs` (~832; one panel builder)
- `pages/settings/ai/models.rs` (~813; one model-management builder)

## Phase 13 — Dense panel/page builders (done)

Verified:

- `cargo check -p nyaterm-gpui -p nyaterm-app` (80 existing warnings)
- `cargo check -p nyaterm-gpui --tests` (79 existing warnings)
- `git diff --check`

### Quick Commands and AI Models

| Module | Layout |
|--------|--------|
| `panels/quick_commands_panel/panel/` | `rows`, `sidebar`; the panel facade owns filtering and shell composition |
| `pages/settings/ai/models/` | `model_groups`, `credential_rows`; the facade owns model-section composition |

The former ~832-line Quick Commands panel and ~813-line AI Models settings builder are now split
along repeated row/sidebar and model/provider credential boundaries. Existing event handlers and
builder data flow are unchanged.

### Security, Docker Compose, and Tunnel Proxy

| Module | Layout |
|--------|--------|
| `layout/security_editors/` | `key`, `otp`, `password`, `credential` |
| `pages/remote/docker_compose/` | `project`, `service`, `menus`, `status` |
| `pages/tunnels/proxy/` | `sections`, `rows`, `helpers`, `editor` |

These three facades now group related editor, row, menu, and status builders without broadening
their feature-level APIs. Cross-sibling helpers remain scoped to the owning page or feature.

## Phase 14 — Workspace rendering + shortcut runtime boundaries (done)

Verified:

- `cargo check -p nyaterm-gpui -p nyaterm-app` (80 existing warnings)
- `cargo check -p nyaterm-gpui --tests` (79 existing warnings)
- `git diff --check`

### Workspace page

`pages/workspace.rs` dropped from ~778 lines to an ~83-line page coordinator. Its two recursive
rendering domains now live in:

| File | Role |
|------|------|
| `pages/workspace/terminal_windows.rs` | multi-leaf terminal window tabs, docking, and drop overlay |
| `pages/workspace/panes.rs` | single-tab pane tree and split rendering |

The coordinator still owns workspace selection, prompt banners, empty state selection, and bottom
panel composition. Child render methods use parent-only visibility.

### Keybinding runtime

The former mixed `shell/keybinding_runtime.rs` is now a six-line facade over:

| File | Role |
|------|------|
| `keybinding_runtime/keybindings.rs` | recording, persistence, conflict labels, and search input |
| `keybinding_runtime/keyword_highlights.rs` | keyword rule import, editing, persistence, and keyboard input |

All existing feature-level method signatures are unchanged.

## Phase 15 — AI settings runtime domains (done)

Verified:

- `cargo check -p nyaterm-gpui -p nyaterm-app` (80 existing warnings)
- `cargo check -p nyaterm-gpui --tests` (79 existing warnings)
- `git diff --check`

`ai/ai_runtime/settings.rs` is now an eight-line facade over three runtime domains:

| File | Role |
|------|------|
| `settings/profile.rs` | AI mode, execution, safety, limits, and history toggles |
| `settings/credentials.rs` | provider credential lifecycle, editing, and keyboard input |
| `settings/models.rs` | enabled/default models, manual models, groups, and keyboard input |

The former ~684-line mixed runtime is now split into files of roughly 145, 318, and 225 lines.
All methods retain their existing `pub(in crate::features)` API and behavior.

## Phase 16 — Local GPUI builder extraction (done)

Verified:

- `cargo check -p nyaterm-gpui -p nyaterm-app` (80 existing warnings)
- `cargo check -p nyaterm-gpui --tests` (79 existing warnings)
- `git diff --check`

### Send Command panel

The former ~898-line `panels/send_command_bar.rs` is now a ~52-line coordinator. Unlike earlier
method-only moves, this extraction introduces a render-only `SendCommandBarViewState` so derived
target labels, validation, preview, and progress values are calculated once and shared by focused
builders:

| File | Role |
|------|------|
| `send_command_bar/state.rs` | immutable derived state for one render pass |
| `send_command_bar/header.rs` | title, target type/scope, and hide action |
| `send_command_bar/controls.rs` | data, target, mode, count, interval, and EOL controls |
| `send_command_bar/editor.rs` | text/hex input, scrolling guides, and hex preview |
| `send_command_bar/footer.rs` | send progress, validation summary, and actions |

The existing listeners still call the same `NyaTermApp` runtime methods. The panel dimensions and
child order are unchanged.

### Security auth panel

`layout/security_panel/panel.rs` dropped from ~768 to ~200 lines. Keys, passwords, credentials, and
OTP now have independent tab-body builders under `security_panel/panel/`. Each returns `gpui::Div`,
allowing the coordinator to append the existing delete confirmation without adding a wrapper or
changing flex/overflow behavior. Header tabs, status/actions, secret footer, and unlock overlay
remain coordinated by the root panel method.

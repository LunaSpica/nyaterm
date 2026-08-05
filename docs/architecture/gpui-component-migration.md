# gpui-component Migration

NyaTerm is moving reusable form and overlay controls behind `nyaterm-ui`
wrappers backed by `gpui-component`. Desktop feature modules should keep using
NyaTerm component names rather than importing `gpui_component` directly.

## Component Inventory

| Current implementation | Main call sites | Target component | Risk | Migrate |
| --- | --- | --- | --- | --- |
| Custom ordinary `TextField` | Ordinary forms and search | `Input/InputState` | Medium | Done for ordinary inputs |
| Custom numeric steppers/text boxes | Connection/network editors, settings, send-command, session delay, OTP, process nice | `NumberInput/InputState` | Medium | Done for ordinary numeric inputs |
| `small_button` | Common action buttons | `Button` | Low | Yes |
| `network_switch_button` / settings switches | Tunnel list, settings pages, Telnet editor, keyword-highlight rules | `Switch` | Low | Yes |
| Manual Select | Settings, commands, tunnels | `Select` | Medium | Yes |
| Manual popup menus | Title bar, connections, terminal | `PopupMenu/ContextMenu` | Medium | Yes |
| `modal_dialog_shell` | CRUD and confirmation dialogs | `Dialog` | Medium | Partial |
| Terminal character area | Terminal | Custom GPUI | High | No |
| `RemoteTextEditor` | SFTP editor | Dedicated editor | High | No |

## Phase Status

Core component phases are in place for reusable controls:

| Item | Status | Evidence |
| --- | --- | --- |
| Add exact `gpui-component` dependency | Done | `gpui-component` is vendored at `vendor/gpui-component/crates/ui` as `0.5.2` at `e1570bdc8fd2dc17d38cab09e74b1783bdf3b24b`. |
| Verify GPUI version consistency | Done | `cargo tree -i gpui` shows one active `gpui v0.2.2` from Zed `4aad57fd1f002f9feeea2b7fb6229ccbcd576cb1` at `vendor/zed/crates/gpui`; `gpui-component` uses that same path. |
| Initialize component library | Done | `gpui_component::init(cx)` runs before the main window opens. |
| Establish `nyaterm-ui` wrappers | Done | `NyaInput`, `NyaButton`, `NyaIconButton`, `NyaSwitch`, `NyaCheckbox`, `NyaRadioGroup`, `NyaTabs`, `NyaSelect`, `NyaDialog`, `NyaConfirmDialog`, `NyaTooltip`, menu wrappers, and `NyaRoot` are exported. |
| Theme bridge | Done for Phase 0 | `apply_component_theme(ThemePalette, &mut App)` maps NyaTerm colors into component theme colors. Startup calls `sync_component_theme` after the main window opens; user-driven appearance changes call it after updating the NyaTerm theme id; UI-triggered store reloads use `refresh_store_from_runtime_and_sync_theme`. |
| Root on all component windows | Done for Phase 0 | Main, settings, connection editor, quick-command editor, remote editor, and external-sync windows open `NyaRoot` first through `nyaterm-ui::nya_root`; stored child-window handles use `NyaWindowHandle`. |

Phase 1 has migrated ordinary inputs: the id-keyed `TextInputRegistry`,
connection-list search, connection-editor fields, and group-name editor now
store `NyaInputState` and render `gpui-component` `Input` through `NyaInput` or
the entity render path. Ordinary numeric fields now use
`nyaterm-ui::NyaNumberInputState` / `NyaNumberInput`, backed by
`gpui_component::input::NumberInput`, while existing feature state remains
authoritative for values, validation and persistence. The old `text_field.rs`
custom caret, hit-testing, selection, paste normalization, and IME
implementation has been deleted. Manual GUI/IME verification is still required
before claiming the full component migration complete.

## Current Search Inventory

Required pre-migration searches were run on 2026-07-30:

| Search | Result summary |
| --- | --- |
| `TextField\|text_input_box\|text_input_field` | Ordinary inputs now use `NyaInputState`; no code exports or imports the old `TextField` implementation. Registry helper names remain for business routing compatibility. |
| `number_stepper\|stepper_button\|NumberInput` | User-editable numeric fields now go through `nyaterm-ui::NyaNumberInput`; remaining ordinary text boxes are textual, masked, multiline, search/path, or semantics-preserving fields such as octal file modes. |
| `small_button\|mode_button\|svg_icon_button` | `small_button`, `mode_button`, `settings_choice_chip`, and `dialog_action_button` now render through `NyaButton`; `svg_icon_button` and `modal_close_icon_button` now render through asset-path based `NyaIconButton`. |
| `modal_dialog_shell\|modal_dialog_footer` | Only two retained custom shells remain: the inline connection-editor fallback and Docker details surface. |
| `overflow_menu\|select_trigger\|select_menu\|selector` | Ordinary selects and action menus use `NyaSelect`/`NyaDropdownMenu`/`NyaContextMenu`; remaining hits are dedicated large-list, editor, terminal, or domain-workspace surfaces. |
| `stand-in\|Tauri Switch\|Tauri Dialog\|Tauri Tabs` | Tunnel, settings, Telnet option, and keyword-highlight rule switch stand-ins now use `NyaSwitch`; tunnel list and Telnet segmented tabs now use `NyaTabs`; dialog comments and other tab comments identify remaining migration candidates. |
| `.absolute().*menu\|.tooltip(` | Manual tooltip/menu positioning is concentrated in title/session/connection/transfer/settings surfaces. |

## Phase 2 Notes

The common `small_button`, `mode_button`, settings `settings_choice_chip`, and
shared `dialog_action_button` helpers keep their public call-site signatures but
now delegate to `nyaterm-ui::NyaButton`, so ordinary small, selected-mode,
choice-chip, and primary/danger action buttons use the component button
implementation without making desktop feature modules import `gpui_component`.
`NyaIconButton` now accepts NyaTerm vendored SVG asset paths instead of
component-library icon names, and the shared `svg_icon_button` and
`modal_close_icon_button` helpers delegate through it. The tunnel row open/close
control, shared settings switch helpers, Telnet option switches, and
keyword-highlight rule switches render controlled `NyaSwitch` instances; tunnel
runtime/open state, connection-editor drafts, and settings feature state remain
authoritative.
The tunnel list's two-tab selector and the Telnet editor's two-tab segmented
selector now render through `nyaterm-ui::NyaTabs`, backed by the component
`TabBar`, while `NetworkTab` and `ConnectionEditorTelnetTab` remain the
authoritative selected values.
`nyaterm-ui::NyaSelectState` now wraps `gpui-component` `SelectState` for
string-valued option sets, supports NyaTerm-owned per-option font previews, and
emits `NyaSelectEvent::Changed`. The desktop `SelectRegistry` retains those
entities by stable control id while persisted settings remain authoritative for
the selected value. UI and terminal themes, minimum contrast, wallpaper fit,
normal/bold terminal font weights, cursor style, every UI/terminal font-stack
row, and AI smart-execution risk now use `NyaSelect`. Their manual trigger,
inline option list, and shared `AppearanceSettingsState.menu_open` mirror were
removed; popup focus/open state belongs only to the component entities.
Tunnel type/group/SSH-connection and proxy protocol/group editor controls now
use the same registry with direct selection instead of click-to-cycle cards.
The send-command data type, mode, target, and line-ending controls also use
component selects; their four manual absolute menus and four pure-UI open flags
were deleted while the send state still gates changes during an active run.
Cloud-sync provider selection is component-backed and no longer stores
`provider_menu_open` in `CloudSyncFeatureState`.
Numeric form controls now follow the same boundary. `nyaterm-ui::NyaNumberInput`
wraps component `NumberInput` with range, step, decimal formatting, disabled,
prefix/suffix and infinity support. The desktop registry keeps number input
entities by stable id and routes their text events through existing feature
owners. Migrated fields include connection editor port/baud/delay, tunnel and
proxy ports, settings limits and intervals, send-command count/interval,
session startup delay, OTP digits/period/counter, and remote process nice
values. Persisted formats and save-time clamps remain unchanged.

Header-specific icon buttons, remote Docker tabs, settings sidebar tabs, and the
terminal search's dedicated mode buttons remain custom in this slice because
they carry window-chrome, domain-specific, or terminal-search behavior that
should migrate with their broader domain passes.

## Phase 3 and 4 Notes

Ordinary select, popup-menu and context-menu surfaces now go through
`nyaterm-ui` wrappers. Title-bar menus, connection list menus, quick-command row
and category menus, active-session menus, network overflow menus, send-command
selectors, tunnel/proxy editor selectors, appearance/settings selectors,
cloud-sync provider selection, and the terminal context-menu shell all avoid
direct `gpui_component` imports in desktop feature modules. Where a selected
value is persisted or workflow-significant, the feature owner remains
authoritative; component entities own only popup/focus interaction state.

`NyaDialog` and `NyaConfirmDialog` now host ordinary CRUD, form, warning and
confirmation flows. Migrated dialogs include network groups, tunnels, proxies,
connection/group delete confirmations, quick-command create/edit/delete,
quick-command category delete/rename, SFTP create/rename/move/delete/properties
and unknown-file prompts, Docker and process confirmations, security delete
confirmations, AI clear-history and auto-execution confirmations, close-all
sessions, session rename/startup-command/temporary-SSH prompts, quick-command
import, connection import, translation result, and native update checks.
Former pure visual state such as import-dialog open flags, import focus handles,
update dialog open state, AI confirmation booleans, close-all confirmation
state, category rename focus, and SFTP properties focus mirrors was removed.
Workflow state such as active import path prompts, translation job/result state,
transfer operation drafts, and pending close-all-session behavior remains on
the focused feature owners.

## Ownership Rules

Business state remains authoritative for persisted or workflow-significant
values. Component state can own pure UI interaction state only when the former
FeatureState mirror is removed in the same change. No desktop feature module
should hold both `FeatureState.menu_open` and a component-owned menu entity for
the same menu.

## Dialog Migration Notes

Two `modal_dialog_shell` call sites remain intentionally retained:

| Call site area | Current reason to keep |
| --- | --- |
| Inline connection-editor fallback | This is the complete connection editor surface reused only when a detached editor window cannot host the draft. It contains protocol-specific sections, validation, icon picking, radio/select controls, and save lifecycle behavior rather than a small CRUD form. The normal detached connection editor window already has `NyaRoot`. |
| Docker details surface | This is a dense, scrollable remote-inspection surface with live Docker metadata, resource sections, copy actions and action gating. It is closer to a read-only workspace/details view than an ordinary confirmation or form dialog. |

## Large List Assessment

| Area | Current data volume | Current selection model | Component candidate | Worth migrating now |
| --- | --- | --- | --- | --- |
| SFTP file browser | Potentially large remote directories | Multi-select, context menu, path actions | `List`/`Table` | No |
| Transfer queue | Potentially large active/history queue | Job rows with live state | `List`/`Table` | No |
| Docker containers and Compose | Medium, remote-refresh driven | Action rows and details | `Table` | No |
| Process manager | Potentially large and frequently refreshed | Filtered rows and details | `Table`/`VirtualList` | No |
| Connection tree | User catalog with groups | Tree expansion, selection, context menus | `Tree` | No |
| AI session history | Medium, provider/workflow coupled | Session selection and actions | `List` | No |
| Terminal buffer | Very large/high refresh | Terminal-specific grid and selection | None | No |

## Retained Custom Surfaces

Terminal input, the terminal character grid, paste review, `RemoteTextEditor`,
the built-in transfer editor, AI mention completion, quick switch keyboard
navigation, and high-refresh or multi-select list surfaces keep their dedicated
models. They are not ordinary form fields and should not be mechanically
replaced with a single-line component input.

## Dependency Boundary

`nyaterm-ui` owns the `gpui-component` integration and exports stable NyaTerm
component names. An architecture check rejects `gpui_component` imports under
`crates/nyaterm-desktop/src/features`.

## Verification Log

| Command | Result |
| --- | --- |
| `cargo tree -i gpui` | Passed on 2026-08-05; one active path-based `gpui v0.2.2` from Zed `4aad57fd1f002f9feeea2b7fb6229ccbcd576cb1`. |
| `cargo tree -d` | Completed on 2026-08-01; duplicate transitive crates remain, but no duplicate `gpui` was introduced. |
| `cargo check -p nyaterm-ui` | Passed after wrapper scaffolding. |
| `cargo test -p nyaterm-ui` | Passed on 2026-08-05; 30 tests. |
| `cargo check -p nyaterm-desktop` | Passed after Root handle migration, ordinary input migration, numeric input migration, and the import/translation/update dialog migration. |
| `cargo test -p nyaterm-desktop` | Passed on 2026-08-05; 812 tests. |
| `cargo test -p nyaterm-terminal-gpui` | Passed on 2026-08-05; 124 tests passed and 1 benchmark test was ignored. |
| `cargo test -p nyaterm-transport` | Passed on 2026-08-05; 147 tests. The SFTP service E2E remained ignored without `NYATERM_TEST_SFTP_*`. |
| `cargo check -p nyaterm-app` | Passed on 2026-08-05 after updating the four vendored snapshots. |
| `cargo run -p nyaterm-app --bin nyaterm` | The Linux binary started, rendered the root view, and remained running during a 10-second smoke check. Segmented tabs, inputs, and dialogs were not visually verified because no display service was available. |
| `bash scripts/check-architecture-boundaries.sh` | Passed on 2026-08-05; desktop feature modules do not import `gpui_component` directly. |
| `cargo fmt --all -- --check` | Failed on 2026-08-05 because stable rustfmt would reformat the fixed upstream `gpui-component` snapshot; the explicit NyaTerm workspace-package format check passed. The upstream snapshot was not rewritten. |
| `cargo check --workspace` | Passed on 2026-08-05. |
| `cargo test --workspace` | Passed on 2026-08-05; SFTP service E2E remains ignored without `NYATERM_TEST_SFTP_*` variables. |
| `cargo clippy --workspace --all-targets` | Completed successfully on 2026-08-05 with 22 warnings from unchanged NyaTerm source under stable Rust 1.97.1. |

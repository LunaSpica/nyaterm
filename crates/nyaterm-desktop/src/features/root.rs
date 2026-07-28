use std::time::{Duration, Instant};

use crate::models::{MainMode, NavItem, PanelResizeSide};
use gpui::{
    AnyElement, Context, Div, ImageSource, IntoElement, KeyDownEvent, MouseButton, MouseMoveEvent,
    MouseUpEvent, NavigationDirection, ObjectFit, Render, SharedString, Stateful, Window, div, img,
    prelude::*, px, rgb, rgba, svg,
};

use super::terminal::{FULL_SHELL_PAINT_COUNT, terminal_surface_paint_count};
use super::{ActivitySide, NyaTermApp, ThemePalette};

const WALLPAPER_TILE_ELEMENT_LIMIT: usize = 8192;
const WALLPAPER_TILE_MIN_SIZE: f32 = 8.;

/// Which overlays the root chrome should render this frame.
///
/// Computed from `NyaTermApp` directly. This used to be read back from a
/// snapshot the same `Render` pass had just published into `OverlayStore`,
/// with a fallback that recomputed exactly these expressions.
struct OverlayFlags {
    tab_actions_open: bool,
    rename_open: bool,
    color_picker_open: bool,
    session_info_open: bool,
    startup_command_open: bool,
    temporary_ssh_link_open: bool,
    multi_line_paste_open: bool,
    terminal_actions_open: bool,
    terminal_context_menu_open: bool,
    action_link_menu_open: bool,
    action_link_tooltip_open: bool,
    command_suggestions_open: bool,
    credential_suggestions_open: bool,
    close_all_sessions_confirm_open: bool,
    locked: bool,
}

impl NyaTermApp {
    pub(crate) fn start_after_window_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_window_render_inputs(window, cx);
        self.start_terminal_frame_event_wake(cx);
        self.try_restore_open_tabs(window, cx);
        let should_pump = self.stores.startup_restore.update(cx, |store, _| {
            store.can_pump_queue(self.has_pending_session_start())
        });
        if should_pump {
            self.pump_startup_restore_queue(window, cx);
        }

        self.ensure_terminal_focus_reporting(window, cx);
    }

    fn root_chrome(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Stateful<Div> {
        self.ensure_paint_theme_caches();
        let palette = self.theme_palette();
        let wallpaper_path = self
            .settings
            .background_image_path
            .as_ref()
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .filter(|p| std::path::Path::new(p).is_file())
            .map(|p| p.to_string());
        let wallpaper_opacity = (self.settings.background_image_opacity.min(100) as f32) / 100.0;
        let wallpaper_fit = self.settings.background_image_fit.clone();
        let wallpaper_tile_size = wallpaper_path
            .as_deref()
            .filter(|_| wallpaper_fit == "tile")
            .map(|path| self.wallpaper_tile_size(path));
        div()
            .id(SharedString::from("nyaterm-root"))
            .size_full()
            .relative()
            .bg(self.shell_transparent_color(palette.bg))
            .text_color(rgb(palette.text))
            .font_family(self.gpui_ui_font_family())
            .text_size(px(self.settings.ui_font_size.clamp(12, 24) as f32))
            .on_click(cx.listener(|this, _, _, _| {
                this.mark_user_activity();
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    let changed = this.shell.chrome.title_menu_open.is_some()
                        || this.shell.chrome.header_status.menu_open
                        || this.shell.chrome.open_tabs_menu_open
                        || this.shell.chrome.new_session_menu_open
                        || this.shell.chrome.new_session_all_sessions_open
                        || !this.shell.chrome.new_session_group_menu_path.is_empty()
                        || this.remote_ops.docker.tab_menu_open
                        || this.remote_ops.docker.header_menu_open
                        || this.connection_state.list_more_menu_is_open();
                    if changed {
                        this.shell.chrome.title_menu_open = None;
                        this.shell.chrome.title_menu_submenu = None;
                        this.shell.chrome.header_status.menu_open = false;
                        this.shell.chrome.open_tabs_menu_open = false;
                        this.shell.chrome.new_session_menu_open = false;
                        this.shell.chrome.new_session_all_sessions_open = false;
                        this.shell.chrome.new_session_group_menu_path.clear();
                        this.remote_ops.docker.tab_menu_open = false;
                        this.remote_ops.docker.header_menu_open = false;
                        this.connection_state.close_list_more_menu();
                        cx.notify();
                    }
                }),
            )
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if this.modal_child_window_open() {
                    this.activate_modal_child_window(cx);
                    cx.stop_propagation();
                    return;
                }
                if this.handle_global_shortcut(event, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                if this.modal_child_window_open() {
                    return;
                }
                this.update_transfer_browser_column_resize(event, cx);
                this.update_panel_resize(event, cx);
                this.update_transfer_height_resize(event, cx);
                this.update_bottom_panel_resize(event, cx);
                this.update_panel_stack_resize(event, cx);
                this.update_workspace_split_resize(event, cx);
                if this.maybe_send_terminal_any_motion_report(event, cx) {
                    return;
                }
                this.update_terminal_selection_drag(event, cx);
                this.update_terminal_scrollbar_drag(event, cx);
                this.update_action_link_hover(event, cx);
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    if this.modal_child_window_open() {
                        return;
                    }
                    this.finish_transfer_browser_column_resize(event, cx);
                    this.finish_panel_resize(event, cx);
                    this.finish_transfer_height_resize(event, cx);
                    this.finish_bottom_panel_resize(event, cx);
                    this.finish_panel_stack_resize(event, cx);
                    this.finish_workspace_split_resize(event, cx);
                    this.finish_terminal_selection(event, cx);
                    this.finish_terminal_scrollbar_drag(cx);
                    this.clear_terminal_window_drop(cx);
                }),
            )
            .on_mouse_up(
                MouseButton::Navigate(NavigationDirection::Back),
                cx.listener(|this, _event: &MouseUpEvent, window, cx| {
                    if this.modal_child_window_open() {
                        this.activate_modal_child_window(cx);
                        return;
                    }
                    if this.current_left_panel() == Some(NavItem::Transfers) {
                        cx.stop_propagation();
                        this.open_transfer_browser_history(1, window, cx);
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Navigate(NavigationDirection::Forward),
                cx.listener(|this, _event: &MouseUpEvent, window, cx| {
                    if this.modal_child_window_open() {
                        this.activate_modal_child_window(cx);
                        return;
                    }
                    if this.current_left_panel() == Some(NavItem::Transfers) {
                        cx.stop_propagation();
                        this.open_transfer_browser_history(-1, window, cx);
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Right,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    if this.modal_child_window_open() {
                        return;
                    }
                    if this.finish_terminal_mouse_report(event, cx) {
                        cx.stop_propagation();
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Middle,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    if this.modal_child_window_open() {
                        return;
                    }
                    if this.finish_terminal_mouse_report(event, cx) {
                        cx.stop_propagation();
                    }
                }),
            )
            .when_some(wallpaper_path.clone(), |this, path| {
                let source: ImageSource = std::sync::Arc::<std::path::Path>::from(
                    std::path::PathBuf::from(path).into_boxed_path(),
                )
                .into();
                if let Some((tile_width, tile_height)) = wallpaper_tile_size {
                    let (columns, rows) =
                        wallpaper_tile_grid(self.shell.viewport.size, (tile_width, tile_height));
                    let mut layer = div()
                        .absolute()
                        .inset_0()
                        .overflow_hidden()
                        .opacity(wallpaper_opacity);
                    for row in 0..rows {
                        for column in 0..columns {
                            layer = layer.child(
                                img(source.clone())
                                    .absolute()
                                    .left(px(column as f32 * tile_width))
                                    .top(px(row as f32 * tile_height))
                                    .w(px(tile_width))
                                    .h(px(tile_height))
                                    .object_fit(ObjectFit::None),
                            );
                        }
                    }
                    return this.child(layer);
                }
                let object_fit = match wallpaper_fit.as_str() {
                    "contain" => ObjectFit::Contain,
                    "stretch" | "fill" => ObjectFit::Fill,
                    _ => ObjectFit::Cover,
                };
                let image = img(source)
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .object_fit(object_fit)
                    .opacity(wallpaper_opacity);
                this.child(image)
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .size_full()
                    .child(self.title_bar(window, cx))
                    .child(self.workspace_surface(palette, window, cx)),
            )
    }

    fn wallpaper_tile_size(&mut self, path: &str) -> (f32, f32) {
        if let Some((cached_path, width, height)) =
            self.shell.viewport.wallpaper_tile_dimensions.as_ref()
            && cached_path == path
        {
            return fit_wallpaper_tile_size(
                self.shell.viewport.size,
                (*width as f32, *height as f32),
            );
        }
        let (width, height) = image::image_dimensions(path)
            .ok()
            .filter(|(width, height)| *width > 0 && *height > 0)
            .unwrap_or((256, 256));
        self.shell.viewport.wallpaper_tile_dimensions = Some((path.to_string(), width, height));
        fit_wallpaper_tile_size(self.shell.viewport.size, (width as f32, height as f32))
    }

    fn workspace_surface(
        &mut self,
        palette: ThemePalette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if self.shell.navigation.main_mode == MainMode::Page
            && self.shell.navigation.selected_nav == NavItem::Settings
        {
            div()
                .flex()
                .flex_1()
                .min_h_0()
                .bg(self.shell_surface_color(palette.bg))
                .child(self.settings_view(cx))
                .into_any_element()
        } else {
            let compact_layout = !cfg!(target_os = "macos");
            let has_left_activity_items = self.activity_side_has_items(ActivitySide::Left);
            let has_right_activity_items = self.activity_side_has_items(ActivitySide::Right);
            let left_overlay_mode = compact_layout && self.shell.viewport.size.0 < 1024.;
            let right_overlay_mode = compact_layout && self.shell.viewport.size.0 < 768.;
            let left_drawer_open = has_left_activity_items
                && left_overlay_mode
                && self.shell.panels.mobile_left_open
                && self.left_side_open();
            let right_drawer_open = has_right_activity_items
                && right_overlay_mode
                && self.shell.panels.mobile_right_open
                && self.right_side_open();
            let mut surface = div()
                .flex()
                .flex_1()
                .min_h_0()
                .relative()
                .overflow_hidden()
                .bg(self.shell_transparent_color(palette.bg))
                .when(has_left_activity_items, |this| {
                    this.child(self.activity_bar(ActivitySide::Left, cx))
                })
                .when(
                    has_left_activity_items && self.left_side_open() && !left_overlay_mode,
                    |this| {
                        this.child(self.sidebar(window, cx))
                            .child(self.panel_resize_handle(PanelResizeSide::Left, cx))
                    },
                )
                .child(self.main_surface(cx))
                .when(
                    has_right_activity_items && self.right_side_open() && !right_overlay_mode,
                    |this| {
                        this.child(self.panel_resize_handle(PanelResizeSide::Right, cx))
                            .child(self.right_panel(window, cx))
                    },
                )
                .when(has_right_activity_items, |this| {
                    this.child(self.activity_bar(ActivitySide::Right, cx))
                });

            if self.terminal_windows_is_multi_leaf() && self.shell.chrome.new_session_menu_open {
                surface = surface.child(self.render_new_session_menu(cx));
            }

            if left_drawer_open || right_drawer_open {
                surface = surface.child(
                    div()
                        .id("mobile-panel-backdrop")
                        .absolute()
                        .inset_0()
                        .bg(rgba(0x00000080))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.shell.panels.mobile_left_open = false;
                            this.shell.panels.mobile_right_open = false;
                            cx.notify();
                        })),
                );
            }

            if left_drawer_open {
                surface = surface.child(
                    div()
                        .id("mobile-left-drawer")
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(px(40.))
                        .flex()
                        .shadow_lg()
                        .on_click(|_, _, cx| cx.stop_propagation())
                        .child(
                            div()
                                .h_full()
                                .flex()
                                .flex_col()
                                .child(self.mobile_drawer_header("mobile-left-close", true, cx))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_h_0()
                                        .flex()
                                        .child(self.sidebar(window, cx)),
                                ),
                        ),
                );
            }

            if right_drawer_open {
                surface = surface.child(
                    div()
                        .id("mobile-right-drawer")
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .right(px(40.))
                        .flex()
                        .shadow_lg()
                        .on_click(|_, _, cx| cx.stop_propagation())
                        .child(
                            div()
                                .h_full()
                                .flex()
                                .flex_col()
                                .child(self.mobile_drawer_header("mobile-right-close", false, cx))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_h_0()
                                        .flex()
                                        .child(self.right_panel(window, cx)),
                                ),
                        ),
                );
            }

            surface.into_any_element()
        }
    }

    fn mobile_drawer_header(
        &self,
        id: &'static str,
        left: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        div()
            .h(px(40.))
            .flex_none()
            .flex()
            .items_center()
            .justify_end()
            .px_2()
            .border_b_1()
            .border_color(rgb(palette.border))
            .bg(self.shell_surface_color(palette.surface))
            .child(
                div()
                    .id(id)
                    .size(px(26.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_sm()
                    .text_color(rgb(palette.text_muted))
                    .cursor_pointer()
                    .hover(|this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
                    .child(
                        svg()
                            .size(px(16.))
                            .path("icons/window/close.svg")
                            .text_color(rgb(palette.text_muted)),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if left {
                            this.shell.panels.mobile_left_open = false;
                        } else {
                            this.shell.panels.mobile_right_open = false;
                        }
                        cx.notify();
                    })),
            )
    }

    fn overlay_host(
        &mut self,
        content: Stateful<Div>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let overlay = OverlayFlags {
            tab_actions_open: self.session.dialogs.tab_actions_session_id.is_some(),
            rename_open: self.session.dialogs.rename_session_id.is_some(),
            color_picker_open: self.session.dialogs.color_picker_open,
            session_info_open: self.session.dialogs.session_info_open,
            startup_command_open: self.session.dialogs.startup_command_open,
            temporary_ssh_link_open: self.session.dialogs.temporary_ssh_link_open,
            multi_line_paste_open: self.terminal.paste.draft.is_some(),
            terminal_actions_open: self.terminal.menus.actions_open,
            terminal_context_menu_open: self.terminal.menus.context_menu.is_some(),
            action_link_menu_open: self.terminal.menus.action_link_menu.is_some(),
            action_link_tooltip_open: self.terminal.menus.action_link_tooltip.is_some(),
            command_suggestions_open: self.terminal.assist.command_suggestions.is_some(),
            credential_suggestions_open: self.terminal.assist.credential_suggestions.is_some(),
            close_all_sessions_confirm_open: self.session.dialogs.close_all_sessions_confirm_open,
            locked: self.security.screen_lock.locked,
        };
        let quick_switch_open = self.quick_switch_open(cx);
        let transfer_properties_open = self
            .transfer
            .file_ops
            .properties
            .as_ref()
            .is_some_and(|state| state.session_id.as_deref() == self.session.active_id.as_deref());
        let transfer_editor_open = self.transfer.editor.workspace.is_some()
            && self.remote_editor_window.is_none()
            && !self.remote_editor_window_open_pending;
        let transfer_external_sync_open = self.active_external_editor_sync_prompt().is_some();
        let ssh_auth_prompt_open = self.session.prompts.active_host_key_prompt.is_some()
            || self.session.prompts.active_credential_prompt.is_some()
            || self
                .session
                .prompts
                .active_keyboard_interactive_prompt
                .is_some();

        content
            .when(overlay.tab_actions_open, |this| {
                this.child(self.tab_actions_overlay(cx))
            })
            .when(overlay.rename_open, |this| {
                this.child(self.rename_session_overlay(cx))
            })
            .when(overlay.color_picker_open, |this| {
                this.child(self.tab_color_picker_overlay(cx))
            })
            .when(overlay.session_info_open, |this| {
                this.child(self.session_info_overlay(cx))
            })
            .when(overlay.startup_command_open, |this| {
                this.child(self.startup_command_overlay(cx))
            })
            .when(overlay.temporary_ssh_link_open, |this| {
                this.child(self.temporary_ssh_link_overlay(cx))
            })
            .when(self.transfer.file_ops.move_to.is_some(), |this| {
                this.child(self.transfer_move_overlay(cx))
            })
            .when(self.transfer.file_ops.delete.is_some(), |this| {
                this.child(self.transfer_delete_overlay(cx))
            })
            .when(self.transfer.queue.job_delete.is_some(), |this| {
                this.child(self.transfer_job_delete_overlay(cx))
            })
            .when(self.transfer.queue.job_menu.is_some(), |this| {
                this.child(self.transfer_job_menu_overlay(cx))
            })
            .when(self.transfer.file_ops.new_folder.is_some(), |this| {
                this.child(self.transfer_new_folder_overlay(cx))
            })
            .when(self.transfer.file_ops.new_file.is_some(), |this| {
                this.child(self.transfer_new_file_overlay(cx))
            })
            .when(self.transfer.file_ops.new_symlink.is_some(), |this| {
                this.child(self.transfer_new_symlink_overlay(cx))
            })
            .when(transfer_properties_open, |this| {
                this.child(self.transfer_properties_overlay(cx))
            })
            .when(transfer_editor_open, |this| {
                this.child(self.transfer_editor_overlay(cx))
            })
            .when(self.transfer.file_ops.unknown_file.is_some(), |this| {
                this.child(self.transfer_unknown_file_overlay(cx))
            })
            .when(transfer_external_sync_open, |this| {
                this.child(self.transfer_external_sync_prompt_overlay(cx))
            })
            .when(self.transfer.browser.context_menu.is_some(), |this| {
                this.child(self.transfer_browser_context_menu_overlay(cx))
            })
            .when(self.transfer.browser.favorites_menu.is_some(), |this| {
                this.child(self.transfer_browser_favorites_menu_overlay(cx))
            })
            .when(self.transfer.browser.path_menu.is_some(), |this| {
                this.child(self.transfer_browser_path_menu_overlay(cx))
            })
            .when(self.transfer.browser.upload_menu.is_some(), |this| {
                this.child(self.transfer_browser_upload_menu_overlay(cx))
            })
            .when(overlay.multi_line_paste_open, |this| {
                this.child(self.multi_line_paste_overlay(cx))
            })
            .when(overlay.terminal_actions_open, |this| {
                this.child(self.terminal_actions_overlay(cx))
            })
            .children(self.network_dialog_overlay(cx))
            .when(overlay.terminal_context_menu_open, |this| {
                this.child(self.terminal_context_menu_overlay(cx))
            })
            .when(overlay.action_link_menu_open, |this| {
                this.child(self.action_link_menu_overlay(cx))
            })
            .when(
                overlay.action_link_tooltip_open
                    && !overlay.action_link_menu_open
                    && !overlay.terminal_context_menu_open
                    && self.translation.dialog.is_none(),
                |this| this.child(self.action_link_tooltip_overlay(cx)),
            )
            .when(self.translation.dialog.is_some(), |this| {
                this.child(self.translation_dialog_overlay(cx))
            })
            .when(overlay.command_suggestions_open, |this| {
                this.child(self.command_suggestions_overlay(cx))
            })
            .when(overlay.credential_suggestions_open, |this| {
                this.child(self.credential_suggestions_overlay(cx))
            })
            .when(self.sync_input.open, |this| {
                this.child(self.sync_groups_overlay(cx))
            })
            .when_some(
                self.connection_state.inline_editor_panel_draft(),
                |this, editor| this.child(self.connection_editor_panel(editor, cx)),
            )
            .when(
                self.quick_command_state.editor.draft.is_some()
                    && self.quick_command_state.editor.window.is_none()
                    && !self.quick_command_state.editor.window_open_pending,
                |this| this.child(self.quick_command_editor_overlay(cx)),
            )
            .when(self.quick_command_state.dialogs.delete.is_some(), |this| {
                this.child(self.quick_command_delete_overlay(cx))
            })
            .when(self.quick_command_state.dialogs.details.is_some(), |this| {
                this.child(self.quick_command_details_overlay(cx))
            })
            .when(self.quick_command_state.list.row_menu.is_some(), |this| {
                this.child(self.quick_command_row_menu_overlay(cx))
            })
            .when(
                self.quick_command_state.list.category_menu.is_some(),
                |this| this.child(self.quick_command_category_menu_overlay(cx)),
            )
            .when(self.session.active_menu.is_some(), |this| {
                this.child(self.active_session_menu_overlay(cx))
            })
            .when(
                self.quick_command_state.dialogs.category_delete.is_some(),
                |this| this.child(self.quick_command_category_delete_overlay(cx)),
            )
            .when(
                self.quick_command_state.dialogs.category_rename.is_some(),
                |this| this.child(self.quick_command_category_rename_overlay(cx)),
            )
            .when(
                self.quick_command_state.dialogs.variable_prompt.is_some(),
                |this| this.child(self.quick_command_variable_prompt_overlay(cx)),
            )
            .when(self.quick_command_state.import.dialog_open, |this| {
                this.child(self.quick_command_import_overlay(cx))
            })
            .when(self.connection_state.import_dialog_is_open(), |this| {
                this.child(self.connection_import_overlay(cx))
            })
            .when(quick_switch_open, |this| {
                this.child(self.quick_switch_overlay(cx))
            })
            .when(
                self.shell.chrome.activity_bar_context_menu.is_some(),
                |this| this.child(self.activity_bar_context_menu_overlay(cx)),
            )
            .when(overlay.locked, |this| {
                this.child(self.lock_screen_overlay(window, cx))
            })
            .when(overlay.close_all_sessions_confirm_open, |this| {
                this.child(self.close_all_sessions_confirm_overlay(cx))
            })
            .when(self.about_open, |this| this.child(self.about_overlay(cx)))
            .when(self.update.dialog_open, |this| {
                this.child(self.update_overlay(cx))
            })
            .when(self.modal_child_window_open(), |this| {
                this.child(self.modal_owner_backdrop(cx))
            })
            // Background SSH operations can request credentials while another
            // modal is open, so authentication must be the topmost overlay.
            .when(ssh_auth_prompt_open, |this| {
                this.child(self.ssh_auth_prompt_overlay(cx))
            })
    }

    fn modal_child_window_open(&self) -> bool {
        self.shell.navigation.settings.window.is_some()
            || self.shell.navigation.settings.window_open_pending
            || self.quick_command_state.editor.window.is_some()
            || self.quick_command_state.editor.window_open_pending
            || self.connection_state.editor_modal_window_open_or_pending()
            || !self.transfer.external_sync.windows.is_empty()
            || !self.transfer.external_sync.window_open_pending.is_empty()
    }

    fn activate_modal_child_window(&mut self, cx: &mut Context<Self>) -> bool {
        if self.shell.navigation.settings.window.is_some() {
            self.activate_settings_window(cx)
        } else if self.shell.navigation.settings.window_open_pending {
            true
        } else if self.quick_command_state.editor.window.is_some() {
            self.activate_quick_command_window(cx)
        } else if self.quick_command_state.editor.window_open_pending {
            true
        } else if self.connection_state.editor_has_window() {
            self.activate_connection_editor_window(cx)
        } else if self.connection_state.editor_window_open_pending() {
            true
        } else if !self.transfer.external_sync.windows.is_empty() {
            self.activate_transfer_external_sync_window(cx)
        } else {
            !self.transfer.external_sync.window_open_pending.is_empty()
        }
    }

    fn modal_owner_backdrop(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("modal-owner-backdrop")
            .absolute()
            .inset_0()
            .bg(rgba(0x00000066))
            .cursor_pointer()
            .on_mouse_move(|_, _, cx| cx.stop_propagation())
            .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_up(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .on_mouse_up(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(|this, _, _, cx| {
                cx.stop_propagation();
                this.activate_modal_child_window(cx);
            }))
    }

    fn ssh_auth_prompt_overlay(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let host_key_prompt = self.session.prompts.active_host_key_prompt.clone();
        let credential_prompt = self.session.prompts.active_credential_prompt.clone();
        let keyboard_interactive_prompt = self
            .session
            .prompts
            .active_keyboard_interactive_prompt
            .clone();
        let dialog_width = if keyboard_interactive_prompt.is_some() {
            384.
        } else {
            416.
        };
        div()
            .id(SharedString::from("ssh-auth-prompt-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .p_3()
            .track_focus(&self.session.prompts.credential_focus)
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Middle, |_, _, cx| cx.stop_propagation())
            .on_click(cx.listener(|this, _, window, cx| {
                cx.stop_propagation();
                if !this.focus_active_ssh_prompt_input(window, cx) {
                    window.focus(&this.session.prompts.credential_focus);
                }
                cx.notify();
            }))
            .child(
                div()
                    .mx_auto()
                    .w_full()
                    .max_w(px(dialog_width))
                    .when_some(host_key_prompt, |this, prompt| {
                        this.child(self.host_key_prompt_banner(prompt, cx))
                    })
                    .when_some(credential_prompt, |this, prompt| {
                        this.child(self.credential_prompt_banner(prompt, cx))
                    })
                    .when_some(keyboard_interactive_prompt, |this, prompt| {
                        this.child(self.keyboard_interactive_prompt_banner(prompt, cx))
                    }),
            )
    }
}

fn wallpaper_tile_grid(viewport: (f32, f32), tile: (f32, f32)) -> (usize, usize) {
    let columns = ((viewport.0.max(1.) / tile.0.max(WALLPAPER_TILE_MIN_SIZE)).ceil() as usize)
        .clamp(1, WALLPAPER_TILE_ELEMENT_LIMIT);
    let requested_rows =
        ((viewport.1.max(1.) / tile.1.max(WALLPAPER_TILE_MIN_SIZE)).ceil() as usize).max(1);
    let rows = requested_rows.min((WALLPAPER_TILE_ELEMENT_LIMIT / columns).max(1));
    (columns, rows)
}

fn fit_wallpaper_tile_size(viewport: (f32, f32), intrinsic: (f32, f32)) -> (f32, f32) {
    let mut tile = (
        intrinsic.0.max(WALLPAPER_TILE_MIN_SIZE),
        intrinsic.1.max(WALLPAPER_TILE_MIN_SIZE),
    );
    for _ in 0..4 {
        let columns = (viewport.0.max(1.) / tile.0).ceil().max(1.);
        let rows = (viewport.1.max(1.) / tile.1).ceil().max(1.);
        let count = columns * rows;
        if count <= WALLPAPER_TILE_ELEMENT_LIMIT as f32 {
            break;
        }
        let scale = (count / WALLPAPER_TILE_ELEMENT_LIMIT as f32).sqrt() * 1.01;
        tile.0 *= scale;
        tile.1 *= scale;
    }
    tile
}

#[cfg(test)]
mod tests {
    use super::{fit_wallpaper_tile_size, wallpaper_tile_grid};

    #[test]
    fn wallpaper_tiles_cover_viewport_and_cap_extreme_counts() {
        assert_eq!(wallpaper_tile_grid((1280., 800.), (256., 256.)), (5, 4));
        assert_eq!(wallpaper_tile_grid((1280., 800.), (2048., 2048.)), (1, 1));
        let (columns, rows) = wallpaper_tile_grid((100_000., 100_000.), (1., 1.));
        assert!(columns * rows <= 8192);
        let tile = fit_wallpaper_tile_size((3840., 2160.), (1., 1.));
        let (columns, rows) = wallpaper_tile_grid((3840., 2160.), tile);
        assert!(columns * rows <= 8192);
        assert!(columns as f32 * tile.0 >= 3840.);
        assert!(rows as f32 * tile.1 >= 2160.);
    }
}

impl Render for NyaTermApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let render_started_at = Instant::now();
        FULL_SHELL_PAINT_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.terminal.view.runtime.full_shell_paint_count = self
            .terminal
            .view
            .runtime
            .full_shell_paint_count
            .saturating_add(1);
        let root_started_at = Instant::now();
        let content = self.root_chrome(window, cx);
        let root_duration = root_started_at.elapsed();
        let overlay_started_at = Instant::now();
        let output = self.overlay_host(content, window, cx);
        let overlay_duration = overlay_started_at.elapsed();
        let render_duration = render_started_at.elapsed();
        if render_duration >= Duration::from_millis(12)
            && self.should_log_slow_diagnostic("root_render", Instant::now())
        {
            tracing::warn!(
                diagnostic = "root_render",
                total_ms = render_duration.as_millis(),
                root_chrome_ms = root_duration.as_millis(),
                overlay_host_ms = overlay_duration.as_millis(),
                active_session_id = self.session.active_id.as_deref().unwrap_or(""),
                visible_session_count = self.visible_terminal_session_ids().len(),
                connect_settle_active = self
                    .terminal
                    .view
                    .runtime
                    .connect_settle_until
                    .is_some_and(|until| Instant::now() < until),
                output_pressure = self.runtime_output_pressure_active(),
                full_shell_paint_count = self.terminal.view.runtime.full_shell_paint_count,
                surface_paint_count = terminal_surface_paint_count(),
                "slow root render"
            );
        }
        output
    }
}

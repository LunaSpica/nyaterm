use super::*;
use gpui::{
    AnyElement, Context, Div, ImageSource, IntoElement, KeyDownEvent, MouseButton, MouseMoveEvent,
    MouseUpEvent, NavigationDirection, ObjectFit, Render, SharedString, Stateful, Window, div, img,
    rgb,
};

impl NyaTermApp {
    pub(crate) fn start_after_window_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_window_render_inputs(window, cx);
        self.try_restore_open_tabs(window, cx);
        let should_pump = self.stores.startup_restore.update(cx, |store, _| {
            store.can_pump_queue(self.pending_session_name.is_some())
        });
        if should_pump {
            self.pump_startup_restore_queue(window, cx);
        }

        self.publish_store_snapshots(cx);
    }

    fn root_chrome(&mut self, cx: &mut Context<Self>) -> Stateful<Div> {
        let palette = self.theme_palette();
        let wallpaper_path = self
            .settings
            .background_image_path
            .as_ref()
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .map(|p| p.to_string());
        let wallpaper_enabled = wallpaper_path.is_some();
        let wallpaper_opacity =
            (self.settings.background_image_opacity.clamp(5, 100) as f32) / 100.0;
        let content_opacity = if wallpaper_enabled {
            (self.settings.background_content_opacity.clamp(20, 100) as f32) / 100.0
        } else {
            1.0
        };
        let wallpaper_fit = self.settings.background_image_fit.as_str();
        div()
            .id(SharedString::from("nyaterm-root"))
            .size_full()
            .relative()
            .bg(rgb(palette.bg))
            .text_color(rgb(palette.text))
            .font_family(if self.settings.ui_font_family.trim().is_empty() {
                self.settings.terminal_font_family.clone()
            } else {
                self.settings.ui_font_family.clone()
            })
            .text_size(px(self.settings.ui_font_size.clamp(12, 24) as f32))
            .on_click(cx.listener(|this, _, _, _| {
                this.mark_user_activity();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if this.handle_global_shortcut(event, window, cx) {
                    cx.stop_propagation();
                }
            }))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                this.update_transfer_browser_column_resize(event, cx);
                this.update_panel_resize(event, cx);
                this.update_transfer_height_resize(event, cx);
                this.update_panel_stack_resize(event, cx);
                this.update_workspace_split_resize(event, cx);
                this.update_terminal_selection_drag(event, cx);
                this.update_terminal_scrollbar_drag(event, cx);
                this.update_action_link_hover(event, cx);
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    this.finish_transfer_browser_column_resize(event, cx);
                    this.finish_panel_resize(event, cx);
                    this.finish_transfer_height_resize(event, cx);
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
                    if this.current_left_panel() == Some(NavItem::Transfers) {
                        cx.stop_propagation();
                        this.open_transfer_browser_history(1, window, cx);
                    }
                }),
            )
            .on_mouse_up(
                MouseButton::Navigate(NavigationDirection::Forward),
                cx.listener(|this, _event: &MouseUpEvent, window, cx| {
                    if this.current_left_panel() == Some(NavItem::Transfers) {
                        cx.stop_propagation();
                        this.open_transfer_browser_history(-1, window, cx);
                    }
                }),
            )
            .when_some(wallpaper_path.clone(), |this, path| {
                let source: ImageSource = std::sync::Arc::<std::path::Path>::from(
                    std::path::PathBuf::from(path).into_boxed_path(),
                )
                .into();
                let object_fit = match wallpaper_fit {
                    "contain" => ObjectFit::Contain,
                    "stretch" | "fill" => ObjectFit::Fill,
                    "tile" => ObjectFit::None, // native tile is approximate (no CSS repeat)
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
                    .opacity(content_opacity)
                    .child(self.title_bar(cx))
                    .child(self.workspace_surface(palette, cx))
                    .child(self.status_bar(cx)),
            )
    }

    fn workspace_surface(&mut self, palette: ThemePalette, cx: &mut Context<Self>) -> AnyElement {
        if self.main_mode == MainMode::Page && self.selected_nav == NavItem::Settings {
            div()
                .flex()
                .flex_1()
                .min_h_0()
                .bg(rgb(palette.bg))
                .child(self.settings_view(cx))
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_1()
                .min_h_0()
                .bg(rgb(palette.bg))
                .child(self.activity_bar(ActivitySide::Left, cx))
                .when(self.left_side_open(), |this| {
                    this.child(self.sidebar(cx))
                        .child(self.panel_resize_handle(PanelResizeSide::Left, cx))
                })
                .child(self.main_surface(cx))
                .when(self.right_side_open(), |this| {
                    this.child(self.panel_resize_handle(PanelResizeSide::Right, cx))
                        .child(self.right_panel(cx))
                })
                .child(self.activity_bar(ActivitySide::Right, cx))
                .into_any_element()
        }
    }

    fn overlay_host(&mut self, content: Stateful<Div>, cx: &mut Context<Self>) -> impl IntoElement {
        let overlay = self
            .stores
            .overlays
            .read_with(cx, |store, _| store.snapshot().cloned())
            .unwrap_or_else(|| crate::entities::OverlaySnapshot {
                quick_switch_open: self.quick_switch_open,
                tab_actions_open: self.tab_actions_session_id.is_some(),
                rename_open: self.rename_session_id.is_some(),
                color_picker_open: self.color_picker_open,
                session_info_open: self.session_info_open,
                startup_command_open: self.startup_command_open,
                temporary_ssh_link_open: self.temporary_ssh_link_open,
                multi_line_paste_open: self.multi_line_paste.is_some(),
                terminal_actions_open: self.terminal_actions_open,
                terminal_context_menu_open: self.terminal_context_menu.is_some(),
                action_link_menu_open: self.action_link_menu.is_some(),
                action_link_tooltip_open: self.action_link_tooltip.is_some(),
                command_suggestions_open: self.command_suggestions.is_some(),
                credential_suggestions_open: self.credential_suggestions.is_some(),
                close_all_sessions_confirm_open: self.close_all_sessions_confirm_open,
                locked: self.is_locked,
            });

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
            .when(self.transfer_move.is_some(), |this| {
                this.child(self.transfer_move_overlay(cx))
            })
            .when(self.transfer_delete.is_some(), |this| {
                this.child(self.transfer_delete_overlay(cx))
            })
            .when(self.transfer_job_delete.is_some(), |this| {
                this.child(self.transfer_job_delete_overlay(cx))
            })
            .when(self.transfer_new_folder.is_some(), |this| {
                this.child(self.transfer_new_folder_overlay(cx))
            })
            .when(self.transfer_new_file.is_some(), |this| {
                this.child(self.transfer_new_file_overlay(cx))
            })
            .when(self.transfer_new_symlink.is_some(), |this| {
                this.child(self.transfer_new_symlink_overlay(cx))
            })
            .when(self.transfer_properties.is_some(), |this| {
                this.child(self.transfer_properties_overlay(cx))
            })
            .when(self.transfer_editor.is_some(), |this| {
                this.child(self.transfer_editor_overlay(cx))
            })
            .when(self.transfer_unknown_file.is_some(), |this| {
                this.child(self.transfer_unknown_file_overlay(cx))
            })
            .when(self.transfer_external_sync_prompt.is_some(), |this| {
                this.child(self.transfer_external_sync_prompt_overlay(cx))
            })
            .when(self.transfer_browser_context_menu.is_some(), |this| {
                this.child(self.transfer_browser_context_menu_overlay(cx))
            })
            .when(self.transfer_browser_favorites_menu.is_some(), |this| {
                this.child(self.transfer_browser_favorites_menu_overlay(cx))
            })
            .when(self.transfer_browser_upload_menu.is_some(), |this| {
                this.child(self.transfer_browser_upload_menu_overlay(cx))
            })
            .when(overlay.multi_line_paste_open, |this| {
                this.child(self.multi_line_paste_overlay(cx))
            })
            .when(overlay.terminal_actions_open, |this| {
                this.child(self.terminal_actions_overlay(cx))
            })
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
                    && self.translation_dialog.is_none(),
                |this| this.child(self.action_link_tooltip_overlay(cx)),
            )
            .when(self.translation_dialog.is_some(), |this| {
                this.child(self.translation_dialog_overlay(cx))
            })
            .when(overlay.command_suggestions_open, |this| {
                this.child(self.command_suggestions_overlay(cx))
            })
            .when(overlay.credential_suggestions_open, |this| {
                this.child(self.credential_suggestions_overlay(cx))
            })
            .when(self.sync_groups_open, |this| {
                this.child(self.sync_groups_overlay(cx))
            })
            .when(self.quick_command_editor.is_some(), |this| {
                this.child(self.quick_command_editor_overlay(cx))
            })
            .when(self.quick_command_delete.is_some(), |this| {
                this.child(self.quick_command_delete_overlay(cx))
            })
            .when(self.quick_command_details.is_some(), |this| {
                this.child(self.quick_command_details_overlay(cx))
            })
            .when(self.quick_command_category_delete.is_some(), |this| {
                this.child(self.quick_command_category_delete_overlay(cx))
            })
            .when(self.quick_command_category_rename.is_some(), |this| {
                this.child(self.quick_command_category_rename_overlay(cx))
            })
            .when(self.quick_command_variable_prompt.is_some(), |this| {
                this.child(self.quick_command_variable_prompt_overlay(cx))
            })
            .when(self.quick_command_import_dialog_open, |this| {
                this.child(self.quick_command_import_overlay(cx))
            })
            .when(overlay.close_all_sessions_confirm_open, |this| {
                this.child(self.close_all_sessions_confirm_overlay(cx))
            })
            .when(overlay.quick_switch_open, |this| {
                this.child(self.quick_switch_overlay(cx))
            })
            .when(self.activity_bar_context_menu.is_some(), |this| {
                this.child(self.activity_bar_context_menu_overlay(cx))
            })
            .when(overlay.locked, |this| {
                this.child(self.lock_screen_overlay(cx))
            })
    }
}

impl Render for NyaTermApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = self.root_chrome(cx);
        self.overlay_host(content, cx)
    }
}

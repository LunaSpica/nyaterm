use super::*;
use gpui::{
    Context, IntoElement, KeyDownEvent, MouseButton, MouseMoveEvent, MouseUpEvent,
    NavigationDirection, Render, SharedString, Window, div, rgb,
};

impl Render for NyaTermApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_event_pump(window, cx);
        if self.ai_chat_focus_pending {
            window.focus(&self.ai_chat_focus);
            self.ai_chat_focus_pending = false;
        }
        if self.transfer_rename_focus_pending && self.transfer_rename.is_some() {
            window.focus(&self.transfer_rename_focus);
            self.transfer_rename_focus_pending = false;
        }
        let content = div()
            .id(SharedString::from("nyaterm-root"))
            .size_full()
            .relative()
            .bg(rgb(0x0d1117))
            .text_color(rgb(0xc9d1d9))
            .font_family("JetBrains Mono")
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
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, event: &MouseUpEvent, _, cx| {
                    this.finish_transfer_browser_column_resize(event, cx);
                    this.finish_panel_resize(event, cx);
                    this.finish_transfer_height_resize(event, cx);
                    this.finish_panel_stack_resize(event, cx);
                    this.finish_workspace_split_resize(event, cx);
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
            .child(
                div()
                    .flex()
                    .flex_col()
                    .size_full()
                    .child(self.title_bar(cx))
                    .child(
                        if self.main_mode == MainMode::Page
                            && self.selected_nav == NavItem::Settings
                        {
                            div()
                                .flex()
                                .flex_1()
                                .min_h_0()
                                .bg(rgb(0x0d1117))
                                .child(self.settings_view(cx))
                                .into_any_element()
                        } else {
                            div()
                                .flex()
                                .flex_1()
                                .min_h_0()
                                .bg(rgb(0x0d1117))
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
                        },
                    )
                    .child(self.status_bar(cx)),
            );
        content
            .when(self.tab_actions_session_id.is_some(), |this| {
                this.child(self.tab_actions_overlay(cx))
            })
            .when(self.rename_session_id.is_some(), |this| {
                this.child(self.rename_session_overlay(cx))
            })
            .when(self.color_picker_open, |this| {
                this.child(self.tab_color_picker_overlay(cx))
            })
            .when(self.session_info_open, |this| {
                this.child(self.session_info_overlay(cx))
            })
            .when(self.startup_command_open, |this| {
                this.child(self.startup_command_overlay(cx))
            })
            .when(self.temporary_ssh_link_open, |this| {
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
            .when(self.multi_line_paste.is_some(), |this| {
                this.child(self.multi_line_paste_overlay(cx))
            })
            .when(self.terminal_actions_open, |this| {
                this.child(self.terminal_actions_overlay(cx))
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
            .when(self.close_all_sessions_confirm_open, |this| {
                this.child(self.close_all_sessions_confirm_overlay(cx))
            })
            .when(self.quick_switch_open, |this| {
                this.child(self.quick_switch_overlay(cx))
            })
            .when(self.activity_bar_context_menu.is_some(), |this| {
                this.child(self.activity_bar_context_menu_overlay(cx))
            })
            .when(self.is_locked, |this| {
                this.child(self.lock_screen_overlay(cx))
            })
    }
}

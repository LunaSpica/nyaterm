use gpui::{Context, IntoElement, MouseButton, SharedString, div, prelude::*, px, rgb};

use crate::features::NyaTermApp;

impl NyaTermApp {
    pub(in crate::features) fn quick_command_row_menu_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let Some(menu) = self.commands.quick.list.row_menu.clone() else {
            return div().into_any_element();
        };
        let command_id = menu.command_id.clone();
        let edit_command_id = command_id.clone();
        let all_command_id = command_id.clone();
        let delete_command_id = command_id.clone();
        let can_send_to_all = self.live_session_count() > 1;
        let (viewport_w, viewport_h) = self.shell.viewport.size;
        let (menu_x, menu_y) = quick_command_clamped_menu_position(
            f32::from(menu.x),
            f32::from(menu.y),
            148.,
            if can_send_to_all { 124. } else { 92. },
            viewport_w,
            viewport_h,
        );

        div()
            .id(SharedString::from("quick-command-row-menu-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .on_click(cx.listener(|this, _, _, cx| {
                this.commands.quick.list.row_menu = None;
                cx.notify();
            }))
            .child(
                div()
                    .id(SharedString::from(format!(
                        "quick-command-row-menu-{command_id}"
                    )))
                    .absolute()
                    .top(px(menu_y))
                    .left(px(menu_x))
                    .w(px(148.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.surface))
                    .shadow_lg()
                    .py_1()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .on_click(|_, _, cx| {
                        cx.stop_propagation();
                    })
                    .child(quick_command_row_menu_item(
                        palette,
                        format!("quick-command-row-menu-edit-{command_id}"),
                        self.tr("quickCommands.edit"),
                        false,
                        cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.commands.quick.list.row_menu = None;
                            this.open_edit_quick_command_editor(
                                edit_command_id.clone(),
                                window,
                                cx,
                            );
                        }),
                    ))
                    .when(can_send_to_all, |this| {
                        this.child(quick_command_row_menu_item(
                            palette,
                            format!("quick-command-row-menu-all-{command_id}"),
                            self.tr("quickCommands.sendToAll"),
                            false,
                            cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.commands.quick.list.row_menu = None;
                                this.send_quick_command_to_all_by_id(all_command_id.clone(), cx);
                            }),
                        ))
                    })
                    .child(div().mx_2().my_1().h(px(1.)).bg(rgb(palette.border)))
                    .child(quick_command_row_menu_item(
                        palette,
                        format!("quick-command-row-menu-delete-{command_id}"),
                        self.tr("common.delete"),
                        true,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.commands.quick.list.row_menu = None;
                            this.open_delete_quick_command_confirm(delete_command_id.clone(), cx);
                        }),
                    )),
            )
            .into_any_element()
    }
}

fn quick_command_row_menu_item(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: impl Into<SharedString>,
    destructive: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .px_3()
        .h(px(30.))
        .flex()
        .items_center()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface_elevated)))
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(if destructive {
                    palette.danger
                } else {
                    palette.text
                }))
                .child(label.into()),
        )
        .on_click(on_click)
}

fn quick_command_clamped_menu_position(
    x: f32,
    y: f32,
    menu_w: f32,
    menu_h: f32,
    viewport_w: f32,
    viewport_h: f32,
) -> (f32, f32) {
    let margin = 8.0;
    let max_x = (viewport_w - menu_w - margin).max(margin);
    let max_y = (viewport_h - menu_h - margin).max(margin);
    (x.clamp(margin, max_x), y.clamp(margin, max_y))
}

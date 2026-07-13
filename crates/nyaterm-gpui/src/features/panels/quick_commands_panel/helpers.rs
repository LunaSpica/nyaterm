use super::*;

pub(super) fn quick_command_row_actions(
    palette: crate::theme::ThemePalette,
    command_id: &str,
    show_badge: bool,
    execution_mode: &str,
    menu_open: bool,
    can_send_to_all: bool,
    on_run: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_details: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_more: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_edit: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_send_all: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_delete: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    // Tauri renderCommandActions: optional badge + Send + Details + More menu.
    div()
        .flex()
        .items_center()
        .gap_1()
        .flex_none()
        .when(show_badge, |this| {
            this.child(status_pill(
                if execution_mode == "append" {
                    "append"
                } else {
                    "exec"
                },
                if execution_mode == "append" {
                    rgb(palette.warning)
                } else {
                    rgb(palette.success)
                },
                if execution_mode == "append" {
                    rgb(0x32280f)
                } else {
                    rgb(palette.hover)
                },
            ))
        })
        .child(icon_button(
            format!("quick-command-run-{command_id}"),
            "▶",
            palette,
            on_run,
        ))
        .child(icon_button(
            format!("quick-command-detail-{command_id}"),
            "ⓘ",
            palette,
            on_details,
        ))
        .child(quick_command_more_menu(
            palette,
            command_id,
            menu_open,
            can_send_to_all,
            on_more,
            on_edit,
            on_send_all,
            on_delete,
        ))
}

pub(super) fn quick_command_more_menu(
    palette: crate::theme::ThemePalette,
    command_id: &str,
    menu_open: bool,
    can_send_to_all: bool,
    on_more: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_edit: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_send_all: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_delete: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .relative()
        .child(
            div()
                .id(SharedString::from(format!(
                    "quick-command-more-{command_id}"
                )))
                .size(px(26.))
                .flex()
                .items_center()
                .justify_center()
                .rounded_md()
                .text_color(rgb(palette.text_muted))
                .cursor_pointer()
                .hover(|this| {
                    this.bg(rgb(palette.surface_elevated))
                        .text_color(rgb(palette.text))
                })
                .child(
                    svg()
                        .size(px(14.))
                        .flex_none()
                        .path("icons/session/more.svg"),
                )
                .on_click(on_more),
        )
        .when(menu_open, move |this| {
            this.child(
                div()
                    .id(SharedString::from(format!(
                        "quick-command-more-menu-{command_id}"
                    )))
                    .absolute()
                    .top(px(28.))
                    .right(px(0.))
                    .w(px(148.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface))
                    .shadow_lg()
                    .py_1()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .child(quick_command_menu_item(
                        palette,
                        format!("quick-command-menu-edit-{command_id}"),
                        "Edit",
                        false,
                        on_edit,
                    ))
                    .when(can_send_to_all, |this| {
                        this.child(quick_command_menu_item(
                            palette,
                            format!("quick-command-menu-all-{command_id}"),
                            "Send to all",
                            false,
                            on_send_all,
                        ))
                    })
                    .child(div().mx_2().my_1().h(px(1.)).bg(rgb(palette.border)))
                    .child(quick_command_menu_item(
                        palette,
                        format!("quick-command-menu-delete-{command_id}"),
                        "Delete",
                        true,
                        on_delete,
                    )),
            )
        })
}

pub(super) fn quick_command_menu_item(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: impl Into<SharedString>,
    destructive: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let label = label.into();
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
                .child(label),
        )
        .on_click(on_click)
}

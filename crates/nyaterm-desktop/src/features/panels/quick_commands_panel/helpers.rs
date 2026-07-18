use super::*;

pub(super) fn quick_command_row_actions(
    palette: crate::theme::ThemePalette,
    command_id: &str,
    show_badge: bool,
    execution_mode: &str,
    menu_open: bool,
    on_run: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_details: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    on_more: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
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
        .child(quick_command_action_icon_button(
            palette,
            format!("quick-command-run-{command_id}"),
            "icons/send.svg",
            on_run,
        ))
        .child(quick_command_action_icon_button(
            palette,
            format!("quick-command-detail-{command_id}"),
            "icons/eye.svg",
            on_details,
        ))
        .child(quick_command_more_menu(
            palette, command_id, menu_open, on_more,
        ))
}

fn quick_command_action_icon_button(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    icon_path: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
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
        .child(svg().size(px(14.)).flex_none().path(icon_path))
        .on_click(on_click)
}

pub(super) fn quick_command_more_menu(
    palette: crate::theme::ThemePalette,
    command_id: &str,
    menu_open: bool,
    on_more: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div().relative().child(
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
            .when(menu_open, |this| {
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
}

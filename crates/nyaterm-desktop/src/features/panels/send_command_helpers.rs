use gpui::{IntoElement, SharedString, div, prelude::*, px, rgb};

pub(super) fn send_command_control_group(
    palette: crate::theme::ThemePalette,
    label: &'static str,
    content: impl IntoElement,
) -> impl IntoElement {
    // Tauri labeled control: h-8 bordered group with muted label prefix.
    div()
        .relative()
        .h(px(32.))
        .min_w(px(136.))
        .flex()
        .items_center()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.bg))
        .child(
            div()
                .flex_none()
                .px_2()
                .text_size(px(10.))
                .text_color(rgb(palette.text_dimmed))
                .child(label),
        )
        .child(
            div()
                .h_full()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .border_l_1()
                .border_color(rgb(palette.border))
                .child(content),
        )
}

pub(super) fn send_command_stepper_button(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    disabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(12.))
        .text_color(rgb(if disabled {
            palette.text_dimmed
        } else {
            palette.text_muted
        }))
        .opacity(if disabled { 0.55 } else { 1.0 })
        .when(!disabled, |this| this.cursor_pointer())
        .when(!disabled, |this| {
            this.hover(|this| {
                this.bg(rgb(palette.surface_elevated))
                    .text_color(rgb(palette.text))
            })
        })
        .child(label)
        .when(!disabled, |this| this.on_click(on_click))
}

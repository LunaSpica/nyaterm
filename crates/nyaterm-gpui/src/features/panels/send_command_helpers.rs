use super::*;


pub(super) fn send_command_control_group(
    palette: crate::theme::ThemePalette,
    label: &'static str,
    content: impl IntoElement,
) -> impl IntoElement {
    // Tauri labeled control: h-8 bordered group with muted label prefix.
    div()
        .h(px(30.))
        .flex()
        .items_center()
        .overflow_hidden()
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

pub(super) fn send_command_chip(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_2()
        .flex()
        .items_center()
        .text_size(px(11.))
        .font_weight(if active {
            FontWeight(600.)
        } else {
            FontWeight(500.)
        })
        .text_color(if active {
            rgb(palette.accent)
        } else {
            rgb(palette.text_muted)
        })
        .bg(if active {
            rgb(palette.hover)
        } else {
            rgb(0x00000000)
        })
        .cursor_pointer()
        .hover(|this| {
            this.bg(rgb(palette.surface_elevated))
                .text_color(rgb(palette.text))
        })
        .child(label)
        .on_click(on_click)
}

pub(super) fn send_command_target_chip(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: impl Into<SharedString>,
    active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_2()
        .flex()
        .items_center()
        .text_size(px(11.))
        .font_weight(if active {
            FontWeight(600.)
        } else {
            FontWeight(500.)
        })
        .text_color(if active {
            rgb(palette.accent)
        } else {
            rgb(palette.text_muted)
        })
        .bg(if active {
            rgb(palette.hover)
        } else {
            rgb(0x00000000)
        })
        .cursor_pointer()
        .hover(|this| {
            this.bg(rgb(palette.surface_elevated))
                .text_color(rgb(palette.text))
        })
        .child(label.into())
        .on_click(on_click)
}

pub(super) fn send_command_stepper_button(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(12.))
        .text_color(rgb(palette.text_muted))
        .cursor_pointer()
        .hover(|this| {
            this.bg(rgb(palette.surface_elevated))
                .text_color(rgb(palette.text))
        })
        .child(label)
        .on_click(on_click)
}

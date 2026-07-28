use gpui::{FontWeight, IntoElement, SharedString, div, prelude::*, px, rgb, svg};

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

pub(super) fn send_command_select_trigger(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    value: impl Into<SharedString>,
    open: bool,
    disabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(SharedString::from(id.into()))
        .relative()
        .h_full()
        .min_w(px(78.))
        .flex_1()
        .px_2()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .text_size(px(11.))
        .font_weight(FontWeight(500.))
        .text_color(if disabled {
            rgb(palette.text_dimmed)
        } else {
            rgb(palette.text)
        })
        .opacity(if disabled { 0.58 } else { 1.0 })
        .when(!disabled, |this| this.cursor_pointer())
        .when(!disabled, |this| {
            this.hover(|this| {
                this.bg(rgb(palette.surface_elevated))
                    .text_color(rgb(palette.text))
            })
        })
        .child(
            div()
                .min_w_0()
                .flex_1()
                .overflow_hidden()
                .child(value.into()),
        )
        .child(
            div()
                .flex_none()
                .text_color(if open {
                    rgb(palette.link)
                } else {
                    rgb(palette.text_dimmed)
                })
                .child(
                    svg()
                        .size(px(13.))
                        .path("icons/chevron-down.svg")
                        .text_color(if open {
                            rgb(palette.link)
                        } else {
                            rgb(palette.text_dimmed)
                        }),
                ),
        )
        .when(!disabled, |this| this.on_click(on_click))
}

pub(super) fn send_command_select_menu(
    palette: crate::theme::ThemePalette,
    menu_background: gpui::Rgba,
    id: impl Into<String>,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(SharedString::from(id.into()))
        .absolute()
        .top(px(34.))
        .right_0()
        .min_w(px(156.))
        .max_h(px(180.))
        .overflow_scroll()
        .scrollbar_width(px(6.))
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(menu_background)
        .shadow_lg()
        .py_1()
        .flex()
        .flex_col()
}

pub(super) fn send_command_select_menu_item(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: impl Into<SharedString>,
    active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .text_size(px(11.))
        .font_weight(if active {
            FontWeight(600.)
        } else {
            FontWeight(500.)
        })
        .text_color(if active {
            rgb(palette.link)
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
        .child(
            div()
                .min_w_0()
                .flex_1()
                .overflow_hidden()
                .child(label.into()),
        )
        .on_click(on_click)
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

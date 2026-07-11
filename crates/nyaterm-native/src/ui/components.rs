use gpui::{
    App, ClickEvent, FontWeight, Hsla, IntoElement, SharedString, Window, div, prelude::*, px, rgb,
};

pub(super) fn status_pill(
    label: &'static str,
    fg: impl Into<Hsla>,
    bg: impl Into<Hsla>,
) -> impl IntoElement {
    div()
        .rounded_sm()
        .px_2()
        .py_1()
        .text_xs()
        .text_color(fg.into())
        .bg(bg.into())
        .child(label)
}

pub(super) fn empty_panel(text: &'static str) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x30363d))
        .bg(rgb(0x0d1117))
        .p_4()
        .text_sm()
        .text_color(rgb(0x8b949e))
        .child(text)
}

pub(super) fn section_header(title: &'static str, detail: &'static str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_2xl().font_weight(FontWeight(800.)).child(title))
        .child(div().text_sm().text_color(rgb(0x8b949e)).child(detail))
}

pub(super) fn capability_line(
    label: &'static str,
    value: impl Into<SharedString>,
) -> impl IntoElement {
    div()
        .mt_2()
        .flex()
        .items_center()
        .justify_between()
        .text_sm()
        .child(div().text_color(rgb(0xcbd5e1)).child(label))
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x8b949e))
                .child(value.into()),
        )
}

pub(super) fn session_info_row(label: &'static str, value: String) -> impl IntoElement {
    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x263142))
        .bg(rgb(0x0d1320))
        .px_3()
        .py_2()
        .flex()
        .items_start()
        .gap_3()
        .child(
            div()
                .w(px(104.))
                .flex_none()
                .text_xs()
                .font_weight(FontWeight(700.))
                .text_color(rgb(0x8f98aa))
                .child(label),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .font_family("JetBrains Mono")
                .text_xs()
                .text_color(rgb(0xdbeafe))
                .child(value),
        )
}

pub(super) fn small_button(
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x303848))
        .bg(rgb(0x151b27))
        .text_color(rgb(0xdbeafe))
        .text_xs()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x223047)))
        .child(label)
        .on_click(on_click)
}

pub(super) fn mode_button(
    id: impl Into<String>,
    label: &'static str,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    // Tauri AI mode switch: compact segment, primary when active.
    div()
        .id(SharedString::from(id.into()))
        .h(px(26.))
        .px_3()
        .flex()
        .items_center()
        .rounded_md()
        .bg(if active { rgb(0x122033) } else { rgb(0x0d1117) })
        .text_color(if active { rgb(0x58a6ff) } else { rgb(0x8b949e) })
        .text_size(px(11.))
        .font_weight(if active { FontWeight(600.) } else { FontWeight(500.) })
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x21262d)).text_color(rgb(0xc9d1d9)))
        .child(label)
        .on_click(on_click)
}

pub(super) fn icon_button(
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    // Tauri ghost icon-sm buttons: no hard border until hover.
    div()
        .id(SharedString::from(id.into()))
        .size(px(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(rgb(0x8b949e))
        .text_xs()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x21262d)).text_color(rgb(0xc9d1d9)))
        .child(label)
        .on_click(on_click)
}

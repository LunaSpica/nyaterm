use super::*;

pub(super) fn disabled_inspector_panel(
    palette: crate::theme::ThemePalette,
    title: &'static str,
    detail: &'static str,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            inspector_card(palette, title)
                .child(
                    div()
                        .mt_3()
                        .text_xs()
                        .line_height(px(18.))
                        .text_color(rgb(palette.text_muted))
                        .child(detail),
                )
                .child(
                    div()
                        .mt_3()
                        .text_size(px(10.))
                        .text_color(rgb(palette.text_dimmed))
                        .child("The page remains available, but background refresh and actions stay paused while disabled."),
                ),
        )
}

pub(super) fn ai_svg_icon_button(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    icon_path: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl gpui::IntoElement {
    use gpui::{SharedString, div, prelude::*, px, rgb, svg};
    div()
        .id(SharedString::from(id.into()))
        .size(px(28.))
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
        .child(svg().size(px(16.)).flex_none().path(icon_path))
        .on_click(on_click)

}

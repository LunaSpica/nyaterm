use super::*;

pub(in crate::features) fn disabled_inspector_panel(
    palette: crate::theme::ThemePalette,
    detail: &'static str,
) -> impl IntoElement {
    div()
        .size_full()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .px_4()
        .text_center()
        .child(
            div()
                .text_sm()
                .line_height(px(20.))
                .text_color(rgb(palette.text_muted))
                .child(detail),
        )
}

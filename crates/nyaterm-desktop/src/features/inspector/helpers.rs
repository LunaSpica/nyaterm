use super::*;

pub(in crate::features) fn disabled_inspector_panel(
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

use super::*;

pub(in crate::features::pages::remote) fn dense_capability_line(
    palette: crate::theme::ThemePalette,
    label: &'static str,
    value: impl Into<String>,
) -> impl IntoElement {
    // Compact key/value for Resource Monitor cards (Tauri denser than workspace cards).
    div()
        .mt(px(4.))
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(palette.text_dimmed))
                .child(label),
        )
        .child(
            div()
                .min_w_0()
                .font_family("JetBrains Mono")
                .text_size(px(10.))
                .text_color(rgb(palette.text))
                .overflow_hidden()
                .child(value.into()),
        )
}

pub(in crate::features::pages::remote) fn resource_gauge_card(
    palette: ThemePalette,
    title: &'static str,
    value: String,
    detail: String,
    ratio: f64,
) -> impl IntoElement {
    // Tauri ResourceMonitor ring-ish card: compact height, dense mono value.
    let ratio = ratio.clamp(0., 1.);
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.bg))
        .px_2()
        .py_2()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .size(px(44.))
                .rounded_full()
                .border_1()
                .border_color(usage_color(palette, ratio))
                .bg(rgb(palette.surface))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .font_family("JetBrains Mono")
                        .text_size(px(11.))
                        .font_weight(FontWeight(700.))
                        .text_color(usage_color(palette, ratio))
                        .child(value),
                ),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight(600.))
                        .text_color(rgb(palette.text))
                        .child(title),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .line_height(px(13.))
                        .text_color(rgb(palette.text_dimmed))
                        .child(truncate_preview(&detail, 48)),
                )
                .child(stats_progress_bar(palette, ratio)),
        )
}

pub(in crate::features::pages::remote) fn resource_summary_card(
    palette: ThemePalette,
    title: &'static str,
    value: String,
    detail: String,
    ratio: f64,
) -> impl IntoElement {
    let ratio = ratio.clamp(0., 1.);
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.bg))
        .px_2()
        .py_2()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_size(px(10.))
                .font_weight(FontWeight(700.))
                .text_color(rgb(palette.text_dimmed))
                .child(title),
        )
        .child(
            div()
                .font_family("JetBrains Mono")
                .text_size(px(12.))
                .font_weight(FontWeight(700.))
                .text_color(usage_color(palette, ratio))
                .child(value),
        )
        .child(
            div()
                .text_size(px(10.))
                .line_height(px(13.))
                .text_color(rgb(palette.text_muted))
                .child(truncate_preview(&detail, 56)),
        )
        .child(stats_progress_bar(palette, ratio))
}

pub(in crate::features::pages::remote) fn usage_color(palette: ThemePalette, ratio: f64) -> gpui::Hsla {
    if ratio >= 0.9 {
        rgb(0xfb7185).into()
    } else if ratio >= 0.7 {
        rgb(palette.warning).into()
    } else {
        rgb(0x38bdf8).into()
    }
}

pub(in crate::features::pages::remote) fn load_ratio(load1: f64, cores: u32) -> f64 {
    let cores = cores.max(1) as f64;
    (load1 / cores).clamp(0., 1.)
}

pub(in crate::features::pages::remote) fn compact_remote_svg_button(
    palette: ThemePalette,
    id: impl Into<String>,
    icon_path: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(gpui::SharedString::from(id.into()))
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

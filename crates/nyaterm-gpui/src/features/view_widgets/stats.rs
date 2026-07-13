use super::*;

pub(in crate::features) fn stats_resource_row(
    palette: ThemePalette,
    label: &str,
    detail: &str,
    ratio: f64,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_3()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .min_w_0()
                        .text_sm()
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(palette.text))
                        .child(truncate_preview(label, 36)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette.text_muted))
                        .child(format!("{:.0}%", ratio.clamp(0., 1.) * 100.)),
                ),
        )
        .child(
            div()
                .mt_1()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child(truncate_preview(detail, 96)),
        )
        .child(stats_progress_bar(palette, ratio))
}

pub(in crate::features) fn stats_progress_bar(
    palette: ThemePalette,
    ratio: f64,
) -> impl IntoElement {
    let ratio = ratio.clamp(0., 1.);
    div()
        .mt_3()
        .h(px(6.))
        .w_full()
        .overflow_hidden()
        .rounded_sm()
        .bg(rgb(palette.border))
        .child(
            div()
                .h(px(6.))
                .w(px(220. * ratio as f32))
                .rounded_sm()
                .bg(if ratio >= 0.9 {
                    rgb(0xfb7185)
                } else if ratio >= 0.75 {
                    rgb(palette.warning)
                } else {
                    rgb(0x38bdf8)
                }),
        )
}

pub(in crate::features) fn service_status(
    palette: ThemePalette,
    status: NativeServiceStatus,
) -> impl IntoElement {
    match status {
        NativeServiceStatus::Ready => {
            status_pill("ready", rgb(palette.success), rgb(palette.hover)).into_any_element()
        }
        NativeServiceStatus::Porting => {
            status_pill("porting", rgb(palette.warning), rgb(palette.hover)).into_any_element()
        }
        NativeServiceStatus::Blocked => {
            status_pill("replace", rgb(palette.danger), rgb(0x3a1717)).into_any_element()
        }
    }
}

pub(in crate::features) fn metric(
    palette: ThemePalette,
    label: &'static str,
    value: impl Into<SharedString>,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.surface))
        .p_4()
        .child(
            div()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child(label),
        )
        .child(
            div()
                .mt_2()
                .text_2xl()
                .font_weight(FontWeight(800.))
                .text_color(rgb(palette.text))
                .child(value.into()),
        )
}

pub(in crate::features) fn setting_state(
    palette: ThemePalette,
    label: &'static str,
    value: &'static str,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.surface))
        .p_4()
        .child(
            div()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child(label),
        )
        .child(
            div()
                .mt_2()
                .text_lg()
                .font_weight(FontWeight(700.))
                .child(value),
        )
}

pub(in crate::features) fn compact_setting_state(
    palette: ThemePalette,
    label: &'static str,
    value: String,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_3()
        .child(
            div()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child(label),
        )
        .child(
            div()
                .mt_1()
                .text_sm()
                .font_weight(FontWeight(700.))
                .child(value),
        )
}

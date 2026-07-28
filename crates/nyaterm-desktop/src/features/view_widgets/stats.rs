use gpui::{FontWeight, IntoElement, SharedString, div, prelude::*, px, rgb};
use nyaterm_core::NativeServiceStatus;

use crate::theme::ThemePalette;
use crate::widgets::status_pill;

pub(in crate::features) fn stats_progress_bar(
    palette: ThemePalette,
    ratio: f64,
) -> impl IntoElement {
    let ratio = ratio.clamp(0., 1.) as f32;
    div()
        .h(px(5.))
        .w_full()
        .overflow_hidden()
        .rounded_sm()
        .bg(rgb(palette.border))
        .child(
            div()
                .h(px(5.))
                .w(gpui::relative(ratio))
                .rounded_sm()
                .bg(if ratio >= 0.9_f32 {
                    rgb(0xfb7185)
                } else if ratio >= 0.7_f32 {
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

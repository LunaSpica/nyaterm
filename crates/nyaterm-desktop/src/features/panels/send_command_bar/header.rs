use super::state::SendCommandBarViewState;
use super::*;

impl NyaTermApp {
    pub(super) fn send_command_bar_header(
        &mut self,
        state: &SendCommandBarViewState,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let palette = state.palette;
        let target_kind = state.target_kind;
        let target_available = state.target_available;
        let target_scope_label = state.target_scope_label.clone();
        div()
            .h(px(28.))
            .flex_none()
            .px_2()
            .border_b_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.section_header))
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_size(px(11.))
                    .font_weight(FontWeight(600.))
                    .text_color(rgb(palette.text))
                    .child("Command Send"),
            )
            .child(
                div()
                    .ml_auto()
                    .text_size(px(10.))
                    .text_color(rgb(palette.text_dimmed))
                    .child(target_kind),
            )
            .child(
                div()
                    .min_w_0()
                    .max_w(px(160.))
                    .font_family("JetBrains Mono")
                    .text_size(px(10.))
                    .text_color(if target_available {
                        rgb(palette.text_muted)
                    } else {
                        rgb(palette.danger)
                    })
                    .overflow_hidden()
                    .child(truncate_preview(&target_scope_label, 28)),
            )
            .child(small_button(
                palette,
                "bottom-command-send-hide",
                "Hide",
                cx.listener(|this, _, _, cx| {
                    this.bottom_panel = BottomPanelMode::Hidden;
                    cx.notify();
                }),
            ))
            .into_any_element()
    }
}

use super::*;

impl NyaTermApp {
    pub(in crate::features) fn open_about(&mut self, cx: &mut Context<Self>) {
        self.about_open = true;
        cx.notify();
    }

    pub(in crate::features) fn close_about(&mut self, cx: &mut Context<Self>) {
        self.about_open = false;
        cx.notify();
    }

    pub(in crate::features) fn about_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        div()
            .id("about-overlay")
            .absolute()
            .inset_0()
            .bg(rgba(0x030508d8))
            .flex()
            .items_center()
            .justify_center()
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_about(cx);
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key.as_str() == "escape" {
                    cx.stop_propagation();
                    this.close_about(cx);
                }
            }))
            .child(
                div()
                    .id("about-dialog")
                    .w(px(320.))
                    .max_w_full()
                    .mx_4()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface))
                    .shadow_lg()
                    .p_6()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_4()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        svg()
                            .size(px(96.))
                            .path("icons/logo.svg")
                            .text_color(rgb(palette.link)),
                    )
                    .child(
                        div()
                            .text_size(px(18.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text))
                            .child("NyaTerm"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(palette.text_dimmed))
                            .child(format!("v{}", env!("CARGO_PKG_VERSION"))),
                    )
                    .child(
                        div()
                            .px_3()
                            .text_xs()
                            .line_height(px(18.))
                            .text_center()
                            .text_color(rgb(palette.text_muted))
                            .child(self.tr("about.description")),
                    )
                    .child(
                        div()
                            .mt_2()
                            .w_full()
                            .flex()
                            .gap_3()
                            .child(
                                div()
                                    .id("about-website")
                                    .h(px(30.))
                                    .flex_1()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(palette.border))
                                    .text_xs()
                                    .text_color(rgb(palette.text))
                                    .cursor_pointer()
                                    .hover(|this| this.bg(rgb(palette.hover)))
                                    .child(self.tr("about.website"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.open_external_url_for_ui("https://nyaterm.app", cx);
                                    })),
                            )
                            .child(
                                div()
                                    .id("about-issues")
                                    .h(px(30.))
                                    .flex_1()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(palette.border))
                                    .text_xs()
                                    .text_color(rgb(palette.text))
                                    .cursor_pointer()
                                    .hover(|this| this.bg(rgb(palette.hover)))
                                    .child(self.tr("about.issues"))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.open_external_url_for_ui(
                                            "https://github.com/nyakang/nyaterm/issues",
                                            cx,
                                        );
                                    })),
                            ),
                    )
                    .child(small_button(
                        palette,
                        "about-close",
                        self.tr("about.close"),
                        cx.listener(|this, _, _, cx| {
                            this.close_about(cx);
                        }),
                    )),
            )
    }
}

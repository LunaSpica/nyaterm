use super::*;

const RELEASES_URL: &str = "https://github.com/nyakang/nyaterm/releases";

impl NyaTermApp {
    pub(in crate::features) fn update_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let update_info = self.update_info.clone();
        let checking = self.update_pending;
        let failed = !checking
            && update_info.is_none()
            && self.update_status.starts_with("update check failed:");
        let (viewport_w, viewport_h) = self.last_viewport_size;
        let dialog_width = (viewport_w - 32.).clamp(320., 560.);
        let release_url = update_info
            .as_ref()
            .and_then(|info| info.html_url.clone())
            .unwrap_or_else(|| RELEASES_URL.to_string());

        div()
            .id("update-overlay")
            .absolute()
            .inset_0()
            .bg(rgba(0x030508d8))
            .flex()
            .items_center()
            .justify_center()
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_update_dialog(cx);
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    this.close_update_dialog(cx);
                }
            }))
            .child(
                div()
                    .id("update-dialog")
                    .w(px(dialog_width))
                    .max_w_full()
                    .max_h(px((viewport_h - 32.).max(220.)))
                    .mx_4()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface))
                    .shadow_lg()
                    .overflow_hidden()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .px_5()
                            .py_4()
                            .border_b_1()
                            .border_color(rgb(palette.border))
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(palette.text))
                                    .child(if checking {
                                        self.tr("updater.checking")
                                    } else if failed {
                                        self.tr("updater.updateFailed")
                                    } else if update_info
                                        .as_ref()
                                        .is_some_and(|info| info.available)
                                    {
                                        self.tr("updater.newVersionAvailable")
                                    } else {
                                        self.tr("updater.noUpdate")
                                    }),
                            )
                            .when_some(update_info.as_ref(), |this, info| {
                                this.child(
                                    div().text_xs().text_color(rgb(palette.text_muted)).child(
                                        format!(
                                            "{}: v{}  ·  {}: v{}",
                                            self.tr("updater.currentVersion"),
                                            info.current_version,
                                            self.tr("updater.newVersion"),
                                            info.latest_version
                                        ),
                                    ),
                                )
                            })
                            .when(failed, |this| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .line_height(px(16.))
                                        .text_color(rgb(palette.danger))
                                        .child(self.update_status.clone()),
                                )
                            }),
                    )
                    .when(checking, |this| {
                        this.child(
                            div()
                                .px_5()
                                .py_5()
                                .text_xs()
                                .text_color(rgb(palette.text_muted))
                                .child(self.tr("updater.checking")),
                        )
                    })
                    .when(!checking && !failed, |this| {
                        this.when_some(
                            update_info
                                .as_ref()
                                .and_then(|info| info.release_date.clone()),
                            |this, date| {
                                this.child(
                                    div()
                                        .px_5()
                                        .pt_4()
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .child(format!(
                                            "{}: {}",
                                            self.tr("updater.releaseDate"),
                                            date
                                        )),
                                )
                            },
                        )
                        .when_some(
                            update_info
                                .as_ref()
                                .and_then(|info| info.release_notes.clone()),
                            |this, notes| {
                                this.child(
                                    div()
                                        .id("update-release-notes")
                                        .mx_5()
                                        .mt_4()
                                        .max_h(px((viewport_h * 0.42).clamp(120., 320.)))
                                        .overflow_y_scroll()
                                        .rounded_md()
                                        .border_1()
                                        .border_color(rgb(palette.border))
                                        .bg(rgb(palette.input))
                                        .p_3()
                                        .text_xs()
                                        .line_height(px(18.))
                                        .whitespace_normal()
                                        .text_color(rgb(palette.text_muted))
                                        .child(notes),
                                )
                            },
                        )
                    })
                    .child(
                        div()
                            .px_5()
                            .py_4()
                            .border_t_1()
                            .border_color(rgb(palette.border))
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "update-close",
                                self.tr("common.close"),
                                cx.listener(|this, _, _, cx| {
                                    this.close_update_dialog(cx);
                                }),
                            ))
                            .when(failed, |this| {
                                this.child(small_button(
                                    palette,
                                    "update-retry",
                                    self.tr("updater.retry"),
                                    cx.listener(|this, _, _, cx| {
                                        this.start_update_check(cx);
                                    }),
                                ))
                            })
                            .when(
                                !checking
                                    && !failed
                                    && update_info.as_ref().is_some_and(|info| info.available),
                                |this| {
                                    this.child(small_button(
                                        palette,
                                        "update-open-releases",
                                        self.tr("updater.openReleases"),
                                        cx.listener(move |this, _, _, cx| {
                                            this.open_external_url_for_ui(&release_url, cx);
                                        }),
                                    ))
                                },
                            ),
                    ),
            )
    }
}

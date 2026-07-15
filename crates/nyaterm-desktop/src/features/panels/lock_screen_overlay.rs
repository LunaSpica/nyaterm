use super::*;

impl NyaTermApp {
    pub(in crate::features) fn lock_screen_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let password_display = if self.lock_password_draft.is_empty() {
            " ".to_string()
        } else {
            "*".repeat(self.lock_password_draft.chars().count())
        };
        let lock_status = if self.lock_status.trim().is_empty() {
            if self.settings.has_master_password {
                "Enter the master password to unlock.".to_string()
            } else {
                "No master password is configured.".to_string()
            }
        } else {
            self.lock_status.clone()
        };
        let status_is_error =
            lock_status.starts_with("Wrong") || lock_status.starts_with("Unlock failed");

        div()
            .id(SharedString::from("lock-screen-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .flex()
            .flex_col()
            .bg(rgb(0x030508))
            .text_color(rgb(0xffffff))
            .track_focus(&self.lock_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.lock_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                this.handle_lock_key_down(event, cx);
            }))
            .child(
                div()
                    .h(px(40.))
                    .flex()
                    .items_center()
                    .text_color(rgb(0xb6bfcc))
                    .child(
                        div()
                            .h_full()
                            .flex_1()
                            .window_control_area(WindowControlArea::Drag),
                    )
                    .child(window_control_button(
                        palette,
                        "lock-window-min",
                        "-",
                        WindowControlArea::Min,
                        |_, window, _| window.minimize_window(),
                    ))
                    .child(window_control_button(
                        palette,
                        "lock-window-max",
                        "[]",
                        WindowControlArea::Max,
                        |_, window, _| window.zoom_window(),
                    ))
                    .child(window_control_button(
                        palette,
                        "lock-window-close",
                        "x",
                        WindowControlArea::Close,
                        |_, window, _| window.remove_window(),
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_5()
                            .child(
                                div()
                                    .relative()
                                    .child(
                                        div()
                                            .size(px(82.))
                                            .rounded_lg()
                                            .bg(rgb(palette.success))
                                            .shadow_lg()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .text_3xl()
                                            .font_weight(FontWeight(900.))
                                            .text_color(rgb(0x062018))
                                            .child("N"),
                                    )
                                    .child(
                                        div()
                                            .absolute()
                                            .right(px(-6.))
                                            .bottom(px(-6.))
                                            .size(px(28.))
                                            .rounded_full()
                                            .border_2()
                                            .border_color(rgb(0x030508))
                                            .bg(rgb(0x1f2937))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .text_sm()
                                            .font_weight(FontWeight(900.))
                                            .child("LK"),
                                    ),
                            )
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(FontWeight(800.))
                                    .child("NyaTerm Locked"),
                            )
                            .child(
                                div()
                                    .max_w(px(360.))
                                    .text_center()
                                    .text_sm()
                                    .line_height(px(20.))
                                    .text_color(rgb(palette.text_muted))
                                    .child("The workspace is hidden until you unlock this window."),
                            )
                            .when(self.settings.has_master_password, |this| {
                                this.child(
                                    div()
                                        .w(px(280.))
                                        .flex()
                                        .flex_col()
                                        .gap_2()
                                        .child(
                                            div()
                                                .id(SharedString::from("lock-password-input"))
                                                .h(px(42.))
                                                .px_3()
                                                .flex()
                                                .items_center()
                                                .rounded_md()
                                                .border_1()
                                                .border_color(if status_is_error {
                                                    rgb(0x8b2d2d)
                                                } else {
                                                    rgb(palette.border)
                                                })
                                                .bg(rgb(palette.input))
                                                .font_family(
                                                    crate::features::gpui_code_font_family(),
                                                )
                                                .text_sm()
                                                .text_color(rgb(palette.text))
                                                .cursor_pointer()
                                                .track_focus(&self.lock_focus)
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    window.focus(&this.lock_focus);
                                                    cx.notify();
                                                }))
                                                .child(password_display),
                                        )
                                        .child(
                                            div()
                                                .text_center()
                                                .text_xs()
                                                .text_color(if status_is_error {
                                                    rgb(palette.danger)
                                                } else {
                                                    rgb(palette.text_muted)
                                                })
                                                .child(lock_status.clone()),
                                        ),
                                )
                            })
                            .when(!self.settings.has_master_password, |this| {
                                this.child(
                                    div()
                                        .text_center()
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .child(lock_status.clone()),
                                )
                            })
                            .child(small_button(
                                palette,
                                "lock-screen-unlock",
                                "Unlock",
                                cx.listener(|this, _, _, cx| {
                                    this.submit_lock_unlock(cx);
                                }),
                            )),
                    ),
            )
    }
}

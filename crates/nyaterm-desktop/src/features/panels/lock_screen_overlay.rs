use super::*;

impl NyaTermApp {
    pub(in crate::features) fn lock_screen_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let input_entity = cx.entity();
        let password_length = self.lock_password_draft.chars().count()
            + self.lock_password_marked_text.chars().count();
        let password_display = if password_length == 0 {
            " ".to_string()
        } else {
            "•".repeat(password_length.min(32))
        };
        let lock_status = if self.lock_status.trim().is_empty() {
            if self.settings.has_master_password {
                self.tr("lockScreen.passwordPlaceholder").to_string()
            } else {
                self.tr("settings.masterPasswordRequired").to_string()
            }
        } else {
            self.lock_status.clone()
        };
        let status_is_error = lock_status == self.tr("lockScreen.wrongPassword")
            || lock_status.starts_with(self.tr("lockScreen.unlockFailed"));

        div()
            .id(SharedString::from("lock-screen-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .flex()
            .flex_col()
            .bg(rgba(0x000000d9))
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
                        "–",
                        WindowControlArea::Min,
                        |_, window, _| window.minimize_window(),
                    ))
                    .child(window_control_button(
                        palette,
                        "lock-window-max",
                        "□",
                        WindowControlArea::Max,
                        |_, window, _| window.zoom_window(),
                    ))
                    .child(window_control_button(
                        palette,
                        "lock-window-close",
                        "×",
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
                                            .child(
                                                svg()
                                                    .size(px(68.))
                                                    .path("icons/logo.svg")
                                                    .text_color(rgb(0x062018)),
                                            ),
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
                                            .child(
                                                svg()
                                                    .size(px(16.))
                                                    .path("icons/lock.svg")
                                                    .text_color(rgb(0xffffff)),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .text_xl()
                                    .font_weight(FontWeight(800.))
                                    .child(self.tr("lockScreen.title")),
                            )
                            .child(
                                div()
                                    .max_w(px(360.))
                                    .text_center()
                                    .text_sm()
                                    .line_height(px(20.))
                                    .text_color(rgb(palette.text_muted))
                                    .child(self.tr("lockScreen.message")),
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
                                                .relative()
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
                                                .child(password_display)
                                                .child(
                                                    gpui::canvas(
                                                        |_bounds, _window, _cx| {},
                                                        move |bounds, _state, window, cx| {
                                                            let focus = input_entity
                                                                .read(cx)
                                                                .lock_focus
                                                                .clone();
                                                            window.handle_input(
                                                                &focus,
                                                                gpui::ElementInputHandler::new(
                                                                    bounds,
                                                                    input_entity.clone(),
                                                                ),
                                                                cx,
                                                            );
                                                        },
                                                    )
                                                    .absolute()
                                                    .inset_0(),
                                                ),
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
                                self.tr("lockScreen.unlock"),
                                cx.listener(|this, _, _, cx| {
                                    this.submit_lock_unlock(cx);
                                }),
                            )),
                    ),
            )
    }
}

use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn rename_session_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let draft_display = if self.rename_draft.is_empty() {
            "Tab name".to_string()
        } else {
            self.rename_draft.clone()
        };
        let can_save = !self.rename_draft.trim().is_empty();

        div()
            .id(SharedString::from("rename-tab-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.rename_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.rename_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                this.handle_rename_key_down(event, cx);
            }))
            .child(
                div()
                    .id(SharedString::from("rename-tab-dialog"))
                    .w(px(320.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x303848))
                    .bg(rgb(0x0b0f16))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child("Rename Tab"),
                    )
                    .child(
                        div()
                            .id(SharedString::from("rename-tab-input"))
                            .mt_3()
                            .h(px(36.))
                            .rounded_sm()
                            .border_1()
                            .border_color(if can_save {
                                rgb(0x334155)
                            } else {
                                rgb(0x7f1d1d)
                            })
                            .bg(rgb(palette.input))
                            .px_3()
                            .flex()
                            .items_center()
                            .font_family("JetBrains Mono")
                            .text_sm()
                            .text_color(if self.rename_draft.is_empty() {
                                rgb(palette.text_muted)
                            } else {
                                rgb(palette.text)
                            })
                            .child(draft_display),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(rgb(0x8f98aa))
                            .child("Enter save / Esc cancel / 64 characters max"),
                    )
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(small_button(palette, 
                                "rename-tab-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.close_rename_session(cx);
                                }),
                            ))
                            .child(div().when(!can_save, |this| this.opacity(0.45)).child(
                                small_button(palette, 
                                    "rename-tab-save",
                                    "Save",
                                    cx.listener(|this, _, _, cx| {
                                        this.submit_rename_session(cx);
                                    }),
                                ),
                            )),
                    ),
            )
    }

    pub(in crate::ui::view) fn tab_color_picker_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let active_color = self
            .active_session_id
            .as_deref()
            .and_then(|session_id| self.session_tab_colors.get(session_id))
            .copied();
        let mut swatches = div().mt_3().grid().grid_cols(6).gap_2();
        for (name, color) in TAB_PRESET_COLORS {
            let selected = active_color == Some(color);
            swatches = swatches.child(
                div()
                    .id(SharedString::from(format!("tab-color-{name}")))
                    .size(px(28.))
                    .rounded_full()
                    .border_2()
                    .border_color(if selected {
                        rgb(0xffffff)
                    } else {
                        rgb(0x1f2937)
                    })
                    .bg(rgb(color))
                    .cursor_pointer()
                    .hover(|this| this.border_color(rgb(palette.text)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_active_session_tab_color(Some(color), cx);
                    })),
            );
        }

        div()
            .id(SharedString::from("tab-color-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.color_picker_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.color_picker_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                if event.keystroke.key == "escape" {
                    this.close_tab_color_picker(cx);
                }
            }))
            .child(
                div()
                    .id(SharedString::from("tab-color-dialog"))
                    .w(px(300.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x303848))
                    .bg(rgb(0x0b0f16))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child("Set Tab Color"),
                    )
                    .child(swatches)
                    .child(
                        div()
                            .mt_3()
                            .text_xs()
                            .text_color(rgb(0x8f98aa))
                            .child("Pick a swatch / Esc cancel"),
                    )
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(small_button(palette, 
                                "tab-color-reset",
                                "Reset",
                                cx.listener(|this, _, _, cx| {
                                    this.set_active_session_tab_color(None, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "tab-color-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.close_tab_color_picker(cx);
                                }),
                            )),
                    ),
            )
    }

    pub(in crate::ui::view) fn session_info_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let details = self.active_session_info_details().unwrap_or_default();
        let title = details
            .iter()
            .find(|(label, _)| *label == "Name")
            .map(|(_, value)| value.clone())
            .unwrap_or_else(|| "Session Info".to_string());
        let mut rows = div().mt_4().flex().flex_col().gap_2();
        if details.is_empty() {
            rows = rows.child(
                div()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(0x303848))
                    .bg(rgb(palette.input))
                    .p_3()
                    .text_sm()
                    .text_color(rgb(palette.text_muted))
                    .child("No active session details."),
            );
        } else {
            for (label, value) in details {
                rows = rows.child(session_info_row(palette, label, value));
            }
        }

        div()
            .id(SharedString::from("session-info-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.session_info_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.session_info_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                if event.keystroke.key == "escape" {
                    this.close_active_session_info(cx);
                }
            }))
            .child(
                div()
                    .id(SharedString::from("session-info-dialog"))
                    .w(px(520.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x303848))
                    .bg(rgb(0x0b0f16))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .gap_3()
                            .child(div().size(px(10.)).mt_1().rounded_full().bg(rgb(0x6ee7b7)))
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight(800.))
                                            .text_color(rgb(palette.text))
                                            .child("Session Info"),
                                    )
                                    .child(
                                        div()
                                            .mt_1()
                                            .text_xs()
                                            .text_color(rgb(palette.text_muted))
                                            .child(truncate_preview(&title, 56)),
                                    ),
                            ),
                    )
                    .child(rows)
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(small_button(palette, 
                                "session-info-copy",
                                "Copy",
                                cx.listener(|this, _, _, cx| {
                                    this.copy_active_session_info(cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "session-info-close",
                                "Close",
                                cx.listener(|this, _, _, cx| {
                                    this.close_active_session_info(cx);
                                }),
                            )),
                    ),
            )
    }

    pub(in crate::ui::view) fn startup_command_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let action = self.startup_command_action;
        let command_display = if self.startup_command_draft.is_empty() {
            action.placeholder().to_string()
        } else {
            self.startup_command_draft.clone()
        };
        let can_submit = !self.startup_command_draft.trim().is_empty();
        let delay_label = format!("{} ms", self.startup_command_delay_ms);

        div()
            .id(SharedString::from("startup-command-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.startup_command_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.startup_command_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                this.handle_startup_command_key_down(event, window, cx);
            }))
            .child(
                div()
                    .id(SharedString::from("startup-command-dialog"))
                    .w(px(420.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x303848))
                    .bg(rgb(0x0b0f16))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(action.title()),
                    )
                    .child(
                        div()
                            .id(SharedString::from("startup-command-input"))
                            .mt_3()
                            .h(px(38.))
                            .rounded_sm()
                            .border_1()
                            .border_color(if can_submit {
                                rgb(0x334155)
                            } else {
                                rgb(0x7f1d1d)
                            })
                            .bg(rgb(palette.input))
                            .px_3()
                            .flex()
                            .items_center()
                            .font_family("JetBrains Mono")
                            .text_sm()
                            .text_color(if self.startup_command_draft.is_empty() {
                                rgb(palette.text_muted)
                            } else {
                                rgb(palette.text)
                            })
                            .child(truncate_preview(&command_display, 76)),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(0x8f98aa))
                                            .child("Command Delay"),
                                    )
                                    .child(
                                        div()
                                            .font_family("JetBrains Mono")
                                            .text_sm()
                                            .text_color(rgb(palette.text))
                                            .child(delay_label),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(small_button(palette, 
                                        "startup-delay-minus",
                                        "-100",
                                        cx.listener(|this, _, _, cx| {
                                            this.adjust_startup_command_delay(-100, cx);
                                        }),
                                    ))
                                    .child(small_button(palette, 
                                        "startup-delay-zero",
                                        "0",
                                        cx.listener(|this, _, _, cx| {
                                            this.startup_command_delay_ms = 0;
                                            cx.notify();
                                        }),
                                    ))
                                    .child(small_button(palette, 
                                        "startup-delay-plus",
                                        "+100",
                                        cx.listener(|this, _, _, cx| {
                                            this.adjust_startup_command_delay(100, cx);
                                        }),
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(rgb(0x8f98aa))
                            .child("Enter submit / Esc cancel / Up Down adjust delay"),
                    )
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(small_button(palette, 
                                "startup-command-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.close_startup_command_dialog(cx);
                                }),
                            ))
                            .child(div().when(!can_submit, |this| this.opacity(0.45)).child(
                                small_button(palette, 
                                    "startup-command-submit",
                                    action.submit_label(),
                                    cx.listener(|this, _, window, cx| {
                                        this.submit_startup_command_dialog(window, cx);
                                    }),
                                ),
                            )),
                    ),
            )
    }
}

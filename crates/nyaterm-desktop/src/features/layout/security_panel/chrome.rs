use super::*;

impl NyaTermApp {
    pub(in crate::features) fn security_secret_footer(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let unlocked = !self.security_secrets_locked();
        let has_master = self.settings.has_master_password;
        let palette = self.theme_palette();
        div()
            .flex_none()
            .border_t_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.section_header))
            .px_2()
            .py_2()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .child(
                div()
                    .text_size(px(10.))
                    .text_color(if unlocked {
                        rgb(palette.success)
                    } else {
                        rgb(palette.warning)
                    })
                    .child(if !has_master {
                        "Secrets open (no master password)"
                    } else if unlocked {
                        "Secrets unlocked"
                    } else {
                        "Secrets locked"
                    }),
            )
            .child(div().flex().items_center().gap_1().child(small_button(
                palette,
                "security-secrets-toggle",
                if unlocked && has_master {
                    "Lock"
                } else if unlocked {
                    "Open"
                } else {
                    "Unlock"
                },
                cx.listener(|this, _, window, cx| {
                    if this.security_secrets_locked() {
                        this.open_security_unlock_prompt(window, cx);
                    } else if this.settings.has_master_password {
                        this.lock_security_secrets(cx);
                    } else {
                        this.security_status =
                            "set a master password in Settings to lock secrets".to_string();
                        cx.notify();
                    }
                }),
            )))
    }

    pub(in crate::features) fn security_unlock_prompt(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let draft = if self.security_unlock_draft.is_empty() {
            " ".to_string()
        } else {
            "•".repeat(self.security_unlock_draft.chars().count().min(32))
        };
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x0d1117cc))
            .child(
                div()
                    .w(px(280.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface))
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .track_focus(&self.security_unlock_focus)
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                        this.handle_security_unlock_key_down(event, cx);
                    }))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child("Unlock Secrets"),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_muted))
                            .child("Enter master password to view/copy secrets."),
                    )
                    .child(
                        div()
                            .id(SharedString::from("security-unlock-input"))
                            .h(px(32.))
                            .px_2()
                            .flex()
                            .items_center()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(palette.accent))
                            .bg(rgb(palette.input))
                            .font_family("JetBrains Mono")
                            .text_xs()
                            .text_color(rgb(palette.text))
                            .child(draft),
                    )
                    .when_some(self.security_unlock_error.clone(), |this, error| {
                        this.child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(palette.danger))
                                .child(error),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "security-unlock-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.close_security_unlock_prompt(cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "security-unlock-submit",
                                "Unlock",
                                cx.listener(|this, _, _, cx| {
                                    this.submit_security_unlock(cx);
                                }),
                            )),
                    ),
            )
    }

    pub(in crate::features) fn security_tab_chip(
        &self,
        tab: SecurityAuthTab,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.security_auth_tab == tab;
        let palette = self.theme_palette();
        // Tauri TabsTrigger text-xs inside h-8 grid segment.
        div()
            .id(SharedString::from(format!("security-tab-{}", tab.label())))
            .h(px(28.))
            .flex_1()
            .px_1()
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .cursor_pointer()
            .text_size(px(11.))
            .font_weight(FontWeight(if selected { 600. } else { 500. }))
            .text_color(if selected {
                rgb(palette.text)
            } else {
                rgb(palette.text_muted)
            })
            .bg(if selected {
                rgb(palette.input)
            } else {
                rgb(palette.surface_elevated)
            })
            .hover(move |this| {
                this.bg(if selected {
                    rgb(palette.input)
                } else {
                    rgb(palette.hover)
                })
                .text_color(rgb(palette.text))
            })
            .child(tab.label())
            .on_click(cx.listener(move |this, _, _, cx| {
                this.set_security_auth_tab(tab, cx);
            }))
    }
}

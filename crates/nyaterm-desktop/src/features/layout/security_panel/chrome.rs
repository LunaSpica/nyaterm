use super::*;

use crate::models::SecurityAuthTab;

impl NyaTermApp {
    pub(in crate::features) fn security_secret_footer(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let unlocked = self.settings.has_master_password && self.security.unlock.secrets_unlocked;
        let palette = self.theme_palette();
        div()
            .id(SharedString::from("security-secrets-toggle"))
            .h(px(40.))
            .flex_none()
            .border_t_1()
            .border_color(rgb(palette.border))
            .bg(rgba((palette.primary << 8) | 0x1a))
            .px_3()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .cursor_pointer()
            .hover(move |this| this.bg(rgba((palette.primary << 8) | 0x26)))
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .size(px(24.))
                            .flex_none()
                            .rounded_md()
                            .border_1()
                            .border_color(rgba((palette.primary << 8) | 0x40))
                            .bg(self.shell_surface_color(palette.bg))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                svg()
                                    .size(px(14.))
                                    .path("icons/lock.svg")
                                    .text_color(rgb(palette.primary)),
                            ),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .overflow_hidden()
                            .text_xs()
                            .text_color(rgb(palette.text))
                            .child(self.tr(if unlocked {
                                "secretUnlock.unlockedTitle"
                            } else {
                                "secretUnlock.lockedTitle"
                            })),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .text_xs()
                    .font_weight(FontWeight(600.))
                    .text_color(rgb(palette.primary))
                    .child(self.tr(if unlocked {
                        "secretUnlock.lockAction"
                    } else {
                        "secretUnlock.unlockAction"
                    })),
            )
            .on_click(cx.listener(|this, _, window, cx| {
                if this.security_secrets_locked() {
                    this.open_security_unlock_prompt(window, cx);
                } else if this.settings.has_master_password {
                    this.lock_security_secrets(cx);
                } else {
                    this.open_security_unlock_prompt(window, cx);
                }
            }))
    }

    pub(in crate::features) fn security_unlock_prompt(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let draft_length = self.security.unlock.draft.chars().count()
            + self.security.unlock.marked_text.chars().count();
        let draft = if draft_length == 0 {
            " ".to_string()
        } else {
            "•".repeat(draft_length.min(32))
        };
        let input_entity = cx.entity();
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
                    .track_focus(&self.security.unlock.focus)
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        this.handle_security_unlock_key_down(event, window, cx);
                    }))
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("secretUnlock.unlockTitle")),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_muted))
                            .child(self.tr("secretUnlock.unlockDescription")),
                    )
                    .child(
                        div()
                            .id(SharedString::from("security-unlock-input"))
                            .relative()
                            .h(px(32.))
                            .px_2()
                            .flex()
                            .items_center()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(palette.link))
                            .bg(rgb(palette.input))
                            .font_family(crate::features::gpui_code_font_family())
                            .text_xs()
                            .text_color(rgb(palette.text))
                            .child(draft)
                            .child(
                                gpui::canvas(
                                    |_bounds, _window, _cx| {},
                                    move |bounds, _state, window, cx| {
                                        let focus =
                                            input_entity.read(cx).security.unlock.focus.clone();
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
                    .when_some(self.security.unlock.error.clone(), |this, error| {
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
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_security_unlock_prompt(cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "security-unlock-submit",
                                self.tr("secretUnlock.unlock"),
                                cx.listener(|this, _, window, cx| {
                                    this.submit_security_unlock(window, cx);
                                }),
                            )),
                    ),
            )
    }

    pub(in crate::features) fn security_master_required_prompt(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgba(0x0d1117cc))
            .child(
                div()
                    .w(px(320.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface))
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("settings.masterPasswordRequired")),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_muted))
                            .child(self.tr("settings.masterPasswordRequiredDesc")),
                    )
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "security-master-required-cancel",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.close_security_master_required_prompt(cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "security-master-required-settings",
                                self.tr("settings.security"),
                                cx.listener(|this, _, _, cx| {
                                    this.open_security_settings_from_prompt(cx);
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
        let selected = self.security.auth_tab == tab;
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
            .child(self.tr(tab.i18n_key()))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.set_security_auth_tab(tab, cx);
            }))
    }
}

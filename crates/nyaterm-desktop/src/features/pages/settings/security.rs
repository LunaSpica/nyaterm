use super::*;

impl NyaTermApp {
    pub(in crate::features) fn security_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let master_password_input = self
            .text_input_box(
                "settings.security.master-password",
                &self.settings_master_password_draft.clone(),
                TextInputSetup::masked(),
                cx,
            )
            .into_any_element();
        let master_password_enabled = self.settings_master_password_enabled;
        let master_password_switch_enabled = !self.cloud_sync_settings.enabled;
        let has_stored_master_password = self.settings.has_master_password;
        let idle_minutes = self.settings.idle_lock_minutes;
        let host_key_policy = match self.settings.host_key_policy.as_str() {
            "strict" | "reject" => "strict",
            "accept" | "accept_new" => "accept",
            _ => "prompt",
        };

        let master_section_label = self.tr("settings.masterPasswordSection");
        let master_switch_label = self.tr("settings.masterPasswordSwitch");
        let master_switch_desc = self.tr("settings.masterPasswordSwitchDesc");
        let master_locked_desc = self.tr("settings.masterPasswordLockedByCloudSync");
        let master_set_label = self.tr("settings.masterPasswordIsSet");
        let master_input_label = self.tr(if has_stored_master_password {
            "settings.masterPasswordNew"
        } else {
            "settings.masterPassword"
        });
        let master_input_desc = self.tr("settings.masterPasswordDesc");
        let master_input_placeholder = self.tr(if has_stored_master_password {
            "settings.masterPasswordNewPlaceholder"
        } else {
            "settings.masterPasswordPlaceholder"
        });
        let session_security_label = self.tr("settings.sessionSecurity");
        let screen_lock_label = self.tr("settings.enableScreenLock");
        let screen_lock_desc = self.tr("settings.enableScreenLockDesc");
        let idle_lock_label = self.tr("settings.idleLockMinutes");
        let idle_lock_desc = self.tr("settings.idleLockMinutesDesc");
        let minutes_label = self.tr("common.minutes");
        let host_key_label = self.tr("settings.hostKeyPolicy");
        let host_key_desc = self.tr("settings.hostKeyPolicyDesc");

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                palette,
                Some(master_section_label),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        master_switch_label,
                        Some(SharedString::from(master_switch_desc)),
                        div()
                            .id("settings-master-password-switch-wrap")
                            .when(!master_password_switch_enabled, |this| {
                                this.tooltip(move |_, cx| {
                                    cx.new(|_| ChromeTooltip::new(master_locked_desc)).into()
                                })
                            })
                            .child(settings_switch_with_enabled(
                                palette,
                                "settings-master-password-enabled",
                                master_password_enabled,
                                master_password_switch_enabled,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_settings_master_password(cx);
                                }),
                            )),
                    ))
                    .when(
                        has_stored_master_password
                            && self.settings_master_password_draft.is_empty(),
                        |this| {
                            this.child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(palette.text_muted))
                                    .child(master_set_label),
                            )
                        },
                    )
                    .child(settings_form_row(
                        palette,
                        master_input_label,
                        Some(SharedString::from(master_input_desc)),
                        div()
                            .opacity(if master_password_enabled { 1.0 } else { 0.45 })
                            .child(master_password_input),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some(session_security_label),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        screen_lock_label,
                        Some(SharedString::from(screen_lock_desc)),
                        settings_switch(
                            palette,
                            "settings-screen-lock-enabled",
                            self.settings.enable_screen_lock,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_screen_lock_enabled(cx);
                            }),
                        ),
                    ))
                    .when(self.settings.enable_screen_lock, |this| {
                        this.child(settings_form_row(
                            palette,
                            idle_lock_label,
                            Some(SharedString::from(idle_lock_desc)),
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(small_button(
                                    palette,
                                    "settings-idle-lock-minus",
                                    "-",
                                    cx.listener(|this, _, _, cx| {
                                        this.adjust_idle_lock_minutes(-1, cx);
                                    }),
                                ))
                                .child(
                                    div()
                                        .min_w(px(42.))
                                        .text_center()
                                        .font_family(crate::features::gpui_code_font_family())
                                        .text_size(px(12.))
                                        .font_weight(FontWeight(600.))
                                        .text_color(rgb(palette.text))
                                        .child(idle_minutes.to_string()),
                                )
                                .child(small_button(
                                    palette,
                                    "settings-idle-lock-plus",
                                    "+",
                                    cx.listener(|this, _, _, cx| {
                                        this.adjust_idle_lock_minutes(1, cx);
                                    }),
                                ))
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text_muted))
                                        .child(minutes_label),
                                ),
                        ))
                    }),
            ))
            .child(settings_form_section(
                palette,
                None,
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight(500.))
                                    .text_color(rgb(palette.text))
                                    .child(host_key_label),
                            )
                            .child(
                                div()
                                    .mt_1()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette.text_dimmed))
                                    .child(host_key_desc),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(
                                palette,
                                "security-host-strict",
                                self.tr("settings.hostKeyStrict"),
                                host_key_policy == "strict",
                                cx.listener(|this, _, _, cx| {
                                    this.update_host_key_policy("strict", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "security-host-prompt",
                                self.tr("settings.hostKeyPrompt"),
                                host_key_policy == "prompt",
                                cx.listener(|this, _, _, cx| {
                                    this.update_host_key_policy("prompt", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "security-host-accept",
                                self.tr("settings.hostKeyAccept"),
                                host_key_policy == "accept",
                                cx.listener(|this, _, _, cx| {
                                    this.update_host_key_policy("accept", cx);
                                }),
                            )),
                    ),
            ))
    }
}

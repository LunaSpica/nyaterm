use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn security_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let master_password_status = if self.settings.has_master_password {
            "Configured"
        } else if self.cloud_sync_settings.enabled {
            "Required for cloud sync"
        } else {
            "Not set"
        };
        let idle_label = if self.settings.idle_lock_minutes == 0 {
            "Manual only".to_string()
        } else {
            format!("{} min", self.settings.idle_lock_minutes)
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(palette, 
                Some("Master password"),
                Some("Protects encrypted snapshots and the native lock screen."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Status",
                        Some(SharedString::from(master_password_status)),
                        div()
                            .text_size(px(11.))
                            .font_weight(FontWeight(600.))
                            .text_color(if self.settings.has_master_password {
                                rgb(0x3fb950)
                            } else {
                                rgb(0xd29922)
                            })
                            .child(if self.settings.has_master_password {
                                "Ready"
                            } else {
                                "Pending"
                            }),
                    ))
                    .child(settings_form_row(palette, 
                        "Cloud sync dependency",
                        Some(SharedString::from(
                            "Push/pull may request this password before encrypted snapshot work.",
                        )),
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0x8b949e))
                            .child(if self.cloud_sync_settings.enabled {
                                "Enabled"
                            } else {
                                "Disabled"
                            }),
                    )),
            ))
            .child(settings_form_section(palette, 
                Some("Screen lock"),
                Some("Lock the window after idle time or on demand."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Enable screen lock",
                        Some(SharedString::from(
                            "Require the master password to unlock the main window.",
                        )),
                        settings_switch(palette, 
                            "settings-screen-lock-enabled",
                            self.settings.enable_screen_lock,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_screen_lock_enabled(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
                        "Idle lock",
                        Some(SharedString::from(
                            "Automatically lock after this many minutes of inactivity (0 = manual).",
                        )),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(
                                div()
                                    .min_w(px(72.))
                                    .font_family("JetBrains Mono")
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(600.))
                                    .text_color(rgb(0xc9d1d9))
                                    .child(idle_label),
                            )
                            .child(small_button(palette, 
                                "settings-idle-lock-minus",
                                "−5",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_idle_lock_minutes(-5, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "settings-idle-lock-plus",
                                "+5",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_idle_lock_minutes(5, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
                        "Window state",
                        Some(SharedString::from(format!(
                            "Last activity {}s ago",
                            self.last_user_activity_at.elapsed().as_secs()
                        ))),
                        div()
                            .text_size(px(11.))
                            .font_weight(FontWeight(600.))
                            .text_color(if self.is_locked {
                                rgb(0xff7b72)
                            } else {
                                rgb(0x3fb950)
                            })
                            .child(if self.is_locked { "Locked" } else { "Unlocked" }),
                    )),
            ))
            .child(settings_form_section(palette, 
                Some("Host key policy"),
                Some("How SSH host key changes are handled."),
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(settings_choice_chip(palette, 
                        "security-host-ask",
                        "Ask",
                        self.settings.host_key_policy == "ask"
                            || self.settings.host_key_policy == "prompt",
                        cx.listener(|this, _, _, cx| {
                            this.update_host_key_policy("ask", cx);
                        }),
                    ))
                    .child(settings_choice_chip(palette, 
                        "security-host-accept",
                        "Accept new",
                        self.settings.host_key_policy == "accept_new",
                        cx.listener(|this, _, _, cx| {
                            this.update_host_key_policy("accept_new", cx);
                        }),
                    ))
                    .child(settings_choice_chip(palette, 
                        "security-host-strict",
                        "Strict",
                        self.settings.host_key_policy == "strict"
                            || self.settings.host_key_policy == "reject",
                        cx.listener(|this, _, _, cx| {
                            this.update_host_key_policy("strict", cx);
                        }),
                    )),
            ))
    }
}

fn security_hint(title: &'static str, detail: &'static str) -> impl IntoElement {
    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x263142))
        .bg(rgb(0x0d1320))
        .p_3()
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight(800.))
                .text_color(rgb(0xe5edf7))
                .child(title),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(10.))
                .text_color(rgb(0x8f98aa))
                .line_height(px(14.))
                .child(detail),
        )
}

fn host_key_policy_label(policy: &str) -> &'static str {
    match policy {
        "strict" => "strict",
        "accept" => "accept",
        _ => "prompt",
    }
}

fn host_key_policy_detail(policy: &str) -> &'static str {
    match policy {
        "strict" => "Current policy: strict. Unknown or changed host keys are rejected.",
        "accept" => "Current policy: accept. New or changed host keys are saved automatically.",
        _ => "Current policy: prompt. New or changed host keys require confirmation.",
    }
}

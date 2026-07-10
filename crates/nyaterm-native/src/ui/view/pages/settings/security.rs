use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn security_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let master_password_status = if self.settings.has_master_password {
            "configured"
        } else if self.cloud_sync_settings.enabled {
            "required"
        } else {
            "not set"
        };

        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("Master Password"),
                            )
                            .child(status_pill(
                                master_password_status,
                                if self.settings.has_master_password {
                                    rgb(0x6ee7b7)
                                } else {
                                    rgb(0xfacc15)
                                },
                                if self.settings.has_master_password {
                                    rgb(0x12342a)
                                } else {
                                    rgb(0x32280f)
                                },
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(4)
                            .gap_3()
                            .child(metric(
                                "Stored Secret",
                                if self.settings.has_master_password {
                                    "present".to_string()
                                } else {
                                    "missing".to_string()
                                },
                            ))
                            .child(metric(
                                "Cloud Sync",
                                if self.cloud_sync_settings.enabled {
                                    "enabled".to_string()
                                } else {
                                    "disabled".to_string()
                                },
                            ))
                            .child(metric(
                                "Unlock",
                                if self.settings.has_master_password {
                                    "password".to_string()
                                } else {
                                    "manual".to_string()
                                },
                            ))
                            .child(metric(
                                "Snapshots",
                                if self.settings.has_master_password {
                                    "encryptable".to_string()
                                } else {
                                    "prompted".to_string()
                                },
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(3)
                            .gap_2()
                            .child(security_hint(
                                "Verification",
                                "Existing encrypted master passwords are verified by the native lock screen.",
                            ))
                            .child(security_hint(
                                "Cloud Sync",
                                "Push and pull flows request a password before encrypted snapshot work.",
                            ))
                            .child(security_hint(
                                "Migration Gap",
                                "Setting or changing the master password still needs a native save flow.",
                            )),
                    ),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("Session Security"),
                            )
                            .child(status_pill(
                                if self.settings.enable_screen_lock {
                                    "lock on"
                                } else {
                                    "lock off"
                                },
                                if self.settings.enable_screen_lock {
                                    rgb(0x6ee7b7)
                                } else {
                                    rgb(0x98a3b8)
                                },
                                if self.settings.enable_screen_lock {
                                    rgb(0x12342a)
                                } else {
                                    rgb(0x202633)
                                },
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(4)
                            .gap_3()
                            .child(metric(
                                "Screen Lock",
                                if self.settings.enable_screen_lock {
                                    "enabled".to_string()
                                } else {
                                    "disabled".to_string()
                                },
                            ))
                            .child(metric(
                                "Idle Lock",
                                if self.settings.idle_lock_minutes == 0 {
                                    "manual only".to_string()
                                } else {
                                    format!("{} min", self.settings.idle_lock_minutes)
                                },
                            ))
                            .child(metric(
                                "Window",
                                if self.is_locked {
                                    "locked".to_string()
                                } else {
                                    "unlocked".to_string()
                                },
                            ))
                            .child(metric(
                                "Idle Timer",
                                format!("{}s ago", self.last_user_activity_at.elapsed().as_secs()),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                "settings-screen-lock-enabled",
                                if self.settings.enable_screen_lock {
                                    "Lock On"
                                } else {
                                    "Lock Off"
                                },
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_screen_lock_enabled(cx);
                                }),
                            ))
                            .child(small_button(
                                "settings-idle-lock-minus",
                                "-1 Min",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_idle_lock_minutes(-1, cx);
                                }),
                            ))
                            .child(small_button(
                                "settings-idle-lock-plus",
                                "+1 Min",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_idle_lock_minutes(1, cx);
                                }),
                            ))
                            .child(small_button(
                                "settings-lock-now",
                                "Lock Now",
                                cx.listener(|this, _, window, cx| {
                                    this.lock_app(window, cx);
                                }),
                            ))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x98a3b8))
                                    .child("0 min keeps automatic locking disabled."),
                            ),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(3)
                            .gap_2()
                            .child(security_hint(
                                "Master Password",
                                if self.settings.has_master_password {
                                    "Configured for unlocking and encrypted snapshots."
                                } else {
                                    "Not configured; manual unlock does not require a password."
                                },
                            ))
                            .child(security_hint(
                                "Idle Lock",
                                "Set minutes to 0 for manual lock only.",
                            ))
                            .child(security_hint(
                                "Lock Now",
                                "Immediately hides the workspace behind the lock screen.",
                            )),
                    )
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("SSH Host Keys"),
                            )
                            .child(status_pill(
                                host_key_policy_label(&self.settings.host_key_policy),
                                rgb(0x93c5fd),
                                rgb(0x17253b),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(policy_button(
                                "settings-host-key-prompt",
                                "Prompt",
                                self.settings.host_key_policy == "prompt",
                                cx.listener(|this, _, _, cx| {
                                    this.update_host_key_policy("prompt", cx);
                                }),
                            ))
                            .child(policy_button(
                                "settings-host-key-strict",
                                "Strict",
                                self.settings.host_key_policy == "strict",
                                cx.listener(|this, _, _, cx| {
                                    this.update_host_key_policy("strict", cx);
                                }),
                            ))
                            .child(policy_button(
                                "settings-host-key-accept",
                                "Accept",
                                self.settings.host_key_policy == "accept",
                                cx.listener(|this, _, _, cx| {
                                    this.update_host_key_policy("accept", cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(3)
                            .gap_2()
                            .child(security_hint(
                                "Strict",
                                "Reject unknown or changed host keys.",
                            ))
                            .child(security_hint(
                                "Prompt",
                                "Ask before trusting new or changed keys.",
                            ))
                            .child(security_hint(
                                "Accept",
                                "Automatically record new or changed keys.",
                            )),
                    ),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Security Coverage"),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(3)
                            .gap_2()
                            .child(security_hint(
                                "Host Prompt",
                                "Unknown and changed host keys surface as native accept/reject banners.",
                            ))
                            .child(security_hint(
                                "Credential Data",
                                "Encrypted credential and key material remains in the domain store.",
                            ))
                            .child(security_hint(
                                "Parity Target",
                                "Next step is a native master-password editor backed by encrypted storage.",
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .text_xs()
                            .text_color(rgb(0x98a3b8))
                            .child(host_key_policy_detail(&self.settings.host_key_policy)),
                    ),
            )
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

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
                                rgb(palette.success)
                            } else {
                                rgb(palette.warning)
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
                            .text_color(rgb(palette.text_muted))
                            .child(if self.cloud_sync_settings.enabled {
                                "Enabled"
                            } else {
                                "Disabled"
                            }),
                    )),
            ))
            .child(settings_form_section(palette, 
                Some("Session security"),
                Some("Lock the window after idle time or on demand (Tauri SecurityTab)."),
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
                    .when(self.settings.enable_screen_lock, |this| {
                        this.child(
                            div()
                                .pl_3()
                                .ml_1()
                                .border_l_1()
                                .border_color(rgb(palette.border))
                                .child(settings_form_row(
                                    palette,
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
                                                .text_color(rgb(palette.text))
                                                .child(idle_label.clone()),
                                        )
                                        .child(small_button(
                                            palette,
                                            "settings-idle-lock-minus",
                                            "−5",
                                            cx.listener(|this, _, _, cx| {
                                                this.adjust_idle_lock_minutes(-5, cx);
                                            }),
                                        ))
                                        .child(small_button(
                                            palette,
                                            "settings-idle-lock-plus",
                                            "+5",
                                            cx.listener(|this, _, _, cx| {
                                                this.adjust_idle_lock_minutes(5, cx);
                                            }),
                                        )),
                                )),
                        )
                    })
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
                                rgb(palette.danger)
                            } else {
                                rgb(palette.success)
                            })
                            .child(if self.is_locked { "Locked" } else { "Unlocked" }),
                    )),
            ))
            .child(settings_form_section(palette, 
                Some("Host key policy"),
                Some("How SSH host key changes are handled."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
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
                                self.settings.host_key_policy == "accept_new"
                                    || self.settings.host_key_policy == "accept",
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
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(host_key_policy_detail(&self.settings.host_key_policy)),
                    ),
            ))
    }
}

fn security_hint(
    palette: crate::ui::theme::ThemePalette,
    title: &'static str,
    detail: &'static str,
) -> impl IntoElement {
    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_3()
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight(800.))
                .text_color(rgb(palette.text))
                .child(title),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(10.))
                .text_color(rgb(palette.text_muted))
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
        "strict" | "reject" => "Current policy: strict. Unknown or changed host keys are rejected.",
        "accept_new" | "accept" => {
            "Current policy: accept new. New host keys are saved automatically; changes still prompt."
        }
        _ => "Current policy: ask. New or changed host keys require confirmation.",
    }
}

use super::*;

impl NyaTermApp {
    pub(super) fn security_otp_body(
        &mut self,
        palette: ThemePalette,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let mut body = security_auth_body_base("security-otp-body");
        let actions_enabled = self.security_otp_editor.is_none();
        let has_entries = !self.connection_otp_entries.is_empty();
        body = body.child(
            div()
                .flex_none()
                .h(px(28.))
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight(600.))
                        .text_color(rgb(palette.text))
                        .child(self.tr("otpManager.title")),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(security_toolbar_action_button(
                            palette,
                            "security-otp-refresh-visible",
                            self.tr("common.refresh"),
                            actions_enabled && has_entries,
                            cx.listener(|this, _, window, cx| {
                                this.refresh_visible_security_otp_codes(window, cx);
                            }),
                        ))
                        .child(security_toolbar_action_button(
                            palette,
                            "security-add-otp",
                            self.tr("otpManager.add"),
                            actions_enabled,
                            cx.listener(|this, _, window, cx| {
                                this.open_security_otp_editor(None, window, cx);
                            }),
                        )),
                ),
        );
        if let Some(editor) = self.security_otp_editor.clone() {
            body = body.child(self.security_otp_editor_view(editor, cx));
        } else if self.connection_otp_entries.is_empty() {
            body = body.child(empty_panel(
                self.tr("otpManager.noEntries"),
                self.theme_palette(),
            ));
        } else {
            let entries = self.connection_otp_entries.clone();
            let entry_count = entries.len();
            let mut rows = div()
                .rounded_md()
                .border_1()
                .border_color(rgb(palette.border))
                .overflow_hidden();
            for (index, entry) in entries.into_iter().enumerate() {
                let otp_id = entry.id.clone();
                let edit_id = entry.id.clone();
                let delete_id = entry.id.clone();
                let code_id = entry.id.clone();
                let title = if !entry.issuer.trim().is_empty() || !entry.username.trim().is_empty()
                {
                    format!(
                        "{}{}",
                        entry.issuer,
                        if entry.username.trim().is_empty() {
                            String::new()
                        } else if entry.issuer.trim().is_empty() {
                            entry.username.clone()
                        } else {
                            format!(" ({})", entry.username)
                        }
                    )
                } else {
                    compact_id(&entry.id)
                };
                let code_raw = self
                    .security_otp_codes
                    .get(&entry.id)
                    .cloned()
                    .unwrap_or_else(|| "------".to_string());
                let code_display = format_otp_code_display(&code_raw);
                let is_totp = entry.otp_type.eq_ignore_ascii_case("totp");
                let period = entry.period.max(1);
                let remaining = if is_totp {
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    period - (now % period)
                } else {
                    0
                };
                let meta = if is_totp {
                    format!(
                        "{} · {} · {}d · {remaining}s left",
                        entry.otp_type.to_uppercase(),
                        entry.algorithm,
                        entry.digits,
                    )
                } else {
                    format!(
                        "{} · {} · {}d · ctr {}",
                        entry.otp_type.to_uppercase(),
                        entry.algorithm,
                        entry.digits,
                        entry.counter,
                    )
                };
                let copy_id = entry.id.clone();
                rows = rows.child(
                    div()
                        .h(px(52.))
                        .when(index + 1 < entry_count, |this| {
                            this.border_b_1().border_color(rgb(palette.border))
                        })
                        .px_3()
                        .flex()
                        .items_center()
                        .gap_2()
                        .hover(|this| this.bg(rgb(palette.hover)))
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(1.))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .min_w_0()
                                                .flex_1()
                                                .text_xs()
                                                .font_weight(FontWeight(600.))
                                                .text_color(rgb(palette.text))
                                                .overflow_hidden()
                                                .child(truncate_preview(&title, 24)),
                                        )
                                        .child(
                                            div()
                                                .font_family(
                                                    crate::features::gpui_code_font_family(),
                                                )
                                                .text_sm()
                                                .font_weight(FontWeight(700.))
                                                .text_color(rgb(if code_raw == "------" {
                                                    palette.text_muted
                                                } else {
                                                    palette.link
                                                }))
                                                .child(code_display),
                                        )
                                        .when(is_totp && code_raw != "------", |this| {
                                            this.child(
                                                div()
                                                    .text_size(px(10.))
                                                    .font_family(
                                                        crate::features::gpui_code_font_family(),
                                                    )
                                                    .text_color(rgb(if remaining <= 5 {
                                                        palette.warning
                                                    } else {
                                                        palette.text_dimmed
                                                    }))
                                                    .child(format!("{remaining}s")),
                                            )
                                        }),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .text_color(rgb(palette.text_dimmed))
                                        .overflow_hidden()
                                        .child(meta),
                                ),
                        )
                        .child(
                            div()
                                .flex_none()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(small_button(
                                    palette,
                                    format!("security-otp-code-{otp_id}"),
                                    self.tr("otp.generateCode"),
                                    cx.listener(move |this, _, window, cx| {
                                        this.generate_security_otp_code(
                                            code_id.clone(),
                                            window,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    format!("security-otp-copy-{otp_id}"),
                                    self.tr("otp.copyCode"),
                                    cx.listener(move |this, _, window, cx| {
                                        this.copy_security_otp_code(copy_id.clone(), window, cx);
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    format!("security-otp-edit-{otp_id}"),
                                    self.tr("common.edit"),
                                    cx.listener(move |this, _, window, cx| {
                                        this.open_security_otp_editor(
                                            Some(edit_id.clone()),
                                            window,
                                            cx,
                                        );
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    format!("security-otp-del-{otp_id}"),
                                    self.tr("common.delete"),
                                    cx.listener(move |this, _, _, cx| {
                                        this.request_delete_security_otp(delete_id.clone(), cx);
                                    }),
                                )),
                        ),
                );
            }
            body = body.child(rows);
        }
        body
    }
}

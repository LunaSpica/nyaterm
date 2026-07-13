use super::*;

impl NyaTermApp {
    pub(in crate::features) fn duplicate_prompt_banner(
        &mut self,
        prompt: SftpDuplicatePromptState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let overwrite_id = prompt.id.clone();
        let skip_id = prompt.id.clone();
        let rename_id = prompt.id.clone();
        let direction = match prompt.request.direction {
            SftpTransferDirection::Download => "Download duplicate",
            SftpTransferDirection::Upload => "Upload duplicate",
        };
        let kind = if prompt.request.is_directory {
            "directory"
        } else {
            "file"
        };

        div()
            .mt_4()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.warning))
            .bg(rgb(palette.input))
            .p_3()
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child(direction),
                            )
                            .child(
                                div().text_xs().text_color(rgb(palette.text)).child(format!(
                                    "Target {kind}: {}",
                                    prompt.request.target_path
                                )),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(palette.text_muted))
                                    .child(format!("Source: {}", prompt.request.source_path)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                palette,
                                format!("duplicate-overwrite-{overwrite_id}"),
                                "Overwrite",
                                cx.listener(move |this, _, _, cx| {
                                    this.resolve_duplicate_prompt(
                                        overwrite_id.clone(),
                                        SftpDuplicateDecision::Overwrite,
                                        cx,
                                    );
                                }),
                            ))
                            .child(small_button(
                                palette,
                                format!("duplicate-skip-{skip_id}"),
                                "Skip",
                                cx.listener(move |this, _, _, cx| {
                                    this.resolve_duplicate_prompt(
                                        skip_id.clone(),
                                        SftpDuplicateDecision::Skip,
                                        cx,
                                    );
                                }),
                            ))
                            .child(small_button(
                                palette,
                                format!("duplicate-rename-{rename_id}"),
                                "Rename",
                                cx.listener(move |this, _, _, cx| {
                                    this.resolve_duplicate_prompt(
                                        rename_id.clone(),
                                        SftpDuplicateDecision::Rename,
                                        cx,
                                    );
                                }),
                            )),
                    ),
            )
    }

    pub(in crate::features) fn host_key_prompt_banner(
        &mut self,
        prompt: HostKeyPromptRequest,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let accept_id = prompt.id.clone();
        let reject_id = prompt.id.clone();
        let tone = match prompt.issue {
            HostKeyPromptIssue::Unknown => "Unknown SSH host key",
            HostKeyPromptIssue::Changed => "Changed SSH host key",
        };
        let action = match prompt.issue {
            HostKeyPromptIssue::Unknown => "Accept will add this key to known_hosts.",
            HostKeyPromptIssue::Changed => "Accept will replace the stored key for this host.",
        };

        div()
            .mx_3()
            .mt_3()
            .rounded_md()
            .border_1()
            .border_color(match prompt.issue {
                HostKeyPromptIssue::Unknown => rgb(palette.warning),
                HostKeyPromptIssue::Changed => rgb(palette.danger),
            })
            .bg(match prompt.issue {
                HostKeyPromptIssue::Unknown => rgb(palette.input),
                HostKeyPromptIssue::Changed => rgb(palette.input),
            })
            .p_3()
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_sm().font_weight(FontWeight(700.)).child(tone))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(palette.text))
                                    .child(prompt.host_key.host_identifier.clone()),
                            )
                            .child(div().text_xs().text_color(rgb(palette.text_muted)).child(
                                format!(
                                    "{} {}",
                                    prompt.host_key.key_type, prompt.host_key.fingerprint
                                ),
                            ))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(palette.text_muted))
                                    .child(action),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                palette,
                                format!("host-key-reject-{reject_id}"),
                                "Reject",
                                cx.listener(move |this, _, _, cx| {
                                    this.resolve_host_key_prompt(
                                        reject_id.clone(),
                                        HostKeyPromptChoice::Reject,
                                        cx,
                                    );
                                }),
                            ))
                            .child(small_button(
                                palette,
                                format!("host-key-accept-{accept_id}"),
                                "Accept",
                                cx.listener(move |this, _, _, cx| {
                                    this.resolve_host_key_prompt(
                                        accept_id.clone(),
                                        HostKeyPromptChoice::Accept,
                                        cx,
                                    );
                                }),
                            )),
                    ),
            )
    }

    pub(in crate::features) fn credential_prompt_banner(
        &mut self,
        prompt: CredentialPromptState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let title = match prompt.prompt.kind {
            SshCredentialPromptKind::Password => "SSH Password",
            SshCredentialPromptKind::KeyPassphrase => "SSH Key Passphrase",
            SshCredentialPromptKind::KeyboardInteractive => "SSH Verification",
        };
        let reason = match prompt.prompt.reason {
            SshCredentialPromptReason::MissingPassword => "Password is required.",
            SshCredentialPromptReason::PasswordRejected => "Previous password was rejected.",
            SshCredentialPromptReason::KeyPassphraseRequired => {
                "Passphrase is required to unlock the key."
            }
            SshCredentialPromptReason::KeyboardInteractive => {
                "Server requested keyboard-interactive verification."
            }
        };
        let display_value = if prompt.value.is_empty() {
            " ".to_string()
        } else if prompt.prompt.echo {
            prompt.value.clone()
        } else {
            "*".repeat(prompt.value.chars().count())
        };
        let mut details = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().font_weight(FontWeight(700.)).child(title))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(palette.text))
                    .child(credential_prompt_target(&prompt.prompt)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .child(reason),
            );
        if let Some(prompt_text) = prompt
            .prompt
            .prompt_text
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            details = details.child(
                div()
                    .mt_1()
                    .text_xs()
                    .text_color(rgb(palette.text))
                    .child(prompt_text.to_string()),
            );
        }

        div()
            .mx_3()
            .mt_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.accent))
            .bg(rgb(palette.input))
            .p_3()
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_3()
                    .child(details)
                    .child(
                        div()
                            .id(SharedString::from(format!(
                                "credential-input-{}",
                                prompt.id
                            )))
                            .w(px(240.))
                            .h(px(32.))
                            .px_3()
                            .flex()
                            .items_center()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(palette.accent))
                            .bg(rgb(palette.bg))
                            .font_family("JetBrains Mono")
                            .text_sm()
                            .track_focus(&self.credential_focus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                window.focus(&this.credential_focus);
                                this.terminal_status = "credential prompt focused".to_string();
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                cx.stop_propagation();
                                this.handle_credential_key_down(event, cx);
                            }))
                            .child(display_value),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                palette,
                                format!("credential-cancel-{}", prompt.id),
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_credential_prompt(cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                format!("credential-submit-{}", prompt.id),
                                "Submit",
                                cx.listener(|this, _, _, cx| {
                                    this.submit_credential_prompt(cx);
                                }),
                            )),
                    ),
            )
    }

    pub(in crate::features) fn snapshot_password_prompt_banner(
        &mut self,
        prompt: SnapshotPasswordPromptState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let title = match prompt.kind {
            SnapshotPasswordPromptKind::Export => "Encrypted Snapshot Export",
            SnapshotPasswordPromptKind::Import => "Encrypted Snapshot Import",
            SnapshotPasswordPromptKind::CloudPush => "Cloud Sync Push",
            SnapshotPasswordPromptKind::CloudPull => "Cloud Sync Pull",
            SnapshotPasswordPromptKind::CloudForcePush => "Force Cloud Sync Push",
            SnapshotPasswordPromptKind::CloudForcePull => "Force Cloud Sync Pull",
            SnapshotPasswordPromptKind::CloudProviderPush => "Provider Sync Push",
            SnapshotPasswordPromptKind::CloudProviderPull => "Provider Sync Pull",
            SnapshotPasswordPromptKind::CloudProviderForcePush => "Force Provider Sync Push",
            SnapshotPasswordPromptKind::CloudProviderForcePull => "Force Provider Sync Pull",
        };
        let masked = if prompt.value.is_empty() {
            " ".to_string()
        } else {
            "*".repeat(prompt.value.chars().count())
        };

        div()
            .mt_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.accent))
            .bg(rgb(palette.input))
            .p_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_sm().font_weight(FontWeight(700.)).child(title))
                            .child(div().text_xs().text_color(rgb(palette.text_muted)).child(
                                match prompt.kind {
                                    SnapshotPasswordPromptKind::CloudPush
                                    | SnapshotPasswordPromptKind::CloudPull
                                    | SnapshotPasswordPromptKind::CloudForcePush
                                    | SnapshotPasswordPromptKind::CloudForcePull
                                    | SnapshotPasswordPromptKind::CloudProviderPush
                                    | SnapshotPasswordPromptKind::CloudProviderPull
                                    | SnapshotPasswordPromptKind::CloudProviderForcePush
                                    | SnapshotPasswordPromptKind::CloudProviderForcePull => {
                                        "Password encrypts or decrypts this cloud snapshot."
                                    }
                                    _ => "Password is used only for this .nya operation.",
                                },
                            )),
                    )
                    .child(
                        div()
                            .id(SharedString::from("snapshot-password-input"))
                            .w(px(240.))
                            .h(px(32.))
                            .px_3()
                            .flex()
                            .items_center()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(palette.accent))
                            .bg(rgb(palette.bg))
                            .font_family("JetBrains Mono")
                            .text_sm()
                            .track_focus(&self.snapshot_password_focus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                window.focus(&this.snapshot_password_focus);
                                this.terminal_status =
                                    "snapshot password prompt focused".to_string();
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                cx.stop_propagation();
                                this.handle_snapshot_password_key_down(event, cx);
                            }))
                            .child(masked),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "snapshot-password-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_snapshot_password_prompt(cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "snapshot-password-submit",
                                "Submit",
                                cx.listener(|this, _, _, cx| {
                                    this.submit_snapshot_password_prompt(cx);
                                }),
                            )),
                    ),
            )
    }
}

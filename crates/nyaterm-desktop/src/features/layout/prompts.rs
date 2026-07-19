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
        let escape_id = prompt.id.clone();
        let rename_id = prompt.id.clone();
        let kind = if prompt.request.is_directory {
            self.tr("fileTransfer.duplicateKindFolder")
        } else {
            self.tr("fileTransfer.duplicateKindFile")
        };
        let target_name = download_file_name_from_remote_path(&prompt.request.target_path);
        let description = self
            .tr("fileTransfer.duplicateDescription")
            .replace("{{kind}}", kind)
            .replace("{{name}}", &target_name);

        div()
            .id("duplicate-prompt-overlay")
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .p_3()
            .track_focus(&self.transfer_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.transfer_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key.as_str() == "escape" {
                    this.resolve_duplicate_prompt(
                        escape_id.clone(),
                        SftpDuplicateDecision::Skip,
                        cx,
                    );
                }
            }))
            .child(
                div()
                    .id("duplicate-prompt-dialog")
                    .w(px((self.last_viewport_size.0 - 32.).min(448.).max(280.)))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child(self.tr("fileTransfer.duplicateTitle")),
                    )
                    .child(
                        div()
                            .text_xs()
                            .line_height(px(17.))
                            .text_color(rgb(palette.text_muted))
                            .child(description),
                    )
                    .child(
                        div()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .px_2()
                            .py_1()
                            .font_family(crate::features::gpui_code_font_family())
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .child(prompt.request.target_path.clone()),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .justify_end()
                            .gap_2()
                            .child(small_button(
                                palette,
                                format!("duplicate-overwrite-{overwrite_id}"),
                                self.tr("fileTransfer.duplicateOverwrite"),
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
                                self.tr("fileTransfer.duplicateSkip"),
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
                                self.tr("common.rename"),
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
        let changed = matches!(prompt.issue, HostKeyPromptIssue::Changed);
        let description = match prompt.issue {
            HostKeyPromptIssue::Unknown => self.tr("settings.hostKeyVerifyNew"),
            HostKeyPromptIssue::Changed => self.tr("settings.hostKeyVerifyChanged"),
        };
        let detail_row = |label: &'static str, value: String| {
            div()
                .flex()
                .items_start()
                .gap_3()
                .text_xs()
                .child(
                    div()
                        .w(px(88.))
                        .flex_none()
                        .text_color(rgb(palette.text_muted))
                        .child(label),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .font_family(crate::features::gpui_code_font_family())
                        .text_color(rgb(palette.text))
                        .child(value),
                )
        };

        div()
            .w_full()
            .max_w(px(384.))
            .mx_auto()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(self.shell_surface_color(palette.bg))
            .shadow_lg()
            .p_6()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("settings.hostKeyVerifyTitle")),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(description),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(detail_row(
                        self.tr("settings.hostKeyVerifyHost"),
                        prompt.host_key.host_identifier.clone(),
                    ))
                    .child(detail_row(
                        self.tr("settings.hostKeyVerifyKeyType"),
                        prompt.host_key.key_type.clone(),
                    ))
                    .child(detail_row(
                        self.tr("settings.hostKeyVerifyFingerprint"),
                        prompt.host_key.fingerprint.clone(),
                    )),
            )
            .when(changed, |this| {
                this.child(
                    div()
                        .rounded_sm()
                        .border_1()
                        .border_color(rgba((palette.danger << 8) | 0x80))
                        .bg(rgba((palette.danger << 8) | 0x1a))
                        .p_2()
                        .text_size(px(11.))
                        .text_color(rgb(palette.danger))
                        .child(self.tr("settings.hostKeyVerifyWarning")),
                )
            })
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .child(small_button(
                        palette,
                        format!("host-key-reject-{reject_id}"),
                        self.tr("settings.hostKeyVerifyReject"),
                        cx.listener(move |this, _, _, cx| {
                            this.resolve_host_key_prompt(
                                reject_id.clone(),
                                HostKeyPromptChoice::Reject,
                                cx,
                            );
                        }),
                    ))
                    .child(dialog_action_button(
                        palette,
                        format!("host-key-accept-{accept_id}"),
                        self.tr("settings.hostKeyVerifyAccept"),
                        changed,
                        cx.listener(move |this, _, _, cx| {
                            this.resolve_host_key_prompt(
                                accept_id.clone(),
                                HostKeyPromptChoice::Accept,
                                cx,
                            );
                        }),
                    )),
            )
    }

    pub(in crate::features) fn credential_prompt_banner(
        &mut self,
        prompt: CredentialPromptState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let title = match prompt.prompt.kind {
            SshCredentialPromptKind::Password => self.tr("runtimePrompt.sshPassword"),
            SshCredentialPromptKind::KeyPassphrase => self.tr("runtimePrompt.sshKeyPassphrase"),
            SshCredentialPromptKind::KeyboardInteractive => {
                self.tr("runtimePrompt.sshVerification")
            }
        };
        let reason = match prompt.prompt.reason {
            SshCredentialPromptReason::MissingPassword => self.tr("sshAuth.missingPassword"),
            SshCredentialPromptReason::PasswordRejected => self.tr("sshAuth.passwordRejected"),
            SshCredentialPromptReason::KeyPassphraseRequired => {
                self.tr("sshAuth.keyPassphraseRequired")
            }
            SshCredentialPromptReason::KeyboardInteractive => {
                self.tr("runtimePrompt.keyboardInteractive")
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
            .w_full()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(self.shell_surface_color(palette.bg))
            .shadow_lg()
            .p_6()
            .flex()
            .flex_col()
            .gap_4()
            .child(details)
            .child(
                div()
                    .id(SharedString::from(format!("credential-input-{}", prompt.id)))
                    .w_full()
                    .h(px(36.))
                    .px_3()
                    .flex()
                    .items_center()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.input))
                    .font_family(crate::features::gpui_code_font_family())
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
                    .justify_end()
                    .gap_2()
                    .child(small_button(
                        palette,
                        format!("credential-cancel-{}", prompt.id),
                        self.tr("common.cancel"),
                        cx.listener(|this, _, _, cx| {
                            this.cancel_credential_prompt(cx);
                        }),
                    ))
                    .child(dialog_action_button(
                        palette,
                        format!("credential-submit-{}", prompt.id),
                        self.tr("sshAuth.submit"),
                        false,
                        cx.listener(|this, _, _, cx| {
                            this.submit_credential_prompt(cx);
                        }),
                    )),
            )
    }

    pub(in crate::features) fn snapshot_password_prompt_banner(
        &mut self,
        prompt: SnapshotPasswordPromptState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let title = match prompt.kind {
            SnapshotPasswordPromptKind::Export => self.tr("runtimePrompt.snapshotExport"),
            SnapshotPasswordPromptKind::Import => self.tr("runtimePrompt.snapshotImport"),
            SnapshotPasswordPromptKind::CloudPush => self.tr("runtimePrompt.cloudPush"),
            SnapshotPasswordPromptKind::CloudPull => self.tr("runtimePrompt.cloudPull"),
            SnapshotPasswordPromptKind::CloudForcePush => self.tr("runtimePrompt.cloudForcePush"),
            SnapshotPasswordPromptKind::CloudForcePull => self.tr("runtimePrompt.cloudForcePull"),
            SnapshotPasswordPromptKind::CloudProviderPush => {
                self.tr("runtimePrompt.cloudProviderPush")
            }
            SnapshotPasswordPromptKind::CloudProviderPull => {
                self.tr("runtimePrompt.cloudProviderPull")
            }
            SnapshotPasswordPromptKind::CloudProviderForcePush => {
                self.tr("runtimePrompt.cloudProviderForcePush")
            }
            SnapshotPasswordPromptKind::CloudProviderForcePull => {
                self.tr("runtimePrompt.cloudProviderForcePull")
            }
        };
        let description = match prompt.kind {
            SnapshotPasswordPromptKind::CloudPush
            | SnapshotPasswordPromptKind::CloudPull
            | SnapshotPasswordPromptKind::CloudForcePush
            | SnapshotPasswordPromptKind::CloudForcePull
            | SnapshotPasswordPromptKind::CloudProviderPush
            | SnapshotPasswordPromptKind::CloudProviderPull
            | SnapshotPasswordPromptKind::CloudProviderForcePush
            | SnapshotPasswordPromptKind::CloudProviderForcePull => {
                self.tr("runtimePrompt.cloudSnapshotDescription")
            }
            _ => self.tr("runtimePrompt.localSnapshotDescription"),
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
            .border_color(rgb(palette.link))
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
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(palette.text_muted))
                                    .child(description),
                            ),
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
                            .border_color(rgb(palette.link))
                            .bg(rgb(palette.bg))
                            .font_family(crate::features::gpui_code_font_family())
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
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_snapshot_password_prompt(cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "snapshot-password-submit",
                                self.tr("runtimePrompt.submit"),
                                cx.listener(|this, _, _, cx| {
                                    this.submit_snapshot_password_prompt(cx);
                                }),
                            )),
                    ),
            )
    }
}

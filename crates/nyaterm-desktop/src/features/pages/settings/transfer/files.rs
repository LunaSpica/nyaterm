use super::*;

impl NyaTermApp {
    pub(in crate::features) fn transfer_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        // Tauri TransferTab top: path/duplicate switches + queue summary (dense rows).
        let running = self
            .transfer_jobs
            .iter()
            .filter(|job| {
                matches!(
                    job.status,
                    TransferJobStatus::Running
                        | TransferJobStatus::Paused
                        | TransferJobStatus::Cancelling
                )
            })
            .count();
        let completed = self
            .transfer_jobs
            .iter()
            .filter(|job| job.status == TransferJobStatus::Completed)
            .count();
        let failed = self
            .transfer_jobs
            .iter()
            .filter(|job| job.status == TransferJobStatus::Failed)
            .count();
        let download_path = if self.settings.transfer_download_path.trim().is_empty() {
            "default download dir".to_string()
        } else {
            truncate_preview(&self.settings.transfer_download_path, 42)
        };
        let policy = self.transfer_duplicate_policy;

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                palette,
                Some("Files"),
                Some("Download location, save prompts, and duplicate strategy."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        "Download path",
                        Some(SharedString::from(download_path)),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
                                palette,
                                "transfer-browse-download",
                                "Browse",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_transfer_download_path_setting(cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "transfer-use-local-draft",
                                "Use Draft",
                                cx.listener(|this, _, _, cx| {
                                    let local_draft = this.transfer_local_path.clone();
                                    if local_draft.trim().is_empty() {
                                        this.terminal_status =
                                            "set a local path draft in Files first".to_string();
                                        cx.notify();
                                        return;
                                    }
                                    this.settings.transfer_download_path = local_draft;
                                    this.save_transfer_settings("transfer download path saved", cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "transfer-clear-download",
                                "Clear",
                                cx.listener(|this, _, _, cx| {
                                    this.settings.transfer_download_path.clear();
                                    this.save_transfer_settings(
                                        "transfer download path cleared",
                                        cx,
                                    );
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        palette,
                        "Ask save location",
                        Some(SharedString::from(
                            "Prompt for a folder before each download when enabled.",
                        )),
                        settings_switch(
                            palette,
                            "transfer-ask-save",
                            self.settings.transfer_ask_save_location,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_transfer_ask_save_location(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some("Duplicate files"),
                Some("What to do when a remote or local file already exists."),
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(settings_choice_chip(
                        palette,
                        "transfer-dup-ask",
                        "Ask",
                        policy == SftpDuplicatePolicy::Ask,
                        cx.listener(|this, _, _, cx| {
                            this.update_transfer_duplicate_policy(SftpDuplicatePolicy::Ask, cx);
                        }),
                    ))
                    .child(settings_choice_chip(
                        palette,
                        "transfer-dup-overwrite",
                        "Overwrite",
                        policy == SftpDuplicatePolicy::Overwrite,
                        cx.listener(|this, _, _, cx| {
                            this.update_transfer_duplicate_policy(
                                SftpDuplicatePolicy::Overwrite,
                                cx,
                            );
                        }),
                    ))
                    .child(settings_choice_chip(
                        palette,
                        "transfer-dup-skip",
                        "Skip",
                        policy == SftpDuplicatePolicy::Skip,
                        cx.listener(|this, _, _, cx| {
                            this.update_transfer_duplicate_policy(SftpDuplicatePolicy::Skip, cx);
                        }),
                    ))
                    .child(settings_choice_chip(
                        palette,
                        "transfer-dup-rename",
                        "Rename",
                        policy == SftpDuplicatePolicy::Rename,
                        cx.listener(|this, _, _, cx| {
                            this.update_transfer_duplicate_policy(SftpDuplicatePolicy::Rename, cx);
                        }),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some("Queue snapshot"),
                None,
                settings_form_row(
                    palette,
                    "Jobs",
                    Some(SharedString::from(format!(
                        "{running} running · {completed} done · {failed} failed · {} total",
                        self.transfer_jobs.len()
                    ))),
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette.text_muted))
                        .child("Live queue"),
                ),
            ))
            .child(self.transfer_editor_settings_section(cx))
            .child(self.transfer_advanced_settings_section(cx))
            .child(self.recording_settings_section(cx))
    }
}

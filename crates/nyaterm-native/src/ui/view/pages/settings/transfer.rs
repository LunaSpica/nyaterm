use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn transfer_settings_section(
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
            .child(settings_form_section(palette, 
                Some("Paths"),
                Some("Default download location and save prompts."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Download path",
                        Some(SharedString::from(download_path)),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
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
                                "transfer-clear-download",
                                "Clear",
                                cx.listener(|this, _, _, cx| {
                                    this.settings.transfer_download_path.clear();
                                    this.save_transfer_settings("transfer download path cleared", cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
                        "Ask save location",
                        Some(SharedString::from(
                            "Prompt for a folder before each download when enabled.",
                        )),
                        settings_switch(palette, 
                            "transfer-ask-save",
                            self.settings.transfer_ask_save_location,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_transfer_ask_save_location(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(palette, 
                Some("Duplicate files"),
                Some("What to do when a remote or local file already exists."),
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(settings_choice_chip(palette, 
                        "transfer-dup-ask",
                        "Ask",
                        policy == SftpDuplicatePolicy::Ask,
                        cx.listener(|this, _, _, cx| {
                            this.update_transfer_duplicate_policy(SftpDuplicatePolicy::Ask, cx);
                        }),
                    ))
                    .child(settings_choice_chip(palette, 
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
                    .child(settings_choice_chip(palette, 
                        "transfer-dup-skip",
                        "Skip",
                        policy == SftpDuplicatePolicy::Skip,
                        cx.listener(|this, _, _, cx| {
                            this.update_transfer_duplicate_policy(SftpDuplicatePolicy::Skip, cx);
                        }),
                    ))
                    .child(settings_choice_chip(palette, 
                        "transfer-dup-rename",
                        "Rename",
                        policy == SftpDuplicatePolicy::Rename,
                        cx.listener(|this, _, _, cx| {
                            this.update_transfer_duplicate_policy(SftpDuplicatePolicy::Rename, cx);
                        }),
                    )),
            ))
            .child(settings_form_section(palette, 
                Some("Queue snapshot"),
                None,
                settings_form_row(palette, 
                    "Jobs",
                    Some(SharedString::from(format!(
                        "{running} running · {completed} done · {failed} failed · {} total",
                        self.transfer_jobs.len()
                    ))),
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(0x8b949e))
                        .child("Live queue"),
                ),
            ))
            .child(self.transfer_editor_settings_section(cx))
            .child(self.transfer_advanced_settings_section(cx))
            .child(self.recording_settings_section(cx))
    }

    fn transfer_editor_settings_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let editor_type = self.settings.transfer_editor_type.clone();
        let default_editor_value = if self.settings.transfer_default_editor.is_empty() {
            " ".to_string()
        } else {
            self.settings.transfer_default_editor.clone()
        };
        let external_cmd = if self.settings.transfer_default_editor.trim().is_empty() {
            "system default".to_string()
        } else {
            truncate_preview(&self.settings.transfer_default_editor, 34)
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(palette, 
                Some("Editor"),
                Some("How remote files open from the transfer browser."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Default open",
                        Some(SharedString::from(transfer_editor_type_status(&editor_type))),
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(palette, 
                                "settings-transfer-editor-external",
                                "External",
                                editor_type == "external",
                                cx.listener(|this, _, _, cx| {
                                    this.update_transfer_editor_type("external", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "settings-transfer-editor-internal",
                                "Internal",
                                editor_type == "internal",
                                cx.listener(|this, _, _, cx| {
                                    this.update_transfer_editor_type("internal", cx);
                                }),
                            )),
                    ))
                    .when(editor_type == "external", |this| {
                        this.child(settings_form_row(palette, 
                            "External command",
                            Some(SharedString::from(external_cmd)),
                            transfer_input(
                                "settings-transfer-default-editor",
                                "Default editor command",
                                default_editor_value,
                                false,
            self.theme_palette(),
        )
                            .track_focus(&self.transfer_default_editor_focus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                window.focus(&this.transfer_default_editor_focus);
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(
                                |this, event: &KeyDownEvent, _, cx| {
                                    cx.stop_propagation();
                                    this.handle_transfer_default_editor_key_down(event, cx);
                                },
                            )),
                        ))
                    })
                    .child(settings_form_row(palette, 
                        "Actions",
                        None,
                        small_button(
                            "settings-transfer-editor-save",
                            "Save",
                            cx.listener(|this, _, _, cx| {
                                this.save_transfer_settings(
                                    "transfer editor settings saved",
                                    cx,
                                );
                            }),
                        ),
                    )),
            ))
    }

    fn transfer_advanced_settings_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let permissions = self.settings.transfer_default_file_permissions.clone();

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(palette, 
                Some("Advanced transfer"),
                Some("Concurrency, buffer size, timestamps, and default permissions."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Download threads",
                        Some(SharedString::from(format!(
                            "{} parallel downloads",
                            self.settings.transfer_download_threads
                        ))),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
                                "settings-transfer-download-threads-dec",
                                "-",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_transfer_download_threads(-1, cx);
                                }),
                            ))
                            .child(
                                div()
                                    .min_w(px(28.))
                                    .text_center()
                                    .font_family("JetBrains Mono")
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(700.))
                                    .text_color(rgb(0xc9d1d9))
                                    .child(self.settings.transfer_download_threads.to_string()),
                            )
                            .child(small_button(
                                "settings-transfer-download-threads-inc",
                                "+",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_transfer_download_threads(1, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
                        "Upload threads",
                        Some(SharedString::from(format!(
                            "{} parallel uploads",
                            self.settings.transfer_upload_threads
                        ))),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
                                "settings-transfer-upload-threads-dec",
                                "-",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_transfer_upload_threads(-1, cx);
                                }),
                            ))
                            .child(
                                div()
                                    .min_w(px(28.))
                                    .text_center()
                                    .font_family("JetBrains Mono")
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(700.))
                                    .text_color(rgb(0xc9d1d9))
                                    .child(self.settings.transfer_upload_threads.to_string()),
                            )
                            .child(small_button(
                                "settings-transfer-upload-threads-inc",
                                "+",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_transfer_upload_threads(1, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
                        "Max retries",
                        Some(SharedString::from(format!(
                            "{} retries",
                            self.settings.transfer_max_retries
                        ))),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
                                "settings-transfer-retries-dec",
                                "-",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_transfer_max_retries(-1, cx);
                                }),
                            ))
                            .child(
                                div()
                                    .min_w(px(28.))
                                    .text_center()
                                    .font_family("JetBrains Mono")
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(700.))
                                    .text_color(rgb(0xc9d1d9))
                                    .child(self.settings.transfer_max_retries.to_string()),
                            )
                            .child(small_button(
                                "settings-transfer-retries-inc",
                                "+",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_transfer_max_retries(1, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
                        "Buffer size",
                        Some(SharedString::from(format!(
                            "{} KiB",
                            self.settings.transfer_buffer_size
                        ))),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
                                "settings-transfer-buffer-dec",
                                "-",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_transfer_buffer_size(-1, cx);
                                }),
                            ))
                            .child(
                                div()
                                    .min_w(px(36.))
                                    .text_center()
                                    .font_family("JetBrains Mono")
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(700.))
                                    .text_color(rgb(0xc9d1d9))
                                    .child(self.settings.transfer_buffer_size.to_string()),
                            )
                            .child(small_button(
                                "settings-transfer-buffer-inc",
                                "+",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_transfer_buffer_size(1, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
                        "Preserve timestamps",
                        Some(SharedString::from(
                            "Keep source mtime when downloading when possible.",
                        )),
                        settings_switch(palette, 
                            "settings-transfer-preserve-timestamps",
                            self.settings.transfer_preserve_timestamps,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_transfer_preserve_timestamps(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
                        "Resume broken",
                        Some(SharedString::from(
                            "Attempt to resume interrupted downloads.",
                        )),
                        settings_switch(palette, 
                            "settings-transfer-resume-broken",
                            self.settings.transfer_resume_broken_transfer,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_transfer_resume_broken(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
                        "Default permissions",
                        Some(SharedString::from(format!("chmod {permissions}"))),
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(palette, 
                                "settings-transfer-perm-600",
                                "600",
                                permissions == "600",
                                cx.listener(|this, _, _, cx| {
                                    this.update_transfer_file_permissions("600", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "settings-transfer-perm-644",
                                "644",
                                permissions == "644",
                                cx.listener(|this, _, _, cx| {
                                    this.update_transfer_file_permissions("644", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "settings-transfer-perm-664",
                                "664",
                                permissions == "664",
                                cx.listener(|this, _, _, cx| {
                                    this.update_transfer_file_permissions("664", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette, 
                                "settings-transfer-perm-755",
                                "755",
                                permissions == "755",
                                cx.listener(|this, _, _, cx| {
                                    this.update_transfer_file_permissions("755", cx);
                                }),
                            )),
                    )),
            ))
    }

    pub(in crate::ui::view) fn recording_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let path = if self.settings.recording_path.trim().is_empty() {
            self.runtime
                .config_dir()
                .join("recordings")
                .display()
                .to_string()
        } else {
            self.settings.recording_path.clone()
        };
        let memory_mib = (self.settings.recording_memory_limit_bytes / (1024 * 1024)).max(1);

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(palette, 
                Some("Recording"),
                Some("Session capture path, memory cap, and stream annotations."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette, 
                        "Path",
                        Some(SharedString::from(truncate_preview(&path, 56))),
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0x8b949e))
                            .child("On disk"),
                    ))
                    .child(settings_form_row(palette, 
                        "Memory limit",
                        Some(SharedString::from(format!("{memory_mib} MiB buffer"))),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
                                "settings-recording-memory-minus",
                                "-1 MiB",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_recording_memory_limit(-1, cx);
                                }),
                            ))
                            .child(
                                div()
                                    .min_w(px(42.))
                                    .text_center()
                                    .font_family("JetBrains Mono")
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(700.))
                                    .text_color(rgb(0xc9d1d9))
                                    .child(format!("{memory_mib}")),
                            )
                            .child(small_button(
                                "settings-recording-memory-plus",
                                "+1 MiB",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_recording_memory_limit(1, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(palette, 
                        "Auto start",
                        Some(SharedString::from(
                            "Begin recording when a session connects.",
                        )),
                        settings_switch(palette, 
                            "settings-recording-auto",
                            self.settings.recording_auto_start,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_recording_auto_start(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
                        "IO labels",
                        Some(SharedString::from(
                            "Annotate recorded streams with in/out labels.",
                        )),
                        settings_switch(palette, 
                            "settings-recording-labels",
                            self.settings.recording_include_io_labels,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_recording_io_labels(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(palette, 
                        "Timestamps",
                        Some(SharedString::from(
                            "Prefix recorded chunks with wall-clock timestamps.",
                        )),
                        settings_switch(palette, 
                            "settings-recording-timestamps",
                            self.settings.recording_include_timestamps,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_recording_timestamps(cx);
                            }),
                        ),
                    )),
            ))
    }
}

fn transfer_path_prompt_label(kind: TransferPathPromptKind) -> &'static str {
    match kind {
        TransferPathPromptKind::UploadFile => "upload file",
        TransferPathPromptKind::UploadDirectory => "upload directory",
        TransferPathPromptKind::DownloadDirectory => "download directory",
    }
}

fn transfer_duplicate_policy_status(policy: SftpDuplicatePolicy) -> &'static str {
    match policy {
        SftpDuplicatePolicy::Ask => "ask",
        SftpDuplicatePolicy::Overwrite => "overwrite",
        SftpDuplicatePolicy::Skip => "skip",
        SftpDuplicatePolicy::Rename => "rename",
    }
}

fn transfer_editor_type_status(editor_type: &str) -> &'static str {
    match editor_type {
        "internal" => "native text editor",
        _ => "external app",
    }
}

fn transfer_stepper(
    label: &'static str,
    value: u32,
    dec_id: &'static str,
    inc_id: &'static str,
    on_dec: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_inc: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x263142))
        .bg(rgb(0x0d1320))
        .p_3()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(10.))
                .font_weight(FontWeight(800.))
                .text_color(rgb(0x98a3b8))
                .child(label),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(small_button(dec_id, "-", on_dec))
                .child(
                    div()
                        .min_w(px(38.))
                        .text_center()
                        .font_family("JetBrains Mono")
                        .text_sm()
                        .font_weight(FontWeight(800.))
                        .text_color(rgb(0xe5edf7))
                        .child(value.to_string()),
                )
                .child(small_button(inc_id, "+", on_inc)),
        )
}

fn permission_preset_button(
    id: &'static str,
    label: &'static str,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id))
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(if active { rgb(0x256d3f) } else { rgb(0x303848) })
        .bg(if active { rgb(0x17253b) } else { rgb(0x151b27) })
        .cursor_pointer()
        .text_xs()
        .font_weight(FontWeight(800.))
        .text_color(if active { rgb(0xdbeafe) } else { rgb(0xaeb7c8) })
        .hover(|this| this.bg(rgb(0x1b2535)))
        .on_click(on_click)
        .child(label)
}

fn transfer_capability_card(title: &'static str, detail: &'static str) -> impl IntoElement {
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

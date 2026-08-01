use gpui::{Context, IntoElement, SharedString, div, prelude::*};
use nyaterm_ui::NyaNumberInputOptions;

use crate::features::NyaTermApp;

use super::super::{
    settings_choice_chip, settings_form_row, settings_form_section, settings_switch,
};

impl NyaTermApp {
    pub(in crate::features::pages::settings) fn transfer_advanced_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let permissions = self
            .settings
            .summary()
            .transfer_default_file_permissions
            .clone();

        div().flex().flex_col().gap_3().child(settings_form_section(
            palette,
            None,
            None,
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(settings_form_row(
                    palette,
                    self.tr("settings.downloadConcurrentTasks"),
                    Some(SharedString::from(
                        self.tr("settings.downloadConcurrentTasksDesc"),
                    )),
                    self.number_input_box(
                        "settings.number.transfer-download-threads",
                        self.settings
                            .summary()
                            .transfer_download_threads
                            .to_string()
                            .as_str(),
                        NyaNumberInputOptions::default().range(1.0, 10.0).step(1.0),
                        cx,
                    ),
                ))
                .child(settings_form_row(
                    palette,
                    self.tr("settings.uploadConcurrentTasks"),
                    Some(SharedString::from(
                        self.tr("settings.uploadConcurrentTasksDesc"),
                    )),
                    self.number_input_box(
                        "settings.number.transfer-upload-threads",
                        self.settings
                            .summary()
                            .transfer_upload_threads
                            .to_string()
                            .as_str(),
                        NyaNumberInputOptions::default().range(1.0, 10.0).step(1.0),
                        cx,
                    ),
                ))
                .child(settings_form_row(
                    palette,
                    self.tr("settings.maxTransferRetries"),
                    Some(SharedString::from(
                        self.tr("settings.maxTransferRetriesDesc"),
                    )),
                    self.number_input_box(
                        "settings.number.transfer-max-retries",
                        self.settings
                            .summary()
                            .transfer_max_retries
                            .to_string()
                            .as_str(),
                        NyaNumberInputOptions::default().range(0.0, 10.0).step(1.0),
                        cx,
                    ),
                ))
                .child(settings_form_row(
                    palette,
                    self.tr("settings.transferBufferSize"),
                    Some(SharedString::from(
                        self.tr("settings.transferBufferSizeDesc"),
                    )),
                    self.number_input_box(
                        "settings.number.transfer-buffer-size",
                        self.settings
                            .summary()
                            .transfer_buffer_size
                            .to_string()
                            .as_str(),
                        NyaNumberInputOptions::default().range(8.0, 256.0).step(8.0),
                        cx,
                    ),
                ))
                .child(settings_form_row(
                    palette,
                    self.tr("settings.preserveTimestamps"),
                    Some(SharedString::from(
                        self.tr("settings.preserveTimestampsDesc"),
                    )),
                    settings_switch(
                        palette,
                        "settings-transfer-preserve-timestamps",
                        self.settings.summary().transfer_preserve_timestamps,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_transfer_preserve_timestamps(cx);
                        }),
                    ),
                ))
                .child(settings_form_row(
                    palette,
                    self.tr("settings.resumeBrokenTransfer"),
                    Some(SharedString::from(
                        self.tr("settings.resumeBrokenTransferDesc"),
                    )),
                    settings_switch(
                        palette,
                        "settings-transfer-resume-broken",
                        self.settings.summary().transfer_resume_broken_transfer,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_transfer_resume_broken(cx);
                        }),
                    ),
                ))
                .child(settings_form_row(
                    palette,
                    self.tr("settings.defaultFilePermissions"),
                    Some(SharedString::from(
                        self.tr("settings.defaultFilePermissionsDesc"),
                    )),
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_1()
                        .child(settings_choice_chip(
                            palette,
                            "settings-transfer-perm-600",
                            "600",
                            permissions == "600",
                            cx.listener(|this, _, _, cx| {
                                this.update_transfer_file_permissions("600", cx);
                            }),
                        ))
                        .child(settings_choice_chip(
                            palette,
                            "settings-transfer-perm-644",
                            "644",
                            permissions == "644",
                            cx.listener(|this, _, _, cx| {
                                this.update_transfer_file_permissions("644", cx);
                            }),
                        ))
                        .child(settings_choice_chip(
                            palette,
                            "settings-transfer-perm-664",
                            "664",
                            permissions == "664",
                            cx.listener(|this, _, _, cx| {
                                this.update_transfer_file_permissions("664", cx);
                            }),
                        ))
                        .child(settings_choice_chip(
                            palette,
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
}

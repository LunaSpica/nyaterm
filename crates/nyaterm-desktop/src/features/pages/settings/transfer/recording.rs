use gpui::{Context, IntoElement, SharedString, div, prelude::*, px};
use nyaterm_core::{ExistingFileBehavior, RecordingMode, RecordingRotationPolicy};
use nyaterm_ui::{NyaNumberInputOptions, NyaSelectOption};

use crate::features::{NyaTermApp, TextInputSetup};
use crate::widgets::small_button;

use super::super::{
    settings_form_row, settings_form_section, settings_input_action_control, settings_switch,
};

impl NyaTermApp {
    pub(in crate::features) fn recording_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        // Built before the form, which reads `self` throughout: creating the
        // box needs it mutably.
        let recording_path_input = self
            .text_input_box(
                "settings.recording.path",
                &self.settings.summary().recording_path.clone(),
                TextInputSetup::placeholder(self.tr("settings.recordingPath")),
                cx,
            )
            .into_any_element();
        let recording_template_input = self
            .text_input_box(
                "settings.recording.path-template",
                &self.settings.summary().recording_path_template.clone(),
                TextInputSetup::placeholder(nyaterm_core::DEFAULT_RECORDING_PATH_TEMPLATE),
                cx,
            )
            .into_any_element();
        let memory_mib =
            (self.settings.summary().recording_memory_limit_bytes / (1024 * 1024)).max(1);
        let rotation_value = match self.settings.summary().recording_rotation {
            RecordingRotationPolicy::Daily => "daily",
            RecordingRotationPolicy::Size { .. } => "size",
            RecordingRotationPolicy::Session => "session",
        };
        let rotation_size_mib = match self.settings.summary().recording_rotation {
            RecordingRotationPolicy::Size { max_bytes } => (max_bytes / (1024 * 1024)).max(1),
            _ => 50,
        };
        let mode_value = match self.settings.summary().recording_default_mode {
            RecordingMode::Raw => "raw",
            RecordingMode::Transcript => "transcript",
        };
        let existing_value = match self.settings.summary().recording_existing_file_behavior {
            ExistingFileBehavior::Append => "append",
            ExistingFileBehavior::Overwrite => "overwrite",
            ExistingFileBehavior::Unique => "unique",
        };

        div().flex().flex_col().gap_3().child(settings_form_section(
            palette,
            Some(self.tr("settings.recordingSettings")),
            Some(self.tr("settings.recordingSettingsDesc")),
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(settings_form_row(
                    palette,
                    self.tr("settings.recordingDefaultMode"),
                    Some(SharedString::from(
                        self.tr("settings.recordingDefaultModeDesc"),
                    )),
                    self.settings_select_control(
                        "settings.recording.default-mode",
                        vec![
                            NyaSelectOption::new(
                                "transcript",
                                self.tr("settings.recordingModeTranscript"),
                            ),
                            NyaSelectOption::new("raw", self.tr("settings.recordingModeRaw")),
                        ],
                        mode_value,
                        false,
                        cx,
                    ),
                ))
                .child(settings_form_row(
                    palette,
                    self.tr("settings.recordingPath"),
                    Some(SharedString::from(self.tr("settings.recordingPathDesc"))),
                    settings_input_action_control(
                        260.,
                        recording_path_input,
                        small_button(
                            palette,
                            "settings-recording-path-browse",
                            self.tr("settings.browse"),
                            cx.listener(|this, _, _, cx| {
                                this.prompt_recording_path_setting(cx);
                            }),
                        ),
                    ),
                ))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(13.))
                                .text_color(gpui::rgb(palette.text))
                                .child(self.tr("settings.recordingPathTemplate")),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .text_color(gpui::rgb(palette.text_dimmed))
                                .child(self.tr("settings.recordingPathTemplateDesc")),
                        )
                        .child(
                            div()
                                .w_full()
                                .max_w(px(576.))
                                .child(recording_template_input),
                        ),
                )
                .child(settings_form_row(
                    palette,
                    self.tr("settings.recordingAutoStart"),
                    Some(SharedString::from(
                        self.tr("settings.recordingAutoStartDesc"),
                    )),
                    settings_switch(
                        palette,
                        "settings-recording-auto",
                        self.settings.summary().recording_auto_start,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_recording_auto_start(cx);
                        }),
                    ),
                ))
                .child(settings_form_row(
                    palette,
                    self.tr("settings.recordingIncludeMetadata"),
                    Some(SharedString::from(
                        self.tr("settings.recordingIncludeMetadataDesc"),
                    )),
                    settings_switch(
                        palette,
                        "settings-recording-metadata",
                        self.settings.summary().recording_include_session_metadata,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_recording_session_metadata(cx);
                        }),
                    ),
                ))
                .child(settings_form_row(
                    palette,
                    self.tr("settings.recordingIncludeIoLabels"),
                    Some(SharedString::from(
                        self.tr("settings.recordingIncludeIoLabelsDesc"),
                    )),
                    settings_switch(
                        palette,
                        "settings-recording-labels",
                        self.settings.summary().recording_include_io_labels,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_recording_io_labels(cx);
                        }),
                    ),
                ))
                .child(settings_form_row(
                    palette,
                    self.tr("settings.recordingIncludeTimestamps"),
                    Some(SharedString::from(
                        self.tr("settings.recordingIncludeTimestampsDesc"),
                    )),
                    settings_switch(
                        palette,
                        "settings-recording-timestamps",
                        self.settings.summary().recording_include_timestamps,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_recording_timestamps(cx);
                        }),
                    ),
                ))
                .child(settings_form_row(
                    palette,
                    self.tr("settings.recordingRotation"),
                    Some(SharedString::from(
                        self.tr("settings.recordingRotationDesc"),
                    )),
                    self.settings_select_control(
                        "settings.recording.rotation",
                        vec![
                            NyaSelectOption::new(
                                "session",
                                self.tr("settings.recordingRotationSession"),
                            ),
                            NyaSelectOption::new(
                                "daily",
                                self.tr("settings.recordingRotationDaily"),
                            ),
                            NyaSelectOption::new("size", self.tr("settings.recordingRotationSize")),
                        ],
                        rotation_value,
                        false,
                        cx,
                    ),
                ))
                .when(
                    matches!(
                        self.settings.summary().recording_rotation,
                        RecordingRotationPolicy::Size { .. }
                    ),
                    |this| {
                        this.child(settings_form_row(
                            palette,
                            self.tr("settings.recordingRotationSizeLimit"),
                            Some(SharedString::from(
                                self.tr("settings.recordingRotationSizeLimitDesc"),
                            )),
                            self.number_input_box(
                                "settings.number.recording-rotation-size",
                                rotation_size_mib.to_string().as_str(),
                                NyaNumberInputOptions::default()
                                    .range(1.0, 102_400.0)
                                    .step(1.0)
                                    .suffix("MiB"),
                                cx,
                            ),
                        ))
                    },
                )
                .child(settings_form_row(
                    palette,
                    self.tr("settings.recordingExistingFileBehavior"),
                    Some(SharedString::from(
                        self.tr("settings.recordingExistingFileBehaviorDesc"),
                    )),
                    self.settings_select_control(
                        "settings.recording.existing-file",
                        vec![
                            NyaSelectOption::new(
                                "unique",
                                self.tr("settings.recordingExistingUnique"),
                            ),
                            NyaSelectOption::new(
                                "append",
                                self.tr("settings.recordingExistingAppend"),
                            ),
                            NyaSelectOption::new(
                                "overwrite",
                                self.tr("settings.recordingExistingOverwrite"),
                            ),
                        ],
                        existing_value,
                        false,
                        cx,
                    ),
                ))
                .child(settings_form_row(
                    palette,
                    self.tr("settings.recordingIncludeBinaryTransfers"),
                    Some(SharedString::from(
                        self.tr("settings.recordingIncludeBinaryTransfersDesc"),
                    )),
                    settings_switch(
                        palette,
                        "settings-recording-binary-transfer-payloads",
                        self.settings
                            .summary()
                            .recording_include_binary_transfer_payloads,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_recording_binary_transfer_payloads(cx);
                        }),
                    ),
                ))
                .child(settings_form_row(
                    palette,
                    self.tr("settings.recordingMemoryLimit"),
                    Some(SharedString::from(
                        self.tr("settings.recordingMemoryLimitDesc"),
                    )),
                    self.number_input_box(
                        "settings.number.recording-memory-limit",
                        memory_mib.to_string().as_str(),
                        NyaNumberInputOptions::default()
                            .range(1.0, 512.0)
                            .step(1.0)
                            .suffix("MiB"),
                        cx,
                    ),
                )),
        ))
    }
}

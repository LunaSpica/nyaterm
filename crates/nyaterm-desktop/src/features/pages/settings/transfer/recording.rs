use super::*;

impl NyaTermApp {
    pub(in crate::features) fn recording_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let recording_path_value = if self.settings.recording_path.is_empty() {
            " ".to_string()
        } else {
            truncate_preview(&self.settings.recording_path, 34)
        };
        let memory_mib = (self.settings.recording_memory_limit_bytes / (1024 * 1024)).max(1);

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
                    self.tr("settings.recordingPath"),
                    Some(SharedString::from(self.tr("settings.recordingPathDesc"))),
                    div()
                        .w(px(260.))
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            transfer_input(
                                "settings-recording-path-input",
                                self.tr("settings.recordingPath"),
                                recording_path_value,
                                true,
                                palette,
                            )
                            .track_focus(&self.recording_path_focus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                window.focus(&this.recording_path_focus);
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(
                                |this, event: &KeyDownEvent, _, cx| {
                                    cx.stop_propagation();
                                    this.handle_recording_path_key_down(event, cx);
                                },
                            )),
                        )
                        .child(small_button(
                            palette,
                            "settings-recording-path-browse",
                            self.tr("settings.browse"),
                            cx.listener(|this, _, _, cx| {
                                this.prompt_recording_path_setting(cx);
                            }),
                        )),
                ))
                .child(settings_form_row(
                    palette,
                    self.tr("settings.recordingAutoStart"),
                    Some(SharedString::from(
                        self.tr("settings.recordingAutoStartDesc"),
                    )),
                    settings_switch(
                        palette,
                        "settings-recording-auto",
                        self.settings.recording_auto_start,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_recording_auto_start(cx);
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
                        self.settings.recording_include_io_labels,
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
                        self.settings.recording_include_timestamps,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_recording_timestamps(cx);
                        }),
                    ),
                ))
                .child(settings_form_row(
                    palette,
                    self.tr("settings.recordingMemoryLimit"),
                    Some(SharedString::from(
                        self.tr("settings.recordingMemoryLimitDesc"),
                    )),
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(small_button(
                            palette,
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
                                .font_family(crate::features::gpui_code_font_family())
                                .text_size(px(12.))
                                .font_weight(FontWeight(700.))
                                .text_color(rgb(palette.text))
                                .child(format!("{memory_mib}")),
                        )
                        .child(small_button(
                            palette,
                            "settings-recording-memory-plus",
                            "+1 MiB",
                            cx.listener(|this, _, _, cx| {
                                this.adjust_recording_memory_limit(1, cx);
                            }),
                        )),
                )),
        ))
    }
}

use super::*;

impl NyaTermApp {
    pub(in crate::features) fn recording_settings_section(
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

        div().flex().flex_col().gap_3().child(settings_form_section(
            palette,
            Some("Recording"),
            Some("Session capture path, memory cap, and stream annotations."),
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(settings_form_row(
                    palette,
                    "Path",
                    Some(SharedString::from(truncate_preview(&path, 56))),
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(small_button(
                            palette,
                            "settings-recording-path-browse",
                            "Browse",
                            cx.listener(|this, _, _, cx| {
                                this.prompt_recording_path_setting(cx);
                            }),
                        ))
                        .child(small_button(
                            palette,
                            "settings-recording-path-clear",
                            "Clear",
                            cx.listener(|this, _, _, cx| {
                                this.settings.recording_path.clear();
                                this.save_recording_settings(cx);
                            }),
                        )),
                ))
                .child(settings_form_row(
                    palette,
                    "Memory limit",
                    Some(SharedString::from(format!("{memory_mib} MiB buffer"))),
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
                ))
                .child(settings_form_row(
                    palette,
                    "Auto start",
                    Some(SharedString::from(
                        "Begin recording when a session connects.",
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
                    "IO labels",
                    Some(SharedString::from(
                        "Annotate recorded streams with in/out labels.",
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
                    "Timestamps",
                    Some(SharedString::from(
                        "Prefix recorded chunks with wall-clock timestamps.",
                    )),
                    settings_switch(
                        palette,
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

use super::*;

impl NyaTermApp {
    pub(in crate::features::pages::settings) fn transfer_advanced_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let permissions = self.settings.transfer_default_file_permissions.clone();

        div().flex().flex_col().gap_3().child(settings_form_section(
            palette,
            Some("Advanced transfer"),
            Some("Concurrency, buffer size, timestamps, and default permissions."),
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(settings_form_row(
                    palette,
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
                            palette,
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
                                .font_family(crate::features::gpui_code_font_family())
                                .text_size(px(12.))
                                .font_weight(FontWeight(700.))
                                .text_color(rgb(palette.text))
                                .child(self.settings.transfer_download_threads.to_string()),
                        )
                        .child(small_button(
                            palette,
                            "settings-transfer-download-threads-inc",
                            "+",
                            cx.listener(|this, _, _, cx| {
                                this.adjust_transfer_download_threads(1, cx);
                            }),
                        )),
                ))
                .child(settings_form_row(
                    palette,
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
                            palette,
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
                                .font_family(crate::features::gpui_code_font_family())
                                .text_size(px(12.))
                                .font_weight(FontWeight(700.))
                                .text_color(rgb(palette.text))
                                .child(self.settings.transfer_upload_threads.to_string()),
                        )
                        .child(small_button(
                            palette,
                            "settings-transfer-upload-threads-inc",
                            "+",
                            cx.listener(|this, _, _, cx| {
                                this.adjust_transfer_upload_threads(1, cx);
                            }),
                        )),
                ))
                .child(settings_form_row(
                    palette,
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
                            palette,
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
                                .font_family(crate::features::gpui_code_font_family())
                                .text_size(px(12.))
                                .font_weight(FontWeight(700.))
                                .text_color(rgb(palette.text))
                                .child(self.settings.transfer_max_retries.to_string()),
                        )
                        .child(small_button(
                            palette,
                            "settings-transfer-retries-inc",
                            "+",
                            cx.listener(|this, _, _, cx| {
                                this.adjust_transfer_max_retries(1, cx);
                            }),
                        )),
                ))
                .child(settings_form_row(
                    palette,
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
                            palette,
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
                                .font_family(crate::features::gpui_code_font_family())
                                .text_size(px(12.))
                                .font_weight(FontWeight(700.))
                                .text_color(rgb(palette.text))
                                .child(self.settings.transfer_buffer_size.to_string()),
                        )
                        .child(small_button(
                            palette,
                            "settings-transfer-buffer-inc",
                            "+",
                            cx.listener(|this, _, _, cx| {
                                this.adjust_transfer_buffer_size(1, cx);
                            }),
                        )),
                ))
                .child(settings_form_row(
                    palette,
                    "Preserve timestamps",
                    Some(SharedString::from(
                        "Keep source mtime when downloading when possible.",
                    )),
                    settings_switch(
                        palette,
                        "settings-transfer-preserve-timestamps",
                        self.settings.transfer_preserve_timestamps,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_transfer_preserve_timestamps(cx);
                        }),
                    ),
                ))
                .child(settings_form_row(
                    palette,
                    "Resume broken",
                    Some(SharedString::from(
                        "Attempt to resume interrupted downloads.",
                    )),
                    settings_switch(
                        palette,
                        "settings-transfer-resume-broken",
                        self.settings.transfer_resume_broken_transfer,
                        cx.listener(|this, _, _, cx| {
                            this.toggle_transfer_resume_broken(cx);
                        }),
                    ),
                ))
                .child(settings_form_row(
                    palette,
                    "Default permissions",
                    Some(SharedString::from(format!("chmod {permissions}"))),
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

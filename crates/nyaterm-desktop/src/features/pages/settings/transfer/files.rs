use super::*;

impl NyaTermApp {
    pub(in crate::features) fn transfer_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let download_path_value = if self.settings.transfer_download_path.is_empty() {
            " ".to_string()
        } else {
            truncate_preview(&self.settings.transfer_download_path, 34)
        };
        let policy = self.transfer.paths.duplicate_policy;

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                palette,
                None,
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.downloadPath"),
                        Some(SharedString::from(self.tr("settings.downloadPathDesc"))),
                        div()
                            .w_full()
                            .max_w(px(260.))
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                transfer_input(
                                    "settings-transfer-download-path",
                                    self.tr("settings.downloadPath"),
                                    download_path_value,
                                    true,
                                    palette,
                                )
                                .track_focus(&self.transfer.paths.download_focus)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    window.focus(&this.transfer.paths.download_focus);
                                    cx.notify();
                                }))
                                .on_key_down(cx.listener(
                                    |this, event: &KeyDownEvent, _, cx| {
                                        cx.stop_propagation();
                                        this.handle_transfer_download_path_key_down(event, cx);
                                    },
                                )),
                            )
                            .child(small_button(
                                palette,
                                "transfer-browse-download",
                                self.tr("settings.browse"),
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_transfer_download_path_setting(cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.askSaveLocation"),
                        Some(SharedString::from(self.tr("settings.askSaveLocationDesc"))),
                        settings_switch(
                            palette,
                            "transfer-ask-save",
                            self.settings.transfer_ask_save_location,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_transfer_ask_save_location(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.duplicateStrategy"),
                        Some(SharedString::from(
                            self.tr("settings.duplicateStrategyDesc"),
                        )),
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(
                                palette,
                                "transfer-dup-ask",
                                self.tr("settings.strategyAsk"),
                                policy == SftpDuplicatePolicy::Ask,
                                cx.listener(|this, _, _, cx| {
                                    this.update_transfer_duplicate_policy(
                                        SftpDuplicatePolicy::Ask,
                                        cx,
                                    );
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "transfer-dup-overwrite",
                                self.tr("settings.strategyOverwrite"),
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
                                self.tr("settings.strategySkip"),
                                policy == SftpDuplicatePolicy::Skip,
                                cx.listener(|this, _, _, cx| {
                                    this.update_transfer_duplicate_policy(
                                        SftpDuplicatePolicy::Skip,
                                        cx,
                                    );
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "transfer-dup-rename",
                                self.tr("settings.strategyRename"),
                                policy == SftpDuplicatePolicy::Rename,
                                cx.listener(|this, _, _, cx| {
                                    this.update_transfer_duplicate_policy(
                                        SftpDuplicatePolicy::Rename,
                                        cx,
                                    );
                                }),
                            )),
                    ))
                    .child(self.transfer_editor_settings_rows(cx)),
            ))
            .child(self.recording_settings_section(cx))
            .child(self.transfer_advanced_settings_section(cx))
    }
}

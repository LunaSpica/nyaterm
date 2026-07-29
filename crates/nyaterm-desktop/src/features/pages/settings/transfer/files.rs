use gpui::{Context, IntoElement, SharedString, div, prelude::*, px};
use nyaterm_transport::SftpDuplicatePolicy;

use crate::features::{NyaTermApp, TextInputSetup};
use crate::widgets::small_button;

use super::super::{
    settings_choice_chip, settings_form_row, settings_form_section, settings_switch,
};

impl NyaTermApp {
    pub(in crate::features) fn transfer_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let download_path_input = self
            .text_input_box(
                "settings.transfer.download-path",
                &self.settings.summary().transfer_download_path.clone(),
                TextInputSetup::placeholder(self.tr("settings.downloadPath")),
                cx,
            )
            .into_any_element();
        let policy = self.transfer.duplicate_policy();

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
                            .child(download_path_input)
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
                            self.settings.summary().transfer_ask_save_location,
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

use super::*;

impl NyaTermApp {
    fn translation_input(
        &mut self,
        id: &'static str,
        label: &'static str,
        value: String,
        field: TranslateInputField,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        transfer_input(
            id,
            label,
            if value.is_empty() {
                " ".to_string()
            } else {
                value
            },
            self.translate_focused_field == field,
        )
        .track_focus(&self.translate_focus)
        .on_click(cx.listener(move |this, _, window, cx| {
            this.translate_focused_field = field;
            window.focus(&this.translate_focus);
            cx.notify();
        }))
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
            cx.stop_propagation();
            this.handle_translate_key_down(event, cx);
        }))
    }

    pub(in crate::ui::view) fn translation_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Tauri TranslationTab density: provider chips + credential fields in sections.
        let translation_target_value = self.translation_settings.target_language.clone();
        let deepl_key_value = cloud_secret_display(
            &self.translation_secret_draft.deepl_api_key,
            &none_if_blank(&self.translation_settings.deepl_api_key),
        );
        let baidu_app_id_value = self.translation_settings.baidu_app_id.clone();
        let baidu_key_value = cloud_secret_display(
            &self.translation_secret_draft.baidu_app_key,
            &none_if_blank(&self.translation_settings.baidu_app_key),
        );
        let ali_app_id_value = self.translation_settings.ali_app_id.clone();
        let ali_key_value = cloud_secret_display(
            &self.translation_secret_draft.ali_app_key,
            &none_if_blank(&self.translation_settings.ali_app_key),
        );
        let youdao_app_id_value = self.translation_settings.youdao_app_id.clone();
        let youdao_key_value = cloud_secret_display(
            &self.translation_secret_draft.youdao_app_key,
            &none_if_blank(&self.translation_settings.youdao_app_key),
        );
        let translation_secret_count = [
            &self.translation_settings.deepl_api_key,
            &self.translation_settings.baidu_app_key,
            &self.translation_settings.ali_app_key,
            &self.translation_settings.youdao_app_key,
        ]
        .iter()
        .filter(|value| !value.trim().is_empty())
        .count();
        let runtime_label = if self.translate_pending {
            "running"
        } else {
            "idle"
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                Some("Provider"),
                Some("Choose the online translation backend."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        "Status",
                        Some(SharedString::from(format!(
                            "{} · {} secrets · {}",
                            translation_provider_status(self.translate_provider.as_str()),
                            translation_secret_count,
                            runtime_label
                        ))),
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0x8b949e))
                            .child(truncate_preview(&self.translate_status, 36)),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .child(settings_choice_chip(
                                "translation-provider-google-settings",
                                "Google",
                                self.translate_provider == "google",
                                cx.listener(|this, _, _, cx| {
                                    this.set_translate_provider("google", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                "translation-provider-microsoft-settings",
                                "Microsoft",
                                self.translate_provider == "microsoft",
                                cx.listener(|this, _, _, cx| {
                                    this.set_translate_provider("microsoft", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                "translation-provider-deepl-settings",
                                "DeepL",
                                self.translate_provider == "deepl",
                                cx.listener(|this, _, _, cx| {
                                    this.set_translate_provider("deepl", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                "translation-provider-baidu-settings",
                                "Baidu",
                                self.translate_provider == "baidu",
                                cx.listener(|this, _, _, cx| {
                                    this.set_translate_provider("baidu", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                "translation-provider-ali-settings",
                                "Ali",
                                self.translate_provider == "ali",
                                cx.listener(|this, _, _, cx| {
                                    this.set_translate_provider("ali", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                "translation-provider-youdao-settings",
                                "Youdao",
                                self.translate_provider == "youdao",
                                cx.listener(|this, _, _, cx| {
                                    this.set_translate_provider("youdao", cx);
                                }),
                            )),
                    )
                    .child(settings_form_row(
                        "Target language",
                        Some(SharedString::from(
                            "Default destination language for panel translations.",
                        )),
                        self.translation_input(
                            "translation-target-language",
                            "Target",
                            translation_target_value,
                            TranslateInputField::SettingsTargetLanguage,
                            cx,
                        ),
                    ))
                    .child(settings_form_row(
                        "Actions",
                        None,
                        small_button(
                            "translation-settings-save",
                            "Save",
                            cx.listener(|this, _, _, cx| {
                                this.save_translation_settings(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                Some("DeepL"),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(self.translation_input(
                        "translation-deepl-key",
                        "DeepL Key",
                        deepl_key_value,
                        TranslateInputField::DeeplApiKey,
                        cx,
                    ))
                    .child(
                        div().child(small_button(
                            "translation-clear-deepl",
                            "Clear DeepL",
                            cx.listener(|this, _, _, cx| {
                                this.clear_translation_secret("deepl", cx);
                            }),
                        )),
                    ),
            ))
            .child(settings_form_section(
                Some("Baidu"),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .grid()
                            .grid_cols(2)
                            .gap_2()
                            .child(self.translation_input(
                                "translation-baidu-app-id",
                                "Baidu App ID",
                                baidu_app_id_value,
                                TranslateInputField::BaiduAppId,
                                cx,
                            ))
                            .child(self.translation_input(
                                "translation-baidu-app-key",
                                "Baidu App Key",
                                baidu_key_value,
                                TranslateInputField::BaiduAppKey,
                                cx,
                            )),
                    )
                    .child(small_button(
                        "translation-clear-baidu",
                        "Clear Baidu",
                        cx.listener(|this, _, _, cx| {
                            this.clear_translation_secret("baidu", cx);
                        }),
                    )),
            ))
            .child(settings_form_section(
                Some("Ali"),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .grid()
                            .grid_cols(2)
                            .gap_2()
                            .child(self.translation_input(
                                "translation-ali-app-id",
                                "Ali App ID",
                                ali_app_id_value,
                                TranslateInputField::AliAppId,
                                cx,
                            ))
                            .child(self.translation_input(
                                "translation-ali-app-key",
                                "Ali App Key",
                                ali_key_value,
                                TranslateInputField::AliAppKey,
                                cx,
                            )),
                    )
                    .child(small_button(
                        "translation-clear-ali",
                        "Clear Ali",
                        cx.listener(|this, _, _, cx| {
                            this.clear_translation_secret("ali", cx);
                        }),
                    )),
            ))
            .child(settings_form_section(
                Some("Youdao"),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .grid()
                            .grid_cols(2)
                            .gap_2()
                            .child(self.translation_input(
                                "translation-youdao-app-id",
                                "Youdao App ID",
                                youdao_app_id_value,
                                TranslateInputField::YoudaoAppId,
                                cx,
                            ))
                            .child(self.translation_input(
                                "translation-youdao-app-key",
                                "Youdao App Key",
                                youdao_key_value,
                                TranslateInputField::YoudaoAppKey,
                                cx,
                            )),
                    )
                    .child(small_button(
                        "translation-clear-youdao",
                        "Clear Youdao",
                        cx.listener(|this, _, _, cx| {
                            this.clear_translation_secret("youdao", cx);
                        }),
                    )),
            ))
    }


}

fn translation_provider_status(provider: &str) -> &'static str {
    match provider {
        "google" | "microsoft" => "no key",
        "deepl" => "DeepL",
        "baidu" => "Baidu",
        "ali" => "Ali",
        "youdao" => "Youdao",
        _ => "unknown",
    }
}

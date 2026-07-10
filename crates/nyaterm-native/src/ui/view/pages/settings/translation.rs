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

        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x2a3140))
            .bg(rgb(0x151923))
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Translation"),
                    )
                    .child(status_pill(
                        translation_provider_status(self.translate_provider.as_str()),
                        rgb(0x93c5fd),
                        rgb(0x17253b),
                    ))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x98a3b8))
                            .child(self.translate_status.clone()),
                    ),
            )
            .child(
                div()
                    .mt_3()
                    .grid()
                    .grid_cols(4)
                    .gap_3()
                    .child(metric("Provider", self.translate_provider.clone()))
                    .child(metric(
                        "Target",
                        self.translation_settings.target_language.clone(),
                    ))
                    .child(metric("Secrets", translation_secret_count.to_string()))
                    .child(metric(
                        "Runtime",
                        if self.translate_pending {
                            "running".to_string()
                        } else {
                            "idle".to_string()
                        },
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(policy_button(
                        "translation-provider-google-settings",
                        "Google",
                        self.translate_provider == "google",
                        cx.listener(|this, _, _, cx| {
                            this.set_translate_provider("google", cx);
                        }),
                    ))
                    .child(policy_button(
                        "translation-provider-microsoft-settings",
                        "Microsoft",
                        self.translate_provider == "microsoft",
                        cx.listener(|this, _, _, cx| {
                            this.set_translate_provider("microsoft", cx);
                        }),
                    ))
                    .child(policy_button(
                        "translation-provider-deepl-settings",
                        "DeepL",
                        self.translate_provider == "deepl",
                        cx.listener(|this, _, _, cx| {
                            this.set_translate_provider("deepl", cx);
                        }),
                    ))
                    .child(policy_button(
                        "translation-provider-baidu-settings",
                        "Baidu",
                        self.translate_provider == "baidu",
                        cx.listener(|this, _, _, cx| {
                            this.set_translate_provider("baidu", cx);
                        }),
                    ))
                    .child(policy_button(
                        "translation-provider-ali-settings",
                        "Ali",
                        self.translate_provider == "ali",
                        cx.listener(|this, _, _, cx| {
                            this.set_translate_provider("ali", cx);
                        }),
                    ))
                    .child(policy_button(
                        "translation-provider-youdao-settings",
                        "Youdao",
                        self.translate_provider == "youdao",
                        cx.listener(|this, _, _, cx| {
                            this.set_translate_provider("youdao", cx);
                        }),
                    ))
                    .child(small_button(
                        "translation-settings-save",
                        "Save",
                        cx.listener(|this, _, _, cx| {
                            this.save_translation_settings(cx);
                        }),
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .grid()
                    .grid_cols(3)
                    .gap_2()
                    .child(self.translation_input(
                        "translation-target-language",
                        "Target",
                        translation_target_value,
                        TranslateInputField::SettingsTargetLanguage,
                        cx,
                    ))
                    .child(self.translation_input(
                        "translation-deepl-key",
                        "DeepL Key",
                        deepl_key_value,
                        TranslateInputField::DeeplApiKey,
                        cx,
                    ))
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
                    ))
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
                    ))
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
            .child(
                div()
                    .mt_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(small_button(
                        "translation-clear-deepl",
                        "Clear DeepL",
                        cx.listener(|this, _, _, cx| {
                            this.clear_translation_secret("deepl", cx);
                        }),
                    ))
                    .child(small_button(
                        "translation-clear-baidu",
                        "Clear Baidu",
                        cx.listener(|this, _, _, cx| {
                            this.clear_translation_secret("baidu", cx);
                        }),
                    ))
                    .child(small_button(
                        "translation-clear-ali",
                        "Clear Ali",
                        cx.listener(|this, _, _, cx| {
                            this.clear_translation_secret("ali", cx);
                        }),
                    ))
                    .child(small_button(
                        "translation-clear-youdao",
                        "Clear Youdao",
                        cx.listener(|this, _, _, cx| {
                            this.clear_translation_secret("youdao", cx);
                        }),
                    )),
            )
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

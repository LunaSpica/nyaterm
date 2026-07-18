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
            self.theme_palette(),
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

    pub(in crate::features) fn translation_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
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

        let deepl_configured = !self.translation_settings.deepl_api_key.trim().is_empty()
            || !self.translation_secret_draft.deepl_api_key.is_empty();
        let baidu_configured = !self.translation_settings.baidu_app_id.trim().is_empty()
            && (!self.translation_settings.baidu_app_key.trim().is_empty()
                || !self.translation_secret_draft.baidu_app_key.is_empty());
        let ali_configured = !self.translation_settings.ali_app_id.trim().is_empty()
            && (!self.translation_settings.ali_app_key.trim().is_empty()
                || !self.translation_secret_draft.ali_app_key.is_empty());
        let youdao_configured = !self.translation_settings.youdao_app_id.trim().is_empty()
            && (!self.translation_settings.youdao_app_key.trim().is_empty()
                || !self.translation_secret_draft.youdao_app_key.is_empty());

        let target_language_label = self.tr("settings.targetLanguage");
        let target_language_desc = self.tr("settings.targetLanguageDesc");
        let providers_label = self.tr("settings.translationProviders");
        let providers_desc = self.tr("settings.translationProvidersDesc");
        let no_key_label = self.tr("settings.noKeyRequired");
        let configured_label = self.tr("settings.configured");
        let not_configured_label = self.tr("settings.notConfigured");
        let api_key_label = self.tr("settings.apiKey");
        let app_id_label = self.tr("settings.appId");
        let app_key_label = self.tr("settings.appKey");
        let remove_label = self.tr("common.remove");

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
                    .child(
                        div()
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight(500.))
                                    .text_color(rgb(palette.text))
                                    .child(target_language_label),
                            )
                            .child(
                                div()
                                    .mt_1()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette.text_dimmed))
                                    .child(target_language_desc),
                            ),
                    )
                    .child(div().flex().flex_wrap().gap_1().children(
                        translation_target_languages().iter().map(|(code, label)| {
                            let code = *code;
                            let label = *label;
                            let selected = self
                                .translation_settings
                                .target_language
                                .eq_ignore_ascii_case(code);
                            settings_choice_chip(
                                palette,
                                format!("translation-target-{code}"),
                                label,
                                selected,
                                cx.listener(move |this, _, _, cx| {
                                    this.translation_settings.target_language = code.to_string();
                                    this.translate_target_language = code.to_string();
                                    this.save_translation_settings(cx);
                                }),
                            )
                        }),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some(providers_label),
                Some(providers_desc),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(translation_provider_card(
                        palette,
                        self.tr("translation.google"),
                        no_key_label,
                        true,
                        true,
                        div(),
                    ))
                    .child(translation_provider_card(
                        palette,
                        self.tr("translation.microsoft"),
                        no_key_label,
                        true,
                        true,
                        div(),
                    ))
                    .child(translation_provider_card(
                        palette,
                        self.tr("translation.deepl"),
                        if deepl_configured {
                            configured_label
                        } else {
                            not_configured_label
                        },
                        deepl_configured,
                        false,
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(self.translation_input(
                                "translation-deepl-api-key",
                                api_key_label,
                                deepl_key_value,
                                TranslateInputField::DeeplApiKey,
                                cx,
                            ))
                            .child(small_button(
                                palette,
                                "translation-clear-deepl",
                                remove_label,
                                cx.listener(|this, _, _, cx| {
                                    this.clear_translation_secret("deepl", cx);
                                }),
                            )),
                    ))
                    .child(translation_provider_card(
                        palette,
                        self.tr("translation.baidu"),
                        if baidu_configured {
                            configured_label
                        } else {
                            not_configured_label
                        },
                        baidu_configured,
                        false,
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
                                        app_id_label,
                                        baidu_app_id_value,
                                        TranslateInputField::BaiduAppId,
                                        cx,
                                    ))
                                    .child(self.translation_input(
                                        "translation-baidu-app-key",
                                        app_key_label,
                                        baidu_key_value,
                                        TranslateInputField::BaiduAppKey,
                                        cx,
                                    )),
                            )
                            .child(small_button(
                                palette,
                                "translation-clear-baidu",
                                remove_label,
                                cx.listener(|this, _, _, cx| {
                                    this.clear_translation_secret("baidu", cx);
                                }),
                            )),
                    ))
                    .child(translation_provider_card(
                        palette,
                        self.tr("translation.ali"),
                        if ali_configured {
                            configured_label
                        } else {
                            not_configured_label
                        },
                        ali_configured,
                        false,
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
                                        app_id_label,
                                        ali_app_id_value,
                                        TranslateInputField::AliAppId,
                                        cx,
                                    ))
                                    .child(self.translation_input(
                                        "translation-ali-app-key",
                                        app_key_label,
                                        ali_key_value,
                                        TranslateInputField::AliAppKey,
                                        cx,
                                    )),
                            )
                            .child(small_button(
                                palette,
                                "translation-clear-ali",
                                remove_label,
                                cx.listener(|this, _, _, cx| {
                                    this.clear_translation_secret("ali", cx);
                                }),
                            )),
                    ))
                    .child(translation_provider_card(
                        palette,
                        self.tr("translation.youdao"),
                        if youdao_configured {
                            configured_label
                        } else {
                            not_configured_label
                        },
                        youdao_configured,
                        false,
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
                                        app_id_label,
                                        youdao_app_id_value,
                                        TranslateInputField::YoudaoAppId,
                                        cx,
                                    ))
                                    .child(self.translation_input(
                                        "translation-youdao-app-key",
                                        app_key_label,
                                        youdao_key_value,
                                        TranslateInputField::YoudaoAppKey,
                                        cx,
                                    )),
                            )
                            .child(small_button(
                                palette,
                                "translation-clear-youdao",
                                remove_label,
                                cx.listener(|this, _, _, cx| {
                                    this.clear_translation_secret("youdao", cx);
                                }),
                            )),
                    )),
            ))
    }
}

fn translation_provider_card(
    palette: crate::theme::ThemePalette,
    title: &'static str,
    status_label: &'static str,
    ok: bool,
    free: bool,
    body: impl IntoElement,
) -> impl IntoElement {
    let (fg, bg) = if free {
        (palette.link, palette.hover)
    } else if ok {
        (palette.success, 0x12261c)
    } else {
        (palette.text_muted, palette.border)
    };
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_3()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight(600.))
                        .text_color(rgb(palette.text))
                        .child(title),
                )
                .child(status_pill(status_label, rgb(fg), rgb(bg))),
        )
        .when(!free, |this| this.child(body))
}

fn translation_target_languages() -> &'static [(&'static str, &'static str)] {
    &[
        ("zh-CN", "中文 (简体)"),
        ("zh-TW", "中文 (繁體)"),
        ("en", "English"),
        ("ja", "日本語"),
        ("ko", "한국어"),
        ("fr", "Français"),
        ("de", "Deutsch"),
        ("es", "Español"),
        ("pt", "Português"),
        ("ru", "Русский"),
        ("it", "Italiano"),
        ("ar", "العربية"),
        ("th", "ไทย"),
        ("vi", "Tiếng Việt"),
    ]
}

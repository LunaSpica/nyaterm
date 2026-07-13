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
            .child(settings_form_section(palette,
                Some("Provider"),
                Some("Choose the online translation backend."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette,
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
                            .child(settings_choice_chip(palette,
                                "translation-provider-google-settings",
                                "Google",
                                self.translate_provider == "google",
                                cx.listener(|this, _, _, cx| {
                                    this.set_translate_provider("google", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette,
                                "translation-provider-microsoft-settings",
                                "Microsoft",
                                self.translate_provider == "microsoft",
                                cx.listener(|this, _, _, cx| {
                                    this.set_translate_provider("microsoft", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette,
                                "translation-provider-deepl-settings",
                                "DeepL",
                                self.translate_provider == "deepl",
                                cx.listener(|this, _, _, cx| {
                                    this.set_translate_provider("deepl", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette,
                                "translation-provider-baidu-settings",
                                "Baidu",
                                self.translate_provider == "baidu",
                                cx.listener(|this, _, _, cx| {
                                    this.set_translate_provider("baidu", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette,
                                "translation-provider-ali-settings",
                                "Ali",
                                self.translate_provider == "ali",
                                cx.listener(|this, _, _, cx| {
                                    this.set_translate_provider("ali", cx);
                                }),
                            ))
                            .child(settings_choice_chip(palette,
                                "translation-provider-youdao-settings",
                                "Youdao",
                                self.translate_provider == "youdao",
                                cx.listener(|this, _, _, cx| {
                                    this.set_translate_provider("youdao", cx);
                                }),
                            )),
                    )
                    .child(settings_form_row(
                        palette,
                        "Target language",
                        Some(SharedString::from(
                            "Default destination language for panel translations.",
                        )),
                        self.translation_input(
                            "translation-target-language",
                            "Target",
                            translation_target_value.clone(),
                            TranslateInputField::SettingsTargetLanguage,
                            cx,
                        ),
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap_1()
                            .children(translation_target_languages().iter().map(|(code, label)| {
                                let code = *code;
                                let label = *label;
                                let selected = self
                                    .translation_settings
                                    .target_language
                                    .eq_ignore_ascii_case(code)
                                    || self.translate_target_language.eq_ignore_ascii_case(code);
                                settings_choice_chip(
                                    palette,
                                    format!("translation-target-{code}"),
                                    label,
                                    selected,
                                    cx.listener(move |this, _, _, cx| {
                                        this.translation_settings.target_language =
                                            code.to_string();
                                        this.translate_target_language = code.to_string();
                                        this.save_translation_settings(cx);
                                    }),
                                )
                            })),
                    )
                    .child(settings_form_row(palette,
                        "Actions",
                        None,
                        small_button(palette,
                            "translation-settings-save",
                            "Save",
                            cx.listener(|this, _, _, cx| {
                                this.save_translation_settings(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some("Providers"),
                Some("Free engines need no key; paid engines store secrets like the Tauri Translation tab."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(translation_provider_card(
                        palette,
                        "Google",
                        "No key required",
                        true,
                        true,
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .child("Built-in free backend for panel and terminal selection translation."),
                    ))
                    .child(translation_provider_card(
                        palette,
                        "Microsoft",
                        "No key required",
                        true,
                        true,
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .child("Built-in free backend (edge-style translator path)."),
                    ))
                    .child(translation_provider_card(
                        palette,
                        "DeepL",
                        if self.translation_settings.deepl_api_key.trim().is_empty() {
                            "Not configured"
                        } else {
                            "Configured"
                        },
                        !self.translation_settings.deepl_api_key.trim().is_empty(),
                        false,
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(self.translation_input(
                                "translation-deepl-api-key",
                                "DeepL API key",
                                deepl_key_value,
                                TranslateInputField::DeeplApiKey,
                                cx,
                            ))
                            .child(small_button(
                                palette,
                                "translation-clear-deepl",
                                "Clear DeepL",
                                cx.listener(|this, _, _, cx| {
                                    this.clear_translation_secret("deepl", cx);
                                }),
                            )),
                    ))
                    .child(translation_provider_card(
                        palette,
                        "Baidu",
                        if self.translation_settings.baidu_app_id.trim().is_empty()
                            || self.translation_settings.baidu_app_key.trim().is_empty()
                        {
                            "Not configured"
                        } else {
                            "Configured"
                        },
                        !(self.translation_settings.baidu_app_id.trim().is_empty()
                            || self.translation_settings.baidu_app_key.trim().is_empty()),
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
                                palette,
                                "translation-clear-baidu",
                                "Clear Baidu",
                                cx.listener(|this, _, _, cx| {
                                    this.clear_translation_secret("baidu", cx);
                                }),
                            )),
                    ))
                    .child(translation_provider_card(
                        palette,
                        "Ali",
                        if self.translation_settings.ali_app_id.trim().is_empty()
                            || self.translation_settings.ali_app_key.trim().is_empty()
                        {
                            "Not configured"
                        } else {
                            "Configured"
                        },
                        !(self.translation_settings.ali_app_id.trim().is_empty()
                            || self.translation_settings.ali_app_key.trim().is_empty()),
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
                                palette,
                                "translation-clear-ali",
                                "Clear Ali",
                                cx.listener(|this, _, _, cx| {
                                    this.clear_translation_secret("ali", cx);
                                }),
                            )),
                    ))
                    .child(translation_provider_card(
                        palette,
                        "Youdao",
                        if self.translation_settings.youdao_app_id.trim().is_empty()
                            || self.translation_settings.youdao_app_key.trim().is_empty()
                        {
                            "Not configured"
                        } else {
                            "Configured"
                        },
                        !(self.translation_settings.youdao_app_id.trim().is_empty()
                            || self.translation_settings.youdao_app_key.trim().is_empty()),
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
                                palette,
                                "translation-clear-youdao",
                                "Clear Youdao",
                                cx.listener(|this, _, _, cx| {
                                    this.clear_translation_secret("youdao", cx);
                                }),
                            )),
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

fn translation_provider_card(
    palette: crate::theme::ThemePalette,
    title: &'static str,
    status_label: &'static str,
    ok: bool,
    free: bool,
    body: impl IntoElement,
) -> impl IntoElement {
    let (fg, bg) = if free {
        (palette.accent, palette.hover)
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
        .child(body)
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

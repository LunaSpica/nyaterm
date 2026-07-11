use super::*;

use crate::translation_http::translate_text;

impl NyaTermApp {
    pub(in crate::ui::view) fn run_translation(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.translate_pending {
            self.translate_status = "translation already running".to_string();
            cx.notify();
            return;
        }
        if self.translate_input.trim().is_empty() {
            self.translate_status = "type text before translating".to_string();
            cx.notify();
            return;
        }

        self.translate_pending = true;
        self.translate_status = format!("translating with {}", self.translate_provider);
        self.ensure_event_pump(window, cx);
        let tx = self.translate_tx.clone();
        let provider = self.translate_provider.clone();
        let target_language = self.translate_target_language.clone();
        let text = self.translate_input.clone();
        let settings = self.translation_settings.clone();
        std::thread::spawn(move || {
            let result = translate_text(&provider, &text, &target_language, &settings);
            let _ = tx.send(TranslateJobResult { result });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn set_translate_provider(
        &mut self,
        provider: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.translate_provider = provider.to_string();
        self.translate_status = format!("translation provider set to {provider}");
        cx.notify();
    }

    pub(in crate::ui::view) fn save_translation_settings(&mut self, cx: &mut Context<Self>) {
        let mut next = self.translation_settings.clone();
        if !self.translation_secret_draft.deepl_api_key.is_empty() {
            next.deepl_api_key = self.translation_secret_draft.deepl_api_key.clone();
        }
        if !self.translation_secret_draft.baidu_app_key.is_empty() {
            next.baidu_app_key = self.translation_secret_draft.baidu_app_key.clone();
        }
        if !self.translation_secret_draft.ali_app_key.is_empty() {
            next.ali_app_key = self.translation_secret_draft.ali_app_key.clone();
        }
        if !self.translation_secret_draft.youdao_app_key.is_empty() {
            next.youdao_app_key = self.translation_secret_draft.youdao_app_key.clone();
        }

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_translation_settings(next))
        {
            Ok(saved) => {
                self.translation_settings = saved;
                self.translation_secret_draft = TranslationSecretDraft::default();
                self.translate_target_language = self.translation_settings.target_language.clone();
                self.translate_status = "translation settings saved".to_string();
                self.store_status.message = "translation settings saved".to_string();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.translate_status = format!("translation settings save failed: {error}");
                self.store_status.message = self.translate_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn clear_translation_secret(
        &mut self,
        provider: &'static str,
        cx: &mut Context<Self>,
    ) {
        match provider {
            "deepl" => {
                self.translation_settings.deepl_api_key.clear();
                self.translation_secret_draft.deepl_api_key.clear();
            }
            "baidu" => {
                self.translation_settings.baidu_app_key.clear();
                self.translation_secret_draft.baidu_app_key.clear();
            }
            "ali" => {
                self.translation_settings.ali_app_key.clear();
                self.translation_secret_draft.ali_app_key.clear();
            }
            "youdao" => {
                self.translation_settings.youdao_app_key.clear();
                self.translation_secret_draft.youdao_app_key.clear();
            }
            _ => {}
        }
        self.translate_status = format!("{provider} translation secret cleared; save to persist");
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_translate_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }
        let settings_field = self.translate_focused_field.is_settings_field();

        match keystroke.key.as_str() {
            "backspace" => {
                self.translate_input_value_mut().pop();
                self.translate_status = if settings_field {
                    "translation settings edited".to_string()
                } else {
                    "translation input edited".to_string()
                };
                cx.notify();
            }
            "enter" if self.translate_focused_field == TranslateInputField::Text => {
                self.translate_input.push('\n');
                self.translate_status = "translation input edited".to_string();
                cx.notify();
            }
            "escape" => {
                self.translate_status = if settings_field {
                    "translation settings input blurred".to_string()
                } else {
                    "translation input blurred".to_string()
                };
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.translate_input_value_mut().push_str(input);
                    self.translate_status = if settings_field {
                        "translation settings edited".to_string()
                    } else {
                        "translation input edited".to_string()
                    };
                    cx.notify();
                }
            }
        }
    }

    fn translate_input_value_mut(&mut self) -> &mut String {
        match self.translate_focused_field {
            TranslateInputField::TargetLanguage => &mut self.translate_target_language,
            TranslateInputField::Text => &mut self.translate_input,
            TranslateInputField::SettingsTargetLanguage => {
                &mut self.translation_settings.target_language
            }
            TranslateInputField::DeeplApiKey => &mut self.translation_secret_draft.deepl_api_key,
            TranslateInputField::BaiduAppId => &mut self.translation_settings.baidu_app_id,
            TranslateInputField::BaiduAppKey => &mut self.translation_secret_draft.baidu_app_key,
            TranslateInputField::AliAppId => &mut self.translation_settings.ali_app_id,
            TranslateInputField::AliAppKey => &mut self.translation_secret_draft.ali_app_key,
            TranslateInputField::YoudaoAppId => &mut self.translation_settings.youdao_app_id,
            TranslateInputField::YoudaoAppKey => &mut self.translation_secret_draft.youdao_app_key,
        }
    }


    pub(in crate::ui::view) fn open_translation_dialog(
        &mut self,
        text: String,
        provider: String,
        provider_label: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = text.trim().to_string();
        if text.is_empty() {
            self.translate_status = "no text to translate".to_string();
            cx.notify();
            return;
        }
        self.translation_dialog = Some(TranslationDialogState {
            source_text: text.clone(),
            provider: provider.clone(),
            provider_label,
        });
        self.translate_provider = provider;
        self.translate_input = text;
        self.translate_result = None;
        self.translate_status = format!("translating with {}", self.translate_provider);
        // Kick off immediately (Tauri TranslationDialog behavior).
        if !self.translate_pending {
            self.run_translation(window, cx);
        } else {
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn close_translation_dialog(&mut self, cx: &mut Context<Self>) {
        if self.translation_dialog.take().is_some() {
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn translation_dialog_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let Some(dialog) = self.translation_dialog.clone() else {
            return div().into_any_element();
        };
        let provider_label = dialog.provider_label.clone();
        let source = dialog.source_text.clone();
        let pending = self.translate_pending;
        let status = self.translate_status.clone();
        let result = self.translate_result.clone();
        let detected = result
            .as_ref()
            .map(|item| item.detected_language.clone())
            .filter(|s| !s.trim().is_empty());
        let translated = result
            .as_ref()
            .map(|item| item.translated.clone())
            .unwrap_or_default();
        let can_copy = !translated.trim().is_empty();

        let mut source_box = div()
            .rounded_sm()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.input))
            .p_3()
            .max_h(px(120.))
            .overflow_hidden()
            .text_sm()
            .text_color(rgb(palette.text));
        for line in source.lines().take(8) {
            source_box = source_box.child(
                div()
                    .whitespace_nowrap()
                    .child(truncate_preview(line, 96)),
            );
        }
        if source.lines().count() == 0 {
            source_box = source_box.child(source.clone());
        }

        let mut result_box = div()
            .rounded_sm()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.input))
            .p_3()
            .min_h(px(60.))
            .max_h(px(200.))
            .overflow_hidden()
            .text_sm()
            .text_color(rgb(palette.text));
        if pending {
            result_box = result_box.child(
                div()
                    .text_color(rgb(palette.text_muted))
                    .child("Translating…"),
            );
        } else if status.starts_with("translation failed") {
            result_box = result_box.child(
                div()
                    .text_color(rgb(palette.danger))
                    .child(status.clone()),
            );
        } else if !translated.is_empty() {
            for line in translated.lines().take(12) {
                result_box = result_box.child(
                    div()
                        .child(line.to_string()),
                );
            }
            if translated.lines().count() == 0 {
                result_box = result_box.child(translated.clone());
            }
        } else {
            result_box = result_box.child(
                div()
                    .text_color(rgb(palette.text_muted))
                    .child(status.clone()),
            );
        }

        div()
            .id(SharedString::from("translation-dialog-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.close_translation_dialog(cx);
                }),
            )
            .child(
                div()
                    .id(SharedString::from("translation-dialog"))
                    .w(px(540.))
                    .max_w_full()
                    .mx_4()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .p_4()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(palette.text))
                                    .child("Translation"),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py(px(2.))
                                    .rounded_sm()
                                    .bg(rgb(palette.surface))
                                    .text_size(px(11.))
                                    .text_color(rgb(palette.text_muted))
                                    .child(provider_label),
                            ),
                    )
                    .child(
                        div()
                            .mt_3()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .child("Source"),
                    )
                    .child(div().mt_1().child(source_box))
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette.text_muted))
                                    .child("Translated"),
                            )
                            .when_some(detected, |this, lang| {
                                this.child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text_dimmed))
                                        .child(format!("detected: {lang}")),
                                )
                            }),
                    )
                    .child(div().mt_1().child(result_box))
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(div().when(!can_copy, |this| this.opacity(0.45)).child(
                                small_button(
                                    palette,
                                    "translation-dialog-copy",
                                    "Copy",
                                    cx.listener(|this, _, _, cx| {
                                        if let Some(result) = this.translate_result.clone() {
                                            cx.write_to_clipboard(ClipboardItem::new_string(
                                                result.translated,
                                            ));
                                            this.translate_status = "translated text copied".to_string();
                                            cx.notify();
                                        }
                                    }),
                                ),
                            ))
                            .child(small_button(
                                palette,
                                "translation-dialog-close",
                                "Close",
                                cx.listener(|this, _, _, cx| {
                                    this.close_translation_dialog(cx);
                                }),
                            )),
                    ),
            )
            .into_any_element()
    }

    pub(super) fn drain_translate_events(&mut self) {
        while let Ok(event) = self.translate_rx.try_recv() {
            self.translate_pending = false;
            match event.result {
                Ok(result) => {
                    self.translate_status = format!(
                        "translated {} character(s) from {}",
                        result.original.chars().count(),
                        result.detected_language
                    );
                    self.terminal_status = self.translate_status.clone();
                    self.translate_result = Some(result);
                }
                Err(error) => {
                    self.translate_status = format!("translation failed: {error}");
                    self.terminal_status = self.translate_status.clone();
                }
            }
        }
    }
}

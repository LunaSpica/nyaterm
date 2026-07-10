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

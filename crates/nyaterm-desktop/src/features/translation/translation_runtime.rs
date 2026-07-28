use gpui::{
    ClipboardItem, Context, FontWeight, IntoElement, MouseButton, SharedString, Window, div,
    prelude::*, px, rgb, rgba,
};
use nyaterm_core::ConnectionStore;

use crate::features::NyaTermApp;
use crate::http::translation::translate_text;
use crate::models::{TranslateInputField, TranslationDialogState, TranslationSecretDraft};
use crate::widgets::small_button;

use super::state::TranslateJobResult;

const TRANSLATE_EVENT_DRAIN_LIMIT: usize = 8;

impl NyaTermApp {
    pub(in crate::features) fn run_translation(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.translation.pending {
            self.translation.status = "translation already running".to_string();
            cx.notify();
            return;
        }
        if self.translation.input.trim().is_empty() {
            self.translation.status = "type text before translating".to_string();
            cx.notify();
            return;
        }

        self.translation.pending = true;
        self.translation.status = format!("translating with {}", self.translation.provider);
        let tx = self.translation.tx.clone();
        let provider = self.translation.provider.clone();
        let target_language = self.translation.target_language.clone();
        let text = self.translation.input.clone();
        let settings = self.translation.settings.clone();
        std::thread::spawn(move || {
            let result = translate_text(&provider, &text, &target_language, &settings);
            let _ = tx.send(TranslateJobResult { result });
        });
        cx.notify();
    }

    pub(in crate::features) fn save_translation_settings(&mut self, cx: &mut Context<Self>) {
        let next = self.pending_translation_settings();
        if self.defer_settings_persistence(cx) {
            self.translation.settings = next;
            self.translation.secret_draft = TranslationSecretDraft::default();
            self.translation.target_language = self.translation.settings.target_language.clone();
            self.translation.status = "translation settings staged".to_string();
            return;
        }

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_translation_settings(next))
        {
            Ok(saved) => {
                self.translation.settings = saved;
                self.translation.secret_draft = TranslationSecretDraft::default();
                self.translation.target_language =
                    self.translation.settings.target_language.clone();
                self.translation.status = "translation settings saved".to_string();
                self.settings.store_status.message = "translation settings saved".to_string();
                self.settings.store_status.ready = true;
            }
            Err(error) => {
                self.translation.status = format!("translation settings save failed: {error}");
                self.settings.store_status.message = self.translation.status.clone();
                self.settings.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn clear_translation_secret(
        &mut self,
        provider: &'static str,
        cx: &mut Context<Self>,
    ) {
        match provider {
            "deepl" => {
                self.translation.settings.deepl_api_key.clear();
                self.translation.secret_draft.deepl_api_key.clear();
            }
            "baidu" => {
                self.translation.settings.baidu_app_key.clear();
                self.translation.secret_draft.baidu_app_key.clear();
            }
            "ali" => {
                self.translation.settings.ali_app_key.clear();
                self.translation.secret_draft.ali_app_key.clear();
            }
            "youdao" => {
                self.translation.settings.youdao_app_key.clear();
                self.translation.secret_draft.youdao_app_key.clear();
            }
            _ => {}
        }
        self.translation.status = format!("{provider} translation secret cleared; save to persist");
        cx.notify();
    }

    /// Apply an edit from one of the translation inputs.
    pub(in crate::features) fn apply_translate_input(
        &mut self,
        field: TranslateInputField,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.translation.focused_field = field;
        *self.translate_input_value_mut() = text;
        self.translation.status = if field.is_settings_field() {
            "translation settings edited".to_string()
        } else {
            "translation input edited".to_string()
        };
        cx.notify();
    }

    fn translate_input_value_mut(&mut self) -> &mut String {
        match self.translation.focused_field {
            TranslateInputField::TargetLanguage => &mut self.translation.target_language,
            TranslateInputField::Text => &mut self.translation.input,
            TranslateInputField::DeeplApiKey => &mut self.translation.secret_draft.deepl_api_key,
            TranslateInputField::BaiduAppId => &mut self.translation.settings.baidu_app_id,
            TranslateInputField::BaiduAppKey => &mut self.translation.secret_draft.baidu_app_key,
            TranslateInputField::AliAppId => &mut self.translation.settings.ali_app_id,
            TranslateInputField::AliAppKey => &mut self.translation.secret_draft.ali_app_key,
            TranslateInputField::YoudaoAppId => &mut self.translation.settings.youdao_app_id,
            TranslateInputField::YoudaoAppKey => &mut self.translation.secret_draft.youdao_app_key,
        }
    }

    pub(in crate::features) fn open_translation_dialog(
        &mut self,
        text: String,
        provider: String,
        provider_label: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let text = text.trim().to_string();
        if text.is_empty() {
            self.translation.status = "no text to translate".to_string();
            cx.notify();
            return;
        }
        self.translation.dialog = Some(TranslationDialogState {
            source_text: text.clone(),
            provider: provider.clone(),
            provider_label,
        });
        self.translation.provider = provider;
        self.translation.input = text;
        self.translation.result = None;
        self.translation.status = format!("translating with {}", self.translation.provider);
        // Kick off immediately (Tauri TranslationDialog behavior).
        if !self.translation.pending {
            self.run_translation(window, cx);
        } else {
            cx.notify();
        }
    }

    pub(in crate::features) fn close_translation_dialog(&mut self, cx: &mut Context<Self>) {
        if self.translation.dialog.take().is_some() {
            cx.notify();
        }
    }

    pub(in crate::features) fn translation_dialog_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let Some(dialog) = self.translation.dialog.clone() else {
            return div().into_any_element();
        };
        let provider_label = dialog.provider_label.clone();
        let source = dialog.source_text.clone();
        let pending = self.translation.pending;
        let status = self.translation.status.clone();
        let title_label = self.tr("translation.title");
        let source_label = self.tr("translation.sourceText");
        let translated_label = self.tr("translation.translatedText");
        let loading_label = self.tr("translation.loading");
        let error_label = self.tr("translation.error");
        let copy_label = self.tr("translation.copy");
        let close_label = self.tr("translation.close");
        let copied_label = self.tr("translation.copied");
        let result = self.translation.result.clone();
        let detected = result
            .as_ref()
            .map(|item| item.detected_language.clone())
            .filter(|s| !s.trim().is_empty());
        let translated = result
            .as_ref()
            .map(|item| item.translated.clone())
            .unwrap_or_default();
        let can_copy = !translated.trim().is_empty();
        let error_detail = status
            .strip_prefix("translation failed:")
            .map(str::trim)
            .filter(|detail| !detail.is_empty());
        let detected_label = detected.as_ref().map(|language| {
            self.tr("translation.detectedLang")
                .replace("{{lang}}", language)
        });

        let source_box = div()
            .id(SharedString::from("translation-dialog-source"))
            .rounded_sm()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.input))
            .p_3()
            .max_h(px(120.))
            .overflow_y_scroll()
            .scrollbar_width(px(6.))
            .text_sm()
            .line_height(px(20.))
            .whitespace_normal()
            .text_color(rgb(palette.text))
            .child(source.clone());

        let mut result_box = div()
            .id(SharedString::from("translation-dialog-result"))
            .rounded_sm()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.input))
            .p_3()
            .min_h(px(60.))
            .max_h(px(200.))
            .overflow_y_scroll()
            .scrollbar_width(px(6.))
            .text_sm()
            .line_height(px(20.))
            .whitespace_normal()
            .text_color(rgb(palette.text));
        if pending {
            result_box = result_box.child(
                div()
                    .text_color(rgb(palette.text_muted))
                    .child(loading_label),
            );
        } else if let Some(detail) = error_detail {
            result_box = result_box.child(
                div()
                    .text_color(rgb(palette.danger))
                    .child(format!("{error_label}: {detail}")),
            );
        } else if !translated.is_empty() {
            result_box = result_box.child(translated.clone());
        }

        div()
            .id(SharedString::from("translation-dialog-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x00000080))
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
                    .p_6()
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
                                    .child(title_label),
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
                            .child(source_label),
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
                                    .child(translated_label),
                            )
                            .when_some(detected_label, |this, label| {
                                this.child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text_dimmed))
                                        .child(label),
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
                                    copy_label,
                                    cx.listener(|this, _, _, cx| {
                                        if let Some(result) = this.translation.result.clone() {
                                            cx.write_to_clipboard(ClipboardItem::new_string(
                                                result.translated,
                                            ));
                                            this.translation.status = copied_label.to_string();
                                            cx.notify();
                                        }
                                    }),
                                ),
                            ))
                            .child(small_button(
                                palette,
                                "translation-dialog-close",
                                close_label,
                                cx.listener(|this, _, _, cx| {
                                    this.close_translation_dialog(cx);
                                }),
                            )),
                    ),
            )
            .into_any_element()
    }

    pub(in crate::features) fn drain_translate_events(&mut self) -> bool {
        if !self.translation.pending {
            return false;
        }
        let mut dirty = false;
        for _ in 0..TRANSLATE_EVENT_DRAIN_LIMIT {
            let Ok(event) = self.translation.rx.try_recv() else {
                break;
            };
            dirty = true;
            self.translation.pending = false;
            match event.result {
                Ok(result) => {
                    self.translation.status = format!(
                        "translated {} character(s) from {}",
                        result.original.chars().count(),
                        result.detected_language
                    );
                    self.terminal.view.status = self.translation.status.clone();
                    self.translation.result = Some(result);
                }
                Err(error) => {
                    self.translation.status = format!("translation failed: {error}");
                    self.terminal.view.status = self.translation.status.clone();
                }
            }
        }
        dirty
    }
}

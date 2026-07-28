use gpui::{
    ClipboardItem, Context, FontWeight, IntoElement, MouseButton, SharedString, Window, div,
    prelude::*, px, rgb, rgba,
};
use nyaterm_core::ConnectionStore;

use crate::features::NyaTermApp;
use crate::http::translation::translate_text;
use crate::models::TranslateInputField;
use crate::widgets::small_button;

use super::state::TranslateJobResult;

impl NyaTermApp {
    pub(in crate::features) fn run_translation(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(request) = self.translation.begin_run() else {
            cx.notify();
            return;
        };
        let (tx, provider, target_language, text, settings) = request.into_parts();
        std::thread::spawn(move || {
            let result = translate_text(&provider, &text, &target_language, &settings);
            let _ = tx.send(TranslateJobResult::new(result));
        });
        cx.notify();
    }

    pub(in crate::features) fn save_translation_settings(&mut self, cx: &mut Context<Self>) {
        let next = self.translation.pending_settings();
        if self.defer_settings_persistence(cx) {
            self.translation.settings_staged(next);
            return;
        }

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_translation_settings(next))
        {
            Ok(saved) => {
                self.translation.settings_saved(saved);
                self.settings.store_status.message = "translation settings saved".to_string();
                self.settings.store_status.ready = true;
            }
            Err(error) => {
                self.translation.settings_save_failed(error);
                self.settings.store_status.message = self.translation.status().to_string();
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
        self.translation.clear_secret(provider);
        cx.notify();
    }

    /// Apply an edit from one of the translation inputs.
    pub(in crate::features) fn apply_translate_input(
        &mut self,
        field: TranslateInputField,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.translation.edit_input(field, text);
        cx.notify();
    }

    pub(in crate::features) fn open_translation_dialog(
        &mut self,
        text: String,
        provider: String,
        provider_label: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.translation.open_dialog(text, provider, provider_label) {
            cx.notify();
            return;
        }
        // Kick off immediately (Tauri TranslationDialog behavior).
        if !self.translation.is_pending() {
            self.run_translation(window, cx);
        } else {
            cx.notify();
        }
    }

    pub(in crate::features) fn close_translation_dialog(&mut self, cx: &mut Context<Self>) {
        if self.translation.close_dialog() {
            cx.notify();
        }
    }

    pub(in crate::features) fn translation_dialog_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let Some(dialog) = self.translation.dialog_snapshot() else {
            return div().into_any_element();
        };
        let provider_label = dialog.provider_label.clone();
        let source = dialog.source_text.clone();
        let pending = self.translation.is_pending();
        let status = self.translation.status().to_string();
        let title_label = self.tr("translation.title");
        let source_label = self.tr("translation.sourceText");
        let translated_label = self.tr("translation.translatedText");
        let loading_label = self.tr("translation.loading");
        let error_label = self.tr("translation.error");
        let copy_label = self.tr("translation.copy");
        let close_label = self.tr("translation.close");
        let copied_label = self.tr("translation.copied");
        let result = self.translation.result_snapshot();
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
                                        if let Some(result) = this.translation.result_snapshot() {
                                            cx.write_to_clipboard(ClipboardItem::new_string(
                                                result.translated,
                                            ));
                                            this.translation
                                                .mark_result_copied(copied_label.to_string());
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
        let dirty = self.translation.drain_events();
        if dirty {
            self.terminal.view.status = self.translation.status().to_string();
        }
        dirty
    }
}

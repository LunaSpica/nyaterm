use gpui::{
    ClipboardItem, Context, FontWeight, IntoElement, KeyDownEvent, SharedString, div, prelude::*,
    px, rgb,
};

use crate::ui::components::{section_header, small_button};
use crate::ui::models::TranslateInputField;

use super::super::{NyaTermApp, configured_pair_status, configured_status, metric};

const TARGET_LANGUAGES: [(&str, &str); 14] = [
    ("zh-CN", "Chinese Simplified"),
    ("zh-TW", "Chinese Traditional"),
    ("en", "English"),
    ("ja", "Japanese"),
    ("ko", "Korean"),
    ("fr", "French"),
    ("de", "German"),
    ("es", "Spanish"),
    ("pt", "Portuguese"),
    ("ru", "Russian"),
    ("it", "Italian"),
    ("ar", "Arabic"),
    ("th", "Thai"),
    ("vi", "Vietnamese"),
];

const TRANSLATION_PROVIDERS: [&str; 6] = ["google", "microsoft", "deepl", "baidu", "ali", "youdao"];

impl NyaTermApp {
    pub(in crate::ui::view) fn translation_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let can_translate = !self.translate_pending && !self.translate_input.trim().is_empty();
        let input_value = if self.translate_input.is_empty() {
            " ".to_string()
        } else {
            self.translate_input.clone()
        };
        let target_value = if self.translate_target_language.is_empty() {
            " ".to_string()
        } else {
            self.translate_target_language.clone()
        };
        let result_text = self
            .translate_result
            .as_ref()
            .map(|result| result.translated.clone())
            .unwrap_or_else(|| "No translation yet.".to_string());
        let detected = self
            .translate_result
            .as_ref()
            .map(|result| result.detected_language.clone())
            .unwrap_or_else(|| "auto".to_string());
        let credential_status = match self.translate_provider.as_str() {
            "google" | "microsoft" => "not required".to_string(),
            "deepl" => configured_status(&self.translation_settings.deepl_api_key),
            "baidu" => configured_pair_status(
                &self.translation_settings.baidu_app_id,
                &self.translation_settings.baidu_app_key,
            ),
            "ali" => configured_pair_status(
                &self.translation_settings.ali_app_id,
                &self.translation_settings.ali_app_key,
            ),
            "youdao" => configured_pair_status(
                &self.translation_settings.youdao_app_id,
                &self.translation_settings.youdao_app_key,
            ),
            _ => "unsupported".to_string(),
        };
        let has_result = self
            .translate_result
            .as_ref()
            .is_some_and(|result| !result.translated.trim().is_empty());

        let mut provider_controls = div().grid().grid_cols(6).gap_2();
        for provider in TRANSLATION_PROVIDERS {
            let selected = self.translate_provider == provider;
            provider_controls = provider_controls.child(translation_provider_button(
                provider,
                selected,
                self.translation_provider_status(provider),
                cx,
            ));
        }

        let mut target_presets = div().mt_3().flex().items_center().gap_2().flex_wrap();
        for (language, label) in TARGET_LANGUAGES {
            let selected = self.translate_target_language == language;
            target_presets = target_presets.child(language_button(language, label, selected, cx));
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_5()
            .gap_4()
            .child(section_header(
                "Translation",
                "Native translation for selected terminal text or manual input.",
            ))
            .child(
                div()
                    .grid()
                    .grid_cols(4)
                    .gap_3()
                    .child(metric("Provider", self.translate_provider.clone()))
                    .child(metric("Target", self.translate_target_language.clone()))
                    .child(metric("Detected", detected))
                    .child(metric("Credentials", credential_status)),
            )
            .child(
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
                                    .text_color(rgb(0xe5edf7))
                                    .child("Provider"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xe5edf7))
                                    .child(self.translate_status.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .when(!can_translate, |this| this.opacity(0.45))
                                    .child(small_button(
                                        "translate-run",
                                        if self.translate_pending {
                                            "Running"
                                        } else {
                                            "Translate"
                                        },
                                        cx.listener(|this, _, window, cx| {
                                            this.run_translation(window, cx);
                                        }),
                                    )),
                            ),
                    )
                    .child(div().mt_3().child(provider_controls)),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(3)
                    .gap_3()
                    .child(
                        div()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x2a3140))
                            .bg(rgb(0x151923))
                            .p_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("Target"),
                            )
                            .child(
                                div()
                                    .id(SharedString::from("translate-target-input"))
                                    .mt_3()
                                    .h(px(36.))
                                    .px_3()
                                    .flex()
                                    .items_center()
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(rgb(0x303848))
                                    .bg(rgb(0x10151e))
                                    .font_family("JetBrains Mono")
                                    .text_sm()
                                    .child(target_value)
                                    .track_focus(&self.translate_focus)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.translate_focused_field =
                                            TranslateInputField::TargetLanguage;
                                        window.focus(&this.translate_focus);
                                        cx.notify();
                                    }))
                                    .on_key_down(cx.listener(
                                        |this, event: &KeyDownEvent, _, cx| {
                                            this.handle_translate_key_down(event, cx);
                                        },
                                    )),
                            )
                            .child(target_presets),
                    )
                    .child(
                        div()
                            .col_span(2)
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x2a3140))
                            .bg(rgb(0x151923))
                            .p_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("Source Text"),
                            )
                            .child(
                                div()
                                    .id(SharedString::from("translate-source-input"))
                                    .mt_3()
                                    .min_h(px(150.))
                                    .p_3()
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(rgb(0x303848))
                                    .bg(rgb(0x10151e))
                                    .font_family("JetBrains Mono")
                                    .text_sm()
                                    .line_height(px(18.))
                                    .whitespace_normal()
                                    .child(input_value)
                                    .track_focus(&self.translate_focus)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.translate_focused_field = TranslateInputField::Text;
                                        window.focus(&this.translate_focus);
                                        cx.notify();
                                    }))
                                    .on_key_down(cx.listener(
                                        |this, event: &KeyDownEvent, _, cx| {
                                            this.handle_translate_key_down(event, cx);
                                        },
                                    )),
                            ),
                    ),
            )
            .child(
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
                                    .child("Result"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .when(!has_result, |this| this.opacity(0.45))
                                    .child(small_button(
                                        "translate-copy-result",
                                        "Copy",
                                        cx.listener(|this, _, _, cx| {
                                            this.copy_translation_result(cx);
                                        }),
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .mt_3()
                            .min_h(px(160.))
                            .p_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(0x303848))
                            .bg(rgb(0x10151e))
                            .text_sm()
                            .line_height(px(20.))
                            .text_color(rgb(0xdbeafe))
                            .child(result_text),
                    ),
            )
    }

    fn set_translate_target_language(&mut self, language: &'static str, cx: &mut Context<Self>) {
        self.translate_target_language = language.to_string();
        self.translate_status = format!("translation target set to {language}");
        cx.notify();
    }

    fn copy_translation_result(&mut self, cx: &mut Context<Self>) {
        let Some(result) = self.translate_result.as_ref() else {
            self.translate_status = "no translation result to copy".to_string();
            cx.notify();
            return;
        };
        if result.translated.trim().is_empty() {
            self.translate_status = "translation result is empty".to_string();
            cx.notify();
            return;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(result.translated.clone()));
        self.translate_status = "translation copied".to_string();
        cx.notify();
    }

    fn translation_provider_status(&self, provider: &str) -> &'static str {
        match provider {
            "google" | "microsoft" => "No Key",
            "deepl" if self.translation_settings.deepl_api_key.trim().is_empty() => "Missing",
            "deepl" => "Ready",
            "baidu"
                if self.translation_settings.baidu_app_id.trim().is_empty()
                    || self.translation_settings.baidu_app_key.trim().is_empty() =>
            {
                "Missing"
            }
            "baidu" => "Ready",
            "ali"
                if self.translation_settings.ali_app_id.trim().is_empty()
                    || self.translation_settings.ali_app_key.trim().is_empty() =>
            {
                "Missing"
            }
            "ali" => "Ready",
            "youdao"
                if self.translation_settings.youdao_app_id.trim().is_empty()
                    || self.translation_settings.youdao_app_key.trim().is_empty() =>
            {
                "Missing"
            }
            "youdao" => "Ready",
            _ => "Unknown",
        }
    }
}

fn translation_provider_button(
    provider: &'static str,
    selected: bool,
    status: &'static str,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("translate-provider-{provider}")))
        .min_h(px(58.))
        .rounded_sm()
        .border_1()
        .border_color(if selected {
            rgb(0x60a5fa)
        } else {
            rgb(0x303848)
        })
        .bg(if selected {
            rgb(0x17253b)
        } else {
            rgb(0x10151e)
        })
        .px_3()
        .py_2()
        .flex()
        .flex_col()
        .justify_center()
        .gap_1()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x202a3a)))
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight(800.))
                .text_color(rgb(0xe5edf7))
                .child(provider),
        )
        .child(
            div()
                .text_size(px(10.))
                .text_color(if matches!(status, "Missing" | "Unknown") {
                    rgb(0xfca5a5)
                } else {
                    rgb(0x93c5fd)
                })
                .child(status),
        )
        .on_click(cx.listener(move |this, _, _, cx| {
            this.set_translate_provider(provider, cx);
        }))
}

fn language_button(
    language: &'static str,
    label: &'static str,
    selected: bool,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("translate-target-{language}")))
        .h(px(26.))
        .px_2()
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(if selected {
            rgb(0x60a5fa)
        } else {
            rgb(0x303848)
        })
        .bg(if selected {
            rgb(0x17253b)
        } else {
            rgb(0x10151e)
        })
        .text_size(px(10.))
        .text_color(if selected {
            rgb(0xdbeafe)
        } else {
            rgb(0xaeb7c8)
        })
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x202a3a)))
        .child(format!("{language} {label}"))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.set_translate_target_language(language, cx);
        }))
}

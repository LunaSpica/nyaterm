use gpui::{Context, IntoElement, KeyDownEvent, MouseButton, div, prelude::*, px, rgb};

use crate::features::{NyaTermApp, TextInputSetup};
use crate::theme::ThemePalette;
use crate::widgets::small_button;

use super::super::settings_form_section;

mod credential_rows;
mod model_groups;

impl NyaTermApp {
    pub(in crate::features) fn ai_models_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let query = self.ai.settings_model_query().to_string();
        let query_placeholder = self.tr("ai.searchModels");
        let search_field = self.text_input(
            "ai.settings.model-search",
            &query,
            TextInputSetup::placeholder(query_placeholder),
            cx,
        );
        let search_focus = search_field.read(cx).focus_handle();
        let has_enabled_custom_credential = self
            .ai
            .settings_config()
            .provider_credentials
            .iter()
            .any(|credential| {
                credential.enabled
                    && credential.provider_kind == nyaterm_core::AiProviderKind::OpenaiCompatible
            });
        let enabled_credentials = self
            .ai
            .settings_config()
            .provider_credentials
            .iter()
            .filter(|credential| credential.enabled)
            .count();
        let has_enabled_model = self
            .ai
            .settings_config()
            .models
            .iter()
            .any(|model| model.enabled);
        let refresh_label = if self.ai.discovery_is_pending() {
            self.tr("common.loading")
        } else {
            self.tr("ai.refreshModels")
        };

        let model_groups = self.ai_model_groups(palette, cx);
        let credential_rows = self.ai_credential_rows(palette, cx);

        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(settings_form_section(
                palette,
                Some(self.tr("ai.modelList")),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .id("ai-settings-model-search")
                                    .h(px(34.))
                                    .min_w_0()
                                    .flex_1()
                                    .px_3()
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(rgb(palette.border))
                                    .bg(rgb(palette.input))
                                    .flex()
                                    .items_center()
                                    .font_family(crate::features::gpui_code_font_family())
                                    .text_size(px(12.))
                                    .text_color(rgb(palette.text))
                                    .cursor_text()
                                    .on_mouse_down(MouseButton::Left, move |_, window, _| {
                                        window.focus(&search_focus);
                                    })
                                    .on_key_down(cx.listener(
                                        |this, event: &KeyDownEvent, _, cx| {
                                            if event.keystroke.key == "escape" {
                                                cx.stop_propagation();
                                                this.ai.clear_settings_model_query();
                                                this.reset_text_input(
                                                    "ai.settings.model-search",
                                                    "",
                                                    cx,
                                                );
                                                cx.notify();
                                            }
                                        },
                                    ))
                                    .child(search_field),
                            )
                            .child(
                                div()
                                    .opacity(
                                        if has_enabled_custom_credential
                                            && !self.ai.discovery_is_pending()
                                        {
                                            1.0
                                        } else {
                                            0.45
                                        },
                                    )
                                    .child(small_button(
                                        palette,
                                        "ai-models-discover",
                                        refresh_label,
                                        cx.listener(move |this, _, _, cx| {
                                            if has_enabled_custom_credential
                                                && !this.ai.discovery_is_pending()
                                            {
                                                this.discover_ai_models(cx);
                                            }
                                        }),
                                    )),
                            ),
                    )
                    .child(model_groups)
                    .when(enabled_credentials == 0, |this| {
                        this.child(ai_models_hint(
                            palette,
                            self.tr("ai.manualModelNoProvider"),
                            false,
                        ))
                    })
                    .when(!has_enabled_model, |this| {
                        this.child(ai_models_hint(
                            palette,
                            self.tr("ai.enableOneModelHint"),
                            true,
                        ))
                    }),
            ))
            .child(settings_form_section(
                palette,
                Some(self.tr("ai.apiKeys")),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(div().flex().justify_end().child(small_button(
                        palette,
                        "ai-cred-add",
                        self.tr("common.add"),
                        cx.listener(|this, _, window, cx| {
                            this.add_ai_credential(window, cx);
                        }),
                    )))
                    .child(credential_rows),
            ))
    }
}

fn ai_models_hint(palette: ThemePalette, text: &'static str, warning: bool) -> impl IntoElement {
    div()
        .text_size(px(11.))
        .text_color(rgb(if warning {
            palette.warning
        } else {
            palette.text_muted
        }))
        .child(text)
}

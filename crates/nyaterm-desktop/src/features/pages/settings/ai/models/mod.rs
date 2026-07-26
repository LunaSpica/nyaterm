use super::*;

mod credential_rows;
mod model_groups;

impl NyaTermApp {
    pub(in crate::features) fn ai_models_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let query = self.ai.settings.model_query.clone();
        let query_placeholder = self.tr("ai.searchModels");
        let has_enabled_custom_credential = self
            .ai
            .settings
            .config
            .provider_credentials
            .iter()
            .any(|credential| {
                credential.enabled
                    && credential.provider_kind == nyaterm_core::AiProviderKind::OpenaiCompatible
            });
        let enabled_credentials = self
            .ai
            .settings
            .config
            .provider_credentials
            .iter()
            .filter(|credential| credential.enabled)
            .count();
        let has_enabled_model = self
            .ai
            .settings
            .config
            .models
            .iter()
            .any(|model| model.enabled);
        let refresh_label = if self.ai.discovery.pending {
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
                                    .text_color(rgb(if query.is_empty() {
                                        palette.text_dimmed
                                    } else {
                                        palette.text
                                    }))
                                    .child(if query.is_empty() {
                                        query_placeholder.to_string()
                                    } else {
                                        query
                                    })
                                    .track_focus(&self.ai.settings.model_search_focus)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        window.focus(&this.ai.settings.model_search_focus);
                                        cx.notify();
                                    }))
                                    .on_key_down(cx.listener(
                                        |this, event: &KeyDownEvent, _, cx| {
                                            this.handle_ai_settings_model_search_key_down(
                                                event, cx,
                                            );
                                        },
                                    )),
                            )
                            .child(
                                div()
                                    .opacity(
                                        if has_enabled_custom_credential
                                            && !self.ai.discovery.pending
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
                                                && !this.ai.discovery.pending
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

use super::*;

impl NyaTermApp {
    pub(super) fn ai_model_groups(
        &mut self,
        palette: crate::theme::ThemePalette,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        // Group models by credential/provider (Tauri AiModelsTab groupModels).
        let credentials = self.ai_settings.provider_credentials.clone();
        let mut groups: Vec<(
            String,
            String,
            Option<nyaterm_core::AiProviderCredential>,
            Vec<nyaterm_core::AiModelConfigItem>,
        )> = Vec::new();
        let mut group_index: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for credential in &credentials {
            let key = credential.id.clone();
            group_index.insert(key.clone(), groups.len());
            groups.push((
                key,
                credential.name.clone(),
                Some(credential.clone()),
                Vec::new(),
            ));
        }
        for model in self.ai_settings.models.iter().cloned() {
            let key = model
                .credential_id
                .clone()
                .or_else(|| {
                    model
                        .provider_kind
                        .as_ref()
                        .map(|kind| format!("{kind:?}").to_ascii_lowercase())
                })
                .unwrap_or_else(|| "unknown".to_string());
            if let Some(index) = group_index.get(&key).copied() {
                groups[index].3.push(model);
            } else if let Some(index) = model.provider_kind.as_ref().and_then(|kind| {
                credentials
                    .iter()
                    .position(|credential| &credential.provider_kind == kind)
            }) {
                groups[index].3.push(model);
            } else {
                let label = model
                    .provider_kind
                    .as_ref()
                    .map(ai_provider_kind_label)
                    .unwrap_or("Unknown")
                    .to_string();
                group_index.insert(key.clone(), groups.len());
                groups.push((key, label, None, vec![model]));
            }
        }
        // Sort models in group: enabled first.
        for group in &mut groups {
            group
                .3
                .sort_by_key(|model| (!model.enabled, model.name.to_ascii_lowercase()));
        }
        let collapsed = self.ai_model_collapsed_groups.clone();
        let default_id = self.ai_settings.default_model_id.clone();
        let manual_drafts = self.ai_manual_model_drafts.clone();
        let manual_edit_group = self.ai_manual_model_edit_group.clone();

        let model_groups = groups.into_iter().fold(
            div().flex().flex_col().gap_2(),
            |rows, (group_key, label, credential, models)| {
                let is_collapsed = collapsed.contains(&group_key);
                let enabled_in_group = models.iter().filter(|model| model.enabled).count();
                let total_in_group = models.len();
                let group_key_toggle = group_key.clone();
                let group_key_add = group_key.clone();
                let draft = manual_drafts
                    .get(&group_key)
                    .cloned()
                    .unwrap_or_else(|| " ".to_string());
                let draft_active = manual_edit_group.as_deref() == Some(group_key.as_str());
                let credential_id_for_add = credential.as_ref().map(|c| c.id.clone());
                let credential_enabled = credential.as_ref().map(|c| c.enabled).unwrap_or(false);
                let credential_id_toggle = credential.as_ref().map(|c| c.id.clone());

                rows.child(
                    div()
                        .id(SharedString::from(format!("ai-model-group-{group_key}")))
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(palette.border))
                        .bg(rgb(palette.input))
                        .overflow_hidden()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .id(SharedString::from(format!(
                                    "ai-model-group-header-{group_key}"
                                )))
                                .px_3()
                                .py_2()
                                .flex()
                                .items_center()
                                .gap_2()
                                .cursor_pointer()
                                .hover(|this| this.bg(rgb(palette.hover)))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.toggle_ai_model_group(group_key_toggle.clone(), cx);
                                }))
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text_dimmed))
                                        .child(if is_collapsed { "▸" } else { "▾" }),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight(600.))
                                        .text_color(rgb(palette.text))
                                        .overflow_hidden()
                                        .child(label),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .text_color(rgb(palette.text_muted))
                                        .child(format!(
                                            "{enabled_in_group}/{total_in_group} enabled"
                                        )),
                                )
                                .when_some(credential_id_toggle, |this, credential_id| {
                                    this.child(settings_switch(
                                        palette,
                                        format!("ai-cred-enabled-{credential_id}"),
                                        credential_enabled,
                                        cx.listener(move |this, _, _, cx| {
                                            this.toggle_ai_credential_enabled(
                                                credential_id.clone(),
                                                cx,
                                            );
                                        }),
                                    ))
                                }),
                        )
                        .when(!is_collapsed, |this| {
                            this.child(
                                div()
                                    .border_t_1()
                                    .border_color(rgb(palette.border))
                                    .bg(rgb(palette.bg))
                                    .flex()
                                    .flex_col()
                                    .children(models.into_iter().map(|model| {
                                        let model_id = model.id.clone();
                                        let model_id_default = model.id.clone();
                                        let model_id_delete = model.id.clone();
                                        let is_default =
                                            default_id.as_deref() == Some(model.id.as_str());
                                        let is_manual =
                                            model.source == nyaterm_core::AiModelSource::Manual;
                                        div()
                                            .id(SharedString::from(format!(
                                                "ai-model-row-{}",
                                                model.id
                                            )))
                                            .px_3()
                                            .py_1()
                                            .border_b_1()
                                            .border_color(rgb(palette.border))
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .bg(if is_default {
                                                rgb(palette.hover)
                                            } else {
                                                rgb(palette.bg)
                                            })
                                            .child(
                                                div().size(px(8.)).rounded_full().flex_none().bg(
                                                    if model.enabled {
                                                        rgb(palette.success)
                                                    } else {
                                                        rgb(palette.border)
                                                    },
                                                ),
                                            )
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .flex_1()
                                                    .flex()
                                                    .flex_col()
                                                    .child(
                                                        div()
                                                            .text_size(px(12.))
                                                            .font_weight(FontWeight(600.))
                                                            .text_color(rgb(palette.text))
                                                            .overflow_hidden()
                                                            .child(truncate_preview(
                                                                &model.name,
                                                                48,
                                                            )),
                                                    )
                                                    .child(
                                                        div()
                                                            .text_size(px(10.))
                                                            .text_color(rgb(palette.text_dimmed))
                                                            .child(format!(
                                                                "{}{}",
                                                                ai_model_source_label(
                                                                    &model.source
                                                                ),
                                                                if is_default {
                                                                    " · default"
                                                                } else {
                                                                    ""
                                                                }
                                                            )),
                                                    ),
                                            )
                                            .child(settings_switch(
                                                palette,
                                                format!("ai-model-enabled-{}", model.id),
                                                model.enabled,
                                                cx.listener(move |this, _, _, cx| {
                                                    this.toggle_ai_model_enabled(
                                                        model_id.clone(),
                                                        cx,
                                                    );
                                                }),
                                            ))
                                            .child(small_button(
                                                palette,
                                                format!("ai-model-default-{}", model.id),
                                                if is_default { "Default" } else { "Set" },
                                                cx.listener(move |this, _, _, cx| {
                                                    this.set_ai_default_model(
                                                        model_id_default.clone(),
                                                        cx,
                                                    );
                                                }),
                                            ))
                                            .when(is_manual, |this| {
                                                this.child(small_button(
                                                    palette,
                                                    format!("ai-model-delete-{}", model.id),
                                                    "Delete",
                                                    cx.listener(move |this, _, _, cx| {
                                                        this.remove_ai_manual_model(
                                                            model_id_delete.clone(),
                                                            cx,
                                                        );
                                                    }),
                                                ))
                                            })
                                    }))
                                    .when_some(credential_id_for_add, |this, credential_id| {
                                        let group_for_focus = group_key_add.clone();
                                        let group_for_add = group_key_add.clone();
                                        let draft_value = if draft.trim().is_empty() {
                                            " ".to_string()
                                        } else {
                                            draft.clone()
                                        };
                                        this.child(
                                        div()
                                            .px_3()
                                            .py_2()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .id(SharedString::from(format!(
                                                        "ai-manual-model-{group_for_focus}"
                                                    )))
                                                    .min_w_0()
                                                    .flex_1()
                                                    .h(px(28.))
                                                    .px_2()
                                                    .rounded_md()
                                                    .border_1()
                                                    .border_color(if draft_active {
                                                        rgb(palette.accent)
                                                    } else {
                                                        rgb(palette.border)
                                                    })
                                                    .bg(rgb(palette.input))
                                                    .flex()
                                                    .items_center()
                                                    .text_size(px(12.))
                                                    .text_color(if draft.trim().is_empty() {
                                                        rgb(palette.text_dimmed)
                                                    } else {
                                                        rgb(palette.text)
                                                    })
                                                    .cursor_pointer()
                                                    .child(if draft.trim().is_empty() {
                                                        "Manual model name".to_string()
                                                    } else {
                                                        draft_value
                                                    })
                                                    .track_focus(&self.ai_manual_model_focus)
                                                    .on_click(cx.listener({
                                                        let group = group_for_focus.clone();
                                                        move |this, _, window, cx| {
                                                            this.ai_manual_model_edit_group =
                                                                Some(group.clone());
                                                            window.focus(
                                                                &this.ai_manual_model_focus,
                                                            );
                                                            cx.notify();
                                                        }
                                                    }))
                                                    .on_key_down(cx.listener({
                                                        let group = group_for_focus.clone();
                                                        move |this, event: &KeyDownEvent, _, cx| {
                                                            this.handle_ai_manual_model_key_down(
                                                                &group, event, cx,
                                                            );
                                                        }
                                                    })),
                                            )
                                            .child(small_button(
                                                palette,
                                                format!("ai-manual-add-{group_for_add}"),
                                                "Add",
                                                cx.listener({
                                                    let credential_id = credential_id.clone();
                                                    let group = group_for_add.clone();
                                                    move |this, _, _, cx| {
                                                        let name = this
                                                            .ai_manual_model_drafts
                                                            .get(&group)
                                                            .cloned()
                                                            .unwrap_or_default();
                                                        this.add_ai_manual_model(
                                                            credential_id.clone(),
                                                            name,
                                                            cx,
                                                        );
                                                        this.ai_manual_model_drafts
                                                            .insert(group.clone(), String::new());
                                                    }
                                                }),
                                            )),
                                    )
                                    }),
                            )
                        }),
                )
            },
        );
        model_groups
    }
}

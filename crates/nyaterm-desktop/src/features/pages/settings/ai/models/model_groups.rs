use super::*;

impl NyaTermApp {
    pub(super) fn ai_model_groups(
        &mut self,
        palette: crate::theme::ThemePalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let credentials: Vec<_> = self
            .ai
            .settings
            .config
            .provider_credentials
            .iter()
            .filter(|credential| credential.enabled)
            .cloned()
            .collect();
        let query = self.ai.settings.model_query.trim().to_ascii_lowercase();
        let mut groups: Vec<(
            String,
            String,
            nyaterm_core::AiProviderCredential,
            Vec<nyaterm_core::AiModelConfigItem>,
        )> = credentials
            .iter()
            .map(|credential| {
                (
                    credential.id.clone(),
                    credential.name.clone(),
                    credential.clone(),
                    Vec::new(),
                )
            })
            .collect();

        for model in
            self.ai.settings.config.models.iter().filter(|model| {
                query.is_empty() || model.name.to_ascii_lowercase().contains(&query)
            })
        {
            let group_index = model
                .credential_id
                .as_deref()
                .and_then(|credential_id| {
                    credentials
                        .iter()
                        .position(|credential| credential.id == credential_id)
                })
                .or_else(|| {
                    model.provider_kind.as_ref().and_then(|kind| {
                        credentials
                            .iter()
                            .position(|credential| &credential.provider_kind == kind)
                    })
                });
            if let Some(index) = group_index {
                groups[index].3.push(model.clone());
            }
        }
        for group in &mut groups {
            group
                .3
                .sort_by_key(|model| (!model.enabled, model.name.to_ascii_lowercase()));
        }

        let collapsed = self.ai.settings.model_collapsed_groups.clone();
        let manual_drafts = self.ai.settings.manual_model_drafts.clone();
        let manual_edit_group = self.ai.settings.manual_model_edit_group.clone();
        let manual_placeholder = self.tr("ai.manualModelPlaceholder");
        let manual_badge = self.tr("ai.manualModelBadge");
        let custom_provider = self.tr("ai.customProvider");
        let add_label = self.tr("common.add");
        let delete_label = self.tr("common.delete");

        if groups.is_empty() {
            return div()
                .rounded_md()
                .border_1()
                .border_color(rgb(palette.border))
                .px_3()
                .py_8()
                .text_center()
                .text_size(px(11.))
                .text_color(rgb(palette.text_muted))
                .child(self.tr("ai.noModels"))
                .into_any_element();
        }

        let rows = groups.into_iter().fold(
            div().flex().flex_col(),
            |rows, (group_key, mut label, credential, models)| {
                if label.trim().is_empty() {
                    label = custom_provider.to_string();
                }
                let is_collapsed = collapsed.contains(&group_key);
                let enabled_in_group = models.iter().filter(|model| model.enabled).count();
                let total_in_group = models.len();
                let group_key_toggle = group_key.clone();
                let group_key_add = group_key.clone();
                let credential_id = credential.id.clone();
                let draft = manual_drafts.get(&group_key).cloned().unwrap_or_default();
                let draft_active = manual_edit_group.as_deref() == Some(group_key.as_str());

                rows.child(
                    div()
                        .id(SharedString::from(format!("ai-model-group-{group_key}")))
                        .border_b_1()
                        .border_color(rgb(palette.border))
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
                                .bg(rgb(palette.input))
                                .cursor_pointer()
                                .hover(|this| this.bg(rgb(palette.hover)))
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.toggle_ai_model_group(group_key_toggle.clone(), cx);
                                }))
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text_dimmed))
                                        .child(if is_collapsed { ">" } else { "v" }),
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
                                        .child(format!("{enabled_in_group}/{total_in_group}")),
                                ),
                        )
                        .when(!is_collapsed, |this| {
                            let group_for_focus = group_key_add.clone();
                            let group_for_add = group_key_add.clone();
                            this.child(
                                div()
                                    .px_3()
                                    .py_2()
                                    .pl_8()
                                    .border_t_1()
                                    .border_color(rgb(palette.border))
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
                                            .h(px(30.))
                                            .px_3()
                                            .rounded_sm()
                                            .border_1()
                                            .border_color(rgb(if draft_active {
                                                palette.link
                                            } else {
                                                palette.border
                                            }))
                                            .bg(rgb(palette.input))
                                            .flex()
                                            .items_center()
                                            .font_family(crate::features::gpui_code_font_family())
                                            .text_size(px(12.))
                                            .text_color(rgb(if draft.is_empty() {
                                                palette.text_dimmed
                                            } else {
                                                palette.text
                                            }))
                                            .child(if draft.is_empty() {
                                                manual_placeholder.to_string()
                                            } else {
                                                draft.clone()
                                            })
                                            .track_focus(&self.ai.settings.manual_model_focus)
                                            .on_click(cx.listener({
                                                let group = group_for_focus.clone();
                                                move |this, _, window, cx| {
                                                    this.ai.settings.manual_model_edit_group =
                                                        Some(group.clone());
                                                    window.focus(
                                                        &this.ai.settings.manual_model_focus,
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
                                    .child(
                                        div()
                                            .opacity(if draft.trim().is_empty() {
                                                0.45
                                            } else {
                                                1.0
                                            })
                                            .child(small_button(
                                                palette,
                                                format!("ai-manual-add-{group_for_add}"),
                                                add_label,
                                                cx.listener({
                                                    let credential_id = credential_id.clone();
                                                    let group = group_for_add.clone();
                                                    move |this, _, _, cx| {
                                                        let name = this
                                                            .ai
                                                            .settings
                                                            .manual_model_drafts
                                                            .get(&group)
                                                            .cloned()
                                                            .unwrap_or_default();
                                                        if !name.trim().is_empty() {
                                                            this.add_ai_manual_model(
                                                                credential_id.clone(),
                                                                name,
                                                                cx,
                                                            );
                                                            this.ai
                                                                .settings
                                                                .manual_model_drafts
                                                                .insert(
                                                                    group.clone(),
                                                                    String::new(),
                                                                );
                                                        }
                                                    }
                                                }),
                                            )),
                                    ),
                            )
                            .children(models.into_iter().map(|model| {
                                let model_id = model.id.clone();
                                let model_id_delete = model.id.clone();
                                let is_manual = model.source == nyaterm_core::AiModelSource::Manual;
                                div()
                                    .id(SharedString::from(format!("ai-model-row-{}", model.id)))
                                    .px_3()
                                    .py_2()
                                    .pl_8()
                                    .border_t_1()
                                    .border_color(rgb(palette.border))
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex_1()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .overflow_hidden()
                                                    .text_size(px(12.))
                                                    .text_color(rgb(palette.text))
                                                    .child(truncate_preview(&model.name, 48)),
                                            )
                                            .when(is_manual, |this| {
                                                this.child(
                                                    div()
                                                        .rounded_sm()
                                                        .border_1()
                                                        .border_color(rgb(palette.border))
                                                        .px_1()
                                                        .text_size(px(10.))
                                                        .text_color(rgb(palette.text_muted))
                                                        .child(manual_badge),
                                                )
                                            }),
                                    )
                                    .child(settings_switch(
                                        palette,
                                        format!("ai-model-enabled-{}", model.id),
                                        model.enabled,
                                        cx.listener(move |this, _, _, cx| {
                                            this.toggle_ai_model_enabled(model_id.clone(), cx);
                                        }),
                                    ))
                                    .when(is_manual, |this| {
                                        this.child(small_button(
                                            palette,
                                            format!("ai-model-delete-{}", model.id),
                                            delete_label,
                                            cx.listener(move |this, _, _, cx| {
                                                this.remove_ai_manual_model(
                                                    model_id_delete.clone(),
                                                    cx,
                                                );
                                            }),
                                        ))
                                    })
                            }))
                        }),
                )
            },
        );

        div()
            .id("ai-model-groups-scroll")
            .max_h(px(352.))
            .overflow_y_scroll()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .child(rows)
            .into_any_element()
    }
}

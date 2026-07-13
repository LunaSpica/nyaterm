use super::*;

impl NyaTermApp {
    pub(in crate::features::pages::settings) fn transfer_editor_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let editor_type = self.settings.transfer_editor_type.clone();
        let default_editor_value = if self.settings.transfer_default_editor.is_empty() {
            " ".to_string()
        } else {
            self.settings.transfer_default_editor.clone()
        };
        let external_cmd = if self.settings.transfer_default_editor.trim().is_empty() {
            "system default".to_string()
        } else {
            truncate_preview(&self.settings.transfer_default_editor, 34)
        };

        div().flex().flex_col().gap_3().child(settings_form_section(
            palette,
            Some("Editor"),
            Some("How remote files open from the transfer browser."),
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(settings_form_row(
                    palette,
                    "Default open",
                    Some(SharedString::from(transfer_editor_type_status(
                        &editor_type,
                    ))),
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_1()
                        .child(settings_choice_chip(
                            palette,
                            "settings-transfer-editor-external",
                            "External",
                            editor_type == "external",
                            cx.listener(|this, _, _, cx| {
                                this.update_transfer_editor_type("external", cx);
                            }),
                        ))
                        .child(settings_choice_chip(
                            palette,
                            "settings-transfer-editor-internal",
                            "Internal",
                            editor_type == "internal",
                            cx.listener(|this, _, _, cx| {
                                this.update_transfer_editor_type("internal", cx);
                            }),
                        )),
                ))
                .when(editor_type == "external", |this| {
                    this.child(settings_form_row(
                        palette,
                        "External command",
                        Some(SharedString::from(external_cmd)),
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                transfer_input(
                                    "settings-transfer-default-editor",
                                    "Default editor command",
                                    default_editor_value,
                                    true,
                                    palette,
                                )
                                .track_focus(&self.transfer_default_editor_focus)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    window.focus(&this.transfer_default_editor_focus);
                                    cx.notify();
                                }))
                                .on_key_down(cx.listener(
                                    |this, event: &KeyDownEvent, _, cx| {
                                        cx.stop_propagation();
                                        this.handle_transfer_default_editor_key_down(event, cx);
                                    },
                                )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .gap_1()
                                    .child(small_button(
                                        palette,
                                        "settings-transfer-editor-browse",
                                        "Browse",
                                        cx.listener(|this, _, _, cx| {
                                            this.prompt_transfer_default_editor_setting(cx);
                                        }),
                                    ))
                                    .child(small_button(
                                        palette,
                                        "settings-transfer-editor-save",
                                        "Save",
                                        cx.listener(|this, _, _, cx| {
                                            this.save_transfer_settings(
                                                "transfer editor settings saved",
                                                cx,
                                            );
                                        }),
                                    )),
                            ),
                    ))
                }),
        ))
    }
}

fn transfer_editor_type_status(editor_type: &str) -> &'static str {
    match editor_type {
        "internal" => "native text editor",
        _ => "external app",
    }
}

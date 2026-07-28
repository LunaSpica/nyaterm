use gpui::{Context, IntoElement, SharedString, div, prelude::*, px};

use crate::features::{NyaTermApp, TextInputSetup};
use crate::widgets::small_button;

use super::super::{settings_choice_chip, settings_form_row};

impl NyaTermApp {
    pub(in crate::features::pages::settings) fn transfer_editor_settings_rows(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let editor_type = self.settings.transfer_editor_type.clone();
        let default_editor_input = self
            .text_input_box(
                "settings.transfer.default-editor",
                &self.settings.transfer_default_editor.clone(),
                TextInputSetup::placeholder(self.tr("settings.defaultEditor")),
                cx,
            )
            .into_any_element();
        let editor_type_label = if editor_type == "internal" {
            self.tr("settings.editorTypeInternal")
        } else {
            self.tr("settings.editorTypeExternal")
        };

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_row(
                palette,
                self.tr("settings.editorType"),
                Some(SharedString::from(editor_type_label)),
                div()
                    .flex()
                    .flex_wrap()
                    .gap_1()
                    .child(settings_choice_chip(
                        palette,
                        "settings-transfer-editor-external",
                        self.tr("settings.editorTypeExternal"),
                        editor_type == "external",
                        cx.listener(|this, _, _, cx| {
                            this.update_transfer_editor_type("external", cx);
                        }),
                    ))
                    .child(settings_choice_chip(
                        palette,
                        "settings-transfer-editor-internal",
                        self.tr("settings.editorTypeInternal"),
                        editor_type == "internal",
                        cx.listener(|this, _, _, cx| {
                            this.update_transfer_editor_type("internal", cx);
                        }),
                    )),
            ))
            .when(editor_type == "external", |this| {
                this.child(settings_form_row(
                    palette,
                    self.tr("settings.defaultEditor"),
                    Some(SharedString::from(self.tr("settings.defaultEditorDesc"))),
                    div()
                        .w_full()
                        .max_w(px(260.))
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(default_editor_input)
                        .child(small_button(
                            palette,
                            "settings-transfer-editor-browse",
                            self.tr("settings.browse"),
                            cx.listener(|this, _, _, cx| {
                                this.prompt_transfer_default_editor_setting(cx);
                            }),
                        )),
                ))
            })
    }
}

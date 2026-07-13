use super::*;

impl NyaTermApp {
    pub(in crate::features) fn connection_group_editor_panel(
        &mut self,
        editor: ConnectionGroupEditorState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let title = if editor.id.is_some() {
            "Edit Group"
        } else {
            "New Group"
        };
        let card = div()
            .id(SharedString::from("connection-group-editor-panel"))
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .track_focus(&self.connection_group_editor_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.connection_group_editor_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                this.handle_connection_group_editor_key_down(event, cx);
            }))
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(palette.text))
                    .child(title),
            )
            .child(editor_field(
                palette,
                "connection-group-name",
                "Group Name",
                editor.name.clone(),
                true,
                cx.listener(|this, _, window, cx| {
                    window.focus(&this.connection_group_editor_focus);
                    cx.notify();
                }),
            ))
            .when_some(editor.error.clone(), |this, error| {
                this.child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette.danger))
                        .child(error),
                )
            })
            .child(modal_dialog_footer(
                palette,
                "connection-group-close",
                "connection-group-save",
                "Save",
                cx.listener(|this, _, _, cx| {
                    this.close_connection_group_editor(cx);
                }),
                cx.listener(|this, _, _, cx| {
                    this.save_connection_group_editor(cx);
                }),
            ));
        modal_dialog_shell(palette, "connection-group-editor-modal", 420., card)
    }

    pub(in crate::features) fn connection_delete_confirm_panel(
        &mut self,
        confirm: ConnectionDeleteConfirmState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let card = div()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(palette.danger))
                    .child("Delete Connection"),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(palette.text))
                    .child(format!("Delete \"{}\"?", confirm.label)),
            )
            .child(modal_dialog_footer(
                palette,
                "connection-delete-cancel",
                "connection-delete-confirm",
                "Delete",
                cx.listener(|this, _, _, cx| {
                    this.close_connection_delete_confirm(cx);
                }),
                cx.listener(|this, _, _, cx| {
                    this.confirm_connection_delete(cx);
                }),
            ));
        modal_dialog_shell(palette, "connection-delete-confirm-modal", 420., card)
    }

    pub(in crate::features) fn connection_group_delete_confirm_panel(
        &mut self,
        confirm: ConnectionGroupDeleteConfirmState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let card = div()
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(palette.danger))
                    .child("Delete Group"),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(palette.text))
                    .child(format!(
                        "Delete \"{}\" ({} connections, {} child groups)?",
                        confirm.label, confirm.connection_count, confirm.child_group_count
                    )),
            )
            .child(modal_dialog_footer(
                palette,
                "connection-group-delete-cancel",
                "connection-group-delete-confirm",
                "Delete",
                cx.listener(|this, _, _, cx| {
                    this.close_connection_group_delete_confirm(cx);
                }),
                cx.listener(|this, _, _, cx| {
                    this.confirm_connection_group_delete(cx);
                }),
            ));
        modal_dialog_shell(palette, "connection-group-delete-modal", 440., card)
    }
}

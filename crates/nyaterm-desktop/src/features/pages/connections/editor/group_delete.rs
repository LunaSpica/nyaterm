use super::*;

impl NyaTermApp {
    pub(in crate::features) fn connection_group_editor_panel(
        &mut self,
        editor: ConnectionGroupEditorState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let title = if editor.id.is_some() {
            self.tr("savedConnections.renameFolder")
        } else {
            self.tr("savedConnections.newFolder")
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
                self.tr("savedConnections.folderName"),
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
            .child(modal_dialog_footer_localized(
                palette,
                "connection-group-close",
                "connection-group-save",
                self.tr("common.cancel"),
                self.tr("common.save"),
                cx.listener(|this, _, _, cx| {
                    this.close_connection_group_editor(cx);
                }),
                cx.listener(|this, _, _, cx| {
                    this.save_connection_group_editor(cx);
                }),
            ));
        modal_dialog_shell(
            palette,
            self.shell_surface_color(palette.bg),
            "connection-group-editor-modal",
            420.,
            card,
        )
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
                    .text_color(rgb(palette.text))
                    .child(self.tr("savedConnections.delete")),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(palette.text))
                    .child(
                        self.tr("savedConnections.deleteConfirm")
                            .replace("{{name}}", &confirm.label),
                    ),
            )
            .child(modal_dialog_footer_localized(
                palette,
                "connection-delete-cancel",
                "connection-delete-confirm",
                self.tr("common.cancel"),
                self.tr("savedConnections.delete"),
                cx.listener(|this, _, _, cx| {
                    this.close_connection_delete_confirm(cx);
                }),
                cx.listener(|this, _, _, cx| {
                    this.confirm_connection_delete(cx);
                }),
            ));
        modal_dialog_shell(
            palette,
            self.shell_surface_color(palette.bg),
            "connection-delete-confirm-modal",
            420.,
            card,
        )
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
                    .text_color(rgb(palette.text))
                    .child(self.tr("savedConnections.deleteFolder")),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(palette.text))
                    .child(
                        self.tr("savedConnections.deleteFolderConfirm")
                            .replace("{{name}}", &confirm.label),
                    ),
            )
            .child(modal_dialog_footer_localized(
                palette,
                "connection-group-delete-cancel",
                "connection-group-delete-confirm",
                self.tr("common.cancel"),
                self.tr("savedConnections.deleteFolder"),
                cx.listener(|this, _, _, cx| {
                    this.close_connection_group_delete_confirm(cx);
                }),
                cx.listener(|this, _, _, cx| {
                    this.confirm_connection_group_delete(cx);
                }),
            ));
        modal_dialog_shell(
            palette,
            self.shell_surface_color(palette.bg),
            "connection-group-delete-modal",
            440.,
            card,
        )
    }

    pub(in crate::features) fn connection_group_open_confirm_panel(
        &mut self,
        confirm: ConnectionGroupOpenConfirmState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let description = self
            .tr("savedConnections.openAllConnectionsConfirm")
            .replace("{{name}}", &confirm.label)
            .replace("{{count}}", &confirm.connection_count.to_string());
        let card = div()
            .id("connection-group-open-confirm-panel")
            .p_4()
            .flex()
            .flex_col()
            .gap_3()
            .track_focus(&self.connection_group_open_confirm_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.connection_group_open_confirm_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                match event.keystroke.key.as_str() {
                    "escape" => this.close_connection_group_open_confirm(cx),
                    "enter" => this.confirm_connection_group_open(window, cx),
                    _ => {}
                }
            }))
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(palette.text))
                    .child(self.tr("savedConnections.openAllConnections")),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .line_height(px(18.))
                    .text_color(rgb(palette.text_muted))
                    .child(description),
            )
            .child(modal_dialog_footer_localized(
                palette,
                "connection-group-open-cancel",
                "connection-group-open-confirm",
                self.tr("common.cancel"),
                self.tr("savedConnections.openAllConnections"),
                cx.listener(|this, _, _, cx| {
                    this.close_connection_group_open_confirm(cx);
                }),
                cx.listener(|this, _, window, cx| {
                    this.confirm_connection_group_open(window, cx);
                }),
            ));
        modal_dialog_shell(
            palette,
            self.shell_surface_color(palette.bg),
            "connection-group-open-confirm-modal",
            400.,
            card,
        )
    }
}

use gpui::{
    Context, FontWeight, IntoElement, KeyDownEvent, SharedString, div,
    prelude::{
        FluentBuilder, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled,
    },
    px, rgb,
};

use crate::features::{
    NyaTermApp, modal_dialog_footer_localized, modal_dialog_footer_localized_danger,
    modal_dialog_shell,
};
use crate::models::{
    ConnectionDeleteConfirmState, ConnectionGroupDeleteConfirmState, ConnectionGroupEditorState,
    ConnectionGroupOpenConfirmState,
};

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
            .p_6()
            .flex()
            .flex_col()
            .gap_3()
            .track_focus(&self.connection_state.group_editor_focus_handle())
            .on_click(cx.listener(|this, _, window, cx| {
                let group_editor_focus = this.connection_state.group_editor_focus_handle();
                window.focus(&group_editor_focus);
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
            .children(self.connection_state.group_editor_field().map(|field| {
                div()
                    .h(px(36.))
                    .px_3()
                    .py_1()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(palette.primary))
                    .bg(rgb(palette.input))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(self.tr("savedConnections.folderName")),
                    )
                    .child(div().min_w_0().flex_1().text_xs().child(field))
            }))
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
            384.,
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
            .p_6()
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
            .child(modal_dialog_footer_localized_danger(
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
            384.,
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
            .p_6()
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
            .child(modal_dialog_footer_localized_danger(
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
            384.,
            card,
        )
    }

    pub(in crate::features) fn connections_clear_all_confirm_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let card = div()
            .p_6()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_size(px(15.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(palette.text))
                    .child(self.tr("savedConnections.clearAll")),
            )
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(palette.text))
                    .child(self.tr("savedConnections.clearAllConfirm")),
            )
            .child(modal_dialog_footer_localized_danger(
                palette,
                "connection-clear-all-cancel",
                "connection-clear-all-confirm",
                self.tr("common.cancel"),
                self.tr("savedConnections.clearAll"),
                cx.listener(|this, _, _, cx| {
                    this.close_connections_clear_all_confirm(cx);
                }),
                cx.listener(|this, _, _, cx| {
                    this.confirm_connections_clear_all(cx);
                }),
            ));
        modal_dialog_shell(
            palette,
            self.shell_surface_color(palette.bg),
            "connection-clear-all-confirm-modal",
            384.,
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
            .p_6()
            .flex()
            .flex_col()
            .gap_3()
            .track_focus(&self.connection_state.group_open_focus_handle())
            .on_click(cx.listener(|this, _, window, cx| {
                let group_open_focus = this.connection_state.group_open_focus_handle();
                window.focus(&group_open_focus);
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
            384.,
            card,
        )
    }
}

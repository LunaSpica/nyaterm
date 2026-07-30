use gpui::{Context, IntoElement, div, prelude::*, px, rgb};
use nyaterm_ui::NyaInput;

use crate::features::NyaTermApp;
use crate::models::ConnectionGroupEditorState;

impl NyaTermApp {
    pub(in crate::features) fn connection_group_editor_content(
        &mut self,
        editor: ConnectionGroupEditorState,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        div()
            .flex()
            .flex_col()
            .gap_3()
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
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_xs()
                            .child(NyaInput::new(&field)),
                    )
            }))
            .when_some(editor.error.clone(), |this, error| {
                this.child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette.danger))
                        .child(error),
                )
            })
    }

    pub(in crate::features) fn connection_group_editor_dialog_content(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(editor) = self.connection_state.active_group_editor_draft() else {
            return div().into_any_element();
        };
        self.connection_group_editor_content(editor, cx)
            .into_any_element()
    }
}

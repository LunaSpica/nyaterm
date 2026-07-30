use gpui::{
    Context, div,
    prelude::{InteractiveElement, ParentElement, StatefulInteractiveElement, Styled},
    px, rgb, svg,
};

use crate::features::NyaTermApp;
use crate::models::{ConnectionEditorField, ConnectionEditorSelect};
use crate::widgets::small_button;

use super::super::super::list::{
    ConnectionEditorRenderContext, connection_editor_select, editor_field,
};

use super::ConnectionEditorSectionContext;

pub(super) fn connection_editor_local_section(
    section: ConnectionEditorSectionContext<'_>,
    cx: &mut Context<NyaTermApp>,
) -> gpui::Div {
    let ConnectionEditorSectionContext {
        palette,
        editor: _,
        language,
        fields,
    } = section;
    let tr = |key: &'static str| crate::i18n::text(language, key);
    div()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette.text_muted))
                        .child(tr("dialog.shellPath")),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .w(px(144.))
                                .flex_none()
                                .child(connection_editor_select(
                                    ConnectionEditorRenderContext {
                                        palette,
                                        fields,
                                        cx,
                                    },
                                    "connection-editor-shell-preset",
                                    "",
                                    ConnectionEditorSelect::Shell,
                                )),
                        )
                        .child(div().min_w_0().flex_1().child(editor_field(
                            palette,
                            "",
                            ConnectionEditorField::ShellPath,
                            fields,
                            cx,
                        )))
                        .child(
                            div()
                                .id("connection-editor-shell-browse")
                                .size(px(30.))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_sm()
                                .border_1()
                                .border_color(rgb(palette.border))
                                .bg(rgb(palette.input))
                                .cursor_pointer()
                                .hover(|this| this.bg(rgb(palette.hover)))
                                .child(
                                    svg()
                                        .size(px(14.))
                                        .path("icons/conn/folder.svg")
                                        .text_color(rgb(palette.text_muted)),
                                )
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.prompt_connection_editor_shell_path(cx);
                                })),
                        ),
                ),
        )
        .child(editor_field(
            palette,
            tr("dialog.shellArgs"),
            ConnectionEditorField::ShellArgs,
            fields,
            cx,
        ))
        .child(
            div()
                .flex()
                .items_end()
                .gap_2()
                .child(div().min_w_0().flex_1().child(editor_field(
                    palette,
                    tr("dialog.workingDir"),
                    ConnectionEditorField::WorkingDir,
                    fields,
                    cx,
                )))
                .child(small_button(
                    palette,
                    "connection-editor-cwd-browse",
                    tr("settings.browse"),
                    cx.listener(|this, _, _, cx| {
                        this.prompt_connection_editor_working_dir(cx);
                    }),
                )),
        )
}

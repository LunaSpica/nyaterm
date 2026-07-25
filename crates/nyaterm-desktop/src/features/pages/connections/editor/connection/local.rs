use gpui::{
    Context, div,
    prelude::{InteractiveElement, ParentElement, StatefulInteractiveElement, Styled},
    px, rgb, svg,
};

use crate::features::{NyaTermApp, small_button};
use crate::models::{ConnectionEditorField, ConnectionEditorMenu, ConnectionEditorState};

use super::super::super::list::{ConnectionEditorChoice, connection_editor_select, editor_field};

pub(super) fn connection_editor_local_section(
    palette: crate::theme::ThemePalette,
    editor: &ConnectionEditorState,
    shell_label: &'static str,
    shell_options: Vec<ConnectionEditorChoice>,
    open_menu: Option<ConnectionEditorMenu>,
    language: &str,
    cx: &mut Context<NyaTermApp>,
) -> gpui::Div {
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
                                    palette,
                                    "connection-editor-shell-preset",
                                    "",
                                    shell_label,
                                    ConnectionEditorMenu::Shell,
                                    open_menu == Some(ConnectionEditorMenu::Shell),
                                    shell_options,
                                    cx,
                                )),
                        )
                        .child(div().min_w_0().flex_1().child(editor_field(
                            palette,
                            "connection-editor-shell",
                            "",
                            editor.shell_path.clone(),
                            editor.focused_field == ConnectionEditorField::ShellPath,
                            cx.listener(|this, _, window, cx| {
                                this.focus_connection_editor_field(
                                    ConnectionEditorField::ShellPath,
                                    window,
                                    cx,
                                );
                            }),
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
                                .child(svg().size(px(14.)).path("icons/conn/folder.svg"))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.prompt_connection_editor_shell_path(cx);
                                })),
                        ),
                ),
        )
        .child(editor_field(
            palette,
            "connection-editor-args",
            tr("dialog.shellArgs"),
            editor.shell_args.clone(),
            editor.focused_field == ConnectionEditorField::ShellArgs,
            cx.listener(|this, _, window, cx| {
                this.focus_connection_editor_field(ConnectionEditorField::ShellArgs, window, cx);
            }),
        ))
        .child(
            div()
                .flex()
                .items_end()
                .gap_2()
                .child(div().min_w_0().flex_1().child(editor_field(
                    palette,
                    "connection-editor-cwd",
                    tr("dialog.workingDir"),
                    editor.working_dir.clone(),
                    editor.focused_field == ConnectionEditorField::WorkingDir,
                    cx.listener(|this, _, window, cx| {
                        this.focus_connection_editor_field(
                            ConnectionEditorField::WorkingDir,
                            window,
                            cx,
                        );
                    }),
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

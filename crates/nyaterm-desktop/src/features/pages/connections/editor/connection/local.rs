use super::*;

pub(super) fn connection_editor_local_section(
    palette: crate::theme::ThemePalette,
    editor: &ConnectionEditorState,
    cx: &mut Context<NyaTermApp>,
) -> gpui::Div {
    let shell_path = editor.shell_path.clone();
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette.text_muted))
                        .child("Shell path"),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_1()
                        .child(kind_chip(
                            palette,
                            "Bash",
                            shell_path == "bash"
                                || shell_path.ends_with("/bash")
                                || shell_path.ends_with("\\bash"),
                            cx.listener(|this, _, _, cx| {
                                this.set_connection_editor_shell_path("bash", cx);
                            }),
                        ))
                        .child(kind_chip(
                            palette,
                            "Zsh",
                            shell_path == "zsh"
                                || shell_path.ends_with("/zsh")
                                || shell_path.ends_with("\\zsh"),
                            cx.listener(|this, _, _, cx| {
                                this.set_connection_editor_shell_path("zsh", cx);
                            }),
                        ))
                        .child(kind_chip(
                            palette,
                            "Fish",
                            shell_path == "fish"
                                || shell_path.ends_with("/fish")
                                || shell_path.ends_with("\\fish"),
                            cx.listener(|this, _, _, cx| {
                                this.set_connection_editor_shell_path("fish", cx);
                            }),
                        ))
                        .child(kind_chip(
                            palette,
                            "sh",
                            shell_path == "sh"
                                || shell_path.ends_with("/sh")
                                || shell_path.ends_with("\\sh"),
                            cx.listener(|this, _, _, cx| {
                                this.set_connection_editor_shell_path("sh", cx);
                            }),
                        ))
                        .child(kind_chip(
                            palette,
                            "PowerShell",
                            shell_path == "powershell.exe"
                                || shell_path.ends_with("powershell.exe")
                                || shell_path.ends_with("pwsh")
                                || shell_path.ends_with("pwsh.exe"),
                            cx.listener(|this, _, _, cx| {
                                this.set_connection_editor_shell_path("powershell.exe", cx);
                            }),
                        ))
                        .child(kind_chip(
                            palette,
                            "CMD",
                            shell_path == "cmd.exe" || shell_path.ends_with("cmd.exe"),
                            cx.listener(|this, _, _, cx| {
                                this.set_connection_editor_shell_path("cmd.exe", cx);
                            }),
                        ))
                        .child(kind_chip(
                            palette,
                            "WSL",
                            shell_path == "wsl.exe" || shell_path.ends_with("wsl.exe"),
                            cx.listener(|this, _, _, cx| {
                                this.set_connection_editor_shell_path("wsl.exe", cx);
                            }),
                        )),
                ),
        )
        .child(
            div()
                .flex()
                .items_end()
                .gap_2()
                .child(div().min_w_0().flex_1().child(editor_field(
                    palette,
                    "connection-editor-shell",
                    "Shell",
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
                .child(small_button(
                    palette,
                    "connection-editor-shell-browse",
                    "Browse",
                    cx.listener(|this, _, _, cx| {
                        this.prompt_connection_editor_shell_path(cx);
                    }),
                )),
        )
        .child(editor_field(
            palette,
            "connection-editor-args",
            "Args",
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
                    "Working Dir",
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
                    "Browse",
                    cx.listener(|this, _, _, cx| {
                        this.prompt_connection_editor_working_dir(cx);
                    }),
                )),
        )
        .child(div().flex().items_center().gap_1().child(toggle_chip(
            palette,
            "Open After Save",
            editor.connect_after_save,
            cx.listener(|this, _, _, cx| {
                this.toggle_connection_editor_flag(ConnectionEditorToggle::ConnectAfterSave, cx);
            }),
        )))
}

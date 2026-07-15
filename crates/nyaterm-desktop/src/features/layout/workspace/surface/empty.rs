use super::*;

impl NyaTermApp {
    pub(in crate::features) fn empty_workspace_state(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Match Tauri EmptyWorkspaceState: large faded logo + label|shortcut rows.
        let temporary_ssh = self.display_shortcut_for("tab.temporarySshLink", "Ctrl+Alt+N");
        let open_chat = self.display_shortcut_for("view.openChat", "Ctrl+Alt+I");
        let show_commands = self.display_shortcut_for("view.showAllCommands", "Ctrl+Shift+P");
        let switch_terminal = self.display_shortcut_for("tab.quickSwitch", "Ctrl+Shift+S");

        let palette = self.theme_palette();
        let failure_banner = if let (Some(name), Some(error)) = (
            self.last_connect_failure_name.clone(),
            self.last_connect_failure_error.clone(),
        ) {
            Some((name, error))
        } else {
            None
        };
        let connecting_status = self.pending_session_status_label();
        let terminal_palette = self.terminal_theme_palette();
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(terminal_palette.terminal_bg))
            .px_6()
            .child(
                div()
                    .w(px(544.))
                    .max_w_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(
                        div()
                            .mb_9()
                            .child(nyaterm_logo_mark(terminal_palette, 256., 0.13)),
                    )
                    .when_some(connecting_status, |this, status| {
                        this.child(
                            div()
                                .mb_4()
                                .w(px(480.))
                                .max_w_full()
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(palette.warning))
                                .bg(rgba((palette.warning << 8) | 0x18))
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(palette.warning))
                                        .child(status),
                                ),
                        )
                    })
                    .when_some(failure_banner, |this, (name, error)| {
                        this.child(
                            div()
                                .mb_4()
                                .w(px(480.))
                                .max_w_full()
                                .px_3()
                                .py_2()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(palette.danger))
                                .bg(rgba((palette.danger << 8) | 0x18))
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(palette.danger))
                                        .child(format!("Failed to start {name}")),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text_muted))
                                        .child(truncate_preview(&error, 120)),
                                ),
                        )
                    })
                    .child(
                        // Tauri EmptyWorkspaceState: grid w-fit max-w-[30rem] gap-x-4 gap-y-3
                        div()
                            .w(px(480.))
                            .max_w_full()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .child(empty_workspace_action(
                                palette,
                                "Temporary Link",
                                temporary_ssh,
                                cx.listener(|this, _, window, cx| {
                                    this.ensure_panel_open(NavItem::Connections);
                                    this.open_temporary_ssh_link_dialog(window, cx);
                                }),
                            ))
                            .child(empty_workspace_action(
                                palette,
                                "Open Chat",
                                open_chat,
                                cx.listener(|this, _, window, cx| {
                                    this.ensure_panel_open(NavItem::AiAssistant);
                                    window.focus(&this.ai_chat_focus);
                                    this.ai_status = "AI assistant focused".to_string();
                                    cx.notify();
                                }),
                            ))
                            .child(empty_workspace_action(
                                palette,
                                "Show All Commands",
                                show_commands,
                                cx.listener(|this, _, window, cx| {
                                    this.bottom_panel = BottomPanelMode::QuickCommands;
                                    window.focus(&this.command_search_focus);
                                    this.terminal_status = "quick commands opened".to_string();
                                    cx.notify();
                                }),
                            ))
                            .child(empty_workspace_action(
                                palette,
                                "Switch Terminal",
                                switch_terminal,
                                cx.listener(|this, _, window, cx| {
                                    this.open_quick_switch(window, cx);
                                }),
                            )),
                    ),
            )
    }
}

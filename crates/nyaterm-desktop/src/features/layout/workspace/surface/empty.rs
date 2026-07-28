use std::time::Duration;

use gpui::{AnimationExt, Context, FontWeight, IntoElement, div, prelude::*, px, rgb, svg};
use nyaterm_core::truncate_preview;

use crate::features::view_widgets::{empty_workspace_action, nyaterm_logo_mark};
use crate::features::{NyaTermApp, TextInputSetup};
use crate::models::BottomPanelMode;
use crate::models::NavItem;

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
        let temporary_ssh_label = self.tr("temporarySsh.title");
        let open_chat_label = self.tr("app.openChat");
        let show_commands_label = self.tr("app.showAllCommands");
        let switch_terminal_label = self.tr("app.switchTerminal");

        let palette = self.theme_palette();
        let terminal_palette = self.terminal_theme_palette();
        div()
            .flex_1()
            .min_h_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(self.shell_surface_color(terminal_palette.terminal_bg))
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
                                temporary_ssh_label,
                                temporary_ssh,
                                cx.listener(|this, _, window, cx| {
                                    this.ensure_panel_open(NavItem::Connections);
                                    this.open_temporary_ssh_link_dialog(window, cx);
                                }),
                            ))
                            .child(empty_workspace_action(
                                palette,
                                open_chat_label,
                                open_chat,
                                cx.listener(|this, _, window, cx| {
                                    this.ensure_panel_open(NavItem::AiAssistant);
                                    window.focus(&this.ai.chat.focus);
                                    this.ai.panel.status = "AI assistant focused".to_string();
                                    cx.notify();
                                }),
                            ))
                            .child(empty_workspace_action(
                                palette,
                                show_commands_label,
                                show_commands,
                                cx.listener(|this, _, window, cx| {
                                    this.set_bottom_panel_mode(BottomPanelMode::QuickCommands);
                                    let search = this.commands.quick.list.search_draft.clone();
                                    let field = this.text_input(
                                        "quick-command.search",
                                        &search,
                                        TextInputSetup::placeholder(
                                            this.tr("quickCommands.search"),
                                        ),
                                        cx,
                                    );
                                    window.focus(&field.read(cx).focus_handle());
                                    this.terminal.view.status = "quick commands opened".to_string();
                                    cx.notify();
                                }),
                            ))
                            .child(empty_workspace_action(
                                palette,
                                switch_terminal_label,
                                switch_terminal,
                                cx.listener(|this, _, window, cx| {
                                    this.open_quick_switch(window, cx);
                                }),
                            )),
                    ),
            )
    }

    pub(in crate::features) fn pending_workspace_state(&self) -> impl IntoElement {
        let palette = self.theme_palette();
        let name = self
            .pending_session_display_name()
            .unwrap_or_else(|| self.tr("terminal.connecting").to_string());

        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .bg(self.shell_surface_color(self.terminal_theme_palette().terminal_bg))
            .child(
                svg()
                    .size(px(28.))
                    .path("icons/conn/connect.svg")
                    .text_color(rgb(palette.primary))
                    .with_animation(
                        "pending-workspace-spinner",
                        gpui::Animation::new(Duration::from_millis(900)).repeat(),
                        |svg, delta| {
                            svg.with_transformation(gpui::Transformation::rotate(gpui::percentage(
                                delta,
                            )))
                        },
                    ),
            )
            .child(
                div()
                    .max_w(px(320.))
                    .px_4()
                    .text_center()
                    .text_size(px(12.))
                    .text_color(rgb(palette.text_dimmed))
                    .child(name),
            )
    }

    pub(in crate::features) fn failed_workspace_state(&self) -> impl IntoElement {
        let palette = self.theme_palette();
        let error = self
            .active_failed_session()
            .map(|failed| failed.error.clone())
            .or_else(|| self.shell.chrome.last_connect_failure_error.clone())
            .unwrap_or_default();

        div()
            .flex_1()
            .min_h_0()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .bg(self.shell_surface_color(self.terminal_theme_palette().terminal_bg))
            .child(
                svg()
                    .size(px(32.))
                    .path("icons/session/disconnect.svg")
                    .text_color(rgb(palette.danger)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
                    .px_6()
                    .text_center()
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(FontWeight(600.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("terminal.connectionFailed")),
                    )
                    .child(
                        div()
                            .max_w(px(320.))
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(truncate_preview(&error, 160)),
                    ),
            )
    }
}

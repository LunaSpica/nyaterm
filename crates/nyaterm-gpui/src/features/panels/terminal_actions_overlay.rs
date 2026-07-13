use super::*;

impl NyaTermApp {
    pub(in crate::features) fn terminal_actions_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let visible_text = self.active_terminal_visible_text();
        let buffer_text = self.active_terminal_buffer_text();
        let visible_lines = visible_text.lines().count();
        let buffer_chars = buffer_text.chars().count();
        let visible_for_translate = visible_text.clone();
        let visible_for_ai = terminal_action_prompt_text(&visible_text, 2_800);
        let buffer_for_ai = terminal_action_prompt_text(&buffer_text, 4_000);
        let has_visible_text = !visible_text.trim().is_empty();
        let has_buffer_text = !buffer_text.trim().is_empty();
        let _has_selection = self.selected_terminal_text().is_some();

        div()
            .id(SharedString::from("terminal-actions-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x030508d8))
            .flex()
            .items_start()
            .justify_center()
            .pt(px(96.))
            .track_focus(&self.terminal_actions_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.terminal_actions_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                match event.keystroke.key.as_str() {
                    "escape" => this.close_terminal_actions(window, cx),
                    "v" | "V"
                        if event.keystroke.modifiers.control
                            || event.keystroke.modifiers.platform =>
                    {
                        this.terminal_actions_open = false;
                        this.paste_from_clipboard(window, cx);
                    }
                    "f" | "F"
                        if event.keystroke.modifiers.control
                            || event.keystroke.modifiers.platform =>
                    {
                        this.open_terminal_search(window, cx);
                    }
                    _ => {}
                }
            }))
            .child(
                div()
                    .id(SharedString::from("terminal-actions-dialog"))
                    .w(px(660.))
                    .max_w_full()
                    .mx_4()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(0x0b0f16))
                    .shadow_lg()
                    .p_4()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight(800.))
                                            .text_color(rgb(0xe5edf7))
                                            .child("Terminal Actions"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(palette.text_muted))
                                            .child(format!(
                                                "{visible_lines} visible line(s) · {buffer_chars} buffered character(s)"
                                            )),
                                    ),
                            )
                            .child(small_button(palette, 
                                "terminal-actions-close",
                                "Close",
                                cx.listener(|this, _, window, cx| {
                                    this.close_terminal_actions(window, cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .mt_4()
                            .grid()
                            .grid_cols(4)
                            .gap_2()
                            .child(tab_action_button(
                                palette,
                                "terminal-actions-copy-visible",
                                "Copy",
                                "selection / screen",
                                cx.listener(|this, _, _, cx| {
                                    this.copy_terminal_selection_or_visible(cx);
                                }),
                            ))
                            .child(tab_action_button(
                                palette,
                                "terminal-actions-select-all",
                                "Select All",
                                "visible grid",
                                cx.listener(|this, _, _, cx| {
                                    this.select_all_terminal_visible(cx);
                                }),
                            ))
                            .child(tab_action_button(
                                palette,
                                "terminal-actions-find",
                                "Find",
                                "search buffer",
                                cx.listener(|this, _, window, cx| {
                                    this.open_terminal_search(window, cx);
                                }),
                            ))
                            .child(tab_action_button(
                                palette,
                                "terminal-actions-sync-groups",
                                "Sync Groups",
                                "broadcast input",
                                cx.listener(|this, _, window, cx| {
                                    this.terminal_actions_open = false;
                                    this.open_sync_groups(window, cx);
                                }),
                            ))
                            .child(tab_action_button(
                                palette,
                                "terminal-actions-paste",
                                "Paste",
                                "clipboard text",
                                cx.listener(|this, _, window, cx| {
                                    this.terminal_actions_open = false;
                                    this.paste_from_clipboard(window, cx);
                                }),
                            ))
                            .child(tab_action_button(
                                palette,
                                "terminal-actions-clear-screen",
                                "Clear Screen",
                                "shell clear",
                                cx.listener(|this, _, _, cx| {
                                    this.send_terminal_clear_screen(cx);
                                }),
                            ))
                            .child(tab_action_button(
                                palette,
                                "terminal-actions-clear-all",
                                "Clear All",
                                "drop buffer",
                                cx.listener(|this, _, _, cx| {
                                    this.terminal_actions_open = false;
                                    this.clear_terminal(cx);
                                }),
                            ))
                            .child(tab_action_button(
                                palette,
                                "terminal-actions-temporary-ssh-link",
                                "Temp SSH",
                                "paste link",
                                cx.listener(|this, _, window, cx| {
                                    this.terminal_actions_open = false;
                                    this.open_temporary_ssh_link_dialog(window, cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(4)
                            .gap_2()
                            .child(
                                div().when(!has_visible_text, |this| this.opacity(0.45)).child(
                                    tab_action_button(
                                        palette,
                                        "terminal-actions-translate-visible",
                                        "Translate",
                                        "visible screen",
                                        cx.listener(move |this, _, _, cx| {
                                            this.terminal_actions_open = false;
                                            if visible_for_translate.trim().is_empty() {
                                                this.terminal_status =
                                                    "terminal visible screen is empty".to_string();
                                            } else {
                                                this.translate_input =
                                                    visible_for_translate.clone();
                                                this.translate_result = None;
                                                this.translate_status =
                                                    "visible terminal text loaded".to_string();
                                                this.select(NavItem::Translation, cx);
                                            }
                                            cx.notify();
                                        }),
                                    ),
                                ),
                            )
                            .child(
                                div().when(!has_visible_text, |this| this.opacity(0.45)).child(
                                    tab_action_button(
                                        palette,
                                        "terminal-actions-ai-visible",
                                        "Ask AI",
                                        "visible screen",
                                        cx.listener(move |this, _, window, cx| {
                                            this.terminal_actions_open = false;
                                            if visible_for_ai.trim().is_empty() {
                                                this.ai_status =
                                                    "terminal visible screen is empty".to_string();
                                            } else {
                                                this.ai_prompt_draft = format!(
                                                    "Explain this terminal output:\n\n{}",
                                                    visible_for_ai
                                                );
                                                this.ai_status =
                                                    "terminal output loaded into AI prompt"
                                                        .to_string();
                                                window.focus(&this.ai_chat_focus);
                                            }
                                            cx.notify();
                                        }),
                                    ),
                                ),
                            )
                            .child(
                                div().when(!has_buffer_text, |this| this.opacity(0.45)).child(
                                    tab_action_button(
                                        palette,
                                        "terminal-actions-ai-buffer",
                                        "AI Buffer",
                                        "buffer context",
                                        cx.listener(move |this, _, window, cx| {
                                            this.terminal_actions_open = false;
                                            if buffer_for_ai.trim().is_empty() {
                                                this.ai_status =
                                                    "terminal buffer is empty".to_string();
                                            } else {
                                                this.ai_prompt_draft = format!(
                                                    "Review this terminal buffer and summarize issues or next actions:\n\n{}",
                                                    buffer_for_ai
                                                );
                                                this.ai_status =
                                                    "terminal buffer loaded into AI prompt"
                                                        .to_string();
                                                window.focus(&this.ai_chat_focus);
                                            }
                                            cx.notify();
                                        }),
                                    ),
                                ),
                            )
                            .child(tab_action_button(
                                palette,
                                "terminal-actions-command-send",
                                "Send Panel",
                                "bottom sender",
                                cx.listener(|this, _, window, cx| {
                                    this.terminal_actions_open = false;
                                    this.bottom_panel = BottomPanelMode::CommandSend;
                                    window.focus(&this.send_command_focus);
                                    cx.notify();
                                }),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(4)
                            .gap_2()
                            .child(tab_action_button(
                                palette,
                                "terminal-actions-history-search",
                                "History",
                                "command history",
                                cx.listener(|this, _, window, cx| {
                                    this.terminal_actions_open = false;
                                    window.focus(&this.command_search_focus);
                                    this.terminal_status =
                                        "command history search focused".to_string();
                                    cx.notify();
                                }),
                            ))
                            .child(tab_action_button(
                                palette,
                                "terminal-actions-quick-commands",
                                "Commands",
                                "quick commands",
                                cx.listener(|this, _, _, cx| {
                                    this.terminal_actions_open = false;
                                    this.bottom_panel = BottomPanelMode::QuickCommands;
                                    cx.notify();
                                }),
                            ))
                            .child(tab_action_button(
                                palette,
                                "terminal-actions-recording",
                                "Recording",
                                "session log",
                                cx.listener(|this, _, _, cx| {
                                    this.terminal_actions_open = false;
                                    this.right_focus = RightFocus::Recording;
                                    cx.notify();
                                }),
                            ))
                            .child(tab_action_button(
                                palette,
                                "terminal-actions-session-info",
                                "Info",
                                "session details",
                                cx.listener(|this, _, window, cx| {
                                    this.terminal_actions_open = false;
                                    this.open_active_session_info(window, cx);
                                }),
                            )),
                    )
            )
    }
}

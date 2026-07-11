use super::*;
use gpui::{MouseDownEvent, px};

impl NyaTermApp {
    pub(in crate::ui::view) fn open_terminal_context_menu(
        &mut self,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        let selected_text = self.selected_terminal_text().unwrap_or_default();
        self.terminal_context_menu = Some(TerminalContextMenuState {
            x: event.position.x,
            y: event.position.y,
            selected_text,
        });
        self.terminal_status = "terminal context menu opened".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn close_terminal_context_menu(&mut self, cx: &mut Context<Self>) {
        if self.terminal_context_menu.take().is_some() {
            self.terminal_status = "terminal context menu closed".to_string();
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn terminal_context_menu_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let Some(menu) = self.terminal_context_menu.clone() else {
            return div().into_any_element();
        };
        let selected = menu.selected_text.clone();
        let has_selection = !selected.trim().is_empty();
        let shortcut = |id: &str, fallback: &str| self.display_shortcut_for(id, fallback);
        let copy_sc = shortcut("terminal.copy", "Ctrl+Shift+C");
        let paste_sc = shortcut("terminal.paste", "Ctrl+Shift+V");
        let paste_sel_sc = shortcut("terminal.pasteSelected", "Ctrl+Shift+X");
        let find_sc = shortcut("terminal.find", "Ctrl+Shift+F");
        let clear_sc = shortcut("terminal.clear", "Ctrl+L");
        let select_all_sc = shortcut("terminal.selectAll", "Ctrl+Shift+A");
        let selected_for_find = selected.clone();
        let selected_for_paste = selected.clone();
        let selected_for_translate = selected.clone();
        let selected_for_ai = selected.clone();

        let mut items = div()
            .id(SharedString::from("terminal-context-menu"))
            .absolute()
            .top(menu.y)
            .left(menu.x)
            .w(px(220.))
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface))
            .shadow_lg()
            .py_1()
            .flex()
            .flex_col()
            .on_mouse_down(MouseButton::Left, |_, _, _| {})
            .on_click(|_, _, cx| cx.stop_propagation());

        if has_selection {
            items = items
                .child(terminal_ctx_item(
                    palette,
                    "term-ctx-copy",
                    "Copy",
                    Some(copy_sc),
                    cx.listener(|this, _, _, cx| {
                        this.close_terminal_context_menu(cx);
                        let _ = this.copy_terminal_selection(cx);
                    }),
                ))
                .child(terminal_ctx_item(
                    palette,
                    "term-ctx-find-selection",
                    "Find",
                    Some(find_sc),
                    cx.listener(move |this, _, window, cx| {
                        this.close_terminal_context_menu(cx);
                        this.terminal_search_query = selected_for_find.clone();
                        this.terminal_search_mode = TerminalSearchMode::Buffer;
                        this.terminal_search_active_index = 0;
                        this.open_terminal_search(window, cx);
                    }),
                ))
                .child(terminal_ctx_item(
                    palette,
                    "term-ctx-translate",
                    "Translate Selection",
                    None,
                    cx.listener(move |this, _, _, cx| {
                        this.close_terminal_context_menu(cx);
                        this.translate_input = selected_for_translate.clone();
                        this.translate_result = None;
                        this.translate_status = "selection loaded for translation".to_string();
                        this.select(NavItem::Translation, cx);
                    }),
                ))
                .child(terminal_ctx_item(
                    palette,
                    "term-ctx-ai",
                    "Ask AI about Selection",
                    None,
                    cx.listener(move |this, _, window, cx| {
                        this.close_terminal_context_menu(cx);
                        this.ensure_panel_open(NavItem::AiAssistant);
                        let body = if selected_for_ai.chars().count() > 2_800 {
                            let clipped: String = selected_for_ai.chars().take(2_800).collect();
                            format!("{clipped}…")
                        } else {
                            selected_for_ai.clone()
                        };
                        this.ai_prompt_draft =
                            format!("Explain this terminal selection:\n\n{body}");
                        this.ai_status = "selection loaded into AI prompt".to_string();
                        window.focus(&this.ai_chat_focus);
                        cx.notify();
                    }),
                ))
                .child(terminal_ctx_separator(palette))
                .child(terminal_ctx_item(
                    palette,
                    "term-ctx-paste",
                    "Paste",
                    Some(paste_sc),
                    cx.listener(|this, _, window, cx| {
                        this.close_terminal_context_menu(cx);
                        this.paste_from_clipboard(window, cx);
                    }),
                ))
                .child(terminal_ctx_item(
                    palette,
                    "term-ctx-paste-selected",
                    "Paste Selected Text",
                    Some(paste_sel_sc),
                    cx.listener(move |this, _, window, cx| {
                        this.close_terminal_context_menu(cx);
                        this.paste_terminal_text(selected_for_paste.clone(), window, cx);
                    }),
                ));
        } else {
            items = items
                .child(terminal_ctx_item(
                    palette,
                    "term-ctx-paste",
                    "Paste",
                    Some(paste_sc),
                    cx.listener(|this, _, window, cx| {
                        this.close_terminal_context_menu(cx);
                        this.paste_from_clipboard(window, cx);
                    }),
                ))
                .child(terminal_ctx_item(
                    palette,
                    "term-ctx-find",
                    "Find",
                    Some(find_sc),
                    cx.listener(|this, _, window, cx| {
                        this.close_terminal_context_menu(cx);
                        this.open_terminal_search(window, cx);
                    }),
                ));
        }

        items = items
            .child(terminal_ctx_separator(palette))
            .child(terminal_ctx_item(
                palette,
                "term-ctx-clear-screen",
                "Clear Screen",
                Some(clear_sc),
                cx.listener(|this, _, _, cx| {
                    this.close_terminal_context_menu(cx);
                    this.send_terminal_clear_screen(cx);
                }),
            ))
            .child(terminal_ctx_item(
                palette,
                "term-ctx-clear-all",
                "Clear All",
                None,
                cx.listener(|this, _, _, cx| {
                    this.close_terminal_context_menu(cx);
                    this.clear_terminal(cx);
                }),
            ))
            .child(terminal_ctx_separator(palette))
            .child(terminal_ctx_item(
                palette,
                "term-ctx-select-all",
                "Select All",
                Some(select_all_sc),
                cx.listener(|this, _, _, cx| {
                    this.close_terminal_context_menu(cx);
                    this.select_all_terminal_visible(cx);
                }),
            ))
            .child(terminal_ctx_item(
                palette,
                "term-ctx-more-actions",
                "More Actions…",
                None,
                cx.listener(|this, _, window, cx| {
                    this.close_terminal_context_menu(cx);
                    this.open_terminal_actions(window, cx);
                }),
            ));

        div()
            .id(SharedString::from("terminal-context-menu-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.close_terminal_context_menu(cx);
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(|this, _, _, cx| {
                    this.close_terminal_context_menu(cx);
                }),
            )
            .child(items)
            .into_any_element()
    }
}

fn terminal_ctx_item(
    palette: crate::ui::theme::ThemePalette,
    id: impl Into<String>,
    label: impl Into<String>,
    shortcut: Option<String>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let label = label.into();
    let mut row = div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .text_size(px(12.))
        .text_color(rgb(palette.text))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.hover)))
        .on_click(on_click)
        .child(div().child(label));
    if let Some(shortcut) = shortcut {
        row = row.child(
            div()
                .text_size(px(10.))
                .text_color(rgb(palette.text_dimmed))
                .child(shortcut),
        );
    }
    row
}

fn terminal_ctx_separator(palette: crate::ui::theme::ThemePalette) -> impl IntoElement {
    div()
        .h(px(1.))
        .my_1()
        .mx_2()
        .bg(rgb(palette.border))
}

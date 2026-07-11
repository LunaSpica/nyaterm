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
        let search_engines: Vec<(String, String)> = self
            .settings
            .search_custom_engines
            .iter()
            .filter(|engine| engine.show_in_menu && !engine.name.trim().is_empty() && !engine.url_template.trim().is_empty())
            .map(|engine| (engine.name.clone(), engine.url_template.clone()))
            .collect();
        let terminal_ai_actions: Vec<(String, String, String)> = if self.ai_settings.enabled {
            self.ai_settings
                .terminal_ai_actions
                .iter()
                .filter(|action| action.enabled && !action.name.trim().is_empty())
                .map(|action| (action.id.clone(), action.name.clone(), action.prompt.clone()))
                .collect()
        } else {
            Vec::new()
        };
        let translation_providers: Vec<(String, String)> = available_translation_providers(
            &self.translation_settings,
        );

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
                .children(search_engines.into_iter().map(|(name, template)| {
                    let query = selected.clone();
                    terminal_ctx_item(
                        palette,
                        format!("term-ctx-search-{name}"),
                        format!("Search Online · {name}"),
                        None,
                        cx.listener(move |this, _, _, cx| {
                            this.close_terminal_context_menu(cx);
                            let url = search_engine_url(&template, &query);
                            match open_external_url(&url) {
                                Ok(()) => {
                                    this.terminal_status =
                                        format!("opened online search: {name}");
                                }
                                Err(error) => {
                                    this.terminal_status =
                                        format!("online search failed: {error}");
                                }
                            }
                            cx.notify();
                        }),
                    )
                }))
                .children(terminal_ai_actions.into_iter().map(|(id, name, prompt)| {
                    let query = selected.clone();
                    terminal_ctx_item(
                        palette,
                        format!("term-ctx-ai-action-{id}"),
                        format!("AI · {name}"),
                        None,
                        cx.listener(move |this, _, window, cx| {
                            this.close_terminal_context_menu(cx);
                            this.ensure_panel_open(NavItem::AiAssistant);
                            let body = if query.chars().count() > 2_800 {
                                let clipped: String = query.chars().take(2_800).collect();
                                format!("{clipped}…")
                            } else {
                                query.clone()
                            };
                            this.ai_prompt_draft = format!("{prompt}

{body}");
                            this.ai_status = format!("AI action loaded: {name}");
                            window.focus(&this.ai_chat_focus);
                            cx.notify();
                        }),
                    )
                }))
                .children({
                    let selected = selected_for_translate.clone();
                    if translation_providers.is_empty() {
                        vec![terminal_ctx_item(
                            palette,
                            "term-ctx-translate",
                            "Translate Selection",
                            None,
                            cx.listener(move |this, _, _, cx| {
                                this.close_terminal_context_menu(cx);
                                this.translate_input = selected.clone();
                                this.translate_result = None;
                                this.translate_status =
                                    "selection loaded for translation".to_string();
                                this.select(NavItem::Translation, cx);
                            }),
                        )
                        .into_any_element()]
                    } else {
                        translation_providers
                            .into_iter()
                            .map(|(id, label)| {
                                let selected = selected.clone();
                                terminal_ctx_item(
                                    palette,
                                    format!("term-ctx-translate-{id}"),
                                    format!("Translate · {label}"),
                                    None,
                                    cx.listener(move |this, _, _, cx| {
                                        this.close_terminal_context_menu(cx);
                                        this.translate_provider = id.clone();
                                        this.translate_input = selected.clone();
                                        this.translate_result = None;
                                        this.translate_status = format!(
                                            "selection loaded for {label} translation"
                                        );
                                        this.select(NavItem::Translation, cx);
                                    }),
                                )
                                .into_any_element()
                            })
                            .collect::<Vec<_>>()
                    }
                })
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


fn open_external_url(url: &str) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() {
        return Err("empty url".to_string());
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open url: {error}"))
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", ""])
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open url: {error}"))
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|error| format!("failed to open url: {error}"))
    }
}

fn search_engine_url(template: &str, query: &str) -> String {
    let encoded = urlencoding_minimal(query);
    if template.contains("%s") {
        template.replace("%s", &encoded)
    } else {
        format!("{template}{encoded}")
    }
}

fn urlencoding_minimal(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for b in input.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char);
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}


fn available_translation_providers(
    settings: &nyaterm_domain::TranslationSettings,
) -> Vec<(String, String)> {
    let mut providers = vec![
        ("google".to_string(), "Google".to_string()),
        ("microsoft".to_string(), "Microsoft".to_string()),
    ];
    if !settings.deepl_api_key.trim().is_empty() {
        providers.push(("deepl".to_string(), "DeepL".to_string()));
    }
    if !settings.baidu_app_id.trim().is_empty() && !settings.baidu_app_key.trim().is_empty() {
        providers.push(("baidu".to_string(), "Baidu".to_string()));
    }
    if !settings.ali_app_id.trim().is_empty() && !settings.ali_app_key.trim().is_empty() {
        providers.push(("ali".to_string(), "Aliyun".to_string()));
    }
    if !settings.youdao_app_id.trim().is_empty() && !settings.youdao_app_key.trim().is_empty() {
        providers.push(("youdao".to_string(), "Youdao".to_string()));
    }
    providers
}


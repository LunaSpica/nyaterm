use super::*;

impl NyaTermApp {
    pub(in crate::features) fn open_terminal_context_menu(
        &mut self,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        let selected_text = self.selected_terminal_text().unwrap_or_default();
        self.action_link_menu = None;
        self.action_link_tooltip = None;
        self.command_suggestions = None;
        self.credential_suggestions = None;
        self.terminal_context_menu = Some(TerminalContextMenuState {
            x: event.position.x,
            y: event.position.y,
            selected_text,
        });
        self.terminal_status = "terminal context menu opened".to_string();
        cx.notify();
    }

    pub(in crate::features) fn close_terminal_context_menu(&mut self, cx: &mut Context<Self>) {
        if self.terminal_context_menu.take().is_some() {
            self.terminal_status = "terminal context menu closed".to_string();
            cx.notify();
        }
    }

    pub(in crate::features) fn terminal_context_menu_overlay(
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
        let search_engines: Vec<(String, String, Option<String>)> = self
            .settings
            .search_custom_engines
            .iter()
            .filter(|engine| {
                engine.show_in_menu
                    && !engine.name.trim().is_empty()
                    && !engine.url_template.trim().is_empty()
            })
            .map(|engine| {
                (
                    engine.name.clone(),
                    engine.url_template.clone(),
                    engine.icon.clone(),
                )
            })
            .collect();
        let terminal_ai_actions: Vec<(String, String, String)> = if self.ai_settings.enabled {
            self.ai_settings
                .terminal_ai_actions
                .iter()
                .filter(|action| action.enabled && !action.name.trim().is_empty())
                .map(|action| {
                    (
                        action.id.clone(),
                        action.name.clone(),
                        action.prompt.clone(),
                    )
                })
                .collect()
        } else {
            Vec::new()
        };
        let translation_providers: Vec<(String, String)> =
            available_translation_providers(&self.translation_settings);
        let selection_link_kind: Option<&'static str> = None;
        let selection_actions: Vec<(String, ActionLinkAction)> =
            if self.settings.terminal_action_links_enabled && has_selection {
                let trimmed = selected.trim();
                let matchers = &self.settings.terminal_action_links_matchers;
                let entity = find_action_links(trimmed, matchers, true)
                    .into_iter()
                    .find(|item| item.text == trimmed || item.value == trimmed)
                    .or_else(|| {
                        find_action_links(trimmed, matchers, true)
                            .into_iter()
                            .next()
                    });
                entity
                    .map(|item| {
                        let kind = item.kind.label().to_string();
                        actions_for_match(&item)
                            .into_iter()
                            .map(|action| (kind.clone(), action))
                            .collect()
                    })
                    .unwrap_or_default()
            } else {
                Vec::new()
            };
        let _ = selection_link_kind;

        let (viewport_w, viewport_h) = self.last_viewport_size;
        // Approximate height from max-h for clamp (scrollable menus can be shorter).
        let (menu_x, menu_y) = clamp_menu_position(
            f32::from(menu.x),
            f32::from(menu.y),
            248.,
            420.,
            viewport_w,
            viewport_h,
        );
        let mut items = div()
            .id(SharedString::from("terminal-context-menu"))
            .absolute()
            .top(px(menu_y))
            .left(px(menu_x))
            .w(px(248.))
            .max_h(px(420.))
            .overflow_y_scroll()
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
                .when_some(selection_as_openable_url(&selected), |this, url| {
                    let open_url = url.clone();
                    this.child(terminal_ctx_item(
                        palette,
                        "term-ctx-open-url",
                        "Open Link",
                        None,
                        cx.listener(move |this, _, _, cx| {
                            this.close_terminal_context_menu(cx);
                            match open_external_url(&open_url) {
                                Ok(()) => {
                                    this.terminal_status = format!("opened link: {open_url}");
                                }
                                Err(error) => {
                                    this.terminal_status = format!("open link failed: {error}");
                                }
                            }
                            cx.notify();
                        }),
                    ))
                })
                .children(selection_actions.into_iter().map(|(kind, action)| {
                    let label = format!("{kind} · {}", action.label);
                    let command = action.command.clone();
                    let open_url = action.open_url.clone();
                    terminal_ctx_item(
                        palette,
                        format!("term-ctx-action-link-{}", action.id),
                        label,
                        None,
                        cx.listener(move |this, _, _, cx| {
                            this.close_terminal_context_menu(cx);
                            if let Some(url) = open_url.clone() {
                                match open_external_url(&url) {
                                    Ok(()) => {
                                        this.terminal_status = format!("opened link: {url}");
                                    }
                                    Err(error) => {
                                        this.terminal_status = format!("open link failed: {error}");
                                    }
                                }
                                cx.notify();
                                return;
                            }
                            if let Some(command) = command.clone() {
                                this.execute_action_link_command(command, cx);
                            }
                        }),
                    )
                }))
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
                .children(search_engines.into_iter().map(|(name, template, icon)| {
                    let query = selected.clone();
                    let icon_prefix = search_engine_menu_icon_prefix(icon.as_deref());
                    terminal_ctx_item(
                        palette,
                        format!("term-ctx-search-{name}"),
                        format!("{icon_prefix}Search Online · {name}"),
                        None,
                        cx.listener(move |this, _, _, cx| {
                            this.close_terminal_context_menu(cx);
                            let url = search_engine_url(&template, &query);
                            match open_external_url(&url) {
                                Ok(()) => {
                                    this.terminal_status = format!("opened online search: {name}");
                                }
                                Err(error) => {
                                    this.terminal_status = format!("online search failed: {error}");
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
                            this.ai_prompt_draft = format!(
                                "{prompt}

{body}"
                            );
                            this.ai_status = format!("AI action loaded: {name}");
                            window.focus(&this.ai_chat_focus);
                            cx.notify();
                        }),
                    )
                }))
                .children({
                    let selected = selected_for_translate.clone();
                    if translation_providers.is_empty() {
                        vec![
                            terminal_ctx_item(
                                palette,
                                "term-ctx-translate",
                                "Translate Selection",
                                None,
                                cx.listener(move |this, _, window, cx| {
                                    this.close_terminal_context_menu(cx);
                                    this.open_translation_dialog(
                                        selected.clone(),
                                        this.translate_provider.clone(),
                                        "Default".to_string(),
                                        window,
                                        cx,
                                    );
                                }),
                            )
                            .into_any_element(),
                        ]
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
                                    cx.listener(move |this, _, window, cx| {
                                        this.close_terminal_context_menu(cx);
                                        this.open_translation_dialog(
                                            selected.clone(),
                                            id.clone(),
                                            label.clone(),
                                            window,
                                            cx,
                                        );
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

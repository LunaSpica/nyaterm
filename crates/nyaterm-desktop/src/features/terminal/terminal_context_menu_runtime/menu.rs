use gpui::{ClipboardItem, Context};
use nyaterm_ui::NyaMenuItem;

use crate::action_links::{ActionLinkAction, actions_for_match, find_action_links};
use crate::features::NyaTermApp;
use crate::models::{NavItem, TerminalSearchMode};

use super::helpers::{
    available_translation_providers, open_external_url, search_engine_url,
    selection_as_openable_url,
};

impl NyaTermApp {
    pub(in crate::features) fn prepare_terminal_context_menu(&mut self, cx: &mut Context<Self>) {
        self.terminal.menus.action_link_menu = None;
        self.terminal.menus.action_link_tooltip = None;
        self.terminal.assist.command_suggestions = None;
        self.terminal.assist.credential_suggestions = None;
        self.shell
            .set_status("terminal context menu opened".to_string());
        cx.notify();
    }

    pub(in crate::features) fn terminal_context_menu_items(
        &mut self,
        selected: String,
        cx: &mut Context<Self>,
    ) -> Vec<NyaMenuItem> {
        let has_selection = !selected.is_empty();
        let shortcut = |id: &str, fallback: &str| self.display_shortcut_for(id, fallback);
        let copy_sc = shortcut("terminal.copy", "Ctrl+Shift+C");
        let paste_sc = shortcut("terminal.paste", "Ctrl+Shift+V");
        let paste_sel_sc = shortcut("terminal.pasteSelected", "Ctrl+Shift+X");
        let find_sc = shortcut("terminal.find", "Ctrl+Shift+F");
        let clear_sc = shortcut("terminal.clear", "Ctrl+L");
        let select_all_sc = shortcut("terminal.selectAll", "Ctrl+Shift+A");
        let mut items = Vec::new();

        if has_selection {
            let selected_for_copy = selected.clone();
            items.push(
                NyaMenuItem::action(self.tr("terminalCtx.copy"))
                    .icon("icons/copy.svg")
                    .shortcut(copy_sc)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(selected_for_copy.clone()));
                        this.shell
                            .set_status("copied terminal selection".to_string());
                        cx.notify();
                    })),
            );
            if let Some(url) = selection_as_openable_url(&selected) {
                let open_url = url.clone();
                items.push(
                    NyaMenuItem::action(self.tr("terminalCtx.openLink"))
                        .icon("icons/conn/connect.svg")
                        .on_click(cx.listener(move |this, _, _, cx| {
                            match open_external_url(&open_url) {
                                Ok(()) => {
                                    this.shell.set_status(format!("opened link: {open_url}"));
                                }
                                Err(error) => {
                                    this.shell.set_status(format!("open link failed: {error}"));
                                }
                            }
                            cx.notify();
                        })),
                );
            }

            items.extend(self.terminal_selection_action_link_items(&selected, cx));

            let selected_for_find = selected.clone();
            items.push(
                NyaMenuItem::action(self.tr("terminalCtx.find"))
                    .icon("icons/fe/search.svg")
                    .shortcut(find_sc)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.terminal.search.query = selected_for_find.clone();
                        this.terminal.search.mode = TerminalSearchMode::Buffer;
                        this.terminal.search.active_index = 0;
                        this.open_terminal_search(window, cx);
                    })),
            );

            let search_items = self.terminal_online_search_menu_items(&selected, cx);
            items.push(
                NyaMenuItem::submenu(self.tr("terminalCtx.searchOnline"), search_items)
                    .icon("icons/menu/travel-explore.svg"),
            );

            let ai_items = self.terminal_ai_context_menu_items(&selected, cx);
            if !ai_items.is_empty() {
                items
                    .push(NyaMenuItem::submenu(self.tr("ai.title"), ai_items).icon("icons/ai.svg"));
            }

            let translation_items = self.terminal_translation_menu_items(&selected, cx);
            if !translation_items.is_empty() {
                items.push(
                    NyaMenuItem::submenu(self.tr("terminalCtx.translate"), translation_items)
                        .icon("icons/translation.svg"),
                );
            }

            let selected_for_paste = selected.clone();
            items.extend([
                NyaMenuItem::separator(),
                NyaMenuItem::action(self.tr("terminalCtx.paste"))
                    .icon("icons/menu/paste.svg")
                    .shortcut(paste_sc)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.paste_from_clipboard(window, cx);
                    })),
                NyaMenuItem::action(self.tr("terminalCtx.pasteSelectedText"))
                    .icon("icons/menu/paste-go.svg")
                    .shortcut(paste_sel_sc)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.paste_terminal_text(selected_for_paste.clone(), window, cx);
                    })),
            ]);
        } else {
            items.extend([
                NyaMenuItem::action(self.tr("terminalCtx.paste"))
                    .icon("icons/menu/paste.svg")
                    .shortcut(paste_sc)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.paste_from_clipboard(window, cx);
                    })),
                NyaMenuItem::action(self.tr("terminalCtx.find"))
                    .icon("icons/fe/search.svg")
                    .shortcut(find_sc)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_terminal_search(window, cx);
                    })),
            ]);
        }

        items.extend([
            NyaMenuItem::separator(),
            NyaMenuItem::action(self.tr("terminalCtx.clearScreen"))
                .icon("icons/menu/clear-all.svg")
                .shortcut(clear_sc)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.send_terminal_clear_screen(cx);
                })),
            NyaMenuItem::action(self.tr("terminalCtx.clearAll"))
                .icon("icons/menu/delete-sweep.svg")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.clear_terminal(cx);
                })),
            NyaMenuItem::separator(),
            NyaMenuItem::action(self.tr("terminalCtx.selectAll"))
                .icon("icons/menu/select-all.svg")
                .shortcut(select_all_sc)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.select_all_terminal(cx);
                })),
            NyaMenuItem::action(self.tr("terminalCtx.moreActions"))
                .icon("icons/session/more.svg")
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_terminal_actions(window, cx);
                })),
        ]);
        items
    }

    fn terminal_selection_action_link_items(
        &mut self,
        selected: &str,
        cx: &mut Context<Self>,
    ) -> Vec<NyaMenuItem> {
        if !self.settings.summary().terminal_action_links_enabled {
            return Vec::new();
        }
        let trimmed = selected.trim();
        let matchers = &self.settings.summary().terminal_action_links_matchers;
        let entity = find_action_links(trimmed, matchers, true)
            .into_iter()
            .find(|item| item.text == trimmed || item.value == trimmed)
            .or_else(|| {
                find_action_links(trimmed, matchers, true)
                    .into_iter()
                    .next()
            });
        let Some(entity) = entity else {
            return Vec::new();
        };
        let kind = entity.kind.label().to_string();
        actions_for_match(&entity)
            .into_iter()
            .map(|action| self.terminal_action_link_menu_item(kind.clone(), action, cx))
            .collect()
    }

    fn terminal_action_link_menu_item(
        &mut self,
        kind: String,
        action: ActionLinkAction,
        cx: &mut Context<Self>,
    ) -> NyaMenuItem {
        let command = action.command.clone();
        let open_url = action.open_url.clone();
        NyaMenuItem::action(format!("{kind} · {}", action.label))
            .icon("icons/fe/forward.svg")
            .on_click(cx.listener(move |this, _, _, cx| {
                if let Some(url) = open_url.clone() {
                    match open_external_url(&url) {
                        Ok(()) => this.shell.set_status(format!("opened link: {url}")),
                        Err(error) => this.shell.set_status(format!("open link failed: {error}")),
                    }
                    cx.notify();
                } else if let Some(command) = command.clone() {
                    this.execute_action_link_command(command, cx);
                }
            }))
    }

    fn terminal_online_search_menu_items(
        &mut self,
        selected: &str,
        cx: &mut Context<Self>,
    ) -> Vec<NyaMenuItem> {
        self.settings
            .summary()
            .search_custom_engines
            .iter()
            .filter(|engine| {
                engine.show_in_menu
                    && !engine.name.trim().is_empty()
                    && !engine.url_template.trim().is_empty()
            })
            .cloned()
            .map(|engine| {
                let query = selected.to_string();
                let name = engine.name.clone();
                let status_name = name.clone();
                let template = engine.url_template;
                let icon = crate::features::search_engine_icon(
                    engine.icon.as_deref(),
                    self.theme_palette(),
                );
                NyaMenuItem::action(name)
                    .icon(icon.path)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        let url = search_engine_url(&template, &query);
                        match open_external_url(&url) {
                            Ok(()) => this
                                .shell
                                .set_status(format!("opened online search: {status_name}")),
                            Err(error) => this
                                .shell
                                .set_status(format!("online search failed: {error}")),
                        }
                        cx.notify();
                    }))
            })
            .collect()
    }

    fn terminal_ai_context_menu_items(
        &mut self,
        selected: &str,
        cx: &mut Context<Self>,
    ) -> Vec<NyaMenuItem> {
        if !self.ai.settings_config().enabled {
            return Vec::new();
        }
        self.ai
            .settings_config()
            .terminal_ai_actions
            .iter()
            .filter(|action| action.enabled && !action.name.trim().is_empty())
            .cloned()
            .map(|action| {
                let query = selected.to_string();
                let name = action.name.clone();
                let status_name = name.clone();
                let prompt = action.prompt;
                NyaMenuItem::action(name)
                    .icon("icons/ai.svg")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.ensure_panel_open(NavItem::AiAssistant);
                        let body = if query.chars().count() > 2_800 {
                            let clipped: String = query.chars().take(2_800).collect();
                            format!("{clipped}…")
                        } else {
                            query.clone()
                        };
                        this.set_ai_prompt_draft(format!("{prompt}\n\n{body}"), cx);
                        this.ai
                            .set_panel_status(format!("AI action loaded: {status_name}"));
                        window.focus(this.ai.chat_focus());
                        cx.notify();
                    }))
            })
            .collect()
    }

    fn terminal_translation_menu_items(
        &mut self,
        selected: &str,
        cx: &mut Context<Self>,
    ) -> Vec<NyaMenuItem> {
        available_translation_providers(self.translation.settings())
            .into_iter()
            .map(|(id, _)| {
                let label = self
                    .tr(match id.as_str() {
                        "google" => "translation.google",
                        "microsoft" => "translation.microsoft",
                        "deepl" => "translation.deepl",
                        "baidu" => "translation.baidu",
                        "ali" => "translation.ali",
                        "youdao" => "translation.youdao",
                        _ => "translation.provider",
                    })
                    .to_string();
                let selected = selected.to_string();
                let provider_id = id.clone();
                let provider_label = label.clone();
                NyaMenuItem::action(label)
                    .icon("icons/translation.svg")
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.open_translation_dialog(
                            selected.clone(),
                            provider_id.clone(),
                            provider_label.clone(),
                            window,
                            cx,
                        );
                    }))
            })
            .collect()
    }
}

use super::*;

use crate::models::SearchEngineEditorField;
use nyaterm_core::SearchEngineConfig;

impl NyaTermApp {
    pub(in crate::features) fn add_search_engine(&mut self, cx: &mut Context<Self>) {
        self.settings.search_custom_engines.insert(
            0,
            SearchEngineConfig {
                name: "New Engine".to_string(),
                url_template: "https://example.com/search?q=%s".to_string(),
                icon: None,
                show_in_menu: true,
            },
        );
        self.search_engine_expanded_index = Some(0);
        self.search_engine_edit_index = Some(0);
        self.search_engine_icon_picker_index = None;
        self.search_engine_actions_index = None;
        self.search_engine_edit_field = SearchEngineEditorField::Name;
        self.save_terminal_settings(cx);
        self.terminal_status = "search engine added".to_string();
    }

    pub(in crate::features) fn remove_search_engine(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        if index >= self.settings.search_custom_engines.len() {
            return;
        }
        self.settings.search_custom_engines.remove(index);
        self.search_engine_icon_picker_index = None;
        self.search_engine_actions_index = None;
        if self.search_engine_expanded_index == Some(index) {
            self.search_engine_expanded_index = None;
        } else if let Some(edit) = self.search_engine_expanded_index {
            if edit > index {
                self.search_engine_expanded_index = Some(edit - 1);
            }
        }
        if let Some(edit) = self.search_engine_edit_index {
            if edit == index {
                self.search_engine_edit_index = None;
            } else if edit > index {
                self.search_engine_edit_index = Some(edit - 1);
            }
        }
        self.save_terminal_settings(cx);
        self.terminal_status = "search engine removed".to_string();
    }

    pub(in crate::features) fn set_search_engine_icon(
        &mut self,
        index: usize,
        icon: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        let Some(engine) = self.settings.search_custom_engines.get_mut(index) else {
            return;
        };
        engine.icon = icon.map(str::to_string);
        self.search_engine_icon_picker_index = None;
        self.save_terminal_settings(cx);
        self.terminal_status = "search engine icon updated".to_string();
    }

    pub(in crate::features) fn toggle_search_engine_in_menu(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        let Some(engine) = self.settings.search_custom_engines.get_mut(index) else {
            return;
        };
        engine.show_in_menu = !engine.show_in_menu;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn focus_search_engine_field(
        &mut self,
        index: usize,
        field: SearchEngineEditorField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if index >= self.settings.search_custom_engines.len() {
            return;
        }
        self.search_engine_edit_index = Some(index);
        self.search_engine_edit_field = field;
        window.focus(&self.search_engine_focus);
        cx.notify();
    }

    pub(in crate::features) fn handle_search_engine_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let Some(index) = self.search_engine_edit_index else {
            return;
        };
        if index >= self.settings.search_custom_engines.len() {
            return;
        }
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }
        match keystroke.key.as_str() {
            "backspace" => {
                let engine = &mut self.settings.search_custom_engines[index];
                match self.search_engine_edit_field {
                    SearchEngineEditorField::Name => {
                        engine.name.pop();
                    }
                    SearchEngineEditorField::Url => {
                        engine.url_template.pop();
                    }
                }
                self.terminal_status = "search engine edited".to_string();
                cx.notify();
            }
            "tab" => {
                self.search_engine_edit_field = self.search_engine_edit_field.next();
                cx.notify();
            }
            "enter" => {
                self.normalize_search_engines();
                self.save_terminal_settings(cx);
                self.terminal_status = "search engines saved".to_string();
            }
            "escape" => {
                self.search_engine_edit_index = None;
                self.terminal_status = "search engine input blurred".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    let engine = &mut self.settings.search_custom_engines[index];
                    match self.search_engine_edit_field {
                        SearchEngineEditorField::Name => engine.name.push_str(input),
                        SearchEngineEditorField::Url => engine.url_template.push_str(input),
                    }
                    self.terminal_status = "search engine edited".to_string();
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn expand_search_engine(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        if self.search_engine_expanded_index == Some(index) {
            self.search_engine_expanded_index = None;
            self.search_engine_edit_index = None;
        } else {
            self.search_engine_expanded_index = Some(index);
        }
        self.search_engine_icon_picker_index = None;
        self.search_engine_actions_index = None;
        cx.notify();
    }

    pub(in crate::features) fn test_search_engine(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(engine) = self.settings.search_custom_engines.get(index) else {
            self.terminal_status = "search engine not found".to_string();
            cx.notify();
            return;
        };
        if !engine.url_template.contains("%s") {
            self.terminal_status = "search engine URL must include %s".to_string();
            cx.notify();
            return;
        }
        let url = engine
            .url_template
            .replace("%s", &urlencoding_query("nyaterm"));
        match open_external_url_simple(&url) {
            Ok(()) => {
                self.terminal_status = format!("tested search engine: {}", engine.name);
            }
            Err(error) => {
                self.terminal_status = format!("test search engine failed: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn toggle_terminal_action_links(&mut self, cx: &mut Context<Self>) {
        self.settings.terminal_action_links_enabled = !self.settings.terminal_action_links_enabled;
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn toggle_terminal_action_links_matcher(
        &mut self,
        which: &'static str,
        cx: &mut Context<Self>,
    ) {
        match which {
            "ipv4" => {
                self.settings.terminal_action_links_matchers.ipv4 =
                    !self.settings.terminal_action_links_matchers.ipv4;
            }
            "archive" => {
                self.settings.terminal_action_links_matchers.archive =
                    !self.settings.terminal_action_links_matchers.archive;
            }
            "host_port" => {
                self.settings.terminal_action_links_matchers.host_port =
                    !self.settings.terminal_action_links_matchers.host_port;
            }
            _ => return,
        }
        self.save_terminal_settings(cx);
    }

    pub(in crate::features) fn execute_action_link_command(
        &mut self,
        command: String,
        cx: &mut Context<Self>,
    ) {
        let mut bytes = command.into_bytes();
        if !bytes.ends_with(b"\n") {
            bytes.push(b'\n');
        }
        if self.send_terminal_input(bytes, cx) {
            self.terminal_status = "action link command sent".to_string();
            cx.notify();
        }
    }

    pub(in crate::features) fn normalize_search_engines(&mut self) {
        for engine in &mut self.settings.search_custom_engines {
            engine.name = engine.name.trim().to_string();
            engine.url_template = engine.url_template.trim().to_string();
        }
    }
}

use super::*;

use crate::models::SearchEngineEditorField;
use nyaterm_core::SearchEngineConfig;

impl NyaTermApp {
    /// Apply an edit from one of the engine editor's inputs.
    ///
    /// `rest` is what follows `settings.search-engine.` in the field id:
    /// `<index>.name` or `<index>.url`.
    pub(in crate::features) fn apply_search_engine_input(
        &mut self,
        rest: &str,
        text: String,
        cx: &mut Context<Self>,
    ) {
        let Some((index, field)) = rest.split_once('.') else {
            return;
        };
        let Ok(index) = index.parse::<usize>() else {
            return;
        };
        let Some(engine) = self.settings.search_custom_engines.get_mut(index) else {
            return;
        };
        match field {
            "name" => engine.name = text,
            "url" => engine.url_template = text,
            _ => return,
        }
        // Persist as it is typed, the way every other settings control does.
        // Trimming waits for the row to close, so a space mid-word survives.
        self.save_terminal_settings(cx);
        self.terminal.view.status = "search engine edited".to_string();
        cx.notify();
    }

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
        // The inputs are keyed by row index and every row just shifted down, so
        // they have to be rebuilt from the engines they now stand for.
        self.forget_text_inputs("settings.search-engine.");
        self.save_terminal_settings(cx);
        self.terminal.view.status = "search engine added".to_string();
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
        self.forget_text_inputs("settings.search-engine.");
        self.save_terminal_settings(cx);
        self.terminal.view.status = "search engine removed".to_string();
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
        self.terminal.view.status = "search engine icon updated".to_string();
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

    pub(in crate::features) fn expand_search_engine(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        if self.search_engine_expanded_index == Some(index) {
            self.search_engine_expanded_index = None;
            self.search_engine_edit_index = None;
            self.normalize_search_engines();
            self.save_terminal_settings(cx);
        } else {
            self.search_engine_expanded_index = Some(index);
        }
        self.search_engine_icon_picker_index = None;
        self.search_engine_actions_index = None;
        cx.notify();
    }

    pub(in crate::features) fn test_search_engine(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(engine) = self.settings.search_custom_engines.get(index) else {
            self.terminal.view.status = "search engine not found".to_string();
            cx.notify();
            return;
        };
        if !engine.url_template.contains("%s") {
            self.terminal.view.status = "search engine URL must include %s".to_string();
            cx.notify();
            return;
        }
        let url = engine
            .url_template
            .replace("%s", &urlencoding_query("nyaterm"));
        match open_external_url_simple(&url) {
            Ok(()) => {
                self.terminal.view.status = format!("tested search engine: {}", engine.name);
            }
            Err(error) => {
                self.terminal.view.status = format!("test search engine failed: {error}");
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
            self.terminal.view.status = "action link command sent".to_string();
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

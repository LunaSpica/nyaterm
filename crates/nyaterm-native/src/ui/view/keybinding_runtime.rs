use super::*;

impl NyaTermApp {

    /// First display chord for empty-workspace / UI labels (Tauri-style chips).
    pub(in crate::ui::view) fn display_shortcut_for(
        &self,
        id: &str,
        fallback: &str,
    ) -> String {
        use crate::ui::shortcuts::{format_hotkey_for_display, shortcut_keys_for};
        let raw = shortcut_keys_for(id, &self.settings.keybindings)
            .unwrap_or_else(|| fallback.to_string());
        let display = format_hotkey_for_display(&raw);
        display
            .split(" / ")
            .next()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(fallback)
            .to_string()
    }
    pub(in crate::ui::view) fn start_keybinding_recording(
        &mut self,
        shortcut_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.keybinding_recording_id = Some(shortcut_id);
        self.keybinding_pending_keys = None;
        self.terminal_status = "recording shortcut".to_string();
        window.focus(&self.keybindings_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn cancel_keybinding_recording(&mut self, cx: &mut Context<Self>) {
        self.keybinding_recording_id = None;
        self.keybinding_pending_keys = None;
        self.terminal_status = "shortcut recording cancelled".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn confirm_keybinding_recording(&mut self, cx: &mut Context<Self>) {
        let Some(shortcut_id) = self.keybinding_recording_id.take() else {
            self.terminal_status = "no shortcut recording is active".to_string();
            cx.notify();
            return;
        };
        let Some(keys) = self.keybinding_pending_keys.take() else {
            self.keybinding_recording_id = Some(shortcut_id);
            self.terminal_status = "press a shortcut before saving".to_string();
            cx.notify();
            return;
        };

        if let Some(conflict) = self.keybinding_conflict_label(&keys, &shortcut_id) {
            self.keybinding_recording_id = Some(shortcut_id);
            self.keybinding_pending_keys = Some(keys);
            self.terminal_status = format!("shortcut conflicts with {conflict}");
            cx.notify();
            return;
        }
        let mut keybindings = self.settings.keybindings.clone();
        let is_default = crate::ui::shortcuts::SHORTCUT_REGISTRY
            .iter()
            .find(|s| s.id == shortcut_id)
            .is_some_and(|def| keys == def.default_keys);
        if is_default {
            keybindings.remove(&shortcut_id);
        } else {
            keybindings.insert(shortcut_id.clone(), keys);
        }
        self.save_keybindings(keybindings, format!("shortcut {shortcut_id} saved"), cx);
    }

    pub(in crate::ui::view) fn reset_keybinding(
        &mut self,
        shortcut_id: String,
        cx: &mut Context<Self>,
    ) {
        let mut keybindings = self.settings.keybindings.clone();
        keybindings.remove(&shortcut_id);
        self.save_keybindings(keybindings, format!("shortcut {shortcut_id} reset"), cx);
    }

    pub(in crate::ui::view) fn reset_all_keybindings(&mut self, cx: &mut Context<Self>) {
        self.save_keybindings(HashMap::new(), "all shortcuts reset".to_string(), cx);
    }

    fn save_keybindings(
        &mut self,
        keybindings: HashMap<String, String>,
        success_message: String,
        cx: &mut Context<Self>,
    ) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_keybindings(&keybindings))
        {
            Ok(settings) => {
                self.settings = settings;
                self.keybinding_recording_id = None;
                self.keybinding_pending_keys = None;
                self.store_status.message = success_message.clone();
                self.store_status.ready = true;
                self.terminal_status = success_message;
            }
            Err(error) => {
                self.store_status.message = format!("shortcut save failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_keybinding_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        let Some(recording_id) = self.keybinding_recording_id.clone() else {
            return;
        };
        match event.keystroke.key.as_str() {
            "escape" => {
                self.cancel_keybinding_recording(cx);
                return;
            }
            "enter" if self.keybinding_pending_keys.is_some() => {
                self.confirm_keybinding_recording(cx);
                return;
            }
            _ => {}
        }

        let Some(keys) = event_to_hotkey_string(event) else {
            return;
        };
        if recording_id == "tab.switchTo"
            && !crate::ui::shortcuts::is_indexed_shortcut_template(&keys)
        {
            self.keybinding_pending_keys = None;
            self.terminal_status = "tab switch shortcut must end with number 1".to_string();
            cx.notify();
            return;
        }
        self.keybinding_pending_keys = Some(keys);
        self.terminal_status = "shortcut captured; press Enter or Save".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_keyword_highlights(&mut self, cx: &mut Context<Self>) {
        self.keyword_highlights.enabled = !self.keyword_highlights.enabled;
        self.save_keyword_highlights(cx);
    }

    pub(in crate::ui::view) fn toggle_keyword_highlights_wrapped(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.keyword_highlights.across_wrapped_lines =
            !self.keyword_highlights.across_wrapped_lines;
        self.save_keyword_highlights(cx);
    }

    fn save_keyword_highlights(&mut self, cx: &mut Context<Self>) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_keyword_highlights(&self.keyword_highlights))
        {
            Ok(config) => {
                self.keyword_highlights = config;
                self.store_status.message = "keyword highlight settings saved".to_string();
                self.store_status.ready = true;
                self.terminal_status = "keyword highlight settings saved".to_string();
            }
            Err(error) => {
                self.store_status.message =
                    format!("keyword highlight settings save failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn prompt_keyword_highlight_import(&mut self, cx: &mut Context<Self>) {
        if self.keyword_highlight_path_prompt.is_some() {
            self.terminal_status = "keyword highlight import picker is already open".to_string();
            cx.notify();
            return;
        }
        let options = PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from("Import keyword highlight JSON")),
        };
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        let receiver = cx.prompt_for_paths(options);
        self.keyword_highlight_path_prompt = Some(KeywordHighlightPathPromptKind::Import);
        self.terminal_status = "selecting keyword highlight import file".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(paths))) => match paths.into_iter().next() {
                    Some(path) => match std::fs::read_to_string(&path) {
                        Ok(raw) => match ConnectionStore::open_with_portable_key_path(
                            &config_dir,
                            portable_key_path.clone(),
                        )
                        .and_then(|store| store.import_keyword_highlights_json(&raw))
                        {
                            Ok((_, result)) => KeywordHighlightPathPromptResult::Imported {
                                imported_rules: result.imported_rules,
                                updated_rules: result.updated_rules,
                                total_rules: result.total_rules,
                            },
                            Err(error) => {
                                KeywordHighlightPathPromptResult::Failed(error.to_string())
                            }
                        },
                        Err(error) => KeywordHighlightPathPromptResult::Failed(error.to_string()),
                    },
                    None => KeywordHighlightPathPromptResult::Cancelled,
                },
                Ok(Ok(None)) => KeywordHighlightPathPromptResult::Cancelled,
                Ok(Err(error)) => KeywordHighlightPathPromptResult::Failed(error.to_string()),
                Err(_) => KeywordHighlightPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_keyword_highlight_import_result(result);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn apply_keyword_highlight_import_result(&mut self, result: KeywordHighlightPathPromptResult) {
        self.keyword_highlight_path_prompt = None;
        match result {
            KeywordHighlightPathPromptResult::Imported {
                imported_rules,
                updated_rules,
                total_rules,
            } => {
                self.refresh_keyword_highlights();
                self.terminal_status = format!(
                    "imported {imported_rules} keyword highlight rule(s), updated {updated_rules}, total {total_rules}"
                );
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = true;
            }
            KeywordHighlightPathPromptResult::Cancelled => {
                self.terminal_status = "keyword highlight import cancelled".to_string();
            }
            KeywordHighlightPathPromptResult::Failed(error) => {
                self.terminal_status = format!("keyword highlight import failed: {error}");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
            }
            KeywordHighlightPathPromptResult::Closed => {
                self.terminal_status =
                    "keyword highlight import picker closed before returning".to_string();
            }
        }
    }

    pub(in crate::ui::view) fn refresh_keyword_highlights(&mut self) {
        if let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            if let Ok(config) = store.load_keyword_highlights() {
                self.keyword_highlights = config;
            }
        }
    }

    pub(in crate::ui::view) fn toggle_keyword_highlight_builtin(
        &mut self,
        rule_id: String,
        cx: &mut Context<Self>,
    ) {
        let enabled = self
            .keyword_highlights
            .builtin_rules
            .get(&rule_id)
            .copied()
            .unwrap_or(true);
        self.keyword_highlights
            .builtin_rules
            .insert(rule_id, !enabled);
        self.save_keyword_highlights(cx);
    }

    pub(in crate::ui::view) fn toggle_keyword_highlight_rule(
        &mut self,
        rule_id: String,
        cx: &mut Context<Self>,
    ) {
        if let Some(rule) = self
            .keyword_highlights
            .rules
            .iter_mut()
            .find(|rule| rule.id == rule_id)
        {
            rule.enabled = !rule.enabled;
            self.save_keyword_highlights(cx);
        }
    }

    pub(in crate::ui::view) fn expand_keyword_highlight_rule(
        &mut self,
        rule_id: String,
        cx: &mut Context<Self>,
    ) {
        if self.keyword_highlight_expanded_id.as_deref() == Some(rule_id.as_str()) {
            self.keyword_highlight_expanded_id = None;
            self.keyword_highlight_edit_id = None;
        } else {
            self.keyword_highlight_expanded_id = Some(rule_id);
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn add_keyword_highlight_rule(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = format!(
            "kh-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );
        self.keyword_highlights.rules.push(KeywordHighlightRule {
            id: id.clone(),
            name: "New rule".to_string(),
            patterns: Vec::new(),
            color_dark: "#79c0ff".to_string(),
            color_light: "#0969da".to_string(),
            enabled: true,
        });
        self.keyword_highlight_expanded_id = Some(id.clone());
        self.keyword_highlight_edit_id = Some(id);
        self.keyword_highlight_edit_field = KeywordHighlightEditorField::Name;
        window.focus(&self.keyword_highlight_focus);
        self.save_keyword_highlights(cx);
    }

    pub(in crate::ui::view) fn remove_keyword_highlight_rule(
        &mut self,
        rule_id: String,
        cx: &mut Context<Self>,
    ) {
        self.keyword_highlights
            .rules
            .retain(|rule| rule.id != rule_id);
        if self.keyword_highlight_expanded_id.as_deref() == Some(rule_id.as_str()) {
            self.keyword_highlight_expanded_id = None;
        }
        if self.keyword_highlight_edit_id.as_deref() == Some(rule_id.as_str()) {
            self.keyword_highlight_edit_id = None;
        }
        self.save_keyword_highlights(cx);
    }

    pub(in crate::ui::view) fn set_keyword_highlight_rule_color(
        &mut self,
        rule_id: String,
        dark: bool,
        color: &str,
        cx: &mut Context<Self>,
    ) {
        let color = color.trim();
        if !(color.starts_with('#') && (color.len() == 4 || color.len() == 7))
            && !color.is_empty()
            && color != "#"
        {
            // allow progressive hex entry only when matching /^#[0-9a-fA-F]{0,6}$/
        }
        if !color.is_empty() && !color.chars().enumerate().all(|(i, ch)| {
            if i == 0 {
                ch == '#'
            } else {
                ch.is_ascii_hexdigit()
            }
        }) {
            return;
        }
        if color.len() > 7 {
            return;
        }
        if let Some(rule) = self
            .keyword_highlights
            .rules
            .iter_mut()
            .find(|rule| rule.id == rule_id)
        {
            if dark {
                rule.color_dark = if color.is_empty() {
                    "#79c0ff".into()
                } else {
                    color.to_string()
                };
            } else {
                rule.color_light = if color.is_empty() {
                    "#0969da".into()
                } else {
                    color.to_string()
                };
            }
            self.save_keyword_highlights(cx);
        }
    }

    pub(in crate::ui::view) fn focus_keyword_highlight_field(
        &mut self,
        rule_id: String,
        field: KeywordHighlightEditorField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.keyword_highlight_expanded_id = Some(rule_id.clone());
        self.keyword_highlight_edit_id = Some(rule_id);
        self.keyword_highlight_edit_field = field;
        window.focus(&self.keyword_highlight_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_keyword_highlight_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        let Some(rule_id) = self.keyword_highlight_edit_id.clone() else {
            return;
        };
        let field = self.keyword_highlight_edit_field;
        match event.keystroke.key.as_str() {
            "escape" => {
                self.keyword_highlight_edit_id = None;
                self.terminal_status = "keyword rule edit cancelled".to_string();
                cx.notify();
                return;
            }
            "tab" => {
                self.keyword_highlight_edit_field = field.next();
                cx.notify();
                return;
            }
            "enter" if field == KeywordHighlightEditorField::Patterns => {
                if let Some(rule) = self
                    .keyword_highlights
                    .rules
                    .iter_mut()
                    .find(|rule| rule.id == rule_id)
                {
                    rule.patterns.push(String::new());
                    self.save_keyword_highlights(cx);
                }
                return;
            }
            "enter" => {
                self.keyword_highlight_edit_id = None;
                self.save_keyword_highlights(cx);
                return;
            }
            "backspace" => {
                if let Some(rule) = self
                    .keyword_highlights
                    .rules
                    .iter_mut()
                    .find(|rule| rule.id == rule_id)
                {
                    match field {
                        KeywordHighlightEditorField::Name => {
                            rule.name.pop();
                        }
                        KeywordHighlightEditorField::Patterns => {
                            if let Some(last) = rule.patterns.last_mut() {
                                if last.is_empty() {
                                    rule.patterns.pop();
                                } else {
                                    last.pop();
                                }
                            }
                        }
                        KeywordHighlightEditorField::ColorDark => {
                            if rule.color_dark.len() > 1 {
                                rule.color_dark.pop();
                            }
                        }
                        KeywordHighlightEditorField::ColorLight => {
                            if rule.color_light.len() > 1 {
                                rule.color_light.pop();
                            }
                        }
                    }
                    self.save_keyword_highlights(cx);
                }
                return;
            }
            _ => {}
        }

        let Some(input) = event.keystroke.key_char.as_deref() else {
            return;
        };
        if input.is_empty() {
            return;
        }
        if let Some(rule) = self
            .keyword_highlights
            .rules
            .iter_mut()
            .find(|rule| rule.id == rule_id)
        {
            match field {
                KeywordHighlightEditorField::Name => {
                    rule.name.push_str(input);
                }
                KeywordHighlightEditorField::Patterns => {
                    if input == "\n" || event.keystroke.key.as_str() == "enter" {
                        rule.patterns.push(String::new());
                    } else {
                        if rule.patterns.is_empty() {
                            rule.patterns.push(String::new());
                        }
                        if let Some(last) = rule.patterns.last_mut() {
                            last.push_str(input);
                        }
                    }
                }
                KeywordHighlightEditorField::ColorDark => {
                    for ch in input.chars() {
                        if rule.color_dark.len() >= 7 {
                            break;
                        }
                        if rule.color_dark.is_empty() {
                            rule.color_dark.push('#');
                        }
                        if ch == '#' && rule.color_dark == "#" {
                            continue;
                        }
                        if ch.is_ascii_hexdigit() {
                            rule.color_dark.push(ch.to_ascii_lowercase());
                        }
                    }
                }
                KeywordHighlightEditorField::ColorLight => {
                    for ch in input.chars() {
                        if rule.color_light.len() >= 7 {
                            break;
                        }
                        if rule.color_light.is_empty() {
                            rule.color_light.push('#');
                        }
                        if ch == '#' && rule.color_light == "#" {
                            continue;
                        }
                        if ch.is_ascii_hexdigit() {
                            rule.color_light.push(ch.to_ascii_lowercase());
                        }
                    }
                }
            }
            self.save_keyword_highlights(cx);
        }
    }

    pub(in crate::ui::view) fn keybinding_conflict_label(
        &self,
        pending_keys: &str,
        exclude_id: &str,
    ) -> Option<String> {
        let normalized_new = pending_keys
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_ascii_lowercase())
            .collect::<Vec<_>>();
        if normalized_new.is_empty() {
            return None;
        }
        for shortcut in crate::ui::shortcuts::SHORTCUT_REGISTRY.iter() {
            if shortcut.id == exclude_id {
                continue;
            }
            let existing = crate::ui::shortcuts::shortcut_keys_for(
                shortcut.id,
                &self.settings.keybindings,
            )
            .unwrap_or_else(|| shortcut.default_keys.to_string());
            let normalized_existing = existing
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_ascii_lowercase())
                .collect::<Vec<_>>();
            if normalized_new
                .iter()
                .any(|n| normalized_existing.iter().any(|e| e == n))
            {
                return Some(shortcut.label.to_string());
            }
        }
        None
    }

    pub(in crate::ui::view) fn handle_keybinding_search_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        match event.keystroke.key.as_str() {
            "escape" => {
                self.keybinding_search_draft.clear();
                cx.notify();
                return;
            }
            "backspace" => {
                self.keybinding_search_draft.pop();
                cx.notify();
                return;
            }
            _ => {}
        }
        if let Some(input) = event.keystroke.key_char.as_deref() {
            self.keybinding_search_draft.push_str(input);
            cx.notify();
        }
    }

}

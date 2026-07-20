use super::*;

const MAX_KEYWORD_HIGHLIGHT_IMPORT_BYTES: u64 = 4 * 1024 * 1024;

impl NyaTermApp {
    pub(in crate::features) fn toggle_keyword_highlights(&mut self, cx: &mut Context<Self>) {
        self.keyword_highlights.enabled = !self.keyword_highlights.enabled;
        self.save_keyword_highlights(cx);
    }

    pub(in crate::features) fn toggle_keyword_highlights_wrapped(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.keyword_highlights.across_wrapped_lines =
            !self.keyword_highlights.across_wrapped_lines;
        self.save_keyword_highlights(cx);
    }

    fn save_keyword_highlights(&mut self, cx: &mut Context<Self>) {
        self.invalidate_paint_theme_caches();
        if self.defer_settings_persistence(cx) {
            return;
        }
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

    pub(in crate::features) fn prompt_keyword_highlight_import(&mut self, cx: &mut Context<Self>) {
        if self.block_import_for_settings_draft(cx) {
            return;
        }
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
                    Some(path) => {
                        cx.background_spawn(async move {
                            match read_keyword_highlight_import_text(&path) {
                                Ok(raw) => match ConnectionStore::open_with_portable_key_path(
                                    &config_dir,
                                    portable_key_path,
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
                                Err(error) => {
                                    KeywordHighlightPathPromptResult::Failed(error.to_string())
                                }
                            }
                        })
                        .await
                    }
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
                self.rebase_open_settings_draft();
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

    pub(in crate::features) fn refresh_keyword_highlights(&mut self) {
        self.invalidate_paint_theme_caches();
        if let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            if let Ok(config) = store.load_keyword_highlights() {
                self.keyword_highlights = config;
            }
        }
    }

    pub(in crate::features) fn toggle_keyword_highlight_builtin(
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

    pub(in crate::features) fn toggle_keyword_highlight_rule(
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

    pub(in crate::features) fn expand_keyword_highlight_rule(
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

    pub(in crate::features) fn add_keyword_highlight_rule(
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

    pub(in crate::features) fn remove_keyword_highlight_rule(
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

    pub(in crate::features) fn set_keyword_highlight_rule_color(
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
        if !color.is_empty()
            && !color.chars().enumerate().all(|(i, ch)| {
                if i == 0 {
                    ch == '#'
                } else {
                    ch.is_ascii_hexdigit()
                }
            })
        {
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

    pub(in crate::features) fn focus_keyword_highlight_field(
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

    pub(in crate::features) fn handle_keyword_highlight_key_down(
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
}

fn read_keyword_highlight_import_text(path: &std::path::Path) -> Result<String, String> {
    let metadata = std::fs::metadata(path).map_err(|error| error.to_string())?;
    if metadata.len() > MAX_KEYWORD_HIGHLIGHT_IMPORT_BYTES {
        return Err(format!(
            "import file is too large to import ({} bytes > {} bytes)",
            metadata.len(),
            MAX_KEYWORD_HIGHLIGHT_IMPORT_BYTES
        ));
    }
    std::fs::read_to_string(path).map_err(|error| error.to_string())
}

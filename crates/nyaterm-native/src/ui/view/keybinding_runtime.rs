use super::*;

impl NyaTermApp {
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

        let mut keybindings = self.settings.keybindings.clone();
        keybindings.insert(shortcut_id.clone(), keys);
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
}

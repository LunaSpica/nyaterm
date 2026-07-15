use super::*;

impl NyaTermApp {
    /// First display chord for empty-workspace / UI labels (Tauri-style chips).
    pub(in crate::features) fn display_shortcut_for(&self, id: &str, fallback: &str) -> String {
        use crate::shortcuts::{format_hotkey_for_display, shortcut_keys_for};
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
    pub(in crate::features) fn start_keybinding_recording(
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

    pub(in crate::features) fn cancel_keybinding_recording(&mut self, cx: &mut Context<Self>) {
        self.keybinding_recording_id = None;
        self.keybinding_pending_keys = None;
        self.terminal_status = "shortcut recording cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn confirm_keybinding_recording(&mut self, cx: &mut Context<Self>) {
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
        let is_default = crate::shortcuts::SHORTCUT_REGISTRY
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

    pub(in crate::features) fn reset_keybinding(
        &mut self,
        shortcut_id: String,
        cx: &mut Context<Self>,
    ) {
        let mut keybindings = self.settings.keybindings.clone();
        keybindings.remove(&shortcut_id);
        self.save_keybindings(keybindings, format!("shortcut {shortcut_id} reset"), cx);
    }

    pub(in crate::features) fn reset_all_keybindings(&mut self, cx: &mut Context<Self>) {
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
                self.apply_gpui_settings(settings);
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

    pub(in crate::features) fn handle_keybinding_key_down(
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
        if recording_id == "tab.switchTo" && !crate::shortcuts::is_indexed_shortcut_template(&keys)
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

    pub(in crate::features) fn keybinding_conflict_label(
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
        for shortcut in crate::shortcuts::SHORTCUT_REGISTRY.iter() {
            if shortcut.id == exclude_id {
                continue;
            }
            let existing =
                crate::shortcuts::shortcut_keys_for(shortcut.id, &self.settings.keybindings)
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

    pub(in crate::features) fn handle_keybinding_search_key_down(
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

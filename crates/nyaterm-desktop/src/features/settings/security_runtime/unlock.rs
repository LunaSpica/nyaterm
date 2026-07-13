use super::*;

impl NyaTermApp {
    pub(in crate::features) fn security_secrets_locked(&self) -> bool {
        self.settings.has_master_password && !self.security_secrets_unlocked
    }

    pub(in crate::features) fn require_security_secrets_unlocked(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.security_secrets_locked() {
            return true;
        }
        self.open_security_unlock_prompt(window, cx);
        false
    }

    pub(in crate::features) fn open_security_unlock_prompt(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.settings.has_master_password {
            self.security_secrets_unlocked = true;
            self.security_status = "no master password required".to_string();
            cx.notify();
            return;
        }
        self.security_unlock_prompt_open = true;
        self.security_unlock_draft.clear();
        self.security_unlock_error = None;
        self.security_status = "enter master password to unlock secrets".to_string();
        window.focus(&self.security_unlock_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_security_unlock_prompt(&mut self, cx: &mut Context<Self>) {
        self.security_unlock_prompt_open = false;
        self.security_unlock_draft.clear();
        self.security_unlock_error = None;
        cx.notify();
    }

    pub(in crate::features) fn lock_security_secrets(&mut self, cx: &mut Context<Self>) {
        self.security_secrets_unlocked = false;
        self.security_revealed_passwords.clear();
        self.security_revealed_credentials.clear();
        self.security_otp_codes.clear();
        self.security_unlock_prompt_open = false;
        self.security_unlock_draft.clear();
        self.security_unlock_error = None;
        self.security_status = "secrets locked".to_string();
        cx.notify();
    }

    pub(in crate::features) fn submit_security_unlock(&mut self, cx: &mut Context<Self>) {
        if !self.settings.has_master_password {
            self.security_secrets_unlocked = true;
            self.close_security_unlock_prompt(cx);
            return;
        }
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.verify_master_password(&self.security_unlock_draft))
        {
            Ok(true) => {
                self.security_secrets_unlocked = true;
                self.security_status = "secrets unlocked".to_string();
                self.close_security_unlock_prompt(cx);
            }
            Ok(false) => {
                self.security_unlock_draft.clear();
                self.security_unlock_error = Some("Wrong master password.".to_string());
                self.security_status = "unlock rejected".to_string();
                cx.notify();
            }
            Err(error) => {
                self.security_unlock_draft.clear();
                self.security_unlock_error = Some(error.to_string());
                self.security_status = "unlock failed".to_string();
                cx.notify();
            }
        }
    }

    pub(in crate::features) fn handle_security_unlock_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }
        match keystroke.key.as_str() {
            "enter" => self.submit_security_unlock(cx),
            "escape" => self.close_security_unlock_prompt(cx),
            "backspace" => {
                self.security_unlock_draft.pop();
                self.security_unlock_error = None;
                cx.notify();
            }
            _ => {
                if let Some(value) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|value| !value.is_empty())
                {
                    self.security_unlock_draft.push_str(value);
                    self.security_unlock_error = None;
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn set_security_auth_tab(
        &mut self,
        tab: SecurityAuthTab,
        cx: &mut Context<Self>,
    ) {
        self.security_auth_tab = tab;
        self.security_status = format!("{} tab", self.security_auth_tab.label().to_lowercase());
        cx.notify();
    }

    pub(in crate::features) fn refresh_security_catalog(&mut self) {
        if let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            self.connection_ssh_keys = store.list_ssh_keys().unwrap_or_default();
            self.connection_otp_entries = store.list_otp_entries().unwrap_or_default();
            self.connection_saved_passwords = store.list_passwords().unwrap_or_default();
            self.connection_saved_credentials = store.list_credentials().unwrap_or_default();
        }
    }
}

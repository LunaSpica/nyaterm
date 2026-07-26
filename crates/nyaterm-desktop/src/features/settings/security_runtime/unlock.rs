use super::*;

use crate::models::{SecurityAuthTab, SecurityUnlockAction, SettingsTab};

impl NyaTermApp {
    pub(in crate::features) fn security_secrets_locked(&self) -> bool {
        self.settings.has_master_password && !self.security.unlock.secrets_unlocked
    }

    pub(in crate::features) fn require_security_secrets_unlocked(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        pending_action: Option<SecurityUnlockAction>,
    ) -> bool {
        if self.settings.has_master_password && self.security.unlock.secrets_unlocked {
            return true;
        }
        self.security.unlock.pending_action = pending_action;
        self.open_security_unlock_prompt(window, cx);
        false
    }

    pub(in crate::features) fn open_security_unlock_prompt(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.settings.has_master_password {
            self.security.unlock.pending_action = None;
            self.security.unlock.prompt_open = false;
            self.security.unlock.master_required_prompt_open = true;
            self.security.unlock.draft.clear();
            self.security.unlock.marked_text.clear();
            self.security.unlock.error = None;
            self.security.status = "master password required".to_string();
            cx.notify();
            return;
        }
        self.security.unlock.master_required_prompt_open = false;
        self.security.unlock.prompt_open = true;
        self.security.unlock.draft.clear();
        self.security.unlock.marked_text.clear();
        self.security.unlock.error = None;
        self.security.status = "enter master password to unlock secrets".to_string();
        window.focus(&self.security.unlock.focus);
        cx.notify();
    }

    pub(in crate::features) fn close_security_unlock_prompt(&mut self, cx: &mut Context<Self>) {
        self.security.unlock.prompt_open = false;
        self.security.unlock.draft.clear();
        self.security.unlock.marked_text.clear();
        self.security.unlock.error = None;
        cx.notify();
    }

    pub(in crate::features) fn cancel_security_unlock_prompt(&mut self, cx: &mut Context<Self>) {
        self.security.unlock.pending_action = None;
        self.close_security_unlock_prompt(cx);
    }

    pub(in crate::features) fn close_security_master_required_prompt(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.security.unlock.master_required_prompt_open = false;
        self.security.unlock.pending_action = None;
        cx.notify();
    }

    pub(in crate::features) fn open_security_settings_from_prompt(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.security.unlock.master_required_prompt_open = false;
        self.security.unlock.pending_action = None;
        self.settings_active_tab = SettingsTab::Security;
        self.open_page(NavItem::Settings, cx);
    }

    pub(in crate::features) fn lock_security_secrets(&mut self, cx: &mut Context<Self>) {
        self.security.unlock.secrets_unlocked = false;
        self.security.revealed.passwords.clear();
        self.security.revealed.credentials.clear();
        self.security.editors.password = None;
        self.security.editors.credential = None;
        self.security.delete_confirm = None;
        self.security.unlock.pending_action = None;
        self.security.unlock.prompt_open = false;
        self.security.unlock.master_required_prompt_open = false;
        self.security.unlock.draft.clear();
        self.security.unlock.marked_text.clear();
        self.security.unlock.error = None;
        self.security.status = "secrets locked".to_string();
        cx.notify();
    }

    pub(in crate::features) fn submit_security_unlock(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.settings.has_master_password {
            self.security.unlock.pending_action = None;
            self.security.unlock.prompt_open = false;
            self.security.unlock.master_required_prompt_open = true;
            self.security.unlock.draft.clear();
            self.security.unlock.marked_text.clear();
            self.security.unlock.error = None;
            self.security.status = "master password required".to_string();
            cx.notify();
            return;
        }
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.verify_master_password(&self.security.unlock.draft))
        {
            Ok(true) => {
                let pending_action = self.security.unlock.pending_action.take();
                self.security.unlock.secrets_unlocked = true;
                self.security.status = "secrets unlocked".to_string();
                self.close_security_unlock_prompt(cx);
                if let Some(action) = pending_action {
                    self.execute_security_unlock_action(action, window, cx);
                }
            }
            Ok(false) => {
                self.security.unlock.draft.clear();
                self.security.unlock.marked_text.clear();
                self.security.unlock.error =
                    Some(self.tr("secretUnlock.wrongPassword").to_string());
                self.security.status = "unlock rejected".to_string();
                cx.notify();
            }
            Err(error) => {
                self.security.unlock.draft.clear();
                self.security.unlock.marked_text.clear();
                self.security.unlock.error = Some(error.to_string());
                self.security.status = "unlock failed".to_string();
                cx.notify();
            }
        }
    }

    pub(in crate::features) fn handle_security_unlock_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }
        match keystroke.key.as_str() {
            "enter" => self.submit_security_unlock(window, cx),
            "escape" => self.cancel_security_unlock_prompt(cx),
            "backspace" => {
                if self.security.unlock.marked_text.is_empty() {
                    self.security.unlock.draft.pop();
                } else {
                    self.security.unlock.marked_text.clear();
                }
                self.security.unlock.error = None;
                cx.notify();
            }
            _ => {
                if let Some(value) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|value| !value.is_empty())
                {
                    self.security.unlock.marked_text.clear();
                    self.security.unlock.draft.push_str(value);
                    self.security.unlock.error = None;
                    cx.notify();
                }
            }
        }
    }

    fn execute_security_unlock_action(
        &mut self,
        action: SecurityUnlockAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            SecurityUnlockAction::OpenPasswordEditor(id) => {
                self.open_security_password_editor(id, window, cx);
            }
            SecurityUnlockAction::RevealPassword(id) => {
                self.reveal_security_password(id, window, cx);
            }
            SecurityUnlockAction::CopyPassword(id) => {
                self.copy_security_password(id, window, cx);
            }
            SecurityUnlockAction::DeletePassword(id) => {
                self.request_delete_security_password(id, window, cx);
            }
            SecurityUnlockAction::OpenCredentialEditor(id) => {
                self.open_security_credential_editor(id, window, cx);
            }
            SecurityUnlockAction::ToggleCredentialEnabled(id) => {
                self.toggle_security_credential_list_enabled(id, window, cx);
            }
            SecurityUnlockAction::RevealCredential(id) => {
                self.reveal_security_credential_password(id, window, cx);
            }
            SecurityUnlockAction::DeleteCredential(id) => {
                self.request_delete_security_credential(id, window, cx);
            }
        }
    }

    pub(in crate::features) fn set_security_auth_tab(
        &mut self,
        tab: SecurityAuthTab,
        cx: &mut Context<Self>,
    ) {
        self.security.auth_tab = tab;
        self.security.status = format!("{} tab", self.security.auth_tab.label().to_lowercase());
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

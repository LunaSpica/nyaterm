use gpui::{ClipboardItem, Context, KeyDownEvent, Window};
use nyaterm_core::{ConnectionStore, SavedCredential};

use crate::features::{NyaTermApp, compact_id, none_if_blank};
use crate::models::{
    SecurityAuthTab, SecurityCredentialEditorField, SecurityCredentialEditorState,
    SecurityDeleteConfirmState, SecurityUnlockAction,
};

impl NyaTermApp {
    pub(in crate::features) fn open_security_credential_editor(
        &mut self,
        credential_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.forget_text_inputs("security.editor.cred-");
        if !self.require_security_secrets_unlocked(
            window,
            cx,
            Some(SecurityUnlockAction::OpenCredentialEditor(
                credential_id.clone(),
            )),
        ) {
            return;
        }
        let editor = if let Some(credential_id) = credential_id {
            let Some(entry) = self
                .security
                .credentials()
                .iter()
                .find(|entry| entry.id == credential_id)
                .cloned()
            else {
                self.security.status = "credential is no longer available".to_string();
                cx.notify();
                return;
            };
            SecurityCredentialEditorState {
                id: Some(entry.id),
                name: entry.name,
                username: entry.username,
                password: String::new(),
                username_prompt_regex: entry.username_prompt_regex.unwrap_or_default(),
                password_prompt_regex: entry.password_prompt_regex.unwrap_or_default(),
                enabled: entry.enabled,
                has_password: entry.has_password,
                focused_field: SecurityCredentialEditorField::Name,
                error: None,
            }
        } else {
            SecurityCredentialEditorState {
                id: None,
                name: String::new(),
                username: String::new(),
                password: String::new(),
                username_prompt_regex: String::new(),
                password_prompt_regex: String::new(),
                enabled: true,
                has_password: false,
                focused_field: SecurityCredentialEditorField::Name,
                error: None,
            }
        };
        self.security
            .open_credential_editor(editor, "credential editor opened".to_string());
        window.focus(self.security.credential_editor_focus());
        cx.notify();
    }

    pub(in crate::features) fn close_security_credential_editor(&mut self, cx: &mut Context<Self>) {
        self.forget_text_inputs("security.editor.cred-");
        self.security.close_credential_editor();
        cx.notify();
    }

    pub(in crate::features) fn toggle_security_credential_enabled(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.security.credential_editor_mut() {
            editor.enabled = !editor.enabled;
        }
        cx.notify();
    }

    pub(in crate::features) fn toggle_security_credential_list_enabled(
        &mut self,
        credential_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.require_security_secrets_unlocked(
            window,
            cx,
            Some(SecurityUnlockAction::ToggleCredentialEnabled(
                credential_id.clone(),
            )),
        ) {
            return;
        }
        let Some(entry) = self
            .security
            .credentials()
            .iter()
            .find(|entry| entry.id == credential_id)
            .cloned()
        else {
            self.security.status = "credential not found".to_string();
            cx.notify();
            return;
        };
        let store = match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            Ok(store) => store,
            Err(error) => {
                self.security.status = error.to_string();
                cx.notify();
                return;
            }
        };
        let mut next = entry;
        next.enabled = !next.enabled;
        match store.save_credential(next.clone()) {
            Ok(_) => {
                self.refresh_security_catalog();
                self.security.status = format!(
                    "credential {} {}",
                    next.name,
                    if next.enabled { "enabled" } else { "disabled" }
                );
            }
            Err(error) => {
                self.security.status = error.to_string();
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn handle_security_credential_editor_key_down(
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
        // The boxes own the text; the editor owns the keys that close or save
        // it, which the boxes leave unconsumed.
        match keystroke.key.as_str() {
            "escape" => {
                self.close_security_credential_editor(cx);
                return;
            }
            "enter" => {
                self.save_security_credential_editor(window, cx);
                return;
            }
            _ => {}
        }
    }

    pub(in crate::features) fn save_security_credential_editor(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.security.credential_editor().cloned() else {
            return;
        };
        let name = editor.name.trim().to_string();
        if name.is_empty() {
            if let Some(editor) = self.security.credential_editor_mut() {
                editor.error = Some("credential name is required".to_string());
            }
            cx.notify();
            return;
        }
        let store = match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            Ok(store) => store,
            Err(error) => {
                if let Some(editor) = self.security.credential_editor_mut() {
                    editor.error = Some(error.to_string());
                }
                cx.notify();
                return;
            }
        };
        let entry = SavedCredential {
            id: editor.id.clone().unwrap_or_default(),
            name,
            username: editor.username.trim().to_string(),
            password: if editor.password.trim().is_empty() {
                None
            } else {
                Some(editor.password.clone())
            },
            username_prompt_regex: none_if_blank(&editor.username_prompt_regex),
            password_prompt_regex: none_if_blank(&editor.password_prompt_regex),
            enabled: editor.enabled,
            has_password: false,
        };
        match store.save_credential(entry) {
            Ok(id) => {
                self.refresh_security_catalog();
                self.security
                    .finish_credential_editor(format!("credential saved ({})", compact_id(&id)));
                self.terminal.view.status = "credential saved".to_string();
            }
            Err(error) => {
                if let Some(editor) = self.security.credential_editor_mut() {
                    editor.error = Some(error.to_string());
                }
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn request_delete_security_credential(
        &mut self,
        credential_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.require_security_secrets_unlocked(
            window,
            cx,
            Some(SecurityUnlockAction::DeleteCredential(
                credential_id.clone(),
            )),
        ) {
            return;
        }
        let label = self
            .security
            .credentials()
            .iter()
            .find(|entry| entry.id == credential_id)
            .map(|entry| entry.name.clone())
            .unwrap_or_else(|| credential_id.clone());
        self.security.request_delete(SecurityDeleteConfirmState {
            kind: SecurityAuthTab::Credentials,
            id: credential_id,
            label,
        });
        cx.notify();
    }

    pub(in crate::features) fn reveal_security_credential_password(
        &mut self,
        credential_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .security
            .revealed
            .credentials
            .contains_key(&credential_id)
        {
            self.security.revealed.credentials.remove(&credential_id);
            self.security.status = "credential password hidden".to_string();
            cx.notify();
            return;
        }
        if !self.require_security_secrets_unlocked(
            window,
            cx,
            Some(SecurityUnlockAction::RevealCredential(
                credential_id.clone(),
            )),
        ) {
            return;
        }
        let store = match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            Ok(store) => store,
            Err(error) => {
                self.security.status = error.to_string();
                cx.notify();
                return;
            }
        };
        match store.load_decrypted_credential_by_id(&credential_id) {
            Ok(Some(entry)) => {
                let value = entry.password.unwrap_or_default();
                if value.is_empty() {
                    self.security.status = "credential has no password".to_string();
                } else {
                    self.security
                        .revealed
                        .credentials
                        .insert(credential_id.clone(), value.clone());
                    cx.write_to_clipboard(ClipboardItem::new_string(value));
                    self.security.status = "credential password revealed and copied".to_string();
                }
            }
            Ok(None) => self.security.status = "credential not found".to_string(),
            Err(error) => self.security.status = error.to_string(),
        }
        cx.notify();
    }
}

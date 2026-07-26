use super::*;

use crate::models::{
    SecurityAuthTab, SecurityDeleteConfirmState, SecurityPasswordEditorField,
    SecurityPasswordEditorState, SecurityUnlockAction,
};

impl NyaTermApp {
    pub(in crate::features) fn open_security_password_editor(
        &mut self,
        password_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.require_security_secrets_unlocked(
            window,
            cx,
            Some(SecurityUnlockAction::OpenPasswordEditor(
                password_id.clone(),
            )),
        ) {
            return;
        }
        let editor = if let Some(password_id) = password_id {
            let Some(entry) = self
                .connection_saved_passwords
                .iter()
                .find(|entry| entry.id == password_id)
                .cloned()
            else {
                self.security_status = "password is no longer available".to_string();
                cx.notify();
                return;
            };
            SecurityPasswordEditorState {
                id: Some(entry.id),
                name: entry.name,
                password: String::new(),
                has_password: entry.has_password,
                show_password: false,
                focused_field: SecurityPasswordEditorField::Name,
                error: None,
            }
        } else {
            SecurityPasswordEditorState {
                id: None,
                name: String::new(),
                password: String::new(),
                has_password: false,
                show_password: false,
                focused_field: SecurityPasswordEditorField::Name,
                error: None,
            }
        };
        self.security_password_editor = Some(editor);
        self.security_key_editor = None;
        self.security_otp_editor = None;
        self.security_credential_editor = None;
        self.security_delete_confirm = None;
        self.security_status = "password editor opened".to_string();
        window.focus(&self.security_password_editor_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_security_password_editor(&mut self, cx: &mut Context<Self>) {
        self.security_password_editor = None;
        self.security_status = "password editor closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn focus_security_password_field(
        &mut self,
        field: SecurityPasswordEditorField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.security_password_editor.as_mut() {
            editor.focused_field = field;
            editor.error = None;
        }
        window.focus(&self.security_password_editor_focus);
        cx.notify();
    }

    pub(in crate::features) fn handle_security_password_editor_key_down(
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
            "escape" => {
                self.close_security_password_editor(cx);
                return;
            }
            "enter" => {
                self.save_security_password_editor(window, cx);
                return;
            }
            _ => {}
        }
        let Some(editor) = self.security_password_editor.as_mut() else {
            return;
        };
        let field = match editor.focused_field {
            SecurityPasswordEditorField::Name => &mut editor.name,
            SecurityPasswordEditorField::Password => &mut editor.password,
        };
        match keystroke.key.as_str() {
            "backspace" => {
                field.pop();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    field.push_str(input);
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn save_security_password_editor(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.security_password_editor.clone() else {
            return;
        };
        let name = editor.name.trim().to_string();
        if name.is_empty() {
            if let Some(editor) = self.security_password_editor.as_mut() {
                editor.error = Some("password name is required".to_string());
            }
            cx.notify();
            return;
        }
        if editor.id.is_none() && editor.password.trim().is_empty() {
            if let Some(editor) = self.security_password_editor.as_mut() {
                editor.error = Some("password value is required".to_string());
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
                if let Some(editor) = self.security_password_editor.as_mut() {
                    editor.error = Some(error.to_string());
                }
                cx.notify();
                return;
            }
        };
        let entry = SavedPassword {
            id: editor.id.clone().unwrap_or_default(),
            name,
            password: if editor.password.trim().is_empty() {
                None
            } else {
                Some(editor.password.clone())
            },
            has_password: false,
        };
        match store.save_password(entry) {
            Ok(id) => {
                self.refresh_security_catalog();
                self.security_password_editor = None;
                self.security_status = format!("password saved ({})", compact_id(&id));
                self.terminal_status = "password saved".to_string();
            }
            Err(error) => {
                if let Some(editor) = self.security_password_editor.as_mut() {
                    editor.error = Some(error.to_string());
                }
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn request_delete_security_password(
        &mut self,
        password_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.require_security_secrets_unlocked(
            window,
            cx,
            Some(SecurityUnlockAction::DeletePassword(password_id.clone())),
        ) {
            return;
        }
        let label = self
            .connection_saved_passwords
            .iter()
            .find(|entry| entry.id == password_id)
            .map(|entry| entry.name.clone())
            .unwrap_or_else(|| password_id.clone());
        self.security_delete_confirm = Some(SecurityDeleteConfirmState {
            kind: SecurityAuthTab::Passwords,
            id: password_id,
            label,
        });
        cx.notify();
    }

    pub(in crate::features) fn reveal_security_password(
        &mut self,
        password_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Tauri PasswordManagementTab: eye toggles reveal; hide does not need unlock.
        if self.security_revealed_passwords.contains_key(&password_id) {
            self.security_revealed_passwords.remove(&password_id);
            self.security_status = "password hidden".to_string();
            cx.notify();
            return;
        }
        if !self.require_security_secrets_unlocked(
            window,
            cx,
            Some(SecurityUnlockAction::RevealPassword(password_id.clone())),
        ) {
            return;
        }
        let store = match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            Ok(store) => store,
            Err(error) => {
                self.security_status = error.to_string();
                cx.notify();
                return;
            }
        };
        match store.load_decrypted_password_by_id(&password_id) {
            Ok(Some(entry)) => {
                let value = entry.password.unwrap_or_default();
                if value.is_empty() {
                    self.security_status = "password has no secret".to_string();
                } else {
                    self.security_revealed_passwords
                        .insert(password_id.clone(), value);
                    self.security_status = "password revealed".to_string();
                }
            }
            Ok(None) => self.security_status = "password not found".to_string(),
            Err(error) => self.security_status = error.to_string(),
        }
        cx.notify();
    }

    pub(in crate::features) fn copy_security_password(
        &mut self,
        password_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.require_security_secrets_unlocked(
            window,
            cx,
            Some(SecurityUnlockAction::CopyPassword(password_id.clone())),
        ) {
            return;
        }
        if let Some(value) = self.security_revealed_passwords.get(&password_id).cloned() {
            if value.is_empty() {
                self.security_status = "password has no secret".to_string();
            } else {
                cx.write_to_clipboard(ClipboardItem::new_string(value));
                self.security_status = "password copied".to_string();
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
                self.security_status = error.to_string();
                cx.notify();
                return;
            }
        };
        match store.load_decrypted_password_by_id(&password_id) {
            Ok(Some(entry)) => {
                let value = entry.password.unwrap_or_default();
                if value.is_empty() {
                    self.security_status = "password has no secret".to_string();
                } else {
                    self.security_revealed_passwords
                        .insert(password_id.clone(), value.clone());
                    cx.write_to_clipboard(ClipboardItem::new_string(value));
                    self.security_status = "password revealed and copied".to_string();
                }
            }
            Ok(None) => self.security_status = "password not found".to_string(),
            Err(error) => self.security_status = error.to_string(),
        }
        cx.notify();
    }

    pub(in crate::features) fn toggle_security_password_editor_visibility(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.security_password_editor.as_mut() {
            editor.show_password = !editor.show_password;
            cx.notify();
        }
    }
}

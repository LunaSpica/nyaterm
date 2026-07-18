use super::*;

impl NyaTermApp {
    pub(in crate::features) fn open_security_credential_editor(
        &mut self,
        credential_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
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
                .connection_saved_credentials
                .iter()
                .find(|entry| entry.id == credential_id)
                .cloned()
            else {
                self.security_status = "credential is no longer available".to_string();
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
        self.security_credential_editor = Some(editor);
        self.security_key_editor = None;
        self.security_otp_editor = None;
        self.security_password_editor = None;
        self.security_delete_confirm = None;
        self.security_status = "credential editor opened".to_string();
        window.focus(&self.security_credential_editor_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_security_credential_editor(&mut self, cx: &mut Context<Self>) {
        self.security_credential_editor = None;
        self.security_status = "credential editor closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn focus_security_credential_field(
        &mut self,
        field: SecurityCredentialEditorField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.security_credential_editor.as_mut() {
            editor.focused_field = field;
            editor.error = None;
        }
        window.focus(&self.security_credential_editor_focus);
        cx.notify();
    }

    pub(in crate::features) fn toggle_security_credential_enabled(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.security_credential_editor.as_mut() {
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
            .connection_saved_credentials
            .iter()
            .find(|entry| entry.id == credential_id)
            .cloned()
        else {
            self.security_status = "credential not found".to_string();
            cx.notify();
            return;
        };
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
        let mut next = entry;
        next.enabled = !next.enabled;
        match store.save_credential(next.clone()) {
            Ok(_) => {
                self.refresh_security_catalog();
                self.security_status = format!(
                    "credential {} {}",
                    next.name,
                    if next.enabled { "enabled" } else { "disabled" }
                );
            }
            Err(error) => {
                self.security_status = error.to_string();
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
        let Some(editor) = self.security_credential_editor.as_mut() else {
            return;
        };
        let field = match editor.focused_field {
            SecurityCredentialEditorField::Name => &mut editor.name,
            SecurityCredentialEditorField::Username => &mut editor.username,
            SecurityCredentialEditorField::Password => &mut editor.password,
            SecurityCredentialEditorField::UsernameRegex => &mut editor.username_prompt_regex,
            SecurityCredentialEditorField::PasswordRegex => &mut editor.password_prompt_regex,
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

    pub(in crate::features) fn save_security_credential_editor(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.security_credential_editor.clone() else {
            return;
        };
        let name = editor.name.trim().to_string();
        if name.is_empty() {
            if let Some(editor) = self.security_credential_editor.as_mut() {
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
                if let Some(editor) = self.security_credential_editor.as_mut() {
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
                self.security_credential_editor = None;
                self.security_status = format!("credential saved ({})", compact_id(&id));
                self.terminal_status = "credential saved".to_string();
            }
            Err(error) => {
                if let Some(editor) = self.security_credential_editor.as_mut() {
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
            .connection_saved_credentials
            .iter()
            .find(|entry| entry.id == credential_id)
            .map(|entry| entry.name.clone())
            .unwrap_or_else(|| credential_id.clone());
        self.security_delete_confirm = Some(SecurityDeleteConfirmState {
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
            .security_revealed_credentials
            .contains_key(&credential_id)
        {
            self.security_revealed_credentials.remove(&credential_id);
            self.security_status = "credential password hidden".to_string();
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
                self.security_status = error.to_string();
                cx.notify();
                return;
            }
        };
        match store.load_decrypted_credential_by_id(&credential_id) {
            Ok(Some(entry)) => {
                let value = entry.password.unwrap_or_default();
                if value.is_empty() {
                    self.security_status = "credential has no password".to_string();
                } else {
                    self.security_revealed_credentials
                        .insert(credential_id.clone(), value.clone());
                    cx.write_to_clipboard(ClipboardItem::new_string(value));
                    self.security_status = "credential password revealed and copied".to_string();
                }
            }
            Ok(None) => self.security_status = "credential not found".to_string(),
            Err(error) => self.security_status = error.to_string(),
        }
        cx.notify();
    }
}

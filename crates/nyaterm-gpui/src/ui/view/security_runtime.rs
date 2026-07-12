use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn security_secrets_locked(&self) -> bool {
        self.settings.has_master_password && !self.security_secrets_unlocked
    }

    pub(in crate::ui::view) fn require_security_secrets_unlocked(
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

    pub(in crate::ui::view) fn open_security_unlock_prompt(
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

    pub(in crate::ui::view) fn close_security_unlock_prompt(&mut self, cx: &mut Context<Self>) {
        self.security_unlock_prompt_open = false;
        self.security_unlock_draft.clear();
        self.security_unlock_error = None;
        cx.notify();
    }

    pub(in crate::ui::view) fn lock_security_secrets(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn submit_security_unlock(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn handle_security_unlock_key_down(
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

    pub(in crate::ui::view) fn set_security_auth_tab(
        &mut self,
        tab: SecurityAuthTab,
        cx: &mut Context<Self>,
    ) {
        self.security_auth_tab = tab;
        self.security_status = format!("{} tab", self.security_auth_tab.label().to_lowercase());
        cx.notify();
    }

    pub(in crate::ui::view) fn refresh_security_catalog(&mut self) {
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

    pub(in crate::ui::view) fn open_security_key_editor(
        &mut self,
        key_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let editor = if let Some(key_id) = key_id {
            let Some(key) = self
                .connection_ssh_keys
                .iter()
                .find(|key| key.id == key_id)
                .cloned()
            else {
                self.security_status = "SSH key is no longer available".to_string();
                cx.notify();
                return;
            };
            SecurityKeyEditorState {
                id: Some(key.id),
                name: key.name,
                key_file_path: String::new(),
                cert_file_path: String::new(),
                passphrase: String::new(),
                has_key_data: key.has_key_data,
                has_cert_data: key.has_cert_data,
                focused_field: SecurityKeyEditorField::Name,
                error: None,
            }
        } else {
            SecurityKeyEditorState {
                id: None,
                name: String::new(),
                key_file_path: String::new(),
                cert_file_path: String::new(),
                passphrase: String::new(),
                has_key_data: false,
                has_cert_data: false,
                focused_field: SecurityKeyEditorField::Name,
                error: None,
            }
        };
        self.security_key_editor = Some(editor);
        self.security_otp_editor = None;
        self.security_password_editor = None;
        self.security_credential_editor = None;
        self.security_delete_confirm = None;
        self.security_status = "SSH key editor opened".to_string();
        window.focus(&self.security_key_editor_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn close_security_key_editor(&mut self, cx: &mut Context<Self>) {
        self.security_key_editor = None;
        self.security_status = "SSH key editor closed".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn focus_security_key_field(
        &mut self,
        field: SecurityKeyEditorField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.security_key_editor.as_mut() {
            editor.focused_field = field;
            editor.error = None;
        }
        window.focus(&self.security_key_editor_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_security_key_editor_key_down(
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
                self.close_security_key_editor(cx);
                return;
            }
            "enter" => {
                self.save_security_key_editor(window, cx);
                return;
            }
            _ => {}
        }
        let Some(editor) = self.security_key_editor.as_mut() else {
            return;
        };
        let field = match editor.focused_field {
            SecurityKeyEditorField::Name => &mut editor.name,
            SecurityKeyEditorField::KeyPath => &mut editor.key_file_path,
            SecurityKeyEditorField::CertPath => &mut editor.cert_file_path,
            SecurityKeyEditorField::Passphrase => &mut editor.passphrase,
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

    pub(in crate::ui::view) fn save_security_key_editor(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.security_key_editor.clone() else {
            return;
        };
        let name = editor.name.trim().to_string();
        if name.is_empty() {
            if let Some(editor) = self.security_key_editor.as_mut() {
                editor.error = Some("key name is required".to_string());
            }
            cx.notify();
            return;
        }
        if editor.id.is_none() && editor.key_file_path.trim().is_empty() && !editor.has_key_data {
            if let Some(editor) = self.security_key_editor.as_mut() {
                editor.error = Some("select a private key file".to_string());
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
                if let Some(editor) = self.security_key_editor.as_mut() {
                    editor.error = Some(error.to_string());
                }
                cx.notify();
                return;
            }
        };

        let key = SshKey {
            id: editor.id.clone().unwrap_or_default(),
            name,
            key: None,
            cert: None,
            passphrase: if editor.passphrase.trim().is_empty() {
                None
            } else {
                Some(editor.passphrase.clone())
            },
            key_file_path: if editor.key_file_path.trim().is_empty() {
                None
            } else {
                Some(editor.key_file_path.trim().to_string())
            },
            cert_file_path: if editor.cert_file_path.trim().is_empty() {
                None
            } else {
                Some(editor.cert_file_path.trim().to_string())
            },
            has_key_data: false,
            has_cert_data: false,
        };

        match store.save_ssh_key(key) {
            Ok(id) => {
                self.refresh_security_catalog();
                self.security_key_editor = None;
                self.security_status = format!("SSH key saved ({})", compact_id(&id));
                self.terminal_status = "SSH key saved".to_string();
            }
            Err(error) => {
                if let Some(editor) = self.security_key_editor.as_mut() {
                    editor.error = Some(error.to_string());
                }
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn request_delete_security_key(
        &mut self,
        key_id: String,
        cx: &mut Context<Self>,
    ) {
        let label = self
            .connection_ssh_keys
            .iter()
            .find(|key| key.id == key_id)
            .map(|key| key.name.clone())
            .unwrap_or_else(|| key_id.clone());
        self.security_delete_confirm = Some(SecurityDeleteConfirmState {
            kind: SecurityAuthTab::Keys,
            id: key_id,
            label,
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn open_security_otp_editor(
        &mut self,
        otp_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let editor = if let Some(otp_id) = otp_id {
            let Some(entry) = self
                .connection_otp_entries
                .iter()
                .find(|entry| entry.id == otp_id)
                .cloned()
            else {
                self.security_status = "OTP entry is no longer available".to_string();
                cx.notify();
                return;
            };
            SecurityOtpEditorState {
                id: Some(entry.id),
                otp_type: entry.otp_type,
                issuer: entry.issuer,
                username: entry.username,
                secret: String::new(),
                algorithm: entry.algorithm,
                digits: entry.digits.to_string(),
                period: entry.period.to_string(),
                counter: entry.counter.to_string(),
                has_secret: entry.has_secret,
                focused_field: SecurityOtpEditorField::Issuer,
                error: None,
            }
        } else {
            SecurityOtpEditorState {
                id: None,
                otp_type: "totp".to_string(),
                issuer: String::new(),
                username: String::new(),
                secret: String::new(),
                algorithm: "SHA1".to_string(),
                digits: "6".to_string(),
                period: "30".to_string(),
                counter: "0".to_string(),
                has_secret: false,
                focused_field: SecurityOtpEditorField::Issuer,
                error: None,
            }
        };
        self.security_otp_editor = Some(editor);
        self.security_key_editor = None;
        self.security_password_editor = None;
        self.security_credential_editor = None;
        self.security_delete_confirm = None;
        self.security_status = "OTP editor opened".to_string();
        window.focus(&self.security_otp_editor_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn close_security_otp_editor(&mut self, cx: &mut Context<Self>) {
        self.security_otp_editor = None;
        self.security_status = "OTP editor closed".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn focus_security_otp_field(
        &mut self,
        field: SecurityOtpEditorField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.security_otp_editor.as_mut() {
            editor.focused_field = field;
            editor.error = None;
        }
        window.focus(&self.security_otp_editor_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn set_security_otp_type(
        &mut self,
        otp_type: &'static str,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.security_otp_editor.as_mut() {
            editor.otp_type = otp_type.to_string();
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn cycle_security_otp_algorithm(&mut self, cx: &mut Context<Self>) {
        if let Some(editor) = self.security_otp_editor.as_mut() {
            editor.algorithm = match editor.algorithm.as_str() {
                "SHA1" => "SHA256".to_string(),
                "SHA256" => "SHA512".to_string(),
                _ => "SHA1".to_string(),
            };
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_security_otp_editor_key_down(
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
                self.close_security_otp_editor(cx);
                return;
            }
            "enter" => {
                self.save_security_otp_editor(window, cx);
                return;
            }
            _ => {}
        }
        let Some(editor) = self.security_otp_editor.as_mut() else {
            return;
        };
        let field = match editor.focused_field {
            SecurityOtpEditorField::Issuer => &mut editor.issuer,
            SecurityOtpEditorField::Username => &mut editor.username,
            SecurityOtpEditorField::Secret => &mut editor.secret,
            SecurityOtpEditorField::Digits => &mut editor.digits,
            SecurityOtpEditorField::Period => &mut editor.period,
            SecurityOtpEditorField::Counter => &mut editor.counter,
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

    pub(in crate::ui::view) fn save_security_otp_editor(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.security_otp_editor.clone() else {
            return;
        };
        if editor.id.is_none() && editor.secret.trim().is_empty() {
            if let Some(editor) = self.security_otp_editor.as_mut() {
                editor.error = Some("OTP secret is required".to_string());
            }
            cx.notify();
            return;
        }
        let digits = editor.digits.trim().parse::<u8>().unwrap_or(6).clamp(4, 10);
        let period = editor.period.trim().parse::<u64>().unwrap_or(30).max(1);
        let counter = editor.counter.trim().parse::<u64>().unwrap_or(0);

        let store = match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            Ok(store) => store,
            Err(error) => {
                if let Some(editor) = self.security_otp_editor.as_mut() {
                    editor.error = Some(error.to_string());
                }
                cx.notify();
                return;
            }
        };

        let entry = OtpEntry {
            id: editor.id.clone().unwrap_or_default(),
            otp_type: if editor.otp_type == "hotp" {
                "hotp".to_string()
            } else {
                "totp".to_string()
            },
            issuer: editor.issuer.trim().to_string(),
            username: editor.username.trim().to_string(),
            secret: if editor.secret.trim().is_empty() {
                None
            } else {
                Some(editor.secret.trim().to_string())
            },
            algorithm: editor.algorithm.clone(),
            digits,
            period,
            counter,
            has_secret: false,
        };

        match store.save_otp_entry(entry) {
            Ok(id) => {
                self.refresh_security_catalog();
                self.security_otp_editor = None;
                self.security_status = format!("OTP entry saved ({})", compact_id(&id));
                self.terminal_status = "OTP entry saved".to_string();
            }
            Err(error) => {
                if let Some(editor) = self.security_otp_editor.as_mut() {
                    editor.error = Some(error.to_string());
                }
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn request_delete_security_otp(
        &mut self,
        otp_id: String,
        cx: &mut Context<Self>,
    ) {
        let label = self
            .connection_otp_entries
            .iter()
            .find(|entry| entry.id == otp_id)
            .map(|entry| {
                if !entry.issuer.trim().is_empty() || !entry.username.trim().is_empty() {
                    format!(
                        "{}{}",
                        entry.issuer,
                        if entry.username.trim().is_empty() {
                            String::new()
                        } else if entry.issuer.trim().is_empty() {
                            entry.username.clone()
                        } else {
                            format!(" ({})", entry.username)
                        }
                    )
                } else {
                    compact_id(&entry.id)
                }
            })
            .unwrap_or_else(|| otp_id.clone());
        self.security_delete_confirm = Some(SecurityDeleteConfirmState {
            kind: SecurityAuthTab::Otp,
            id: otp_id,
            label,
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn cancel_security_delete(&mut self, cx: &mut Context<Self>) {
        self.security_delete_confirm = None;
        cx.notify();
    }

    pub(in crate::ui::view) fn confirm_security_delete(&mut self, cx: &mut Context<Self>) {
        let Some(confirm) = self.security_delete_confirm.clone() else {
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
        let result = match confirm.kind {
            SecurityAuthTab::Keys => store.delete_ssh_key(&confirm.id),
            SecurityAuthTab::Passwords => store.delete_password(&confirm.id),
            SecurityAuthTab::Credentials => store.delete_credential(&confirm.id),
            SecurityAuthTab::Otp => store.delete_otp_entry(&confirm.id),
        };
        match result {
            Ok(()) => {
                match confirm.kind {
                    SecurityAuthTab::Otp => {
                        self.security_otp_codes.remove(&confirm.id);
                    }
                    SecurityAuthTab::Passwords => {
                        self.security_revealed_passwords.remove(&confirm.id);
                    }
                    SecurityAuthTab::Credentials => {
                        self.security_revealed_credentials.remove(&confirm.id);
                    }
                    SecurityAuthTab::Keys => {}
                }
                self.refresh_security_catalog();
                self.security_delete_confirm = None;
                self.security_status = format!("{} deleted", confirm.label);
                self.terminal_status = self.security_status.clone();
            }
            Err(error) => {
                self.security_status = error.to_string();
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn generate_security_otp_code(
        &mut self,
        otp_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.require_security_secrets_unlocked(window, cx) {
            return;
        }
        match self.otp_provider.request_otp_code(&otp_id) {
            Ok(Some(code)) => {
                self.security_otp_codes.insert(otp_id.clone(), code.clone());
                self.security_status = format!("OTP code ready for {}", compact_id(&otp_id));
                cx.write_to_clipboard(ClipboardItem::new_string(code));
                self.terminal_status = "OTP code copied".to_string();
            }
            Ok(None) => {
                self.security_status = "OTP entry not found".to_string();
            }
            Err(error) => {
                self.security_status = error;
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn refresh_visible_security_otp_codes(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.security_secrets_locked() {
            return;
        }
        let ids = self
            .connection_otp_entries
            .iter()
            .filter(|entry| entry.has_secret || entry.otp_type.eq_ignore_ascii_case("totp"))
            .map(|entry| entry.id.clone())
            .collect::<Vec<_>>();
        let mut refreshed = 0usize;
        for otp_id in ids {
            match self.otp_provider.request_otp_code(&otp_id) {
                Ok(Some(code)) => {
                    self.security_otp_codes.insert(otp_id, code);
                    refreshed += 1;
                }
                Ok(None) | Err(_) => {}
            }
        }
        if refreshed > 0 {
            self.security_status = format!("refreshed {refreshed} OTP code(s)");
        }
        let _ = window;
        cx.notify();
    }

    pub(in crate::ui::view) fn copy_security_otp_code(
        &mut self,
        otp_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(code) = self.security_otp_codes.get(&otp_id).cloned() {
            if code != "------" && !code.trim().is_empty() {
                cx.write_to_clipboard(ClipboardItem::new_string(code));
                self.security_status = format!("OTP code copied ({})", compact_id(&otp_id));
                self.terminal_status = "OTP code copied".to_string();
                cx.notify();
                return;
            }
        }
        self.generate_security_otp_code(otp_id, window, cx);
    }

    pub(in crate::ui::view) fn pick_security_key_file(
        &mut self,
        is_cert: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let options = PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from(if is_cert {
                "Select certificate file"
            } else {
                "Select private key file"
            })),
        };
        let receiver = cx.prompt_for_paths(options);
        self.security_status = if is_cert {
            "selecting certificate file".to_string()
        } else {
            "selecting private key file".to_string()
        };
        cx.spawn(async move |this, cx| {
            let selected = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let _ = this.update(cx, |this, cx| {
                if let Some(path) = selected {
                    let path = path.display().to_string();
                    if let Some(editor) = this.security_key_editor.as_mut() {
                        if is_cert {
                            editor.cert_file_path = path;
                            editor.has_cert_data = true;
                        } else {
                            editor.key_file_path = path;
                            editor.has_key_data = true;
                        }
                        editor.error = None;
                        this.security_status = "key file selected".to_string();
                    }
                } else {
                    this.security_status = "key file selection cancelled".to_string();
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::ui::view) fn open_security_password_editor(
        &mut self,
        password_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.require_security_secrets_unlocked(window, cx) {
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

    pub(in crate::ui::view) fn close_security_password_editor(&mut self, cx: &mut Context<Self>) {
        self.security_password_editor = None;
        self.security_status = "password editor closed".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn focus_security_password_field(
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

    pub(in crate::ui::view) fn handle_security_password_editor_key_down(
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

    pub(in crate::ui::view) fn save_security_password_editor(
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

    pub(in crate::ui::view) fn request_delete_security_password(
        &mut self,
        password_id: String,
        cx: &mut Context<Self>,
    ) {
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

    pub(in crate::ui::view) fn reveal_security_password(
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
        if !self.require_security_secrets_unlocked(window, cx) {
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

    pub(in crate::ui::view) fn copy_security_password(
        &mut self,
        password_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.require_security_secrets_unlocked(window, cx) {
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

    pub(in crate::ui::view) fn toggle_security_password_editor_visibility(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.security_password_editor.as_mut() {
            editor.show_password = !editor.show_password;
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn open_security_credential_editor(
        &mut self,
        credential_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.require_security_secrets_unlocked(window, cx) {
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

    pub(in crate::ui::view) fn close_security_credential_editor(&mut self, cx: &mut Context<Self>) {
        self.security_credential_editor = None;
        self.security_status = "credential editor closed".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn focus_security_credential_field(
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

    pub(in crate::ui::view) fn toggle_security_credential_enabled(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.security_credential_editor.as_mut() {
            editor.enabled = !editor.enabled;
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_security_credential_list_enabled(
        &mut self,
        credential_id: String,
        cx: &mut Context<Self>,
    ) {
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

    pub(in crate::ui::view) fn handle_security_credential_editor_key_down(
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

    pub(in crate::ui::view) fn save_security_credential_editor(
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

    pub(in crate::ui::view) fn request_delete_security_credential(
        &mut self,
        credential_id: String,
        cx: &mut Context<Self>,
    ) {
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

    pub(in crate::ui::view) fn reveal_security_credential_password(
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
        if !self.require_security_secrets_unlocked(window, cx) {
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

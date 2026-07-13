use super::*;

impl NyaTermApp {
    pub(in crate::features) fn open_security_otp_editor(
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

    pub(in crate::features) fn close_security_otp_editor(&mut self, cx: &mut Context<Self>) {
        self.security_otp_editor = None;
        self.security_status = "OTP editor closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn focus_security_otp_field(
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

    pub(in crate::features) fn set_security_otp_type(
        &mut self,
        otp_type: &'static str,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.security_otp_editor.as_mut() {
            editor.otp_type = otp_type.to_string();
        }
        cx.notify();
    }

    pub(in crate::features) fn cycle_security_otp_algorithm(&mut self, cx: &mut Context<Self>) {
        if let Some(editor) = self.security_otp_editor.as_mut() {
            editor.algorithm = match editor.algorithm.as_str() {
                "SHA1" => "SHA256".to_string(),
                "SHA256" => "SHA512".to_string(),
                _ => "SHA1".to_string(),
            };
        }
        cx.notify();
    }

    pub(in crate::features) fn handle_security_otp_editor_key_down(
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

    pub(in crate::features) fn save_security_otp_editor(
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

    pub(in crate::features) fn request_delete_security_otp(
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

    pub(in crate::features) fn generate_security_otp_code(
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

    pub(in crate::features) fn refresh_visible_security_otp_codes(
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

    pub(in crate::features) fn copy_security_otp_code(
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

}

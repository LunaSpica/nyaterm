use gpui::{
    AppContext, ClipboardItem, Context, KeyDownEvent, PathPromptOptions, SharedString, Window,
};
use nyaterm_core::{ConnectionStore, OtpEntry};

use crate::features::{NyaTermApp, compact_id};
use crate::models::{SecurityAuthTab, SecurityOtpEditorState};

impl NyaTermApp {
    pub(in crate::features) fn import_security_otp_from_qr(&mut self, cx: &mut Context<Self>) {
        let scanning_status = self.tr("otpManager.scanningQr").to_string();
        if !self.security.begin_otp_qr_import(scanning_status) {
            return;
        }
        let options = PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from(self.tr("otpManager.selectQrImage"))),
        };
        let receiver = cx.prompt_for_paths(options);
        cx.spawn(async move |this, cx| {
            let selected = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let result = match selected {
                Some(path) => {
                    cx.background_spawn(async move { decode_security_otp_qr(&path).map(Some) })
                        .await
                }
                None => Ok(None),
            };
            let _ = this.update(cx, |this, cx| {
                this.security.finish_otp_qr_import();
                match result {
                    Ok(Some(editor)) => {
                        let status = this.tr("otpManager.scanQr").to_string();
                        this.security.open_otp_editor(editor, status);
                    }
                    Ok(None) => {
                        let status = this.tr("common.cancel").to_string();
                        this.security.set_status(status);
                    }
                    Err(error) => {
                        let status = format!("{}: {error}", this.tr("otpManager.qrImportFailed"));
                        this.security.set_status(status.clone());
                        this.shell.set_status(status);
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}

fn decode_security_otp_qr(path: &std::path::Path) -> Result<SecurityOtpEditorState, String> {
    let image = image::open(path).map_err(|error| format!("failed to open image: {error}"))?;
    let gray = image.to_luma8();
    let mut prepared = rqrr::PreparedImage::prepare(gray);
    let grid = prepared
        .detect_grids()
        .into_iter()
        .next()
        .ok_or_else(|| "no QR code found in the image".to_string())?;
    let (_, uri) = grid
        .decode()
        .map_err(|error| format!("failed to decode QR code: {error}"))?;

    if uri.starts_with("otpauth://totp/") {
        let totp = nyaterm_otp::Totp::from_uri(&uri)
            .map_err(|error| format!("invalid TOTP URI: {error}"))?;
        Ok(SecurityOtpEditorState {
            id: None,
            otp_type: "totp".to_string(),
            issuer: totp.issuer().to_string(),
            username: totp.label().to_string(),
            secret: totp.secret().into_base32(),
            algorithm: totp.alg().to_string(),
            digits: totp.digits().to_string(),
            period: totp.period().to_string(),
            counter: "0".to_string(),
            has_secret: false,
            error: None,
        })
    } else if uri.starts_with("otpauth://hotp/") {
        let hotp = nyaterm_otp::Hotp::from_uri(&uri)
            .map_err(|error| format!("invalid HOTP URI: {error}"))?;
        Ok(SecurityOtpEditorState {
            id: None,
            otp_type: "hotp".to_string(),
            issuer: hotp.issuer().to_string(),
            username: hotp.label().to_string(),
            secret: hotp.secret().into_base32(),
            algorithm: hotp.alg().to_string(),
            digits: hotp.digits().to_string(),
            period: "30".to_string(),
            counter: hotp.counter().to_string(),
            has_secret: false,
            error: None,
        })
    } else {
        Err("QR image does not contain an otpauth URI".to_string())
    }
}

impl NyaTermApp {
    pub(in crate::features) fn open_security_otp_editor(
        &mut self,
        otp_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.forget_text_inputs("security.editor.otp-");
        let editor = if let Some(otp_id) = otp_id {
            let Some(entry) = self
                .security
                .otp_entries()
                .iter()
                .find(|entry| entry.id == otp_id)
                .cloned()
            else {
                self.security.set_status("OTP entry is no longer available");
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
                error: None,
            }
        };
        self.security
            .open_otp_editor(editor, "OTP editor opened".to_string());
        window.focus(self.security.otp_editor_focus());
        cx.notify();
    }

    pub(in crate::features) fn close_security_otp_editor(&mut self, cx: &mut Context<Self>) {
        self.forget_text_inputs("security.editor.otp-");
        self.security.close_otp_editor();
        cx.notify();
    }

    pub(in crate::features) fn set_security_otp_type(
        &mut self,
        otp_type: &'static str,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.security.otp_editor_mut() {
            editor.otp_type = otp_type.to_string();
        }
        cx.notify();
    }

    pub(in crate::features) fn cycle_security_otp_algorithm(&mut self, cx: &mut Context<Self>) {
        self.security.cycle_otp_algorithm();
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
        // The boxes own the text; the editor owns the keys that close or save
        // it, which the boxes leave unconsumed.
        match keystroke.key.as_str() {
            "escape" => {
                self.close_security_otp_editor(cx);
            }
            "enter" => {
                self.save_security_otp_editor(window, cx);
            }
            _ => {}
        }
    }

    pub(in crate::features) fn save_security_otp_editor(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.security.otp_editor().cloned() else {
            return;
        };
        if editor.id.is_none() && editor.secret.trim().is_empty() {
            if let Some(editor) = self.security.otp_editor_mut() {
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
                if let Some(editor) = self.security.otp_editor_mut() {
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
                self.security
                    .finish_otp_editor(format!("OTP entry saved ({})", compact_id(&id)));
                self.shell.set_status("OTP entry saved".to_string());
            }
            Err(error) => {
                if let Some(editor) = self.security.otp_editor_mut() {
                    editor.error = Some(error.to_string());
                }
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn request_delete_security_otp(
        &mut self,
        otp_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let label = self
            .security
            .otp_entries()
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
        self.open_security_delete_dialog(SecurityAuthTab::Otp, otp_id, label, window, cx);
        cx.notify();
    }

    pub(in crate::features) fn generate_security_otp_code(
        &mut self,
        otp_id: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.security.clear_revealed_otp_code(&otp_id);
        match self.session.prompt_otp_provider().preview_otp_code(&otp_id) {
            Ok(Some(preview)) => {
                let code = preview.code;
                self.security.reveal_otp_code(otp_id.clone(), code);
                self.security
                    .set_status(format!("OTP code ready for {}", compact_id(&otp_id)));
                self.shell.set_status("OTP code ready".to_string());
            }
            Ok(None) => {
                self.security.set_status("OTP entry not found");
            }
            Err(error) => {
                self.security.set_status(error);
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn refresh_visible_security_otp_codes(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ids = self
            .security
            .otp_entries()
            .iter()
            .filter(|entry| entry.otp_type.eq_ignore_ascii_case("totp"))
            .map(|entry| entry.id.clone())
            .collect::<Vec<_>>();
        let mut refreshed = 0usize;
        for otp_id in ids {
            if let Ok(Some(preview)) = self.session.prompt_otp_provider().preview_otp_code(&otp_id)
            {
                self.security.reveal_otp_code(otp_id, preview.code);
                refreshed += 1;
            }
        }
        if refreshed > 0 {
            self.security
                .set_status(format!("refreshed {refreshed} OTP code(s)"));
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
        if let Some(code) = self.security.revealed_otp_code(&otp_id).map(str::to_string)
            && code != "------"
            && !code.trim().is_empty()
        {
            cx.write_to_clipboard(ClipboardItem::new_string(code));
            self.security
                .set_status(format!("OTP code copied ({})", compact_id(&otp_id)));
            self.shell.set_status("OTP code copied".to_string());
            cx.notify();
            return;
        }
        self.generate_security_otp_code(otp_id.clone(), window, cx);
        if let Some(code) = self.security.revealed_otp_code(&otp_id).map(str::to_string)
            && code != "------"
            && !code.trim().is_empty()
        {
            cx.write_to_clipboard(ClipboardItem::new_string(code));
            self.security
                .set_status(format!("OTP code copied ({})", compact_id(&otp_id)));
            self.shell.set_status("OTP code copied".to_string());
            cx.notify();
        }
    }
}

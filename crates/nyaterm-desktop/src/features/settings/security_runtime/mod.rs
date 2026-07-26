use super::*;

mod credentials;
mod delete;
mod keys;
mod otp;
mod passwords;
mod unlock;

impl NyaTermApp {
    /// Apply an edit from one of the security editors' inputs.
    ///
    /// `id` is what follows `security.editor.` in the field id, which names the
    /// editor and the field: `key-name`, `otp-secret`, `cred-pass-re`.
    pub(in crate::features) fn apply_security_editor_input(
        &mut self,
        id: &str,
        text: String,
        cx: &mut Context<Self>,
    ) {
        let mut changed = true;
        match id {
            "key-name" | "key-passphrase" => {
                let Some(editor) = self.security.editors.key.as_mut() else {
                    return;
                };
                match id {
                    "key-name" => editor.name = text,
                    _ => editor.passphrase = text,
                }
            }
            "pw-name" | "pw-value" => {
                let Some(editor) = self.security.editors.password.as_mut() else {
                    return;
                };
                match id {
                    "pw-name" => editor.name = text,
                    _ => editor.password = text,
                }
            }
            "otp-issuer" | "otp-username" | "otp-secret" | "otp-digits" | "otp-period"
            | "otp-counter" => {
                let Some(editor) = self.security.editors.otp.as_mut() else {
                    return;
                };
                match id {
                    "otp-issuer" => editor.issuer = text,
                    "otp-username" => editor.username = text,
                    "otp-secret" => editor.secret = text,
                    // The numeric ones keep digits only: the box takes anything,
                    // and the draft is what gets parsed on save.
                    "otp-digits" => editor.digits = digits_only(&text),
                    "otp-period" => editor.period = digits_only(&text),
                    _ => editor.counter = digits_only(&text),
                }
            }
            "cred-name" | "cred-user" | "cred-pass" | "cred-user-re" | "cred-pass-re" => {
                let Some(editor) = self.security.editors.credential.as_mut() else {
                    return;
                };
                match id {
                    "cred-name" => editor.name = text,
                    "cred-user" => editor.username = text,
                    "cred-pass" => editor.password = text,
                    "cred-user-re" => editor.username_prompt_regex = text,
                    _ => editor.password_prompt_regex = text,
                }
            }
            _ => changed = false,
        }
        if changed {
            cx.notify();
        }
    }
}

/// Keep only the digits, for the fields that are parsed as numbers on save.
fn digits_only(text: &str) -> String {
    text.chars().filter(char::is_ascii_digit).collect()
}

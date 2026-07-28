use gpui::Context;

use crate::features::NyaTermApp;

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
        if self.security.apply_editor_input(id, text) {
            cx.notify();
        }
    }
}

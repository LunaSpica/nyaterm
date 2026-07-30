use gpui::{Context, Window};
use nyaterm_core::ConnectionStore;

use crate::features::NyaTermApp;
use crate::models::SecurityAuthTab;

impl NyaTermApp {
    pub(in crate::features) fn open_security_delete_dialog(
        &mut self,
        kind: SecurityAuthTab,
        id: String,
        label: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let (title_key, description_key) = match kind {
            SecurityAuthTab::Keys => ("settings.deleteKey", "settings.deleteKeyConfirm"),
            SecurityAuthTab::Passwords => (
                "passwordManager.deleteTitle",
                "passwordManager.deleteConfirm",
            ),
            SecurityAuthTab::Credentials => (
                "credentialManager.deleteTitle",
                "credentialManager.deleteConfirm",
            ),
            SecurityAuthTab::Otp => ("otpManager.deleteTitle", "otpManager.deleteConfirm"),
        };
        let message = self.tr(description_key).replace("{{name}}", &label);
        self.open_confirm_dialog(
            (
                self.tr(title_key).to_string(),
                message,
                self.tr("common.delete").to_string(),
                true,
                move |app, _, cx| app.delete_security_item(kind, id.clone(), label.clone(), cx),
            ),
            window,
            cx,
        );
    }

    fn delete_security_item(
        &mut self,
        kind: SecurityAuthTab,
        id: String,
        label: String,
        cx: &mut Context<Self>,
    ) -> bool {
        let store = match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            Ok(store) => store,
            Err(error) => {
                self.security.set_status(error.to_string());
                cx.notify();
                return false;
            }
        };
        let result = match kind {
            SecurityAuthTab::Keys => store.delete_ssh_key(&id),
            SecurityAuthTab::Passwords => store.delete_password(&id),
            SecurityAuthTab::Credentials => store.delete_credential(&id),
            SecurityAuthTab::Otp => store.delete_otp_entry(&id),
        };
        match result {
            Ok(()) => {
                self.security.clear_revealed_for_deleted(kind, &id);
                self.refresh_security_catalog();
                let status = format!("{label} deleted");
                self.security.set_status(status.clone());
                self.shell.set_status(status);
                cx.notify();
                true
            }
            Err(error) => {
                self.security.set_status(error.to_string());
                cx.notify();
                false
            }
        }
    }
}

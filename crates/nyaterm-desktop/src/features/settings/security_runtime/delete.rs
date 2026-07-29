use gpui::Context;
use nyaterm_core::ConnectionStore;

use crate::features::NyaTermApp;
use crate::models::SecurityAuthTab;

impl NyaTermApp {
    pub(in crate::features) fn cancel_security_delete(&mut self, cx: &mut Context<Self>) {
        self.security.cancel_delete();
        cx.notify();
    }

    pub(in crate::features) fn confirm_security_delete(&mut self, cx: &mut Context<Self>) {
        let Some(confirm) = self.security.delete_confirm().cloned() else {
            return;
        };
        let store = match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            Ok(store) => store,
            Err(error) => {
                self.security.set_status(error.to_string());
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
                self.security
                    .clear_revealed_for_deleted(confirm.kind, &confirm.id);
                self.refresh_security_catalog();
                self.security.cancel_delete();
                let status = format!("{} deleted", confirm.label);
                self.security.set_status(status.clone());
                self.shell.status = status;
            }
            Err(error) => {
                self.security.set_status(error.to_string());
            }
        }
        cx.notify();
    }
}

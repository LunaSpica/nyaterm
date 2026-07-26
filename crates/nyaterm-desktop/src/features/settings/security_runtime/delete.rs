use super::*;

use crate::models::SecurityAuthTab;

impl NyaTermApp {
    pub(in crate::features) fn cancel_security_delete(&mut self, cx: &mut Context<Self>) {
        self.security_delete_confirm = None;
        cx.notify();
    }

    pub(in crate::features) fn confirm_security_delete(&mut self, cx: &mut Context<Self>) {
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
}

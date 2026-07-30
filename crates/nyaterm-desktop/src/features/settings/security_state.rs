//! Grouped security panel state.
//!
//! Only UI state lives here. Keys, passwords, credentials and OTP entries stay
//! in `nyaterm-core`; the maps below hold values the user has explicitly
//! revealed or codes generated for display, and they are cleared through the
//! same paths as before.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use gpui::FocusHandle;
use nyaterm_core::{OtpEntry, SavedCredential, SavedPassword, SshKey};

use crate::models::{
    SecurityAuthTab, SecurityCredentialEditorState, SecurityDeleteConfirmState,
    SecurityKeyEditorState, SecurityOtpEditorState, SecurityPasswordEditorState,
    SecurityUnlockAction,
};

pub(in crate::features) struct SecurityFeatureState {
    catalog: SecurityCatalogState,
    auth_tab: SecurityAuthTab,
    editors: SecurityEditorState,
    delete_confirm: Option<SecurityDeleteConfirmState>,
    revealed: SecurityRevealedState,
    status: String,
    unlock: SecurityUnlockState,
    screen_lock: SecurityScreenLockState,
}

/// Persisted secret-adjacent catalogs loaded through `ConnectionStore`.
///
/// This type deliberately has no `Debug` implementation so callers cannot
/// accidentally log secret-bearing entries through the feature state.
pub(in crate::features) struct SecurityCatalogState {
    ssh_keys: Vec<SshKey>,
    otp_entries: Vec<OtpEntry>,
    passwords: Vec<SavedPassword>,
    credentials: Vec<SavedCredential>,
}

impl SecurityCatalogState {
    pub(in crate::features) fn new(
        ssh_keys: Vec<SshKey>,
        otp_entries: Vec<OtpEntry>,
        passwords: Vec<SavedPassword>,
        credentials: Vec<SavedCredential>,
    ) -> Self {
        Self {
            ssh_keys,
            otp_entries,
            passwords,
            credentials,
        }
    }
}

/// Focus handles the security panel needs at construction time.
pub(in crate::features) struct SecurityFeatureFocus {
    pub key_editor: FocusHandle,
    pub otp_editor: FocusHandle,
    pub password_editor: FocusHandle,
    pub credential_editor: FocusHandle,
    pub unlock: FocusHandle,
    pub screen_lock: FocusHandle,
}

/// The four security editors, each an optional draft plus its focus handle.
struct SecurityEditorState {
    key: Option<SecurityKeyEditorState>,
    key_focus: FocusHandle,
    otp: Option<SecurityOtpEditorState>,
    otp_focus: FocusHandle,
    otp_qr_importing: bool,
    password: Option<SecurityPasswordEditorState>,
    password_focus: FocusHandle,
    credential: Option<SecurityCredentialEditorState>,
    credential_focus: FocusHandle,
}

/// Values the user has explicitly revealed, plus generated OTP codes.
struct SecurityRevealedState {
    otp_codes: HashMap<String, String>,
    passwords: HashMap<String, String>,
    credentials: HashMap<String, String>,
}

/// Master password unlock prompt.
struct SecurityUnlockState {
    secrets_unlocked: bool,
    prompt_open: bool,
    master_required_prompt_open: bool,
    draft: String,
    error: Option<String>,
    pending_action: Option<SecurityUnlockAction>,
    focus: FocusHandle,
}

/// Whole-application idle/manual lock screen.
///
/// This is distinct from `SecurityUnlockState`, which gates access to stored
/// secrets while the rest of the application remains usable.
struct SecurityScreenLockState {
    locked: bool,
    password_draft: String,
    status: String,
    focus: FocusHandle,
    last_user_activity_at: Instant,
}

impl SecurityFeatureState {
    pub(in crate::features) fn new(
        catalog: SecurityCatalogState,
        secrets_unlocked: bool,
        status: String,
        focus: SecurityFeatureFocus,
    ) -> Self {
        Self {
            catalog,
            auth_tab: SecurityAuthTab::Keys,
            editors: SecurityEditorState {
                key: None,
                key_focus: focus.key_editor,
                otp: None,
                otp_focus: focus.otp_editor,
                otp_qr_importing: false,
                password: None,
                password_focus: focus.password_editor,
                credential: None,
                credential_focus: focus.credential_editor,
            },
            delete_confirm: None,
            revealed: SecurityRevealedState {
                otp_codes: HashMap::new(),
                passwords: HashMap::new(),
                credentials: HashMap::new(),
            },
            status,
            unlock: SecurityUnlockState {
                secrets_unlocked,
                prompt_open: false,
                master_required_prompt_open: false,
                draft: String::new(),
                error: None,
                pending_action: None,
                focus: focus.unlock,
            },
            screen_lock: SecurityScreenLockState {
                locked: false,
                password_draft: String::new(),
                status: String::new(),
                focus: focus.screen_lock,
                last_user_activity_at: Instant::now(),
            },
        }
    }

    pub(in crate::features) fn ssh_keys(&self) -> &[SshKey] {
        &self.catalog.ssh_keys
    }

    pub(in crate::features) fn otp_entries(&self) -> &[OtpEntry] {
        &self.catalog.otp_entries
    }

    pub(in crate::features) fn passwords(&self) -> &[SavedPassword] {
        &self.catalog.passwords
    }

    pub(in crate::features) fn credentials(&self) -> &[SavedCredential] {
        &self.catalog.credentials
    }

    pub(in crate::features) fn replace_catalog(
        &mut self,
        ssh_keys: Vec<SshKey>,
        otp_entries: Vec<OtpEntry>,
        passwords: Vec<SavedPassword>,
        credentials: Vec<SavedCredential>,
    ) {
        self.catalog = SecurityCatalogState::new(ssh_keys, otp_entries, passwords, credentials);
    }

    pub(in crate::features) fn clear_catalog(&mut self) {
        self.replace_catalog(Vec::new(), Vec::new(), Vec::new(), Vec::new());
    }

    pub(in crate::features) fn auth_tab(&self) -> SecurityAuthTab {
        self.auth_tab
    }

    pub(in crate::features) fn set_auth_tab(&mut self, tab: SecurityAuthTab) {
        self.auth_tab = tab;
        self.status = format!("{} tab", tab.label().to_lowercase());
    }

    pub(in crate::features) fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    pub(in crate::features) fn secrets_unlocked(&self) -> bool {
        self.unlock.secrets_unlocked
    }

    pub(in crate::features) fn unlock_prompt_open(&self) -> bool {
        self.unlock.prompt_open
    }

    pub(in crate::features) fn master_required_prompt_open(&self) -> bool {
        self.unlock.master_required_prompt_open
    }

    pub(in crate::features) fn unlock_draft(&self) -> &str {
        &self.unlock.draft
    }

    pub(in crate::features) fn unlock_error(&self) -> Option<&str> {
        self.unlock.error.as_deref()
    }

    pub(in crate::features) fn unlock_focus(&self) -> &FocusHandle {
        &self.unlock.focus
    }

    pub(in crate::features) fn set_pending_unlock_action(
        &mut self,
        action: Option<SecurityUnlockAction>,
    ) {
        self.unlock.pending_action = action;
    }

    pub(in crate::features) fn show_master_required_prompt(&mut self) {
        self.unlock.pending_action = None;
        self.unlock.prompt_open = false;
        self.unlock.master_required_prompt_open = true;
        self.unlock.draft.clear();
        self.unlock.error = None;
        self.status = "master password required".to_string();
    }

    pub(in crate::features) fn show_unlock_prompt(&mut self) {
        self.unlock.master_required_prompt_open = false;
        self.unlock.prompt_open = true;
        self.unlock.draft.clear();
        self.unlock.error = None;
        self.status = "enter master password to unlock secrets".to_string();
    }

    pub(in crate::features) fn cancel_unlock_prompt(&mut self) {
        self.unlock.pending_action = None;
        self.close_unlock_prompt();
    }

    pub(in crate::features) fn close_master_required_prompt(&mut self) {
        self.unlock.master_required_prompt_open = false;
        self.unlock.pending_action = None;
    }

    pub(in crate::features) fn complete_unlock(&mut self) -> Option<SecurityUnlockAction> {
        let pending_action = self.unlock.pending_action.take();
        self.unlock.secrets_unlocked = true;
        self.status = "secrets unlocked".to_string();
        self.close_unlock_prompt();
        pending_action
    }

    pub(in crate::features) fn reject_unlock(&mut self, error: String, status: &'static str) {
        self.unlock.draft.clear();
        self.unlock.error = Some(error);
        self.status = status.to_string();
    }

    pub(in crate::features) fn apply_unlock_input(&mut self, text: String) {
        self.unlock.draft = text;
        self.unlock.error = None;
    }

    pub(in crate::features) fn unlock_without_master_password(&mut self) {
        self.unlock.secrets_unlocked = true;
    }

    pub(in crate::features) fn screen_locked(&self) -> bool {
        self.screen_lock.locked
    }

    pub(in crate::features) fn screen_lock_password_draft(&self) -> &str {
        &self.screen_lock.password_draft
    }

    pub(in crate::features) fn screen_lock_status(&self) -> &str {
        &self.screen_lock.status
    }

    pub(in crate::features) fn screen_lock_focus(&self) -> &FocusHandle {
        &self.screen_lock.focus
    }

    pub(in crate::features) fn screen_lock_idle_for(&self) -> Duration {
        self.screen_lock.last_user_activity_at.elapsed()
    }

    pub(in crate::features) fn activate_screen_lock(&mut self, status: String) {
        self.screen_lock.locked = true;
        self.screen_lock.password_draft.clear();
        self.screen_lock.status = status;
    }

    pub(in crate::features) fn deactivate_screen_lock(&mut self) {
        self.screen_lock.locked = false;
        self.screen_lock.password_draft.clear();
        self.screen_lock.status.clear();
        self.screen_lock.last_user_activity_at = Instant::now();
    }

    pub(in crate::features) fn record_screen_lock_user_activity(&mut self) {
        if !self.screen_lock.locked {
            self.reset_screen_lock_idle_timer();
        }
    }

    pub(in crate::features) fn reset_screen_lock_idle_timer(&mut self) {
        self.screen_lock.last_user_activity_at = Instant::now();
    }

    pub(in crate::features) fn set_screen_lock_password_draft(
        &mut self,
        text: String,
        status: String,
    ) {
        self.screen_lock.password_draft = text;
        self.screen_lock.status = status;
    }

    pub(in crate::features) fn clear_screen_lock_password_with_status(&mut self, status: String) {
        self.screen_lock.password_draft.clear();
        self.screen_lock.status = status;
    }

    pub(in crate::features) fn revealed_password(&self, id: &str) -> Option<&str> {
        self.revealed.passwords.get(id).map(String::as_str)
    }

    pub(in crate::features) fn hide_revealed_password(&mut self, id: &str) -> bool {
        self.revealed.passwords.remove(id).is_some()
    }

    pub(in crate::features) fn reveal_password(&mut self, id: String, value: String) {
        self.revealed.passwords.insert(id, value);
    }

    pub(in crate::features) fn revealed_credential(&self, id: &str) -> Option<&str> {
        self.revealed.credentials.get(id).map(String::as_str)
    }

    pub(in crate::features) fn hide_revealed_credential(&mut self, id: &str) -> bool {
        self.revealed.credentials.remove(id).is_some()
    }

    pub(in crate::features) fn reveal_credential(&mut self, id: String, value: String) {
        self.revealed.credentials.insert(id, value);
    }

    pub(in crate::features) fn revealed_otp_code(&self, id: &str) -> Option<&str> {
        self.revealed.otp_codes.get(id).map(String::as_str)
    }

    pub(in crate::features) fn clear_revealed_otp_code(&mut self, id: &str) {
        self.revealed.otp_codes.remove(id);
    }

    pub(in crate::features) fn reveal_otp_code(&mut self, id: String, code: String) {
        self.revealed.otp_codes.insert(id, code);
    }

    pub(in crate::features) fn clear_revealed_for_deleted(
        &mut self,
        kind: SecurityAuthTab,
        id: &str,
    ) {
        match kind {
            SecurityAuthTab::Otp => {
                self.revealed.otp_codes.remove(id);
            }
            SecurityAuthTab::Passwords => {
                self.revealed.passwords.remove(id);
            }
            SecurityAuthTab::Credentials => {
                self.revealed.credentials.remove(id);
            }
            SecurityAuthTab::Keys => {}
        }
    }

    pub(in crate::features) fn key_editor(&self) -> Option<&SecurityKeyEditorState> {
        self.editors.key.as_ref()
    }

    pub(in crate::features) fn key_editor_mut(&mut self) -> Option<&mut SecurityKeyEditorState> {
        self.editors.key.as_mut()
    }

    pub(in crate::features) fn key_editor_focus(&self) -> &FocusHandle {
        &self.editors.key_focus
    }

    pub(in crate::features) fn otp_editor(&self) -> Option<&SecurityOtpEditorState> {
        self.editors.otp.as_ref()
    }

    pub(in crate::features) fn otp_editor_mut(&mut self) -> Option<&mut SecurityOtpEditorState> {
        self.editors.otp.as_mut()
    }

    pub(in crate::features) fn otp_editor_focus(&self) -> &FocusHandle {
        &self.editors.otp_focus
    }

    pub(in crate::features) fn otp_qr_importing(&self) -> bool {
        self.editors.otp_qr_importing
    }

    pub(in crate::features) fn password_editor(&self) -> Option<&SecurityPasswordEditorState> {
        self.editors.password.as_ref()
    }

    pub(in crate::features) fn password_editor_mut(
        &mut self,
    ) -> Option<&mut SecurityPasswordEditorState> {
        self.editors.password.as_mut()
    }

    pub(in crate::features) fn password_editor_focus(&self) -> &FocusHandle {
        &self.editors.password_focus
    }

    pub(in crate::features) fn credential_editor(&self) -> Option<&SecurityCredentialEditorState> {
        self.editors.credential.as_ref()
    }

    pub(in crate::features) fn credential_editor_mut(
        &mut self,
    ) -> Option<&mut SecurityCredentialEditorState> {
        self.editors.credential.as_mut()
    }

    pub(in crate::features) fn credential_editor_focus(&self) -> &FocusHandle {
        &self.editors.credential_focus
    }

    pub(in crate::features) fn open_key_editor(
        &mut self,
        editor: SecurityKeyEditorState,
        status: String,
    ) {
        self.clear_editors();
        self.editors.key = Some(editor);
        self.delete_confirm = None;
        self.status = status;
    }

    pub(in crate::features) fn open_otp_editor(
        &mut self,
        editor: SecurityOtpEditorState,
        status: String,
    ) {
        self.clear_editors();
        self.editors.otp = Some(editor);
        self.delete_confirm = None;
        self.status = status;
    }

    pub(in crate::features) fn open_password_editor(
        &mut self,
        editor: SecurityPasswordEditorState,
        status: String,
    ) {
        self.clear_editors();
        self.editors.password = Some(editor);
        self.delete_confirm = None;
        self.status = status;
    }

    pub(in crate::features) fn open_credential_editor(
        &mut self,
        editor: SecurityCredentialEditorState,
        status: String,
    ) {
        self.clear_editors();
        self.editors.credential = Some(editor);
        self.delete_confirm = None;
        self.status = status;
    }

    pub(in crate::features) fn finish_key_editor(&mut self, status: String) {
        self.editors.key = None;
        self.status = status;
    }

    pub(in crate::features) fn finish_otp_editor(&mut self, status: String) {
        self.editors.otp = None;
        self.status = status;
    }

    pub(in crate::features) fn finish_password_editor(&mut self, status: String) {
        self.editors.password = None;
        self.status = status;
    }

    pub(in crate::features) fn finish_credential_editor(&mut self, status: String) {
        self.editors.credential = None;
        self.status = status;
    }

    pub(in crate::features) fn begin_otp_qr_import(&mut self, status: String) -> bool {
        if self.editors.otp_qr_importing || self.editors.otp.is_some() {
            return false;
        }
        self.editors.otp_qr_importing = true;
        self.status = status;
        true
    }

    pub(in crate::features) fn finish_otp_qr_import(&mut self) {
        self.editors.otp_qr_importing = false;
    }

    pub(in crate::features) fn delete_confirm(&self) -> Option<&SecurityDeleteConfirmState> {
        self.delete_confirm.as_ref()
    }

    pub(in crate::features) fn request_delete(&mut self, confirm: SecurityDeleteConfirmState) {
        self.delete_confirm = Some(confirm);
    }

    pub(in crate::features) fn apply_editor_input(&mut self, id: &str, text: String) -> bool {
        match id {
            "key-name" | "key-passphrase" => {
                let Some(editor) = self.key_editor_mut() else {
                    return false;
                };
                match id {
                    "key-name" => editor.name = text,
                    _ => editor.passphrase = text,
                }
            }
            "pw-name" | "pw-value" => {
                let Some(editor) = self.password_editor_mut() else {
                    return false;
                };
                match id {
                    "pw-name" => editor.name = text,
                    _ => editor.password = text,
                }
            }
            "otp-issuer" | "otp-username" | "otp-secret" | "otp-digits" | "otp-period"
            | "otp-counter" => {
                let Some(editor) = self.otp_editor_mut() else {
                    return false;
                };
                match id {
                    "otp-issuer" => editor.issuer = text,
                    "otp-username" => editor.username = text,
                    "otp-secret" => editor.secret = text,
                    "otp-digits" => editor.digits = digits_only(&text),
                    "otp-period" => editor.period = digits_only(&text),
                    _ => editor.counter = digits_only(&text),
                }
            }
            "cred-name" | "cred-user" | "cred-pass" | "cred-user-re" | "cred-pass-re" => {
                let Some(editor) = self.credential_editor_mut() else {
                    return false;
                };
                match id {
                    "cred-name" => editor.name = text,
                    "cred-user" => editor.username = text,
                    "cred-pass" => editor.password = text,
                    "cred-user-re" => editor.username_prompt_regex = text,
                    _ => editor.password_prompt_regex = text,
                }
            }
            _ => return false,
        }
        true
    }

    fn clear_editors(&mut self) {
        self.editors.key = None;
        self.editors.otp = None;
        self.editors.password = None;
        self.editors.credential = None;
    }
}

fn digits_only(text: &str) -> String {
    text.chars().filter(char::is_ascii_digit).collect()
}

/// Panel transitions that only rearrange security UI state.
///
/// These live on the state rather than on `NyaTermApp` so closing an editor or
/// locking secrets cannot reach any other app state. Callers own the redraw.
impl SecurityFeatureState {
    pub(in crate::features) fn close_key_editor(&mut self) {
        self.editors.key = None;
        self.status = "SSH key editor closed".to_string();
    }

    pub(in crate::features) fn close_otp_editor(&mut self) {
        self.editors.otp = None;
        self.status = "OTP editor closed".to_string();
    }

    pub(in crate::features) fn close_password_editor(&mut self) {
        self.editors.password = None;
        self.status = "password editor closed".to_string();
    }

    pub(in crate::features) fn close_credential_editor(&mut self) {
        self.editors.credential = None;
        self.status = "credential editor closed".to_string();
    }

    pub(in crate::features) fn cycle_otp_algorithm(&mut self) {
        if let Some(editor) = self.editors.otp.as_mut() {
            editor.algorithm = match editor.algorithm.as_str() {
                "SHA1" => "SHA256".to_string(),
                "SHA256" => "SHA512".to_string(),
                _ => "SHA1".to_string(),
            };
        }
    }

    pub(in crate::features) fn cancel_delete(&mut self) {
        self.delete_confirm = None;
    }

    pub(in crate::features) fn close_unlock_prompt(&mut self) {
        self.unlock.prompt_open = false;
        self.unlock.draft.clear();
        self.unlock.error = None;
    }

    /// Drops every revealed secret and every editor holding one.
    ///
    /// Revealed OTP codes are display-only and regenerate on demand, so they
    /// are left alone here exactly as before.
    pub(in crate::features) fn lock_secrets(&mut self) {
        self.unlock.secrets_unlocked = false;
        self.revealed.passwords.clear();
        self.revealed.credentials.clear();
        self.editors.password = None;
        self.editors.credential = None;
        self.delete_confirm = None;
        self.unlock.pending_action = None;
        self.close_unlock_prompt();
        self.unlock.master_required_prompt_open = false;
        self.status = "secrets locked".to_string();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use gpui::TestAppContext;
    use nyaterm_core::{OtpEntry, SavedCredential, SavedPassword, SshKey};

    use super::{SecurityCatalogState, SecurityFeatureFocus, SecurityFeatureState};
    use crate::models::{
        SecurityAuthTab, SecurityDeleteConfirmState, SecurityKeyEditorField,
        SecurityKeyEditorState, SecurityPasswordEditorState, SecurityUnlockAction,
    };

    fn security_state() -> SecurityFeatureState {
        let cx = TestAppContext::single();
        let focus = || cx.update(|cx| cx.focus_handle());
        SecurityFeatureState::new(
            SecurityCatalogState::new(Vec::new(), Vec::new(), Vec::new(), Vec::new()),
            true,
            "ready".to_string(),
            SecurityFeatureFocus {
                key_editor: focus(),
                otp_editor: focus(),
                password_editor: focus(),
                credential_editor: focus(),
                unlock: focus(),
                screen_lock: focus(),
            },
        )
    }

    #[test]
    fn catalog_replacement_and_clear_update_all_security_collections() {
        let mut security = security_state();

        security.replace_catalog(
            vec![SshKey {
                id: "key-id".to_string(),
                name: "key".to_string(),
                key: None,
                cert: None,
                passphrase: None,
                key_file_path: None,
                cert_file_path: None,
                has_key_data: false,
                has_cert_data: false,
            }],
            vec![OtpEntry {
                id: "otp-id".to_string(),
                otp_type: "totp".to_string(),
                issuer: String::new(),
                username: String::new(),
                secret: None,
                algorithm: "SHA1".to_string(),
                digits: 6,
                period: 30,
                counter: 0,
                has_secret: false,
            }],
            vec![SavedPassword {
                id: "password-id".to_string(),
                name: "password".to_string(),
                password: None,
                has_password: false,
            }],
            vec![SavedCredential {
                id: "credential-id".to_string(),
                name: "credential".to_string(),
                username: String::new(),
                password: None,
                username_prompt_regex: None,
                password_prompt_regex: None,
                enabled: true,
                has_password: false,
            }],
        );

        assert_eq!(security.ssh_keys().len(), 1);
        assert_eq!(security.otp_entries().len(), 1);
        assert_eq!(security.passwords().len(), 1);
        assert_eq!(security.credentials().len(), 1);

        security.clear_catalog();

        assert!(security.ssh_keys().is_empty());
        assert!(security.otp_entries().is_empty());
        assert!(security.passwords().is_empty());
        assert!(security.credentials().is_empty());
    }

    #[test]
    fn opening_an_editor_replaces_the_previous_editor_and_delete_confirmation() {
        let mut security = security_state();
        security.request_delete(SecurityDeleteConfirmState {
            kind: SecurityAuthTab::Passwords,
            id: "password-id".to_string(),
            label: "password".to_string(),
        });

        security.open_password_editor(
            SecurityPasswordEditorState {
                id: None,
                name: String::new(),
                password: String::new(),
                has_password: false,
                show_password: false,
                error: None,
            },
            "password editor opened".to_string(),
        );
        assert!(security.password_editor().is_some());
        assert!(security.delete_confirm().is_none());

        security.open_key_editor(
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
            },
            "SSH key editor opened".to_string(),
        );

        assert!(security.password_editor().is_none());
        assert!(security.key_editor().is_some());
        assert!(security.otp_editor().is_none());
        assert!(security.credential_editor().is_none());
        assert!(security.apply_editor_input("key-name", "new key".to_string()));
        assert_eq!(
            security.key_editor().map(|editor| editor.name.as_str()),
            Some("new key")
        );

        security.finish_key_editor("saved".to_string());
        assert!(security.key_editor().is_none());
        assert_eq!(security.status, "saved");
    }

    #[test]
    fn otp_qr_import_admission_is_owned_by_security_state() {
        let mut security = security_state();

        assert!(security.begin_otp_qr_import("scanning".to_string()));
        assert!(!security.begin_otp_qr_import("duplicate".to_string()));
        assert!(security.otp_qr_importing());
        assert_eq!(security.status, "scanning");

        security.finish_otp_qr_import();
        assert!(!security.otp_qr_importing());
    }

    #[test]
    fn unlock_transitions_keep_pending_action_until_success_or_explicit_close() {
        let mut security = security_state();
        security.set_pending_unlock_action(Some(SecurityUnlockAction::RevealPassword(
            "password-id".to_string(),
        )));
        security.show_unlock_prompt();
        security.apply_unlock_input("attempt".to_string());

        security.reject_unlock("wrong password".to_string(), "unlock rejected");
        assert!(security.unlock_draft().is_empty());
        assert!(security.unlock_error().is_some());
        assert!(security.unlock_prompt_open());

        assert_eq!(
            security.complete_unlock(),
            Some(SecurityUnlockAction::RevealPassword(
                "password-id".to_string()
            ))
        );
        assert!(security.secrets_unlocked());
        assert!(!security.unlock_prompt_open());
        assert!(security.unlock_error().is_none());

        security.set_pending_unlock_action(Some(SecurityUnlockAction::RevealCredential(
            "credential-id".to_string(),
        )));
        security.show_master_required_prompt();
        assert!(security.master_required_prompt_open());
        assert_eq!(security.complete_unlock(), None);
    }

    #[test]
    fn locking_secrets_clears_revealed_passwords_and_credentials_but_keeps_otp_codes() {
        let mut security = security_state();
        security.reveal_password("password-id".to_string(), "value".to_string());
        security.reveal_credential("credential-id".to_string(), "value".to_string());
        security.reveal_otp_code("otp-id".to_string(), "123456".to_string());

        security.lock_secrets();

        assert!(!security.secrets_unlocked());
        assert!(security.revealed_password("password-id").is_none());
        assert!(security.revealed_credential("credential-id").is_none());
        assert!(security.revealed_otp_code("otp-id").is_some());
    }

    #[test]
    fn screen_lock_lifecycle_clears_password_and_resets_activity() {
        let mut security = security_state();
        security.set_screen_lock_password_draft("secret".to_string(), "ready".to_string());
        security.screen_lock.last_user_activity_at =
            std::time::Instant::now() - Duration::from_secs(60);
        let stale_activity = security.screen_lock.last_user_activity_at;

        security.activate_screen_lock("locked".to_string());
        assert!(security.screen_locked());
        assert!(security.screen_lock_password_draft().is_empty());
        assert_eq!(security.screen_lock_status(), "locked");

        security.set_screen_lock_password_draft("retry".to_string(), "retry".to_string());
        security.deactivate_screen_lock();
        assert!(!security.screen_locked());
        assert!(security.screen_lock_password_draft().is_empty());
        assert!(security.screen_lock_status().is_empty());
        assert!(security.screen_lock.last_user_activity_at > stale_activity);
    }

    #[test]
    fn locked_screen_does_not_record_background_activity() {
        let mut security = security_state();
        security.activate_screen_lock("locked".to_string());
        let locked_at = security.screen_lock.last_user_activity_at;

        security.record_screen_lock_user_activity();

        assert_eq!(security.screen_lock.last_user_activity_at, locked_at);
    }
}

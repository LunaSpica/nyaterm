//! Grouped security panel state.
//!
//! Only UI state lives here. Keys, passwords, credentials and OTP entries stay
//! in `nyaterm-core`; the maps below hold values the user has explicitly
//! revealed or codes generated for display, and they are cleared through the
//! same paths as before.

use std::collections::HashMap;
use std::time::Instant;

use gpui::FocusHandle;
use nyaterm_core::{OtpEntry, SavedCredential, SavedPassword, SshKey};

use crate::models::{
    SecurityAuthTab, SecurityCredentialEditorState, SecurityDeleteConfirmState,
    SecurityKeyEditorState, SecurityOtpEditorState, SecurityPasswordEditorState,
    SecurityUnlockAction,
};

pub(in crate::features) struct SecurityFeatureState {
    pub catalog: SecurityCatalogState,
    pub auth_tab: SecurityAuthTab,
    pub editors: SecurityEditorState,
    pub delete_confirm: Option<SecurityDeleteConfirmState>,
    pub revealed: SecurityRevealedState,
    pub status: String,
    pub unlock: SecurityUnlockState,
    pub screen_lock: SecurityScreenLockState,
}

/// Persisted secret-adjacent catalogs loaded through `ConnectionStore`.
///
/// This type deliberately has no `Debug` implementation so callers cannot
/// accidentally log secret-bearing entries through the feature state.
pub(in crate::features) struct SecurityCatalogState {
    pub ssh_keys: Vec<SshKey>,
    pub otp_entries: Vec<OtpEntry>,
    pub passwords: Vec<SavedPassword>,
    pub credentials: Vec<SavedCredential>,
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
pub(in crate::features) struct SecurityEditorState {
    pub key: Option<SecurityKeyEditorState>,
    pub key_focus: FocusHandle,
    pub otp: Option<SecurityOtpEditorState>,
    pub otp_focus: FocusHandle,
    pub otp_qr_importing: bool,
    pub password: Option<SecurityPasswordEditorState>,
    pub password_focus: FocusHandle,
    pub credential: Option<SecurityCredentialEditorState>,
    pub credential_focus: FocusHandle,
}

/// Values the user has explicitly revealed, plus generated OTP codes.
pub(in crate::features) struct SecurityRevealedState {
    pub otp_codes: HashMap<String, String>,
    pub passwords: HashMap<String, String>,
    pub credentials: HashMap<String, String>,
}

/// Master password unlock prompt.
pub(in crate::features) struct SecurityUnlockState {
    pub secrets_unlocked: bool,
    pub prompt_open: bool,
    pub master_required_prompt_open: bool,
    pub draft: String,
    pub error: Option<String>,
    pub pending_action: Option<SecurityUnlockAction>,
    pub focus: FocusHandle,
}

/// Whole-application idle/manual lock screen.
///
/// This is distinct from `SecurityUnlockState`, which gates access to stored
/// secrets while the rest of the application remains usable.
pub(in crate::features) struct SecurityScreenLockState {
    pub locked: bool,
    pub password_draft: String,
    pub status: String,
    pub focus: FocusHandle,
    pub last_user_activity_at: Instant,
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
}

impl SecurityScreenLockState {
    pub(in crate::features) fn activate(&mut self, status: String) {
        self.locked = true;
        self.password_draft.clear();
        self.status = status;
    }

    pub(in crate::features) fn deactivate(&mut self) {
        self.locked = false;
        self.password_draft.clear();
        self.status.clear();
        self.last_user_activity_at = Instant::now();
    }

    pub(in crate::features) fn record_user_activity(&mut self) {
        if !self.locked {
            self.reset_idle_timer();
        }
    }

    pub(in crate::features) fn reset_idle_timer(&mut self) {
        self.last_user_activity_at = Instant::now();
    }

    pub(in crate::features) fn set_password_draft(&mut self, text: String, status: String) {
        self.password_draft = text;
        self.status = status;
    }

    pub(in crate::features) fn clear_password_with_status(&mut self, status: String) {
        self.password_draft.clear();
        self.status = status;
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use gpui::TestAppContext;

    use super::{SecurityCatalogState, SecurityFeatureFocus, SecurityFeatureState};

    fn security_state() -> SecurityFeatureState {
        let cx = TestAppContext::single();
        let focus = || cx.update(|cx| cx.focus_handle());
        SecurityFeatureState::new(
            SecurityCatalogState {
                ssh_keys: Vec::new(),
                otp_entries: Vec::new(),
                passwords: Vec::new(),
                credentials: Vec::new(),
            },
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
    fn screen_lock_lifecycle_clears_password_and_resets_activity() {
        let mut security = security_state();
        security.screen_lock.password_draft = "secret".to_string();
        security.screen_lock.last_user_activity_at =
            std::time::Instant::now() - Duration::from_secs(60);
        let stale_activity = security.screen_lock.last_user_activity_at;

        security.screen_lock.activate("locked".to_string());
        assert!(security.screen_lock.locked);
        assert!(security.screen_lock.password_draft.is_empty());
        assert_eq!(security.screen_lock.status, "locked");

        security.screen_lock.password_draft = "retry".to_string();
        security.screen_lock.deactivate();
        assert!(!security.screen_lock.locked);
        assert!(security.screen_lock.password_draft.is_empty());
        assert!(security.screen_lock.status.is_empty());
        assert!(security.screen_lock.last_user_activity_at > stale_activity);
    }

    #[test]
    fn locked_screen_does_not_record_background_activity() {
        let mut security = security_state();
        security.screen_lock.activate("locked".to_string());
        let locked_at = security.screen_lock.last_user_activity_at;

        security.screen_lock.record_user_activity();

        assert_eq!(security.screen_lock.last_user_activity_at, locked_at);
    }
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

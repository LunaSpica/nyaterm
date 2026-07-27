//! Grouped security panel state.
//!
//! Only UI state lives here. Keys, passwords, credentials and OTP entries stay
//! in `nyaterm-core`; the maps below hold values the user has explicitly
//! revealed or codes generated for display, and they are cleared through the
//! same paths as before.

use std::collections::HashMap;

use gpui::FocusHandle;

use crate::models::{
    SecurityAuthTab, SecurityCredentialEditorState, SecurityDeleteConfirmState,
    SecurityKeyEditorState, SecurityOtpEditorState, SecurityPasswordEditorState,
    SecurityUnlockAction,
};

pub(in crate::features) struct SecurityFeatureState {
    pub auth_tab: SecurityAuthTab,
    pub editors: SecurityEditorState,
    pub delete_confirm: Option<SecurityDeleteConfirmState>,
    pub revealed: SecurityRevealedState,
    pub status: String,
    pub unlock: SecurityUnlockState,
}

/// Focus handles the security panel needs at construction time.
pub(in crate::features) struct SecurityFeatureFocus {
    pub key_editor: FocusHandle,
    pub otp_editor: FocusHandle,
    pub password_editor: FocusHandle,
    pub credential_editor: FocusHandle,
    pub unlock: FocusHandle,
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

impl SecurityFeatureState {
    pub(in crate::features) fn new(
        secrets_unlocked: bool,
        status: String,
        focus: SecurityFeatureFocus,
    ) -> Self {
        Self {
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
        }
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

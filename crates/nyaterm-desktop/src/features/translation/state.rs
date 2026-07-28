//! Authoritative transient state for translation settings and jobs.

use std::sync::mpsc;

use nyaterm_core::{TranslateResult, TranslationSettings};

use crate::models::{TranslateInputField, TranslationDialogState, TranslationSecretDraft};

pub(super) struct TranslateJobResult {
    pub result: Result<TranslateResult, String>,
}

pub(in crate::features) struct TranslationFeatureState {
    pub dialog: Option<TranslationDialogState>,
    pub(super) tx: mpsc::Sender<TranslateJobResult>,
    pub(super) rx: mpsc::Receiver<TranslateJobResult>,
    pub provider: String,
    pub settings: TranslationSettings,
    pub secret_draft: TranslationSecretDraft,
    pub target_language: String,
    pub input: String,
    pub result: Option<TranslateResult>,
    pub status: String,
    pub pending: bool,
    pub focused_field: TranslateInputField,
}

impl TranslationFeatureState {
    pub(in crate::features) fn new(settings: TranslationSettings) -> Self {
        let (tx, rx) = mpsc::channel();
        let target_language = settings.target_language.clone();
        Self {
            dialog: None,
            tx,
            rx,
            provider: "google".to_string(),
            settings,
            secret_draft: TranslationSecretDraft::default(),
            target_language,
            input: String::new(),
            result: None,
            status: "Google translation ready".to_string(),
            pending: false,
            focused_field: TranslateInputField::Text,
        }
    }
}

#[cfg(test)]
mod tests {
    use nyaterm_core::TranslationSettings;

    use super::TranslationFeatureState;

    #[test]
    fn translation_state_owns_job_channel_and_loaded_settings() {
        let settings = TranslationSettings {
            target_language: "ja".to_string(),
            ..TranslationSettings::default()
        };

        let state = TranslationFeatureState::new(settings.clone());

        assert_eq!(state.settings, settings);
        assert_eq!(state.target_language, "ja");
        assert!(state.rx.try_recv().is_err());
        assert!(!state.pending);
        assert!(state.dialog.is_none());
    }
}

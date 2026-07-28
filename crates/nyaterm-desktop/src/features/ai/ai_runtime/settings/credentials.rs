use gpui::{Context, Window};

use crate::features::NyaTermApp;
use crate::models::AiCredentialEditorField;

use super::super::helpers::{is_builtin_ai_provider_id, seed_builtin_ai_models_for_provider};

impl NyaTermApp {
    pub(in crate::features) fn toggle_ai_credential_enabled(
        &mut self,
        credential_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self
            .ai
            .settings
            .config
            .provider_credentials
            .iter()
            .position(|credential| credential.id == credential_id)
        else {
            return;
        };
        let enabled = !self.ai.settings.config.provider_credentials[index].enabled;
        let name = self.ai.settings.config.provider_credentials[index]
            .name
            .clone();
        let provider_kind = self.ai.settings.config.provider_credentials[index]
            .provider_kind
            .clone();
        let is_builtin = is_builtin_ai_provider_id(&credential_id);
        self.ai.settings.config.provider_credentials[index].enabled = enabled;

        // Keep matching provider profile enablement in sync for built-ins.
        if let Some(profile) = self
            .ai
            .settings
            .config
            .provider_profiles
            .iter_mut()
            .find(|profile| profile.id == credential_id)
        {
            profile.enabled = enabled;
        }

        if is_builtin {
            if enabled {
                seed_builtin_ai_models_for_provider(&mut self.ai.settings.config, &provider_kind);
            } else {
                self.ai.settings.config.models.retain(|model| {
                    model.provider_kind.as_ref() != Some(&provider_kind)
                        || model.credential_id.is_some()
                });
                if self
                    .ai
                    .settings
                    .config
                    .default_model_id
                    .as_ref()
                    .is_some_and(|id| {
                        !self
                            .ai
                            .settings
                            .config
                            .models
                            .iter()
                            .any(|model| model.id == *id && model.enabled)
                    })
                {
                    self.ai.settings.config.default_model_id = self
                        .ai
                        .settings
                        .config
                        .models
                        .iter()
                        .find(|model| model.enabled)
                        .map(|model| model.id.clone());
                }
            }
        }

        self.ai.panel.status = format!(
            "AI credential {name} {}",
            if enabled { "enabled" } else { "disabled" }
        );
        self.persist_ai_settings_now(cx);
    }

    /// Apply an edit from one of a credential's inputs.
    ///
    /// `rest` is what follows `ai.credential.` in the field id: the credential
    /// id, then the field.
    pub(in crate::features) fn apply_ai_credential_input(
        &mut self,
        rest: &str,
        text: String,
        cx: &mut Context<Self>,
    ) {
        let Some((credential_id, field)) = rest.rsplit_once('.') else {
            return;
        };
        let credential_id = credential_id.to_string();
        match field {
            "api-key" => {
                self.ai
                    .settings
                    .credential_secret_drafts
                    .insert(credential_id.clone(), text);
            }
            "name" | "base-url" => {
                let Some(credential) = self
                    .ai
                    .settings
                    .config
                    .provider_credentials
                    .iter_mut()
                    .find(|credential| credential.id == credential_id)
                else {
                    return;
                };
                if field == "name" {
                    credential.name = text;
                } else {
                    // An empty base URL means "use the provider default", which
                    // the config spells as absent rather than blank.
                    credential.base_url = (!text.trim().is_empty()).then_some(text);
                }
            }
            _ => return,
        }
        self.ai.settings.credential_edit = None;
        self.ai.panel.status = "AI credential edited".to_string();
        cx.notify();
    }

    pub(in crate::features) fn persist_ai_credential_edits(
        &mut self,
        credential_id: &str,
        cx: &mut Context<Self>,
    ) {
        let secret_draft = self
            .ai
            .settings
            .credential_secret_drafts
            .get(credential_id)
            .cloned()
            .unwrap_or_default();
        if let Some(credential) = self
            .ai
            .settings
            .config
            .provider_credentials
            .iter_mut()
            .find(|credential| credential.id == credential_id)
        {
            if !secret_draft.is_empty() {
                credential.api_key = Some(secret_draft.clone());
            }
            // Mirror name/base_url/api_key into matching provider profile when ids align.
            let name = credential.name.clone();
            let base_url = credential.base_url.clone();
            let api_key = credential.api_key.clone();
            let enabled = credential.enabled;
            if let Some(profile) = self
                .ai
                .settings
                .config
                .provider_profiles
                .iter_mut()
                .find(|profile| profile.id == credential_id)
            {
                profile.name = name;
                profile.base_url = base_url;
                if !secret_draft.is_empty() {
                    profile.api_key = Some(secret_draft);
                } else if api_key.is_some() {
                    // Keep existing encrypted key; merge_masked handles __SET__/None.
                }
                profile.enabled = enabled;
            }
        }
        self.ai
            .settings
            .credential_secret_drafts
            .remove(credential_id);
        self.ai.panel.status = "AI credential saved".to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn add_ai_credential(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = format!(
            "credential-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );
        let credential = nyaterm_core::AiProviderCredential {
            id: id.clone(),
            name: String::new(),
            provider_kind: nyaterm_core::AiProviderKind::OpenaiCompatible,
            base_url: Some(String::new()),
            api_key: None,
            enabled: true,
        };
        self.ai
            .settings
            .config
            .provider_credentials
            .insert(0, credential);
        self.ai.settings.credential_edit = Some((id, AiCredentialEditorField::Name));
        window.focus(&self.ai.settings.credential_focus);
        self.ai.panel.status = "AI credential added".to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn remove_ai_credential(
        &mut self,
        credential_id: String,
        cx: &mut Context<Self>,
    ) {
        if is_builtin_ai_provider_id(&credential_id) {
            self.ai.panel.status = "Built-in AI credentials cannot be deleted".to_string();
            cx.notify();
            return;
        }
        self.ai
            .settings
            .config
            .provider_credentials
            .retain(|credential| credential.id != credential_id);
        self.ai
            .settings
            .config
            .models
            .retain(|model| model.credential_id.as_deref() != Some(credential_id.as_str()));
        if self
            .ai
            .settings
            .config
            .default_model_id
            .as_ref()
            .is_some_and(|id| {
                !self
                    .ai
                    .settings
                    .config
                    .models
                    .iter()
                    .any(|model| model.id == *id && model.enabled)
            })
        {
            self.ai.settings.config.default_model_id = self
                .ai
                .settings
                .config
                .models
                .iter()
                .find(|model| model.enabled)
                .map(|model| model.id.clone());
        }
        self.ai.settings.credential_edit = None;
        self.ai
            .settings
            .credential_secret_drafts
            .remove(&credential_id);
        self.ai.panel.status = "AI credential removed".to_string();
        self.persist_ai_settings_now(cx);
    }
}

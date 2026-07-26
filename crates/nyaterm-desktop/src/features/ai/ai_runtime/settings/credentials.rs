use super::*;

use crate::models::AiCredentialEditorField;

impl NyaTermApp {
    pub(in crate::features) fn toggle_ai_credential_enabled(
        &mut self,
        credential_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(index) = self
            .ai_settings
            .provider_credentials
            .iter()
            .position(|credential| credential.id == credential_id)
        else {
            return;
        };
        let enabled = !self.ai_settings.provider_credentials[index].enabled;
        let name = self.ai_settings.provider_credentials[index].name.clone();
        let provider_kind = self.ai_settings.provider_credentials[index]
            .provider_kind
            .clone();
        let is_builtin = is_builtin_ai_provider_id(&credential_id);
        self.ai_settings.provider_credentials[index].enabled = enabled;

        // Keep matching provider profile enablement in sync for built-ins.
        if let Some(profile) = self
            .ai_settings
            .provider_profiles
            .iter_mut()
            .find(|profile| profile.id == credential_id)
        {
            profile.enabled = enabled;
        }

        if is_builtin {
            if enabled {
                seed_builtin_ai_models_for_provider(&mut self.ai_settings, &provider_kind);
            } else {
                self.ai_settings.models.retain(|model| {
                    model.provider_kind.as_ref() != Some(&provider_kind)
                        || model.credential_id.is_some()
                });
                if self
                    .ai_settings
                    .default_model_id
                    .as_ref()
                    .is_some_and(|id| {
                        !self
                            .ai_settings
                            .models
                            .iter()
                            .any(|model| model.id == *id && model.enabled)
                    })
                {
                    self.ai_settings.default_model_id = self
                        .ai_settings
                        .models
                        .iter()
                        .find(|model| model.enabled)
                        .map(|model| model.id.clone());
                }
            }
        }

        self.ai_status = format!(
            "AI credential {name} {}",
            if enabled { "enabled" } else { "disabled" }
        );
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn focus_ai_credential_field(
        &mut self,
        credential_id: String,
        field: AiCredentialEditorField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.ai_credential_edit = Some((credential_id, field));
        window.focus(&self.ai_credential_focus);
        cx.notify();
    }

    pub(in crate::features) fn handle_ai_credential_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let Some((credential_id, field)) = self.ai_credential_edit.clone() else {
            return;
        };
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }
        let builtin = is_builtin_ai_provider_id(&credential_id);
        match keystroke.key.as_str() {
            "backspace" => {
                match field {
                    AiCredentialEditorField::ApiKey => {
                        self.ai_credential_secret_drafts
                            .entry(credential_id.clone())
                            .or_default()
                            .pop();
                    }
                    AiCredentialEditorField::Name | AiCredentialEditorField::BaseUrl => {
                        if let Some(credential) = self
                            .ai_settings
                            .provider_credentials
                            .iter_mut()
                            .find(|credential| credential.id == credential_id)
                        {
                            match field {
                                AiCredentialEditorField::Name => {
                                    credential.name.pop();
                                }
                                AiCredentialEditorField::BaseUrl => {
                                    if let Some(base_url) = credential.base_url.as_mut() {
                                        base_url.pop();
                                        if base_url.is_empty() {
                                            credential.base_url = None;
                                        }
                                    }
                                }
                                AiCredentialEditorField::ApiKey => {}
                            }
                        }
                    }
                }
                self.ai_status = "AI credential edited".to_string();
                cx.notify();
            }
            "tab" => {
                self.ai_credential_edit = Some((credential_id, field.next(builtin)));
                cx.notify();
            }
            "enter" => {
                self.persist_ai_credential_edits(&credential_id, cx);
            }
            "escape" => {
                self.ai_credential_edit = None;
                self.ai_status = "AI credential input blurred".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    match field {
                        AiCredentialEditorField::ApiKey => {
                            self.ai_credential_secret_drafts
                                .entry(credential_id)
                                .or_default()
                                .push_str(input);
                        }
                        AiCredentialEditorField::Name | AiCredentialEditorField::BaseUrl => {
                            if let Some(credential) = self
                                .ai_settings
                                .provider_credentials
                                .iter_mut()
                                .find(|credential| credential.id == credential_id)
                            {
                                match field {
                                    AiCredentialEditorField::Name => {
                                        credential.name.push_str(input);
                                    }
                                    AiCredentialEditorField::BaseUrl => {
                                        let base =
                                            credential.base_url.get_or_insert_with(String::new);
                                        base.push_str(input);
                                    }
                                    AiCredentialEditorField::ApiKey => {}
                                }
                            }
                        }
                    }
                    self.ai_status = "AI credential edited".to_string();
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn persist_ai_credential_edits(
        &mut self,
        credential_id: &str,
        cx: &mut Context<Self>,
    ) {
        let secret_draft = self
            .ai_credential_secret_drafts
            .get(credential_id)
            .cloned()
            .unwrap_or_default();
        if let Some(credential) = self
            .ai_settings
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
                .ai_settings
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
        self.ai_credential_secret_drafts.remove(credential_id);
        self.ai_status = "AI credential saved".to_string();
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
        self.ai_settings.provider_credentials.insert(0, credential);
        self.ai_credential_edit = Some((id, AiCredentialEditorField::Name));
        window.focus(&self.ai_credential_focus);
        self.ai_status = "AI credential added".to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn remove_ai_credential(
        &mut self,
        credential_id: String,
        cx: &mut Context<Self>,
    ) {
        if is_builtin_ai_provider_id(&credential_id) {
            self.ai_status = "Built-in AI credentials cannot be deleted".to_string();
            cx.notify();
            return;
        }
        self.ai_settings
            .provider_credentials
            .retain(|credential| credential.id != credential_id);
        self.ai_settings
            .models
            .retain(|model| model.credential_id.as_deref() != Some(credential_id.as_str()));
        if self
            .ai_settings
            .default_model_id
            .as_ref()
            .is_some_and(|id| {
                !self
                    .ai_settings
                    .models
                    .iter()
                    .any(|model| model.id == *id && model.enabled)
            })
        {
            self.ai_settings.default_model_id = self
                .ai_settings
                .models
                .iter()
                .find(|model| model.enabled)
                .map(|model| model.id.clone());
        }
        self.ai_credential_edit = None;
        self.ai_credential_secret_drafts.remove(&credential_id);
        self.ai_status = "AI credential removed".to_string();
        self.persist_ai_settings_now(cx);
    }
}

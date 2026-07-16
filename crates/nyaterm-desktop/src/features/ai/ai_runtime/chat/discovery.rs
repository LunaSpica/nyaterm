use super::*;

const AI_DISCOVERY_EVENT_DRAIN_LIMIT: usize = 8;

impl NyaTermApp {
    pub(in crate::features) fn discover_ai_models(&mut self, cx: &mut Context<Self>) {
        if self.ai_discovery_pending {
            self.ai_status = "AI model discovery already running".to_string();
            cx.notify();
            return;
        }

        let credential = match self.active_ai_discovery_credential() {
            Ok(credential) => credential,
            Err(error) => {
                self.ai_status = error;
                cx.notify();
                return;
            }
        };
        if credential
            .base_url
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            self.ai_status = "AI model discovery requires a Base URL".to_string();
            cx.notify();
            return;
        }

        let settings = self.ai_settings.clone();
        let profile_id = credential.id.clone();
        let tx = self.ai_discovery_tx.clone();
        self.ai_discovery_pending = true;
        self.ai_status = "Discovering AI models...".to_string();
        std::thread::spawn(move || {
            let result = discover_openai_compatible_models(&settings, &credential);
            let _ = tx.send(AiDiscoveryJobResult { profile_id, result });
        });
        cx.notify();
    }

    pub(in crate::features) fn active_ai_discovery_credential(
        &self,
    ) -> Result<AiProviderCredential, String> {
        let active_id = self.ai_settings.active_profile_id.as_str();
        let profile = self
            .ai_settings
            .provider_profiles
            .iter()
            .find(|profile| profile.id == active_id)
            .ok_or_else(|| format!("AI provider profile '{active_id}' is not configured"))?;
        let current_credential = self
            .ai_settings
            .provider_credentials
            .iter()
            .find(|credential| credential.id == active_id);

        Ok(AiProviderCredential {
            id: profile.id.clone(),
            name: profile.name.clone(),
            provider_kind: profile.provider_kind.clone(),
            base_url: none_if_blank(&self.ai_base_url_draft),
            api_key: if self.ai_secret_draft.is_empty() {
                current_credential
                    .and_then(|credential| credential.api_key.clone())
                    .or_else(|| profile.api_key.clone())
            } else {
                Some(self.ai_secret_draft.clone())
            },
            enabled: true,
        })
    }

    pub(in crate::features) fn drain_ai_discovery_events(&mut self) -> bool {
        if !self.ai_discovery_pending {
            return false;
        }
        let mut dirty = false;
        for _ in 0..AI_DISCOVERY_EVENT_DRAIN_LIMIT {
            let Ok(event) = self.ai_discovery_rx.try_recv() else {
                break;
            };
            dirty = true;
            self.ai_discovery_pending = false;
            match event.result {
                Ok(discoveries) if discoveries.is_empty() => {
                    self.ai_status = "AI discovery returned no models".to_string();
                }
                Ok(discoveries) => {
                    let count = self.apply_ai_model_discoveries(&event.profile_id, discoveries);
                    self.ai_status = format!("Discovered {count} AI model(s); save to persist");
                    self.store_status.message = self.ai_status.clone();
                    self.store_status.ready = true;
                }
                Err(error) => {
                    self.ai_status = format!("AI model discovery failed: {error}");
                    self.store_status.message = self.ai_status.clone();
                    self.store_status.ready = false;
                }
            }
        }
        dirty
    }

    pub(in crate::features) fn apply_ai_model_discoveries(
        &mut self,
        profile_id: &str,
        discoveries: Vec<AiModelDiscovery>,
    ) -> usize {
        let discoveries = merge_model_discoveries(discoveries);
        let first_discovery = discoveries.first().cloned();
        let discovered_ids: HashSet<String> =
            discoveries.iter().map(|model| model.id.clone()).collect();
        let last_seen_at = Some(now_rfc3339());

        for discovery in &discoveries {
            if let Some(model) = self
                .ai_settings
                .models
                .iter_mut()
                .find(|model| model.id == discovery.id)
            {
                model.name = discovery.name.clone();
                model.provider_kind = discovery.provider_kind.clone();
                model.credential_id = discovery.credential_id.clone();
                model.enabled = true;
                model.source = discovery.source.clone();
                model.last_seen_at = last_seen_at.clone();
            } else {
                self.ai_settings
                    .models
                    .push(nyaterm_core::AiModelConfigItem {
                        id: discovery.id.clone(),
                        name: discovery.name.clone(),
                        provider_kind: discovery.provider_kind.clone(),
                        credential_id: discovery.credential_id.clone(),
                        enabled: true,
                        source: discovery.source.clone(),
                        last_seen_at: last_seen_at.clone(),
                    });
            }
        }

        if self.ai_settings.active_profile_id == profile_id {
            let draft_model_id = ai_model_id_for_credential(profile_id, self.ai_model_draft.trim());
            if discovered_ids.contains(&draft_model_id) {
                self.ai_settings.default_model_id = Some(draft_model_id);
            } else {
                let current_default_is_valid = self
                    .ai_settings
                    .default_model_id
                    .as_deref()
                    .is_some_and(|id| {
                        self.ai_settings
                            .models
                            .iter()
                            .any(|model| model.enabled && model.id == id)
                    });
                if !current_default_is_valid && let Some(first_discovery) = first_discovery.as_ref()
                {
                    self.ai_settings.default_model_id = Some(first_discovery.id.clone());
                }
                if self.ai_model_draft.trim().is_empty()
                    && let Some(first_discovery) = first_discovery.as_ref()
                {
                    self.ai_model_draft = first_discovery.name.clone();
                }
            }
        }

        discoveries.len()
    }
}

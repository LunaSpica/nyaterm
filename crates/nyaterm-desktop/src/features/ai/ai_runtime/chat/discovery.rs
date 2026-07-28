use gpui::Context;

use crate::http::ai::discover_openai_compatible_models;
use nyaterm_core::{AiModelDiscovery, AiProviderKind, merge_model_discoveries, now_rfc3339};

use crate::features::{AiDiscoveryJobResult, NyaTermApp};

const AI_DISCOVERY_EVENT_DRAIN_LIMIT: usize = 8;

impl NyaTermApp {
    pub(in crate::features) fn discover_ai_models(&mut self, cx: &mut Context<Self>) {
        if self.ai.discovery.pending {
            self.ai.panel.status = "AI model discovery already running".to_string();
            cx.notify();
            return;
        }

        let credentials: Vec<_> = self
            .ai
            .settings
            .config
            .provider_credentials
            .iter()
            .filter(|credential| {
                credential.enabled
                    && credential.provider_kind == AiProviderKind::OpenaiCompatible
                    && credential
                        .base_url
                        .as_deref()
                        .is_some_and(|value| !value.trim().is_empty())
            })
            .cloned()
            .collect();
        if credentials.is_empty() {
            self.ai.panel.status =
                "AI model discovery requires an enabled custom provider".to_string();
            cx.notify();
            return;
        }

        let settings = self.ai.settings.config.clone();
        let tx = self.ai.discovery.tx.clone();
        self.ai.discovery.pending = true;
        self.ai.panel.status = "Discovering AI models...".to_string();
        std::thread::spawn(move || {
            let mut discoveries = Vec::new();
            let mut errors = Vec::new();
            for credential in credentials {
                match discover_openai_compatible_models(&settings, &credential) {
                    Ok(models) => discoveries.extend(models),
                    Err(error) => errors.push(format!("{}: {error}", credential.name)),
                }
            }
            let result = if discoveries.is_empty() && !errors.is_empty() {
                Err(errors.join("; "))
            } else {
                Ok(discoveries)
            };
            let _ = tx.send(AiDiscoveryJobResult {
                profile_id: String::new(),
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn drain_ai_discovery_events(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.ai.discovery.pending {
            return false;
        }
        let mut dirty = false;
        for _ in 0..AI_DISCOVERY_EVENT_DRAIN_LIMIT {
            let Ok(event) = self.ai.discovery.rx.try_recv() else {
                break;
            };
            dirty = true;
            self.ai.discovery.pending = false;
            match event.result {
                Ok(discoveries) if discoveries.is_empty() => {
                    self.ai.panel.status = "AI discovery returned no models".to_string();
                }
                Ok(discoveries) => {
                    let count = self.apply_ai_model_discoveries(&event.profile_id, discoveries);
                    self.ai.panel.status = format!("Discovered {count} AI model(s)");
                    self.settings.store_status.message = self.ai.panel.status.clone();
                    self.settings.store_status.ready = true;
                    self.persist_ai_settings_now(cx);
                }
                Err(error) => {
                    self.ai.panel.status = format!("AI model discovery failed: {error}");
                    self.settings.store_status.message = self.ai.panel.status.clone();
                    self.settings.store_status.ready = false;
                }
            }
        }
        dirty
    }

    pub(in crate::features) fn apply_ai_model_discoveries(
        &mut self,
        _profile_id: &str,
        discoveries: Vec<AiModelDiscovery>,
    ) -> usize {
        let discoveries = merge_model_discoveries(discoveries);
        let last_seen_at = Some(now_rfc3339());

        for discovery in &discoveries {
            if let Some(model) = self
                .ai
                .settings
                .config
                .models
                .iter_mut()
                .find(|model| model.id == discovery.id)
            {
                model.name = discovery.name.clone();
                model.provider_kind = discovery.provider_kind.clone();
                model.credential_id = discovery.credential_id.clone();
                model.source = discovery.source.clone();
                model.last_seen_at = last_seen_at.clone();
            } else {
                self.ai
                    .settings
                    .config
                    .models
                    .push(nyaterm_core::AiModelConfigItem {
                        id: discovery.id.clone(),
                        name: discovery.name.clone(),
                        provider_kind: discovery.provider_kind.clone(),
                        credential_id: discovery.credential_id.clone(),
                        enabled: false,
                        source: discovery.source.clone(),
                        last_seen_at: last_seen_at.clone(),
                    });
            }
        }

        discoveries.len()
    }
}

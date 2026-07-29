use gpui::Context;

use crate::http::ai::discover_openai_compatible_models;
use nyaterm_core::AiModelDiscovery;

use crate::features::{AiDiscoveryJobResult, NyaTermApp};

const AI_DISCOVERY_EVENT_DRAIN_LIMIT: usize = 8;

impl NyaTermApp {
    pub(in crate::features) fn discover_ai_models(&mut self, cx: &mut Context<Self>) {
        if self.ai.discovery_is_pending() {
            self.ai
                .set_panel_status("AI model discovery already running".to_string());
            cx.notify();
            return;
        }

        let (settings, credentials) = self.ai.discovery_settings();
        if credentials.is_empty() {
            self.ai.set_panel_status(
                "AI model discovery requires an enabled custom provider".to_string(),
            );
            cx.notify();
            return;
        }

        let Some(tx) = self.ai.begin_discovery_job() else {
            cx.notify();
            return;
        };
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
        let events = self
            .ai
            .drain_discovery_events(AI_DISCOVERY_EVENT_DRAIN_LIMIT);
        let dirty = !events.is_empty();
        for event in events {
            match event.result {
                Ok(discoveries) if discoveries.is_empty() => {
                    self.ai
                        .set_panel_status("AI discovery returned no models".to_string());
                }
                Ok(discoveries) => {
                    let count = self.apply_ai_model_discoveries(&event.profile_id, discoveries);
                    self.ai
                        .set_panel_status(format!("Discovered {count} AI model(s)"));
                    self.settings
                        .update_store_status(self.ai.panel_status().to_string(), true);
                    self.persist_ai_settings_now(cx);
                }
                Err(error) => {
                    self.ai
                        .set_panel_status(format!("AI model discovery failed: {error}"));
                    self.settings
                        .update_store_status(self.ai.panel_status().to_string(), false);
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
        self.ai.apply_settings_model_discoveries(discoveries)
    }
}

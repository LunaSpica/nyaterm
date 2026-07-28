use gpui::{Context, KeyDownEvent, Window};

use nyaterm_core::{ai_model_id_for_credential, ai_model_id_for_provider};

use crate::features::{NyaTermApp, TextInputSetup};

impl NyaTermApp {
    pub(in crate::features) fn toggle_ai_model_enabled(
        &mut self,
        model_id: String,
        cx: &mut Context<Self>,
    ) {
        if let Some(model) = self
            .ai
            .settings
            .config
            .models
            .iter_mut()
            .find(|model| model.id == model_id)
        {
            model.enabled = !model.enabled;
            self.ai.panel.status = "AI model list updated".to_string();
        }
        if self
            .ai
            .settings
            .config
            .default_model_id
            .as_deref()
            .is_none_or(|id| {
                !self
                    .ai
                    .settings
                    .config
                    .models
                    .iter()
                    .any(|model| model.enabled && model.id == id)
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
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn apply_ai_settings_model_search(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.ai.settings.model_query = text;
        cx.notify();
    }

    pub(in crate::features) fn set_ai_default_model(
        &mut self,
        model_id: String,
        cx: &mut Context<Self>,
    ) {
        if let Some(model) = self
            .ai
            .settings
            .config
            .models
            .iter_mut()
            .find(|model| model.id == model_id)
        {
            model.enabled = true;
            self.ai.settings.config.default_model_id = Some(model.id.clone());
            self.ai.panel.status = "AI default model updated".to_string();
        }
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn remove_ai_manual_model(
        &mut self,
        model_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(model) = self
            .ai
            .settings
            .config
            .models
            .iter()
            .find(|model| model.id == model_id)
            .cloned()
        else {
            return;
        };
        if model.source != nyaterm_core::AiModelSource::Manual {
            self.ai.panel.status = "Only manual models can be deleted".to_string();
            cx.notify();
            return;
        }
        self.ai
            .settings
            .config
            .models
            .retain(|item| item.id != model_id);
        if self.ai.settings.config.default_model_id.as_deref() == Some(model_id.as_str()) {
            self.ai.settings.config.default_model_id = self
                .ai
                .settings
                .config
                .models
                .iter()
                .find(|item| item.enabled)
                .map(|item| item.id.clone());
        }
        self.ai.panel.status = format!("Deleted manual model {}", model.name);
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn add_ai_manual_model(
        &mut self,
        credential_id: String,
        name: String,
        cx: &mut Context<Self>,
    ) {
        let name = name.trim().to_string();
        if name.is_empty() {
            self.ai.panel.status = "Manual model name is required".to_string();
            cx.notify();
            return;
        }
        let Some(credential) = self
            .ai
            .settings
            .config
            .provider_credentials
            .iter()
            .find(|credential| credential.id == credential_id)
            .cloned()
        else {
            self.ai.panel.status = "Credential not found".to_string();
            cx.notify();
            return;
        };
        let builtin = matches!(
            credential.id.as_str(),
            "openai"
                | "anthropic"
                | "gemini"
                | "deepseek"
                | "groq"
                | "ollama"
                | "xai"
                | "cohere"
                | "mimo"
                | "zai"
        );
        let model_id = if builtin {
            ai_model_id_for_provider(&credential.provider_kind, &name)
        } else {
            ai_model_id_for_credential(&credential.id, &name)
        };
        if let Some(existing) = self
            .ai
            .settings
            .config
            .models
            .iter_mut()
            .find(|model| model.id == model_id)
        {
            existing.enabled = true;
            existing.name = name.clone();
            existing.provider_kind = Some(credential.provider_kind.clone());
            existing.credential_id = (!builtin).then(|| credential.id.clone());
            self.ai.settings.config.default_model_id = Some(model_id);
            self.ai.panel.status = format!("Enabled model {name}");
            self.persist_ai_settings_now(cx);
            return;
        }
        self.ai.settings.config.models.insert(
            0,
            nyaterm_core::AiModelConfigItem {
                id: model_id.clone(),
                name: name.clone(),
                provider_kind: Some(credential.provider_kind.clone()),
                credential_id: (!builtin).then(|| credential.id.clone()),
                enabled: true,
                source: nyaterm_core::AiModelSource::Manual,
                last_seen_at: None,
            },
        );
        if self
            .ai
            .settings
            .config
            .default_model_id
            .as_ref()
            .is_none_or(|id| {
                !self
                    .ai
                    .settings
                    .config
                    .models
                    .iter()
                    .any(|model| model.enabled && &model.id == id)
            })
        {
            self.ai.settings.config.default_model_id = Some(model_id);
        }
        self.ai.panel.status = format!("Added manual model {name}");
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn toggle_ai_model_group(
        &mut self,
        group_key: String,
        cx: &mut Context<Self>,
    ) {
        if self.ai.settings.model_collapsed_groups.contains(&group_key) {
            self.ai.settings.model_collapsed_groups.remove(&group_key);
        } else {
            self.ai.settings.model_collapsed_groups.insert(group_key);
        }
        cx.notify();
    }

    pub(in crate::features) fn handle_ai_manual_model_key_down(
        &mut self,
        group_key: &str,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        self.ai.settings.manual_model_edit_group = Some(group_key.to_string());
        match event.keystroke.key.as_str() {
            "escape" => {
                self.ai.settings.manual_model_edit_group = None;
                window.focus(&self.ai.settings.manual_model_focus);
                cx.notify();
                return;
            }
            "enter" => {
                if let Some(credential_id) = self
                    .ai
                    .settings
                    .config
                    .provider_credentials
                    .iter()
                    .find(|credential| credential.id == group_key)
                    .map(|credential| credential.id.clone())
                {
                    let name = self
                        .ai
                        .settings
                        .manual_model_drafts
                        .get(group_key)
                        .cloned()
                        .unwrap_or_default();
                    self.add_ai_manual_model(credential_id, name, cx);
                    self.clear_ai_manual_model_draft(group_key, cx);
                }
                return;
            }
            _ => {}
        }
    }

    pub(in crate::features) fn apply_ai_manual_model_input(
        &mut self,
        group_key: &str,
        text: String,
        cx: &mut Context<Self>,
    ) {
        if !self
            .ai
            .settings
            .config
            .provider_credentials
            .iter()
            .any(|credential| credential.id == group_key)
        {
            return;
        }
        self.ai
            .settings
            .manual_model_drafts
            .insert(group_key.to_string(), text);
        self.ai.settings.manual_model_edit_group = Some(group_key.to_string());
        cx.notify();
    }

    pub(in crate::features) fn focus_ai_manual_model_input(
        &mut self,
        group_key: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let draft = self
            .ai
            .settings
            .manual_model_drafts
            .get(&group_key)
            .cloned()
            .unwrap_or_default();
        let input = self.text_input(
            format!("ai.settings.manual-model.{group_key}"),
            &draft,
            TextInputSetup::placeholder(self.tr("ai.manualModelPlaceholder")),
            cx,
        );
        self.ai.settings.manual_model_edit_group = Some(group_key);
        window.focus(&input.read(cx).focus_handle());
        cx.notify();
    }

    pub(in crate::features) fn clear_ai_manual_model_draft(
        &mut self,
        group_key: &str,
        cx: &mut Context<Self>,
    ) {
        self.ai
            .settings
            .manual_model_drafts
            .insert(group_key.to_string(), String::new());
        self.reset_text_input(&format!("ai.settings.manual-model.{group_key}"), "", cx);
        cx.notify();
    }
}

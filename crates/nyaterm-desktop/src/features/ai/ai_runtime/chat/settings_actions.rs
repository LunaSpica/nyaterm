use super::*;

impl NyaTermApp {
    pub(in crate::features) fn pending_ai_settings(&self) -> AiSettings {
        let mut next = self.ai_settings.clone();
        let active_id = next.active_profile_id.clone();
        let mut active_kind = None;
        let mut active_name = active_id.clone();
        let mut active_base_url = none_if_blank(&self.ai_base_url_draft);
        let active_model = self.ai_model_draft.trim().to_string();

        if let Some(profile) = next
            .provider_profiles
            .iter_mut()
            .find(|profile| profile.id == active_id)
        {
            profile.enabled = true;
            if !active_model.is_empty() {
                profile.model = active_model.clone();
            }
            profile.base_url = active_base_url.clone();
            if !self.ai_secret_draft.is_empty() {
                profile.api_key = Some(self.ai_secret_draft.clone());
            }
            active_kind = Some(profile.provider_kind.clone());
            active_name = profile.name.clone();
            active_base_url = profile.base_url.clone();
        }

        if let Some(kind) = active_kind.clone() {
            let credential = AiProviderCredential {
                id: active_id.clone(),
                name: active_name,
                provider_kind: kind.clone(),
                base_url: active_base_url.clone(),
                api_key: if self.ai_secret_draft.is_empty() {
                    next.provider_credentials
                        .iter()
                        .find(|credential| credential.id == active_id)
                        .and_then(|credential| credential.api_key.clone())
                } else {
                    Some(self.ai_secret_draft.clone())
                },
                enabled: true,
            };
            if let Some(existing) = next
                .provider_credentials
                .iter_mut()
                .find(|credential| credential.id == active_id)
            {
                *existing = credential;
            } else {
                next.provider_credentials.push(credential);
            }

            if !active_model.is_empty() {
                let model_id = if active_base_url
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                    || kind == AiProviderKind::OpenaiCompatible
                {
                    ai_model_id_for_credential(&active_id, &active_model)
                } else {
                    ai_model_id_for_provider(&kind, &active_model)
                };
                let model_index = next
                    .models
                    .iter()
                    .position(|model| model.credential_id.as_deref() == Some(active_id.as_str()))
                    .or_else(|| next.models.iter().position(|model| model.id == model_id));
                if let Some(model_index) = model_index {
                    let model = &mut next.models[model_index];
                    model.id = model_id.clone();
                    model.name = active_model.clone();
                    model.provider_kind = Some(kind);
                    model.credential_id = active_base_url
                        .as_deref()
                        .is_some_and(|value| !value.trim().is_empty())
                        .then(|| active_id.clone());
                    model.enabled = true;
                } else {
                    next.models.push(nyaterm_core::AiModelConfigItem {
                        id: model_id.clone(),
                        name: active_model,
                        provider_kind: Some(kind),
                        credential_id: active_base_url
                            .as_deref()
                            .is_some_and(|value| !value.trim().is_empty())
                            .then(|| active_id.clone()),
                        enabled: true,
                        source: nyaterm_core::AiModelSource::Manual,
                        last_seen_at: None,
                    });
                }
                next.default_model_id = Some(model_id);
            }
        }
        next
    }

    pub(in crate::features) fn save_ai_settings(&mut self, cx: &mut Context<Self>) {
        let next = self.pending_ai_settings();
        if self.defer_settings_persistence(cx) {
            self.ai_settings = next;
            self.ai_secret_draft.clear();
            self.sync_ai_drafts_from_active_profile();
            self.ai_status = "AI settings staged".to_string();
            return;
        }
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_ai_settings(next))
        {
            Ok(saved) => {
                self.ai_settings = saved;
                self.ai_secret_draft.clear();
                self.sync_ai_drafts_from_active_profile();
                self.refresh_ai_usage_counts();
                self.ai_status = "AI settings saved".to_string();
                self.store_status.message = "AI settings saved".to_string();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.ai_status = format!("AI settings save failed: {error}");
                self.store_status.message = self.ai_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    /// Persist current `ai_settings` without rewriting active profile drafts (Tauri live update).

    pub(in crate::features) fn persist_ai_settings_now(&mut self, cx: &mut Context<Self>) {
        if self.defer_settings_persistence(cx) {
            self.ai_status = "AI settings staged".to_string();
            return;
        }
        let next = self.ai_settings.clone();
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_ai_settings(next))
        {
            Ok(saved) => {
                self.ai_settings = saved;
                self.refresh_ai_usage_counts();
                if self.ai_status.trim().is_empty() {
                    self.ai_status = "AI settings saved".to_string();
                }
                self.store_status.message = self.ai_status.clone();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.ai_status = format!("AI settings save failed: {error}");
                self.store_status.message = self.ai_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn set_ai_request_user_agent(
        &mut self,
        value: String,
        cx: &mut Context<Self>,
    ) {
        self.ai_settings.request_user_agent = value;
        self.ai_status = "AI request user-agent updated".to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn expand_ai_action(
        &mut self,
        kind: AiActionListKind,
        action_id: String,
        cx: &mut Context<Self>,
    ) {
        let key = (kind, action_id);
        if self.ai_action_expanded.as_ref() == Some(&key) {
            self.ai_action_expanded = None;
            self.ai_action_edit = None;
        } else {
            self.ai_action_expanded = Some(key);
        }
        cx.notify();
    }

    pub(in crate::features) fn focus_ai_action_field(
        &mut self,
        kind: AiActionListKind,
        action_id: String,
        field: AiActionEditorField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.ai_action_expanded = Some((kind, action_id.clone()));
        self.ai_action_edit = Some((kind, action_id, field));
        window.focus(&self.ai_action_focus);
        cx.notify();
    }

    pub(in crate::features) fn toggle_ai_action_enabled(
        &mut self,
        kind: AiActionListKind,
        action_id: String,
        cx: &mut Context<Self>,
    ) {
        if let Some(action) = self.ai_action_mut(kind, &action_id) {
            action.enabled = !action.enabled;
            self.ai_status = "AI action toggled".to_string();
            self.persist_ai_settings_now(cx);
        }
    }

    pub(in crate::features) fn add_ai_action(
        &mut self,
        kind: AiActionListKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = format!(
            "ai-action-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );
        let action = nyaterm_core::AiCustomActionConfig {
            id: id.clone(),
            name: "Custom AI action".to_string(),
            prompt: String::new(),
            enabled: true,
        };
        match kind {
            AiActionListKind::Terminal => self.ai_settings.terminal_ai_actions.push(action),
            AiActionListKind::File => self.ai_settings.file_ai_actions.push(action),
        }
        self.ai_action_expanded = Some((kind, id.clone()));
        self.ai_action_edit = Some((kind, id, AiActionEditorField::Name));
        window.focus(&self.ai_action_focus);
        self.ai_status = "AI action added".to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn remove_ai_action(
        &mut self,
        kind: AiActionListKind,
        action_id: String,
        cx: &mut Context<Self>,
    ) {
        match kind {
            AiActionListKind::Terminal => {
                self.ai_settings
                    .terminal_ai_actions
                    .retain(|action| action.id != action_id);
            }
            AiActionListKind::File => {
                self.ai_settings
                    .file_ai_actions
                    .retain(|action| action.id != action_id);
            }
        }
        if self
            .ai_action_expanded
            .as_ref()
            .is_some_and(|(k, id)| *k == kind && id == &action_id)
        {
            self.ai_action_expanded = None;
        }
        if self
            .ai_action_edit
            .as_ref()
            .is_some_and(|(k, id, _)| *k == kind && id == &action_id)
        {
            self.ai_action_edit = None;
        }
        self.ai_status = "AI action removed".to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn handle_ai_action_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        let Some((kind, action_id, field)) = self.ai_action_edit.clone() else {
            return;
        };
        match event.keystroke.key.as_str() {
            "escape" => {
                self.ai_action_edit = None;
                cx.notify();
                return;
            }
            "tab" => {
                self.ai_action_edit = Some((kind, action_id, field.next()));
                cx.notify();
                return;
            }
            "enter" if field == AiActionEditorField::Name => {
                self.ai_action_edit = Some((kind, action_id, AiActionEditorField::Prompt));
                cx.notify();
                return;
            }
            "enter" if field == AiActionEditorField::Prompt => {
                if let Some(action) = self.ai_action_mut(kind, &action_id) {
                    action.prompt.push('\n');
                    self.persist_ai_settings_now(cx);
                }
                return;
            }
            "backspace" => {
                if let Some(action) = self.ai_action_mut(kind, &action_id) {
                    match field {
                        AiActionEditorField::Name => {
                            action.name.pop();
                        }
                        AiActionEditorField::Prompt => {
                            action.prompt.pop();
                        }
                    }
                    self.persist_ai_settings_now(cx);
                }
                return;
            }
            _ => {}
        }
        if let Some(input) = event
            .keystroke
            .key_char
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            if let Some(action) = self.ai_action_mut(kind, &action_id) {
                match field {
                    AiActionEditorField::Name => action.name.push_str(input),
                    AiActionEditorField::Prompt => action.prompt.push_str(input),
                }
                self.persist_ai_settings_now(cx);
            }
        }
    }

    pub(in crate::features) fn ai_action_mut(
        &mut self,
        kind: AiActionListKind,
        action_id: &str,
    ) -> Option<&mut nyaterm_core::AiCustomActionConfig> {
        match kind {
            AiActionListKind::Terminal => self
                .ai_settings
                .terminal_ai_actions
                .iter_mut()
                .find(|action| action.id == action_id),
            AiActionListKind::File => self
                .ai_settings
                .file_ai_actions
                .iter_mut()
                .find(|action| action.id == action_id),
        }
    }
}

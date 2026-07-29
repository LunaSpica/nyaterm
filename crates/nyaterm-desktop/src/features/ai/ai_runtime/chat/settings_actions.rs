use gpui::{Context, KeyDownEvent, Window};

use crate::features::{NyaTermApp, TextInputSetup, none_if_blank};
use crate::models::{AiActionEditorField, AiActionListKind};
use nyaterm_core::{
    AiProviderCredential, AiProviderKind, AiSettings, ConnectionStore, ai_model_id_for_credential,
    ai_model_id_for_provider,
};

impl NyaTermApp {
    pub(in crate::features) fn pending_ai_settings(&self) -> AiSettings {
        let mut next = self.ai.settings.config.clone();
        let active_id = next.active_profile_id.clone();
        let mut active_kind = None;
        let mut active_name = active_id.clone();
        let mut active_base_url = none_if_blank(&self.ai.settings.base_url_draft);
        let active_model = self.ai.settings.model_draft.trim().to_string();

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
            if !self.ai.settings.secret_draft.is_empty() {
                profile.api_key = Some(self.ai.settings.secret_draft.clone());
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
                api_key: if self.ai.settings.secret_draft.is_empty() {
                    next.provider_credentials
                        .iter()
                        .find(|credential| credential.id == active_id)
                        .and_then(|credential| credential.api_key.clone())
                } else {
                    Some(self.ai.settings.secret_draft.clone())
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

    /// Persist current `ai_settings` without rewriting active profile drafts (Tauri live update).

    pub(in crate::features) fn persist_ai_settings_now(&mut self, cx: &mut Context<Self>) {
        if self.defer_settings_persistence(cx) {
            self.ai.panel.status = "AI settings staged".to_string();
            return;
        }
        let next = self.ai.settings.config.clone();
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_ai_settings(next))
        {
            Ok(saved) => {
                self.ai.settings.config = saved;
                self.refresh_ai_usage_counts(cx);
                if self.ai.panel.status.trim().is_empty() {
                    self.ai.panel.status = "AI settings saved".to_string();
                }
                self.settings
                    .set_store_message(self.ai.panel.status.clone());
                self.settings.set_store_ready(true);
            }
            Err(error) => {
                self.ai.panel.status = format!("AI settings save failed: {error}");
                self.settings
                    .set_store_message(self.ai.panel.status.clone());
                self.settings.set_store_ready(false);
            }
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
        let value = self
            .ai_action_mut(kind, &action_id)
            .map(|action| match field {
                AiActionEditorField::Name => action.name.clone(),
                AiActionEditorField::Prompt => action.prompt.clone(),
            })
            .unwrap_or_default();
        let setup = match field {
            AiActionEditorField::Name => TextInputSetup::placeholder(self.tr("ai.actionName")),
            AiActionEditorField::Prompt => TextInputSetup::multi_line(self.tr("ai.actionPrompt")),
        };
        let input_id = Self::ai_action_text_input_id(kind, &action_id, field);
        let input = self.text_input(input_id, &value, setup, cx);
        self.ai.settings.action_edit = Some((kind, action_id, field));
        window.focus(&input.read(cx).focus_handle());
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
            self.ai.panel.status = "AI action toggled".to_string();
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
            AiActionListKind::Terminal => self.ai.settings.config.terminal_ai_actions.push(action),
            AiActionListKind::File => self.ai.settings.config.file_ai_actions.push(action),
        }
        self.ai.settings.action_edit = Some((kind, id.clone(), AiActionEditorField::Name));
        let input_id = Self::ai_action_text_input_id(kind, &id, AiActionEditorField::Name);
        let input = self.text_input(
            input_id,
            "Custom AI action",
            TextInputSetup::placeholder(self.tr("ai.actionName")),
            cx,
        );
        window.focus(&input.read(cx).focus_handle());
        self.ai.panel.status = "AI action added".to_string();
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
                self.ai
                    .settings
                    .config
                    .terminal_ai_actions
                    .retain(|action| action.id != action_id);
            }
            AiActionListKind::File => {
                self.ai
                    .settings
                    .config
                    .file_ai_actions
                    .retain(|action| action.id != action_id);
            }
        }
        if self
            .ai
            .settings
            .action_edit
            .as_ref()
            .is_some_and(|(k, id, _)| *k == kind && id == &action_id)
        {
            self.ai.settings.action_edit = None;
        }
        self.forget_text_inputs(&format!(
            "ai.settings.action.{}.{action_id}.",
            kind.input_key()
        ));
        self.ai.panel.status = "AI action removed".to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn handle_ai_action_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        let Some((kind, action_id, field)) = self.ai.settings.action_edit.clone() else {
            return;
        };
        match event.keystroke.key.as_str() {
            "escape" => {
                self.ai.settings.action_edit = None;
                window.focus(&self.ai.settings.action_focus);
                cx.notify();
                return;
            }
            "tab" => {
                self.focus_ai_action_field(kind, action_id, field.next(), window, cx);
                return;
            }
            "enter" if field == AiActionEditorField::Name => {
                self.focus_ai_action_field(
                    kind,
                    action_id,
                    AiActionEditorField::Prompt,
                    window,
                    cx,
                );
                return;
            }
            _ => {}
        }
    }

    pub(in crate::features) fn apply_ai_action_input(
        &mut self,
        field_id: &str,
        text: String,
        cx: &mut Context<Self>,
    ) {
        let Some((kind, action_id, field)) = parse_ai_action_text_input_id(field_id) else {
            return;
        };
        let Some(action) = self.ai_action_mut(kind, action_id) else {
            return;
        };
        match field {
            AiActionEditorField::Name => action.name = text,
            AiActionEditorField::Prompt => action.prompt = text,
        }
        self.ai.settings.action_edit = Some((kind, action_id.to_string(), field));
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn ai_action_text_input_id(
        kind: AiActionListKind,
        action_id: &str,
        field: AiActionEditorField,
    ) -> String {
        format!(
            "ai.settings.action.{}.{action_id}.{}",
            kind.input_key(),
            field.input_key()
        )
    }

    pub(in crate::features) fn ai_action_mut(
        &mut self,
        kind: AiActionListKind,
        action_id: &str,
    ) -> Option<&mut nyaterm_core::AiCustomActionConfig> {
        match kind {
            AiActionListKind::Terminal => self
                .ai
                .settings
                .config
                .terminal_ai_actions
                .iter_mut()
                .find(|action| action.id == action_id),
            AiActionListKind::File => self
                .ai
                .settings
                .config
                .file_ai_actions
                .iter_mut()
                .find(|action| action.id == action_id),
        }
    }
}

fn parse_ai_action_text_input_id(
    field_id: &str,
) -> Option<(AiActionListKind, &str, AiActionEditorField)> {
    let (kind, rest) = field_id.split_once('.')?;
    let (action_id, field) = rest.rsplit_once('.')?;
    if action_id.is_empty() {
        return None;
    }
    Some((
        AiActionListKind::from_input_key(kind)?,
        action_id,
        AiActionEditorField::from_input_key(field)?,
    ))
}

#[cfg(test)]
mod tests {
    use crate::models::{AiActionEditorField, AiActionListKind};

    use super::parse_ai_action_text_input_id;

    #[test]
    fn parses_ai_action_text_input_id() {
        assert_eq!(
            parse_ai_action_text_input_id("terminal.some-action.name"),
            Some((
                AiActionListKind::Terminal,
                "some-action",
                AiActionEditorField::Name,
            ))
        );
    }

    #[test]
    fn parses_ai_action_id_containing_dots() {
        assert_eq!(
            parse_ai_action_text_input_id("file.some.nested.action.prompt"),
            Some((
                AiActionListKind::File,
                "some.nested.action",
                AiActionEditorField::Prompt,
            ))
        );
    }

    #[test]
    fn rejects_invalid_ai_action_text_input_ids() {
        for field_id in [
            "terminal.action",
            "terminal..name",
            "unknown.action.name",
            "file.action.unknown",
        ] {
            assert_eq!(parse_ai_action_text_input_id(field_id), None, "{field_id}");
        }
    }
}

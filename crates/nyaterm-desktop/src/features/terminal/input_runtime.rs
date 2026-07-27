use super::*;

use crate::models::AiInputField;

impl NyaTermApp {
    /// Apply an edit from one of the AI settings inputs.
    pub(in crate::features) fn apply_ai_input(
        &mut self,
        field: AiInputField,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.ai.panel.focused_field = field;
        *self.ai_input_value_mut() = text;
        self.ai.panel.status = "AI settings edited".to_string();
        // The user-agent is a live setting rather than a draft, so it is saved
        // as it is typed the way it always was.
        if field == AiInputField::RequestUserAgent {
            self.persist_ai_settings_now(cx);
        } else {
            cx.notify();
        }
    }

    pub(in crate::features) fn ai_input_value_mut(&mut self) -> &mut String {
        match self.ai.panel.focused_field {
            AiInputField::Model => &mut self.ai.settings.model_draft,
            AiInputField::BaseUrl => &mut self.ai.settings.base_url_draft,
            AiInputField::ApiKey => &mut self.ai.settings.secret_draft,
            AiInputField::RequestUserAgent => &mut self.ai.settings.config.request_user_agent,
        }
    }
}

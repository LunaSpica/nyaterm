use super::*;

use crate::models::{AiInputField, TransferInputField};

impl NyaTermApp {
    pub(in crate::features) fn handle_transfer_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        let value = match self.transfer.panel.focused_field {
            TransferInputField::Remote => &mut self.transfer.paths.remote,
            TransferInputField::Local => &mut self.transfer.paths.local,
        };
        match keystroke.key.as_str() {
            "backspace" => {
                value.pop();
                cx.notify();
            }
            "escape" => {
                self.terminal.view.status = "transfer input blurred".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    value.push_str(input);
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn handle_ai_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                self.ai_input_value_mut().pop();
                self.ai.panel.status = "AI settings edited".to_string();
                if self.ai.panel.focused_field == AiInputField::RequestUserAgent {
                    self.persist_ai_settings_now(cx);
                } else {
                    cx.notify();
                }
            }
            "escape" => {
                self.ai.panel.status = "AI input blurred".to_string();
                cx.notify();
            }
            "enter" if self.ai.panel.focused_field == AiInputField::RequestUserAgent => {
                self.ai.panel.status = "AI request user-agent updated".to_string();
                self.persist_ai_settings_now(cx);
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.ai_input_value_mut().push_str(input);
                    self.ai.panel.status = "AI settings edited".to_string();
                    if self.ai.panel.focused_field == AiInputField::RequestUserAgent {
                        self.persist_ai_settings_now(cx);
                    } else {
                        cx.notify();
                    }
                }
            }
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

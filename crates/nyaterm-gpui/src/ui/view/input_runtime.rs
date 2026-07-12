use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn handle_transfer_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        let value = match self.transfer_focused_field {
            TransferInputField::Remote => &mut self.transfer_remote_path,
            TransferInputField::Local => &mut self.transfer_local_path,
        };
        match keystroke.key.as_str() {
            "backspace" => {
                value.pop();
                cx.notify();
            }
            "escape" => {
                self.terminal_status = "transfer input blurred".to_string();
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

    pub(in crate::ui::view) fn handle_ai_key_down(
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
                self.ai_status = "AI settings edited".to_string();
                if self.ai_focused_field == AiInputField::RequestUserAgent {
                    self.persist_ai_settings_now(cx);
                } else {
                    cx.notify();
                }
            }
            "escape" => {
                self.ai_status = "AI input blurred".to_string();
                cx.notify();
            }
            "enter" if self.ai_focused_field == AiInputField::RequestUserAgent => {
                self.ai_status = "AI request user-agent updated".to_string();
                self.persist_ai_settings_now(cx);
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.ai_input_value_mut().push_str(input);
                    self.ai_status = "AI settings edited".to_string();
                    if self.ai_focused_field == AiInputField::RequestUserAgent {
                        self.persist_ai_settings_now(cx);
                    } else {
                        cx.notify();
                    }
                }
            }
        }
    }

    pub(in crate::ui::view) fn ai_input_value_mut(&mut self) -> &mut String {
        match self.ai_focused_field {
            AiInputField::Model => &mut self.ai_model_draft,
            AiInputField::BaseUrl => &mut self.ai_base_url_draft,
            AiInputField::ApiKey => &mut self.ai_secret_draft,
            AiInputField::RequestUserAgent => &mut self.ai_settings.request_user_agent,
        }
    }
}

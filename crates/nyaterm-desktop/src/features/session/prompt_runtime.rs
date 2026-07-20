use std::hash::{Hash, Hasher};

use super::*;

impl NyaTermApp {
    pub(in crate::features) fn resolve_host_key_prompt(
        &mut self,
        request_id: String,
        choice: HostKeyPromptChoice,
        cx: &mut Context<Self>,
    ) {
        let Some(request) = self.active_host_key_prompt.take() else {
            self.terminal_status = "no SSH host key prompt is active".to_string();
            cx.notify();
            return;
        };

        if request.id != request_id {
            self.active_host_key_prompt = Some(request);
            self.terminal_status = "SSH host key prompt changed before response".to_string();
            cx.notify();
            return;
        }

        let host = request.host_key.host_identifier.clone();
        let _ = request.response_tx.send(choice);
        self.terminal_status = match choice {
            HostKeyPromptChoice::Accept => format!("accepted SSH host key for {host}"),
            HostKeyPromptChoice::Reject => format!("rejected SSH host key for {host}"),
        };
        cx.notify();
    }

    pub(in crate::features) fn resolve_duplicate_prompt(
        &mut self,
        request_id: String,
        decision: SftpDuplicateDecision,
        cx: &mut Context<Self>,
    ) {
        let Some(prompt) = self.active_duplicate_prompt.take() else {
            self.terminal_status = "no SFTP duplicate prompt is active".to_string();
            cx.notify();
            return;
        };

        if prompt.id != request_id {
            self.active_duplicate_prompt = Some(prompt);
            self.terminal_status = "SFTP duplicate prompt changed before response".to_string();
            cx.notify();
            return;
        }

        let target = prompt.request.target_path.clone();
        let _ = prompt.response_tx.send(decision);
        self.terminal_status = format!(
            "SFTP duplicate decision for {target}: {}",
            duplicate_decision_label(decision)
        );
        cx.notify();
    }

    pub(in crate::features) fn submit_credential_prompt(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.active_credential_prompt.take() else {
            return;
        };
        let host = credential_prompt_target(&state.prompt);
        let _ = state.response_tx.send(Some(state.value));
        self.credential_prompt_focus_pending = false;
        self.terminal_status = format!("submitted SSH credential for {host}");
        cx.notify();
    }

    pub(in crate::features) fn cancel_credential_prompt(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.active_credential_prompt.take() else {
            return;
        };
        let host = credential_prompt_target(&state.prompt);
        let _ = state.response_tx.send(None);
        self.credential_prompt_focus_pending = false;
        self.terminal_status = format!("cancelled SSH credential prompt for {host}");
        cx.notify();
    }

    pub(in crate::features) fn submit_keyboard_interactive_prompt(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.active_keyboard_interactive_prompt.take() else {
            return;
        };
        let target = keyboard_interactive_prompt_target(&state.request);
        let _ = state.response_tx.send(Some(state.responses));
        self.credential_prompt_focus_pending = false;
        self.terminal_status = format!("submitted SSH verification for {target}");
        cx.notify();
    }

    pub(in crate::features) fn cancel_keyboard_interactive_prompt(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.active_keyboard_interactive_prompt.take() else {
            return;
        };
        let target = keyboard_interactive_prompt_target(&state.request);
        let _ = state.response_tx.send(None);
        self.credential_prompt_focus_pending = false;
        self.terminal_status = format!("cancelled SSH verification for {target}");
        cx.notify();
    }

    pub(in crate::features) fn generate_keyboard_interactive_otp_code(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(otp_id) = self
            .active_keyboard_interactive_prompt
            .as_ref()
            .and_then(|state| state.request.otp_id.clone())
        else {
            return;
        };
        let result = self.otp_provider.preview_otp_code(&otp_id);
        let Some(state) = self.active_keyboard_interactive_prompt.as_mut() else {
            return;
        };
        match result {
            Ok(Some(preview)) => {
                apply_keyboard_interactive_otp_preview(state, preview);
                self.terminal_status = "OTP code ready".to_string();
            }
            Ok(None) => {
                state.otp_code = None;
                state.otp_error = Some("OTP entry not found".to_string());
            }
            Err(error) => {
                state.otp_code = None;
                state.otp_time_step = None;
                state.otp_error = Some(error);
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn send_keyboard_interactive_otp_to_input(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.active_keyboard_interactive_prompt.as_mut() else {
            return;
        };
        let Some(code) = state.otp_code.clone() else {
            return;
        };
        if let Some(response) = state.responses.first_mut() {
            *response = code;
            state.focused_index = 0;
            self.terminal_status = "OTP code sent to verification input".to_string();
            cx.notify();
        }
    }

    pub(in crate::features) fn copy_keyboard_interactive_otp_code(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(code) = self
            .active_keyboard_interactive_prompt
            .as_ref()
            .and_then(|state| state.otp_code.clone())
        else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(code));
        self.terminal_status = "OTP code copied".to_string();
        cx.notify();
    }

    pub(in crate::features) fn handle_credential_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let Some(state) = self.active_credential_prompt.as_mut() else {
            return;
        };
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "enter" => {
                self.submit_credential_prompt(cx);
            }
            "escape" => {
                self.cancel_credential_prompt(cx);
            }
            "backspace" => {
                state.value.pop();
                cx.notify();
            }
            _ => {
                if let Some(value) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|value| !value.is_empty())
                {
                    state.value.push_str(value);
                    cx.notify();
                }
            }
        }
    }

    pub(in crate::features) fn handle_keyboard_interactive_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let Some(state) = self.active_keyboard_interactive_prompt.as_mut() else {
            return;
        };
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "enter" => self.submit_keyboard_interactive_prompt(cx),
            "escape" => self.cancel_keyboard_interactive_prompt(cx),
            "tab" => {
                let prompt_count = state.responses.len();
                if prompt_count > 0 {
                    state.focused_index = if keystroke.modifiers.shift {
                        state
                            .focused_index
                            .checked_sub(1)
                            .unwrap_or(prompt_count - 1)
                    } else {
                        (state.focused_index + 1) % prompt_count
                    };
                    cx.notify();
                }
            }
            "backspace" => {
                if let Some(response) = state.responses.get_mut(state.focused_index) {
                    response.pop();
                    cx.notify();
                }
            }
            _ => {
                if let Some(value) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    && let Some(response) = state.responses.get_mut(state.focused_index)
                {
                    response.push_str(value);
                    cx.notify();
                }
            }
        }
    }

    pub(super) fn drain_host_key_prompts(&mut self) -> bool {
        if self.active_host_key_prompt.is_some() || !self.host_key_prompts.has_pending() {
            return false;
        }

        if let Some(request) = self.host_key_prompts.pop_pending() {
            self.terminal_status = format!(
                "SSH host key decision required for {}",
                request.host_key.host_identifier
            );
            self.active_host_key_prompt = Some(request);
            return true;
        }
        false
    }

    pub(super) fn drain_credential_prompts(&mut self) -> bool {
        if self.active_credential_prompt.is_some()
            || self.active_keyboard_interactive_prompt.is_some()
            || !self.credential_prompts.has_pending()
        {
            return false;
        }

        if let Some(request) = self.credential_prompts.pop_pending() {
            match request {
                CredentialPromptRequest::Secret {
                    id,
                    prompt,
                    response_tx,
                } => {
                    self.terminal_status = format!(
                        "SSH credential required for {}",
                        credential_prompt_target(&prompt)
                    );
                    self.active_credential_prompt = Some(CredentialPromptState {
                        id,
                        prompt,
                        response_tx,
                        value: String::new(),
                    });
                }
                CredentialPromptRequest::KeyboardInteractive {
                    id,
                    request,
                    response_tx,
                } => {
                    self.terminal_status = format!(
                        "SSH verification required for {}",
                        keyboard_interactive_prompt_target(&request)
                    );
                    let responses = vec![String::new(); request.prompts.len()];
                    let otp_type = request.otp_id.as_deref().and_then(|otp_id| {
                        self.connection_otp_entries
                            .iter()
                            .find(|entry| entry.id == otp_id)
                            .map(|entry| entry.otp_type.to_ascii_lowercase())
                    });
                    let otp_preview = if otp_type.as_deref() == Some("totp") {
                        request
                            .otp_id
                            .as_deref()
                            .and_then(|otp_id| self.otp_provider.preview_otp_code(otp_id).ok())
                            .flatten()
                    } else {
                        None
                    };
                    let otp_code = otp_preview.as_ref().map(|preview| preview.code.clone());
                    let otp_period = otp_preview
                        .as_ref()
                        .map(|preview| preview.period)
                        .unwrap_or(0);
                    let otp_time_step = otp_preview.as_ref().and_then(|preview| preview.time_step);
                    self.active_keyboard_interactive_prompt =
                        Some(KeyboardInteractivePromptState {
                            id,
                            request,
                            response_tx,
                            responses,
                            focused_index: 0,
                            otp_code,
                            otp_type,
                            otp_period,
                            otp_time_step,
                            otp_error: None,
                        });
                }
            }
            self.credential_prompt_focus_pending = true;
            return true;
        }
        false
    }

    pub(super) fn refresh_keyboard_interactive_totp(&mut self) -> bool {
        let Some(state) = self.active_keyboard_interactive_prompt.as_ref() else {
            return false;
        };
        if state.otp_type.as_deref() != Some("totp") || state.otp_code.is_none() {
            return false;
        }
        let period = state.otp_period.max(1);
        let current_step = unix_seconds_now() / period;
        if state.otp_time_step == Some(current_step) {
            return false;
        }
        let Some(otp_id) = state.request.otp_id.clone() else {
            return false;
        };
        let result = self.otp_provider.preview_otp_code(&otp_id);
        let Some(state) = self.active_keyboard_interactive_prompt.as_mut() else {
            return false;
        };
        match result {
            Ok(Some(preview)) => apply_keyboard_interactive_otp_preview(state, preview),
            Ok(None) => {
                state.otp_code = None;
                state.otp_time_step = None;
                state.otp_error = Some("OTP entry not found".to_string());
            }
            Err(error) => {
                state.otp_code = None;
                state.otp_time_step = None;
                state.otp_error = Some(error);
            }
        }
        true
    }

    pub(super) fn drain_duplicate_prompts(&mut self) -> bool {
        if self.active_duplicate_prompt.is_some() || !self.duplicate_prompts.has_pending() {
            return false;
        }

        if let Some(request) = self.duplicate_prompts.pop_pending() {
            self.terminal_status = format!(
                "SFTP duplicate decision required for {}",
                request.request.target_path
            );
            self.active_duplicate_prompt = Some(SftpDuplicatePromptState {
                id: request.id,
                request: request.request,
                response_tx: request.response_tx,
            });
            return true;
        }
        false
    }
}

fn apply_keyboard_interactive_otp_preview(
    state: &mut KeyboardInteractivePromptState,
    preview: NativeOtpCodePreview,
) {
    state.otp_code = Some(preview.code);
    state.otp_type = Some(preview.otp_type);
    state.otp_period = preview.period;
    state.otp_time_step = preview.time_step;
    state.otp_error = None;
}

pub(in crate::features) fn uuid_like_prompt_id(host_key: &SshHostKey) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    host_key.host_identifier.hash(&mut hasher);
    host_key.key_type.hash(&mut hasher);
    host_key.key_base64.hash(&mut hasher);
    format!("hk-{:016x}", hasher.finish())
}

pub(in crate::features) fn credential_prompt_id(prompt: &SshCredentialPrompt) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    prompt.connection_name.hash(&mut hasher);
    prompt.host.hash(&mut hasher);
    prompt.port.hash(&mut hasher);
    prompt.username.hash(&mut hasher);
    prompt.kind.hash(&mut hasher);
    prompt.reason.hash(&mut hasher);
    prompt.attempt.hash(&mut hasher);
    prompt.prompt_text.hash(&mut hasher);
    prompt.echo.hash(&mut hasher);
    format!("cred-{:016x}", hasher.finish())
}

pub(in crate::features) fn keyboard_interactive_prompt_id(
    request: &SshKeyboardInteractiveRequest,
) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    request.hash(&mut hasher);
    format!("keyboard-interactive-{:016x}", hasher.finish())
}

pub(in crate::features) fn sftp_duplicate_prompt_id(request: &SftpDuplicateRequest) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    request.direction.hash(&mut hasher);
    request.source_path.hash(&mut hasher);
    request.target_path.hash(&mut hasher);
    request.is_directory.hash(&mut hasher);
    format!("sftp-dup-{:016x}", hasher.finish())
}

pub(in crate::features) fn credential_prompt_target(prompt: &SshCredentialPrompt) -> String {
    format!(
        "{}@{}:{} (attempt {})",
        prompt.username, prompt.host, prompt.port, prompt.attempt
    )
}

pub(in crate::features) fn keyboard_interactive_prompt_target(
    request: &SshKeyboardInteractiveRequest,
) -> String {
    if request.connection_name.trim().is_empty() {
        format!("{}@{}:{}", request.username, request.host, request.port)
    } else {
        request.connection_name.clone()
    }
}

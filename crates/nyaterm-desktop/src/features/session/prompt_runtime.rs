use std::hash::{Hash, Hasher};

use gpui::{ClipboardItem, Context, KeyDownEvent, Window};
use nyaterm_transport::{
    SftpDuplicateDecision, SftpDuplicateRequest, SshCredentialPrompt, SshHostKey,
    SshKeyboardInteractiveRequest,
};

use super::{
    CredentialPromptRequest, CredentialPromptState, HostKeyPromptChoice,
    KeyboardInteractivePromptState, NativeOtpCodePreview, SftpDuplicatePromptState,
    unix_seconds_now,
};
use crate::features::{NyaTermApp, TextInputSetup, duplicate_decision_label};

impl NyaTermApp {
    pub(in crate::features) fn resolve_host_key_prompt(
        &mut self,
        request_id: String,
        choice: HostKeyPromptChoice,
        cx: &mut Context<Self>,
    ) {
        let Some(request) = self.session.prompts.active_host_key_prompt.take() else {
            self.terminal.view.status = "no SSH host key prompt is active".to_string();
            cx.notify();
            return;
        };

        if request.id != request_id {
            self.session.prompts.active_host_key_prompt = Some(request);
            self.terminal.view.status = "SSH host key prompt changed before response".to_string();
            cx.notify();
            return;
        }

        let host = request.host_key.host_identifier.clone();
        let _ = request.response_tx.send(choice);
        self.terminal.view.status = match choice {
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
        let Some(prompt) = self.session.prompts.active_duplicate_prompt.take() else {
            self.terminal.view.status = "no SFTP duplicate prompt is active".to_string();
            cx.notify();
            return;
        };

        if prompt.id != request_id {
            self.session.prompts.active_duplicate_prompt = Some(prompt);
            self.terminal.view.status = "SFTP duplicate prompt changed before response".to_string();
            cx.notify();
            return;
        }

        let target = prompt.request.target_path.clone();
        let _ = prompt.response_tx.send(decision);
        self.terminal.view.status = format!(
            "SFTP duplicate decision for {target}: {}",
            duplicate_decision_label(decision)
        );
        cx.notify();
    }

    pub(in crate::features) fn submit_credential_prompt(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.session.prompts.active_credential_prompt.take() else {
            return;
        };
        let host = credential_prompt_target(&state.prompt);
        let _ = state.response_tx.send(Some(state.value));
        self.forget_text_inputs("ssh.credential.");
        self.session.prompts.credential_prompt_focus_pending = false;
        self.terminal.view.status = format!("submitted SSH credential for {host}");
        cx.notify();
    }

    pub(in crate::features) fn cancel_credential_prompt(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.session.prompts.active_credential_prompt.take() else {
            return;
        };
        let host = credential_prompt_target(&state.prompt);
        let _ = state.response_tx.send(None);
        self.forget_text_inputs("ssh.credential.");
        self.session.prompts.credential_prompt_focus_pending = false;
        self.terminal.view.status = format!("cancelled SSH credential prompt for {host}");
        cx.notify();
    }

    pub(in crate::features) fn submit_keyboard_interactive_prompt(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self
            .session
            .prompts
            .active_keyboard_interactive_prompt
            .take()
        else {
            return;
        };
        let target = keyboard_interactive_prompt_target(&state.request);
        let _ = state.response_tx.send(Some(state.responses));
        self.forget_text_inputs("ssh.keyboard-interactive.");
        self.session.prompts.credential_prompt_focus_pending = false;
        self.terminal.view.status = format!("submitted SSH verification for {target}");
        cx.notify();
    }

    pub(in crate::features) fn cancel_keyboard_interactive_prompt(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self
            .session
            .prompts
            .active_keyboard_interactive_prompt
            .take()
        else {
            return;
        };
        let target = keyboard_interactive_prompt_target(&state.request);
        let _ = state.response_tx.send(None);
        self.forget_text_inputs("ssh.keyboard-interactive.");
        self.session.prompts.credential_prompt_focus_pending = false;
        self.terminal.view.status = format!("cancelled SSH verification for {target}");
        cx.notify();
    }

    pub(in crate::features) fn generate_keyboard_interactive_otp_code(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(otp_id) = self
            .session
            .prompts
            .active_keyboard_interactive_prompt
            .as_ref()
            .and_then(|state| state.request.otp_id.clone())
        else {
            return;
        };
        let result = self.session.prompts.otp_provider.preview_otp_code(&otp_id);
        let Some(state) = self
            .session
            .prompts
            .active_keyboard_interactive_prompt
            .as_mut()
        else {
            return;
        };
        match result {
            Ok(Some(preview)) => {
                apply_keyboard_interactive_otp_preview(state, preview);
                self.terminal.view.status = "OTP code ready".to_string();
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
        let Some(state) = self
            .session
            .prompts
            .active_keyboard_interactive_prompt
            .as_mut()
        else {
            return;
        };
        let Some(code) = state.otp_code.clone() else {
            return;
        };
        if let Some(response) = state.responses.first_mut() {
            *response = code;
            state.focused_index = 0;
            let input_id = keyboard_interactive_text_input_id(&state.id, 0);
            let response = response.clone();
            self.reset_text_input(&input_id, &response, cx);
            self.terminal.view.status = "OTP code sent to verification input".to_string();
            cx.notify();
        }
    }

    pub(in crate::features) fn copy_keyboard_interactive_otp_code(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(code) = self
            .session
            .prompts
            .active_keyboard_interactive_prompt
            .as_ref()
            .and_then(|state| state.otp_code.clone())
        else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(code));
        self.terminal.view.status = "OTP code copied".to_string();
        cx.notify();
    }

    pub(in crate::features) fn handle_credential_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        if self.session.prompts.active_credential_prompt.is_none() {
            return;
        }
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
            _ => {}
        }
    }

    pub(in crate::features) fn handle_keyboard_interactive_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        if self
            .session
            .prompts
            .active_keyboard_interactive_prompt
            .is_none()
        {
            return;
        }
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "enter" => self.submit_keyboard_interactive_prompt(cx),
            "escape" => self.cancel_keyboard_interactive_prompt(cx),
            "tab" => {
                let Some((input_id, seed, setup)) = self
                    .session
                    .prompts
                    .active_keyboard_interactive_prompt
                    .as_mut()
                    .and_then(|state| {
                        let prompt_count = state.responses.len();
                        if prompt_count == 0 {
                            return None;
                        }
                        state.focused_index = if keystroke.modifiers.shift {
                            state
                                .focused_index
                                .checked_sub(1)
                                .unwrap_or(prompt_count - 1)
                        } else {
                            (state.focused_index + 1) % prompt_count
                        };
                        let index = state.focused_index;
                        let setup = if state.request.prompts[index].echo {
                            TextInputSetup::default()
                        } else {
                            TextInputSetup::masked()
                        };
                        Some((
                            keyboard_interactive_text_input_id(&state.id, index),
                            state.responses[index].clone(),
                            setup,
                        ))
                    })
                else {
                    return;
                };
                let field = self.text_input(input_id, &seed, setup, cx);
                window.focus(&field.read(cx).focus_handle());
                cx.notify();
            }
            _ => {}
        }
    }

    pub(in crate::features) fn apply_ssh_credential_input(
        &mut self,
        prompt_id: &str,
        text: String,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.session.prompts.active_credential_prompt.as_mut() else {
            return;
        };
        if state.id != prompt_id {
            return;
        }
        state.value = text;
        self.mark_user_activity();
        cx.notify();
    }

    pub(in crate::features) fn apply_keyboard_interactive_input(
        &mut self,
        field_id: &str,
        text: String,
        cx: &mut Context<Self>,
    ) {
        let Some((prompt_id, index)) = parse_keyboard_interactive_text_input_id(field_id) else {
            return;
        };
        let Some(state) = self
            .session
            .prompts
            .active_keyboard_interactive_prompt
            .as_mut()
        else {
            return;
        };
        if state.id != prompt_id {
            return;
        }
        let Some(response) = state.responses.get_mut(index) else {
            return;
        };
        *response = text;
        state.focused_index = index;
        self.mark_user_activity();
        cx.notify();
    }

    pub(in crate::features) fn focus_active_ssh_prompt_input(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if let Some(state) = self.session.prompts.active_credential_prompt.as_ref() {
            let input_id = credential_text_input_id(&state.id);
            let seed = state.value.clone();
            let setup = if state.prompt.echo {
                TextInputSetup::default()
            } else {
                TextInputSetup::masked()
            };
            let field = self.text_input(input_id, &seed, setup, cx);
            window.focus(&field.read(cx).focus_handle());
            return true;
        }

        let Some(state) = self
            .session
            .prompts
            .active_keyboard_interactive_prompt
            .as_ref()
        else {
            return false;
        };
        let Some(index) = (!state.responses.is_empty())
            .then_some(state.focused_index.min(state.responses.len() - 1))
        else {
            return false;
        };
        let input_id = keyboard_interactive_text_input_id(&state.id, index);
        let seed = state.responses[index].clone();
        let setup = if state.request.prompts[index].echo {
            TextInputSetup::default()
        } else {
            TextInputSetup::masked()
        };
        let field = self.text_input(input_id, &seed, setup, cx);
        window.focus(&field.read(cx).focus_handle());
        true
    }

    pub(in crate::features) fn drain_host_key_prompts(&mut self) -> bool {
        if self.session.prompts.active_host_key_prompt.is_some()
            || !self.session.prompts.host_key_prompts.has_pending()
        {
            return false;
        }

        if let Some(request) = self.session.prompts.host_key_prompts.pop_pending() {
            self.terminal.view.status = format!(
                "SSH host key decision required for {}",
                request.host_key.host_identifier
            );
            self.session.prompts.active_host_key_prompt = Some(request);
            return true;
        }
        false
    }

    pub(in crate::features) fn drain_credential_prompts(&mut self) -> bool {
        if self.session.prompts.active_credential_prompt.is_some()
            || self
                .session
                .prompts
                .active_keyboard_interactive_prompt
                .is_some()
            || !self.session.prompts.credential_prompts.has_pending()
        {
            return false;
        }

        if let Some(request) = self.session.prompts.credential_prompts.pop_pending() {
            match request {
                CredentialPromptRequest::Secret {
                    id,
                    prompt,
                    response_tx,
                } => {
                    self.forget_text_inputs("ssh.credential.");
                    self.terminal.view.status = format!(
                        "SSH credential required for {}",
                        credential_prompt_target(&prompt)
                    );
                    self.session.prompts.active_credential_prompt = Some(CredentialPromptState {
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
                    self.forget_text_inputs("ssh.keyboard-interactive.");
                    self.terminal.view.status = format!(
                        "SSH verification required for {}",
                        keyboard_interactive_prompt_target(&request)
                    );
                    let responses = vec![String::new(); request.prompts.len()];
                    let otp_type = request.otp_id.as_deref().and_then(|otp_id| {
                        self.security
                            .catalog
                            .otp_entries
                            .iter()
                            .find(|entry| entry.id == otp_id)
                            .map(|entry| entry.otp_type.to_ascii_lowercase())
                    });
                    let otp_preview = if otp_type.as_deref() == Some("totp") {
                        request
                            .otp_id
                            .as_deref()
                            .and_then(|otp_id| {
                                self.session
                                    .prompts
                                    .otp_provider
                                    .preview_otp_code(otp_id)
                                    .ok()
                            })
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
                    self.session.prompts.active_keyboard_interactive_prompt =
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
            self.session.prompts.credential_prompt_focus_pending = true;
            return true;
        }
        false
    }

    pub(in crate::features) fn refresh_keyboard_interactive_totp(&mut self) -> bool {
        let Some(state) = self
            .session
            .prompts
            .active_keyboard_interactive_prompt
            .as_ref()
        else {
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
        let result = self.session.prompts.otp_provider.preview_otp_code(&otp_id);
        let Some(state) = self
            .session
            .prompts
            .active_keyboard_interactive_prompt
            .as_mut()
        else {
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

    pub(in crate::features) fn drain_duplicate_prompts(&mut self) -> bool {
        if self.session.prompts.active_duplicate_prompt.is_some()
            || !self.session.prompts.duplicate_prompts.has_pending()
        {
            return false;
        }

        if let Some(request) = self.session.prompts.duplicate_prompts.pop_pending() {
            self.terminal.view.status = format!(
                "SFTP duplicate decision required for {}",
                request.request.target_path
            );
            self.session.prompts.active_duplicate_prompt = Some(SftpDuplicatePromptState {
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

pub(in crate::features) fn credential_text_input_id(prompt_id: &str) -> String {
    format!("ssh.credential.{prompt_id}")
}

pub(in crate::features) fn keyboard_interactive_text_input_id(
    prompt_id: &str,
    index: usize,
) -> String {
    format!("ssh.keyboard-interactive.{prompt_id}.{index}")
}

fn parse_keyboard_interactive_text_input_id(field_id: &str) -> Option<(&str, usize)> {
    let (prompt_id, index) = field_id.rsplit_once('.')?;
    Some((prompt_id, index.parse().ok()?))
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

#[cfg(test)]
mod text_input_id_tests {
    use super::{keyboard_interactive_text_input_id, parse_keyboard_interactive_text_input_id};

    #[test]
    fn keyboard_interactive_input_ids_keep_prompt_and_index_separate() {
        let id = keyboard_interactive_text_input_id("keyboard-interactive.abc", 3);

        assert_eq!(
            parse_keyboard_interactive_text_input_id(
                id.strip_prefix("ssh.keyboard-interactive.").unwrap()
            ),
            Some(("keyboard-interactive.abc", 3))
        );
    }

    #[test]
    fn keyboard_interactive_input_ids_reject_missing_or_invalid_indexes() {
        assert_eq!(parse_keyboard_interactive_text_input_id("prompt"), None);
        assert_eq!(
            parse_keyboard_interactive_text_input_id("prompt.not-a-number"),
            None
        );
    }
}

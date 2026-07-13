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
        self.terminal_status = format!("submitted SSH credential for {host}");
        cx.notify();
    }

    pub(in crate::features) fn cancel_credential_prompt(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.active_credential_prompt.take() else {
            return;
        };
        let host = credential_prompt_target(&state.prompt);
        let _ = state.response_tx.send(None);
        self.terminal_status = format!("cancelled SSH credential prompt for {host}");
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

    pub(super) fn drain_host_key_prompts(&mut self) -> bool {
        if self.active_host_key_prompt.is_some() {
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
        if self.active_credential_prompt.is_some() {
            return false;
        }

        if let Some(request) = self.credential_prompts.pop_pending() {
            self.terminal_status = format!(
                "SSH credential required for {}",
                credential_prompt_target(&request.prompt)
            );
            self.active_credential_prompt = Some(CredentialPromptState {
                id: request.id,
                prompt: request.prompt,
                response_tx: request.response_tx,
                value: String::new(),
            });
            return true;
        }
        false
    }

    pub(super) fn drain_duplicate_prompts(&mut self) -> bool {
        if self.active_duplicate_prompt.is_some() {
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

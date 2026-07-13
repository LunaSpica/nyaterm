use super::*;
use crate::temporary_ssh_link::{TemporarySshLinkConfig, parse_temporary_ssh_link};

impl NyaTermApp {
    pub(in crate::features) fn open_temporary_ssh_link_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_session_name.is_some() {
            self.terminal_status = "wait for the pending session to finish connecting".to_string();
            cx.notify();
            return;
        }
        self.temporary_ssh_link_open = true;
        self.temporary_ssh_link_error = None;
        self.terminal_status = "temporary SSH link opened".to_string();
        window.focus(&self.temporary_ssh_link_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_temporary_ssh_link_dialog(&mut self, cx: &mut Context<Self>) {
        self.temporary_ssh_link_open = false;
        self.temporary_ssh_link_draft.clear();
        self.temporary_ssh_link_error = None;
        self.terminal_status = "temporary SSH link cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn submit_temporary_ssh_link_dialog(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_session_name.is_some() {
            self.temporary_ssh_link_error =
                Some("A session is already connecting. Try again after it finishes.".to_string());
            self.terminal_status = "wait for the pending session to finish connecting".to_string();
            cx.notify();
            return;
        }

        let parsed = match parse_temporary_ssh_link(&self.temporary_ssh_link_draft) {
            Ok(parsed) => parsed,
            Err(error) => {
                self.temporary_ssh_link_error = Some(error.message().to_string());
                self.terminal_status = "temporary SSH link is invalid".to_string();
                cx.notify();
                return;
            }
        };
        let config = self.temporary_ssh_session_config(parsed.clone());
        self.temporary_ssh_link_open = false;
        self.temporary_ssh_link_draft.clear();
        self.temporary_ssh_link_error = None;
        self.begin_background_ssh_start(
            parsed.name,
            config,
            None,
            AiExecutionProfile::Auto,
            None,
            None,
            None,
            None,
            None,
            None,
            cx,
        );
    }

    pub(in crate::features) fn handle_temporary_ssh_link_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        let primary = keystroke.modifiers.control || keystroke.modifiers.platform;
        if primary && !keystroke.modifiers.alt && matches!(keystroke.key.as_str(), "v" | "V") {
            if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                self.temporary_ssh_link_draft.push_str(text.trim());
                self.temporary_ssh_link_error = None;
                cx.notify();
            }
            return;
        }
        if primary || keystroke.modifiers.alt || keystroke.modifiers.function {
            return;
        }

        match keystroke.key.as_str() {
            "escape" => self.close_temporary_ssh_link_dialog(cx),
            "enter" => self.submit_temporary_ssh_link_dialog(window, cx),
            "backspace" => {
                self.temporary_ssh_link_draft.pop();
                self.temporary_ssh_link_error = None;
                cx.notify();
            }
            "space" => {
                self.temporary_ssh_link_draft.push(' ');
                self.temporary_ssh_link_error = None;
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.temporary_ssh_link_draft.push_str(input);
                    self.temporary_ssh_link_error = None;
                    cx.notify();
                }
            }
        }
    }

    fn temporary_ssh_session_config(&self, parsed: TemporarySshLinkConfig) -> SshSessionConfig {
        SshSessionConfig {
            name: parsed.name,
            host: parsed.host,
            port: parsed.port,
            username: parsed.username,
            password: None,
            key_auth: None,
            otp_id: None,
            auto_fill_otp: false,
            proxy_jump: None,
            proxy: None,
            allow_none_auth: false,
            backspace_mode: "del".to_string(),
            term: "xterm-256color".to_string(),
            x11_forwarding: false,
            x11_display: String::new(),
            cols: 80,
            rows: 24,
            host_key_verifier: Some(Arc::new(NativeHostKeyVerifier {
                config_dir: self.runtime.config_dir().to_path_buf(),
                portable_key_path: self.runtime.portable_key_path().map(ToOwned::to_owned),
                policy: self.settings.host_key_policy.clone(),
                prompt_broker: self.host_key_prompts.clone(),
            })),
            credential_provider: Some(self.credential_prompts.clone()),
            otp_provider: Some(self.otp_provider.clone()),
        }
    }
}

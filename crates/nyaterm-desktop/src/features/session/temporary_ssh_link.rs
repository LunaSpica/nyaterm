use std::sync::Arc;

use gpui::{Context, KeyDownEvent, Window};
use nyaterm_core::AiExecutionProfile;
use nyaterm_transport::SshSessionConfig;

use super::NativeHostKeyVerifier;
use crate::features::{NyaTermApp, TextInputSetup};
use crate::temporary_ssh_link::{TemporarySshLinkConfig, parse_temporary_ssh_link};

impl NyaTermApp {
    pub(in crate::features) fn open_temporary_ssh_link_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.has_pending_session_start() {
            self.terminal.view.status =
                "wait for the pending session to finish connecting".to_string();
            cx.notify();
            return;
        }
        self.session.dialogs.open_temporary_ssh_link();
        self.forget_text_inputs("temporary-ssh.link");
        let draft = self.session.dialogs.temporary_ssh_link_draft().to_string();
        let field = self.text_input(
            "temporary-ssh.link",
            &draft,
            TextInputSetup::placeholder(self.tr("temporarySsh.placeholder")),
            cx,
        );
        self.terminal.view.status = "temporary SSH link opened".to_string();
        window.focus(&field.read(cx).focus_handle());
        cx.notify();
    }

    pub(in crate::features) fn close_temporary_ssh_link_dialog(&mut self, cx: &mut Context<Self>) {
        self.session.dialogs.close_temporary_ssh_link();
        self.forget_text_inputs("temporary-ssh.link");
        self.terminal.view.status = "temporary SSH link cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn submit_temporary_ssh_link_dialog(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.has_pending_session_start() {
            self.session
                .dialogs
                .reject_temporary_ssh_link("temporarySsh.connecting");
            self.terminal.view.status =
                "wait for the pending session to finish connecting".to_string();
            cx.notify();
            return;
        }

        let parsed = match parse_temporary_ssh_link(self.session.dialogs.temporary_ssh_link_draft())
        {
            Ok(parsed) => parsed,
            Err(error) => {
                self.session
                    .dialogs
                    .reject_temporary_ssh_link(error.locale_key());
                self.terminal.view.status = "temporary SSH link is invalid".to_string();
                cx.notify();
                return;
            }
        };
        let config = self.temporary_ssh_session_config(parsed.clone());
        self.session.dialogs.close_temporary_ssh_link();
        self.forget_text_inputs("temporary-ssh.link");
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
        match keystroke.key.as_str() {
            "escape" => self.close_temporary_ssh_link_dialog(cx),
            "enter" => self.submit_temporary_ssh_link_dialog(window, cx),
            _ => {}
        }
    }

    pub(in crate::features) fn apply_temporary_ssh_link(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.session.dialogs.apply_temporary_ssh_link(text);
        cx.notify();
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
            deferred_pty: true,
            keep_alive_interval_secs: self.settings.summary.terminal_keep_alive_interval,
            cols: 80,
            rows: 24,
            pixel_width: 0,
            pixel_height: 0,
            host_key_verifier: Some(Arc::new(NativeHostKeyVerifier {
                config_dir: self.runtime.config_dir().to_path_buf(),
                portable_key_path: self.runtime.portable_key_path().map(ToOwned::to_owned),
                policy: self.settings.summary.host_key_policy.clone(),
                prompt_broker: self.session.prompts.host_key_broker(),
            })),
            credential_provider: Some(self.session.prompts.credential_broker()),
            otp_provider: Some(self.session.prompts.otp_provider()),
        }
    }
}

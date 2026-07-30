use std::sync::Arc;

use gpui::{Context, Window};
use nyaterm_core::AiExecutionProfile;
use nyaterm_transport::SshSessionConfig;

use super::NativeHostKeyVerifier;
use crate::features::{NyaTermApp, SavedConnectionStartOptions};
use crate::temporary_ssh_link::{TemporarySshLinkConfig, parse_temporary_ssh_link};

impl NyaTermApp {
    pub(in crate::features) fn open_temporary_ssh_link_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.session.start_has_pending() {
            self.shell
                .set_status("wait for the pending session to finish connecting".to_string());
            cx.notify();
            return;
        }
        self.session.dialogs.open_temporary_ssh_link();
        self.forget_text_inputs("temporary-ssh.link");
        self.shell
            .set_status("temporary SSH link opened".to_string());
        self.open_form_dialog(
            (
                self.tr("temporarySsh.title").to_string(),
                480.,
                self.tr("temporarySsh.connect").to_string(),
                |app, _, cx| app.temporary_ssh_link_dialog_content(cx),
                |app, window, cx| app.submit_temporary_ssh_link_dialog(window, cx),
                |app, cx| app.close_temporary_ssh_link_dialog(cx),
            ),
            window,
            cx,
        );
        cx.notify();
    }

    pub(in crate::features) fn close_temporary_ssh_link_dialog(&mut self, cx: &mut Context<Self>) {
        self.session.dialogs.close_temporary_ssh_link();
        self.forget_text_inputs("temporary-ssh.link");
        self.shell
            .set_status("temporary SSH link cancelled".to_string());
        cx.notify();
    }

    pub(in crate::features) fn submit_temporary_ssh_link_dialog(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.session.start_has_pending() {
            self.session
                .dialogs
                .reject_temporary_ssh_link("temporarySsh.connecting");
            self.shell
                .set_status("wait for the pending session to finish connecting".to_string());
            cx.notify();
            return false;
        }

        let parsed = match parse_temporary_ssh_link(self.session.dialogs.temporary_ssh_link_draft())
        {
            Ok(parsed) => parsed,
            Err(error) => {
                self.session
                    .dialogs
                    .reject_temporary_ssh_link(error.locale_key());
                self.shell
                    .set_status("temporary SSH link is invalid".to_string());
                cx.notify();
                return false;
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
            SavedConnectionStartOptions::default(),
            cx,
        );
        true
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
            keep_alive_interval_secs: self.settings.summary().terminal_keep_alive_interval,
            cols: 80,
            rows: 24,
            pixel_width: 0,
            pixel_height: 0,
            host_key_verifier: Some(Arc::new(NativeHostKeyVerifier {
                config_dir: self.runtime.config_dir().to_path_buf(),
                portable_key_path: self.runtime.portable_key_path().map(ToOwned::to_owned),
                policy: self.settings.summary().host_key_policy.clone(),
                prompt_broker: self.session.prompts.host_key_broker(),
            })),
            credential_provider: Some(self.session.prompts.credential_broker()),
            otp_provider: Some(self.session.prompts.otp_provider()),
        }
    }
}

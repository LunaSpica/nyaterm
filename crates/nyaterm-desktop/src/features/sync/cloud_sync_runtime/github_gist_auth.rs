use super::*;

const GITHUB_GIST_AUTH_EVENT_DRAIN_LIMIT: usize = 8;

impl NyaTermApp {
    pub(in crate::features) fn start_github_gist_auth(&mut self, cx: &mut Context<Self>) {
        if !self.cloud_sync_form_enabled() || self.github_gist_auth.pending {
            return;
        }
        if let Some(cancel) = self.github_gist_auth_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.github_gist_auth_job_id = self.github_gist_auth_job_id.wrapping_add(1);
        let job_id = self.github_gist_auth_job_id;
        let cancel = Arc::new(AtomicBool::new(false));
        let existing_gist_id = self
            .cloud_sync_settings
            .github_gist
            .gist_id
            .trim()
            .to_string();
        let existing_gist_id = (!existing_gist_id.is_empty()).then_some(existing_gist_id);
        let tx = self.github_gist_auth_tx.clone();
        self.github_gist_auth = GithubGistAuthState {
            pending: true,
            message: Some(self.tr("settings.githubGistWaitingForAuth").to_string()),
            ..Default::default()
        };
        self.github_gist_auth_cancel = Some(cancel.clone());
        self.cloud_sync_status = self.tr("settings.githubGistWaitingForAuth").to_string();
        std::thread::spawn(move || {
            run_github_gist_device_flow(job_id, existing_gist_id, cancel, tx);
        });
        cx.notify();
    }

    pub(in crate::features) fn cancel_github_gist_auth(&mut self, cx: &mut Context<Self>) {
        if let Some(cancel) = self.github_gist_auth_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.github_gist_auth_job_id = self.github_gist_auth_job_id.wrapping_add(1);
        self.github_gist_auth = GithubGistAuthState::default();
        cx.notify();
    }

    pub(in crate::features) fn copy_github_gist_user_code(&mut self, cx: &mut Context<Self>) {
        let Some(code) = self.github_gist_auth.user_code.clone() else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(code));
        self.cloud_sync_status = self.tr("settings.githubGistUserCodeCopied").to_string();
        cx.notify();
    }

    pub(in crate::features) fn open_github_gist_verification_url(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(url) = self.github_gist_auth.verification_uri.clone() else {
            return;
        };
        self.open_external_url_for_ui(&url, cx);
    }

    pub(in crate::features) fn drain_github_gist_auth_events(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut dirty = false;
        for _ in 0..GITHUB_GIST_AUTH_EVENT_DRAIN_LIMIT {
            let Ok(job) = self.github_gist_auth_rx.try_recv() else {
                break;
            };
            if job.job_id != self.github_gist_auth_job_id {
                continue;
            }
            dirty = true;
            match job.event {
                GithubGistAuthEvent::Started {
                    user_code,
                    verification_uri,
                } => {
                    self.github_gist_auth.pending = true;
                    self.github_gist_auth.user_code = Some(user_code);
                    self.github_gist_auth.verification_uri = Some(verification_uri.clone());
                    self.github_gist_auth.message =
                        Some(self.tr("settings.githubGistWaitingForAuth").to_string());
                    self.open_external_url_for_ui(&verification_uri, cx);
                }
                GithubGistAuthEvent::Polling { slow_down } => {
                    self.github_gist_auth.message = Some(
                        self.tr(if slow_down {
                            "settings.githubGistSlowDown"
                        } else {
                            "settings.githubGistWaitingForAuth"
                        })
                        .to_string(),
                    );
                }
                GithubGistAuthEvent::Succeeded {
                    access_token,
                    gist_id,
                    login,
                } => {
                    self.github_gist_auth_cancel = None;
                    self.cloud_sync_secret_draft.github_token = access_token;
                    self.cloud_sync_settings.github_gist.gist_id = gist_id;
                    self.github_gist_auth = GithubGistAuthState {
                        pending: false,
                        login: Some(login),
                        message: Some(self.tr("settings.githubGistConnected").to_string()),
                        ..Default::default()
                    };
                    self.cloud_sync_status = self.tr("settings.githubGistConnected").to_string();
                    self.terminal_status = self.cloud_sync_status.clone();
                }
                GithubGistAuthEvent::Failed(error) => {
                    self.github_gist_auth_cancel = None;
                    let message = if error.contains("OAuth Client ID is not configured") {
                        self.tr("settings.githubGistClientIdMissing").to_string()
                    } else {
                        error
                    };
                    self.github_gist_auth.pending = false;
                    self.github_gist_auth.user_code = None;
                    self.github_gist_auth.verification_uri = None;
                    self.github_gist_auth.message = Some(message.clone());
                    self.cloud_sync_status = message.clone();
                    self.terminal_status = message;
                }
                GithubGistAuthEvent::Cancelled => {
                    self.github_gist_auth_cancel = None;
                    self.github_gist_auth = GithubGistAuthState::default();
                }
            }
        }
        dirty
    }
}

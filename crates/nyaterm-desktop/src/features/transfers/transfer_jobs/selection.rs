use super::*;

impl NyaTermApp {
    pub(in crate::features) fn select_transfer_job(
        &mut self,
        job_id: String,
        cx: &mut Context<Self>,
    ) {
        if self.transfer_jobs.iter().any(|job| job.id == job_id) {
            self.transfer_selected_job_id = Some(job_id.clone());
            self.terminal_status = format!("selected transfer {job_id}");
        } else {
            self.terminal_status = "transfer job not found".to_string();
        }
        cx.notify();
    }

    pub(in crate::features) fn open_transfer_job_menu(
        &mut self,
        job_id: String,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.transfer_queue_focus);
        if self.transfer_jobs.iter().any(|job| job.id == job_id) {
            self.transfer_selected_job_id = Some(job_id.clone());
            self.transfer_job_menu = Some(TransferJobMenuState {
                job_id,
                x: event.position.x,
                y: event.position.y,
            });
            self.terminal_status = "transfer menu opened".to_string();
        } else {
            self.transfer_job_menu = None;
            self.terminal_status = "transfer job not found".to_string();
        }
        cx.notify();
    }

    pub(in crate::features) fn close_transfer_job_menu(&mut self, cx: &mut Context<Self>) {
        self.transfer_job_menu = None;
        cx.notify();
    }

    pub(in crate::features) fn request_delete_transfer_job(
        &mut self,
        job_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(job) = self.transfer_jobs.iter().find(|job| job.id == job_id) else {
            self.terminal_status = "transfer job not found".to_string();
            cx.notify();
            return;
        };
        if !self.can_delete_transfer_job(&job_id) {
            self.terminal_status = format!("transfer {} cannot be deleted yet", job.id);
            cx.notify();
            return;
        }
        self.transfer_selected_job_id = Some(job.id.clone());
        self.transfer_job_delete = Some(TransferJobDeleteState {
            job_id: job.id.clone(),
            title: transfer_job_title(&job.kind),
        });
        cx.notify();
    }

    pub(in crate::features) fn request_delete_selected_transfer_job(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let active_session_id = self.active_session_id.as_deref();
        let job_id = self
            .transfer_selected_job_id
            .clone()
            .filter(|job_id| {
                self.transfer_jobs
                    .iter()
                    .any(|job| job.id == *job_id && job.is_visible_for_session(active_session_id))
            })
            .or_else(|| {
                self.transfer_jobs
                    .iter()
                    .rev()
                    .find(|job| job.is_visible_for_session(active_session_id))
                    .map(|job| job.id.clone())
            });
        let Some(job_id) = job_id else {
            self.terminal_status = "transfer queue is empty".to_string();
            cx.notify();
            return;
        };
        self.request_delete_transfer_job(job_id, cx);
    }

    pub(in crate::features) fn confirm_delete_transfer_job(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.transfer_job_delete.take() else {
            cx.notify();
            return;
        };
        let before = self.transfer_jobs.len();
        self.transfer_jobs.retain(|job| job.id != state.job_id);
        if self.transfer_selected_job_id.as_deref() == Some(state.job_id.as_str()) {
            self.transfer_selected_job_id = None;
        }
        self.terminal_status = if self.transfer_jobs.len() < before {
            format!("deleted transfer {}", state.job_id)
        } else {
            "transfer job not found".to_string()
        };
        cx.notify();
    }

    pub(in crate::features) fn cancel_delete_transfer_job(&mut self, cx: &mut Context<Self>) {
        self.transfer_job_delete = None;
        self.terminal_status = "transfer delete cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn reveal_transfer_job_target_directory(
        &mut self,
        job_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(job) = self.transfer_jobs.iter().find(|job| job.id == job_id) else {
            self.terminal_status = "transfer job not found".to_string();
            cx.notify();
            return;
        };
        let Some(target_path) = transfer_job_local_target_path(job) else {
            self.terminal_status = format!("transfer {} has no local target", job.id);
            cx.notify();
            return;
        };
        let target_dir = transfer_job_reveal_dir(target_path);
        cx.reveal_path(&target_dir);
        self.terminal_status = format!("opened transfer directory {}", target_dir.display());
        cx.notify();
    }

    pub(in crate::features) fn handle_transfer_queue_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        let keystroke = &event.keystroke;
        let unmodified = !keystroke.modifiers.alt
            && !keystroke.modifiers.control
            && !keystroke.modifiers.platform
            && !keystroke.modifiers.shift;
        if unmodified && keystroke.key == "delete" && self.transfer_job_delete.is_none() {
            cx.stop_propagation();
            self.request_delete_selected_transfer_job(cx);
        }
    }

    pub(in crate::features) fn can_delete_transfer_job(&self, job_id: &str) -> bool {
        let active_session_id = self.active_session_id.as_deref();
        self.transfer_jobs
            .iter()
            .find(|job| job.id == job_id)
            .is_some_and(|job| {
                job.is_visible_for_session(active_session_id)
                    && !matches!(
                        job.status,
                        TransferJobStatus::Running
                            | TransferJobStatus::Paused
                            | TransferJobStatus::Cancelling
                    )
            })
    }

    pub(in crate::features) fn next_transfer_id(&self, prefix: &str) -> String {
        format!("{prefix}-{}", self.transfer_jobs.len() + 1)
    }
}

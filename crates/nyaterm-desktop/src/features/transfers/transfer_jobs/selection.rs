use gpui::{Context, KeyDownEvent, MouseDownEvent, Window};

use crate::features::NyaTermApp;
use crate::models::{TransferJobDeleteState, TransferJobMenuState, TransferJobStatus};

use super::super::transfer_widgets::transfer_job_title;
use super::helpers::{transfer_job_local_target_path, transfer_job_reveal_dir};

impl NyaTermApp {
    pub(in crate::features) fn select_transfer_job(
        &mut self,
        job_id: String,
        cx: &mut Context<Self>,
    ) {
        if self.transfer.queue.jobs.iter().any(|job| job.id == job_id) {
            self.transfer.queue.selected_job_id = Some(job_id.clone());
            self.terminal.view.status = format!("selected transfer {job_id}");
        } else {
            self.terminal.view.status = "transfer job not found".to_string();
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
        window.focus(&self.transfer.queue.focus);
        if self.transfer.queue.jobs.iter().any(|job| job.id == job_id) {
            self.transfer.queue.selected_job_id = Some(job_id.clone());
            self.transfer.queue.job_menu = Some(TransferJobMenuState {
                job_id,
                x: event.position.x,
                y: event.position.y,
            });
            self.terminal.view.status = "transfer menu opened".to_string();
        } else {
            self.transfer.queue.job_menu = None;
            self.terminal.view.status = "transfer job not found".to_string();
        }
        cx.notify();
    }

    pub(in crate::features) fn close_transfer_job_menu(&mut self, cx: &mut Context<Self>) {
        self.transfer.queue.close_job_menu();
        cx.notify();
    }

    pub(in crate::features) fn request_delete_transfer_job(
        &mut self,
        job_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(job) = self.transfer.queue.jobs.iter().find(|job| job.id == job_id) else {
            self.terminal.view.status = "transfer job not found".to_string();
            cx.notify();
            return;
        };
        if !self.can_delete_transfer_job(&job_id) {
            self.terminal.view.status = format!("transfer {} cannot be deleted yet", job.id);
            cx.notify();
            return;
        }
        self.transfer.queue.selected_job_id = Some(job.id.clone());
        self.transfer.queue.job_delete = Some(TransferJobDeleteState {
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
        let job_id =
            self.transfer
                .queue
                .selected_job_id
                .clone()
                .filter(|job_id| {
                    self.transfer.queue.jobs.iter().any(|job| {
                        job.id == *job_id && job.is_visible_for_session(active_session_id)
                    })
                })
                .or_else(|| {
                    self.transfer
                        .queue
                        .jobs
                        .iter()
                        .rev()
                        .find(|job| job.is_visible_for_session(active_session_id))
                        .map(|job| job.id.clone())
                });
        let Some(job_id) = job_id else {
            self.terminal.view.status = "transfer queue is empty".to_string();
            cx.notify();
            return;
        };
        self.request_delete_transfer_job(job_id, cx);
    }

    pub(in crate::features) fn confirm_delete_transfer_job(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.transfer.queue.job_delete.take() else {
            cx.notify();
            return;
        };
        let before = self.transfer.queue.jobs.len();
        self.transfer
            .queue
            .jobs
            .retain(|job| job.id != state.job_id);
        if self.transfer.queue.selected_job_id.as_deref() == Some(state.job_id.as_str()) {
            self.transfer.queue.selected_job_id = None;
        }
        self.terminal.view.status = if self.transfer.queue.jobs.len() < before {
            format!("deleted transfer {}", state.job_id)
        } else {
            "transfer job not found".to_string()
        };
        cx.notify();
    }

    pub(in crate::features) fn cancel_delete_transfer_job(&mut self, cx: &mut Context<Self>) {
        self.transfer.queue.job_delete = None;
        self.terminal.view.status = "transfer delete cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn reveal_transfer_job_target_directory(
        &mut self,
        job_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(job) = self.transfer.queue.jobs.iter().find(|job| job.id == job_id) else {
            self.terminal.view.status = "transfer job not found".to_string();
            cx.notify();
            return;
        };
        let Some(target_path) = transfer_job_local_target_path(job) else {
            self.terminal.view.status = format!("transfer {} has no local target", job.id);
            cx.notify();
            return;
        };
        let target_dir = transfer_job_reveal_dir(target_path);
        cx.reveal_path(&target_dir);
        self.terminal.view.status = format!("opened transfer directory {}", target_dir.display());
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
        if unmodified && keystroke.key == "delete" && self.transfer.queue.job_delete.is_none() {
            cx.stop_propagation();
            self.request_delete_selected_transfer_job(cx);
        }
    }

    pub(in crate::features) fn can_delete_transfer_job(&self, job_id: &str) -> bool {
        let active_session_id = self.active_session_id.as_deref();
        self.transfer
            .queue
            .jobs
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
        format!("{prefix}-{}", self.transfer.queue.jobs.len() + 1)
    }
}

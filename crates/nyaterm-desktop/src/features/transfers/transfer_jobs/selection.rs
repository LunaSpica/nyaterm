use gpui::{Context, KeyDownEvent, MouseDownEvent, Window};

use super::super::transfer_widgets::transfer_job_title;
use super::helpers::{transfer_job_local_target_path, transfer_job_reveal_dir};
use crate::features::NyaTermApp;

impl NyaTermApp {
    pub(in crate::features) fn select_transfer_job(
        &mut self,
        job_id: String,
        cx: &mut Context<Self>,
    ) {
        if self.transfer.select_transfer_job_id(&job_id) {
            self.shell.status = format!("selected transfer {job_id}");
        } else {
            self.shell.status = "transfer job not found".to_string();
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
        window.focus(self.transfer.queue_focus());
        if self
            .transfer
            .open_transfer_job_menu_at(&job_id, event.position.x, event.position.y)
        {
            self.shell.status = "transfer menu opened".to_string();
        } else {
            self.shell.status = "transfer job not found".to_string();
        }
        cx.notify();
    }

    pub(in crate::features) fn close_transfer_job_menu(&mut self, cx: &mut Context<Self>) {
        self.transfer.close_transfer_job_menu();
        cx.notify();
    }

    pub(in crate::features) fn request_delete_transfer_job(
        &mut self,
        job_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(job) = self.transfer.transfer_job(&job_id) else {
            self.shell.status = "transfer job not found".to_string();
            cx.notify();
            return;
        };
        if !self.can_delete_transfer_job(&job_id) {
            self.shell.status = format!("transfer {} cannot be deleted yet", job.id);
            cx.notify();
            return;
        }
        let title = transfer_job_title(&job.kind);
        self.transfer.request_transfer_job_delete(&job_id, title);
        cx.notify();
    }

    pub(in crate::features) fn request_delete_selected_transfer_job(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let active_session_id = self.session.active_id();
        let job_id = self
            .transfer
            .selected_or_latest_visible_transfer_job_id(active_session_id);
        let Some(job_id) = job_id else {
            self.shell.status = "transfer queue is empty".to_string();
            cx.notify();
            return;
        };
        self.request_delete_transfer_job(job_id, cx);
    }

    pub(in crate::features) fn confirm_delete_transfer_job(&mut self, cx: &mut Context<Self>) {
        let Some((job_id, removed)) = self.transfer.confirm_transfer_job_delete() else {
            cx.notify();
            return;
        };
        self.shell.status = if removed {
            format!("deleted transfer {job_id}")
        } else {
            "transfer job not found".to_string()
        };
        cx.notify();
    }

    pub(in crate::features) fn cancel_delete_transfer_job(&mut self, cx: &mut Context<Self>) {
        self.transfer.cancel_transfer_job_delete();
        self.shell.status = "transfer delete cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn reveal_transfer_job_target_directory(
        &mut self,
        job_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(job) = self.transfer.transfer_job(&job_id) else {
            self.shell.status = "transfer job not found".to_string();
            cx.notify();
            return;
        };
        let Some(target_path) = transfer_job_local_target_path(job) else {
            self.shell.status = format!("transfer {} has no local target", job.id);
            cx.notify();
            return;
        };
        let target_dir = transfer_job_reveal_dir(target_path);
        cx.reveal_path(&target_dir);
        self.shell.status = format!("opened transfer directory {}", target_dir.display());
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
        if unmodified && keystroke.key == "delete" && self.transfer.transfer_job_delete().is_none()
        {
            cx.stop_propagation();
            self.request_delete_selected_transfer_job(cx);
        }
    }

    pub(in crate::features) fn can_delete_transfer_job(&self, job_id: &str) -> bool {
        let active_session_id = self.session.active_id();
        self.transfer
            .transfer_job_can_be_deleted(job_id, active_session_id)
    }

    pub(in crate::features) fn next_transfer_id(&mut self, prefix: &str) -> String {
        self.transfer.next_transfer_job_id(prefix)
    }
}

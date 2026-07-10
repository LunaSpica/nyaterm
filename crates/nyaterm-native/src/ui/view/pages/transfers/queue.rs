use super::*;

impl NyaTermApp {
    pub(super) fn transfer_queue_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_running = self
            .transfer_jobs
            .iter()
            .any(|job| job.status == TransferJobStatus::Running);
        let has_paused = self
            .transfer_jobs
            .iter()
            .any(|job| job.status == TransferJobStatus::Paused);
        let has_active = self.transfer_jobs.iter().any(|job| {
            matches!(
                job.status,
                TransferJobStatus::Running | TransferJobStatus::Paused
            )
        });
        let has_completed = self
            .transfer_jobs
            .iter()
            .any(|job| job.status == TransferJobStatus::Completed);
        let has_stopped = self.transfer_jobs.iter().any(|job| {
            matches!(
                job.status,
                TransferJobStatus::Completed
                    | TransferJobStatus::Failed
                    | TransferJobStatus::Cancelled
            )
        });

        let mut jobs = div()
            .id(SharedString::from("transfer-queue-list"))
            .mt_3()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(0x98a3b8))
                            .child("Transfer Queue"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .flex_wrap()
                            .child(queue_action_button(
                                "transfer-open-downloads",
                                "Open Downloads",
                                true,
                                cx.listener(|this, _, _, cx| {
                                    this.reveal_transfer_download_dir(cx);
                                }),
                            ))
                            .child(queue_action_button(
                                "transfer-pause-all",
                                "Pause All",
                                has_running,
                                cx.listener(|this, _, _, cx| {
                                    this.pause_all_transfer_jobs(cx);
                                }),
                            ))
                            .child(queue_action_button(
                                "transfer-resume-all",
                                "Resume All",
                                has_paused,
                                cx.listener(|this, _, _, cx| {
                                    this.resume_all_transfer_jobs(cx);
                                }),
                            ))
                            .child(queue_action_button(
                                "transfer-cancel-all",
                                "Cancel All",
                                has_active,
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_all_transfer_jobs(cx);
                                }),
                            ))
                            .child(queue_action_button(
                                "transfer-clear-completed",
                                "Clear Completed",
                                has_completed,
                                cx.listener(|this, _, _, cx| {
                                    this.clear_completed_transfer_jobs(cx);
                                }),
                            ))
                            .child(queue_action_button(
                                "transfer-clear-stopped",
                                "Clear All",
                                has_stopped,
                                cx.listener(|this, _, _, cx| {
                                    this.clear_stopped_transfer_jobs(cx);
                                }),
                            )),
                    ),
            );
        jobs = jobs
            .track_focus(&self.transfer_queue_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.transfer_queue_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_transfer_queue_key_down(event, cx);
            }));
        if self.transfer_jobs.is_empty() {
            jobs = jobs.child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x10151e))
                    .p_4()
                    .text_sm()
                    .text_color(rgb(0xaeb7c8))
                    .child("Queue is empty."),
            );
        } else {
            for job in ordered_transfer_jobs(&self.transfer_jobs) {
                jobs = jobs.child(transfer_job_row(
                    job,
                    self.transfer_selected_remote_path.clone(),
                    self.transfer_selected_job_id.clone(),
                    cx,
                ));
            }
        }
        jobs
    }
}

fn ordered_transfer_jobs(jobs: &[TransferJobState]) -> Vec<TransferJobState> {
    let mut indexed_jobs = jobs.iter().cloned().enumerate().collect::<Vec<_>>();
    indexed_jobs.sort_by(|(left_index, left), (right_index, right)| {
        transfer_job_display_rank(left.status)
            .cmp(&transfer_job_display_rank(right.status))
            .then_with(|| right_index.cmp(left_index))
    });
    indexed_jobs.into_iter().map(|(_, job)| job).collect()
}

fn transfer_job_display_rank(status: TransferJobStatus) -> u8 {
    match status {
        TransferJobStatus::Running | TransferJobStatus::Cancelling => 0,
        TransferJobStatus::Paused
        | TransferJobStatus::Cancelled
        | TransferJobStatus::Completed
        | TransferJobStatus::Failed => 2,
    }
}

use super::*;

impl NyaTermApp {
    pub(super) fn transfer_queue_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();

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
            !matches!(
                job.status,
                TransferJobStatus::Running
                    | TransferJobStatus::Paused
                    | TransferJobStatus::Cancelling
            )
        });
        let download_path = if self.transfer_local_path.trim().is_empty() {
            "download path unset".to_string()
        } else {
            truncate_preview(&self.transfer_local_path, 48)
        };

        let mut list = div().flex().flex_col();
        if self.transfer_jobs.is_empty() {
            list = list.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .px_3()
                    .py_6()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text_dimmed))
                    .child("No transfers"),
            );
        } else {
            for job in ordered_transfer_jobs(&self.transfer_jobs) {
                list = list.child(transfer_job_row(
                    palette,
                    job,
                    self.transfer_selected_remote_path.clone(),
                    self.transfer_selected_job_id.clone(),
                    cx,
                ));
            }
        }
        div()
            .id(SharedString::from("transfer-queue-panel"))
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(palette.surface))
            .track_focus(&self.transfer_queue_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.transfer_queue_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_transfer_queue_key_down(event, cx);
            }))
            .child(
                div()
                    .h(px(32.))
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.section_header))
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text_muted))
                            .child("FILE TRANSFER"),
                    )
                    .child(div().flex_1())
                    .child(queue_action_button(
                        palette,
                        "transfer-pause-all",
                        "❚❚",
                        has_running,
                        cx.listener(|this, _, _, cx| {
                            this.pause_all_transfer_jobs(cx);
                        }),
                    ))
                    .child(queue_action_button(
                        palette,
                        "transfer-resume-all",
                        "▶",
                        has_paused,
                        cx.listener(|this, _, _, cx| {
                            this.resume_all_transfer_jobs(cx);
                        }),
                    ))
                    .child(queue_action_button(
                        palette,
                        "transfer-cancel-all",
                        "■",
                        has_active,
                        cx.listener(|this, _, _, cx| {
                            this.cancel_all_transfer_jobs(cx);
                        }),
                    ))
                    .child(queue_action_button(
                        palette,
                        "transfer-clear-completed",
                        "✓",
                        has_completed,
                        cx.listener(|this, _, _, cx| {
                            this.clear_completed_transfer_jobs(cx);
                        }),
                    ))
                    .child(queue_action_button(
                        palette,
                        "transfer-clear-stopped",
                        "CLR",
                        has_stopped,
                        cx.listener(|this, _, _, cx| {
                            this.clear_stopped_transfer_jobs(cx);
                        }),
                    )),
            )
            .child(
                div()
                    .id(SharedString::from("transfer-queue-scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_scroll()
                    .scrollbar_width(px(6.))
                    .child(list),
            )
            .child(
                div()
                    .h(px(24.))
                    .px_2()
                    .border_t_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.section_header))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_dimmed))
                            .child("↓"),
                    )
                    .child(
                        div()
                            .id(SharedString::from("transfer-download-path-footer"))
                            .min_w_0()
                            .flex_1()
                            .font_family("JetBrains Mono")
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_muted))
                            .cursor_pointer()
                            .hover(|this| this.text_color(rgb(palette.text)))
                            .child(download_path)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.reveal_transfer_download_dir(cx);
                            })),
                    ),
            )
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
        TransferJobStatus::Paused => 1,
        TransferJobStatus::Cancelled | TransferJobStatus::Completed | TransferJobStatus::Failed => {
            2
        }
    }
}

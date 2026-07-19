use super::*;

impl NyaTermApp {
    pub(super) fn transfer_queue_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let active_session_id = self.active_session_id.as_deref();
        let visible_jobs = self
            .transfer_jobs
            .iter()
            .filter(|job| job.is_visible_for_session(active_session_id))
            .cloned()
            .collect::<Vec<_>>();

        let has_running = visible_jobs
            .iter()
            .any(|job| job.status == TransferJobStatus::Running && job.control.is_some());
        let has_paused = visible_jobs
            .iter()
            .any(|job| job.status == TransferJobStatus::Paused && job.control.is_some());
        let has_active = visible_jobs.iter().any(|job| {
            job.control.is_some()
                && matches!(
                    job.status,
                    TransferJobStatus::Running | TransferJobStatus::Paused
                )
        });
        let has_completed = visible_jobs
            .iter()
            .any(|job| job.status == TransferJobStatus::Completed);
        let has_stopped = visible_jobs.iter().any(|job| {
            !matches!(
                job.status,
                TransferJobStatus::Running
                    | TransferJobStatus::Paused
                    | TransferJobStatus::Cancelling
            )
        });
        let download_path = if self.transfer_local_path.trim().is_empty() {
            format!("{}: -", self.tr("fileTransfer.downloadPath"))
        } else {
            truncate_preview(&self.transfer_local_path, 48)
        };

        let mut list = div().flex().flex_col();
        if self.active_session_id.is_none() {
            list = list.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .px_3()
                    .py_6()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text_dimmed))
                    .child(self.tr("fileExplorer.connectToSession")),
            );
        } else if visible_jobs.is_empty() {
            list = list.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .px_3()
                    .py_6()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text_dimmed))
                    .child(self.tr("fileTransfer.noTransfers")),
            );
        } else {
            list = list.gap(px(2.)).p_1();
            for job in ordered_transfer_jobs(&visible_jobs) {
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
            .bg(self.shell_surface_color(palette.surface))
            .track_focus(&self.transfer_queue_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.transfer_queue_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_transfer_queue_key_down(event, cx);
            }))
            .child(panel_header_with_actions(
                self.tr("panel.fileTransfer"),
                "",
                palette,
                self.shell_transparent_color(palette.section_header),
                Some(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(queue_action_button(
                            palette,
                            "transfer-pause-all",
                            "icons/transfer/pause.svg",
                            self.tr("fileTransfer.pauseAll"),
                            has_running,
                            cx.listener(|this, _, _, cx| {
                                this.pause_all_transfer_jobs(cx);
                            }),
                        ))
                        .child(queue_action_button(
                            palette,
                            "transfer-resume-all",
                            "icons/transfer/play.svg",
                            self.tr("fileTransfer.resumeAll"),
                            has_paused,
                            cx.listener(|this, _, _, cx| {
                                this.resume_all_transfer_jobs(cx);
                            }),
                        ))
                        .child(queue_action_button(
                            palette,
                            "transfer-cancel-all",
                            "icons/transfer/stop.svg",
                            self.tr("fileTransfer.cancelAll"),
                            has_active,
                            cx.listener(|this, _, _, cx| {
                                this.cancel_all_transfer_jobs(cx);
                            }),
                        ))
                        .child(queue_action_button(
                            palette,
                            "transfer-clear-completed",
                            "icons/transfer/playlist-remove.svg",
                            self.tr("fileTransfer.clearCompleted"),
                            has_completed,
                            cx.listener(|this, _, _, cx| {
                                this.clear_completed_transfer_jobs(cx);
                            }),
                        ))
                        .child(queue_action_button(
                            palette,
                            "transfer-clear-stopped",
                            "icons/transfer/clear-all.svg",
                            self.tr("fileTransfer.clearAll"),
                            has_stopped,
                            cx.listener(|this, _, _, cx| {
                                this.clear_stopped_transfer_jobs(cx);
                            }),
                        ))
                        .into_any_element(),
                ),
            ))
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
                    .h(px(26.))
                    .px_2()
                    .border_t_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface))
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .id(SharedString::from("transfer-download-path-footer"))
                            .min_w_0()
                            .flex_1()
                            .font_family(crate::features::gpui_code_font_family())
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .cursor_pointer()
                            .hover(|this| this.text_color(rgb(palette.text)))
                            .tooltip({
                                let label = self.tr("fileTransfer.downloadPath").to_string();
                                move |_, cx| {
                                    cx.new(|_| crate::features::ChromeTooltip::new(label.clone()))
                                        .into()
                                }
                            })
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

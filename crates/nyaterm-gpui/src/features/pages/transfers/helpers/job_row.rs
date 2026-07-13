use super::*;

pub(in crate::features::pages::transfers) fn transfer_job_row(
    palette: crate::theme::ThemePalette,
    job: TransferJobState,
    selected_remote_path: Option<String>,
    selected_job_id: Option<String>,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let status_color = match job.status {
        TransferJobStatus::Running => rgb(palette.warning),
        TransferJobStatus::Paused => rgb(palette.accent),
        TransferJobStatus::Cancelling => rgb(palette.warning),
        TransferJobStatus::Cancelled => rgb(palette.text_muted),
        TransferJobStatus::Completed => rgb(0x34d399),
        TransferJobStatus::Failed => rgb(0xfb7185),
    };
    let job_selected = selected_job_id.as_deref() == Some(job.id.as_str());
    let direction = transfer_direction_label(&job.kind);
    let can_reveal_local_target = transfer_job_has_local_target(&job);
    let can_retry = transfer_job_can_retry(&job);
    let mut status_action = div().flex().items_center().gap_1();
    if job.status == TransferJobStatus::Running && job.control.is_some() {
        let job_id = job.id.clone();
        status_action = status_action.child(small_button(
            palette,
            format!("transfer-pause-{job_id}"),
            "Pause",
            cx.listener(move |this, _, _, cx| {
                this.pause_transfer_job(&job_id, cx);
            }),
        ));
    }
    if job.status == TransferJobStatus::Paused && job.control.is_some() {
        let job_id = job.id.clone();
        status_action = status_action.child(small_button(
            palette,
            format!("transfer-resume-{job_id}"),
            "Resume",
            cx.listener(move |this, _, _, cx| {
                this.resume_transfer_job(&job_id, cx);
            }),
        ));
    }
    if matches!(
        job.status,
        TransferJobStatus::Running | TransferJobStatus::Paused
    ) && job.control.is_some()
    {
        let job_id = job.id.clone();
        status_action = status_action.child(small_button(
            palette,
            format!("transfer-cancel-{job_id}"),
            "Cancel",
            cx.listener(move |this, _, _, cx| {
                this.cancel_transfer_job(&job_id, cx);
            }),
        ));
    }
    if !matches!(
        job.status,
        TransferJobStatus::Running | TransferJobStatus::Paused | TransferJobStatus::Cancelling
    ) {
        if can_retry {
            let job_id = job.id.clone();
            status_action = status_action.child(small_button(
                palette,
                format!("transfer-retry-job-{job_id}"),
                "Retry",
                cx.listener(move |this, _, window, cx| {
                    this.retry_transfer_job(job_id.clone(), window, cx);
                }),
            ));
        }
        let job_id = job.id.clone();
        status_action = status_action.child(small_button(
            palette,
            format!("transfer-delete-job-{job_id}"),
            "Delete",
            cx.listener(move |this, _, _, cx| {
                this.request_delete_transfer_job(job_id.clone(), cx);
            }),
        ));
    }
    if can_reveal_local_target {
        let job_id = job.id.clone();
        status_action = status_action.child(small_button(
            palette,
            format!("transfer-open-target-dir-{job_id}"),
            "Open Dir",
            cx.listener(move |this, _, _, cx| {
                this.reveal_transfer_job_target_directory(job_id.clone(), cx);
            }),
        ));
    }

    let mut entries = div().mt_2().flex().flex_col().gap_1();
    for entry in job.entries.iter().take(6) {
        let entry_path = entry.path.clone();
        let entry_name = entry.name.clone();
        let is_selected = selected_remote_path.as_deref() == Some(entry.path.as_str());
        entries = entries.child(
            div()
                .id(SharedString::from(format!("transfer-entry-{entry_path}")))
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .rounded_sm()
                .px_2()
                .py_1()
                .cursor_pointer()
                .bg(if is_selected {
                    rgb(0x15351f)
                } else {
                    rgb(palette.input)
                })
                .text_xs()
                .text_color(if is_selected {
                    rgb(0xdcfce7)
                } else {
                    rgb(palette.text_muted)
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.transfer_selected_remote_path = Some(entry_path.clone());
                    this.transfer_remote_path = entry_path.clone();
                    this.transfer_focused_field = TransferInputField::Remote;
                    this.terminal_status = format!("selected remote {entry_path}");
                    cx.notify();
                }))
                .child(entry_kind_label(entry.file_type))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(truncate_preview(&entry_name, 54)),
                )
                .child(format_file_size(entry.size)),
        );
    }
    if let Some(summary) = job.summary.as_ref() {
        entries = entries.child(
            div()
                .mt_2()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child(format!(
                    "{} -> {}",
                    summary.remote_path,
                    summary.local_path.display()
                )),
        );
    }

    let progress = job
        .progress
        .as_ref()
        .map(|progress| transfer_progress_bar(palette, progress))
        .unwrap_or_else(|| div().into_any_element());
    let progress_detail = job
        .progress
        .as_ref()
        .map(transfer_progress_percent_label)
        .unwrap_or_else(|| "-".to_string());

    div()
        .id(SharedString::from(format!("transfer-job-row-{}", job.id)))
        .border_b_1()
        .border_color(if job_selected {
            rgb(palette.success)
        } else {
            rgb(palette.surface_elevated)
        })
        .bg(if job_selected {
            rgb(0x10251d)
        } else {
            rgb(palette.surface)
        })
        .px_2()
        .py_2()
        .cursor_pointer()
        .on_click({
            let job_id = job.id.clone();
            cx.listener(move |this, _, window, cx| {
                window.focus(&this.transfer_queue_focus);
                this.select_transfer_job(job_id.clone(), cx);
            })
        })
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(status_pill(
                                    direction,
                                    rgb(palette.accent),
                                    rgb(palette.hover),
                                ))
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(palette.text))
                                        .child(transfer_job_title(&job.kind)),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .text_xs()
                                .text_color(rgb(palette.text_muted))
                                .child(job.detail.clone())
                                .child("·")
                                .child(progress_detail),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight(700.))
                                .text_color(status_color)
                                .child(transfer_status_label(job.status)),
                        )
                        .child(status_action),
                ),
        )
        .child(progress)
        .child(entries)
}

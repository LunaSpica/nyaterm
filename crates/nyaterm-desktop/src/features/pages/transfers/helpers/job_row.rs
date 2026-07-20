use super::*;

pub(in crate::features::pages::transfers) fn transfer_job_row(
    palette: crate::theme::ThemePalette,
    job: TransferJobState,
    _selected_remote_path: Option<String>,
    selected_job_id: Option<String>,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let status_color = match job.status {
        TransferJobStatus::Running => rgb(palette.warning),
        TransferJobStatus::Paused => rgb(palette.link),
        TransferJobStatus::Cancelling => rgb(palette.warning),
        TransferJobStatus::Cancelled => rgb(palette.text_muted),
        TransferJobStatus::Completed => rgb(0x34d399),
        TransferJobStatus::Failed => rgb(0xfb7185),
    };
    let job_selected = selected_job_id.as_deref() == Some(job.id.as_str());
    let direction = transfer_direction_label(&job.kind);
    let title = transfer_job_title(&job.kind);
    let entry_detail = job.entries.first().map(|entry| {
        let size = format_file_size(entry.size);
        if size == "-" {
            entry.name.clone()
        } else {
            format!("{} · {size}", entry.name)
        }
    });
    let summary_detail = job.summary.as_ref().map(|summary| {
        format!(
            "{} -> {}",
            summary.remote_path,
            summary.local_path.display()
        )
    });
    let detail = entry_detail
        .or(summary_detail)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| job.detail.clone());

    let progress_label = job
        .progress
        .as_ref()
        .map(transfer_progress_percent_label)
        .unwrap_or_else(|| transfer_status_label(job.status).to_string());
    let progress_percent = job
        .progress
        .as_ref()
        .and_then(|progress| {
            progress
                .total_bytes
                .filter(|total| *total > 0)
                .map(|total| progress.bytes_transferred as f32 / total as f32)
        })
        .map(|percent| percent.clamp(0., 1.));
    let context_job_id = job.id.clone();

    div()
        .id(SharedString::from(format!("transfer-job-row-{}", job.id)))
        .rounded_sm()
        .bg(if job_selected {
            rgb(palette.hover)
        } else {
            rgb(palette.surface)
        })
        .border_1()
        .border_color(if job_selected {
            rgb(palette.link)
        } else {
            rgb(palette.surface)
        })
        .px_2()
        .py(px(6.))
        .cursor_pointer()
        .on_click({
            let job_id = job.id.clone();
            cx.listener(move |this, _, window, cx| {
                window.focus(&this.transfer_queue_focus);
                this.select_transfer_job(job_id.clone(), cx);
            })
        })
        .on_mouse_down(
            MouseButton::Right,
            cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                cx.stop_propagation();
                this.open_transfer_job_menu(context_job_id.clone(), event, window, cx);
            }),
        )
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
                                .gap_1()
                                .child(status_pill(direction, rgb(palette.link), rgb(palette.bg)))
                                .child(
                                    div()
                                        .min_w_0()
                                        .text_size(px(12.))
                                        .text_color(rgb(palette.text))
                                        .child(truncate_preview(&title, 44)),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .overflow_hidden()
                                .text_size(px(10.))
                                .text_color(rgb(palette.text_muted))
                                .child(truncate_preview(&detail, 58)),
                        ),
                )
                .child(
                    div()
                        .flex_none()
                        .text_size(px(10.))
                        .font_weight(FontWeight(700.))
                        .text_color(status_color)
                        .child(progress_label),
                ),
        )
        .when_some(progress_percent, |this, percent| {
            this.child(
                div()
                    .mt_1()
                    .h(px(4.))
                    .rounded_full()
                    .overflow_hidden()
                    .bg(rgb(palette.border))
                    .child(
                        div()
                            .h_full()
                            .w(gpui::relative(percent))
                            .rounded_full()
                            .bg(rgb(palette.link)),
                    ),
            )
        })
}

use gpui::{
    Context, FontWeight, InteractiveElement as _, IntoElement, MouseButton, MouseDownEvent,
    ParentElement as _, SharedString, StatefulInteractiveElement as _, Styled as _, div,
    prelude::FluentBuilder as _, px, relative, rgb, svg,
};
use nyaterm_core::truncate_preview;

use crate::features::{NyaTermApp, format_file_size, transfer_status_label};
use crate::models::{TransferJobKind, TransferJobState, TransferJobStatus};
use crate::theme::ThemePalette;

use super::{transfer_progress_percent_label, transfer_progress_ratio};

pub(in crate::features::pages::transfers) fn transfer_job_row(
    palette: ThemePalette,
    job: TransferJobState,
    directory_progress: Option<String>,
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
    let file_name = transfer_job_file_name(&job);
    let icon_path = transfer_job_icon_path(&job.kind);
    let direction_color = transfer_job_direction_color(&job.kind);
    let detail = transfer_job_detail(&job, directory_progress);
    let progress_label = if job.status == TransferJobStatus::Running {
        job.progress
            .as_ref()
            .map(transfer_progress_percent_label)
            .unwrap_or_else(|| "0 B/s".to_string())
    } else {
        transfer_status_label(job.status).to_string()
    };
    let progress_percent = job.progress.as_ref().and_then(transfer_progress_ratio);
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
                window.focus(this.transfer.queue_focus(), cx);
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
                .gap_2()
                .child(
                    svg()
                        .size(px(14.))
                        .flex_none()
                        .path(icon_path)
                        .text_color(direction_color),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .min_w_0()
                                .text_size(px(12.))
                                .text_color(rgb(palette.text))
                                .child(truncate_preview(&file_name, 48)),
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
                        .min_w(px(52.))
                        .text_align(gpui::TextAlign::Right)
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
                            .w(relative(percent))
                            .rounded_full()
                            .bg(direction_color),
                    ),
            )
        })
}

fn transfer_job_file_name(job: &TransferJobState) -> String {
    job.summary
        .as_ref()
        .map(|summary| match &job.kind {
            TransferJobKind::Upload { .. } => local_file_name(&summary.local_path),
            _ => remote_file_name(&summary.remote_path),
        })
        .or_else(|| {
            job.progress
                .as_ref()
                .map(|progress| remote_file_name(&progress.remote_path))
        })
        .unwrap_or_else(|| match &job.kind {
            TransferJobKind::Download { remote_path, .. }
            | TransferJobKind::OpenExternal { remote_path, .. }
            | TransferJobKind::LoadEditor { remote_path }
            | TransferJobKind::SaveEditor { remote_path }
            | TransferJobKind::LoadProperties { remote_path }
            | TransferJobKind::UpdateProperties { remote_path, .. }
            | TransferJobKind::AiFileAction { remote_path, .. } => remote_file_name(remote_path),
            TransferJobKind::Upload { local_path, .. } => local_file_name(local_path),
            TransferJobKind::ZmodemUpload { file_name, .. }
            | TransferJobKind::ZmodemDownload { file_name, .. }
            | TransferJobKind::TrzszDownload { file_name, .. }
            | TransferJobKind::TrzszUpload { file_name, .. } => file_name.clone(),
            other => truncate_preview(&format!("{other:?}"), 48),
        })
}

fn transfer_job_detail(job: &TransferJobState, directory_progress: Option<String>) -> String {
    let time = format_transfer_row_time();
    let size_detail = job.progress.as_ref().and_then(|progress| {
        progress
            .total_bytes
            .filter(|total| *total > 0)
            .map(|total| {
                format!(
                    "{} / {}",
                    format_file_size(Some(progress.bytes_transferred)),
                    format_file_size(Some(total))
                )
            })
    });
    let completed_size = job.summary.as_ref().and_then(|summary| {
        (summary.bytes > 0).then(|| {
            format!(
                "{} / {}",
                format_file_size(Some(summary.bytes)),
                format_file_size(Some(summary.bytes))
            )
        })
    });
    let text = directory_progress
        .or(size_detail)
        .or(completed_size)
        .or_else(|| (job.status == TransferJobStatus::Failed).then(|| job.detail.clone()))
        .filter(|value| !value.trim().is_empty());
    match text {
        Some(text) => format!("{time} · {text}"),
        None => time,
    }
}

fn format_transfer_row_time() -> String {
    let Ok(now) = time::OffsetDateTime::now_local() else {
        return "--:--:--".to_string();
    };
    let format = time::macros::format_description!("[hour]:[minute]:[second]");
    now.format(&format)
        .unwrap_or_else(|_| "--:--:--".to_string())
}

fn transfer_job_icon_path(kind: &TransferJobKind) -> &'static str {
    match kind {
        TransferJobKind::Upload { .. }
        | TransferJobKind::ZmodemUpload { .. }
        | TransferJobKind::TrzszUpload { .. } => "icons/fe/upload.svg",
        _ => "icons/fe/download.svg",
    }
}

fn transfer_job_direction_color(kind: &TransferJobKind) -> gpui::Rgba {
    match kind {
        TransferJobKind::Upload { .. }
        | TransferJobKind::ZmodemUpload { .. }
        | TransferJobKind::TrzszUpload { .. } => rgb(0x4ade80),
        _ => rgb(0x60a5fa),
    }
}

fn remote_file_name(path: &str) -> String {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path)
        .to_string()
}

fn local_file_name(path: &std::path::Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

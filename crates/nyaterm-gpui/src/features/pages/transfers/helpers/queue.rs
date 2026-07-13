use super::*;

pub(in crate::features::pages::transfers) fn transfer_queue_counts(
    jobs: &[TransferJobState],
) -> (usize, usize, usize, usize, usize) {
    let total = jobs.len();
    let running = jobs
        .iter()
        .filter(|job| {
            matches!(
                job.status,
                TransferJobStatus::Running | TransferJobStatus::Cancelling
            )
        })
        .count();
    let paused = jobs
        .iter()
        .filter(|job| job.status == TransferJobStatus::Paused)
        .count();
    let completed = jobs
        .iter()
        .filter(|job| job.status == TransferJobStatus::Completed)
        .count();
    let failed = jobs
        .iter()
        .filter(|job| job.status == TransferJobStatus::Failed)
        .count();
    (total, running, paused, completed, failed)
}

pub(in crate::features::pages::transfers) fn queue_metric(
    palette: crate::theme::ThemePalette,
    label: &'static str,
    value: usize,
    color: impl Into<Hsla>,
) -> impl IntoElement {
    let color = color.into();
    div()
        .flex()
        .items_center()
        .gap_1()
        .rounded_sm()
        .bg(rgb(palette.input))
        .px_2()
        .py_1()
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(palette.text_muted))
                .child(label),
        )
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight(800.))
                .text_color(color)
                .child(value.to_string()),
        )
}

pub(in crate::features::pages::transfers) fn queue_action_button(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(22.))
        .min_w(px(22.))
        .px_1()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .text_size(px(10.))
        .text_color(if enabled {
            rgb(palette.text_muted)
        } else {
            rgb(palette.border)
        })
        .cursor_pointer()
        .hover(|this| {
            this.bg(rgb(palette.surface_elevated))
                .text_color(rgb(palette.text))
        })
        .opacity(if enabled { 1. } else { 0.4 })
        .when(enabled, |this| this.on_click(on_click))
        .child(label)
}

pub(in crate::features::pages::transfers) fn duplicate_policy_short_label(policy: SftpDuplicatePolicy) -> &'static str {
    match policy {
        SftpDuplicatePolicy::Ask => "ask",
        SftpDuplicatePolicy::Overwrite => "overwrite",
        SftpDuplicatePolicy::Skip => "skip",
        SftpDuplicatePolicy::Rename => "rename",
    }
}

pub(in crate::features::pages::transfers) fn transfer_direction_label(kind: &TransferJobKind) -> &'static str {
    match kind {
        TransferJobKind::ListDir { .. } => "LIST",
        TransferJobKind::ResolveHome => "HOME",
        TransferJobKind::SyncCwd => "CWD",
        TransferJobKind::Download { .. } => "DOWN",
        TransferJobKind::Upload { .. } => "UP",
        TransferJobKind::Rename { .. } => "REN",
        TransferJobKind::Move { .. } => "MOV",
        TransferJobKind::Delete { .. } => "DEL",
        TransferJobKind::Mkdir { .. } => "MKD",
        TransferJobKind::CreateFile { .. } => "NEW",
        TransferJobKind::Symlink { .. } => "LNK",
        TransferJobKind::LoadProperties { .. } => "GET",
        TransferJobKind::UpdateProperties { .. } => "SET",
        TransferJobKind::LoadEditor { .. } => "EDIT",
        TransferJobKind::SaveEditor { .. } => "SAVE",
        TransferJobKind::OpenExternal { .. } => "OPEN",
        TransferJobKind::AiFileAction { .. } => "AI",
        TransferJobKind::ZmodemUpload { .. } => "Z↑",
        TransferJobKind::ZmodemDownload { .. } => "Z↓",
        TransferJobKind::ZmodemConflictProbe { .. } => "Z?",
    }
}

pub(in crate::features::pages::transfers) fn transfer_job_has_local_target(job: &TransferJobState) -> bool {
    job.summary.is_some()
        || job.progress.is_some()
        || matches!(
            job.kind,
            TransferJobKind::Download { .. } | TransferJobKind::OpenExternal { .. }
        )
}

pub(in crate::features::pages::transfers) fn transfer_job_can_retry(job: &TransferJobState) -> bool {
    matches!(
        job.status,
        TransferJobStatus::Failed | TransferJobStatus::Cancelled
    ) && matches!(
        job.kind,
        TransferJobKind::Download { .. } | TransferJobKind::Upload { .. }
    )
}

pub(in crate::features::pages::transfers) fn transfer_progress_percent_label(progress: &SftpTransferProgress) -> String {
    match progress.total_bytes.filter(|total| *total > 0) {
        Some(total) => {
            let percent = (progress.bytes_transferred as f64 / total as f64 * 100.).clamp(0., 100.);
            format!("{percent:.0}%")
        }
        None => "streaming".to_string(),
    }
}

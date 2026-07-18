use crate::theme::ThemePalette;
use gpui::{FontWeight, IntoElement, SharedString, div, prelude::*, px, rgb};
use nyaterm_core::truncate_preview;
use nyaterm_transport::{SftpDuplicateDecision, SftpDuplicatePolicy, SftpTransferProgress};

use crate::models::{TransferJobKind, TransferJobState, TransferJobStatus};
use crate::widgets::status_pill;

pub(in crate::features) fn compact_transfer_job_row(
    palette: ThemePalette,
    job: &TransferJobState,
) -> impl IntoElement {
    let (status_fg, status_bg) = match job.status {
        TransferJobStatus::Running => (rgb(palette.success), rgb(palette.hover)),
        TransferJobStatus::Paused => (rgb(0xfacc15), rgb(0x3a2f14)),
        TransferJobStatus::Cancelling => (rgb(0xfbbf24), rgb(0x3a2f14)),
        TransferJobStatus::Cancelled => (rgb(0xcbd5e1), rgb(palette.border)),
        TransferJobStatus::Completed => (rgb(0x86efac), rgb(0x12301f)),
        TransferJobStatus::Failed => (rgb(0xfca5a5), rgb(0x3a1717)),
    };
    let detail = if job.detail.trim().is_empty() {
        transfer_job_title(&job.kind)
    } else {
        job.detail.clone()
    };

    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_3()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .text_sm()
                        .font_weight(FontWeight(800.))
                        .child(truncate_preview(&transfer_job_title(&job.kind), 30)),
                )
                .child(status_pill(
                    transfer_status_label(job.status),
                    status_fg,
                    status_bg,
                )),
        )
        .child(
            div()
                .mt_1()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child(truncate_preview(&detail, 48)),
        )
}

pub(in crate::features) fn duplicate_policy_label(policy: SftpDuplicatePolicy) -> &'static str {
    match policy {
        SftpDuplicatePolicy::Ask => "ask",
        SftpDuplicatePolicy::Overwrite => "overwrite",
        SftpDuplicatePolicy::Skip => "skip",
        SftpDuplicatePolicy::Rename => "rename",
    }
}

pub(in crate::features) fn transfer_input(
    id: impl Into<String>,
    label: &'static str,
    value: String,
    active: bool,
    palette: ThemePalette,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(SharedString::from(id.into()))
        .h(px(36.))
        .px_3()
        .py_1()
        .flex()
        .flex_col()
        .gap_0()
        .rounded_sm()
        .border_1()
        .border_color(if active {
            rgb(palette.link)
        } else {
            rgb(palette.border)
        })
        .bg(rgb(palette.input))
        .cursor_pointer()
        .child(
            div()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child(label),
        )
        .child(
            div()
                .font_family(crate::features::gpui_code_font_family())
                .text_xs()
                .text_color(rgb(palette.text))
                .child(value),
        )
}

pub(in crate::features) fn transfer_job_title(kind: &TransferJobKind) -> String {
    match kind {
        TransferJobKind::ListDir { remote_path, .. } => format!("List {remote_path}"),
        TransferJobKind::ResolveHome => "Resolve remote home".to_string(),
        TransferJobKind::SyncCwd => "Sync remote cwd".to_string(),
        TransferJobKind::Download {
            remote_path,
            local_path,
        } => format!("Download {remote_path} -> {}", local_path.display()),
        TransferJobKind::Upload {
            local_path,
            remote_path,
        } => format!("Upload {} -> {remote_path}", local_path.display()),
        TransferJobKind::Rename {
            old_path, new_path, ..
        } => format!("Rename {old_path} -> {new_path}"),
        TransferJobKind::Move {
            old_path, new_path, ..
        } => format!("Move {old_path} -> {new_path}"),
        TransferJobKind::Delete { remote_path, .. } => format!("Delete {remote_path}"),
        TransferJobKind::Mkdir { remote_path, .. } => format!("Create folder {remote_path}"),
        TransferJobKind::CreateFile { remote_path, .. } => format!("Create file {remote_path}"),
        TransferJobKind::Symlink {
            link_path,
            target_path,
            ..
        } => format!("Symlink {link_path} -> {target_path}"),
        TransferJobKind::LoadProperties { remote_path } => {
            format!("Load properties {remote_path}")
        }
        TransferJobKind::UpdateProperties { remote_path, .. } => {
            format!("Update properties {remote_path}")
        }
        TransferJobKind::LoadEditor { remote_path } => format!("Open text {remote_path}"),
        TransferJobKind::SaveEditor { remote_path } => format!("Save text {remote_path}"),
        TransferJobKind::OpenExternal {
            remote_path,
            local_path,
        } => format!("Open {remote_path} -> {}", local_path.display()),
        TransferJobKind::AiFileAction {
            remote_path,
            action_name,
            ..
        } => format!("AI {action_name} <- {remote_path}"),
        TransferJobKind::ZmodemUpload {
            file_name,
            session_id,
        } => format!("ZMODEM ↑ {file_name} ({session_id})"),
        TransferJobKind::ZmodemDownload {
            file_name,
            session_id,
        } => format!("ZMODEM ↓ {file_name} ({session_id})"),
        TransferJobKind::TrzszDownload {
            file_name,
            session_id,
        } => format!("trzsz ↓ {file_name} ({session_id})"),
        TransferJobKind::TrzszUpload {
            file_name,
            session_id,
        } => format!("trzsz ↑ {file_name} ({session_id})"),
        TransferJobKind::ZmodemConflictProbe {
            session_id,
            remote_dir,
        } => format!("ZMODEM probe {remote_dir} ({session_id})"),
    }
}

pub(in crate::features) fn transfer_status_label(status: TransferJobStatus) -> &'static str {
    match status {
        TransferJobStatus::Running => "Running",
        TransferJobStatus::Paused => "Paused",
        TransferJobStatus::Cancelling => "Cancelling",
        TransferJobStatus::Cancelled => "Cancelled",
        TransferJobStatus::Completed => "Done",
        TransferJobStatus::Failed => "Failed",
    }
}

pub(in crate::features) fn format_transfer_progress(progress: &SftpTransferProgress) -> String {
    let transferred = format_file_size(Some(progress.bytes_transferred));
    match progress.total_bytes.filter(|total| *total > 0) {
        Some(total) => {
            let percent = (progress.bytes_transferred as f64 / total as f64 * 100.).clamp(0., 100.);
            format!(
                "{transferred} / {} ({percent:.0}%)",
                format_file_size(Some(total))
            )
        }
        None => format!("{transferred} transferred"),
    }
}

pub(in crate::features) fn format_file_size(size: Option<u64>) -> String {
    let Some(size) = size else {
        return String::new();
    };
    if size >= 1024 * 1024 {
        format!("{:.1} MiB", size as f64 / 1024. / 1024.)
    } else if size >= 1024 {
        format!("{:.1} KiB", size as f64 / 1024.)
    } else {
        format!("{size} B")
    }
}

pub(in crate::features) fn duplicate_decision_label(
    decision: SftpDuplicateDecision,
) -> &'static str {
    match decision {
        SftpDuplicateDecision::Overwrite => "overwrite",
        SftpDuplicateDecision::Skip => "skip",
        SftpDuplicateDecision::Rename => "rename",
    }
}

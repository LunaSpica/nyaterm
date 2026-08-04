//! Transfer jobs, transfer options, path prompts and transfer widgets.

mod editor_window;
mod external_sync_window;
mod remote_text_editor;
mod state;
mod transfer_events;
mod transfer_jobs;
mod transfer_options;
mod transfer_paths;
mod transfer_widgets;

use nyaterm_transport::{SftpService, SshMultiplexHandle, SshProcessService, SshSessionConfig};

#[derive(Clone)]
pub(in crate::features) struct SftpJobSession {
    pub session_id: Option<String>,
    pub config: SshSessionConfig,
    pub multiplex: Option<SshMultiplexHandle>,
}

pub(in crate::features) fn session_sftp_service(
    config: SshSessionConfig,
    multiplex: Option<SshMultiplexHandle>,
) -> anyhow::Result<SftpService> {
    match multiplex {
        Some(multiplex) => SftpService::with_multiplex(config, multiplex),
        None => Ok(SftpService::new(config)),
    }
}

pub(in crate::features) fn session_ssh_process_service(
    config: SshSessionConfig,
    multiplex: Option<SshMultiplexHandle>,
) -> anyhow::Result<SshProcessService> {
    match multiplex {
        Some(multiplex) => SshProcessService::with_multiplex(config, multiplex),
        None => Ok(SshProcessService::new(config)),
    }
}

pub(in crate::features) use remote_text_editor::RemoteTextEditor;
pub(in crate::features) use state::{
    TransferEditorCloseAfterSave, TransferEditorCloseOutcome, TransferEditorDiscardOutcome,
    TransferFeatureFocus, TransferFeatureState,
};
pub(in crate::features) use transfer_widgets::{
    duplicate_decision_label, duplicate_policy_label, format_file_size, transfer_job_title,
    transfer_status_label,
};

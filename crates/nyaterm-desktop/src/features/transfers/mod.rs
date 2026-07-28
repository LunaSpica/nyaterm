//! Transfer jobs, transfer options, path prompts and transfer widgets.

mod state;
mod transfer_events;
mod transfer_jobs;
mod transfer_options;
mod transfer_paths;
mod transfer_widgets;

pub(in crate::features) use state::{TransferFeatureFocus, TransferFeatureState};
pub(in crate::features) use transfer_widgets::{
    duplicate_decision_label, duplicate_policy_label, format_file_size, transfer_job_title,
    transfer_status_label,
};

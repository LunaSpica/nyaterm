//! Authoritative transient state for native update checks.

use std::sync::mpsc;

use nyaterm_core::NativeUpdateInfo;

pub(super) struct UpdateJobResult {
    pub result: Result<NativeUpdateInfo, String>,
}

pub(in crate::features) struct UpdateFeatureState {
    pub(super) tx: mpsc::Sender<UpdateJobResult>,
    pub(super) rx: mpsc::Receiver<UpdateJobResult>,
    pub status: String,
    pub info: Option<NativeUpdateInfo>,
    pub pending: bool,
    pub dialog_open: bool,
}

impl UpdateFeatureState {
    pub(in crate::features) fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx,
            status: format!("Current version {}", env!("CARGO_PKG_VERSION")),
            info: None,
            pending: false,
            dialog_open: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::UpdateFeatureState;

    #[test]
    fn update_state_owns_job_channel_and_initial_status() {
        let state = UpdateFeatureState::new();

        assert!(state.status.contains(env!("CARGO_PKG_VERSION")));
        assert!(state.rx.try_recv().is_err());
        assert!(state.info.is_none());
        assert!(!state.pending);
        assert!(!state.dialog_open);
    }
}

//! Authoritative transient state for native update checks.

use std::sync::mpsc;

use nyaterm_core::NativeUpdateInfo;

pub(super) struct UpdateJobResult {
    result: Result<NativeUpdateInfo, String>,
}

impl UpdateJobResult {
    pub(super) fn new(result: Result<NativeUpdateInfo, String>) -> Self {
        Self { result }
    }
}

const UPDATE_EVENT_DRAIN_LIMIT: usize = 4;

pub(in crate::features) struct UpdateFeatureState {
    tx: mpsc::Sender<UpdateJobResult>,
    rx: mpsc::Receiver<UpdateJobResult>,
    status: String,
    info: Option<NativeUpdateInfo>,
    pending: bool,
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
        }
    }

    pub(in crate::features) fn status(&self) -> &str {
        &self.status
    }

    pub(in crate::features) fn info(&self) -> Option<&NativeUpdateInfo> {
        self.info.as_ref()
    }

    pub(in crate::features) fn is_pending(&self) -> bool {
        self.pending
    }

    pub(super) fn begin_check(&mut self) -> Option<mpsc::Sender<UpdateJobResult>> {
        if self.pending {
            self.status = "update check already running".to_string();
            return None;
        }
        self.pending = true;
        self.status = "checking GitHub releases...".to_string();
        self.info = None;
        Some(self.tx.clone())
    }

    pub(super) fn drain_events(&mut self) -> bool {
        if !self.pending {
            return false;
        }
        let mut dirty = false;
        for _ in 0..UPDATE_EVENT_DRAIN_LIMIT {
            let Ok(event) = self.rx.try_recv() else {
                break;
            };
            dirty = true;
            self.pending = false;
            match event.result {
                Ok(info) => {
                    self.status = if info.available {
                        format!(
                            "update available: {} -> {}",
                            info.current_version, info.latest_version
                        )
                    } else {
                        format!("NyaTerm is up to date ({})", info.current_version)
                    };
                    self.info = Some(info);
                }
                Err(error) => {
                    self.status = format!("update check failed: {error}");
                    self.info = None;
                }
            }
        }
        dirty
    }
}

#[cfg(test)]
mod tests {
    use super::{UpdateFeatureState, UpdateJobResult};

    #[test]
    fn update_state_owns_job_channel_and_initial_status() {
        let state = UpdateFeatureState::new();

        assert!(state.status().contains(env!("CARGO_PKG_VERSION")));
        assert!(state.rx.try_recv().is_err());
        assert!(state.info().is_none());
        assert!(!state.is_pending());
    }

    #[test]
    fn update_check_admission_prevents_overlapping_jobs() {
        let mut state = UpdateFeatureState::new();

        assert!(state.begin_check().is_some());
        assert!(state.is_pending());
        assert_eq!(state.status(), "checking GitHub releases...");
        assert!(state.begin_check().is_none());
        assert_eq!(state.status(), "update check already running");
    }

    #[test]
    fn update_event_drain_completes_failed_job() {
        let mut state = UpdateFeatureState::new();
        let tx = state.begin_check().expect("first check should start");
        tx.send(UpdateJobResult::new(Err("offline".to_string())))
            .expect("state should retain its event receiver");

        assert!(state.drain_events());
        assert!(!state.is_pending());
        assert_eq!(state.status(), "update check failed: offline");
        assert!(state.info().is_none());
    }
}

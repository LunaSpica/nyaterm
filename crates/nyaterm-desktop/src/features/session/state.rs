use std::collections::{HashMap, HashSet};
use std::sync::{Arc, mpsc};
use std::time::Instant;

use nyaterm_core::AiExecutionProfile;
use nyaterm_transport::{SessionKind, SessionManager, SshMultiplexHandle, SshSessionConfig};

use crate::features::runtime_jobs::SessionStartResult;
use crate::models::{
    ActiveSessionMenuState, SessionEventBridge, SessionLaunchConfig, SessionRuntimeMetadata,
    StartupCommandRequest, WorkspaceSplitDirection,
};

use super::trzsz_runtime::TrzszSessionState;
use super::zmodem_runtime::ZmodemSessionState;

pub(in crate::features) struct SessionFeatureState {
    pub manager: Arc<SessionManager>,
    pub event_bridge: SessionEventBridge,
    pub start: SessionStartFeatureState,
    pub command_history: HashMap<String, Vec<String>>,
    pub active_search_draft: String,
    pub active_menu: Option<ActiveSessionMenuState>,
    /// Per-session reconnect/disconnect busy state ("reconnect" | "disconnect").
    pub busy_actions: HashMap<String, String>,
    pub active_id: Option<String>,
    pub active_ssh_config: Option<SshSessionConfig>,
    pub active_ai_execution_profile: AiExecutionProfile,
    pub order: Vec<String>,
    pub metadata: HashMap<String, SessionRuntimeMetadata>,
    pub custom_names: HashMap<String, String>,
    /// OSC 0/2 titles from the session PTY (fall back when no custom rename).
    pub dynamic_titles: HashMap<String, String>,
    /// Latest OSC 7 working directories per session.
    pub cwds: HashMap<String, String>,
    /// Per-session ZMODEM detector / transfer state (UI-layer interception).
    pub zmodem: HashMap<String, ZmodemSessionState>,
    /// Per-session trzsz trigger detector state (pre-parser protocol slot).
    pub trzsz: HashMap<String, TrzszSessionState>,
    pub tab_colors: HashMap<String, u32>,
    pub multiplex_handles: HashMap<String, SshMultiplexHandle>,
}

impl SessionFeatureState {
    pub(in crate::features) fn new(
        manager: Arc<SessionManager>,
        event_bridge: SessionEventBridge,
    ) -> Self {
        Self {
            manager,
            event_bridge,
            start: SessionStartFeatureState::new(),
            command_history: HashMap::new(),
            active_search_draft: String::new(),
            active_menu: None,
            busy_actions: HashMap::new(),
            active_id: None,
            active_ssh_config: None,
            active_ai_execution_profile: AiExecutionProfile::SendOnly,
            order: Vec::new(),
            metadata: HashMap::new(),
            custom_names: HashMap::new(),
            dynamic_titles: HashMap::new(),
            cwds: HashMap::new(),
            zmodem: HashMap::new(),
            trzsz: HashMap::new(),
            tab_colors: HashMap::new(),
            multiplex_handles: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::features) enum SessionPaneState {
    Connecting {
        request_id: String,
        name: String,
        kind: SessionKind,
    },
    Live {
        session_id: String,
    },
    Failed {
        name: String,
        error: String,
    },
    Disconnected {
        session_id: String,
    },
}

pub(in crate::features) struct PendingSessionStart {
    pub connection_name: String,
    pub launch_config: Option<SessionLaunchConfig>,
    pub requested_at: Instant,
    pub kind: SessionKind,
    pub ai_execution_profile: AiExecutionProfile,
    pub custom_name: Option<String>,
    pub tab_color: Option<u32>,
    pub after_session_id: Option<String>,
    pub insert_index: Option<usize>,
    pub seed_output: Option<String>,
    pub startup_command: Option<StartupCommandRequest>,
    pub multiplex_key: Option<String>,
    pub source_connection_id: Option<String>,
    /// Existing pane being replaced by this request, when this is a reconnect.
    pub reconnect_session_id: Option<String>,
}

/// A session start that remains visible after its worker failed.
///
/// Tauri keeps the failed pane in its original tab, so the GPUI shell must
/// retain the pending metadata instead of reducing the failure to a global
/// banner.
pub(in crate::features) struct FailedSessionStart {
    pub pending: PendingSessionStart,
    pub error: String,
}

pub(in crate::features) struct SessionStartFeatureState {
    tx: mpsc::Sender<SessionStartResult>,
    rx: mpsc::Receiver<SessionStartResult>,
    pub pending: HashMap<String, PendingSessionStart>,
    pub active_pending: Option<String>,
    pub failed: HashMap<String, FailedSessionStart>,
    pub active_failed: Option<String>,
    pub cancelled: HashSet<String>,
    pub panes: HashMap<String, SessionPaneState>,
    pub reconnect_replace_id: Option<String>,
    pub reconnect_failures: HashMap<String, String>,
    pub pending_workspace_split: Option<(WorkspaceSplitDirection, String)>,
}

impl SessionStartFeatureState {
    pub(in crate::features) fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx,
            pending: HashMap::new(),
            active_pending: None,
            failed: HashMap::new(),
            active_failed: None,
            cancelled: HashSet::new(),
            panes: HashMap::new(),
            reconnect_replace_id: None,
            reconnect_failures: HashMap::new(),
            pending_workspace_split: None,
        }
    }

    pub(in crate::features) fn sender(&self) -> mpsc::Sender<SessionStartResult> {
        self.tx.clone()
    }

    pub(in crate::features) fn try_recv(&self) -> Result<SessionStartResult, mpsc::TryRecvError> {
        self.rx.try_recv()
    }

    pub(in crate::features) fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    pub(in crate::features) fn has_failed(&self) -> bool {
        !self.failed.is_empty()
    }

    pub(in crate::features) fn pending_display_name(&self) -> Option<String> {
        self.active_pending
            .as_deref()
            .and_then(|request_id| self.pending.get(request_id))
            .or_else(|| {
                self.pending
                    .values()
                    .filter(|pending| pending.reconnect_session_id.is_none())
                    .min_by(|left, right| {
                        left.requested_at
                            .cmp(&right.requested_at)
                            .then_with(|| left.connection_name.cmp(&right.connection_name))
                    })
            })
            .map(pending_session_start_display_name)
    }

    pub(in crate::features) fn active_failed(&self) -> Option<&FailedSessionStart> {
        self.active_failed
            .as_deref()
            .and_then(|request_id| self.failed.get(request_id))
    }

    pub(in crate::features) fn failed_display_name(&self) -> Option<String> {
        self.active_failed()
            .or_else(|| {
                self.failed.values().min_by(|left, right| {
                    left.pending
                        .requested_at
                        .cmp(&right.pending.requested_at)
                        .then_with(|| {
                            left.pending
                                .connection_name
                                .cmp(&right.pending.connection_name)
                        })
                })
            })
            .map(failed_session_start_display_name)
    }

    pub(in crate::features) fn select_pending(&mut self, request_id: &str) -> bool {
        if !self.pending.contains_key(request_id) {
            return false;
        }
        self.active_pending = Some(request_id.to_string());
        self.active_failed = None;
        true
    }

    pub(in crate::features) fn close_pending(
        &mut self,
        request_id: &str,
    ) -> Option<PendingSessionStart> {
        let pending = self.pending.remove(request_id)?;
        self.cancelled.insert(request_id.to_string());
        self.panes.remove(request_id);
        if self.active_pending.as_deref() == Some(request_id) {
            self.active_pending = self.latest_pending_request_id();
            if self.active_pending.is_none() {
                self.active_failed = self.latest_failed_request_id();
            }
        }
        Some(pending)
    }

    pub(in crate::features) fn select_failed(&mut self, request_id: &str) -> bool {
        if !self.failed.contains_key(request_id) {
            return false;
        }
        self.active_failed = Some(request_id.to_string());
        self.active_pending = None;
        true
    }

    pub(in crate::features) fn close_failed(
        &mut self,
        request_id: &str,
    ) -> Option<FailedSessionStart> {
        let failed = self.failed.remove(request_id)?;
        self.panes.remove(request_id);
        if self.active_failed.as_deref() == Some(request_id) {
            self.active_failed = None;
            self.active_pending = self.latest_pending_request_id();
            if self.active_pending.is_none() {
                self.active_failed = self.latest_failed_request_id();
            }
        }
        Some(failed)
    }

    pub(in crate::features) fn pending_status_source(&self) -> Option<(String, Instant)> {
        self.pending
            .values()
            .min_by(|left, right| {
                left.requested_at
                    .cmp(&right.requested_at)
                    .then_with(|| left.connection_name.cmp(&right.connection_name))
            })
            .map(|pending| (pending.connection_name.clone(), pending.requested_at))
    }

    fn latest_pending_request_id(&self) -> Option<String> {
        self.pending
            .iter()
            .filter(|(_, pending)| pending.reconnect_session_id.is_none())
            .max_by(|(left_id, left), (right_id, right)| {
                left.requested_at
                    .cmp(&right.requested_at)
                    .then_with(|| left_id.cmp(right_id))
            })
            .map(|(request_id, _)| request_id.clone())
    }

    fn latest_failed_request_id(&self) -> Option<String> {
        self.failed
            .iter()
            .max_by(|(left_id, left), (right_id, right)| {
                left.pending
                    .requested_at
                    .cmp(&right.pending.requested_at)
                    .then_with(|| left_id.cmp(right_id))
            })
            .map(|(request_id, _)| request_id.clone())
    }
}

pub(super) fn pending_session_start_display_name(pending: &PendingSessionStart) -> String {
    pending
        .custom_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(&pending.connection_name)
        .to_string()
}

pub(super) fn failed_session_start_display_name(failed: &FailedSessionStart) -> String {
    pending_session_start_display_name(&failed.pending)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::Instant;

    use nyaterm_core::AiExecutionProfile;
    use nyaterm_transport::{SessionKind, SessionManager};

    use crate::features::runtime_jobs::SessionStartResult;
    use crate::models::{SessionEventBridge, TerminalFramePipeline};

    use super::{
        FailedSessionStart, PendingSessionStart, SessionFeatureState, SessionStartFeatureState,
    };

    fn pending(name: &str) -> PendingSessionStart {
        PendingSessionStart {
            connection_name: name.to_string(),
            launch_config: None,
            requested_at: Instant::now(),
            kind: SessionKind::LocalPty,
            ai_execution_profile: AiExecutionProfile::default(),
            custom_name: None,
            tab_color: None,
            after_session_id: None,
            insert_index: None,
            seed_output: None,
            startup_command: None,
            multiplex_key: None,
            source_connection_id: None,
            reconnect_session_id: None,
        }
    }

    #[test]
    fn session_state_owns_live_runtime_and_initializes_transient_state() {
        let manager = Arc::new(SessionManager::new());
        let event_bridge = SessionEventBridge::spawn(
            Arc::clone(&manager),
            TerminalFramePipeline::default(),
            "utf-8".to_string(),
            10_000,
        );
        let sessions = SessionFeatureState::new(Arc::clone(&manager), event_bridge);

        assert!(Arc::ptr_eq(&sessions.manager, &manager));
        assert!(!sessions.start.has_pending());
        assert!(!sessions.start.has_failed());
        assert!(sessions.command_history.is_empty());
        assert!(sessions.active_id.is_none());
        assert!(sessions.active_ssh_config.is_none());
        assert_eq!(
            sessions.active_ai_execution_profile,
            AiExecutionProfile::SendOnly
        );
        assert!(sessions.order.is_empty());
        assert!(sessions.metadata.is_empty());
        assert!(sessions.zmodem.is_empty());
        assert!(sessions.trzsz.is_empty());
        assert!(sessions.multiplex_handles.is_empty());
    }

    #[test]
    fn session_start_state_owns_channel_selection_and_cancellation() {
        let mut starts = SessionStartFeatureState::new();
        starts
            .pending
            .insert("request-1".to_string(), pending("local shell"));

        assert!(starts.select_pending("request-1"));
        assert_eq!(
            starts.pending_display_name().as_deref(),
            Some("local shell")
        );

        starts
            .sender()
            .send(SessionStartResult {
                request_id: "request-1".to_string(),
                connection_name: "local shell".to_string(),
                kind: SessionKind::LocalPty,
                worker_started_at: Instant::now(),
                worker_finished_at: Instant::now(),
                result: Err("cancelled".to_string()),
            })
            .expect("session start event channel should stay connected");
        assert_eq!(
            starts
                .try_recv()
                .expect("session start result should reach its owner")
                .request_id,
            "request-1"
        );

        let closed = starts
            .close_pending("request-1")
            .expect("selected pending start should close");
        assert_eq!(closed.connection_name, "local shell");
        assert!(starts.cancelled.contains("request-1"));
        assert!(!starts.has_pending());
        assert!(starts.active_pending.is_none());
    }

    #[test]
    fn closing_pending_starts_preserves_non_reconnect_and_failed_fallback_order() {
        let mut starts = SessionStartFeatureState::new();
        starts
            .pending
            .insert("request-active".to_string(), pending("active"));
        starts
            .pending
            .insert("request-normal".to_string(), pending("normal"));
        let mut reconnect = pending("reconnect");
        reconnect.reconnect_session_id = Some("old-session".to_string());
        starts
            .pending
            .insert("request-reconnect".to_string(), reconnect);
        starts.failed.insert(
            "request-failed".to_string(),
            FailedSessionStart {
                pending: pending("failed"),
                error: "failed".to_string(),
            },
        );

        assert!(starts.select_pending("request-active"));
        starts
            .close_pending("request-active")
            .expect("active start should close");
        assert_eq!(starts.active_pending.as_deref(), Some("request-normal"));
        assert!(starts.active_failed.is_none());

        starts
            .close_pending("request-normal")
            .expect("normal start should close");
        assert!(starts.active_pending.is_none());
        assert_eq!(starts.active_failed.as_deref(), Some("request-failed"));
        assert!(starts.pending.contains_key("request-reconnect"));
    }
}

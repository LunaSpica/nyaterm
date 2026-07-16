use super::*;

pub(in crate::features) struct TerminalRuntimeUiState {
    pub event_pump_started: bool,
    pub session_event_backlog_active: bool,
    pub session_event_queued_events: usize,
    pub session_event_queued_output_bytes: usize,
    pub session_event_dropped_output_bytes: u64,
    pub session_event_last_output_event_count: usize,
    pub session_event_last_drained_output_bytes: usize,
    pub last_session_start_drain_duration: Duration,
    pub last_store_snapshot_publish_at: Option<Instant>,
    pub last_pending_session_status_at: Option<Instant>,
    pub last_terminal_resize_at: Option<Instant>,
    pub last_terminal_frame_apply_at: Option<Instant>,
    /// After connect success, demote idle/visual work until this time (no faster tick).
    pub connect_settle_until: Option<Instant>,
    /// Open-tabs / window-layout settings need a durable write.
    pub open_tabs_persist_dirty: bool,
    pub window_layout_persist_dirty: bool,
    pub cursor_blink_on: bool,
    pub cursor_blink_tick: u8,
    pub visual_bell_ticks: u8,
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
}

#[derive(Clone, Default)]
pub(in crate::features) struct SavedConnectionStartOptions {
    pub custom_name: Option<String>,
    pub tab_color: Option<u32>,
    pub after_session_id: Option<String>,
    pub insert_index: Option<usize>,
    pub seed_output: Option<String>,
    pub startup_command: Option<StartupCommandRequest>,
}

#[derive(Clone)]
pub(in crate::features) struct PendingSavedConnectionStart {
    pub connection: SavedConnection,
    pub options: SavedConnectionStartOptions,
}

impl Default for TerminalRuntimeUiState {
    fn default() -> Self {
        Self {
            event_pump_started: false,
            session_event_backlog_active: false,
            session_event_queued_events: 0,
            session_event_queued_output_bytes: 0,
            session_event_dropped_output_bytes: 0,
            session_event_last_output_event_count: 0,
            session_event_last_drained_output_bytes: 0,
            last_session_start_drain_duration: Duration::ZERO,
            last_store_snapshot_publish_at: None,
            last_pending_session_status_at: None,
            last_terminal_resize_at: None,
            last_terminal_frame_apply_at: None,
            connect_settle_until: None,
            open_tabs_persist_dirty: false,
            window_layout_persist_dirty: false,
            cursor_blink_on: true,
            cursor_blink_tick: 0,
            visual_bell_ticks: 0,
        }
    }
}

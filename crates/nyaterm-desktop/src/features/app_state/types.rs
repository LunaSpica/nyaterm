use super::*;

pub(in crate::features) struct TerminalRuntimeUiState {
    pub event_pump_started: bool,
    pub session_event_backlog_active: bool,
    pub session_event_queued_events: usize,
    pub session_event_queued_output_bytes: usize,
    pub session_event_dropped_output_bytes: u64,
    pub session_event_last_drained_output_bytes: usize,
    pub last_terminal_resize_at: Option<Instant>,
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
    pub launch_config: SessionLaunchConfig,
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

impl Default for TerminalRuntimeUiState {
    fn default() -> Self {
        Self {
            event_pump_started: false,
            session_event_backlog_active: false,
            session_event_queued_events: 0,
            session_event_queued_output_bytes: 0,
            session_event_dropped_output_bytes: 0,
            session_event_last_drained_output_bytes: 0,
            last_terminal_resize_at: None,
            cursor_blink_on: true,
            cursor_blink_tick: 0,
            visual_bell_ticks: 0,
        }
    }
}

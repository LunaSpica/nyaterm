use std::collections::HashSet;
use std::time::{Duration, Instant};

use nyaterm_core::{
    AiSettings, AppSettingsSummary, CloudSyncSettings, KeywordHighlightConfig, SavedConnection,
    TranslationSettings,
};

use crate::models::{CloudSyncSecretDraft, StartupCommandRequest, TranslationSecretDraft};

#[derive(Debug, Clone)]
pub(in crate::features) struct SettingsDraftSnapshot {
    pub settings: AppSettingsSummary,
    pub ai_settings: AiSettings,
    pub ai_model_draft: String,
    pub ai_base_url_draft: String,
    pub ai_secret_draft: String,
    pub cloud_sync_settings: CloudSyncSettings,
    pub cloud_sync_secret_draft: CloudSyncSecretDraft,
    pub translation_settings: TranslationSettings,
    pub translation_secret_draft: TranslationSecretDraft,
    pub keyword_highlights: KeywordHighlightConfig,
    pub master_password_enabled: bool,
    pub master_password_draft: String,
}

pub(in crate::features) struct TerminalRuntimeUiState {
    pub event_pump_started: bool,
    pub session_event_backlog_active: bool,
    pub session_event_queued_events: usize,
    pub session_event_queued_output_bytes: usize,
    pub session_event_dropped_output_bytes: u64,
    pub session_event_last_output_event_count: usize,
    pub session_event_last_drained_output_bytes: usize,
    pub last_session_start_drain_duration: Duration,
    pub last_pending_session_status_at: Option<Instant>,
    pub last_terminal_resize_at: Option<Instant>,
    pub last_terminal_frame_apply_at: Option<Instant>,
    /// Last user-driven terminal scroll input. During this short window the
    /// terminal paint path favors text/position over enhanced decorations.
    pub last_terminal_user_scroll_at: Option<Instant>,
    /// Last successful user terminal input write. During this short window the
    /// terminal paint path favors low-latency echo over enhanced decorations.
    pub last_terminal_input_at: Option<Instant>,
    /// Sessions whose scroll position changed and should repaint on the next frame tick.
    pub pending_terminal_scroll_position_sessions: HashSet<String>,
    /// Sessions already scrolled locally by TerminalSurface; only snapshot requests
    /// should run on the app sync tick.
    pub pending_terminal_scroll_snapshot_only_sessions: HashSet<String>,
    /// Sessions whose position changed again after the immediate scroll repaint.
    pub pending_terminal_scroll_position_repaint_sessions: HashSet<String>,
    /// True while a frame-coalesced scroll-position repaint task is armed.
    pub terminal_scroll_position_notify_armed: bool,
    /// Sessions whose scrollbar drag position changed and should repaint soon.
    pub pending_terminal_scrollbar_drag_sessions: HashSet<String>,
    /// True while a coalesced scrollbar-drag visual repaint task is armed.
    pub terminal_scrollbar_drag_notify_armed: bool,
    /// Sessions whose selection drag changed and should repaint soon.
    pub pending_terminal_selection_drag_sessions: HashSet<String>,
    /// True while a coalesced selection-drag visual repaint task is armed.
    pub terminal_selection_drag_notify_armed: bool,
    /// Sessions that need a full decoration repaint once user scrolling idles.
    pub pending_terminal_user_scroll_idle_sessions: HashSet<String>,
    /// True while a delayed scroll-idle repaint task is armed.
    pub terminal_user_scroll_idle_notify_armed: bool,
    /// Sessions that need a full decoration repaint once typing idles.
    pub pending_terminal_input_idle_sessions: HashSet<String>,
    /// True while a delayed input-idle repaint task is armed.
    pub terminal_input_idle_notify_armed: bool,
    /// After connect success, demote idle/visual work until this time (no faster tick).
    pub connect_settle_until: Option<Instant>,
    /// A short post-input pump task is armed to drain echo output/frame events.
    pub terminal_input_wake_armed: bool,
    /// Incremented on every user input write so an armed wake can extend itself.
    pub terminal_input_wake_generation: u64,
    /// Last full-shell cx.notify from the runtime tick (paint throttle).
    pub last_ui_notify_at: Option<Instant>,
    /// A visual update was deferred by paint throttle and still needs a notify.
    pub pending_ui_notify: bool,
    /// Full NyaTermApp shell paints (chrome + workspace structure).
    pub full_shell_paint_count: u64,
    /// Output frames that notified only a TerminalSurface.
    pub terminal_surface_frame_notify_count: u64,
    /// Output frames that also dirtied chrome (unread/effects).
    pub terminal_chrome_frame_notify_count: u64,
    /// Last periodic terminal performance heartbeat.
    pub last_terminal_perf_heartbeat_at: Option<Instant>,
    pub last_perf_full_shell_paint_count: u64,
    pub last_perf_surface_paint_count: u64,
    pub last_perf_surface_frame_notify_count: u64,
    pub last_perf_chrome_frame_notify_count: u64,
    pub last_perf_layout_cache_hits: u64,
    pub last_perf_layout_cache_misses: u64,
    /// Open-tabs / window-layout settings need a durable write.
    pub open_tabs_persist_dirty: bool,
    pub window_layout_persist_dirty: bool,
    pub cursor_blink_on: bool,
    pub cursor_blink_next_at: Option<Instant>,
    pub visual_bell_ticks: u8,
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
            last_pending_session_status_at: None,
            last_terminal_resize_at: None,
            last_terminal_frame_apply_at: None,
            last_terminal_user_scroll_at: None,
            last_terminal_input_at: None,
            pending_terminal_scroll_position_sessions: HashSet::new(),
            pending_terminal_scroll_snapshot_only_sessions: HashSet::new(),
            pending_terminal_scroll_position_repaint_sessions: HashSet::new(),
            terminal_scroll_position_notify_armed: false,
            pending_terminal_scrollbar_drag_sessions: HashSet::new(),
            terminal_scrollbar_drag_notify_armed: false,
            pending_terminal_selection_drag_sessions: HashSet::new(),
            terminal_selection_drag_notify_armed: false,
            pending_terminal_user_scroll_idle_sessions: HashSet::new(),
            terminal_user_scroll_idle_notify_armed: false,
            pending_terminal_input_idle_sessions: HashSet::new(),
            terminal_input_idle_notify_armed: false,
            connect_settle_until: None,
            terminal_input_wake_armed: false,
            terminal_input_wake_generation: 0,
            last_ui_notify_at: None,
            pending_ui_notify: false,
            full_shell_paint_count: 0,
            terminal_surface_frame_notify_count: 0,
            terminal_chrome_frame_notify_count: 0,
            last_terminal_perf_heartbeat_at: None,
            last_perf_full_shell_paint_count: 0,
            last_perf_surface_paint_count: 0,
            last_perf_surface_frame_notify_count: 0,
            last_perf_chrome_frame_notify_count: 0,
            last_perf_layout_cache_hits: 0,
            last_perf_layout_cache_misses: 0,
            open_tabs_persist_dirty: false,
            window_layout_persist_dirty: false,
            cursor_blink_on: true,
            cursor_blink_next_at: None,
            visual_bell_ticks: 0,
        }
    }
}

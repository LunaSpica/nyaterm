//! Grouped terminal feature state.
//!
//! This is presentation state only: which terminals exist, what the user has
//! selected, where the surface was painted. Parsing, snapshots and the wire
//! protocol stay in `nyaterm-terminal` and `nyaterm-transport`.

use std::collections::HashMap;

use gpui::{Entity, FocusHandle, Subscription};
use nyaterm_core::TerminalInputState as CommandInputState;
use nyaterm_terminal::{TerminalOutputDecoder, TerminalScreen};

use super::terminal_surface_entity::TerminalSurface;
use crate::features::app_state::TerminalRuntimeUiState;
use crate::models::{
    CommandSuggestionState, CredentialAutofillMatchPipeline, CredentialAutofillMatchRequestKey,
    CredentialSuggestionState, PendingCredentialAutofill, RecordingHistorySearchEvent,
    RecordingHistorySearchKey, TabDockZone, TerminalContextMenuState, TerminalFramePipeline,
    TerminalSearchMode, TerminalSelection, TerminalViewState, TerminalWindowNode,
};

pub(in crate::features) struct TerminalFeatureState {
    pub search: TerminalSearchState,
    pub view: TerminalViewRuntimeState,
    pub input: TerminalInputState,
    pub assist: TerminalAssistState,
    pub selection: TerminalSelectionState,
    pub layout: TerminalLayoutState,
    pub menus: TerminalMenuState,
    pub windows: TerminalWindowState,
}

/// Focus handles the terminal feature needs at construction time.
pub(in crate::features) struct TerminalFeatureFocus {
    pub actions: FocusHandle,
    pub x11_display: FocusHandle,
    pub terminal: FocusHandle,
}

/// In-terminal find bar and recording history search.
pub(in crate::features) struct TerminalSearchState {
    pub open: bool,
    pub query: String,
    pub mode: TerminalSearchMode,
    pub case_sensitive: bool,
    pub regex: bool,
    pub whole_word: bool,
    pub active_index: usize,
    pub history_pending_key: Option<RecordingHistorySearchKey>,
    pub history_result: Option<RecordingHistorySearchEvent>,
}

/// Live terminal views, their surfaces, and the frame/scroll pipeline.
pub(in crate::features) struct TerminalViewRuntimeState {
    pub views: HashMap<String, TerminalViewState>,
    /// Per-session terminal grid entities (frame notify isolation).
    pub surfaces: HashMap<String, Entity<TerminalSurface>>,
    pub output: String,
    pub output_decoder: TerminalOutputDecoder,
    pub screen: TerminalScreen,
    pub frame_pipeline: TerminalFramePipeline,
    pub live_prefetch_generation: u64,
    pub live_prefetch_task: Option<gpui::Task<()>>,
    pub scroll_offset: usize,
    pub scroll_delta_residuals: HashMap<String, f32>,
    pub scrollbar_dragging: bool,
    pub scrollbar_drag_session_id: Option<String>,
    pub status: String,
    pub runtime: TerminalRuntimeUiState,
}

/// Keyboard focus and IME composition for the terminal surface.
pub(in crate::features) struct TerminalInputState {
    pub focus: FocusHandle,
    pub focus_active: bool,
    pub focus_subscriptions: Vec<Subscription>,
    pub ime_marked_text: String,
}

/// Inline command completion and terminal-output credential prompt assistance.
///
/// These states share the terminal input lifecycle: session switches, terminal
/// mode changes, and settings updates reset them together. The credential
/// matcher remains a background pipeline and never runs in a render path.
pub(in crate::features) struct TerminalAssistState {
    pub command_suggestions: Option<CommandSuggestionState>,
    pub command_input_tracker: CommandInputState,
    pub command_suggestions_suppressed: bool,
    pub pending_command_history_entry: Option<String>,
    pub command_suggestion_search_gen: u64,
    pub command_suggestion_refresh_task: Option<gpui::Task<()>>,
    pub credential_suggestions: Option<CredentialSuggestionState>,
    pub credential_autofill_buffer: String,
    pub credential_autofill_recent: HashMap<String, u64>,
    pub credential_autofill_pending: Option<PendingCredentialAutofill>,
    pub credential_autofill_detection_pending: bool,
    pub credential_autofill_next_request_id: u64,
    pub credential_autofill_pending_request: Option<CredentialAutofillMatchRequestKey>,
    pub credential_autofill_match_pipeline: CredentialAutofillMatchPipeline,
    pub credential_autofill_sending: bool,
    pub credential_prompt_input_until_ms: u64,
}

/// Text selection and mouse reporting.
pub(in crate::features) struct TerminalSelectionState {
    pub selection: Option<TerminalSelection>,
    pub session_id: Option<String>,
    pub dragging: bool,
    pub mouse_report_button: Option<u8>,
    pub mouse_report_session_id: Option<String>,
    pub mouse_report_peer_session_ids: Vec<String>,
    pub mouse_report_position: Option<(u16, u16)>,
}

/// Last painted geometry, used to map pointer positions onto cells.
pub(in crate::features) struct TerminalLayoutState {
    /// Last painted bounds of the active terminal text area (window coords).
    pub surface_bounds: Option<gpui::Bounds<gpui::Pixels>>,
    pub session_surface_bounds: HashMap<String, gpui::Bounds<gpui::Pixels>>,
    pub scale_factor: f32,
    pub cell_metrics: Option<(f32, f32)>,
}

/// Terminal actions overlay and context menu.
pub(in crate::features) struct TerminalMenuState {
    pub actions_open: bool,
    pub actions_focus: FocusHandle,
    pub context_menu: Option<TerminalContextMenuState>,
}

/// Split/tab window tree and drag-and-drop targets over it.
pub(in crate::features) struct TerminalWindowState {
    pub tree: Option<TerminalWindowNode>,
    pub drop: Option<(String, TabDockZone)>,
    /// Whether we already attempted startup restore of multi-leaf layout.
    pub restored: bool,
    pub file_drop_hover: Option<String>,
}

impl TerminalFeatureState {
    #[allow(clippy::too_many_arguments)]
    pub(in crate::features) fn new(
        screen: TerminalScreen,
        output_decoder: TerminalOutputDecoder,
        frame_pipeline: TerminalFramePipeline,
        output: String,
        status: String,
        scale_factor: f32,
        focus: TerminalFeatureFocus,
    ) -> Self {
        Self {
            search: TerminalSearchState {
                open: false,
                query: String::new(),
                mode: TerminalSearchMode::Buffer,
                case_sensitive: false,
                regex: false,
                whole_word: false,
                active_index: 0,
                history_pending_key: None,
                history_result: None,
            },
            view: TerminalViewRuntimeState {
                views: HashMap::new(),
                surfaces: HashMap::new(),
                output,
                output_decoder,
                screen,
                frame_pipeline,
                live_prefetch_generation: 0,
                live_prefetch_task: None,
                scroll_offset: 0,
                scroll_delta_residuals: HashMap::new(),
                scrollbar_dragging: false,
                scrollbar_drag_session_id: None,
                status,
                runtime: TerminalRuntimeUiState::default(),
            },
            input: TerminalInputState {
                focus: focus.terminal,
                focus_active: false,
                focus_subscriptions: Vec::new(),
                ime_marked_text: String::new(),
            },
            assist: TerminalAssistState::new(),
            selection: TerminalSelectionState {
                selection: None,
                session_id: None,
                dragging: false,
                mouse_report_button: None,
                mouse_report_session_id: None,
                mouse_report_peer_session_ids: Vec::new(),
                mouse_report_position: None,
            },
            layout: TerminalLayoutState {
                surface_bounds: None,
                session_surface_bounds: HashMap::new(),
                scale_factor,
                cell_metrics: None,
            },
            menus: TerminalMenuState {
                actions_open: false,
                actions_focus: focus.actions,
                context_menu: None,
            },
            windows: TerminalWindowState {
                tree: None,
                drop: None,
                restored: false,
                file_drop_hover: None,
            },
        }
    }
}

impl TerminalAssistState {
    fn new() -> Self {
        Self {
            command_suggestions: None,
            command_input_tracker: CommandInputState::new(),
            command_suggestions_suppressed: false,
            pending_command_history_entry: None,
            command_suggestion_search_gen: 0,
            command_suggestion_refresh_task: None,
            credential_suggestions: None,
            credential_autofill_buffer: String::new(),
            credential_autofill_recent: HashMap::new(),
            credential_autofill_pending: None,
            credential_autofill_detection_pending: false,
            credential_autofill_next_request_id: 0,
            credential_autofill_pending_request: None,
            credential_autofill_match_pipeline: CredentialAutofillMatchPipeline::spawn(),
            credential_autofill_sending: false,
            credential_prompt_input_until_ms: 0,
        }
    }

    pub(in crate::features) fn clear_command_tracking(&mut self) {
        self.command_suggestions = None;
        self.command_input_tracker = CommandInputState::new();
        self.command_suggestions_suppressed = false;
        self.pending_command_history_entry = None;
    }

    pub(in crate::features) fn invalidate_command_suggestion_search(&mut self) {
        self.command_suggestion_search_gen = self.command_suggestion_search_gen.saturating_add(1);
    }

    pub(in crate::features) fn reset_for_session_switch(&mut self) {
        self.credential_suggestions = None;
        self.credential_autofill_buffer.clear();
        self.credential_autofill_recent.clear();
        self.credential_autofill_pending = None;
        self.credential_autofill_sending = false;
        self.credential_prompt_input_until_ms = 0;
        self.clear_command_tracking();
        self.invalidate_command_suggestion_search();
    }

    pub(in crate::features) fn dismiss_credential_suggestions(&mut self) -> bool {
        let had_panel = self.credential_suggestions.take().is_some();
        self.credential_autofill_buffer.clear();
        self.credential_autofill_recent.clear();
        self.credential_autofill_detection_pending = false;
        self.credential_autofill_pending_request = None;
        had_panel
    }

    pub(in crate::features) fn credential_prompt_input_mode(&self, now_ms: u64) -> bool {
        self.credential_prompt_input_until_ms > now_ms
    }
}

#[cfg(test)]
mod tests {
    use nyaterm_core::TerminalInputState as CommandInputState;

    use super::TerminalAssistState;

    #[test]
    fn session_switch_reset_clears_terminal_assist_transients() {
        let mut state = TerminalAssistState::new();
        state.command_input_tracker.value = "git status".to_string();
        state.command_suggestions_suppressed = true;
        state.pending_command_history_entry = Some("git status".to_string());
        state.credential_autofill_buffer = "login:".to_string();
        state
            .credential_autofill_recent
            .insert("username:login:".to_string(), 42);
        state.credential_autofill_sending = true;
        state.credential_prompt_input_until_ms = 99;
        let search_generation = state.command_suggestion_search_gen;

        state.reset_for_session_switch();

        assert_eq!(state.command_input_tracker, CommandInputState::new());
        assert!(!state.command_suggestions_suppressed);
        assert!(state.pending_command_history_entry.is_none());
        assert!(state.credential_autofill_buffer.is_empty());
        assert!(state.credential_autofill_recent.is_empty());
        assert!(!state.credential_autofill_sending);
        assert_eq!(state.credential_prompt_input_until_ms, 0);
        assert_eq!(
            state.command_suggestion_search_gen,
            search_generation.saturating_add(1)
        );
    }
}

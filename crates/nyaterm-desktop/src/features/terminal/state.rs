//! Grouped terminal feature state.
//!
//! This is presentation state only: which terminals exist, what the user has
//! selected, where the surface was painted. Parsing, snapshots and the wire
//! protocol stay in `nyaterm-terminal` and `nyaterm-transport`.

use std::collections::{HashMap, VecDeque};
use std::ops::Range;
use std::sync::Arc;
use std::time::Instant;

use gpui::{Entity, FocusHandle, Subscription};
use nyaterm_core::{ResolvedKeywordHighlightRule, TerminalInputState as CommandInputState};
use nyaterm_terminal::{TerminalOutputDecoder, TerminalScreen};

use super::terminal_surface_entity::TerminalSurface;
use crate::features::app_state::TerminalRuntimeUiState;
use crate::models::{
    ActionLinkMenuState, ActionLinkTooltipState, CommandSuggestionState,
    CredentialAutofillMatchPipeline, CredentialAutofillMatchRequestKey, CredentialSuggestionState,
    MultiLinePasteDraft, PendingCredentialAutofill, RecordingHistorySearchEvent,
    RecordingHistorySearchKey, TabDockZone, TerminalContextMenuState, TerminalFrameEvent,
    TerminalFramePipeline, TerminalSearchMode, TerminalSelection, TerminalViewState,
    TerminalWindowNode, normalize_paste_newlines,
};
use crate::theme::ThemePalette;

pub(in crate::features) struct TerminalFeatureState {
    pub search: TerminalSearchState,
    pub view: TerminalViewRuntimeState,
    pub input: TerminalInputState,
    pub paste: TerminalPasteReviewState,
    pub assist: TerminalAssistState,
    pub selection: TerminalSelectionState,
    pub layout: TerminalLayoutState,
    pub menus: TerminalMenuState,
    pub paint: TerminalPaintCacheState,
    pub windows: TerminalWindowState,
}

/// Focus handles the terminal feature needs at construction time.
pub(in crate::features) struct TerminalFeatureFocus {
    pub actions: FocusHandle,
    pub x11_display: FocusHandle,
    pub terminal: FocusHandle,
    pub paste: FocusHandle,
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
    pub pending_frame_events: VecDeque<TerminalFrameEvent>,
}

/// Keyboard focus and IME composition for the terminal surface.
pub(in crate::features) struct TerminalInputState {
    pub focus: FocusHandle,
    pub focus_active: bool,
    pub focus_subscriptions: Vec<Subscription>,
    pub ime_marked_text: String,
}

/// Dedicated multi-line paste editor state.
///
/// This remains separate from registry-backed single-line inputs because it
/// owns a byte cursor, selection anchor and IME composition range.
pub(in crate::features) struct TerminalPasteReviewState {
    pub draft: Option<MultiLinePasteDraft>,
    pub marked_text: String,
    pub marked_range: Option<Range<usize>>,
    pub cursor: usize,
    pub anchor: Option<usize>,
    pub focus: FocusHandle,
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
    pub action_link_menu: Option<ActionLinkMenuState>,
    pub action_link_tooltip: Option<ActionLinkTooltipState>,
    /// Pending action-link hover (Tauri 250ms delay before showing tooltip).
    pub action_link_hover_pending: Option<(String, Instant, ActionLinkTooltipState)>,
}

/// Paint-time caches invalidated whenever appearance settings change.
pub(in crate::features) struct TerminalPaintCacheState {
    pub cached_terminal_theme_palette: Option<(String, String, String, ThemePalette)>,
    pub cached_keyword_highlight_rules: Option<Arc<Vec<ResolvedKeywordHighlightRule>>>,
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
                pending_frame_events: VecDeque::new(),
            },
            input: TerminalInputState {
                focus: focus.terminal,
                focus_active: false,
                focus_subscriptions: Vec::new(),
                ime_marked_text: String::new(),
            },
            paste: TerminalPasteReviewState::new(focus.paste),
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
                action_link_menu: None,
                action_link_tooltip: None,
                action_link_hover_pending: None,
            },
            paint: TerminalPaintCacheState {
                cached_terminal_theme_palette: None,
                cached_keyword_highlight_rules: None,
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

impl TerminalPasteReviewState {
    fn new(focus: FocusHandle) -> Self {
        Self {
            draft: None,
            marked_text: String::new(),
            marked_range: None,
            cursor: 0,
            anchor: None,
            focus,
        }
    }

    pub(in crate::features) fn open(&mut self, text: String) {
        let text = normalize_paste_newlines(&text);
        self.cursor = text.len();
        self.anchor = None;
        self.marked_range = None;
        self.draft = Some(MultiLinePasteDraft::new(text));
        self.marked_text.clear();
    }

    pub(in crate::features) fn clear(&mut self) {
        self.draft = None;
        self.reset_editing_state();
    }

    pub(in crate::features) fn take_normalized_text(&mut self) -> Option<String> {
        let text = self.draft.take().map(|draft| draft.normalized_text());
        self.reset_editing_state();
        text
    }

    pub(in crate::features) fn text(&self) -> &str {
        self.draft
            .as_ref()
            .map(|draft| draft.text.as_str())
            .unwrap_or_default()
    }

    pub(in crate::features) fn selected_byte_range(&self) -> Range<usize> {
        let cursor = floor_char_boundary(self.text(), self.cursor);
        let anchor = floor_char_boundary(self.text(), self.anchor.unwrap_or(cursor));
        if anchor <= cursor {
            anchor..cursor
        } else {
            cursor..anchor
        }
    }

    pub(in crate::features) fn select_all(&mut self) {
        self.anchor = Some(0);
        self.cursor = self.text().len();
        self.clear_marked_text();
    }

    pub(in crate::features) fn previous_char_boundary(&self) -> usize {
        previous_char_boundary(self.text(), self.cursor)
    }

    pub(in crate::features) fn next_char_boundary(&self) -> usize {
        next_char_boundary(self.text(), self.cursor)
    }

    pub(in crate::features) fn current_line_start(&self) -> usize {
        line_start(self.text(), self.cursor)
    }

    pub(in crate::features) fn current_line_end(&self) -> usize {
        line_end(self.text(), self.cursor)
    }

    pub(in crate::features) fn move_cursor(&mut self, cursor: usize, extend: bool) {
        let cursor = floor_char_boundary(self.text(), cursor);
        if extend {
            self.anchor.get_or_insert(self.cursor);
        } else {
            self.anchor = None;
        }
        self.cursor = cursor;
        self.clear_marked_text();
    }

    pub(in crate::features) fn move_vertical(&mut self, delta: isize, extend: bool) {
        let text = self.text();
        let cursor = floor_char_boundary(text, self.cursor);
        let current_start = line_start(text, cursor);
        let column = text[current_start..cursor].chars().count();
        let target_start = if delta < 0 {
            if current_start == 0 {
                0
            } else {
                line_start(text, current_start - 1)
            }
        } else {
            let current_end = line_end(text, cursor);
            if current_end >= text.len() {
                current_start
            } else {
                current_end + 1
            }
        };
        let target_end = line_end(text, target_start);
        let target = text[target_start..target_end]
            .char_indices()
            .nth(column)
            .map(|(offset, _)| target_start + offset)
            .unwrap_or(target_end);
        self.move_cursor(target, extend);
    }

    pub(in crate::features) fn replace_selection(&mut self, text: &str) -> bool {
        self.replace_range(self.selected_byte_range(), text)
    }

    pub(in crate::features) fn replace_range(&mut self, range: Range<usize>, text: &str) -> bool {
        let Some(draft) = self.draft.as_mut() else {
            return false;
        };
        let start = floor_char_boundary(&draft.text, range.start);
        let end = floor_char_boundary(&draft.text, range.end).max(start);
        draft.text.replace_range(start..end, text);
        self.cursor = start + text.len();
        self.anchor = None;
        self.clear_marked_text();
        true
    }

    fn reset_editing_state(&mut self) {
        self.marked_text.clear();
        self.marked_range = None;
        self.cursor = 0;
        self.anchor = None;
    }

    fn clear_marked_text(&mut self) {
        self.marked_text.clear();
        self.marked_range = None;
    }
}

fn floor_char_boundary(text: &str, offset: usize) -> usize {
    let mut offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

fn previous_char_boundary(text: &str, offset: usize) -> usize {
    let offset = floor_char_boundary(text, offset);
    text[..offset]
        .char_indices()
        .next_back()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn next_char_boundary(text: &str, offset: usize) -> usize {
    let offset = floor_char_boundary(text, offset);
    text[offset..]
        .chars()
        .next()
        .map(|ch| offset + ch.len_utf8())
        .unwrap_or(offset)
}

fn line_start(text: &str, offset: usize) -> usize {
    text[..floor_char_boundary(text, offset)]
        .rfind('\n')
        .map(|index| index + 1)
        .unwrap_or(0)
}

fn line_end(text: &str, offset: usize) -> usize {
    let offset = floor_char_boundary(text, offset);
    text[offset..]
        .find('\n')
        .map(|index| offset + index)
        .unwrap_or(text.len())
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
    use gpui::TestAppContext;
    use nyaterm_core::TerminalInputState as CommandInputState;

    use super::{TerminalAssistState, TerminalPasteReviewState};

    fn paste_state() -> TerminalPasteReviewState {
        let cx = TestAppContext::single();
        let focus = cx.update(|cx| cx.focus_handle());
        TerminalPasteReviewState::new(focus)
    }

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

    #[test]
    fn paste_cursor_operations_stay_on_utf8_boundaries() {
        let mut state = paste_state();
        state.open("a你🙂b".to_string());

        state.move_cursor(2, false);
        assert_eq!(state.cursor, 1);
        assert_eq!(state.next_char_boundary(), 4);

        state.move_cursor(4, false);
        assert_eq!(state.previous_char_boundary(), 1);
        assert_eq!(state.next_char_boundary(), 8);
    }

    #[test]
    fn paste_selection_replacement_resets_selection_and_ime_state() {
        let mut state = paste_state();
        state.open("alpha\nβeta".to_string());
        state.move_cursor(0, false);
        state.move_cursor(5, true);
        state.marked_text = "composition".to_string();
        state.marked_range = Some(0..5);

        assert_eq!(state.selected_byte_range(), 0..5);
        assert!(state.replace_selection("替换"));

        assert_eq!(state.text(), "替换\nβeta");
        assert_eq!(state.cursor, "替换".len());
        assert!(state.anchor.is_none());
        assert!(state.marked_text.is_empty());
        assert!(state.marked_range.is_none());
    }

    #[test]
    fn paste_vertical_movement_preserves_character_column() {
        let mut state = paste_state();
        state.open("ab\n你cde\nz".to_string());

        state.move_cursor(7, false);
        state.move_vertical(-1, false);
        assert_eq!(state.cursor, 2);

        state.move_vertical(1, true);
        assert_eq!(state.cursor, 7);
        assert_eq!(state.anchor, Some(2));
        assert_eq!(state.selected_byte_range(), 2..7);
    }

    #[test]
    fn taking_or_clearing_paste_draft_resets_editor_transients() {
        let mut state = paste_state();
        state.open("first\r\nsecond\rthird".to_string());
        state.marked_text = "ime".to_string();
        state.marked_range = Some(0..3);
        state.anchor = Some(1);

        assert_eq!(
            state.take_normalized_text().as_deref(),
            Some("first\nsecond\nthird")
        );
        assert!(state.draft.is_none());
        assert_eq!(state.cursor, 0);
        assert!(state.anchor.is_none());
        assert!(state.marked_text.is_empty());
        assert!(state.marked_range.is_none());

        state.open("another draft".to_string());
        state.clear();
        assert!(state.draft.is_none());
        assert_eq!(state.cursor, 0);
    }
}

//! Grouped terminal feature state.
//!
//! This is presentation state only: which terminals exist, what the user has
//! selected, where the surface was painted. Parsing, snapshots and the wire
//! protocol stay in `nyaterm-terminal` and `nyaterm-transport`.

use std::collections::HashMap;

use gpui::{Entity, FocusHandle, Subscription};
use nyaterm_terminal::{TerminalOutputDecoder, TerminalScreen};

use super::terminal_surface_entity::TerminalSurface;
use crate::features::app_state::TerminalRuntimeUiState;
use crate::models::{
    RecordingHistorySearchEvent, RecordingHistorySearchKey, TabDockZone, TerminalContextMenuState,
    TerminalFramePipeline, TerminalSearchMode, TerminalSelection, TerminalViewState,
    TerminalWindowNode,
};

pub(in crate::features) struct TerminalFeatureState {
    pub search: TerminalSearchState,
    pub view: TerminalViewRuntimeState,
    pub input: TerminalInputState,
    pub selection: TerminalSelectionState,
    pub layout: TerminalLayoutState,
    pub menus: TerminalMenuState,
    pub windows: TerminalWindowState,
}

/// Focus handles the terminal feature needs at construction time.
pub(in crate::features) struct TerminalFeatureFocus {
    pub search: FocusHandle,
    pub actions: FocusHandle,
    pub x11_display: FocusHandle,
    pub terminal: FocusHandle,
}

/// In-terminal find bar and recording history search.
pub(in crate::features) struct TerminalSearchState {
    pub open: bool,
    pub query: String,
    pub focus: FocusHandle,
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
                focus: focus.search,
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

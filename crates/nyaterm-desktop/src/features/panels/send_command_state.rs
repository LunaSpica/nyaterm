//! Grouped send-command bar state.
//!
//! The bar composes one payload, chooses how to send it, and then reports
//! progress while sending. Those are three distinct phases and the twenty-four
//! `send_command_*` fields interleaved them.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use gpui::FocusHandle;

use crate::send_command::{
    SendCommandControlFocus, SendCommandDataType, SendCommandLineEnding, SendCommandMode,
    SendCommandTarget,
};

pub(in crate::features) struct SendCommandFeatureState {
    pub composer: SendCommandComposerState,
    pub options: SendCommandOptionsState,
    pub progress: SendCommandProgressState,
}

/// Focus handles the send-command bar needs at construction time.
pub(in crate::features) struct SendCommandFeatureFocus {
    pub editor: FocusHandle,
    pub controls: FocusHandle,
}

/// The payload being composed and where the caret is.
pub(in crate::features) struct SendCommandComposerState {
    pub draft: String,
    pub focus: FocusHandle,
    pub controls_focus: FocusHandle,
    pub control_focus: Option<SendCommandControlFocus>,
    pub hex_scroll_x: f32,
    pub hex_scroll_y: f32,
}

/// How the payload is interpreted and delivered, plus the menus that set it.
pub(in crate::features) struct SendCommandOptionsState {
    pub data_type: SendCommandDataType,
    pub mode: SendCommandMode,
    pub line_ending: SendCommandLineEnding,
    pub target: SendCommandTarget,
    pub count: Option<u32>,
    pub count_input: String,
    pub interval_seconds: f64,
    pub interval_input: String,
    pub data_menu_open: bool,
    pub mode_menu_open: bool,
    pub target_menu_open: bool,
    pub line_ending_menu_open: bool,
}

/// In-flight send: cancellation flag and the counters shown in the bar.
pub(in crate::features) struct SendCommandProgressState {
    pub sending: bool,
    pub cancel: Option<Arc<AtomicBool>>,
    pub completed: u32,
    pub total: u32,
    pub round: u32,
    pub rounds: u32,
}

impl SendCommandFeatureState {
    pub(in crate::features) fn new(focus: SendCommandFeatureFocus) -> Self {
        Self {
            composer: SendCommandComposerState {
                draft: String::new(),
                focus: focus.editor,
                controls_focus: focus.controls,
                control_focus: None,
                hex_scroll_x: 0.,
                hex_scroll_y: 0.,
            },
            options: SendCommandOptionsState {
                data_type: SendCommandDataType::Text,
                mode: SendCommandMode::Line,
                line_ending: SendCommandLineEnding::Crlf,
                target: SendCommandTarget::Current,
                count: Some(1),
                count_input: "1".to_string(),
                interval_seconds: 1.0,
                interval_input: "1.00".to_string(),
                data_menu_open: false,
                mode_menu_open: false,
                target_menu_open: false,
                line_ending_menu_open: false,
            },
            progress: SendCommandProgressState {
                sending: false,
                cancel: None,
                completed: 0,
                total: 0,
                round: 0,
                rounds: 0,
            },
        }
    }
}

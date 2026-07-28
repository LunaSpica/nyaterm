//! Authoritative transient state for the application shell.
//!
//! Shell rendering remains on `NyaTermApp` views. This state owns interaction
//! lifecycles that span those views so the composition root does not retain
//! independently mutable mirrors.

use gpui::Pixels;

use crate::models::{BottomPanelMode, BottomPanelResizeState};

pub(in crate::features) struct ShellFeatureState {
    pub bottom_panel: ShellBottomPanelState,
}

pub(in crate::features) struct ShellBottomPanelState {
    pub mode: BottomPanelMode,
    pub quick_commands_height: f32,
    pub command_send_height: f32,
    pub resize: Option<BottomPanelResizeState>,
}

impl ShellFeatureState {
    pub(in crate::features) fn new(
        bottom_panel_mode: BottomPanelMode,
        quick_commands_height: f32,
        command_send_height: f32,
    ) -> Self {
        Self {
            bottom_panel: ShellBottomPanelState {
                mode: bottom_panel_mode,
                quick_commands_height,
                command_send_height,
                resize: None,
            },
        }
    }
}

impl ShellBottomPanelState {
    const QUICK_COMMANDS_HEIGHT_MIN: f32 = 36.;
    const COMMAND_SEND_HEIGHT_MIN: f32 = 60.;
    const HEIGHT_MAX: f32 = 520.;

    pub(in crate::features) fn start_resize(&mut self, start_y: Pixels) -> bool {
        let start_height = match self.mode {
            BottomPanelMode::QuickCommands => self.quick_commands_height,
            BottomPanelMode::CommandSend => self.command_send_height,
            BottomPanelMode::Hidden => return false,
        };
        self.resize = Some(BottomPanelResizeState {
            mode: self.mode,
            start_y,
            start_height: gpui::px(start_height),
        });
        true
    }

    pub(in crate::features) fn update_resize(&mut self, current_y: Pixels) -> Option<f32> {
        let state = self.resize?;
        let delta = f32::from(current_y - state.start_y);
        let minimum = match state.mode {
            BottomPanelMode::QuickCommands => Self::QUICK_COMMANDS_HEIGHT_MIN,
            BottomPanelMode::CommandSend => Self::COMMAND_SEND_HEIGHT_MIN,
            BottomPanelMode::Hidden => return None,
        };
        let next = (f32::from(state.start_height) - delta).clamp(minimum, Self::HEIGHT_MAX);
        match state.mode {
            BottomPanelMode::QuickCommands => self.quick_commands_height = next,
            BottomPanelMode::CommandSend => self.command_send_height = next,
            BottomPanelMode::Hidden => return None,
        }
        Some(next)
    }

    pub(in crate::features) fn finish_resize(&mut self) -> bool {
        self.resize.take().is_some()
    }
}

#[cfg(test)]
mod tests {
    use gpui::px;

    use super::ShellFeatureState;
    use crate::models::BottomPanelMode;

    #[test]
    fn bottom_panel_resize_updates_only_the_mode_that_started_the_drag() {
        let mut shell = ShellFeatureState::new(BottomPanelMode::QuickCommands, 120., 180.);

        assert!(shell.bottom_panel.start_resize(px(400.)));
        shell.bottom_panel.mode = BottomPanelMode::CommandSend;
        assert_eq!(shell.bottom_panel.update_resize(px(430.)), Some(90.));
        assert_eq!(shell.bottom_panel.quick_commands_height, 90.);
        assert_eq!(shell.bottom_panel.command_send_height, 180.);
        assert!(shell.bottom_panel.finish_resize());
        assert!(!shell.bottom_panel.finish_resize());
    }

    #[test]
    fn hidden_bottom_panel_does_not_start_resize() {
        let mut shell = ShellFeatureState::new(BottomPanelMode::Hidden, 120., 180.);

        assert!(!shell.bottom_panel.start_resize(px(400.)));
        assert!(shell.bottom_panel.resize.is_none());
    }
}

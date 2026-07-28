//! Authoritative transient state for the application shell.
//!
//! Shell rendering remains on `NyaTermApp` views. This state owns interaction
//! lifecycles that span those views so the composition root does not retain
//! independently mutable mirrors.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use gpui::{Pixels, ScrollHandle, WindowHandle};

use super::super::app_state::SettingsDraftSnapshot;
use super::super::settings_window::SettingsWindow;
use crate::models::{
    ActivityBarContextMenuState, ActivityBarLayoutState, BottomPanelMode, BottomPanelResizeState,
    HeaderStatusState, MainMode, NavItem, PanelResizeSide, PanelResizeState, PanelSide,
    PanelStackResizeState, RightFocus, SettingsTab, TitleMenu, TitleMenuSubmenu, WorkspacePaneNode,
    WorkspaceSplitResizeState, WorkspaceSplitState,
};

pub(in crate::features) struct ShellFeatureState {
    pub bottom_panel: ShellBottomPanelState,
    pub viewport: ShellViewportState,
    pub navigation: ShellNavigationState,
    pub panels: ShellPanelState,
    pub chrome: ShellChromeState,
    pub workspace: ShellWorkspaceState,
    pub diagnostics: ShellDiagnosticState,
}

#[derive(Default)]
pub(in crate::features) struct ShellDiagnosticState {
    last_log_at: HashMap<&'static str, Instant>,
}

pub(in crate::features) struct ShellFeatureInit {
    pub bottom_panel_mode: BottomPanelMode,
    pub quick_commands_height: f32,
    pub command_send_height: f32,
    pub active_left_panel: Option<NavItem>,
    pub active_right_panel: Option<NavItem>,
    pub left_open_panels: Vec<String>,
    pub right_open_panels: Vec<String>,
    pub panel_stack_sizes: HashMap<String, f32>,
    pub panel_multi_open: bool,
    pub left_sidebar_collapsed: bool,
    pub right_inspector_collapsed: bool,
    pub left_panel_width: f32,
    pub right_panel_width: f32,
    pub activity_bar_layout: ActivityBarLayoutState,
}

pub(in crate::features) struct ShellBottomPanelState {
    pub mode: BottomPanelMode,
    pub quick_commands_height: f32,
    pub command_send_height: f32,
    pub resize: Option<BottomPanelResizeState>,
}

/// Window geometry and viewport-derived caches.
pub(in crate::features) struct ShellViewportState {
    pub size: (f32, f32),
    pub wallpaper_tile_dimensions: Option<(String, u32, u32)>,
    pub last_change_at: Option<Instant>,
    pub title_drag_active_until: Option<Instant>,
}

/// Top-level page navigation and the settings-page/window lifecycle.
pub(in crate::features) struct ShellNavigationState {
    pub selected_nav: NavItem,
    pub main_mode: MainMode,
    pub settings: ShellSettingsNavigationState,
}

pub(in crate::features) struct ShellSettingsNavigationState {
    pub active_tab: SettingsTab,
    pub expanded_groups: HashSet<String>,
    pub draft_snapshot: Option<SettingsDraftSnapshot>,
    pub window: Option<WindowHandle<SettingsWindow>>,
    pub window_open_pending: bool,
    pub previous_left_collapsed: Option<bool>,
    pub previous_right_collapsed: Option<bool>,
}

/// Side-panel selection, stack layout and resize interaction state.
pub(in crate::features) struct ShellPanelState {
    pub active_left: Option<NavItem>,
    pub active_right: Option<NavItem>,
    pub left_open: Vec<String>,
    pub right_open: Vec<String>,
    pub stack_sizes: HashMap<String, f32>,
    pub multi_open: bool,
    pub right_focus: RightFocus,
    pub left_collapsed: bool,
    pub right_collapsed: bool,
    pub mobile_left_open: bool,
    pub mobile_right_open: bool,
    pub left_width: f32,
    pub right_width: f32,
    pub resize: Option<PanelResizeState>,
    pub stack_resize: Option<PanelStackResizeState>,
}

/// Activity bar, title menus, tab-strip menus and connection-failure chrome.
pub(in crate::features) struct ShellChromeState {
    pub about_open: bool,
    pub activity_bar_layout: ActivityBarLayoutState,
    pub activity_bar_context_menu: Option<ActivityBarContextMenuState>,
    pub title_menu_open: Option<TitleMenu>,
    pub title_menu_submenu: Option<TitleMenuSubmenu>,
    pub header_status: HeaderStatusState,
    pub open_tabs_menu_open: bool,
    pub new_session_menu_open: bool,
    pub new_session_all_sessions_open: bool,
    pub new_session_group_menu_path: Vec<String>,
    pub session_tab_strip_scroll: ScrollHandle,
    pub session_tab_scroll_into_view_pending: bool,
    pub last_connect_failure_name: Option<String>,
    pub last_connect_failure_error: Option<String>,
}

/// Global and per-tab pane trees for the workspace surface.
pub(in crate::features) struct ShellWorkspaceState {
    pub split: Option<WorkspaceSplitState>,
    pub split_resize: Option<WorkspaceSplitResizeState>,
    pub pane_roots: HashMap<String, WorkspacePaneNode>,
    pub tab_owner: HashMap<String, String>,
    pub focused_terminal_leaf_id: Option<String>,
    pub pane_layout_restored: bool,
}

impl ShellFeatureState {
    pub(in crate::features) fn new(init: ShellFeatureInit) -> Self {
        Self {
            bottom_panel: ShellBottomPanelState {
                mode: init.bottom_panel_mode,
                quick_commands_height: init.quick_commands_height,
                command_send_height: init.command_send_height,
                resize: None,
            },
            viewport: ShellViewportState {
                size: (1280., 800.),
                wallpaper_tile_dimensions: None,
                last_change_at: None,
                title_drag_active_until: None,
            },
            navigation: ShellNavigationState {
                selected_nav: NavItem::Workspace,
                main_mode: MainMode::Workspace,
                settings: ShellSettingsNavigationState {
                    active_tab: SettingsTab::General,
                    expanded_groups: HashSet::from(["workspace".to_string()]),
                    draft_snapshot: None,
                    window: None,
                    window_open_pending: false,
                    previous_left_collapsed: None,
                    previous_right_collapsed: None,
                },
            },
            panels: ShellPanelState {
                active_left: init.active_left_panel,
                active_right: init.active_right_panel,
                left_open: init.left_open_panels,
                right_open: init.right_open_panels,
                stack_sizes: init.panel_stack_sizes,
                multi_open: init.panel_multi_open,
                right_focus: RightFocus::Default,
                left_collapsed: init.left_sidebar_collapsed,
                right_collapsed: init.right_inspector_collapsed,
                mobile_left_open: false,
                mobile_right_open: false,
                left_width: init.left_panel_width,
                right_width: init.right_panel_width,
                resize: None,
                stack_resize: None,
            },
            chrome: ShellChromeState {
                about_open: false,
                activity_bar_layout: init.activity_bar_layout,
                activity_bar_context_menu: None,
                title_menu_open: None,
                title_menu_submenu: None,
                header_status: HeaderStatusState::default(),
                open_tabs_menu_open: false,
                new_session_menu_open: false,
                new_session_all_sessions_open: false,
                new_session_group_menu_path: Vec::new(),
                session_tab_strip_scroll: ScrollHandle::new(),
                session_tab_scroll_into_view_pending: false,
                last_connect_failure_name: None,
                last_connect_failure_error: None,
            },
            workspace: ShellWorkspaceState {
                split: None,
                split_resize: None,
                pane_roots: HashMap::new(),
                tab_owner: HashMap::new(),
                focused_terminal_leaf_id: None,
                pane_layout_restored: false,
            },
            diagnostics: ShellDiagnosticState::default(),
        }
    }
}

impl ShellDiagnosticState {
    pub(in crate::features) fn should_log(
        &mut self,
        key: &'static str,
        now: Instant,
        throttle: Duration,
    ) -> bool {
        if self.last_log_at.get(key).is_some_and(|last| {
            now.checked_duration_since(*last)
                .is_some_and(|elapsed| elapsed < throttle)
        }) {
            return false;
        }
        self.last_log_at.insert(key, now);
        true
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

impl ShellViewportState {
    pub(in crate::features) fn update_size(&mut self, size: (f32, f32), now: Instant) -> bool {
        if self.size == size {
            return false;
        }
        self.size = size;
        self.last_change_at = Some(now);
        true
    }

    pub(in crate::features) fn mark_title_drag(&mut self, now: Instant, hold: Duration) {
        self.title_drag_active_until = Some(now + hold);
    }

    pub(in crate::features) fn title_drag_active(&self, now: Instant) -> bool {
        self.title_drag_active_until
            .is_some_and(|until| now < until)
    }
}

impl ShellPanelState {
    const LEFT_WIDTH_MIN: f32 = 160.;
    const LEFT_WIDTH_MAX: f32 = 720.;
    const RIGHT_WIDTH_MIN: f32 = 200.;
    const RIGHT_WIDTH_MAX: f32 = 720.;

    pub(in crate::features) fn start_resize(&mut self, side: PanelResizeSide, start_x: Pixels) {
        let start_width = match side {
            PanelResizeSide::Left => self.left_width,
            PanelResizeSide::Right => self.right_width,
        };
        self.resize = Some(PanelResizeState {
            side,
            start_x,
            start_width: gpui::px(start_width),
        });
    }

    pub(in crate::features) fn update_resize(
        &mut self,
        current_x: Pixels,
    ) -> Option<(PanelResizeSide, f32)> {
        let state = self.resize?;
        let delta = f32::from(current_x - state.start_x);
        let start = f32::from(state.start_width);
        let width = match state.side {
            PanelResizeSide::Left => {
                self.left_width = (start + delta).clamp(Self::LEFT_WIDTH_MIN, Self::LEFT_WIDTH_MAX);
                self.left_width
            }
            PanelResizeSide::Right => {
                self.right_width =
                    (start - delta).clamp(Self::RIGHT_WIDTH_MIN, Self::RIGHT_WIDTH_MAX);
                self.right_width
            }
        };
        Some((state.side, width))
    }

    pub(in crate::features) fn finish_resize(&mut self) -> bool {
        self.resize.take().is_some()
    }

    pub(in crate::features) fn stack_weight(&self, panel_id: &str) -> f32 {
        self.stack_sizes
            .get(panel_id)
            .copied()
            .filter(|value| value.is_finite() && *value > 0.)
            .unwrap_or(1.)
    }

    pub(in crate::features) fn start_stack_resize(
        &mut self,
        side: PanelSide,
        above_id: String,
        below_id: String,
        start_y: Pixels,
        container_height: f32,
    ) {
        self.stack_resize = Some(PanelStackResizeState {
            side,
            above_weight: self.stack_weight(&above_id),
            below_weight: self.stack_weight(&below_id),
            above_id,
            below_id,
            start_y,
            container_height: container_height.max(1.),
        });
    }

    pub(in crate::features) fn update_stack_resize(&mut self, current_y: Pixels) -> bool {
        let Some(state) = self.stack_resize.as_ref() else {
            return false;
        };
        let delta_px = f32::from(current_y - state.start_y);
        let pair = state.above_weight + state.below_weight;
        if pair <= 0. || state.container_height <= 0. {
            return false;
        }
        let px_per_weight = state.container_height / pair;
        let min_weight = (48. / px_per_weight).min(pair / 2.).max(0.05);
        let next_above =
            (state.above_weight + delta_px / px_per_weight).clamp(min_weight, pair - min_weight);
        let next_below = pair - next_above;
        self.stack_sizes.insert(state.above_id.clone(), next_above);
        self.stack_sizes.insert(state.below_id.clone(), next_below);
        true
    }

    pub(in crate::features) fn finish_stack_resize(&mut self) -> bool {
        self.stack_resize.take().is_some()
    }
}

impl ShellChromeState {
    pub(in crate::features) fn prepare_session_switch(&mut self) {
        self.open_tabs_menu_open = false;
        self.close_new_session_menu();
        self.session_tab_scroll_into_view_pending = true;
    }

    pub(in crate::features) fn toggle_open_tabs_menu(&mut self) {
        self.open_tabs_menu_open = !self.open_tabs_menu_open;
        if self.open_tabs_menu_open {
            self.close_new_session_menu();
            self.title_menu_open = None;
        }
    }

    pub(in crate::features) fn close_open_tabs_menu(&mut self) -> bool {
        std::mem::take(&mut self.open_tabs_menu_open)
    }

    pub(in crate::features) fn toggle_new_session_menu(&mut self) {
        self.new_session_menu_open = !self.new_session_menu_open;
        if self.new_session_menu_open {
            self.open_tabs_menu_open = false;
            self.title_menu_open = None;
        }
        self.new_session_all_sessions_open = false;
        self.new_session_group_menu_path.clear();
    }

    pub(in crate::features) fn close_new_session_menu(&mut self) -> bool {
        let changed = self.new_session_menu_open
            || self.new_session_all_sessions_open
            || !self.new_session_group_menu_path.is_empty();
        self.new_session_menu_open = false;
        self.new_session_all_sessions_open = false;
        self.new_session_group_menu_path.clear();
        changed
    }
}

impl ShellWorkspaceState {
    pub(in crate::features) fn rebuild_tab_owners(&mut self) {
        let mut owners = HashMap::new();
        for (tab_root, tree) in &self.pane_roots {
            for leaf in tree.session_ids() {
                owners.insert(leaf, tab_root.clone());
            }
        }
        self.tab_owner = owners;
    }

    pub(in crate::features) fn replace_session_id(&mut self, old_id: &str, new_id: &str) {
        for root in self.pane_roots.values_mut() {
            root.replace_session_id(old_id, new_id);
        }
        if let Some(root) = self.pane_roots.remove(old_id) {
            self.pane_roots.insert(new_id.to_string(), root);
        }
        if let Some(root) = self.split.as_mut() {
            root.replace_session_id(old_id, new_id);
        }
        self.rebuild_tab_owners();
    }

    pub(in crate::features) fn remove_session(&mut self, session_id: &str) {
        self.tab_owner.remove(session_id);
        self.pane_roots.remove(session_id);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::time::{Duration, Instant};

    use gpui::px;

    use super::{ShellFeatureInit, ShellFeatureState};
    use crate::models::{
        ActivityBarLayoutState, BottomPanelMode, PanelResizeSide, PanelSide, WorkspacePaneNode,
        WorkspaceSplitDirection,
    };

    fn shell(mode: BottomPanelMode) -> ShellFeatureState {
        ShellFeatureState::new(ShellFeatureInit {
            bottom_panel_mode: mode,
            quick_commands_height: 120.,
            command_send_height: 180.,
            active_left_panel: None,
            active_right_panel: None,
            left_open_panels: Vec::new(),
            right_open_panels: Vec::new(),
            panel_stack_sizes: HashMap::new(),
            panel_multi_open: false,
            left_sidebar_collapsed: true,
            right_inspector_collapsed: true,
            left_panel_width: 240.,
            right_panel_width: 320.,
            activity_bar_layout: ActivityBarLayoutState::default(),
        })
    }

    #[test]
    fn bottom_panel_resize_updates_only_the_mode_that_started_the_drag() {
        let mut shell = shell(BottomPanelMode::QuickCommands);

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
        let mut shell = shell(BottomPanelMode::Hidden);

        assert!(!shell.bottom_panel.start_resize(px(400.)));
        assert!(shell.bottom_panel.resize.is_none());
    }

    #[test]
    fn panel_resize_clamps_each_side_and_finishes_once() {
        let mut shell = shell(BottomPanelMode::Hidden);

        shell.panels.start_resize(PanelResizeSide::Left, px(100.));
        assert_eq!(
            shell.panels.update_resize(px(-100.)),
            Some((PanelResizeSide::Left, 160.))
        );
        assert!(shell.panels.finish_resize());
        assert!(!shell.panels.finish_resize());

        shell.panels.start_resize(PanelResizeSide::Right, px(100.));
        assert_eq!(
            shell.panels.update_resize(px(-500.)),
            Some((PanelResizeSide::Right, 720.))
        );
    }

    #[test]
    fn panel_stack_resize_preserves_pair_weight() {
        let mut shell = shell(BottomPanelMode::Hidden);
        shell.panels.stack_sizes.insert("above".to_string(), 2.);
        shell.panels.stack_sizes.insert("below".to_string(), 1.);
        shell.panels.start_stack_resize(
            PanelSide::Left,
            "above".to_string(),
            "below".to_string(),
            px(100.),
            300.,
        );

        assert!(shell.panels.update_stack_resize(px(150.)));
        let total = shell.panels.stack_sizes["above"] + shell.panels.stack_sizes["below"];
        assert!((total - 3.).abs() < f32::EPSILON);
        assert!(shell.panels.finish_stack_resize());
    }

    #[test]
    fn chrome_menu_transitions_are_mutually_exclusive() {
        let mut shell = shell(BottomPanelMode::Hidden);
        shell.chrome.new_session_menu_open = true;
        shell.chrome.new_session_all_sessions_open = true;
        shell
            .chrome
            .new_session_group_menu_path
            .push("group".to_string());

        shell.chrome.toggle_open_tabs_menu();
        assert!(shell.chrome.open_tabs_menu_open);
        assert!(!shell.chrome.new_session_menu_open);
        assert!(!shell.chrome.new_session_all_sessions_open);
        assert!(shell.chrome.new_session_group_menu_path.is_empty());

        shell.chrome.toggle_new_session_menu();
        assert!(!shell.chrome.open_tabs_menu_open);
        assert!(shell.chrome.new_session_menu_open);
    }

    #[test]
    fn viewport_tracks_only_real_geometry_changes_and_title_drag_deadline() {
        let mut shell = shell(BottomPanelMode::Hidden);
        let now = Instant::now();
        assert!(!shell.viewport.update_size((1280., 800.), now));
        assert!(shell.viewport.update_size((1024., 768.), now));
        assert_eq!(shell.viewport.last_change_at, Some(now));

        shell
            .viewport
            .mark_title_drag(now, Duration::from_millis(10));
        assert!(shell.viewport.title_drag_active(now));
        assert!(
            !shell
                .viewport
                .title_drag_active(now + Duration::from_millis(10))
        );
    }

    #[test]
    fn diagnostic_throttle_is_keyed_and_advances_after_interval() {
        let mut shell = shell(BottomPanelMode::Hidden);
        let now = Instant::now();
        let throttle = Duration::from_secs(2);

        assert!(shell.diagnostics.should_log("session", now, throttle));
        assert!(
            !shell
                .diagnostics
                .should_log("session", now + Duration::from_secs(1), throttle)
        );
        assert!(shell.diagnostics.should_log("frame", now, throttle));
        assert!(
            shell
                .diagnostics
                .should_log("session", now + Duration::from_secs(2), throttle)
        );
        assert!(
            shell
                .diagnostics
                .should_log("clock", now + Duration::from_secs(2), throttle)
        );
        assert!(shell.diagnostics.should_log("clock", now, throttle));
    }

    #[test]
    fn workspace_rebuilds_and_renames_tab_ownership() {
        let mut shell = shell(BottomPanelMode::Hidden);
        shell.workspace.pane_roots.insert(
            "root".to_string(),
            WorkspacePaneNode::Split {
                id: "split".to_string(),
                direction: WorkspaceSplitDirection::Vertical,
                ratio_percent: 50,
                first: Box::new(WorkspacePaneNode::leaf("root".to_string())),
                second: Box::new(WorkspacePaneNode::leaf("leaf".to_string())),
            },
        );
        shell.workspace.rebuild_tab_owners();
        assert_eq!(shell.workspace.tab_owner["leaf"], "root");

        shell.workspace.replace_session_id("root", "renamed");
        assert!(shell.workspace.pane_roots.contains_key("renamed"));
        assert_eq!(shell.workspace.tab_owner["leaf"], "renamed");
        assert_eq!(shell.workspace.tab_owner["renamed"], "renamed");
    }
}

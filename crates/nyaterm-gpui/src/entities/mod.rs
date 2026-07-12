//! GPUI entity-state boundaries for the native shell.

use std::{collections::HashSet, time::Duration};

use gpui::{Context, Entity, Timer, Window};
use nyaterm_core::{
    AppRuntime, NativeServices, RestorableOpenTab, RestorableWorkspacePaneNode,
};

use crate::ui::NyaTermApp;

#[derive(Debug)]
pub struct RuntimeStore {
    runtime: AppRuntime,
    services: NativeServices,
}

impl RuntimeStore {
    pub fn new(runtime: AppRuntime) -> Self {
        Self {
            runtime,
            services: NativeServices::new(),
        }
    }

    pub fn runtime(&self) -> &AppRuntime {
        &self.runtime
    }

    pub fn services(&self) -> &NativeServices {
        &self.services
    }
}

#[derive(Debug, Default)]
pub struct WindowRuntimeStore {
    pump_started: bool,
}

impl WindowRuntimeStore {
    pub fn ensure_started(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        app: Entity<NyaTermApp>,
    ) -> bool {
        if !self.mark_started() {
            return false;
        }
        app.update(cx, |app, _| app.mark_window_runtime_started());
        window
            .spawn(cx, async move |cx| {
                loop {
                    Timer::after(Duration::from_millis(50)).await;
                    let keep_running = cx
                        .update(|window, cx| {
                            app.update(cx, |app, cx| app.drive_window_runtime_tick(window, cx))
                        })
                        .unwrap_or(false);
                    if !keep_running {
                        break;
                    }
                }
            })
            .detach();
        true
    }

    pub fn mark_started(&mut self) -> bool {
        if self.pump_started {
            return false;
        }
        self.pump_started = true;
        true
    }

    pub fn pump_started(&self) -> bool {
        self.pump_started
    }
}

#[derive(Debug, Default)]
pub struct StartupRestoreStore {
    started_after_window_open: bool,
    open_tabs_restored: bool,
    complete: bool,
    queue: Vec<RestorableOpenTab>,
    pending_pane_layouts: Vec<RestorableWorkspacePaneNode>,
    pending_active_pane_indexes: Vec<usize>,
}

impl StartupRestoreStore {
    pub fn mark_started_after_window_open(&mut self) -> bool {
        if self.started_after_window_open {
            return false;
        }
        self.started_after_window_open = true;
        true
    }

    pub fn started_after_window_open(&self) -> bool {
        self.started_after_window_open
    }

    pub fn open_tabs_restored(&self) -> bool {
        self.open_tabs_restored
    }

    pub fn mark_open_tabs_restored(&mut self) -> bool {
        if self.open_tabs_restored {
            return false;
        }
        self.open_tabs_restored = true;
        true
    }

    pub fn complete(&self) -> bool {
        self.complete
    }

    pub fn mark_complete(&mut self) -> bool {
        if self.complete {
            return false;
        }
        self.complete = true;
        true
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    pub fn set_queue(&mut self, queue: Vec<RestorableOpenTab>) {
        self.queue = queue;
    }

    pub fn queue_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn pop_next_tab(&mut self) -> Option<RestorableOpenTab> {
        if self.queue.is_empty() {
            None
        } else {
            Some(self.queue.remove(0))
        }
    }

    pub fn can_pump_queue(&self, pending_session_active: bool) -> bool {
        !self.complete && !pending_session_active && !self.queue.is_empty()
    }

    pub fn clear_pending_layouts(&mut self) {
        self.pending_pane_layouts.clear();
        self.pending_active_pane_indexes.clear();
    }

    pub fn push_pending_pane_layout(&mut self, layout: RestorableWorkspacePaneNode) {
        self.pending_pane_layouts.push(layout);
    }

    pub fn push_pending_active_pane_index(&mut self, index: usize) {
        self.pending_active_pane_indexes.push(index);
    }

    pub fn take_pending_pane_layouts(&mut self) -> Vec<RestorableWorkspacePaneNode> {
        std::mem::take(&mut self.pending_pane_layouts)
    }

    pub fn take_pending_active_pane_indexes(&mut self) -> Vec<usize> {
        std::mem::take(&mut self.pending_active_pane_indexes)
    }
}

#[derive(Clone)]
pub struct UiStoreHandles {
    pub startup_restore: Entity<StartupRestoreStore>,
    pub workspace: Entity<WorkspaceStore>,
    pub sessions: Entity<SessionStore>,
    pub overlays: Entity<OverlayStore>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSnapshot {
    pub active_session_id: Option<String>,
    pub ordered_tab_roots: Vec<String>,
    pub selected_nav: String,
    pub main_mode: String,
    pub active_left_panel: Option<String>,
    pub active_right_panel: Option<String>,
    pub left_sidebar_collapsed: bool,
    pub right_inspector_collapsed: bool,
    pub workspace_split_active: bool,
    pub terminal_windows_active: bool,
}

#[derive(Debug, Default)]
pub struct WorkspaceStore {
    active_session_id: Option<String>,
    ordered_tab_roots: Vec<String>,
    snapshot: Option<WorkspaceSnapshot>,
}

impl WorkspaceStore {
    pub fn active_session_id(&self) -> Option<&str> {
        self.active_session_id.as_deref()
    }

    pub fn ordered_tab_roots(&self) -> &[String] {
        &self.ordered_tab_roots
    }

    pub fn activate_session(&mut self, session_id: impl Into<String>) {
        self.active_session_id = Some(session_id.into());
    }

    pub fn set_ordered_tab_roots(&mut self, roots: Vec<String>) {
        self.ordered_tab_roots = roots;
    }

    pub fn snapshot(&self) -> Option<&WorkspaceSnapshot> {
        self.snapshot.as_ref()
    }

    pub fn replace_snapshot(&mut self, snapshot: WorkspaceSnapshot) -> bool {
        if self.snapshot.as_ref() == Some(&snapshot) {
            return false;
        }
        self.active_session_id = snapshot.active_session_id.clone();
        self.ordered_tab_roots = snapshot.ordered_tab_roots.clone();
        self.snapshot = Some(snapshot);
        true
    }

    pub fn select_nav(&mut self, nav: impl Into<String>, main_mode: impl Into<String>) -> bool {
        let snapshot = self
            .snapshot
            .get_or_insert_with(WorkspaceSnapshot::default);
        let nav = nav.into();
        let main_mode = main_mode.into();
        if snapshot.selected_nav == nav && snapshot.main_mode == main_mode {
            return false;
        }
        snapshot.selected_nav = nav;
        snapshot.main_mode = main_mode;
        true
    }

    pub fn open_left_panel(&mut self, panel: impl Into<String>) -> bool {
        let snapshot = self
            .snapshot
            .get_or_insert_with(WorkspaceSnapshot::default);
        let panel = Some(panel.into());
        if snapshot.active_left_panel == panel && !snapshot.left_sidebar_collapsed {
            return false;
        }
        snapshot.active_left_panel = panel;
        snapshot.left_sidebar_collapsed = false;
        true
    }

    pub fn close_left_panel(&mut self) -> bool {
        let snapshot = self
            .snapshot
            .get_or_insert_with(WorkspaceSnapshot::default);
        if snapshot.active_left_panel.is_none() && snapshot.left_sidebar_collapsed {
            return false;
        }
        snapshot.active_left_panel = None;
        snapshot.left_sidebar_collapsed = true;
        true
    }
}

impl Default for WorkspaceSnapshot {
    fn default() -> Self {
        Self {
            active_session_id: None,
            ordered_tab_roots: Vec::new(),
            selected_nav: "Workspace".to_string(),
            main_mode: "Workspace".to_string(),
            active_left_panel: None,
            active_right_panel: None,
            left_sidebar_collapsed: true,
            right_inspector_collapsed: true,
            workspace_split_active: false,
            terminal_windows_active: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSnapshot {
    pub active_session_id: Option<String>,
    pub ordered_session_ids: Vec<String>,
    pub live_session_ids: Vec<String>,
    pub metadata_count: usize,
    pub terminal_view_count: usize,
    pub pending_start_count: usize,
    pub host_prompt_active: bool,
    pub credential_prompt_active: bool,
    pub zmodem_session_count: usize,
}

#[derive(Debug, Default)]
pub struct SessionStore {
    live_sessions: HashSet<String>,
    active_session_id: Option<String>,
    ordered_session_ids: Vec<String>,
    snapshot: Option<SessionSnapshot>,
}

impl SessionStore {
    pub fn live_session_count(&self) -> usize {
        self.live_sessions.len()
    }

    pub fn is_live(&self, session_id: &str) -> bool {
        self.live_sessions.contains(session_id)
    }

    pub fn active_session_id(&self) -> Option<&str> {
        self.active_session_id.as_deref()
    }

    pub fn ordered_session_ids(&self) -> &[String] {
        &self.ordered_session_ids
    }

    pub fn mark_live(&mut self, session_id: impl Into<String>) {
        self.live_sessions.insert(session_id.into());
    }

    pub fn mark_closed(&mut self, session_id: &str) {
        self.live_sessions.remove(session_id);
    }

    pub fn snapshot(&self) -> Option<&SessionSnapshot> {
        self.snapshot.as_ref()
    }

    pub fn replace_snapshot(&mut self, snapshot: SessionSnapshot) -> bool {
        if self.snapshot.as_ref() == Some(&snapshot) {
            return false;
        }
        self.active_session_id = snapshot.active_session_id.clone();
        self.ordered_session_ids = snapshot.ordered_session_ids.clone();
        self.live_sessions = snapshot.live_session_ids.iter().cloned().collect();
        self.snapshot = Some(snapshot);
        true
    }

    pub fn activate(&mut self, session_id: impl Into<String>) -> bool {
        let session_id = session_id.into();
        if self.active_session_id.as_deref() == Some(session_id.as_str()) {
            return false;
        }
        self.active_session_id = Some(session_id.clone());
        let snapshot = self.snapshot.get_or_insert_with(SessionSnapshot::default);
        snapshot.active_session_id = Some(session_id);
        true
    }

    pub fn set_ordered_session_ids(&mut self, ordered_session_ids: Vec<String>) -> bool {
        if self.ordered_session_ids == ordered_session_ids {
            return false;
        }
        self.ordered_session_ids = ordered_session_ids.clone();
        let snapshot = self.snapshot.get_or_insert_with(SessionSnapshot::default);
        snapshot.ordered_session_ids = ordered_session_ids;
        true
    }

    pub fn move_session_to_index(&mut self, session_id: &str, index: usize) -> bool {
        let Some(current_index) = self
            .ordered_session_ids
            .iter()
            .position(|id| id == session_id)
        else {
            return false;
        };
        let session_id = self.ordered_session_ids.remove(current_index);
        let index = index.min(self.ordered_session_ids.len());
        self.ordered_session_ids.insert(index, session_id);
        let snapshot = self.snapshot.get_or_insert_with(SessionSnapshot::default);
        snapshot.ordered_session_ids = self.ordered_session_ids.clone();
        true
    }

    pub fn remove_session(&mut self, session_id: &str) -> bool {
        let before_len = self.ordered_session_ids.len();
        self.ordered_session_ids.retain(|id| id != session_id);
        self.live_sessions.remove(session_id);
        if self.active_session_id.as_deref() == Some(session_id) {
            self.active_session_id = None;
        }
        let changed = before_len != self.ordered_session_ids.len();
        if changed {
            let snapshot = self.snapshot.get_or_insert_with(SessionSnapshot::default);
            snapshot.ordered_session_ids = self.ordered_session_ids.clone();
            snapshot.live_session_ids = self.live_sessions.iter().cloned().collect();
            snapshot.active_session_id = self.active_session_id.clone();
        }
        changed
    }
}

impl Default for SessionSnapshot {
    fn default() -> Self {
        Self {
            active_session_id: None,
            ordered_session_ids: Vec::new(),
            live_session_ids: Vec::new(),
            metadata_count: 0,
            terminal_view_count: 0,
            pending_start_count: 0,
            host_prompt_active: false,
            credential_prompt_active: false,
            zmodem_session_count: 0,
        }
    }
}

#[derive(Debug, Default)]
pub struct SettingsStore;

#[derive(Debug, Default)]
pub struct ConnectionsStore;

#[derive(Debug, Default)]
pub struct TransferStore;

#[derive(Debug, Default)]
pub struct AiStore;

#[derive(Debug, Default)]
pub struct CloudSyncStore;

#[derive(Debug, Default)]
pub struct RemoteOpsStore;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlaySnapshot {
    pub quick_switch_open: bool,
    pub tab_actions_open: bool,
    pub rename_open: bool,
    pub color_picker_open: bool,
    pub session_info_open: bool,
    pub startup_command_open: bool,
    pub temporary_ssh_link_open: bool,
    pub multi_line_paste_open: bool,
    pub terminal_actions_open: bool,
    pub terminal_context_menu_open: bool,
    pub action_link_menu_open: bool,
    pub action_link_tooltip_open: bool,
    pub command_suggestions_open: bool,
    pub credential_suggestions_open: bool,
    pub close_all_sessions_confirm_open: bool,
    pub locked: bool,
}

impl Default for OverlaySnapshot {
    fn default() -> Self {
        Self {
            quick_switch_open: false,
            tab_actions_open: false,
            rename_open: false,
            color_picker_open: false,
            session_info_open: false,
            startup_command_open: false,
            temporary_ssh_link_open: false,
            multi_line_paste_open: false,
            terminal_actions_open: false,
            terminal_context_menu_open: false,
            action_link_menu_open: false,
            action_link_tooltip_open: false,
            command_suggestions_open: false,
            credential_suggestions_open: false,
            close_all_sessions_confirm_open: false,
            locked: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct OverlayStore {
    snapshot: Option<OverlaySnapshot>,
}

impl OverlayStore {
    pub fn snapshot(&self) -> Option<&OverlaySnapshot> {
        self.snapshot.as_ref()
    }

    pub fn replace_snapshot(&mut self, snapshot: OverlaySnapshot) -> bool {
        if self.snapshot.as_ref() == Some(&snapshot) {
            return false;
        }
        self.snapshot = Some(snapshot);
        true
    }

    pub fn set_quick_switch_open(&mut self, open: bool) -> bool {
        let snapshot = self.snapshot.get_or_insert_with(OverlaySnapshot::default);
        if snapshot.quick_switch_open == open {
            return false;
        }
        snapshot.quick_switch_open = open;
        true
    }

    pub fn set_tab_actions_open(&mut self, open: bool) -> bool {
        let snapshot = self.snapshot.get_or_insert_with(OverlaySnapshot::default);
        if snapshot.tab_actions_open == open {
            return false;
        }
        snapshot.tab_actions_open = open;
        true
    }

    pub fn set_terminal_context_menu_open(&mut self, open: bool) -> bool {
        let snapshot = self.snapshot.get_or_insert_with(OverlaySnapshot::default);
        if snapshot.terminal_context_menu_open == open {
            return false;
        }
        snapshot.terminal_context_menu_open = open;
        if open {
            snapshot.action_link_menu_open = false;
        }
        true
    }

    pub fn set_action_link_menu_open(&mut self, open: bool) -> bool {
        let snapshot = self.snapshot.get_or_insert_with(OverlaySnapshot::default);
        if snapshot.action_link_menu_open == open {
            return false;
        }
        snapshot.action_link_menu_open = open;
        if open {
            snapshot.terminal_context_menu_open = false;
        }
        true
    }

    pub fn set_locked(&mut self, locked: bool) -> bool {
        let snapshot = self.snapshot.get_or_insert_with(OverlaySnapshot::default);
        if snapshot.locked == locked {
            return false;
        }
        snapshot.locked = locked;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{
        OverlaySnapshot, OverlayStore, SessionSnapshot, SessionStore, StartupRestoreStore,
        WindowRuntimeStore, WorkspaceSnapshot, WorkspaceStore,
    };

    #[test]
    fn workspace_store_tracks_active_session_and_tab_order() {
        let mut store = WorkspaceStore::default();
        store.activate_session("session-a");
        store.set_ordered_tab_roots(vec!["session-a".into(), "session-b".into()]);

        assert_eq!(store.active_session_id(), Some("session-a"));
        assert_eq!(store.ordered_tab_roots(), ["session-a", "session-b"]);
    }

    #[test]
    fn session_store_tracks_live_sessions() {
        let mut store = SessionStore::default();
        store.mark_live("session-a");
        store.mark_live("session-b");
        store.mark_closed("session-a");

        assert!(!store.is_live("session-a"));
        assert!(store.is_live("session-b"));
        assert_eq!(store.live_session_count(), 1);
    }

    #[test]
    fn window_runtime_store_starts_pump_once() {
        let mut store = WindowRuntimeStore::default();

        assert!(store.mark_started());
        assert!(!store.mark_started());
        assert!(store.pump_started());
    }

    #[test]
    fn startup_restore_store_starts_after_window_open_once() {
        let mut store = StartupRestoreStore::default();

        assert!(store.mark_started_after_window_open());
        assert!(!store.mark_started_after_window_open());
        assert!(store.started_after_window_open());
    }

    #[test]
    fn startup_restore_store_tracks_queue_and_completion() {
        let mut store = StartupRestoreStore::default();
        store.set_queue(vec![
            nyaterm_core::RestorableOpenTab::with_leaf_root(
                "one",
                "Local",
                None,
                None,
                None,
            ),
            nyaterm_core::RestorableOpenTab::with_leaf_root(
                "two",
                "Local",
                None,
                None,
                None,
            ),
        ]);

        assert_eq!(store.queue_len(), 2);
        assert!(store.can_pump_queue(false));
        assert!(!store.can_pump_queue(true));
        assert_eq!(store.pop_next_tab().as_ref().map(|tab| tab.title.as_str()), Some("one"));
        assert_eq!(store.queue_len(), 1);
        assert!(store.mark_complete());
        assert!(!store.can_pump_queue(false));
    }

    #[test]
    fn workspace_store_notifies_only_for_changed_snapshots() {
        let mut store = WorkspaceStore::default();
        let snapshot = WorkspaceSnapshot {
            active_session_id: Some("session-a".into()),
            ordered_tab_roots: vec!["session-a".into()],
            selected_nav: "Workspace".into(),
            main_mode: "Workspace".into(),
            active_left_panel: Some("Saved Connections".into()),
            active_right_panel: None,
            left_sidebar_collapsed: false,
            right_inspector_collapsed: false,
            workspace_split_active: false,
            terminal_windows_active: false,
        };

        assert!(store.replace_snapshot(snapshot.clone()));
        assert!(!store.replace_snapshot(snapshot));
        assert_eq!(store.active_session_id(), Some("session-a"));
        assert_eq!(store.ordered_tab_roots(), ["session-a"]);
    }

    #[test]
    fn workspace_store_mutates_navigation_and_left_panel() {
        let mut store = WorkspaceStore::default();

        assert!(store.select_nav("Settings", "Page"));
        assert!(!store.select_nav("Settings", "Page"));
        assert!(store.open_left_panel("File Explorer"));
        assert!(!store.open_left_panel("File Explorer"));
        assert!(store.close_left_panel());
        assert!(!store.close_left_panel());

        let snapshot = store.snapshot().expect("workspace snapshot");
        assert_eq!(snapshot.selected_nav, "Settings");
        assert_eq!(snapshot.main_mode, "Page");
        assert!(snapshot.left_sidebar_collapsed);
        assert_eq!(snapshot.active_left_panel, None);
    }

    #[test]
    fn session_store_notifies_only_for_changed_snapshots() {
        let mut store = SessionStore::default();
        let snapshot = SessionSnapshot {
            active_session_id: Some("session-a".into()),
            ordered_session_ids: vec!["session-a".into(), "session-b".into()],
            live_session_ids: vec!["session-b".into()],
            metadata_count: 1,
            terminal_view_count: 1,
            pending_start_count: 0,
            host_prompt_active: false,
            credential_prompt_active: false,
            zmodem_session_count: 0,
        };

        assert!(store.replace_snapshot(snapshot.clone()));
        assert!(!store.replace_snapshot(snapshot));
        assert!(!store.is_live("session-a"));
        assert!(store.is_live("session-b"));
    }

    #[test]
    fn session_store_mutates_active_order_and_removal() {
        let mut store = SessionStore::default();

        assert!(store.set_ordered_session_ids(vec![
            "session-a".into(),
            "session-b".into(),
            "session-c".into(),
        ]));
        assert!(store.activate("session-b"));
        assert!(!store.activate("session-b"));
        assert!(store.move_session_to_index("session-c", 0));
        assert_eq!(
            store.ordered_session_ids(),
            ["session-c", "session-a", "session-b"]
        );
        assert!(store.remove_session("session-b"));
        assert_eq!(store.active_session_id(), None);
        assert_eq!(store.ordered_session_ids(), ["session-c", "session-a"]);
    }

    #[test]
    fn overlay_store_notifies_only_for_changed_snapshots() {
        let mut store = OverlayStore::default();
        let snapshot = OverlaySnapshot {
            quick_switch_open: true,
            tab_actions_open: false,
            rename_open: false,
            color_picker_open: false,
            session_info_open: false,
            startup_command_open: false,
            temporary_ssh_link_open: false,
            multi_line_paste_open: false,
            terminal_actions_open: false,
            terminal_context_menu_open: false,
            action_link_menu_open: false,
            action_link_tooltip_open: false,
            command_suggestions_open: false,
            credential_suggestions_open: false,
            close_all_sessions_confirm_open: false,
            locked: false,
        };

        assert!(store.replace_snapshot(snapshot.clone()));
        assert!(!store.replace_snapshot(snapshot));
        assert!(store.snapshot().is_some_and(|snapshot| snapshot.quick_switch_open));
    }

    #[test]
    fn overlay_store_mutates_open_close_and_menu_exclusion() {
        let mut store = OverlayStore::default();

        assert!(store.set_quick_switch_open(true));
        assert!(!store.set_quick_switch_open(true));
        assert!(store.set_terminal_context_menu_open(true));
        assert!(store.set_action_link_menu_open(true));
        let snapshot = store.snapshot().expect("overlay snapshot");
        assert!(snapshot.action_link_menu_open);
        assert!(!snapshot.terminal_context_menu_open);
        assert!(store.set_locked(true));
        assert!(!store.set_locked(true));
    }
}

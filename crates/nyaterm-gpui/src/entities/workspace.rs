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

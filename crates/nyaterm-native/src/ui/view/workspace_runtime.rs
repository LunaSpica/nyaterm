use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn live_session_ids(&self) -> HashSet<String> {
        self.session_manager
            .list_sessions()
            .unwrap_or_default()
            .into_iter()
            .map(|session| session.id)
            .collect()
    }

    pub(in crate::ui::view) fn prune_workspace_split(&mut self) {
        let Some(split) = self.workspace_split.clone() else {
            return;
        };
        let live_ids = self.live_session_ids();
        if !live_ids.contains(&split.primary_session_id)
            || !live_ids.contains(&split.secondary_session_id)
            || split.primary_session_id == split.secondary_session_id
        {
            self.workspace_split = None;
        }
    }

    fn attach_workspace_split(
        &mut self,
        direction: WorkspaceSplitDirection,
        primary_session_id: String,
        secondary_session_id: String,
    ) {
        self.workspace_split = Some(WorkspaceSplitState {
            direction,
            primary_session_id,
            secondary_session_id,
            ratio_percent: WorkspaceSplitState::DEFAULT_RATIO_PERCENT,
        });
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
    }

    pub(in crate::ui::view) fn adjust_workspace_split_ratio(
        &mut self,
        delta: i8,
        cx: &mut Context<Self>,
    ) {
        let Some(split) = self.workspace_split.as_mut() else {
            self.terminal_status = "workspace is not split".to_string();
            cx.notify();
            return;
        };
        split.adjust_ratio(delta);
        self.terminal_status = format!("split ratio {}%", split.ratio_percent);
        cx.notify();
    }

    pub(in crate::ui::view) fn split_workspace_with_duplicate(
        &mut self,
        direction: WorkspaceSplitDirection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_session_name.is_some() {
            self.terminal_status = "wait for the pending session to finish connecting".to_string();
            cx.notify();
            return;
        }
        let Some(source_session_id) = self.active_session_id.clone() else {
            self.terminal_status = "start a session before splitting".to_string();
            cx.notify();
            return;
        };
        if !self.session_metadata.contains_key(&source_session_id) {
            self.terminal_status = "active session cannot be duplicated for split".to_string();
            cx.notify();
            return;
        }
        self.pending_workspace_split = Some((direction, source_session_id));
        self.duplicate_active_session(window, cx);
    }

    pub(in crate::ui::view) fn apply_pending_workspace_split_for_duplicate(
        &mut self,
        new_session_id: &str,
    ) {
        let Some((direction, source_session_id)) = self.pending_workspace_split.take() else {
            return;
        };
        self.attach_workspace_split(direction, source_session_id, new_session_id.to_string());
        self.terminal_status =
            format!("split {} pane duplicated", direction.label().to_lowercase());
    }

    pub(in crate::ui::view) fn unsplit_workspace(&mut self, cx: &mut Context<Self>) {
        if self.workspace_split.take().is_some() {
            self.terminal_status = "workspace split closed".to_string();
        } else {
            self.terminal_status = "workspace is not split".to_string();
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn activate_workspace_pane(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        self.select_session(session_id, cx);
        self.prune_workspace_split();
    }
}

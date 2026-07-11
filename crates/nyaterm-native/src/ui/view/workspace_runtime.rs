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
        let Some(root) = self.workspace_split.take() else {
            return;
        };
        let live_ids = self.live_session_ids();
        match root.prune(&live_ids) {
            Some(node) if node.is_split() => {
                self.workspace_split = Some(node);
            }
            Some(WorkspacePaneNode::Leaf { session_id }) => {
                // Collapse fully to single leaf: clear tree and keep active session.
                if self.active_session_id.is_none() {
                    self.active_session_id = Some(session_id);
                }
                self.workspace_split = None;
            }
            _ => {
                self.workspace_split = None;
            }
        }
    }

    fn attach_workspace_split(
        &mut self,
        direction: WorkspaceSplitDirection,
        primary_session_id: String,
        secondary_session_id: String,
    ) {
        let split_id = uuid();
        if let Some(root) = self.workspace_split.as_mut() {
            if root.split_leaf(
                &primary_session_id,
                secondary_session_id.clone(),
                direction,
                split_id.clone(),
            ) {
                self.selected_nav = NavItem::Workspace;
                self.main_mode = MainMode::Workspace;
                return;
            }
        }
        // No existing tree or target leaf missing: create root dual split.
        self.workspace_split = Some(WorkspacePaneNode::Split {
            id: split_id,
            direction,
            ratio_percent: WorkspacePaneNode::DEFAULT_RATIO_PERCENT,
            first: Box::new(WorkspacePaneNode::leaf(primary_session_id)),
            second: Box::new(WorkspacePaneNode::leaf(secondary_session_id)),
        });
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
    }

    pub(in crate::ui::view) fn activate_workspace_pane(
        &mut self,
        session_id: String,
        cx: &mut Context<Self>,
    ) {
        if self.active_session_id.as_deref() == Some(session_id.as_str()) {
            return;
        }
        self.activate_session_id(&session_id);
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
        self.terminal_status = format!("focused pane {session_id}");
        cx.notify();
    }

    pub(in crate::ui::view) fn focused_workspace_split_id(&self) -> Option<String> {
        self.workspace_split.as_ref()?.focused_split_id(
            self.active_session_id.as_deref(),
        )
    }

    pub(in crate::ui::view) fn adjust_workspace_split_ratio(
        &mut self,
        delta: i8,
        cx: &mut Context<Self>,
    ) {
        let Some(split_id) = self.focused_workspace_split_id() else {
            self.terminal_status = "workspace is not split".to_string();
            cx.notify();
            return;
        };
        let Some(root) = self.workspace_split.as_mut() else {
            self.terminal_status = "workspace is not split".to_string();
            cx.notify();
            return;
        };
        if root.adjust_ratio_for_split(&split_id, delta) {
            let ratio = root.ratio_for_split(&split_id).unwrap_or(50);
            self.terminal_status = format!("split ratio {ratio}%");
        } else {
            self.terminal_status = "workspace is not split".to_string();
        }
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
        let Some(root) = self.workspace_split.take() else {
            self.terminal_status = "workspace is not split".to_string();
            cx.notify();
            return;
        };
        self.workspace_split_resize = None;

        // Prefer collapsing the focused split around the active session leaf.
        // If the active leaf is inside a split, remove the sibling leaf path by
        // keeping only the active session's branch.
        if let Some(active_id) = self.active_session_id.clone() {
            if let Some(collapsed) = collapse_around_session(root.clone(), &active_id) {
                match collapsed {
                    WorkspacePaneNode::Split { .. } => {
                        self.workspace_split = Some(collapsed);
                        self.terminal_status = "collapsed focused split".to_string();
                    }
                    WorkspacePaneNode::Leaf { session_id } => {
                        self.active_session_id = Some(session_id);
                        self.workspace_split = None;
                        self.terminal_status = "workspace split closed".to_string();
                    }
                }
                cx.notify();
                return;
            }
        }

        // Fallback: clear entire tree.
        let _ = root;
        self.workspace_split = None;
        self.terminal_status = "workspace split closed".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn ensure_workspace_focus(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.prune_workspace_split();
        cx.notify();
    }

    pub(in crate::ui::view) fn start_workspace_split_resize(
        &mut self,
        split_id: String,
        event: &gpui::MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        let Some(root) = self.workspace_split.clone() else {
            return;
        };
        let Some(direction) = root.direction_for_split(&split_id) else {
            return;
        };
        let Some(start_ratio) = root.ratio_for_split(&split_id) else {
            return;
        };
        let start_pos = match direction {
            WorkspaceSplitDirection::Horizontal => event.position.y,
            WorkspaceSplitDirection::Vertical => event.position.x,
        };
        self.workspace_split_resize = Some(WorkspaceSplitResizeState {
            split_id,
            direction,
            start_pos,
            start_ratio,
            container_size: 0.,
        });
        self.terminal_status = "resizing workspace split".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn update_workspace_split_resize(
        &mut self,
        event: &gpui::MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.workspace_split_resize.clone() else {
            return;
        };
        let Some(root) = self.workspace_split.as_mut() else {
            return;
        };
        let current = match state.direction {
            WorkspaceSplitDirection::Horizontal => event.position.y,
            WorkspaceSplitDirection::Vertical => event.position.x,
        };
        let delta_px = f32::from(current - state.start_pos);
        // Approximate container size from a stable heuristic when unknown: treat 4px ~ 1%.
        let container = if state.container_size > 1. {
            state.container_size
        } else {
            400.
        };
        let delta_ratio = ((delta_px / container) * 100.).round() as i16;
        let next = (state.start_ratio as i16 + delta_ratio).clamp(
            WorkspacePaneNode::MIN_RATIO_PERCENT as i16,
            WorkspacePaneNode::MAX_RATIO_PERCENT as i16,
        ) as u8;
        if root.set_ratio_for_split(&state.split_id, next) {
            self.terminal_status = format!("split ratio {next}%");
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn finish_workspace_split_resize(
        &mut self,
        _event: &gpui::MouseUpEvent,
        cx: &mut Context<Self>,
    ) {
        if let Some(state) = self.workspace_split_resize.take() {
            if let Some(root) = self.workspace_split.as_ref() {
                if let Some(ratio) = root.ratio_for_split(&state.split_id) {
                    self.terminal_status = format!("split ratio set to {ratio}%");
                }
            }
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn workspace_split_resize_handle(
        &self,
        split_id: String,
        direction: WorkspaceSplitDirection,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id = SharedString::from(format!("workspace-split-resize-{split_id}"));
        match direction {
            WorkspaceSplitDirection::Horizontal => div()
                .id(id)
                .h(px(5.))
                .flex_none()
                .w_full()
                .rounded_sm()
                .bg(rgb(0x30363d))
                .cursor_row_resize()
                .hover(|this| this.bg(rgb(0x58a6ff)))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                        this.start_workspace_split_resize(split_id.clone(), event, cx);
                    }),
                )
                .into_any_element(),
            WorkspaceSplitDirection::Vertical => div()
                .id(id)
                .w(px(5.))
                .flex_none()
                .h_full()
                .rounded_sm()
                .bg(rgb(0x30363d))
                .cursor_col_resize()
                .hover(|this| this.bg(rgb(0x58a6ff)))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                        this.start_workspace_split_resize(split_id.clone(), event, cx);
                    }),
                )
                .into_any_element(),
        }
    }
}

/// Keep only the branch that contains `session_id`, collapsing every split on the path
/// into that single branch (closes the sibling panes of the active leaf).
fn collapse_around_session(
    node: WorkspacePaneNode,
    session_id: &str,
) -> Option<WorkspacePaneNode> {
    match node {
        WorkspacePaneNode::Leaf { session_id: id } => {
            if id == session_id {
                Some(WorkspacePaneNode::Leaf { session_id: id })
            } else {
                None
            }
        }
        WorkspacePaneNode::Split {
            first,
            second,
            ..
        } => {
            let in_first = first.contains_session(session_id);
            let in_second = second.contains_session(session_id);
            if in_first && !in_second {
                collapse_around_session(*first, session_id)
            } else if in_second && !in_first {
                collapse_around_session(*second, session_id)
            } else if in_first && in_second {
                // Should not happen for unique session leaves; keep first match.
                collapse_around_session(*first, session_id)
            } else {
                None
            }
        }
    }
}

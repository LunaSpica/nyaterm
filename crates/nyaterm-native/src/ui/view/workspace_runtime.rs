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
        let live_ids = self.live_session_ids_with_disconnected();
        let before_roots = self.session_pane_roots.clone();
        let root_keys: Vec<String> = self.session_pane_roots.keys().cloned().collect();
        for tab_root in root_keys {
            let Some(root) = self.session_pane_roots.remove(&tab_root) else {
                continue;
            };
            match root.prune(&live_ids) {
                Some(node) => {
                    if node.is_split() {
                        // Keep map key as original tab root when still present; else rekey.
                        let key = if node.contains_session(&tab_root) {
                            tab_root.clone()
                        } else {
                            node.session_ids()
                                .into_iter()
                                .next()
                                .unwrap_or_else(|| tab_root.clone())
                        };
                        self.session_pane_roots.insert(key, node);
                    }
                    // Single leaf collapses to no stored tree for this tab.
                }
                None => {}
            }
        }
        // Also prune legacy workspace_split if roots empty (migration path).
        if self.session_pane_roots.is_empty() {
            if let Some(root) = self.workspace_split.take() {
                match root.prune(&live_ids) {
                    Some(node) => {
                        if node.is_split() {
                            if let Some(first) = node.session_ids().into_iter().next() {
                                self.session_pane_roots.insert(first.clone(), node);
                            }
                        } else if let WorkspacePaneNode::Leaf { session_id } = node {
                            if self.active_session_id.is_none() {
                                self.active_session_id = Some(session_id);
                            }
                        }
                    }
                    None => {}
                }
            }
        }
        self.rebuild_session_tab_owners();
        self.sync_workspace_split_from_active_tab();
        if self.session_pane_roots != before_roots {
            self.persist_workspace_pane_layout();
            if self.startup_restore_complete {
                self.persist_open_tabs();
            }
        }
    }

    fn live_session_ids_with_disconnected(&self) -> HashSet<String> {
        let mut live = self.live_session_ids();
        for (session_id, metadata) in &self.session_metadata {
            if metadata.disconnected {
                live.insert(session_id.clone());
            }
        }
        live
    }

    /// Rebuild leaf→tab-root ownership from `session_pane_roots`.
    pub(in crate::ui::view) fn rebuild_session_tab_owners(&mut self) {
        self.session_tab_owner.clear();
        let roots: Vec<(String, WorkspacePaneNode)> = self
            .session_pane_roots
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        for (tab_root, tree) in roots {
            for leaf in tree.session_ids() {
                self.session_tab_owner.insert(leaf, tab_root.clone());
            }
        }
    }

    /// Expose the active tab's pane tree via `workspace_split` for existing renderers.
    pub(in crate::ui::view) fn sync_workspace_split_from_active_tab(&mut self) {
        let Some(active) = self.active_session_id.clone() else {
            self.workspace_split = None;
            return;
        };
        let tab_root = self.tab_root_for_session(&active);
        self.workspace_split = self
            .session_pane_roots
            .get(&tab_root)
            .filter(|root| root.is_split())
            .cloned();
    }

    fn write_back_active_tab_pane_root(&mut self) {
        let Some(active) = self.active_session_id.clone() else {
            return;
        };
        let tab_root = self.tab_root_for_session(&active);
        if let Some(root) = self.workspace_split.clone() {
            if root.is_split() {
                self.session_pane_roots.insert(tab_root, root);
                self.rebuild_session_tab_owners();
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
        let tab_root = self.tab_root_for_session(&primary_session_id);
        if let Some(root) = self.session_pane_roots.get_mut(&tab_root) {
            if root.split_leaf(
                &primary_session_id,
                secondary_session_id.clone(),
                direction,
                split_id.clone(),
            ) {
                self.rebuild_session_tab_owners();
                self.activate_session_id(&secondary_session_id);
                self.sync_workspace_split_from_active_tab();
                self.selected_nav = NavItem::Workspace;
                self.main_mode = MainMode::Workspace;
                self.persist_workspace_pane_layout();
                if self.startup_restore_complete {
                    self.persist_open_tabs();
                }
                return;
            }
        }
        // Create a new per-tab dual split rooted at the primary session tab.
        let root = WorkspacePaneNode::Split {
            id: split_id,
            direction,
            ratio_percent: WorkspacePaneNode::DEFAULT_RATIO_PERCENT,
            first: Box::new(WorkspacePaneNode::leaf(primary_session_id.clone())),
            second: Box::new(WorkspacePaneNode::leaf(secondary_session_id.clone())),
        };
        self.session_pane_roots.insert(tab_root, root);
        self.rebuild_session_tab_owners();
        self.activate_session_id(&secondary_session_id);
        self.sync_workspace_split_from_active_tab();
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
        self.persist_workspace_pane_layout();
        if self.startup_restore_complete {
            self.persist_open_tabs();
        }
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
        self.sync_workspace_split_from_active_tab();
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
        self.terminal_status = format!("focused pane {}", short_id(&session_id));
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
            self.write_back_active_tab_pane_root();
            self.persist_workspace_pane_layout();
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
        let Some(active_id) = self.active_session_id.clone() else {
            self.terminal_status = "workspace is not split".to_string();
            cx.notify();
            return;
        };
        let tab_root = self.tab_root_for_session(&active_id);
        let Some(root) = self.session_pane_roots.remove(&tab_root) else {
            self.workspace_split = None;
            self.terminal_status = "workspace is not split".to_string();
            cx.notify();
            return;
        };
        self.workspace_split_resize = None;

        if let Some(collapsed) = collapse_around_session(root.clone(), &active_id) {
            match collapsed {
                WorkspacePaneNode::Split { .. } => {
                    self.session_pane_roots.insert(tab_root, collapsed);
                    self.terminal_status = "collapsed focused split".to_string();
                }
                WorkspacePaneNode::Leaf { session_id } => {
                    self.activate_session_id(&session_id);
                    self.terminal_status = "workspace split closed".to_string();
                }
            }
            self.rebuild_session_tab_owners();
            self.sync_workspace_split_from_active_tab();
            self.persist_workspace_pane_layout();
            if self.startup_restore_complete {
                self.persist_open_tabs();
            }
            cx.notify();
            return;
        }

        let _ = root;
        self.rebuild_session_tab_owners();
        self.sync_workspace_split_from_active_tab();
        self.terminal_status = "workspace split closed".to_string();
        self.persist_workspace_pane_layout();
        if self.startup_restore_complete {
            self.persist_open_tabs();
        }
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
        // Prefer multi-leaf tab-window splits when active; otherwise pane splits.
        let (direction, start_ratio) = if let Some(root) = self.terminal_windows.as_ref() {
            match (
                root.direction_for_split(&split_id),
                root.ratio_for_split(&split_id),
            ) {
                (Some(direction), Some(start_ratio)) => (direction, start_ratio),
                _ => {
                    let Some(root) = self.workspace_split.as_ref() else {
                        return;
                    };
                    let Some(direction) = root.direction_for_split(&split_id) else {
                        return;
                    };
                    let Some(start_ratio) = root.ratio_for_split(&split_id) else {
                        return;
                    };
                    (direction, start_ratio)
                }
            }
        } else {
            let Some(root) = self.workspace_split.as_ref() else {
                return;
            };
            let Some(direction) = root.direction_for_split(&split_id) else {
                return;
            };
            let Some(start_ratio) = root.ratio_for_split(&split_id) else {
                return;
            };
            (direction, start_ratio)
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
        let mut applied = false;
        if let Some(root) = self.terminal_windows.as_mut() {
            if root.set_ratio_for_split(&state.split_id, next) {
                applied = true;
            }
        }
        if !applied {
            if let Some(root) = self.workspace_split.as_mut() {
                if root.set_ratio_for_split(&state.split_id, next) {
                    applied = true;
                    self.write_back_active_tab_pane_root();
                }
            }
        }
        if applied {
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
            let ratio = self
                .terminal_windows
                .as_ref()
                .and_then(|root| root.ratio_for_split(&state.split_id))
                .or_else(|| {
                    self.workspace_split
                        .as_ref()
                        .and_then(|root| root.ratio_for_split(&state.split_id))
                });
            if let Some(ratio) = ratio {
                self.terminal_status = format!("split ratio set to {ratio}%");
            }
            if self.terminal_windows_is_multi_leaf() {
                self.persist_terminal_window_layout();
            } else if self.workspace_split.as_ref().is_some_and(|root| root.is_split()) {
                self.write_back_active_tab_pane_root();
                self.persist_workspace_pane_layout();
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
        let palette = self.theme_palette();
        let accent = rgb(palette.accent);
        let id = SharedString::from(format!("workspace-split-resize-{split_id}"));
        match direction {
            WorkspaceSplitDirection::Horizontal => div()
                .id(id)
                .h(px(5.))
                .flex_none()
                .w_full()
                .rounded_sm()
                .bg(rgb(palette.border))
                .cursor_row_resize()
                .hover(move |this| this.bg(accent))
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
                .bg(rgb(palette.border))
                .cursor_col_resize()
                .hover(move |this| this.bg(accent))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                        this.start_workspace_split_resize(split_id.clone(), event, cx);
                    }),
                )
                .into_any_element(),
        }
    }
    pub(in crate::ui::view) fn persist_workspace_pane_layout(&mut self) {
        if !self.settings.startup_restore || !self.settings.startup_restore_window_layout {
            return;
        }
        if !self.startup_restore_complete {
            return;
        }
        self.sync_workspace_split_from_active_tab();
        // Prefer serializing a global layout when a single split covers every tab-root leaf.
        // Otherwise store the active tab's split against ordered_sessions indexes (legacy key).
        let ordered = self
            .ordered_sessions()
            .into_iter()
            .map(|session| session.id)
            .collect::<Vec<_>>();
        let layout = self
            .workspace_split
            .as_ref()
            .filter(|root| root.is_split())
            .and_then(|root| root.serialize_layout(&ordered))
            .or_else(|| {
                self.session_pane_roots
                    .values()
                    .find(|root| root.is_split())
                    .and_then(|root| root.serialize_layout(&ordered))
            });
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_workspace_pane_layout(layout.as_ref()))
        {
            Ok(()) => {}
            Err(error) => {
                self.terminal_status = format!("failed to save pane layout: {error}");
            }
        }
    }

    pub(in crate::ui::view) fn try_restore_workspace_pane_layout(&mut self) {
        if self.workspace_pane_layout_restored {
            return;
        }
        if !self.settings.startup_restore || !self.settings.startup_restore_window_layout {
            self.workspace_pane_layout_restored = true;
            return;
        }
        // Multi-leaf tab windows take visual precedence; skip pane restore when active.
        if self.terminal_windows_is_multi_leaf() {
            self.workspace_pane_layout_restored = true;
            return;
        }
        let ordered = self
            .ordered_sessions()
            .into_iter()
            .map(|session| session.id)
            .collect::<Vec<_>>();
        if ordered.len() < 2 {
            // After startup finishes, don't keep waiting forever for a second tab.
            if self.startup_restore_complete {
                self.workspace_pane_layout_restored = true;
            }
            return;
        }
        self.workspace_pane_layout_restored = true;
        let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) else {
            return;
        };
        let Ok(Some(layout)) = store.load_workspace_pane_layout() else {
            return;
        };
        let Some(restored) = WorkspacePaneNode::restore_layout(&layout, &ordered) else {
            return;
        };
        if !restored.is_split() {
            return;
        }
        // Prefer active session still present in the restored tree.
        if let Some(active) = self.active_session_id.clone() {
            if !restored.contains_session(&active) {
                if let Some(first) = restored.session_ids().into_iter().next() {
                    self.active_session_id = Some(first);
                }
            }
        } else if let Some(first) = restored.session_ids().into_iter().next() {
            self.active_session_id = Some(first);
        }
        // Install as a per-tab root under the first leaf (legacy global layout path).
        if let Some(first) = restored.session_ids().into_iter().next() {
            self.session_pane_roots.insert(first, restored);
            self.rebuild_session_tab_owners();
            self.sync_workspace_split_from_active_tab();
        }
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
        self.terminal_status = "restored workspace pane layout".to_string();
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

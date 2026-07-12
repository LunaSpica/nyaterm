use super::*;
use gpui::{FontWeight, MouseDownEvent, Render, Window, rgba};
use nyaterm_domain::truncate_preview;

#[derive(Clone, Debug)]
pub(in crate::ui::view) struct ConnectionDragPayload {
    pub kind: ConnectionDragKind,
    pub id: String,
    pub label: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::ui::view) enum ConnectionDragKind {
    Connection,
    Group,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ui::view) enum ConnectionDropPosition {
    Before,
    After,
    Inside,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ui::view) struct ConnectionDropTarget {
    pub id: Option<String>,
    pub kind: ConnectionDragKind,
    pub position: ConnectionDropPosition,
}

pub(in crate::ui::view) struct ConnectionDragPreview {
    payload: ConnectionDragPayload,
    position: gpui::Point<gpui::Pixels>,
}

impl ConnectionDragPreview {
    pub(in crate::ui::view) fn new(
        payload: ConnectionDragPayload,
        position: gpui::Point<gpui::Pixels>,
    ) -> Self {
        Self { payload, position }
    }
}

impl Render for ConnectionDragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (kind, accent) = match self.payload.kind {
            ConnectionDragKind::Connection => ("⌂", rgb(0x3fb950)),
            ConnectionDragKind::Group => ("▸", rgb(0x58a6ff)),
        };
        div()
            .pl(self.position.x - px(90.))
            .pt(self.position.y - px(16.))
            .child(
                div()
                    .w(px(200.))
                    .h(px(36.))
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x388bfd))
                    .bg(rgba(0x0d1117ee))
                    .shadow_lg()
                    .child(div().text_size(px(13.)).text_color(accent).child(kind))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(12.))
                            .font_weight(FontWeight(600.))
                            .text_color(rgb(0xe5edf7))
                            .child(truncate_preview(&self.payload.label, 24)),
                    ),
            )
    }
}

impl NyaTermApp {
    pub(in crate::ui::view) fn selected_connections(&self) -> Vec<SavedConnection> {
        let visible_ids = self
            .connections
            .iter()
            .map(|connection| connection.id.as_str())
            .collect::<HashSet<_>>();
        self.connections
            .iter()
            .filter(|connection| self.selected_connection_ids.contains(&connection.id))
            .cloned()
            .chain(
                self.selected_connection_ids
                    .iter()
                    .filter(|id| !visible_ids.contains(id.as_str()))
                    .filter_map(|id| {
                        self.connections
                            .iter()
                            .find(|connection| connection.id == *id)
                            .cloned()
                    }),
            )
            .collect()
    }

    /// Tauri connection selection: plain click replaces, Ctrl/Cmd toggles, Shift ranges.
    pub(in crate::ui::view) fn select_connection(
        &mut self,
        connection_id: String,
        additive: bool,
        range: bool,
        cx: &mut Context<Self>,
    ) {
        let visible_ids = self.visible_connection_ids();
        if range {
            let anchor = self
                .last_selected_connection_id
                .clone()
                .unwrap_or_else(|| connection_id.clone());
            let mut next = if additive {
                self.selected_connection_ids.clone()
            } else {
                HashSet::new()
            };
            if let (Some(start), Some(end)) = (
                visible_ids.iter().position(|id| id == &anchor),
                visible_ids.iter().position(|id| id == &connection_id),
            ) {
                let (lo, hi) = if start <= end {
                    (start, end)
                } else {
                    (end, start)
                };
                for id in &visible_ids[lo..=hi] {
                    next.insert(id.clone());
                }
            } else {
                next.insert(connection_id.clone());
            }
            self.selected_connection_ids = next;
        } else if additive {
            if self.selected_connection_ids.contains(&connection_id) {
                self.selected_connection_ids.remove(&connection_id);
            } else {
                self.selected_connection_ids.insert(connection_id.clone());
            }
        } else {
            self.selected_connection_ids.clear();
            self.selected_connection_ids.insert(connection_id.clone());
        }
        self.last_selected_connection_id = Some(connection_id);
        let count = self.selected_connection_ids.len();
        self.terminal_status = if count == 0 {
            "connection selection cleared".to_string()
        } else {
            format!("selected {count} connection(s)")
        };
        cx.notify();
    }

    pub(in crate::ui::view) fn visible_connection_ids(&self) -> Vec<String> {
        let query = self.connection_search_draft.trim().to_ascii_lowercase();
        // Mirror `connection_sections` ordering for Shift-range selection.
        let mut by_group: std::collections::HashMap<Option<String>, Vec<&SavedConnection>> =
            std::collections::HashMap::new();
        for connection in &self.connections {
            if !query.is_empty() {
                let haystack = format!(
                    "{} {} {} {} {}",
                    connection.name,
                    connection.endpoint(),
                    connection.kind_label(),
                    connection.description.clone().unwrap_or_default(),
                    connection.id
                )
                .to_ascii_lowercase();
                if !haystack.contains(&query) {
                    continue;
                }
            }
            by_group
                .entry(connection.group_id.clone())
                .or_default()
                .push(connection);
        }
        for list in by_group.values_mut() {
            list.sort_by(|left, right| match self.connection_sort_mode {
                crate::ui::models::ConnectionSortMode::Default => {
                    left.sort_order.cmp(&right.sort_order).then_with(|| {
                        left.name
                            .to_ascii_lowercase()
                            .cmp(&right.name.to_ascii_lowercase())
                    })
                }
                crate::ui::models::ConnectionSortMode::NameAsc => left
                    .name
                    .to_ascii_lowercase()
                    .cmp(&right.name.to_ascii_lowercase()),
                crate::ui::models::ConnectionSortMode::NameDesc => right
                    .name
                    .to_ascii_lowercase()
                    .cmp(&left.name.to_ascii_lowercase()),
                crate::ui::models::ConnectionSortMode::Recent => right
                    .last_used_at_ms
                    .unwrap_or(0)
                    .cmp(&left.last_used_at_ms.unwrap_or(0))
                    .then_with(|| {
                        left.name
                            .to_ascii_lowercase()
                            .cmp(&right.name.to_ascii_lowercase())
                    }),
            });
        }

        let mut ids = Vec::new();
        let mut ordered_groups = self.connection_groups.clone();
        ordered_groups.sort_by(|left, right| {
            left.sort_order.cmp(&right.sort_order).then_with(|| {
                left.name
                    .to_ascii_lowercase()
                    .cmp(&right.name.to_ascii_lowercase())
            })
        });
        // Groups first, ungrouped last (matches connection_sections / Tauri).
        for group in ordered_groups {
            if let Some(list) = by_group.remove(&Some(group.id)) {
                for connection in list {
                    ids.push(connection.id.clone());
                }
            }
        }
        if let Some(root) = by_group.remove(&None) {
            for connection in root {
                ids.push(connection.id.clone());
            }
        }
        for list in by_group.into_values() {
            for connection in list {
                ids.push(connection.id.clone());
            }
        }
        ids
    }

    pub(in crate::ui::view) fn select_all_connections(&mut self, cx: &mut Context<Self>) {
        self.selected_connection_ids = self
            .connections
            .iter()
            .map(|connection| connection.id.clone())
            .collect();
        self.terminal_status = format!(
            "selected {} connection(s)",
            self.selected_connection_ids.len()
        );
        cx.notify();
    }

    pub(in crate::ui::view) fn clear_selected_connections(&mut self, cx: &mut Context<Self>) {
        self.selected_connection_ids.clear();
        self.last_selected_connection_id = None;
        self.terminal_status = "connection selection cleared".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn copy_selected_connections(&mut self, cx: &mut Context<Self>) {
        let selected = self.selected_connections();
        if selected.is_empty() {
            self.terminal_status = "select saved connections before copying".to_string();
            cx.notify();
            return;
        }

        match self.copy_connections_to_store(&selected) {
            Ok(count) => {
                self.selected_connection_ids.clear();
                self.terminal_status = format!("copied {count} saved connection(s)");
            }
            Err(error) => {
                self.terminal_status = format!("copy selected connections failed: {error}");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    fn copy_connections_to_store(
        &mut self,
        connections: &[SavedConnection],
    ) -> Result<usize, String> {
        let store = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .map_err(|error| error.to_string())?;

        for connection in connections {
            let mut copy = connection.clone();
            copy.id = uuid();
            copy.name = format!("{} (copy)", connection.name);
            copy.created_at_ms = None;
            copy.updated_at_ms = None;
            copy.last_used_at_ms = None;
            if let Some(auth) = copy.auth.as_mut() {
                auth.password = None;
                auth.password_id = None;
                auth.has_password = false;
            }
            store
                .save_connection(&copy)
                .map_err(|error| error.to_string())?;
        }

        self.connections = store
            .load_sessions()
            .map_err(|error| error.to_string())?
            .connections;
        self.store_status.message = "saved connections copied".to_string();
        self.store_status.ready = true;
        Ok(connections.len())
    }
}

impl NyaTermApp {
    pub(in crate::ui::view) fn open_connection_context_menu(
        &mut self,
        connection_id: String,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.connections_more_menu_open = false;
        self.connection_group_context_menu = None;
        self.connection_details_tooltip_id = None;
        if !self.selected_connection_ids.contains(&connection_id) {
            self.selected_connection_ids.clear();
            self.selected_connection_ids.insert(connection_id.clone());
            self.last_selected_connection_id = Some(connection_id.clone());
        }
        self.connection_context_menu = Some(ConnectionContextMenuState {
            connection_id,
            x: event.position.x,
            y: event.position.y,
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn open_connection_group_context_menu(
        &mut self,
        group_id: String,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.connections_more_menu_open = false;
        self.connection_context_menu = None;
        self.connection_group_context_menu = Some(ConnectionGroupContextMenuState {
            group_id,
            x: event.position.x,
            y: event.position.y,
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn close_connection_context_menus(&mut self, cx: &mut Context<Self>) {
        self.connection_context_menu = None;
        self.connection_group_context_menu = None;
        cx.notify();
    }

    pub(in crate::ui::view) fn copy_connection_by_id(
        &mut self,
        connection_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(connection) = self
            .connections
            .iter()
            .find(|connection| connection.id == connection_id)
            .cloned()
        else {
            self.terminal_status = "connection is no longer available".to_string();
            cx.notify();
            return;
        };
        match self.copy_connections_to_store(&[connection]) {
            Ok(count) => {
                self.terminal_status = format!("copied {count} saved connection(s)");
            }
            Err(error) => {
                self.terminal_status = format!("copy connection failed: {error}");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn start_selected_saved_connections(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selected = self.selected_connections();
        if selected.is_empty() {
            self.terminal_status = "select saved connections before connecting".to_string();
            cx.notify();
            return;
        }
        // Connect first selected immediately; remaining are queued sequentially via status for now.
        // Tauri opens all; we open first and report count to avoid multi-pending races.
        let first = selected[0].clone();
        let remaining = selected.len().saturating_sub(1);
        self.start_saved_connection(first, window, cx);
        if remaining > 0 {
            // Best-effort open remaining after first is started (may still be pending).
            for connection in selected.into_iter().skip(1) {
                self.start_saved_connection(connection, window, cx);
            }
            self.terminal_status = format!("opening {} connection(s)", remaining + 1);
        }
    }

    pub(in crate::ui::view) fn start_group_connections(
        &mut self,
        group_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut group_ids = std::collections::HashSet::from([group_id.clone()]);
        let mut changed = true;
        while changed {
            changed = false;
            for group in &self.connection_groups {
                if let Some(parent) = group.parent_id.as_ref() {
                    if group_ids.contains(parent) && group_ids.insert(group.id.clone()) {
                        changed = true;
                    }
                }
            }
        }
        let connections = self
            .connections
            .iter()
            .filter(|connection| {
                connection
                    .group_id
                    .as_ref()
                    .is_some_and(|id| group_ids.contains(id))
            })
            .cloned()
            .collect::<Vec<_>>();
        if connections.is_empty() {
            self.terminal_status = "group has no connections".to_string();
            cx.notify();
            return;
        }
        let total = connections.len();
        for connection in connections {
            self.start_saved_connection(connection, window, cx);
        }
        self.terminal_status = format!("opening {total} connection(s) from group");
    }
}

impl NyaTermApp {
    pub(in crate::ui::view) fn move_connection_before(
        &mut self,
        source_id: String,
        target_id: String,
        cx: &mut Context<Self>,
    ) {
        self.connection_drop_target = None;
        if source_id == target_id {
            return;
        }
        let Some(source) = self.connections.iter().find(|c| c.id == source_id).cloned() else {
            self.terminal_status = "drag source connection missing".to_string();
            cx.notify();
            return;
        };
        let Some(target) = self.connections.iter().find(|c| c.id == target_id).cloned() else {
            self.terminal_status = "drop target connection missing".to_string();
            cx.notify();
            return;
        };

        let parent = target.group_id.clone();
        let mut siblings = self
            .connections
            .iter()
            .filter(|c| c.group_id == parent && c.id != source_id)
            .cloned()
            .collect::<Vec<_>>();
        siblings.sort_by(|a, b| {
            a.sort_order.cmp(&b.sort_order).then_with(|| {
                a.name
                    .to_ascii_lowercase()
                    .cmp(&b.name.to_ascii_lowercase())
            })
        });
        let target_idx = siblings
            .iter()
            .position(|c| c.id == target_id)
            .unwrap_or(siblings.len());
        let mut moved = source;
        moved.group_id = parent;
        siblings.insert(target_idx, moved);

        match self.persist_connection_order(&siblings) {
            Ok(()) => {
                self.refresh_store_from_runtime();
                self.terminal_status = "connection reordered".to_string();
            }
            Err(error) => {
                self.terminal_status = format!("reorder connection failed: {error}");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
            }
        }
        self.connection_drop_target = None;
        cx.notify();
    }

    pub(in crate::ui::view) fn move_connection_after(
        &mut self,
        source_id: String,
        target_id: String,
        cx: &mut Context<Self>,
    ) {
        self.connection_drop_target = None;
        if source_id == target_id {
            return;
        }
        let Some(source) = self.connections.iter().find(|c| c.id == source_id).cloned() else {
            self.terminal_status = "drag source connection missing".to_string();
            cx.notify();
            return;
        };
        let Some(target) = self.connections.iter().find(|c| c.id == target_id).cloned() else {
            self.terminal_status = "drop target connection missing".to_string();
            cx.notify();
            return;
        };

        let parent = target.group_id.clone();
        let mut siblings = self
            .connections
            .iter()
            .filter(|c| c.group_id == parent && c.id != source_id)
            .cloned()
            .collect::<Vec<_>>();
        siblings.sort_by(|a, b| {
            a.sort_order.cmp(&b.sort_order).then_with(|| {
                a.name
                    .to_ascii_lowercase()
                    .cmp(&b.name.to_ascii_lowercase())
            })
        });
        let target_idx = siblings
            .iter()
            .position(|c| c.id == target_id)
            .map(|idx| idx + 1)
            .unwrap_or(siblings.len());
        let mut moved = source;
        moved.group_id = parent;
        siblings.insert(target_idx.min(siblings.len()), moved);

        match self.persist_connection_order(&siblings) {
            Ok(()) => {
                self.refresh_store_from_runtime();
                self.terminal_status = "connection reordered".to_string();
            }
            Err(error) => {
                self.terminal_status = format!("reorder connection failed: {error}");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
            }
        }
        self.connection_drop_target = None;
        cx.notify();
    }

    pub(in crate::ui::view) fn move_connection_into_group(
        &mut self,
        source_id: String,
        group_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.connection_drop_target = None;
        let Some(source) = self.connections.iter().find(|c| c.id == source_id).cloned() else {
            self.terminal_status = "drag source connection missing".to_string();
            cx.notify();
            return;
        };
        if source.group_id == group_id {
            // already there: append to end of group order
        }
        let mut siblings = self
            .connections
            .iter()
            .filter(|c| c.group_id == group_id && c.id != source_id)
            .cloned()
            .collect::<Vec<_>>();
        siblings.sort_by(|a, b| a.sort_order.cmp(&b.sort_order));
        let mut moved = source;
        moved.group_id = group_id.clone();
        siblings.push(moved);

        match self.persist_connection_order(&siblings) {
            Ok(()) => {
                if let Some(gid) = group_id {
                    self.expanded_connection_groups.insert(gid);
                }
                self.refresh_store_from_runtime();
                self.terminal_status = "connection moved".to_string();
            }
            Err(error) => {
                self.terminal_status = format!("move connection failed: {error}");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn move_group_before(
        &mut self,
        source_id: String,
        target_id: String,
        cx: &mut Context<Self>,
    ) {
        self.connection_drop_target = None;
        if source_id == target_id {
            return;
        }
        let Some(source) = self
            .connection_groups
            .iter()
            .find(|g| g.id == source_id)
            .cloned()
        else {
            return;
        };
        let Some(target) = self
            .connection_groups
            .iter()
            .find(|g| g.id == target_id)
            .cloned()
        else {
            return;
        };
        // Only reorder among same parent for now.
        let parent = target.parent_id.clone();
        let mut siblings = self
            .connection_groups
            .iter()
            .filter(|g| g.parent_id == parent && g.id != source_id)
            .cloned()
            .collect::<Vec<_>>();
        siblings.sort_by(|a, b| a.sort_order.cmp(&b.sort_order));
        let target_idx = siblings
            .iter()
            .position(|g| g.id == target_id)
            .unwrap_or(siblings.len());
        let mut moved = source;
        moved.parent_id = parent;
        siblings.insert(target_idx, moved);
        match self.persist_group_order(&siblings) {
            Ok(()) => {
                self.refresh_store_from_runtime();
                self.terminal_status = "group reordered".to_string();
            }
            Err(error) => {
                self.terminal_status = format!("reorder group failed: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn move_group_into_group(
        &mut self,
        source_id: String,
        parent_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.connection_drop_target = None;
        if parent_id.as_deref() == Some(source_id.as_str()) {
            self.terminal_status = "cannot nest group into itself".to_string();
            cx.notify();
            return;
        }
        // Prevent cycles: parent cannot be descendant of source.
        if let Some(pid) = parent_id.as_ref() {
            if self.group_is_descendant(pid, &source_id) {
                self.terminal_status = "cannot create group cycle".to_string();
                cx.notify();
                return;
            }
        }
        let Some(source) = self
            .connection_groups
            .iter()
            .find(|g| g.id == source_id)
            .cloned()
        else {
            return;
        };
        let mut siblings = self
            .connection_groups
            .iter()
            .filter(|g| g.parent_id == parent_id && g.id != source_id)
            .cloned()
            .collect::<Vec<_>>();
        siblings.sort_by(|a, b| a.sort_order.cmp(&b.sort_order));
        let mut moved = source;
        moved.parent_id = parent_id;
        siblings.push(moved);
        match self.persist_group_order(&siblings) {
            Ok(()) => {
                self.refresh_store_from_runtime();
                self.terminal_status = "group moved".to_string();
            }
            Err(error) => {
                self.terminal_status = format!("move group failed: {error}");
            }
        }
        cx.notify();
    }

    fn group_is_descendant(&self, candidate_id: &str, ancestor_id: &str) -> bool {
        let mut current = Some(candidate_id.to_string());
        let mut guard = 0;
        while let Some(id) = current {
            if id == ancestor_id {
                return true;
            }
            guard += 1;
            if guard > 64 {
                break;
            }
            current = self
                .connection_groups
                .iter()
                .find(|g| g.id == id)
                .and_then(|g| g.parent_id.clone());
        }
        false
    }

    fn persist_connection_order(&mut self, ordered: &[SavedConnection]) -> Result<(), String> {
        self.with_connection_store(|store| {
            for (index, connection) in ordered.iter().enumerate() {
                let mut updated = connection.clone();
                updated.sort_order = index as i32;
                store.save_connection(&updated)?;
            }
            Ok(())
        })
    }

    fn persist_group_order(&mut self, ordered: &[Group]) -> Result<(), String> {
        self.with_connection_store(|store| {
            for (index, group) in ordered.iter().enumerate() {
                let mut updated = group.clone();
                updated.sort_order = index as i32;
                store.save_group(&updated)?;
            }
            Ok(())
        })
    }
}

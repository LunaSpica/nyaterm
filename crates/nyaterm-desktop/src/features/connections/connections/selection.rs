use std::collections::HashSet;

use gpui::Context;
use nyaterm_core::{ConnectionStore, SavedConnection, uuid};

use crate::features::NyaTermApp;

impl NyaTermApp {
    pub(in crate::features) fn selected_connections(&self) -> Vec<SavedConnection> {
        let visible_ids = self
            .connections
            .iter()
            .map(|connection| connection.id.as_str())
            .collect::<HashSet<_>>();
        self.connections
            .iter()
            .filter(|connection| {
                self.connection_state
                    .list
                    .contains_selected_id(&connection.id)
            })
            .cloned()
            .chain(
                self.connection_state
                    .list
                    .selected_connection_ids()
                    .filter(|id| !visible_ids.contains(*id))
                    .filter_map(|id| {
                        self.connections
                            .iter()
                            .find(|connection| connection.id == id)
                            .cloned()
                    }),
            )
            .collect()
    }

    /// Tauri connection selection: plain click replaces, Ctrl/Cmd toggles, Shift ranges.

    pub(in crate::features) fn select_connection(
        &mut self,
        connection_id: String,
        additive: bool,
        range: bool,
        cx: &mut Context<Self>,
    ) {
        let visible_ids = self.visible_connection_ids();
        let count = self.connection_state.list.select_connection(
            connection_id,
            &visible_ids,
            additive,
            range,
        );
        self.terminal.view.status = if count == 0 {
            "connection selection cleared".to_string()
        } else {
            format!("selected {count} connection(s)")
        };
        cx.notify();
    }

    pub(in crate::features) fn visible_connection_ids(&self) -> Vec<String> {
        let query = self.connection_state.list.search_query();
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
            list.sort_by(|left, right| match self.connection_state.list.sort_mode() {
                crate::models::ConnectionSortMode::Default => {
                    left.sort_order.cmp(&right.sort_order).then_with(|| {
                        left.name
                            .to_ascii_lowercase()
                            .cmp(&right.name.to_ascii_lowercase())
                    })
                }
                crate::models::ConnectionSortMode::NameAsc => left
                    .name
                    .to_ascii_lowercase()
                    .cmp(&right.name.to_ascii_lowercase()),
                crate::models::ConnectionSortMode::NameDesc => right
                    .name
                    .to_ascii_lowercase()
                    .cmp(&left.name.to_ascii_lowercase()),
            });
        }

        let group_ids = self
            .connection_groups
            .iter()
            .map(|group| group.id.clone())
            .collect::<std::collections::HashSet<_>>();
        let mut children_by_parent: std::collections::HashMap<
            Option<String>,
            Vec<nyaterm_core::Group>,
        > = std::collections::HashMap::new();
        for group in &self.connection_groups {
            let parent_id = group
                .parent_id
                .clone()
                .filter(|parent_id| group_ids.contains(parent_id));
            let mut group = group.clone();
            group.parent_id = parent_id.clone();
            children_by_parent.entry(parent_id).or_default().push(group);
        }
        for groups in children_by_parent.values_mut() {
            groups.sort_by(|left, right| {
                left.sort_order.cmp(&right.sort_order).then_with(|| {
                    left.name
                        .to_ascii_lowercase()
                        .cmp(&right.name.to_ascii_lowercase())
                })
            });
        }

        let mut ids = Vec::new();
        let mut visited = std::collections::HashSet::new();
        // Groups first, ungrouped last (matches connection_sections / Tauri).
        for group in children_by_parent.get(&None).cloned().unwrap_or_default() {
            append_visible_connection_ids(
                group,
                &children_by_parent,
                &mut by_group,
                &mut ids,
                &mut visited,
            );
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

    pub(in crate::features) fn clear_selected_connections(&mut self, cx: &mut Context<Self>) {
        self.connection_state.list.clear_selection();
        self.terminal.view.status = "connection selection cleared".to_string();
        cx.notify();
    }

    pub(in crate::features) fn copy_selected_connections(&mut self, cx: &mut Context<Self>) {
        let selected = self.selected_connections();
        if selected.is_empty() {
            self.terminal.view.status = "select saved connections before copying".to_string();
            cx.notify();
            return;
        }

        match self.copy_connections_to_store(&selected) {
            Ok(count) => {
                self.connection_state.list.clear_selection();
                self.terminal.view.status = format!("copied {count} saved connection(s)");
            }
            Err(error) => {
                self.terminal.view.status = format!("copy selected connections failed: {error}");
                self.store_status.message = self.terminal.view.status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn copy_connections_to_store(
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

fn append_visible_connection_ids<'a>(
    group: nyaterm_core::Group,
    children_by_parent: &std::collections::HashMap<Option<String>, Vec<nyaterm_core::Group>>,
    by_group: &mut std::collections::HashMap<Option<String>, Vec<&'a SavedConnection>>,
    ids: &mut Vec<String>,
    visited: &mut std::collections::HashSet<String>,
) {
    if !visited.insert(group.id.clone()) {
        return;
    }
    for child in children_by_parent
        .get(&Some(group.id.clone()))
        .cloned()
        .unwrap_or_default()
    {
        append_visible_connection_ids(child, children_by_parent, by_group, ids, visited);
    }
    if let Some(list) = by_group.remove(&Some(group.id)) {
        for connection in list {
            ids.push(connection.id.clone());
        }
    }
}

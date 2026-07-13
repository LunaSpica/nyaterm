use super::*;

impl NyaTermApp {
    pub(in crate::features) fn selected_connections(&self) -> Vec<SavedConnection> {
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

    pub(in crate::features) fn select_connection(
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

    pub(in crate::features) fn visible_connection_ids(&self) -> Vec<String> {
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
                crate::models::ConnectionSortMode::Recent => right
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

    pub(in crate::features) fn select_all_connections(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn clear_selected_connections(&mut self, cx: &mut Context<Self>) {
        self.selected_connection_ids.clear();
        self.last_selected_connection_id = None;
        self.terminal_status = "connection selection cleared".to_string();
        cx.notify();
    }

    pub(in crate::features) fn copy_selected_connections(&mut self, cx: &mut Context<Self>) {
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

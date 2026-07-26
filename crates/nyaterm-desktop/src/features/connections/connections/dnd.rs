use gpui::Context;
use nyaterm_core::{Group, SavedConnection};

use crate::features::NyaTermApp;

/// The target group's connections in their new order after a move.
///
/// Existing members keep their relative order and the moved connections are
/// appended in list order — not in whatever order the selection happened to be
/// built — so a multi-select move is not silently shuffled.
fn connections_reordered_into_group(
    connections: &[SavedConnection],
    source_ids: &[String],
    group_id: &Option<String>,
) -> Vec<SavedConnection> {
    let mut staying = connections
        .iter()
        .filter(|connection| {
            &connection.group_id == group_id && !source_ids.contains(&connection.id)
        })
        .cloned()
        .collect::<Vec<_>>();
    staying.sort_by(|left, right| left.sort_order.cmp(&right.sort_order));

    for connection in connections
        .iter()
        .filter(|connection| source_ids.contains(&connection.id))
    {
        let mut moved = connection.clone();
        moved.group_id = group_id.clone();
        staying.push(moved);
    }
    staying
}

impl NyaTermApp {
    pub(in crate::features) fn move_connection_before(
        &mut self,
        source_id: String,
        target_id: String,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.list.clear_drop_target();
        if source_id == target_id {
            return;
        }
        let Some(source) = self.connections.iter().find(|c| c.id == source_id).cloned() else {
            self.terminal.view.status = "drag source connection missing".to_string();
            cx.notify();
            return;
        };
        let Some(target) = self.connections.iter().find(|c| c.id == target_id).cloned() else {
            self.terminal.view.status = "drop target connection missing".to_string();
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
                self.terminal.view.status = "connection reordered".to_string();
            }
            Err(error) => {
                self.terminal.view.status = format!("reorder connection failed: {error}");
                self.store_status.message = self.terminal.view.status.clone();
                self.store_status.ready = false;
            }
        }
        self.connection_state.list.clear_drop_target();
        cx.notify();
    }

    pub(in crate::features) fn move_connection_after(
        &mut self,
        source_id: String,
        target_id: String,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.list.clear_drop_target();
        if source_id == target_id {
            return;
        }
        let Some(source) = self.connections.iter().find(|c| c.id == source_id).cloned() else {
            self.terminal.view.status = "drag source connection missing".to_string();
            cx.notify();
            return;
        };
        let Some(target) = self.connections.iter().find(|c| c.id == target_id).cloned() else {
            self.terminal.view.status = "drop target connection missing".to_string();
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
                self.terminal.view.status = "connection reordered".to_string();
            }
            Err(error) => {
                self.terminal.view.status = format!("reorder connection failed: {error}");
                self.store_status.message = self.terminal.view.status.clone();
                self.store_status.ready = false;
            }
        }
        self.connection_state.list.clear_drop_target();
        cx.notify();
    }

    pub(in crate::features) fn move_connection_into_group(
        &mut self,
        source_id: String,
        group_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.list.clear_drop_target();
        let Some(source) = self.connections.iter().find(|c| c.id == source_id).cloned() else {
            self.terminal.view.status = "drag source connection missing".to_string();
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
                    self.connection_state.list.expand_group(gid);
                }
                self.refresh_store_from_runtime();
                self.terminal.view.status = "connection moved".to_string();
            }
            Err(error) => {
                self.terminal.view.status = format!("move connection failed: {error}");
                self.store_status.message = self.terminal.view.status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    /// Reparent several connections in one write.
    ///
    /// Looping [`Self::move_connection_into_group`] would re-read the list, persist
    /// a fresh order and refresh the store once per connection; the old UI sent a
    /// single reorder. One ordered write also means the list cannot be observed
    /// half-moved.
    pub(in crate::features) fn move_connections_into_group(
        &mut self,
        source_ids: Vec<String>,
        group_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.list.clear_drop_target();
        let moving = self
            .connections
            .iter()
            .filter(|connection| source_ids.contains(&connection.id))
            .cloned()
            .collect::<Vec<_>>();
        if moving.is_empty() {
            self.terminal.view.status = "no connections to move".to_string();
            cx.notify();
            return;
        }

        let moved_count = moving.len();
        let ordered = connections_reordered_into_group(&self.connections, &source_ids, &group_id);

        match self.persist_connection_order(&ordered) {
            Ok(()) => {
                if let Some(group_id) = group_id {
                    self.connection_state.list.expand_group(group_id);
                }
                self.refresh_store_from_runtime();
                self.terminal.view.status = format!("moved {moved_count} connection(s)");
            }
            Err(error) => {
                self.terminal.view.status = format!("move connections failed: {error}");
                self.store_status.message = self.terminal.view.status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn move_group_before(
        &mut self,
        source_id: String,
        target_id: String,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.list.clear_drop_target();
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
                self.terminal.view.status = "group reordered".to_string();
            }
            Err(error) => {
                self.terminal.view.status = format!("reorder group failed: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn move_group_into_group(
        &mut self,
        source_id: String,
        parent_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.list.clear_drop_target();
        if parent_id.as_deref() == Some(source_id.as_str()) {
            self.terminal.view.status = "cannot nest group into itself".to_string();
            cx.notify();
            return;
        }
        // Prevent cycles: parent cannot be descendant of source.
        if let Some(pid) = parent_id.as_ref() {
            if self.group_is_descendant(pid, &source_id) {
                self.terminal.view.status = "cannot create group cycle".to_string();
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
                self.terminal.view.status = "group moved".to_string();
            }
            Err(error) => {
                self.terminal.view.status = format!("move group failed: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn group_is_descendant(
        &self,
        candidate_id: &str,
        ancestor_id: &str,
    ) -> bool {
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

    pub(in crate::features) fn persist_connection_order(
        &mut self,
        ordered: &[SavedConnection],
    ) -> Result<(), String> {
        self.with_connection_store(|store| {
            for (index, connection) in ordered.iter().enumerate() {
                let mut updated = connection.clone();
                updated.sort_order = index as i32;
                store.save_connection(&updated)?;
            }
            Ok(())
        })
    }

    pub(in crate::features) fn persist_group_order(
        &mut self,
        ordered: &[Group],
    ) -> Result<(), String> {
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

#[cfg(test)]
mod tests {
    use nyaterm_core::{AiExecutionProfile, ConnectionType, SavedConnection};

    use super::connections_reordered_into_group;

    fn connection(id: &str, group_id: Option<&str>, sort_order: i32) -> SavedConnection {
        SavedConnection {
            id: id.to_string(),
            name: id.to_string(),
            config: ConnectionType::LocalTerminal {
                shell_path: String::new(),
                shell_args: String::new(),
                working_dir: None,
                ai_execution_profile: AiExecutionProfile::Auto,
            },
            group_id: group_id.map(ToOwned::to_owned),
            description: None,
            sort_order,
            icon: None,
            icon_auto_detect: None,
            auth: None,
            network: None,
            post_login: None,
            created_at_ms: None,
            updated_at_ms: None,
            last_used_at_ms: None,
        }
    }

    fn ids(connections: &[SavedConnection]) -> Vec<&str> {
        connections.iter().map(|c| c.id.as_str()).collect()
    }

    #[test]
    fn moved_connections_land_after_the_group_in_list_order() {
        let connections = vec![
            connection("a", None, 0),
            connection("target-1", Some("target"), 1),
            connection("b", None, 2),
            connection("target-0", Some("target"), 0),
        ];

        let ordered = connections_reordered_into_group(
            &connections,
            // Deliberately reversed: the selection order must not leak through.
            &["b".to_string(), "a".to_string()],
            &Some("target".to_string()),
        );

        assert_eq!(ids(&ordered), vec!["target-0", "target-1", "a", "b"]);
        assert!(
            ordered
                .iter()
                .all(|c| c.group_id.as_deref() == Some("target"))
        );
    }

    #[test]
    fn moving_to_ungrouped_clears_the_group_and_skips_the_movers() {
        let connections = vec![
            connection("root", None, 0),
            connection("grouped", Some("g"), 0),
        ];

        let ordered =
            connections_reordered_into_group(&connections, &["grouped".to_string()], &None);

        assert_eq!(ids(&ordered), vec!["root", "grouped"]);
        assert!(ordered.iter().all(|c| c.group_id.is_none()));
    }

    #[test]
    fn a_connection_already_in_the_target_is_not_duplicated() {
        let connections = vec![
            connection("stay", Some("g"), 0),
            connection("move", Some("g"), 1),
        ];

        let ordered = connections_reordered_into_group(
            &connections,
            &["move".to_string()],
            &Some("g".to_string()),
        );

        assert_eq!(ids(&ordered), vec!["stay", "move"]);
    }
}

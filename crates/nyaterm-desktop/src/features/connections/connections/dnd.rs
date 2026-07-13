use super::*;

impl NyaTermApp {
    pub(in crate::features) fn move_connection_before(
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

    pub(in crate::features) fn move_connection_after(
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

    pub(in crate::features) fn move_connection_into_group(
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

    pub(in crate::features) fn move_group_before(
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

    pub(in crate::features) fn move_group_into_group(
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

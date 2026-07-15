use super::*;

impl NyaTermApp {
    pub(in crate::features) fn open_connection_context_menu(
        &mut self,
        connection_id: String,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.connections_more_menu_open = false;
        self.connection_group_context_menu = None;
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

    pub(in crate::features) fn open_connection_group_context_menu(
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

    pub(in crate::features) fn close_connection_context_menus(&mut self, cx: &mut Context<Self>) {
        self.connection_context_menu = None;
        self.connection_group_context_menu = None;
        cx.notify();
    }

    pub(in crate::features) fn copy_connection_by_id(
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

    pub(in crate::features) fn start_selected_saved_connections(
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

    pub(in crate::features) fn start_group_connections(
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

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
        let queued = self.enqueue_saved_connection_starts(selected, cx);
        self.terminal_status = format!("queued {queued} connection(s)");
        self.drive_saved_connection_start_queue(window, cx);
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
        let queued = self.enqueue_saved_connection_starts(connections, cx);
        self.terminal_status = format!("queued {queued} connection(s) from group");
        self.drive_saved_connection_start_queue(window, cx);
    }

    pub(in crate::features) fn enqueue_saved_connection_start(
        &mut self,
        connection: SavedConnection,
        cx: &mut Context<Self>,
    ) -> bool {
        self.enqueue_saved_connection_start_with_options(
            connection,
            SavedConnectionStartOptions::default(),
            cx,
        )
    }

    pub(in crate::features) fn enqueue_saved_connection_start_with_options(
        &mut self,
        connection: SavedConnection,
        options: SavedConnectionStartOptions,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.saved_connection_start_is_pending_or_queued(&connection) {
            self.terminal_status = format!("{} is already queued", connection.name);
            self.selected_nav = NavItem::Workspace;
            self.main_mode = MainMode::Workspace;
            cx.notify();
            return false;
        }
        let name = connection.name.clone();
        self.pending_saved_connection_queue
            .push_back(PendingSavedConnectionStart {
                connection,
                options,
            });
        self.terminal_status = format!(
            "queued {name} ({} pending)",
            self.pending_saved_connection_queue.len()
        );
        self.selected_nav = NavItem::Workspace;
        self.main_mode = MainMode::Workspace;
        cx.notify();
        true
    }

    fn enqueue_saved_connection_starts(
        &mut self,
        connections: Vec<SavedConnection>,
        cx: &mut Context<Self>,
    ) -> usize {
        let mut queued = 0usize;
        for connection in connections {
            if self.enqueue_saved_connection_start(connection, cx) {
                queued = queued.saturating_add(1);
            }
        }
        queued
    }

    pub(in crate::features) fn drive_saved_connection_start_queue(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.pending_session_name.is_some() {
            return false;
        }
        let mut dirty = false;
        while self.pending_session_name.is_none() {
            let Some(start) = self.pending_saved_connection_queue.pop_front() else {
                return dirty;
            };
            if self.saved_connection_start_is_pending(&start.connection) {
                dirty = true;
                continue;
            }
            let before_pending_count = self.pending_session_starts.len();
            self.start_saved_connection_with_options(start.connection, start.options, window, cx);
            dirty = true;
            if self.pending_session_name.is_some()
                || self.pending_session_starts.len() > before_pending_count
            {
                return true;
            }
        }
        dirty
    }
}

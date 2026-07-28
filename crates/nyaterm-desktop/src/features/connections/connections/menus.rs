use gpui::{Context, MouseDownEvent, Window};
use nyaterm_core::SavedConnection;

use crate::features::{NyaTermApp, PendingSavedConnectionStart, SavedConnectionStartOptions};
use crate::models::{ConnectionGroupOpenConfirmState, MainMode, NavItem};

impl NyaTermApp {
    pub(in crate::features) fn open_connection_context_menu(
        &mut self,
        connection_id: String,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.open_list_connection_context_menu(
            connection_id,
            event.position.x,
            event.position.y,
        );
        cx.notify();
    }

    pub(in crate::features) fn open_connection_list_context_menu(
        &mut self,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.connection_state
            .open_list_background_context_menu(event.position.x, event.position.y);
        cx.notify();
    }

    pub(in crate::features) fn open_connection_group_context_menu(
        &mut self,
        group_id: String,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.open_list_group_context_menu(
            group_id,
            event.position.x,
            event.position.y,
        );
        cx.notify();
    }

    pub(in crate::features) fn close_connection_context_menus(&mut self, cx: &mut Context<Self>) {
        self.connection_state.close_list_context_menus();
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
            self.terminal.view.status = "connection is no longer available".to_string();
            cx.notify();
            return;
        };
        match self.copy_connections_to_store(&[connection]) {
            Ok(count) => {
                self.terminal.view.status = format!("copied {count} saved connection(s)");
            }
            Err(error) => {
                self.terminal.view.status = format!("copy connection failed: {error}");
                self.store_status.message = self.terminal.view.status.clone();
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
        let selected = self
            .connection_state
            .selected_connections(&self.connections);
        if selected.is_empty() {
            self.terminal.view.status = "select saved connections before connecting".to_string();
            cx.notify();
            return;
        }
        let queued = self.enqueue_saved_connection_starts(selected, cx);
        self.terminal.view.status = format!("queued {queued} connection(s)");
        self.drive_saved_connection_start_queue(window, cx);
    }

    pub(in crate::features) fn start_group_connections(
        &mut self,
        group_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let connections = self.connection_state.saved_connections_in_group_tree(
            &self.connections,
            &self.connection_groups,
            &group_id,
        );
        if connections.is_empty() {
            self.terminal.view.status = "group has no connections".to_string();
            cx.notify();
            return;
        }
        let queued = self.enqueue_saved_connection_starts(connections, cx);
        self.terminal.view.status = format!("queued {queued} connection(s) from group");
        self.drive_saved_connection_start_queue(window, cx);
    }

    pub(in crate::features) fn open_connection_group_open_confirm(
        &mut self,
        group_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(group) = self
            .connection_groups
            .iter()
            .find(|group| group.id == group_id)
        else {
            return;
        };
        let connection_count = self
            .connection_state
            .saved_connections_in_group_tree(&self.connections, &self.connection_groups, &group_id)
            .len();
        if connection_count == 0 {
            return;
        }
        self.connection_state
            .open_group_open_confirm(ConnectionGroupOpenConfirmState {
                group_id,
                label: group.name.clone(),
                connection_count,
            });
        let group_open_focus = self.connection_state.group_open_focus_handle();
        window.focus(&group_open_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_connection_group_open_confirm(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.close_group_open_confirm();
        cx.notify();
    }

    pub(in crate::features) fn confirm_connection_group_open(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(confirm) = self.connection_state.take_group_open_confirm() else {
            return;
        };
        self.start_group_connections(confirm.group_id, window, cx);
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
            self.terminal.view.status = format!("{} is already queued", connection.name);
            self.shell.navigation.selected_nav = NavItem::Workspace;
            self.shell.navigation.main_mode = MainMode::Workspace;
            cx.notify();
            return false;
        }
        let name = connection.name.clone();
        self.pending_saved_connection_queue
            .push_back(PendingSavedConnectionStart {
                connection,
                options,
            });
        self.terminal.view.status = format!(
            "queued {name} ({} pending)",
            self.pending_saved_connection_queue.len()
        );
        self.shell.navigation.selected_nav = NavItem::Workspace;
        self.shell.navigation.main_mode = MainMode::Workspace;
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
        if self.pending_saved_connection_queue.is_empty() || self.has_pending_session_start() {
            return false;
        }
        let mut dirty = false;
        while !self.has_pending_session_start() {
            let Some(start) = self.pending_saved_connection_queue.pop_front() else {
                return dirty;
            };
            if self.saved_connection_start_is_pending(&start.connection) {
                dirty = true;
                continue;
            }
            let before_pending_count = self.session.start.pending.len();
            self.start_saved_connection_with_options(start.connection, start.options, window, cx);
            dirty = true;
            if self.has_pending_session_start()
                || self.session.start.pending.len() > before_pending_count
            {
                return true;
            }
        }
        dirty
    }
}

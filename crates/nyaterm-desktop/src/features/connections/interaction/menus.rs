use gpui::{Context, Window};
use nyaterm_core::SavedConnection;

use crate::features::{NyaTermApp, session::SavedConnectionStartOptions};
impl NyaTermApp {
    pub(in crate::features) fn prepare_connection_context_menu(
        &mut self,
        connection_id: String,
        cx: &mut Context<Self>,
    ) {
        self.connection_state
            .prepare_list_connection_context_menu(connection_id);
        cx.notify();
    }

    pub(in crate::features) fn copy_connection_by_id(
        &mut self,
        connection_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(connection) = self
            .connection_state
            .connections()
            .iter()
            .find(|connection| connection.id == connection_id)
            .cloned()
        else {
            self.shell
                .set_status("connection is no longer available".to_string());
            cx.notify();
            return;
        };
        self.submit_connection_copies(vec![connection], cx);
    }

    pub(in crate::features) fn start_selected_saved_connections(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let selected = self.connection_state.selected_connections();
        if selected.is_empty() {
            self.shell
                .set_status("select saved connections before connecting".to_string());
            cx.notify();
            return;
        }
        let queued = self.enqueue_saved_connection_starts(selected, cx);
        self.shell
            .set_status(format!("queued {queued} connection(s)"));
        self.drive_saved_connection_start_queue(window, cx);
    }

    pub(in crate::features) fn start_group_connections(
        &mut self,
        group_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let connections = self
            .connection_state
            .saved_connections_in_group_tree(&group_id);
        if connections.is_empty() {
            self.shell
                .set_status("group has no connections".to_string());
            cx.notify();
            return;
        }
        let queued = self.enqueue_saved_connection_starts(connections, cx);
        self.shell
            .set_status(format!("queued {queued} connection(s) from group"));
        self.drive_saved_connection_start_queue(window, cx);
    }

    pub(in crate::features) fn open_connection_group_open_confirm(
        &mut self,
        group_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(group) = self
            .connection_state
            .groups()
            .iter()
            .find(|group| group.id == group_id)
        else {
            return;
        };
        let connection_count = self
            .connection_state
            .saved_connections_in_group_tree(&group_id)
            .len();
        if connection_count == 0 {
            return;
        }
        let label = group.name.clone();
        let description = self
            .tr("savedConnections.openAllConnectionsConfirm")
            .replace("{{name}}", &label)
            .replace("{{count}}", &connection_count.to_string());
        self.open_confirm_dialog(
            (
                self.tr("savedConnections.openAllConnections").to_string(),
                description,
                self.tr("savedConnections.openAllConnections").to_string(),
                false,
                move |app, window, cx| {
                    app.start_group_connections(group_id.clone(), window, cx);
                    true
                },
            ),
            window,
            cx,
        );
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
        if self
            .session
            .start_saved_connection_is_pending_or_queued(&connection)
        {
            self.shell
                .set_status(format!("{} is already queued", connection.name));
            self.shell.show_workspace();
            cx.notify();
            return false;
        }
        let name = connection.name.clone();
        let pending_count = self
            .session
            .start_queue_saved_connection(connection, options);
        self.shell
            .set_status(format!("queued {name} ({} pending)", pending_count));
        self.shell.show_workspace();
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
        if !self.session.start_has_queued_saved_connections() || self.session.start_has_pending() {
            return false;
        }
        let mut dirty = false;
        while !self.session.start_has_pending() {
            let Some(start) = self.session.start_pop_saved_connection() else {
                return dirty;
            };
            if self
                .session
                .start_saved_connection_is_pending(&start.connection)
            {
                dirty = true;
                continue;
            }
            let before_pending_count = self.session.start_pending_count();
            self.start_saved_connection_with_options(start.connection, start.options, window, cx);
            dirty = true;
            if self.session.start_has_pending()
                || self.session.start_pending_count() > before_pending_count
            {
                return true;
            }
        }
        dirty
    }
}

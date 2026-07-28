use gpui::Context;
use nyaterm_core::{ConnectionStore, SavedConnection, uuid};

use crate::features::NyaTermApp;

impl NyaTermApp {
    /// Tauri connection selection: plain click replaces, Ctrl/Cmd toggles, Shift ranges.

    pub(in crate::features) fn select_connection(
        &mut self,
        connection_id: String,
        additive: bool,
        range: bool,
        cx: &mut Context<Self>,
    ) {
        let visible_ids = self.connection_state.visible_connection_ids(
            &self.connection_catalog.connections,
            &self.connection_catalog.groups,
        );
        let count = self.connection_state.select_list_connection(
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

    pub(in crate::features) fn clear_selected_connections(&mut self, cx: &mut Context<Self>) {
        self.connection_state.clear_list_selection();
        self.terminal.view.status = "connection selection cleared".to_string();
        cx.notify();
    }

    pub(in crate::features) fn copy_selected_connections(&mut self, cx: &mut Context<Self>) {
        let selected = self
            .connection_state
            .selected_connections(&self.connection_catalog.connections);
        if selected.is_empty() {
            self.terminal.view.status = "select saved connections before copying".to_string();
            cx.notify();
            return;
        }

        match self.copy_connections_to_store(&selected) {
            Ok(count) => {
                self.connection_state.clear_list_selection();
                self.terminal.view.status = format!("copied {count} saved connection(s)");
            }
            Err(error) => {
                self.terminal.view.status = format!("copy selected connections failed: {error}");
                self.settings.store_status.message = self.terminal.view.status.clone();
                self.settings.store_status.ready = false;
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

        self.connection_catalog.connections = store
            .load_sessions()
            .map_err(|error| error.to_string())?
            .connections;
        self.settings.store_status.message = "saved connections copied".to_string();
        self.settings.store_status.ready = true;
        Ok(connections.len())
    }
}

use gpui::Context;
use nyaterm_core::{SavedConnection, uuid};

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
        let visible_ids = self.connection_state.visible_connection_ids();
        let count = self.connection_state.select_list_connection(
            connection_id,
            &visible_ids,
            additive,
            range,
        );
        self.shell.set_status(if count == 0 {
            "connection selection cleared".to_string()
        } else {
            format!("selected {count} connection(s)")
        });
        cx.notify();
    }

    pub(in crate::features) fn clear_selected_connections(&mut self, cx: &mut Context<Self>) {
        self.connection_state.clear_list_selection();
        self.shell
            .set_status("connection selection cleared".to_string());
        cx.notify();
    }

    pub(in crate::features) fn copy_selected_connections(&mut self, cx: &mut Context<Self>) {
        let selected = self.connection_state.selected_connections();
        if selected.is_empty() {
            self.shell
                .set_status("select saved connections before copying".to_string());
            cx.notify();
            return;
        }

        match self.copy_connections_to_store(&selected) {
            Ok(count) => {
                self.connection_state.clear_list_selection();
                self.shell
                    .set_status(format!("copied {count} saved connection(s)"));
            }
            Err(error) => {
                self.shell
                    .set_status(format!("copy selected connections failed: {error}"));
                self.settings
                    .update_store_status(self.shell.status().to_string(), false);
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn copy_connections_to_store(
        &mut self,
        connections: &[SavedConnection],
    ) -> Result<usize, String> {
        let connections = connections.to_vec();
        let count = connections.len();
        let loaded = self.with_connection_store(move |store| {
            for connection in &connections {
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
                store.save_connection(&copy)?;
            }
            store.load_sessions().map(|sessions| sessions.connections)
        })?;
        self.connection_state.replace_connections(loaded);
        self.settings
            .update_store_status("saved connections copied", true);
        Ok(count)
    }
}

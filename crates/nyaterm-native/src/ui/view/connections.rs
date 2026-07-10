use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn selected_connections(&self) -> Vec<SavedConnection> {
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

    pub(in crate::ui::view) fn toggle_connection_selected(
        &mut self,
        connection_id: String,
        cx: &mut Context<Self>,
    ) {
        if self.selected_connection_ids.contains(&connection_id) {
            self.selected_connection_ids.remove(&connection_id);
            self.terminal_status = "connection deselected".to_string();
        } else {
            self.selected_connection_ids.insert(connection_id);
            self.terminal_status = "connection selected".to_string();
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn select_all_connections(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn clear_selected_connections(&mut self, cx: &mut Context<Self>) {
        self.selected_connection_ids.clear();
        self.terminal_status = "connection selection cleared".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn copy_selected_connections(&mut self, cx: &mut Context<Self>) {
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

    fn copy_connections_to_store(
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

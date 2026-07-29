use gpui::{Context, KeyDownEvent, Window};
use nyaterm_core::SessionsConfig;

use crate::features::NyaTermApp;
use crate::models::{ConnectionDeleteConfirmState, ConnectionGroupDeleteConfirmState};

impl NyaTermApp {
    pub(in crate::features) fn open_connections_clear_all_confirm(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.close_list_more_menu();
        self.connection_state.open_clear_all();
        self.terminal.view.status = "confirm clearing all saved connections".to_string();
        cx.notify();
    }

    pub(in crate::features) fn close_connections_clear_all_confirm(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.close_clear_all();
        cx.notify();
    }

    pub(in crate::features) fn confirm_connections_clear_all(&mut self, cx: &mut Context<Self>) {
        if !self.connection_state.clear_all_is_open() {
            return;
        }
        match self.with_connection_store(|store| store.replace_sessions(&SessionsConfig::default()))
        {
            Ok(()) => {
                self.connection_state.close_clear_all();
                self.connection_state.clear_list_runtime_state();
                self.refresh_store_from_runtime();
                self.terminal.view.status = self.tr("savedConnections.clearAllSuccess").to_string();
            }
            Err(error) => {
                self.connection_state.close_clear_all();
                self.terminal.view.status = format!("clear saved connections failed: {error}");
                self.settings
                    .set_store_message(self.terminal.view.status.clone());
                self.settings.set_store_ready(false);
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn open_connection_delete_confirm(
        &mut self,
        connection_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(connection) = self
            .connection_catalog
            .connections()
            .iter()
            .find(|connection| connection.id == connection_id)
        else {
            self.terminal.view.status = "connection is no longer available".to_string();
            cx.notify();
            return;
        };
        self.connection_state
            .open_delete_confirm(ConnectionDeleteConfirmState {
                connection_id,
                label: connection.name.clone(),
            });
        self.terminal.view.status = "confirm connection delete".to_string();
        cx.notify();
    }

    pub(in crate::features) fn close_connection_delete_confirm(&mut self, cx: &mut Context<Self>) {
        self.connection_state.close_delete_confirm();
        cx.notify();
    }

    pub(in crate::features) fn confirm_connection_delete(&mut self, cx: &mut Context<Self>) {
        let Some(confirm) = self.connection_state.take_delete_confirm() else {
            return;
        };
        match self.with_connection_store(|store| store.delete_connection(&confirm.connection_id)) {
            Ok(()) => {
                self.connection_state
                    .remove_list_connection_references(&confirm.connection_id);
                self.refresh_store_from_runtime();
                self.terminal.view.status = format!("deleted connection {}", confirm.label);
            }
            Err(error) => {
                self.terminal.view.status = format!("delete connection failed: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn open_connection_group_delete_confirm(
        &mut self,
        group_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(group) = self
            .connection_catalog
            .groups()
            .iter()
            .find(|group| group.id == group_id)
        else {
            self.terminal.view.status = "connection group is no longer available".to_string();
            cx.notify();
            return;
        };
        let connection_count = self
            .connection_catalog
            .connections()
            .iter()
            .filter(|connection| connection.group_id.as_deref() == Some(group_id.as_str()))
            .count();
        let child_group_count = self
            .connection_catalog
            .groups()
            .iter()
            .filter(|child| child.parent_id.as_deref() == Some(group_id.as_str()))
            .count();
        self.connection_state
            .open_group_delete_confirm(ConnectionGroupDeleteConfirmState {
                group_id,
                label: group.name.clone(),
                connection_count,
                child_group_count,
            });
        self.terminal.view.status = "confirm connection group delete".to_string();
        cx.notify();
    }

    pub(in crate::features) fn close_connection_group_delete_confirm(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.close_group_delete_confirm();
        cx.notify();
    }

    pub(in crate::features) fn confirm_connection_group_delete(&mut self, cx: &mut Context<Self>) {
        let Some(confirm) = self.connection_state.take_group_delete_confirm() else {
            return;
        };
        match self.with_connection_store(|store| store.delete_group(&confirm.group_id)) {
            Ok(()) => {
                self.connection_state
                    .remove_list_group_references(&confirm.group_id);
                self.refresh_store_from_runtime();
                self.terminal.view.status = format!("deleted connection group {}", confirm.label);
            }
            Err(error) => {
                self.terminal.view.status = format!("delete connection group failed: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn toggle_connection_group_expanded(
        &mut self,
        group_id: String,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.toggle_list_group_expanded(group_id);
        cx.notify();
    }

    pub(in crate::features) fn cycle_connection_sort_mode(&mut self, cx: &mut Context<Self>) {
        let sort_mode = self.connection_state.cycle_list_sort_mode();
        self.settings
            .set_saved_connections_sort_mode(sort_mode.persistence_id().to_string());
        self.persist_ui_layout();
        self.terminal.view.status = format!("connections sorted by {}", sort_mode.label());
        cx.notify();
    }

    /// Move the keyboard-active row through the filtered results, wrapping around.
    ///
    /// Returns whether the key was consumed, so the caller does not also feed it
    /// to the text field.
    fn step_connection_keyboard_active(&mut self, forward: bool, cx: &mut Context<Self>) -> bool {
        let visible = self.connection_state.visible_connection_ids(
            self.connection_catalog.connections(),
            self.connection_catalog.groups(),
        );
        if visible.is_empty() {
            return false;
        }
        let current = self
            .connection_state
            .list_keyboard_active_connection_id()
            .and_then(|id| visible.iter().position(|candidate| candidate == id));
        let next = match (current, forward) {
            (Some(index), true) => (index + 1) % visible.len(),
            (Some(index), false) => (index + visible.len() - 1) % visible.len(),
            (None, true) => 0,
            (None, false) => visible.len() - 1,
        };
        self.connection_state
            .set_list_keyboard_active_connection_id(Some(visible[next].clone()));
        cx.notify();
        true
    }

    /// Open the keyboard-active row, or the first result when nothing is active.
    fn open_connection_keyboard_active(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let visible = self.connection_state.visible_connection_ids(
            self.connection_catalog.connections(),
            self.connection_catalog.groups(),
        );
        let Some(target) = self
            .connection_state
            .list_keyboard_active_connection_id()
            .filter(|id| visible.iter().any(|candidate| candidate == id))
            .map(ToOwned::to_owned)
            .or_else(|| visible.first().cloned())
        else {
            return false;
        };
        let Some(connection) = self
            .connection_catalog
            .connections()
            .iter()
            .find(|connection| connection.id == target)
            .cloned()
        else {
            return false;
        };
        self.start_saved_connection(connection, window, cx);
        true
    }

    /// Drop the keyboard-active row once the filter no longer shows it.
    pub(in crate::features) fn sync_connection_keyboard_active(&mut self, cx: &mut Context<Self>) {
        let Some(active) = self
            .connection_state
            .list_keyboard_active_connection_id()
            .map(ToOwned::to_owned)
        else {
            return;
        };
        if !self
            .connection_state
            .visible_connection_ids(
                self.connection_catalog.connections(),
                self.connection_catalog.groups(),
            )
            .iter()
            .any(|candidate| candidate == &active)
        {
            self.connection_state
                .set_list_keyboard_active_connection_id(None);
            cx.notify();
        }
    }

    /// Keys the filter field deliberately leaves alone.
    ///
    /// The field consumes its own editing keys, so anything arriving here is a
    /// list gesture: walk the filtered results, open one, or clear the filter.
    pub(in crate::features) fn handle_connection_search_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.alt
            || keystroke.modifiers.function
            || keystroke.modifiers.platform
            || keystroke.modifiers.control
        {
            return;
        }

        match keystroke.key.as_str() {
            "escape" => {
                cx.stop_propagation();
                self.clear_connection_search(window, cx);
            }
            "up" | "down" if !self.connection_state.list_search_is_empty() => {
                if self.step_connection_keyboard_active(keystroke.key == "down", cx) {
                    cx.stop_propagation();
                }
            }
            "enter" if !self.connection_state.list_search_is_empty() => {
                if self.open_connection_keyboard_active(window, cx) {
                    cx.stop_propagation();
                }
            }
            _ => {}
        }
    }

    pub(in crate::features) fn clear_connection_search(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let field = self.connection_state.list_search_field();
        field.update(cx, |field, cx| field.set_content(String::new(), cx));
        self.connection_state.set_list_search_text(String::new());
        window.focus(&field.read(cx).focus_handle());
        self.terminal.view.status = "connection search cleared".to_string();
        self.sync_connection_keyboard_active(cx);
        cx.notify();
    }

    pub(in crate::features) fn delete_selected_connections(&mut self, cx: &mut Context<Self>) {
        let selected = self
            .connection_state
            .selected_connections(self.connection_catalog.connections());
        if selected.is_empty() {
            self.terminal.view.status = "select saved connections before deleting".to_string();
            cx.notify();
            return;
        }
        if selected.len() == 1 {
            self.open_connection_delete_confirm(selected[0].id.clone(), cx);
            return;
        }
        match self.with_connection_store(|store| {
            for connection in &selected {
                store.delete_connection(&connection.id)?;
            }
            Ok(())
        }) {
            Ok(()) => {
                for connection in &selected {
                    self.connection_state
                        .remove_list_connection_references(&connection.id);
                }
                self.refresh_store_from_runtime();
                self.terminal.view.status = format!("deleted {} connection(s)", selected.len());
            }
            Err(error) => {
                self.terminal.view.status = format!("delete selected connections failed: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn rename_connection(
        &mut self,
        connection_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_connection_editor(Some(connection_id), None, false, window, cx);
    }
}

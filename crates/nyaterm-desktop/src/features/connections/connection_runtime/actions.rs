use gpui::{Context, KeyDownEvent, Window};
use nyaterm_core::SessionsConfig;

use crate::features::NyaTermApp;
use crate::models::{ConnectionDeleteConfirmState, ConnectionGroupDeleteConfirmState};

impl NyaTermApp {
    pub(in crate::features) fn open_connections_clear_all_confirm(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.list.close_more_menu();
        self.connection_state.confirmations.open_clear_all();
        self.terminal.view.status = "confirm clearing all saved connections".to_string();
        cx.notify();
    }

    pub(in crate::features) fn close_connections_clear_all_confirm(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.confirmations.close_clear_all();
        cx.notify();
    }

    pub(in crate::features) fn confirm_connections_clear_all(&mut self, cx: &mut Context<Self>) {
        if !self.connection_state.confirmations.clear_all_is_open() {
            return;
        }
        match self.with_connection_store(|store| store.replace_sessions(&SessionsConfig::default()))
        {
            Ok(()) => {
                self.connection_state.confirmations.close_clear_all();
                self.connection_state.list.clear_runtime_state();
                self.refresh_store_from_runtime();
                self.terminal.view.status = self.tr("savedConnections.clearAllSuccess").to_string();
            }
            Err(error) => {
                self.connection_state.confirmations.close_clear_all();
                self.terminal.view.status = format!("clear saved connections failed: {error}");
                self.store_status.message = self.terminal.view.status.clone();
                self.store_status.ready = false;
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
            .connections
            .iter()
            .find(|connection| connection.id == connection_id)
        else {
            self.terminal.view.status = "connection is no longer available".to_string();
            cx.notify();
            return;
        };
        self.connection_state
            .confirmations
            .open_delete(ConnectionDeleteConfirmState {
                connection_id,
                label: connection.name.clone(),
            });
        self.terminal.view.status = "confirm connection delete".to_string();
        cx.notify();
    }

    pub(in crate::features) fn close_connection_delete_confirm(&mut self, cx: &mut Context<Self>) {
        self.connection_state.confirmations.close_delete();
        cx.notify();
    }

    pub(in crate::features) fn confirm_connection_delete(&mut self, cx: &mut Context<Self>) {
        let Some(confirm) = self.connection_state.confirmations.take_delete() else {
            return;
        };
        match self.with_connection_store(|store| store.delete_connection(&confirm.connection_id)) {
            Ok(()) => {
                self.connection_state
                    .list
                    .remove_connection_references(&confirm.connection_id);
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
            .connection_groups
            .iter()
            .find(|group| group.id == group_id)
        else {
            self.terminal.view.status = "connection group is no longer available".to_string();
            cx.notify();
            return;
        };
        let connection_count = self
            .connections
            .iter()
            .filter(|connection| connection.group_id.as_deref() == Some(group_id.as_str()))
            .count();
        let child_group_count = self
            .connection_groups
            .iter()
            .filter(|child| child.parent_id.as_deref() == Some(group_id.as_str()))
            .count();
        self.connection_state
            .confirmations
            .open_group_delete(ConnectionGroupDeleteConfirmState {
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
        self.connection_state.confirmations.close_group_delete();
        cx.notify();
    }

    pub(in crate::features) fn confirm_connection_group_delete(&mut self, cx: &mut Context<Self>) {
        let Some(confirm) = self.connection_state.confirmations.take_group_delete() else {
            return;
        };
        match self.with_connection_store(|store| store.delete_group(&confirm.group_id)) {
            Ok(()) => {
                self.connection_state
                    .list
                    .remove_group_references(&confirm.group_id);
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
        self.connection_state.list.toggle_group_expanded(group_id);
        cx.notify();
    }

    pub(in crate::features) fn cycle_connection_sort_mode(&mut self, cx: &mut Context<Self>) {
        let sort_mode = self.connection_state.list.cycle_sort_mode();
        self.settings.ui_saved_connections_sort_mode = sort_mode.persistence_id().to_string();
        self.persist_ui_layout();
        self.terminal.view.status = format!("connections sorted by {}", sort_mode.label());
        cx.notify();
    }

    /// Move the keyboard-active row through the filtered results, wrapping around.
    ///
    /// Returns whether the key was consumed, so the caller does not also feed it
    /// to the text field.
    fn step_connection_keyboard_active(&mut self, forward: bool, cx: &mut Context<Self>) -> bool {
        let visible = self.visible_connection_ids();
        if visible.is_empty() {
            return false;
        }
        let current = self
            .connection_state
            .list
            .keyboard_active_connection_id()
            .and_then(|id| visible.iter().position(|candidate| candidate == id));
        let next = match (current, forward) {
            (Some(index), true) => (index + 1) % visible.len(),
            (Some(index), false) => (index + visible.len() - 1) % visible.len(),
            (None, true) => 0,
            (None, false) => visible.len() - 1,
        };
        self.connection_state
            .list
            .set_keyboard_active_connection_id(Some(visible[next].clone()));
        cx.notify();
        true
    }

    /// Open the keyboard-active row, or the first result when nothing is active.
    fn open_connection_keyboard_active(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let visible = self.visible_connection_ids();
        let Some(target) = self
            .connection_state
            .list
            .keyboard_active_connection_id()
            .filter(|id| visible.iter().any(|candidate| candidate == id))
            .map(ToOwned::to_owned)
            .or_else(|| visible.first().cloned())
        else {
            return false;
        };
        let Some(connection) = self
            .connections
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
    fn sync_connection_keyboard_active(&mut self, cx: &mut Context<Self>) {
        let Some(active) = self
            .connection_state
            .list
            .keyboard_active_connection_id()
            .map(ToOwned::to_owned)
        else {
            return;
        };
        if !self
            .visible_connection_ids()
            .iter()
            .any(|candidate| candidate == &active)
        {
            self.connection_state
                .list
                .set_keyboard_active_connection_id(None);
            cx.notify();
        }
    }

    pub(in crate::features) fn handle_connection_search_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.alt || keystroke.modifiers.function {
            return;
        }
        let key = keystroke.key.as_str();

        // While a filter is active the box also drives the results: up/down walk
        // them and enter opens the active one, as the old UI did.
        if !self.connection_state.list.search_is_empty()
            && matches!(key, "up" | "down" | "enter")
            && !keystroke.modifiers.platform
            && !keystroke.modifiers.control
        {
            match key {
                "up" | "down" => {
                    if self.step_connection_keyboard_active(key == "down", cx) {
                        return;
                    }
                }
                _ => {
                    if self.open_connection_keyboard_active(window, cx) {
                        return;
                    }
                }
            }
        }

        let changed = if key == "escape" {
            self.connection_state.list.clear_search()
        } else if !keystroke.modifiers.platform && !keystroke.modifiers.control {
            self.connection_state.list.apply_search_key(
                key,
                keystroke.key_char.as_deref(),
                keystroke.modifiers.shift,
            )
        } else {
            false
        };
        if changed {
            if key == "escape" {
                self.terminal.view.status = "connection search cleared".to_string();
            }
            self.sync_connection_keyboard_active(cx);
            cx.notify();
        }
    }

    pub(in crate::features) fn delete_selected_connections(&mut self, cx: &mut Context<Self>) {
        let selected = self.selected_connections();
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
                        .list
                        .remove_connection_references(&connection.id);
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

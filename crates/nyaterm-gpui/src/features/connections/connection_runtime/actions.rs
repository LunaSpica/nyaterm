use super::*;

impl NyaTermApp {
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
            self.terminal_status = "connection is no longer available".to_string();
            cx.notify();
            return;
        };
        self.connection_delete_confirm = Some(ConnectionDeleteConfirmState {
            connection_id,
            label: connection.name.clone(),
        });
        self.terminal_status = "confirm connection delete".to_string();
        cx.notify();
    }

    pub(in crate::features) fn close_connection_delete_confirm(&mut self, cx: &mut Context<Self>) {
        self.connection_delete_confirm = None;
        cx.notify();
    }

    pub(in crate::features) fn confirm_connection_delete(&mut self, cx: &mut Context<Self>) {
        let Some(confirm) = self.connection_delete_confirm.clone() else {
            return;
        };
        match self.with_connection_store(|store| store.delete_connection(&confirm.connection_id)) {
            Ok(()) => {
                self.selected_connection_ids.remove(&confirm.connection_id);
                self.connection_delete_confirm = None;
                self.refresh_store_from_runtime();
                self.terminal_status = format!("deleted connection {}", confirm.label);
            }
            Err(error) => {
                self.terminal_status = format!("delete connection failed: {error}");
                self.connection_delete_confirm = None;
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
            self.terminal_status = "connection group is no longer available".to_string();
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
        self.connection_group_delete_confirm = Some(ConnectionGroupDeleteConfirmState {
            group_id,
            label: group.name.clone(),
            connection_count,
            child_group_count,
        });
        self.terminal_status = "confirm connection group delete".to_string();
        cx.notify();
    }

    pub(in crate::features) fn close_connection_group_delete_confirm(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.connection_group_delete_confirm = None;
        cx.notify();
    }

    pub(in crate::features) fn confirm_connection_group_delete(&mut self, cx: &mut Context<Self>) {
        let Some(confirm) = self.connection_group_delete_confirm.clone() else {
            return;
        };
        if confirm.connection_count > 0 || confirm.child_group_count > 0 {
            self.terminal_status =
                "move or delete child groups/connections before deleting this folder".to_string();
            self.connection_group_delete_confirm = None;
            cx.notify();
            return;
        }
        match self.with_connection_store(|store| store.delete_group(&confirm.group_id)) {
            Ok(()) => {
                self.expanded_connection_groups.remove(&confirm.group_id);
                self.connection_group_delete_confirm = None;
                self.refresh_store_from_runtime();
                self.terminal_status = format!("deleted connection group {}", confirm.label);
            }
            Err(error) => {
                self.terminal_status = format!("delete connection group failed: {error}");
                self.connection_group_delete_confirm = None;
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn toggle_connection_group_expanded(
        &mut self,
        group_id: String,
        cx: &mut Context<Self>,
    ) {
        if self.expanded_connection_groups.contains(&group_id) {
            self.expanded_connection_groups.remove(&group_id);
        } else {
            self.expanded_connection_groups.insert(group_id);
        }
        self.connection_list_offset = 0;
        cx.notify();
    }

    pub(in crate::features) fn cycle_connection_sort_mode(&mut self, cx: &mut Context<Self>) {
        self.connection_sort_mode = self.connection_sort_mode.next();
        self.connection_list_offset = 0;
        self.terminal_status = format!(
            "connections sorted by {}",
            self.connection_sort_mode.label()
        );
        cx.notify();
    }

    pub(in crate::features) fn handle_connection_search_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.alt || keystroke.modifiers.function {
            return;
        }
        match keystroke.key.as_str() {
            "escape" => {
                self.connection_search_draft.clear();
                self.connection_list_offset = 0;
                self.terminal_status = "connection search cleared".to_string();
                cx.notify();
            }
            "backspace" if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                self.connection_search_draft.pop();
                self.connection_list_offset = 0;
                cx.notify();
            }
            _ if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.connection_search_draft.push_str(input);
                    self.connection_list_offset = 0;
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    pub(in crate::features) fn delete_selected_connections(&mut self, cx: &mut Context<Self>) {
        let selected = self.selected_connections();
        if selected.is_empty() {
            self.terminal_status = "select saved connections before deleting".to_string();
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
                self.selected_connection_ids.clear();
                self.refresh_store_from_runtime();
                self.terminal_status = format!("deleted {} connection(s)", selected.len());
            }
            Err(error) => {
                self.terminal_status = format!("delete selected connections failed: {error}");
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

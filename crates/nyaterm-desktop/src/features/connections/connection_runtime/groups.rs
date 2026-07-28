use gpui::{Context, KeyDownEvent, Window};
use nyaterm_core::{Group, uuid};

use crate::features::NyaTermApp;
use crate::models::ConnectionGroupEditorState;

impl NyaTermApp {
    pub(in crate::features) fn open_connection_group_editor(
        &mut self,
        group_id: Option<String>,
        parent_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = match group_id.as_deref() {
            Some(id) => self
                .connection_catalog
                .groups()
                .iter()
                .find(|group| group.id == id)
                .map(|group| group.name.clone()),
            None => Some(String::new()),
        };
        let Some(name) = name else {
            self.terminal.view.status = "connection group is no longer available".to_string();
            cx.notify();
            return;
        };
        let parent_id = group_id
            .as_deref()
            .and_then(|id| {
                self.connection_catalog
                    .groups()
                    .iter()
                    .find(|group| group.id == id)
                    .and_then(|group| group.parent_id.clone())
            })
            .or(parent_id);

        self.connection_state
            .begin_group_editor(ConnectionGroupEditorState {
                id: group_id,
                name,
                parent_id,
                error: None,
            });
        self.connection_state.build_group_editor_field(cx);
        self.terminal.view.status = "connection group editor opened".to_string();
        if let Some(field) = self.connection_state.group_editor_field() {
            window.focus(&field.read(cx).focus_handle());
        }
        cx.notify();
    }

    pub(in crate::features) fn close_connection_group_editor(&mut self, cx: &mut Context<Self>) {
        self.connection_state.close_group_editor();
        self.connection_state.clear_group_editor_field();
        self.terminal.view.status = "connection group editor closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn handle_connection_group_editor_key_down(
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
            "escape" => self.close_connection_group_editor(cx),
            "enter" => self.save_connection_group_editor(cx),
            "backspace" if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                if self
                    .connection_state
                    .apply_group_editor_name_key("backspace", None)
                {
                    cx.notify();
                }
            }
            _ if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    if self
                        .connection_state
                        .apply_group_editor_name_key(keystroke.key.as_str(), Some(input))
                    {
                        cx.notify();
                    }
                }
            }
            _ => {}
        }
    }

    pub(in crate::features) fn save_connection_group_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.connection_state.active_group_editor_draft() else {
            return;
        };
        let name = editor.name.trim().to_string();
        if name.is_empty() {
            let message = self.tr("savedConnections.folderNameRequired").to_string();
            self.connection_state.set_group_editor_error(message);
            cx.notify();
            return;
        }

        let group = Group {
            id: editor.id.clone().unwrap_or_else(uuid),
            name,
            parent_id: editor.parent_id.clone(),
            sort_order: editor
                .id
                .as_deref()
                .and_then(|id| {
                    self.connection_catalog
                        .groups()
                        .iter()
                        .find(|group| group.id == id)
                        .map(|group| group.sort_order)
                })
                .unwrap_or(self.connection_catalog.groups().len() as i32),
            created_at_ms: None,
            updated_at_ms: None,
        };

        match self.with_connection_store(|store| store.save_group(&group)) {
            Ok(()) => {
                self.connection_state.expand_list_group(group.id.clone());
                self.connection_state.close_group_editor();
                self.refresh_store_from_runtime();
                self.terminal.view.status = format!("saved connection group {}", group.name);
            }
            Err(error) => {
                self.connection_state.set_group_editor_error(error);
            }
        }
        cx.notify();
    }
}

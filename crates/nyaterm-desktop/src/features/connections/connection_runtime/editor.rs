use gpui::{Context, KeyDownEvent, PathPromptOptions, SharedString, Window};
use nyaterm_core::{Group, uuid};

use super::helpers::{
    ConnectionEditorToggle, build_saved_connection_from_editor, connection_editor_from_saved,
};
use crate::features::NyaTermApp;
use crate::models::{
    ConnectionEditorAdvancedTab, ConnectionEditorField, ConnectionEditorMenu,
    ConnectionEditorPasswordSource, ConnectionEditorState, ConnectionEditorTelnetTab,
    ConnectionKindTab,
};

impl NyaTermApp {
    pub(in crate::features) fn connection_editor_validation_error(
        &self,
        editor: &ConnectionEditorState,
    ) -> Option<String> {
        build_saved_connection_from_editor(editor).err()
    }

    pub(in crate::features) fn open_connection_editor(
        &mut self,
        connection_id: Option<String>,
        parent_group_id: Option<String>,
        connect_after_save: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.refresh_connection_auth_catalog();
        let editor = if let Some(connection_id) = connection_id {
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
            connection_editor_from_saved(connection, connect_after_save)
        } else {
            ConnectionEditorState {
                id: None,
                kind: ConnectionKindTab::Ssh,
                name: String::new(),
                description: String::new(),
                icon: None,
                // A new connection has no icon yet, so let the first successful
                // SSH session fill one in.
                icon_auto_detect: true,
                group_id: parent_group_id.filter(|value| !value.trim().is_empty()),
                new_group_name: String::new(),
                pending_group_name: None,
                pending_group_parent_id: None,
                host: String::new(),
                port: "22".to_string(),
                username: "root".to_string(),
                auth_mode: "password".to_string(),
                password_source: ConnectionEditorPasswordSource::Ask,
                password_id: None,
                password: String::new(),
                existing_password: None,
                key_id: None,
                otp_id: None,
                auto_fill_otp: false,
                proxy_id: None,
                proxy_jump_id: None,
                x11_forwarding: false,
                backspace_mode: "del".to_string(),
                shell_path: String::new(),
                shell_args: String::new(),
                working_dir: String::new(),
                serial_port: self
                    .connection_serial_ports
                    .first()
                    .cloned()
                    .unwrap_or_default(),
                baud_rate: "115200".to_string(),
                data_bits: "8".to_string(),
                parity: "none".to_string(),
                stop_bits: "1".to_string(),
                raw_tcp_cli: false,
                telnet_enter_mode: "cr".to_string(),
                local_echo: false,
                local_line_edit: false,
                force_character_at_a_time: false,
                send_naws: true,
                send_sga: true,
                post_login_enabled: false,
                post_login_command: String::new(),
                post_login_delay_ms: "1000".to_string(),
                advanced_open: false,
                advanced_network_tab: ConnectionEditorAdvancedTab::Proxy,
                advanced_behavior_tab: ConnectionEditorAdvancedTab::PostLogin,
                telnet_advanced_tab: ConnectionEditorTelnetTab::Input,
                connect_after_save,
                focused_field: ConnectionEditorField::Name,
                error: None,
            }
        };

        self.connection_state.editor.begin_edit(editor);
        self.terminal.view.status = "connection editor opened".to_string();
        if !self.open_connection_editor_window(cx) {
            let editor_focus = self.connection_state.editor.focus_handle();
            window.focus(&editor_focus);
        }
        cx.notify();
    }

    pub(in crate::features) fn close_connection_editor(&mut self, cx: &mut Context<Self>) {
        self.connection_state.editor.close();
        self.terminal.view.status = "connection editor closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn toggle_connection_icon_picker(&mut self, cx: &mut Context<Self>) {
        self.connection_state.editor.toggle_icon_picker();
        cx.notify();
    }

    pub(in crate::features) fn set_connection_editor_icon(
        &mut self,
        icon: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.editor.set_icon(icon);
        cx.notify();
    }

    pub(in crate::features) fn set_connection_editor_icon_auto_detect(
        &mut self,
        enabled: bool,
        cx: &mut Context<Self>,
    ) {
        if self.connection_state.editor.set_icon_auto_detect(enabled) {
            cx.notify();
        }
    }

    pub(in crate::features) fn toggle_connection_editor_menu(
        &mut self,
        menu: ConnectionEditorMenu,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.editor.toggle_menu(menu);
        cx.notify();
    }

    pub(in crate::features) fn set_connection_editor_menu_value(
        &mut self,
        menu: ConnectionEditorMenu,
        value: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        self.connection_state
            .editor
            .set_menu_value(menu, value.map(ToOwned::to_owned));
        cx.notify();
    }

    pub(in crate::features) fn set_connection_editor_password_source(
        &mut self,
        source: ConnectionEditorPasswordSource,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.editor.set_password_source(source);
        cx.notify();
    }

    pub(in crate::features) fn set_connection_editor_advanced_tab(
        &mut self,
        tab: ConnectionEditorAdvancedTab,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.editor.set_advanced_tab(tab);
        cx.notify();
    }

    pub(in crate::features) fn set_connection_editor_telnet_tab(
        &mut self,
        tab: ConnectionEditorTelnetTab,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.editor.set_telnet_tab(tab);
        cx.notify();
    }

    pub(in crate::features) fn focus_connection_editor_field(
        &mut self,
        field: ConnectionEditorField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.editor.focus_field(field);
        let editor_focus = self.connection_state.editor.focus_handle();
        window.focus(&editor_focus);
        cx.notify();
    }

    pub(in crate::features) fn focus_connection_editor_new_group(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.editor.focus_new_group_field();
        let editor_focus = self.connection_state.editor.focus_handle();
        window.focus(&editor_focus);
        cx.notify();
    }

    pub(in crate::features) fn commit_connection_editor_new_group(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let required_message = self.tr("dialog.groupNameRequired").to_string();
        if self
            .connection_state
            .editor
            .commit_new_group(required_message)
        {
            cx.notify();
        }
    }

    pub(in crate::features) fn set_connection_editor_kind(
        &mut self,
        kind: ConnectionKindTab,
        cx: &mut Context<Self>,
    ) {
        if self.connection_state.editor.set_kind(kind) {
            self.terminal.view.status = format!("connection type set to {}", kind.label());
        }
        cx.notify();
    }

    pub(in crate::features) fn prompt_connection_editor_shell_path(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let options = PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from("Select shell executable")),
        };
        let receiver = cx.prompt_for_paths(options);
        self.terminal.view.status = "selecting shell executable".to_string();
        cx.spawn(async move |this, cx| {
            let selected = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let _ = this.update(cx, |this, cx| {
                if let Some(path) = selected {
                    let path = path.display().to_string();
                    this.connection_state.editor.apply_shell_path(path.clone());
                    this.terminal.view.status = format!("shell path: {path}");
                } else {
                    this.terminal.view.status = "shell path selection cancelled".to_string();
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::features) fn prompt_connection_editor_working_dir(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let options = PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some(SharedString::from("Select working directory")),
        };
        let receiver = cx.prompt_for_paths(options);
        self.terminal.view.status = "selecting working directory".to_string();
        cx.spawn(async move |this, cx| {
            let selected = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let _ = this.update(cx, |this, cx| {
                if let Some(path) = selected {
                    let path = path.display().to_string();
                    this.connection_state.editor.apply_working_dir(path.clone());
                    this.terminal.view.status = format!("working dir: {path}");
                } else {
                    this.terminal.view.status = "working directory selection cancelled".to_string();
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::features) fn toggle_connection_editor_flag(
        &mut self,
        flag: ConnectionEditorToggle,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.editor.toggle_flag(flag);
        cx.notify();
    }

    pub(in crate::features) fn handle_connection_editor_key_down(
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

        match keystroke.key.as_str() {
            "escape" => {
                if self.connection_state.editor.icon_picker_is_open() {
                    self.connection_state.editor.close_popovers();
                    cx.notify();
                    return;
                }
                if self.connection_state.editor.menu_is_open() {
                    self.connection_state
                        .editor
                        .close_popovers_and_cancel_group_draft();
                    cx.notify();
                    return;
                }
                self.close_connection_editor(cx);
                return;
            }
            "enter" => {
                if !keystroke.modifiers.platform
                    && !keystroke.modifiers.control
                    && self.connection_state.editor.description_is_focused()
                {
                    self.connection_state.editor.insert_description_newline();
                    cx.notify();
                    return;
                }
                if self
                    .connection_state
                    .editor
                    .new_group_name_focused_in_group_menu()
                {
                    self.commit_connection_editor_new_group(cx);
                    return;
                }
                self.save_connection_editor(window, cx);
                return;
            }
            "tab" if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                self.connection_state.editor.advance_focus();
                cx.notify();
                return;
            }
            _ => {}
        }

        if keystroke.modifiers.platform || keystroke.modifiers.control {
            return;
        }

        if self
            .connection_state
            .editor
            .apply_text_key(keystroke.key.as_str(), keystroke.key_char.as_deref())
        {
            cx.notify();
        }
    }

    pub(in crate::features) fn save_connection_editor(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(mut editor) = self.connection_state.editor.active_draft() else {
            return;
        };

        let pending_group = editor.pending_group_name.as_ref().map(|name| Group {
            id: uuid(),
            name: name.clone(),
            parent_id: editor.pending_group_parent_id.clone(),
            sort_order: self.connection_groups.len() as i32,
            created_at_ms: None,
            updated_at_ms: None,
        });
        if let Some(group) = pending_group.as_ref() {
            editor.group_id = Some(group.id.clone());
        }

        let built = match build_saved_connection_from_editor(&editor) {
            Ok(connection) => connection,
            Err(error) => {
                self.set_connection_editor_error(error, cx);
                return;
            }
        };

        match self.persist_saved_connection_with_group(built.clone(), pending_group.as_ref()) {
            Ok(saved) => {
                let connect_after_save = editor.connect_after_save;
                self.connection_state
                    .finish_editor_save(saved.id.clone(), saved.group_id.clone());
                self.terminal.view.status = format!("saved connection {}", saved.name);
                if connect_after_save {
                    self.start_saved_connection(saved, window, cx);
                } else {
                    cx.notify();
                }
            }
            Err(error) => self.set_connection_editor_error(error, cx),
        }
    }
}

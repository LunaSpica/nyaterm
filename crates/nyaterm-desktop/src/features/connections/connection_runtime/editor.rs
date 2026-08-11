use gpui::{Context, KeyDownEvent, PathPromptOptions, SharedString, Window};
use nyaterm_core::{
    Group, RdpClipboardSettings, RdpDisplaySettings, RdpReconnectSettings, RdpSecuritySettings,
    uuid,
};

use super::helpers::{
    ConnectionEditorToggle, build_saved_connection_from_editor, connection_editor_from_saved,
};
use crate::features::NyaTermApp;
use crate::models::{
    ConnectionEditorAdvancedTab, ConnectionEditorField, ConnectionEditorPasswordSource,
    ConnectionEditorSelect, ConnectionEditorState, ConnectionEditorTelnetTab, ConnectionKindTab,
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
                domain: String::new(),
                auth_mode: "password".to_string(),
                rdp_security: RdpSecuritySettings::default(),
                rdp_display: RdpDisplaySettings::default(),
                rdp_clipboard: RdpClipboardSettings::default(),
                rdp_reconnect: RdpReconnectSettings::default(),
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
                encoding: "global".to_string(),
                sftp_enabled: true,
                sftp_cwd_follow_mode: "shell_integration".to_string(),
                sftp_shell_detection_timeout_ms: "3000".to_string(),
                sftp_filename_encoding: "terminal".to_string(),
                ssh_algorithm_mode: "compatible".to_string(),
                ssh_algorithm_kex: Vec::new(),
                ssh_algorithm_ciphers: Vec::new(),
                ssh_algorithm_macs: Vec::new(),
                ssh_algorithm_host_keys: Vec::new(),
                shell_path: String::new(),
                shell_args: String::new(),
                working_dir: String::new(),
                serial_port: self
                    .connection_state
                    .serial_ports()
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
                telnet_auto_login_enabled: true,
                telnet_auto_login_send_wake_enter: true,
                telnet_auto_login_timeout_ms: "60000".to_string(),
                telnet_auto_login_username_prompt_regex: String::new(),
                telnet_auto_login_password_prompt_regex: String::new(),
                telnet_auto_login_success_prompt_regex: String::new(),
                telnet_auto_login_failure_prompt_regex: String::new(),
                telnet_auto_login_max_retries: "0".to_string(),
                post_login_enabled: false,
                post_login_command: String::new(),
                post_login_delay_ms: "1000".to_string(),
                recording: None,
                advanced_open: false,
                advanced_network_tab: ConnectionEditorAdvancedTab::Proxy,
                advanced_behavior_tab: ConnectionEditorAdvancedTab::PostLogin,
                telnet_advanced_tab: ConnectionEditorTelnetTab::Input,
                connect_after_save,
                focused_field: ConnectionEditorField::Name,
                error: None,
            }
        };

        self.connection_state.begin_editor(editor);
        // Fields mirror the draft, so they are rebuilt with it.
        self.connection_state.build_editor_fields(cx);
        self.shell
            .set_status("connection editor opened".to_string());
        if !self.open_connection_editor_window(cx) {
            // Land on the name and select it, so an edit can start by typing.
            match self
                .connection_state
                .editor_fields()
                .get(&ConnectionEditorField::Name)
                .cloned()
            {
                Some(field) => {
                    window.focus(&field.read(cx).focus_handle(), cx);
                    field.update(cx, |field, cx| field.select_all(window, cx));
                }
                None => {
                    let editor_focus = self.connection_state.editor_focus_handle();
                    window.focus(&editor_focus, cx);
                }
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn close_connection_editor(&mut self, cx: &mut Context<Self>) {
        self.connection_state.close_editor();
        self.connection_state.clear_editor_fields();
        self.shell
            .set_status("connection editor closed".to_string());
        cx.notify();
    }

    pub(in crate::features) fn set_connection_icon_picker_open(
        &mut self,
        open: bool,
        cx: &mut Context<Self>,
    ) {
        if self.connection_state.set_editor_icon_picker_open(open) {
            cx.notify();
        }
    }

    pub(in crate::features) fn toggle_connection_group_select(&mut self, cx: &mut Context<Self>) {
        self.connection_state.toggle_editor_group_select();
        cx.notify();
    }

    pub(in crate::features) fn close_connection_group_select(&mut self, cx: &mut Context<Self>) {
        self.connection_state.close_editor_group_select();
        cx.notify();
    }

    pub(in crate::features) fn set_connection_group_select_trigger_bounds(
        &mut self,
        bounds: gpui::Bounds<gpui::Pixels>,
        cx: &mut Context<Self>,
    ) {
        if self
            .connection_state
            .set_editor_group_select_trigger_bounds(bounds)
        {
            cx.notify();
        }
    }

    pub(in crate::features) fn set_connection_editor_icon(
        &mut self,
        icon: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.set_editor_icon(icon);
        cx.notify();
    }

    pub(in crate::features) fn set_connection_editor_icon_auto_detect(
        &mut self,
        enabled: bool,
        cx: &mut Context<Self>,
    ) {
        if self.connection_state.set_editor_icon_auto_detect(enabled) {
            cx.notify();
        }
    }

    pub(in crate::features) fn set_connection_editor_select_value(
        &mut self,
        select: ConnectionEditorSelect,
        value: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        self.connection_state
            .set_editor_select_value(select, value.map(ToOwned::to_owned));
        cx.notify();
    }

    pub(in crate::features) fn set_connection_editor_password_source(
        &mut self,
        source: ConnectionEditorPasswordSource,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.set_editor_password_source(source);
        cx.notify();
    }

    pub(in crate::features) fn set_connection_editor_advanced_tab(
        &mut self,
        tab: ConnectionEditorAdvancedTab,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.set_editor_advanced_tab(tab);
        cx.notify();
    }

    pub(in crate::features) fn set_connection_editor_telnet_tab(
        &mut self,
        tab: ConnectionEditorTelnetTab,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.set_editor_telnet_tab(tab);
        cx.notify();
    }

    /// Take an edit from a field widget into the draft.
    pub(in crate::features) fn apply_connection_editor_field_text(
        &mut self,
        field: ConnectionEditorField,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.set_editor_field_text(field, text);
        cx.notify();
    }

    pub(in crate::features) fn commit_connection_editor_new_group(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let required_message = self.tr("dialog.groupNameRequired").to_string();
        if self
            .connection_state
            .commit_editor_new_group(required_message)
        {
            // The draft's copy is cleared by the commit; the box the name was
            // typed into holds its own buffer and has to be told.
            self.connection_state
                .reset_editor_field(ConnectionEditorField::NewGroupName, "", cx);
            cx.notify();
        }
    }

    pub(in crate::features) fn set_connection_editor_kind(
        &mut self,
        kind: ConnectionKindTab,
        cx: &mut Context<Self>,
    ) {
        if self.connection_state.set_editor_kind(kind) {
            // Switching kind rewrites the default port on the draft; the box has
            // to be told, or it keeps showing the other protocol's.
            self.connection_state.sync_editor_fields_from_draft(cx);
            self.shell
                .set_status(format!("connection type set to {}", kind.label()));
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
        self.shell
            .set_status("selecting shell executable".to_string());
        cx.spawn(async move |this, cx| {
            let selected = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let _ = this.update(cx, |this, cx| {
                if let Some(path) = selected {
                    let path = path.display().to_string();
                    this.connection_state.apply_editor_shell_path(path.clone());
                    this.shell.set_status(format!("shell path: {path}"));
                } else {
                    this.shell
                        .set_status("shell path selection cancelled".to_string());
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
        self.shell
            .set_status("selecting working directory".to_string());
        cx.spawn(async move |this, cx| {
            let selected = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let _ = this.update(cx, |this, cx| {
                if let Some(path) = selected {
                    let path = path.display().to_string();
                    this.connection_state.apply_editor_working_dir(path.clone());
                    this.shell.set_status(format!("working dir: {path}"));
                } else {
                    this.shell
                        .set_status("working directory selection cancelled".to_string());
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
        self.connection_state.toggle_editor_flag(flag);
        cx.notify();
    }

    pub(in crate::features) fn handle_connection_editor_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.alt || keystroke.modifiers.function {
            return false;
        }

        match keystroke.key.as_str() {
            "escape" => {
                if self.connection_state.editor_icon_picker_is_open() {
                    self.connection_state.close_editor_icon_picker();
                    cx.notify();
                    return true;
                }
                if self.connection_state.editor_group_select_is_open() {
                    self.connection_state.close_editor_group_select();
                    cx.notify();
                    return true;
                }
                self.close_connection_editor(cx);
                return true;
            }
            "enter" => {
                if !keystroke.modifiers.platform
                    && !keystroke.modifiers.control
                    && self.connection_state.editor_description_is_focused()
                {
                    self.connection_state.insert_editor_description_newline();
                    cx.notify();
                    return true;
                }
                if self.connection_state.editor_new_group_field_is_focused(cx) {
                    self.commit_connection_editor_new_group(cx);
                    return true;
                }
                self.save_connection_editor(window, cx);
                return true;
            }
            "tab" if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                self.connection_state.advance_editor_focus();
                cx.notify();
                return true;
            }
            _ => {}
        }

        false
    }

    pub(in crate::features) fn save_connection_editor(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(mut editor) = self.connection_state.active_editor_draft() else {
            return;
        };

        let pending_group = editor.pending_group_name.as_ref().map(|name| Group {
            id: uuid(),
            name: name.clone(),
            parent_id: editor.pending_group_parent_id.clone(),
            sort_order: self.connection_state.groups().len() as i32,
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
                self.shell
                    .set_status(format!("saved connection {}", saved.name));
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

use super::*;

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
                self.terminal_status = "connection is no longer available".to_string();
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

        self.connection_icon_picker_open = false;
        self.connection_editor_menu = None;
        self.connection_editor = Some(editor);
        self.terminal_status = "connection editor opened".to_string();
        if !self.open_connection_editor_window(cx) {
            window.focus(&self.connection_editor_focus);
        }
        cx.notify();
    }

    pub(in crate::features) fn close_connection_editor(&mut self, cx: &mut Context<Self>) {
        self.connection_icon_picker_open = false;
        self.connection_editor_menu = None;
        self.connection_editor = None;
        self.connection_editor_window = None;
        self.terminal_status = "connection editor closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn toggle_connection_icon_picker(&mut self, cx: &mut Context<Self>) {
        self.connection_editor_menu = None;
        self.connection_icon_picker_open = !self.connection_icon_picker_open;
        cx.notify();
    }

    pub(in crate::features) fn set_connection_editor_icon(
        &mut self,
        icon: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        self.connection_icon_picker_open = false;
        self.connection_editor_menu = None;
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.icon = icon
                .map(str::trim)
                .filter(|icon| !icon.is_empty())
                .map(ToOwned::to_owned);
            editor.error = None;
        }
        cx.notify();
    }

    pub(in crate::features) fn toggle_connection_editor_menu(
        &mut self,
        menu: ConnectionEditorMenu,
        cx: &mut Context<Self>,
    ) {
        self.connection_icon_picker_open = false;
        let opening = self.connection_editor_menu != Some(menu);
        self.connection_editor_menu = opening.then_some(menu);
        if !opening && menu == ConnectionEditorMenu::Group {
            if let Some(editor) = self.connection_editor.as_mut() {
                editor.new_group_name.clear();
                if editor.focused_field == ConnectionEditorField::NewGroupName {
                    editor.focused_field = ConnectionEditorField::Name;
                }
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn set_connection_editor_menu_value(
        &mut self,
        menu: ConnectionEditorMenu,
        value: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        let Some(editor) = self.connection_editor.as_mut() else {
            return;
        };
        let value = value.map(ToOwned::to_owned);
        match menu {
            ConnectionEditorMenu::Authentication => {
                editor.auth_mode = value.unwrap_or_else(|| "password".to_string());
                if editor.auth_mode == "none" {
                    editor.password_source = ConnectionEditorPasswordSource::Ask;
                    editor.password_id = None;
                    editor.password.clear();
                    editor.existing_password = None;
                    editor.key_id = None;
                }
            }
            ConnectionEditorMenu::Group => {
                editor.group_id = value;
                editor.new_group_name.clear();
                editor.pending_group_name = None;
                editor.pending_group_parent_id = None;
                editor.focused_field = ConnectionEditorField::Name;
            }
            ConnectionEditorMenu::SavedPassword => editor.password_id = value,
            ConnectionEditorMenu::SshKey => editor.key_id = value,
            ConnectionEditorMenu::Otp => {
                editor.otp_id = value;
                if editor.otp_id.is_none() {
                    editor.auto_fill_otp = false;
                }
            }
            ConnectionEditorMenu::Proxy => editor.proxy_id = value,
            ConnectionEditorMenu::ProxyJump => editor.proxy_jump_id = value,
            ConnectionEditorMenu::Backspace => {
                editor.backspace_mode = value.unwrap_or_else(|| "del".to_string())
            }
            ConnectionEditorMenu::TelnetEnterMode => {
                editor.telnet_enter_mode = value.unwrap_or_else(|| "cr".to_string())
            }
            ConnectionEditorMenu::Shell => {
                editor.shell_path = value.unwrap_or_else(|| "powershell.exe".to_string())
            }
            ConnectionEditorMenu::SerialPort => editor.serial_port = value.unwrap_or_default(),
            ConnectionEditorMenu::BaudRate => {
                editor.baud_rate = value.unwrap_or_else(|| "115200".to_string())
            }
            ConnectionEditorMenu::DataBits => {
                editor.data_bits = value.unwrap_or_else(|| "8".to_string())
            }
            ConnectionEditorMenu::Parity => {
                editor.parity = value.unwrap_or_else(|| "none".to_string())
            }
            ConnectionEditorMenu::StopBits => {
                editor.stop_bits = value.unwrap_or_else(|| "1".to_string())
            }
        }
        editor.error = None;
        self.connection_editor_menu = None;
        cx.notify();
    }

    pub(in crate::features) fn set_connection_editor_password_source(
        &mut self,
        source: ConnectionEditorPasswordSource,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.password_source = source;
            match source {
                ConnectionEditorPasswordSource::Ask => {
                    editor.password_id = None;
                    editor.password.clear();
                    editor.existing_password = None;
                }
                ConnectionEditorPasswordSource::Direct => editor.password_id = None,
                ConnectionEditorPasswordSource::Saved => {
                    editor.password.clear();
                    editor.existing_password = None;
                }
            }
            editor.error = None;
        }
        self.connection_editor_menu = None;
        cx.notify();
    }

    pub(in crate::features) fn set_connection_editor_advanced_tab(
        &mut self,
        tab: ConnectionEditorAdvancedTab,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.connection_editor.as_mut() {
            match tab {
                ConnectionEditorAdvancedTab::Proxy
                | ConnectionEditorAdvancedTab::JumpHost
                | ConnectionEditorAdvancedTab::TwoFactor => editor.advanced_network_tab = tab,
                ConnectionEditorAdvancedTab::PostLogin
                | ConnectionEditorAdvancedTab::X11
                | ConnectionEditorAdvancedTab::Backspace => editor.advanced_behavior_tab = tab,
            }
            if matches!(
                editor.focused_field,
                ConnectionEditorField::PostLoginCommand | ConnectionEditorField::PostLoginDelay
            ) && tab != ConnectionEditorAdvancedTab::PostLogin
            {
                editor.focused_field = ConnectionEditorField::Name;
            }
        }
        self.connection_editor_menu = None;
        cx.notify();
    }

    pub(in crate::features) fn set_connection_editor_telnet_tab(
        &mut self,
        tab: ConnectionEditorTelnetTab,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.telnet_advanced_tab = tab;
            editor.error = None;
        }
        self.connection_editor_menu = None;
        cx.notify();
    }

    pub(in crate::features) fn focus_connection_editor_field(
        &mut self,
        field: ConnectionEditorField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.connection_icon_picker_open = false;
        self.connection_editor_menu = None;
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.focused_field = field;
            editor.error = None;
        }
        window.focus(&self.connection_editor_focus);
        cx.notify();
    }

    pub(in crate::features) fn focus_connection_editor_new_group(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.focused_field = ConnectionEditorField::NewGroupName;
            editor.error = None;
        }
        window.focus(&self.connection_editor_focus);
        cx.notify();
    }

    pub(in crate::features) fn commit_connection_editor_new_group(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let required_message = self.tr("dialog.groupNameRequired").to_string();
        let Some(editor) = self.connection_editor.as_mut() else {
            return;
        };
        let name = editor.new_group_name.trim().to_string();
        if name.is_empty() {
            editor.error = Some(required_message);
            cx.notify();
            return;
        }
        editor.pending_group_parent_id = if editor.pending_group_name.is_some() {
            editor.pending_group_parent_id.clone()
        } else {
            editor.group_id.clone()
        };
        editor.pending_group_name = Some(name);
        editor.group_id = None;
        editor.new_group_name.clear();
        editor.focused_field = ConnectionEditorField::Name;
        editor.error = None;
        self.connection_editor_menu = None;
        cx.notify();
    }

    pub(in crate::features) fn set_connection_editor_kind(
        &mut self,
        kind: ConnectionKindTab,
        cx: &mut Context<Self>,
    ) {
        self.connection_icon_picker_open = false;
        self.connection_editor_menu = None;
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.kind = kind;
            editor.focused_field = ConnectionEditorField::Name;
            editor.port = match kind {
                ConnectionKindTab::Ssh => {
                    if editor.port.trim().is_empty() || editor.port == "23" {
                        "22".to_string()
                    } else {
                        editor.port.clone()
                    }
                }
                ConnectionKindTab::Telnet => {
                    if editor.port.trim().is_empty() || editor.port == "22" {
                        "23".to_string()
                    } else {
                        editor.port.clone()
                    }
                }
                _ => editor.port.clone(),
            };
            editor.error = None;
            self.terminal_status = format!("connection type set to {}", kind.label());
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
        self.terminal_status = "selecting shell executable".to_string();
        cx.spawn(async move |this, cx| {
            let selected = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let _ = this.update(cx, |this, cx| {
                if let Some(path) = selected {
                    if let Some(editor) = this.connection_editor.as_mut() {
                        editor.shell_path = path.display().to_string();
                        editor.error = None;
                    }
                    this.terminal_status = format!("shell path: {}", path.display());
                } else {
                    this.terminal_status = "shell path selection cancelled".to_string();
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
        self.terminal_status = "selecting working directory".to_string();
        cx.spawn(async move |this, cx| {
            let selected = match receiver.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let _ = this.update(cx, |this, cx| {
                if let Some(path) = selected {
                    if let Some(editor) = this.connection_editor.as_mut() {
                        editor.working_dir = path.display().to_string();
                        editor.error = None;
                    }
                    this.terminal_status = format!("working dir: {}", path.display());
                } else {
                    this.terminal_status = "working directory selection cancelled".to_string();
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
        if let Some(editor) = self.connection_editor.as_mut() {
            match flag {
                ConnectionEditorToggle::AutoFillOtp => {
                    editor.auto_fill_otp = editor.otp_id.is_some() && !editor.auto_fill_otp
                }
                ConnectionEditorToggle::X11 => editor.x11_forwarding = !editor.x11_forwarding,
                ConnectionEditorToggle::RawTcp => {
                    editor.raw_tcp_cli = !editor.raw_tcp_cli;
                    if editor.raw_tcp_cli {
                        editor.telnet_enter_mode = "cr".to_string();
                    }
                }
                ConnectionEditorToggle::LocalEcho => editor.local_echo = !editor.local_echo,
                ConnectionEditorToggle::LocalLineEdit => {
                    editor.local_line_edit = !editor.local_line_edit
                }
                ConnectionEditorToggle::ForceCharacterAtATime => {
                    editor.force_character_at_a_time = !editor.force_character_at_a_time
                }
                ConnectionEditorToggle::SendNaws => {
                    if !editor.raw_tcp_cli {
                        editor.send_naws = !editor.send_naws;
                    }
                }
                ConnectionEditorToggle::SendSga => {
                    if !editor.raw_tcp_cli {
                        editor.send_sga = !editor.send_sga;
                    }
                }
                ConnectionEditorToggle::PostLogin => {
                    editor.post_login_enabled = !editor.post_login_enabled
                }
                ConnectionEditorToggle::Advanced => {
                    editor.advanced_open = !editor.advanced_open;
                    if !editor.advanced_open
                        && matches!(
                            editor.focused_field,
                            ConnectionEditorField::PostLoginCommand
                                | ConnectionEditorField::PostLoginDelay
                        )
                    {
                        editor.focused_field = ConnectionEditorField::Name;
                    }
                    self.connection_editor_menu = None;
                }
            }
            editor.error = None;
        }
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
                if self.connection_icon_picker_open {
                    self.connection_icon_picker_open = false;
                    cx.notify();
                    return;
                }
                if self.connection_editor_menu.take().is_some() {
                    if let Some(editor) = self.connection_editor.as_mut() {
                        editor.new_group_name.clear();
                        if editor.focused_field == ConnectionEditorField::NewGroupName {
                            editor.focused_field = ConnectionEditorField::Name;
                        }
                    }
                    cx.notify();
                    return;
                }
                self.close_connection_editor(cx);
                return;
            }
            "enter" => {
                if !keystroke.modifiers.platform
                    && !keystroke.modifiers.control
                    && self.connection_editor.as_ref().is_some_and(|editor| {
                        editor.focused_field == ConnectionEditorField::Description
                    })
                {
                    if let Some(editor) = self.connection_editor.as_mut() {
                        editor.description.push('\n');
                        editor.error = None;
                    }
                    cx.notify();
                    return;
                }
                if self.connection_editor_menu == Some(ConnectionEditorMenu::Group)
                    && self.connection_editor.as_ref().is_some_and(|editor| {
                        editor.focused_field == ConnectionEditorField::NewGroupName
                    })
                {
                    self.commit_connection_editor_new_group(cx);
                    return;
                }
                self.save_connection_editor(window, cx);
                return;
            }
            "tab" if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                if let Some(editor) = self.connection_editor.as_mut() {
                    let password_field_visible = editor.auth_mode == "password"
                        && editor.password_source == ConnectionEditorPasswordSource::Direct;
                    let post_login_fields_visible = editor.advanced_open
                        && editor.post_login_enabled
                        && editor.advanced_behavior_tab == ConnectionEditorAdvancedTab::PostLogin;
                    editor.focused_field = editor.focused_field.next(
                        editor.kind,
                        editor.auth_mode.as_str(),
                        password_field_visible,
                        post_login_fields_visible,
                    );
                    editor.error = None;
                }
                cx.notify();
                return;
            }
            _ => {}
        }

        let Some(editor) = self.connection_editor.as_mut() else {
            return;
        };
        if keystroke.modifiers.platform || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                connection_editor_field_mut(editor).pop();
                editor.error = None;
                cx.notify();
            }
            _ => {
                let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                else {
                    return;
                };
                let field = editor.focused_field;
                let target = connection_editor_field_mut(editor);
                match field {
                    ConnectionEditorField::Port
                    | ConnectionEditorField::BaudRate
                    | ConnectionEditorField::PostLoginDelay => {
                        target.extend(input.chars().filter(|character| character.is_ascii_digit()));
                    }
                    _ => target.push_str(input),
                }
                editor.error = None;
                cx.notify();
            }
        }
    }

    pub(in crate::features) fn save_connection_editor(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(mut editor) = self.connection_editor.clone() else {
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
                self.connection_icon_picker_open = false;
                self.connection_editor_menu = None;
                self.connection_editor = None;
                self.connection_editor_window = None;
                self.selected_connection_ids.clear();
                self.selected_connection_ids.insert(saved.id.clone());
                if let Some(group_id) = saved.group_id.clone() {
                    self.expanded_connection_groups.insert(group_id);
                }
                self.terminal_status = format!("saved connection {}", saved.name);
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

use super::*;

impl NyaTermApp {
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
                group_id: parent_group_id.filter(|value| !value.trim().is_empty()),
                host: String::new(),
                port: "22".to_string(),
                username: "root".to_string(),
                auth_mode: "password".to_string(),
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
                local_echo: false,
                post_login_enabled: false,
                post_login_command: String::new(),
                post_login_delay_ms: "1000".to_string(),
                connect_after_save,
                focused_field: ConnectionEditorField::Name,
                error: None,
            }
        };

        self.connection_editor = Some(editor);
        self.terminal_status = "connection editor opened".to_string();
        window.focus(&self.connection_editor_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_connection_editor(&mut self, cx: &mut Context<Self>) {
        self.connection_editor = None;
        self.terminal_status = "connection editor closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn focus_connection_editor_field(
        &mut self,
        field: ConnectionEditorField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.focused_field = field;
            editor.error = None;
        }
        window.focus(&self.connection_editor_focus);
        cx.notify();
    }

    pub(in crate::features) fn set_connection_editor_kind(
        &mut self,
        kind: ConnectionKindTab,
        cx: &mut Context<Self>,
    ) {
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

    pub(in crate::features) fn cycle_connection_editor_auth_mode(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.auth_mode = match editor.auth_mode.as_str() {
                "none" => "password",
                "password" => "key",
                _ => "none",
            }
            .to_string();
            editor.error = None;
            self.terminal_status = format!("auth mode set to {}", editor.auth_mode);
        }
        cx.notify();
    }

    pub(in crate::features) fn cycle_connection_editor_group(&mut self, cx: &mut Context<Self>) {
        let group_ids = self
            .connection_groups
            .iter()
            .map(|group| group.id.as_str())
            .collect::<Vec<_>>();
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.group_id = next_optional_id(editor.group_id.as_deref(), group_ids.into_iter());
            editor.error = None;
            self.terminal_status = "connection group changed".to_string();
        }
        cx.notify();
    }

    pub(in crate::features) fn cycle_connection_editor_key(&mut self, cx: &mut Context<Self>) {
        let key_ids = self
            .connection_ssh_keys
            .iter()
            .map(|key| key.id.as_str())
            .collect::<Vec<_>>();
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.key_id = next_optional_id(editor.key_id.as_deref(), key_ids.into_iter());
            editor.error = None;
            self.terminal_status = "SSH key selection changed".to_string();
        }
        cx.notify();
    }

    pub(in crate::features) fn cycle_connection_editor_otp(&mut self, cx: &mut Context<Self>) {
        let otp_ids = self
            .connection_otp_entries
            .iter()
            .map(|entry| entry.id.as_str())
            .collect::<Vec<_>>();
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.otp_id = next_optional_id(editor.otp_id.as_deref(), otp_ids.into_iter());
            editor.error = None;
            self.terminal_status = "OTP selection changed".to_string();
        }
        cx.notify();
    }

    pub(in crate::features) fn cycle_connection_editor_proxy(&mut self, cx: &mut Context<Self>) {
        let proxy_ids = self
            .proxies
            .iter()
            .map(|proxy| proxy.id.as_str())
            .collect::<Vec<_>>();
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.proxy_id = next_optional_id(editor.proxy_id.as_deref(), proxy_ids.into_iter());
            editor.error = None;
            self.terminal_status = "proxy selection changed".to_string();
        }
        cx.notify();
    }

    pub(in crate::features) fn cycle_connection_editor_jump(&mut self, cx: &mut Context<Self>) {
        let current_id = self
            .connection_editor
            .as_ref()
            .and_then(|editor| editor.id.clone());
        let jump_ids = self
            .connections
            .iter()
            .filter(|connection| matches!(&connection.config, ConnectionType::Ssh { .. }))
            .filter(|connection| current_id.as_deref() != Some(connection.id.as_str()))
            .map(|connection| connection.id.as_str())
            .collect::<Vec<_>>();
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.proxy_jump_id =
                next_optional_id(editor.proxy_jump_id.as_deref(), jump_ids.into_iter());
            editor.error = None;
            self.terminal_status = "proxy jump selection changed".to_string();
        }
        cx.notify();
    }

    pub(in crate::features) fn cycle_connection_editor_backspace(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.backspace_mode = match editor.backspace_mode.as_str() {
                "ctrl-h" | "bs" | "ctrl_h" => "del".to_string(),
                _ => "ctrl-h".to_string(),
            };
            editor.error = None;
            self.terminal_status = format!("backspace mode: {}", editor.backspace_mode);
        }
        cx.notify();
    }

    pub(in crate::features) fn cycle_connection_editor_serial_port(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if self.connection_serial_ports.is_empty() {
            self.refresh_connection_serial_ports();
        }
        let ports = self.connection_serial_ports.clone();
        if ports.is_empty() {
            self.terminal_status = "no serial ports detected".to_string();
            cx.notify();
            return;
        }
        if let Some(editor) = self.connection_editor.as_mut() {
            let current = ports
                .iter()
                .position(|port| port == &editor.serial_port)
                .unwrap_or(ports.len().saturating_sub(1));
            let next = (current + 1) % ports.len();
            editor.serial_port = ports[next].clone();
            editor.error = None;
            self.terminal_status = format!("serial port set to {}", editor.serial_port);
        }
        cx.notify();
    }

    pub(in crate::features) fn cycle_connection_editor_data_bits(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.data_bits = match editor.data_bits.trim() {
                "5" => "6".to_string(),
                "6" => "7".to_string(),
                "7" => "8".to_string(),
                _ => "5".to_string(),
            };
            self.terminal_status = format!("serial data bits: {}", editor.data_bits);
        }
        cx.notify();
    }

    pub(in crate::features) fn cycle_connection_editor_parity(&mut self, cx: &mut Context<Self>) {
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.parity = match editor.parity.trim().to_ascii_lowercase().as_str() {
                "none" => "odd".to_string(),
                "odd" => "even".to_string(),
                "even" => "mark".to_string(),
                "mark" => "space".to_string(),
                _ => "none".to_string(),
            };
            self.terminal_status = format!("serial parity: {}", editor.parity);
        }
        cx.notify();
    }

    pub(in crate::features) fn cycle_connection_editor_stop_bits(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.stop_bits = match editor.stop_bits.trim() {
                "1" => "1.5".to_string(),
                "1.5" => "2".to_string(),
                _ => "1".to_string(),
            };
            self.terminal_status = format!("serial stop bits: {}", editor.stop_bits);
        }
        cx.notify();
    }

    pub(in crate::features) fn cycle_connection_editor_baud_preset(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        const PRESETS: &[&str] = &[
            "9600", "19200", "38400", "57600", "115200", "230400", "460800", "921600",
        ];
        if let Some(editor) = self.connection_editor.as_mut() {
            let current = editor.baud_rate.trim();
            let next = PRESETS
                .iter()
                .position(|preset| *preset == current)
                .map(|index| PRESETS[(index + 1) % PRESETS.len()])
                .unwrap_or("115200");
            editor.baud_rate = next.to_string();
            self.terminal_status = format!("serial baud: {}", editor.baud_rate);
        }
        cx.notify();
    }

    pub(in crate::features) fn set_connection_editor_shell_path(
        &mut self,
        shell_path: &str,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.shell_path = shell_path.to_string();
            editor.error = None;
            self.terminal_status = format!("shell path: {shell_path}");
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
                ConnectionEditorToggle::AutoFillOtp => editor.auto_fill_otp = !editor.auto_fill_otp,
                ConnectionEditorToggle::X11 => editor.x11_forwarding = !editor.x11_forwarding,
                ConnectionEditorToggle::RawTcp => editor.raw_tcp_cli = !editor.raw_tcp_cli,
                ConnectionEditorToggle::LocalEcho => editor.local_echo = !editor.local_echo,
                ConnectionEditorToggle::PostLogin => {
                    editor.post_login_enabled = !editor.post_login_enabled
                }
                ConnectionEditorToggle::ConnectAfterSave => {
                    editor.connect_after_save = !editor.connect_after_save
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
                self.close_connection_editor(cx);
                return;
            }
            "enter" => {
                self.save_connection_editor(window, cx);
                return;
            }
            "tab" if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                if let Some(editor) = self.connection_editor.as_mut() {
                    editor.focused_field = editor
                        .focused_field
                        .next(editor.kind, editor.auth_mode.as_str());
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
        let Some(editor) = self.connection_editor.clone() else {
            return;
        };

        let built = match build_saved_connection_from_editor(&editor) {
            Ok(connection) => connection,
            Err(error) => {
                self.set_connection_editor_error(error, cx);
                return;
            }
        };

        match self.persist_saved_connection(built.clone()) {
            Ok(saved) => {
                let connect_after_save = editor.connect_after_save;
                self.connection_editor = None;
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

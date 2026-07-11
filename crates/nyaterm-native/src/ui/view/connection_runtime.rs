use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn open_connection_editor(
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

    pub(in crate::ui::view) fn close_connection_editor(&mut self, cx: &mut Context<Self>) {
        self.connection_editor = None;
        self.terminal_status = "connection editor closed".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn focus_connection_editor_field(
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

    pub(in crate::ui::view) fn set_connection_editor_kind(
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

    pub(in crate::ui::view) fn cycle_connection_editor_auth_mode(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn cycle_connection_editor_group(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn cycle_connection_editor_key(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn cycle_connection_editor_otp(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn cycle_connection_editor_proxy(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn cycle_connection_editor_jump(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn cycle_connection_editor_backspace(
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

    pub(in crate::ui::view) fn cycle_connection_editor_serial_port(
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


    pub(in crate::ui::view) fn cycle_connection_editor_data_bits(
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

    pub(in crate::ui::view) fn cycle_connection_editor_parity(
        &mut self,
        cx: &mut Context<Self>,
    ) {
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

    pub(in crate::ui::view) fn cycle_connection_editor_stop_bits(
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

    pub(in crate::ui::view) fn cycle_connection_editor_baud_preset(
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

    pub(in crate::ui::view) fn set_connection_editor_shell_path(
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

    pub(in crate::ui::view) fn prompt_connection_editor_shell_path(
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

    pub(in crate::ui::view) fn prompt_connection_editor_working_dir(
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

    pub(in crate::ui::view) fn toggle_connection_editor_flag(
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

    pub(in crate::ui::view) fn handle_connection_editor_key_down(
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
                    editor.focused_field =
                        editor.focused_field.next(editor.kind, editor.auth_mode.as_str());
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

    pub(in crate::ui::view) fn save_connection_editor(
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

    pub(in crate::ui::view) fn open_connection_group_editor(
        &mut self,
        group_id: Option<String>,
        parent_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let name = match group_id.as_deref() {
            Some(id) => self
                .connection_groups
                .iter()
                .find(|group| group.id == id)
                .map(|group| group.name.clone()),
            None => Some(String::new()),
        };
        let Some(name) = name else {
            self.terminal_status = "connection group is no longer available".to_string();
            cx.notify();
            return;
        };
        let parent_id = group_id
            .as_deref()
            .and_then(|id| {
                self.connection_groups
                    .iter()
                    .find(|group| group.id == id)
                    .and_then(|group| group.parent_id.clone())
            })
            .or(parent_id);

        self.connection_group_editor = Some(ConnectionGroupEditorState {
            id: group_id,
            name,
            parent_id,
            error: None,
        });
        self.terminal_status = "connection group editor opened".to_string();
        window.focus(&self.connection_group_editor_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn close_connection_group_editor(&mut self, cx: &mut Context<Self>) {
        self.connection_group_editor = None;
        self.terminal_status = "connection group editor closed".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_connection_group_editor_key_down(
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
                if let Some(editor) = self.connection_group_editor.as_mut() {
                    editor.name.pop();
                    editor.error = None;
                }
                cx.notify();
            }
            _ if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    if let Some(editor) = self.connection_group_editor.as_mut() {
                        editor.name.push_str(input);
                        editor.error = None;
                    }
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    pub(in crate::ui::view) fn save_connection_group_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.connection_group_editor.clone() else {
            return;
        };
        let name = editor.name.trim().to_string();
        if name.is_empty() {
            if let Some(state) = self.connection_group_editor.as_mut() {
                state.error = Some("Group name is required".to_string());
            }
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
                    self.connection_groups
                        .iter()
                        .find(|group| group.id == id)
                        .map(|group| group.sort_order)
                })
                .unwrap_or(self.connection_groups.len() as i32),
            created_at_ms: None,
            updated_at_ms: None,
        };

        match self.with_connection_store(|store| store.save_group(&group)) {
            Ok(()) => {
                self.expanded_connection_groups.insert(group.id.clone());
                self.connection_group_editor = None;
                self.refresh_store_from_runtime();
                self.terminal_status = format!("saved connection group {}", group.name);
            }
            Err(error) => {
                if let Some(state) = self.connection_group_editor.as_mut() {
                    state.error = Some(error);
                }
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn open_connection_delete_confirm(
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

    pub(in crate::ui::view) fn close_connection_delete_confirm(&mut self, cx: &mut Context<Self>) {
        self.connection_delete_confirm = None;
        cx.notify();
    }

    pub(in crate::ui::view) fn confirm_connection_delete(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn open_connection_group_delete_confirm(
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

    pub(in crate::ui::view) fn close_connection_group_delete_confirm(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.connection_group_delete_confirm = None;
        cx.notify();
    }

    pub(in crate::ui::view) fn confirm_connection_group_delete(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn toggle_connection_group_expanded(
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

    pub(in crate::ui::view) fn cycle_connection_sort_mode(&mut self, cx: &mut Context<Self>) {
        self.connection_sort_mode = self.connection_sort_mode.next();
        self.connection_list_offset = 0;
        self.terminal_status = format!("connections sorted by {}", self.connection_sort_mode.label());
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_connection_search_key_down(
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

    pub(in crate::ui::view) fn delete_selected_connections(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn rename_connection(
        &mut self,
        connection_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_connection_editor(Some(connection_id), None, false, window, cx);
    }

    fn set_connection_editor_error(&mut self, error: String, cx: &mut Context<Self>) {
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.error = Some(error.clone());
        }
        self.terminal_status = error;
        cx.notify();
    }

    fn persist_saved_connection(&mut self, connection: SavedConnection) -> Result<SavedConnection, String> {
        self.with_connection_store(|store| {
            store.save_connection(&connection)?;
            Ok(())
        })?;
        self.refresh_store_from_runtime();
        self.connections
            .iter()
            .find(|item| item.id == connection.id)
            .cloned()
            .ok_or_else(|| "saved connection was not reloaded".to_string())
    }

    pub(in crate::ui::view) fn with_connection_store<T>(
        &self,
        f: impl FnOnce(&ConnectionStore) -> Result<T, StorageError>,
    ) -> Result<T, String> {
        let store = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .map_err(|error| error.to_string())?;
        f(&store).map_err(|error| error.to_string())
    }

    fn refresh_connection_auth_catalog(&mut self) {
        if let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            self.connection_ssh_keys = store.list_ssh_keys().unwrap_or_default();
            self.connection_otp_entries = store.list_otp_entries().unwrap_or_default();
            self.connection_saved_passwords = store.list_passwords().unwrap_or_default();
            self.connection_saved_credentials = store.list_credentials().unwrap_or_default();
        }
        self.refresh_connection_serial_ports();
    }

    fn refresh_connection_serial_ports(&mut self) {
        self.connection_serial_ports = self
            .session_manager
            .list_serial_ports()
            .unwrap_or_default();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ui::view) enum ConnectionEditorToggle {
    AutoFillOtp,
    X11,
    RawTcp,
    LocalEcho,
    PostLogin,
    ConnectAfterSave,
}

fn connection_editor_from_saved(
    connection: SavedConnection,
    connect_after_save: bool,
) -> ConnectionEditorState {
    let auth = connection.auth.clone().unwrap_or_default();
    let network = connection.network.clone().unwrap_or(ConnectionNetwork {
        proxy_id: None,
        proxy_jump_id: None,
    });
    let post_login = connection.post_login.clone().unwrap_or(ConnectionPostLogin {
        enabled: false,
        command: String::new(),
        delay_ms: 1000,
    });
    let mut editor = ConnectionEditorState {
        id: Some(connection.id),
        kind: ConnectionKindTab::from_connection_type(&connection.config),
        name: connection.name,
        description: connection.description.unwrap_or_default(),
        group_id: connection.group_id,
        host: String::new(),
        port: String::new(),
        username: "root".to_string(),
        auth_mode: auth.mode,
        password: String::new(),
        existing_password: auth.password.filter(|value| !value.is_empty()),
        key_id: auth.key_id,
        otp_id: auth.otp_id,
        auto_fill_otp: auth.auto_fill_otp,
        proxy_id: network.proxy_id,
        proxy_jump_id: network.proxy_jump_id,
        x11_forwarding: false,
        backspace_mode: "del".to_string(),
        shell_path: String::new(),
        shell_args: String::new(),
        working_dir: String::new(),
        serial_port: String::new(),
        baud_rate: "115200".to_string(),
        data_bits: "8".to_string(),
        parity: "none".to_string(),
        stop_bits: "1".to_string(),
        raw_tcp_cli: false,
        local_echo: false,
        post_login_enabled: post_login.enabled,
        post_login_command: post_login.command,
        post_login_delay_ms: post_login.delay_ms.to_string(),
        connect_after_save,
        focused_field: ConnectionEditorField::Name,
        error: None,
    };

    match connection.config {
        ConnectionType::Ssh {
            host,
            port,
            username,
            backspace_mode,
            x11_forwarding,
            ..
        } => {
            editor.host = host;
            editor.port = port.to_string();
            editor.username = username;
            editor.backspace_mode = backspace_mode;
            editor.x11_forwarding = x11_forwarding;
        }
        ConnectionType::LocalTerminal {
            shell_path,
            shell_args,
            working_dir,
            ..
        } => {
            editor.shell_path = shell_path;
            editor.shell_args = shell_args;
            editor.working_dir = working_dir.unwrap_or_default();
        }
        ConnectionType::Telnet {
            host,
            port,
            raw_tcp_cli,
            local_echo,
            backspace_mode,
            ..
        } => {
            editor.host = host;
            editor.port = port.to_string();
            editor.raw_tcp_cli = raw_tcp_cli;
            editor.local_echo = local_echo;
            editor.backspace_mode = backspace_mode;
        }
        ConnectionType::Serial {
            port_name,
            baud_rate,
            data_bits,
            parity,
            stop_bits,
            backspace_mode,
            ..
        } => {
            editor.serial_port = port_name;
            editor.baud_rate = baud_rate.to_string();
            editor.data_bits = data_bits.to_string();
            editor.parity = parity;
            editor.stop_bits = stop_bits;
            editor.backspace_mode = backspace_mode;
        }
    }
    editor
}

fn connection_editor_field_mut(editor: &mut ConnectionEditorState) -> &mut String {
    match editor.focused_field {
        ConnectionEditorField::Name => &mut editor.name,
        ConnectionEditorField::Description => &mut editor.description,
        ConnectionEditorField::Host => &mut editor.host,
        ConnectionEditorField::Port => &mut editor.port,
        ConnectionEditorField::Username => &mut editor.username,
        ConnectionEditorField::Password => &mut editor.password,
        ConnectionEditorField::ShellPath => &mut editor.shell_path,
        ConnectionEditorField::ShellArgs => &mut editor.shell_args,
        ConnectionEditorField::WorkingDir => &mut editor.working_dir,
        ConnectionEditorField::SerialPort => &mut editor.serial_port,
        ConnectionEditorField::BaudRate => &mut editor.baud_rate,
        ConnectionEditorField::PostLoginCommand => &mut editor.post_login_command,
        ConnectionEditorField::PostLoginDelay => &mut editor.post_login_delay_ms,
    }
}

fn build_saved_connection_from_editor(
    editor: &ConnectionEditorState,
) -> Result<SavedConnection, String> {
    let name = editor.name.trim().to_string();
    if name.is_empty() {
        return Err("Connection name is required".to_string());
    }

    let config = match editor.kind {
        ConnectionKindTab::Ssh => {
            let host = editor.host.trim().to_string();
            if host.is_empty() {
                return Err("SSH host is required".to_string());
            }
            let port = parse_port(&editor.port, "SSH port")?;
            let username = editor.username.trim().to_string();
            if username.is_empty() {
                return Err("SSH username is required".to_string());
            }
            if editor.auth_mode == "key"
                && editor
                    .key_id
                    .as_deref()
                    .is_none_or(|value| value.trim().is_empty())
            {
                return Err("Select an SSH key for key authentication".to_string());
            }
            ConnectionType::Ssh {
                host,
                port,
                username,
                backspace_mode: non_empty_or(editor.backspace_mode.clone(), "del"),
                ai_execution_profile: AiExecutionProfile::Auto,
                x11_forwarding: editor.x11_forwarding,
            }
        }
        ConnectionKindTab::Local => ConnectionType::LocalTerminal {
            shell_path: editor.shell_path.trim().to_string(),
            shell_args: editor.shell_args.trim().to_string(),
            working_dir: non_empty_optional(&editor.working_dir),
            ai_execution_profile: AiExecutionProfile::Posix,
        },
        ConnectionKindTab::Telnet => {
            let host = editor.host.trim().to_string();
            if host.is_empty() {
                return Err("Telnet host is required".to_string());
            }
            let port = parse_port(&editor.port, "Telnet port")?;
            ConnectionType::Telnet {
                host,
                port,
                ai_execution_profile: AiExecutionProfile::Auto,
                backspace_mode: non_empty_or(editor.backspace_mode.clone(), "del"),
                raw_tcp_cli: editor.raw_tcp_cli,
                enter_mode: "crlf".to_string(),
                local_echo: editor.local_echo,
                local_line_edit: false,
                force_character_at_a_time: false,
                send_naws: true,
                send_sga: true,
            }
        }
        ConnectionKindTab::Serial => {
            let port_name = editor.serial_port.trim().to_string();
            if port_name.is_empty() {
                return Err("Serial port is required".to_string());
            }
            let baud_rate = editor
                .baud_rate
                .trim()
                .parse::<u32>()
                .map_err(|_| "Baud rate must be a number".to_string())?;
            if !(1..=4_000_000).contains(&baud_rate) {
                return Err("Baud rate must be between 1 and 4000000".to_string());
            }
            let data_bits = editor
                .data_bits
                .trim()
                .parse::<u8>()
                .unwrap_or(8)
                .clamp(5, 8);
            ConnectionType::Serial {
                port_name,
                baud_rate,
                data_bits,
                parity: non_empty_or(editor.parity.clone(), "none"),
                stop_bits: non_empty_or(editor.stop_bits.clone(), "1"),
                ai_execution_profile: AiExecutionProfile::Auto,
                backspace_mode: non_empty_or(editor.backspace_mode.clone(), "del"),
            }
        }
    };

    let auth = match editor.kind {
        ConnectionKindTab::Ssh => {
            let password = editor.password.trim().to_string();
            let existing = editor.existing_password.clone();
            Some(ConnectionAuth {
                mode: non_empty_or(editor.auth_mode.clone(), "password"),
                password_id: None,
                password: if !password.is_empty() {
                    Some(password)
                } else {
                    existing
                },
                key_id: editor.key_id.clone().filter(|value| !value.trim().is_empty()),
                otp_id: editor.otp_id.clone().filter(|value| !value.trim().is_empty()),
                auto_fill_otp: editor.auto_fill_otp,
                has_password: false,
            })
        }
        _ => None,
    };

    let network = match editor.kind {
        ConnectionKindTab::Ssh => {
            let proxy_id = editor.proxy_id.clone().filter(|value| !value.trim().is_empty());
            let proxy_jump_id = editor
                .proxy_jump_id
                .clone()
                .filter(|value| !value.trim().is_empty());
            if proxy_id.is_some() || proxy_jump_id.is_some() {
                Some(ConnectionNetwork {
                    proxy_id,
                    proxy_jump_id,
                })
            } else {
                None
            }
        }
        _ => None,
    };

    let post_login = if editor.kind == ConnectionKindTab::Ssh
        && (editor.post_login_enabled || !editor.post_login_command.trim().is_empty())
    {
        let delay_ms = editor
            .post_login_delay_ms
            .trim()
            .parse::<u64>()
            .unwrap_or(1000)
            .min(60_000);
        Some(ConnectionPostLogin {
            enabled: editor.post_login_enabled,
            command: editor.post_login_command.clone(),
            delay_ms,
        })
    } else {
        None
    };

    Ok(SavedConnection {
        id: editor.id.clone().unwrap_or_else(uuid),
        name,
        config,
        group_id: editor.group_id.clone().filter(|value| !value.trim().is_empty()),
        description: non_empty_optional(&editor.description),
        sort_order: 0,
        icon: None,
        auth,
        network,
        post_login,
        created_at_ms: None,
        updated_at_ms: None,
        last_used_at_ms: None,
    })
}

fn parse_port(value: &str, label: &str) -> Result<u16, String> {
    let port = value
        .trim()
        .parse::<u16>()
        .map_err(|_| format!("{label} must be 1-65535"))?;
    if port == 0 {
        return Err(format!("{label} must be 1-65535"));
    }
    Ok(port)
}

fn non_empty_or(value: String, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn non_empty_optional(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn next_optional_id<'a>(
    current: Option<&str>,
    ids: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let ids = ids.into_iter().collect::<Vec<_>>();
    if ids.is_empty() {
        return None;
    }
    let current_index = current.and_then(|value| ids.iter().position(|id| *id == value));
    match current_index {
        None => Some(ids[0].to_string()),
        Some(index) if index + 1 < ids.len() => Some(ids[index + 1].to_string()),
        Some(_) => None,
    }
}

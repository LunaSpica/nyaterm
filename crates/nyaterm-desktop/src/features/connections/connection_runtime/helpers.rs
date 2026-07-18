use super::*;

pub(super) fn connection_editor_from_saved(
    connection: SavedConnection,
    connect_after_save: bool,
) -> ConnectionEditorState {
    let auth = connection.auth.clone().unwrap_or_default();
    let network = connection.network.clone().unwrap_or(ConnectionNetwork {
        proxy_id: None,
        proxy_jump_id: None,
    });
    let post_login = connection
        .post_login
        .clone()
        .unwrap_or(ConnectionPostLogin {
            enabled: false,
            command: String::new(),
            delay_ms: 1000,
        });
    let password_source = if auth.password_id.is_some() {
        ConnectionEditorPasswordSource::Saved
    } else if auth.password.is_some() || auth.has_password {
        ConnectionEditorPasswordSource::Direct
    } else {
        ConnectionEditorPasswordSource::Ask
    };
    let mut editor = ConnectionEditorState {
        id: Some(connection.id),
        kind: ConnectionKindTab::from_connection_type(&connection.config),
        name: connection.name,
        description: connection.description.unwrap_or_default(),
        icon: connection.icon,
        group_id: connection.group_id,
        new_group_name: String::new(),
        pending_group_name: None,
        pending_group_parent_id: None,
        host: String::new(),
        port: String::new(),
        username: "root".to_string(),
        auth_mode: auth.mode,
        password_source,
        password_id: auth.password_id,
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
        telnet_enter_mode: "cr".to_string(),
        local_echo: false,
        local_line_edit: false,
        force_character_at_a_time: false,
        send_naws: true,
        send_sga: true,
        post_login_enabled: post_login.enabled,
        post_login_command: post_login.command,
        post_login_delay_ms: post_login.delay_ms.to_string(),
        advanced_open: false,
        advanced_network_tab: ConnectionEditorAdvancedTab::Proxy,
        advanced_behavior_tab: ConnectionEditorAdvancedTab::PostLogin,
        telnet_advanced_tab: ConnectionEditorTelnetTab::Input,
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
            enter_mode,
            local_echo,
            local_line_edit,
            force_character_at_a_time,
            send_naws,
            send_sga,
            backspace_mode,
            ..
        } => {
            editor.host = host;
            editor.port = port.to_string();
            editor.raw_tcp_cli = raw_tcp_cli;
            editor.telnet_enter_mode = enter_mode;
            editor.local_echo = local_echo;
            editor.local_line_edit = local_line_edit;
            editor.force_character_at_a_time = force_character_at_a_time;
            editor.send_naws = send_naws;
            editor.send_sga = send_sga;
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

pub(super) fn connection_editor_field_mut(editor: &mut ConnectionEditorState) -> &mut String {
    match editor.focused_field {
        ConnectionEditorField::Name => &mut editor.name,
        ConnectionEditorField::NewGroupName => &mut editor.new_group_name,
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

pub(super) fn build_saved_connection_from_editor(
    editor: &ConnectionEditorState,
) -> Result<SavedConnection, String> {
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
            ConnectionType::Ssh {
                host,
                port,
                username,
                backspace_mode: non_empty_or(editor.backspace_mode.clone(), "del"),
                ai_execution_profile: AiExecutionProfile::Auto,
                x11_forwarding: editor.x11_forwarding,
            }
        }
        ConnectionKindTab::Local => {
            let shell_path = editor.shell_path.trim().to_string();
            if shell_path.is_empty() {
                return Err("Shell path is required".to_string());
            }
            ConnectionType::LocalTerminal {
                shell_path,
                shell_args: editor.shell_args.trim().to_string(),
                working_dir: non_empty_optional(&editor.working_dir),
                ai_execution_profile: AiExecutionProfile::Posix,
            }
        }
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
                enter_mode: non_empty_or(editor.telnet_enter_mode.clone(), "cr"),
                local_echo: editor.local_echo,
                local_line_edit: editor.local_line_edit,
                force_character_at_a_time: editor.force_character_at_a_time,
                send_naws: editor.send_naws,
                send_sga: editor.send_sga,
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

    if editor.kind == ConnectionKindTab::Ssh {
        if editor.post_login_enabled && editor.post_login_command.trim().is_empty() {
            return Err("Post-login command is required".to_string());
        }
        let delay = editor
            .post_login_delay_ms
            .trim()
            .parse::<u64>()
            .map_err(|_| "Post-login delay must be between 0 and 60000 ms".to_string())?;
        if delay > 60_000 {
            return Err("Post-login delay must be between 0 and 60000 ms".to_string());
        }
    }

    let name = if editor.name.trim().is_empty() {
        match &config {
            ConnectionType::Ssh { host, port, .. } | ConnectionType::Telnet { host, port, .. } => {
                format!("{host}:{port}")
            }
            ConnectionType::LocalTerminal { .. } => "Local Terminal".to_string(),
            ConnectionType::Serial { port_name, .. } => port_name.clone(),
        }
    } else {
        editor.name.trim().to_string()
    };

    let auth = match editor.kind {
        ConnectionKindTab::Ssh => {
            let password = editor.password.trim().to_string();
            let existing = editor.existing_password.clone();
            let mode = match editor.auth_mode.as_str() {
                "password" => "password".to_string(),
                "key"
                    if editor
                        .key_id
                        .as_deref()
                        .is_some_and(|value| !value.trim().is_empty()) =>
                {
                    "key".to_string()
                }
                _ => "none".to_string(),
            };
            Some(ConnectionAuth {
                password_id: (mode == "password"
                    && editor.password_source == ConnectionEditorPasswordSource::Saved)
                    .then(|| editor.password_id.clone())
                    .flatten(),
                password: (mode == "password"
                    && editor.password_source == ConnectionEditorPasswordSource::Direct)
                    .then(|| {
                        if !password.is_empty() {
                            Some(password)
                        } else {
                            existing
                        }
                    })
                    .flatten(),
                key_id: (mode == "key")
                    .then(|| {
                        editor
                            .key_id
                            .clone()
                            .filter(|value| !value.trim().is_empty())
                    })
                    .flatten(),
                otp_id: editor
                    .otp_id
                    .clone()
                    .filter(|value| !value.trim().is_empty()),
                auto_fill_otp: editor.auto_fill_otp,
                has_password: false,
                mode,
            })
        }
        _ => None,
    };

    let network = match editor.kind {
        ConnectionKindTab::Ssh => {
            let proxy_id = editor
                .proxy_id
                .clone()
                .filter(|value| !value.trim().is_empty());
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
        group_id: editor
            .group_id
            .clone()
            .filter(|value| !value.trim().is_empty()),
        description: non_empty_optional(&editor.description),
        sort_order: 0,
        icon: editor.icon.clone().filter(|value| !value.trim().is_empty()),
        auth,
        network,
        post_login,
        created_at_ms: None,
        updated_at_ms: None,
        last_used_at_ms: None,
    })
}

pub(super) fn parse_port(value: &str, label: &str) -> Result<u16, String> {
    let port = value
        .trim()
        .parse::<u16>()
        .map_err(|_| format!("{label} must be 1-65535"))?;
    if port == 0 {
        return Err(format!("{label} must be 1-65535"));
    }
    Ok(port)
}

pub(super) fn non_empty_or(value: String, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_editor_round_trip_preserves_icon() {
        let connection = SavedConnection {
            id: "connection-1".to_string(),
            name: "Local".to_string(),
            config: ConnectionType::LocalTerminal {
                shell_path: "bash".to_string(),
                shell_args: String::new(),
                working_dir: None,
                ai_execution_profile: AiExecutionProfile::Posix,
            },
            group_id: None,
            description: None,
            sort_order: 0,
            icon: Some("linux".to_string()),
            auth: None,
            network: None,
            post_login: None,
            created_at_ms: None,
            updated_at_ms: None,
            last_used_at_ms: None,
        };

        let editor = connection_editor_from_saved(connection, false);
        assert_eq!(editor.icon.as_deref(), Some("linux"));
        let saved = build_saved_connection_from_editor(&editor).expect("valid connection");
        assert_eq!(saved.icon.as_deref(), Some("linux"));
    }

    #[test]
    fn connection_editor_round_trip_preserves_saved_password_reference() {
        let connection = SavedConnection {
            id: "connection-ssh".to_string(),
            name: "SSH".to_string(),
            config: ConnectionType::Ssh {
                host: "example.com".to_string(),
                port: 22,
                username: "root".to_string(),
                backspace_mode: "del".to_string(),
                ai_execution_profile: AiExecutionProfile::Auto,
                x11_forwarding: false,
            },
            group_id: None,
            description: None,
            sort_order: 0,
            icon: None,
            auth: Some(ConnectionAuth {
                mode: "password".to_string(),
                password_id: Some("password-1".to_string()),
                ..ConnectionAuth::default()
            }),
            network: None,
            post_login: None,
            created_at_ms: None,
            updated_at_ms: None,
            last_used_at_ms: None,
        };

        let editor = connection_editor_from_saved(connection, false);
        assert_eq!(
            editor.password_source,
            ConnectionEditorPasswordSource::Saved
        );
        assert_eq!(editor.password_id.as_deref(), Some("password-1"));

        let saved = build_saved_connection_from_editor(&editor).expect("valid connection");
        assert_eq!(
            saved.auth.and_then(|auth| auth.password_id).as_deref(),
            Some("password-1")
        );
    }

    #[test]
    fn connection_editor_round_trip_preserves_telnet_behavior() {
        let connection = SavedConnection {
            id: "connection-telnet".to_string(),
            name: "Telnet".to_string(),
            config: ConnectionType::Telnet {
                host: "device.local".to_string(),
                port: 2323,
                ai_execution_profile: AiExecutionProfile::Auto,
                backspace_mode: "ctrl_h".to_string(),
                raw_tcp_cli: true,
                enter_mode: "lf".to_string(),
                local_echo: true,
                local_line_edit: true,
                force_character_at_a_time: true,
                send_naws: false,
                send_sga: false,
            },
            group_id: None,
            description: None,
            sort_order: 0,
            icon: None,
            auth: None,
            network: None,
            post_login: None,
            created_at_ms: None,
            updated_at_ms: None,
            last_used_at_ms: None,
        };

        let editor = connection_editor_from_saved(connection, false);
        let saved = build_saved_connection_from_editor(&editor).expect("valid connection");
        let ConnectionType::Telnet {
            enter_mode,
            local_line_edit,
            force_character_at_a_time,
            send_naws,
            send_sga,
            ..
        } = saved.config
        else {
            panic!("expected telnet connection");
        };
        assert_eq!(enter_mode, "lf");
        assert!(local_line_edit);
        assert!(force_character_at_a_time);
        assert!(!send_naws);
        assert!(!send_sga);
    }
}

pub(super) fn non_empty_optional(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

impl NyaTermApp {
    pub(in crate::features) fn set_connection_editor_error(
        &mut self,
        error: String,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.connection_editor.as_mut() {
            editor.error = Some(error.clone());
        }
        self.terminal_status = error;
        cx.notify();
    }

    pub(in crate::features) fn persist_saved_connection_with_group(
        &mut self,
        connection: SavedConnection,
        group: Option<&Group>,
    ) -> Result<SavedConnection, String> {
        self.with_connection_store(|store| {
            if let Some(group) = group {
                store.save_group_and_connection(group, &connection)?;
            } else {
                store.save_connection(&connection)?;
            }
            Ok(())
        })?;
        self.refresh_store_from_runtime();
        self.connections
            .iter()
            .find(|item| item.id == connection.id)
            .cloned()
            .ok_or_else(|| "saved connection was not reloaded".to_string())
    }

    pub(in crate::features) fn with_connection_store<T>(
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

    pub(in crate::features) fn refresh_connection_auth_catalog(&mut self) {
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

    pub(in crate::features) fn refresh_connection_serial_ports(&mut self) {
        self.connection_serial_ports = self.session_manager.list_serial_ports().unwrap_or_default();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::features) enum ConnectionEditorToggle {
    AutoFillOtp,
    X11,
    RawTcp,
    LocalEcho,
    LocalLineEdit,
    ForceCharacterAtATime,
    SendNaws,
    SendSga,
    PostLogin,
    Advanced,
}

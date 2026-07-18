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

pub(super) fn connection_editor_field_mut(editor: &mut ConnectionEditorState) -> &mut String {
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

pub(super) fn build_saved_connection_from_editor(
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
                key_id: editor
                    .key_id
                    .clone()
                    .filter(|value| !value.trim().is_empty()),
                otp_id: editor
                    .otp_id
                    .clone()
                    .filter(|value| !value.trim().is_empty()),
                auto_fill_otp: editor.auto_fill_otp,
                has_password: false,
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
        icon: None,
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

pub(super) fn non_empty_optional(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub(super) fn next_optional_id<'a>(
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

    pub(in crate::features) fn persist_saved_connection(
        &mut self,
        connection: SavedConnection,
    ) -> Result<SavedConnection, String> {
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
    PostLogin,
}

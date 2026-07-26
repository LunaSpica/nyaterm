use super::*;

use crate::models::{MainMode, StartupCommandRequest};

#[derive(Clone)]
pub(in crate::features) struct SshSessionConfigBuildContext {
    pub(in crate::features) config_dir: PathBuf,
    pub(in crate::features) portable_key_path: Option<PathBuf>,
    pub(in crate::features) host_key_policy: String,
    pub(in crate::features) x11_display: String,
    pub(in crate::features) keep_alive_interval_secs: u32,
    pub(in crate::features) host_key_prompts: Arc<HostKeyPromptBroker>,
    pub(in crate::features) credential_prompts: Arc<CredentialPromptBroker>,
    pub(in crate::features) otp_provider: Arc<NativeOtpProvider>,
}

impl NyaTermApp {
    pub(in crate::features) fn start_local_session(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut config = LocalSessionConfig::default();
        self.apply_desired_geometry_to_local_config(&mut config);
        let name = config.name.clone();
        self.begin_background_session_start(
            name,
            SessionLaunchConfig::Local(config),
            None,
            AiExecutionProfile::Posix,
            None,
            None,
            None,
            None,
            None,
            None,
            cx,
        );
    }

    pub(in crate::features) fn start_saved_connection(
        &mut self,
        connection: SavedConnection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.start_saved_connection_with_options(
            connection,
            SavedConnectionStartOptions::default(),
            window,
            cx,
        );
    }

    pub(in crate::features) fn start_saved_connection_with_options(
        &mut self,
        connection: SavedConnection,
        options: SavedConnectionStartOptions,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dismiss_connection_hover(cx);
        if self.saved_connection_start_is_pending_or_queued(&connection) {
            self.terminal.view.status =
                format!("{} is already connecting or queued", connection.name);
            self.selected_nav = NavItem::Workspace;
            self.main_mode = MainMode::Workspace;
            cx.notify();
            return;
        }
        if self.has_pending_session_start() {
            self.enqueue_saved_connection_start_with_options(connection, options, cx);
            return;
        }

        match connection.config.clone() {
            ConnectionType::LocalTerminal {
                shell_path,
                shell_args,
                working_dir,
                ai_execution_profile,
            } => {
                let mut config = LocalSessionConfig {
                    name: connection.name.clone(),
                    shell_path: non_empty_string(shell_path),
                    shell_args: split_shell_args(&shell_args),
                    working_dir: working_dir
                        .filter(|value| !value.trim().is_empty())
                        .map(Into::into),
                    cols: 80,
                    rows: 24,
                    pixel_width: 0,
                    pixel_height: 0,
                };
                self.apply_desired_geometry_to_local_config(&mut config);
                self.begin_background_session_start(
                    connection.name,
                    SessionLaunchConfig::Local(config),
                    Some(connection.id),
                    ai_execution_profile,
                    options.custom_name,
                    options.tab_color,
                    options.after_session_id,
                    options.insert_index,
                    options.seed_output,
                    options.startup_command,
                    cx,
                );
            }
            ConnectionType::Telnet {
                host,
                port,
                ai_execution_profile,
                raw_tcp_cli,
                enter_mode,
                force_character_at_a_time,
                send_naws,
                send_sga,
                ..
            } => {
                let config = TelnetSessionConfig {
                    name: connection.name.clone(),
                    host,
                    port,
                    raw_tcp: raw_tcp_cli,
                    enter_mode: parse_telnet_enter_mode(&enter_mode),
                    force_character_at_a_time,
                    send_naws,
                    send_sga,
                    cols: 80,
                    rows: 24,
                };
                self.begin_background_session_start(
                    connection.name,
                    SessionLaunchConfig::Telnet(config),
                    Some(connection.id),
                    ai_execution_profile,
                    options.custom_name,
                    options.tab_color,
                    options.after_session_id,
                    options.insert_index,
                    options.seed_output,
                    options.startup_command,
                    cx,
                );
            }
            ConnectionType::Ssh {
                ai_execution_profile,
                ..
            } => {
                self.begin_background_saved_ssh_start(
                    connection,
                    ai_execution_profile,
                    options.custom_name,
                    options.tab_color,
                    options.after_session_id,
                    options.insert_index,
                    options.seed_output,
                    options.startup_command,
                    cx,
                );
            }
            ConnectionType::Serial {
                port_name,
                baud_rate,
                data_bits,
                parity,
                stop_bits,
                ai_execution_profile,
                backspace_mode,
            } => {
                let config = SerialSessionConfig {
                    name: connection.name.clone(),
                    port_name,
                    baud_rate,
                    data_bits,
                    parity,
                    stop_bits,
                    backspace_mode,
                };
                self.begin_background_session_start(
                    connection.name,
                    SessionLaunchConfig::Serial(config),
                    Some(connection.id),
                    ai_execution_profile,
                    options.custom_name,
                    options.tab_color,
                    options.after_session_id,
                    options.insert_index,
                    options.seed_output,
                    options.startup_command,
                    cx,
                );
            }
        }
    }

    pub(in crate::features) fn saved_connection_start_is_pending(
        &self,
        connection: &SavedConnection,
    ) -> bool {
        self.pending_session_starts
            .values()
            .any(|pending| pending.source_connection_id.as_deref() == Some(connection.id.as_str()))
    }

    pub(in crate::features) fn saved_connection_start_is_pending_or_queued(
        &self,
        connection: &SavedConnection,
    ) -> bool {
        self.saved_connection_start_is_pending(connection)
            || self
                .pending_saved_connection_queue
                .iter()
                .any(|queued| queued.connection.id == connection.id)
    }

    pub(in crate::features) fn begin_background_saved_ssh_start(
        &mut self,
        connection: SavedConnection,
        ai_execution_profile: AiExecutionProfile,
        custom_name: Option<String>,
        tab_color: Option<u32>,
        after_session_id: Option<String>,
        insert_index: Option<usize>,
        seed_output: Option<String>,
        startup_command: Option<StartupCommandRequest>,
        cx: &mut Context<Self>,
    ) {
        let connection_name = connection.name.clone();
        let source_connection_id = Some(connection.id.clone());
        let geometry_session_hint = after_session_id
            .as_deref()
            .or(self.pending_reconnect_replace_id.as_deref());
        let desired_geometry =
            self.desired_terminal_resize_geometry_for_session_hint(geometry_session_hint);
        let build_context = self.ssh_session_config_build_context();
        let request_id = self.register_pending_session_start(
            PendingSessionStartRegistration {
                connection_name: connection_name.clone(),
                launch_config: None,
                kind: SessionKind::Ssh,
                ai_execution_profile,
                custom_name,
                tab_color,
                after_session_id,
                insert_index,
                seed_output,
                startup_command,
                multiplex_key: None,
                source_connection_id,
                status_message: format!("connecting to {connection_name}"),
                append_start_log: true,
            },
            cx,
        );

        let session_manager = self.session_manager.clone();
        let session_start_tx = self.session_start_tx.clone();
        let request_id_for_worker = request_id.clone();
        std::thread::spawn(move || {
            let worker_started_at = Instant::now();
            let result = (|| {
                let mut config = build_ssh_session_config_with_context(
                    &connection,
                    &mut Vec::new(),
                    &build_context,
                )?;
                config.deferred_pty = true;
                if let Some(geometry) = desired_geometry {
                    config.cols = geometry.cols;
                    config.rows = geometry.rows;
                    config.pixel_width = geometry.pixel_width;
                    config.pixel_height = geometry.pixel_height;
                }
                let session_info = session_manager
                    .create_ssh_session(config.clone())
                    .map_err(|error| error.to_string())?;
                Ok(SessionStartSuccess {
                    session_info,
                    multiplex_handle: None,
                    launch_config: Some(SessionLaunchConfig::Ssh(config)),
                })
            })();
            let worker_finished_at = Instant::now();
            let _ = session_start_tx.send(SessionStartResult {
                request_id: request_id_for_worker,
                connection_name,
                kind: SessionKind::Ssh,
                worker_started_at,
                worker_finished_at,
                result,
            });
        });
    }

    pub(in crate::features) fn ssh_session_config_build_context(
        &self,
    ) -> SshSessionConfigBuildContext {
        SshSessionConfigBuildContext {
            config_dir: self.runtime.config_dir().to_path_buf(),
            portable_key_path: self.runtime.portable_key_path().map(ToOwned::to_owned),
            host_key_policy: self.settings.host_key_policy.clone(),
            x11_display: self.settings.x11_display.clone(),
            keep_alive_interval_secs: self.settings.terminal_keep_alive_interval,
            host_key_prompts: self.host_key_prompts.clone(),
            credential_prompts: self.credential_prompts.clone(),
            otp_provider: self.otp_provider.clone(),
        }
    }

    pub(in crate::features) fn load_ssh_key_auth(
        &self,
        key_id: Option<&str>,
        auth_mode: &str,
    ) -> Result<Option<SshKeyAuthConfig>, String> {
        load_ssh_key_auth_with_context(&self.ssh_session_config_build_context(), key_id, auth_mode)
    }

    pub(in crate::features) fn build_ssh_session_config(
        &self,
        connection: &SavedConnection,
        visited_proxy_jumps: &mut Vec<String>,
    ) -> Result<SshSessionConfig, String> {
        build_ssh_session_config_with_context(
            connection,
            visited_proxy_jumps,
            &self.ssh_session_config_build_context(),
        )
    }

    pub(in crate::features) fn load_proxy_config(
        &self,
        connection: &SavedConnection,
    ) -> Result<Option<SshProxyConfig>, String> {
        load_proxy_config_with_context(&self.ssh_session_config_build_context(), connection)
    }

    pub(in crate::features) fn apply_desired_geometry_to_local_config(
        &self,
        config: &mut LocalSessionConfig,
    ) {
        if let Some(geometry) = self.desired_terminal_resize_geometry() {
            config.cols = geometry.cols;
            config.rows = geometry.rows;
            config.pixel_width = geometry.pixel_width;
            config.pixel_height = geometry.pixel_height;
        }
    }

    pub(in crate::features) fn load_proxy_jump_config(
        &self,
        connection: &SavedConnection,
        visited_proxy_jumps: &mut Vec<String>,
    ) -> Result<Option<Box<SshSessionConfig>>, String> {
        load_proxy_jump_config_with_context(
            &self.ssh_session_config_build_context(),
            connection,
            visited_proxy_jumps,
        )
    }
}

pub(in crate::features) fn build_ssh_session_config_with_context(
    connection: &SavedConnection,
    visited_proxy_jumps: &mut Vec<String>,
    context: &SshSessionConfigBuildContext,
) -> Result<SshSessionConfig, String> {
    let ConnectionType::Ssh {
        host,
        port,
        username,
        backspace_mode,
        ai_execution_profile: _,
        x11_forwarding,
    } = connection.config.clone()
    else {
        return Err("only SSH connections can be used for SSH sessions".to_string());
    };
    let auth = connection.auth.clone().unwrap_or_default();
    let allow_none_auth = auth.mode == "none";
    let password = load_ssh_connection_password_with_context(context, &auth)?;
    let key_auth = load_ssh_key_auth_with_context(context, auth.key_id.as_deref(), &auth.mode)?;
    let proxy_jump = load_proxy_jump_config_with_context(context, connection, visited_proxy_jumps)?;
    let proxy = load_proxy_config_with_context(context, connection)?;

    Ok(SshSessionConfig {
        name: connection.name.clone(),
        host,
        port,
        username,
        password,
        key_auth,
        otp_id: auth.otp_id.filter(|value| !value.trim().is_empty()),
        auto_fill_otp: auth.auto_fill_otp,
        proxy_jump,
        proxy,
        allow_none_auth,
        backspace_mode,
        term: "xterm-256color".to_string(),
        x11_forwarding,
        x11_display: context.x11_display.clone(),
        deferred_pty: true,
        keep_alive_interval_secs: context.keep_alive_interval_secs,
        cols: 80,
        rows: 24,
        pixel_width: 0,
        pixel_height: 0,
        host_key_verifier: Some(Arc::new(NativeHostKeyVerifier {
            config_dir: context.config_dir.clone(),
            portable_key_path: context.portable_key_path.clone(),
            policy: context.host_key_policy.clone(),
            prompt_broker: context.host_key_prompts.clone(),
        })),
        credential_provider: Some(context.credential_prompts.clone()),
        otp_provider: Some(context.otp_provider.clone()),
    })
}

fn load_ssh_connection_password_with_context(
    context: &SshSessionConfigBuildContext,
    auth: &ConnectionAuth,
) -> Result<Option<String>, String> {
    if let Some(password) = auth
        .password
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        if auth.has_password {
            return Err("saved SSH password is locked or could not be decrypted".to_string());
        }
        return Ok(Some(password.to_string()));
    }

    let Some(password_id) = auth
        .password_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };

    let store = ConnectionStore::open_with_portable_key_path(
        &context.config_dir,
        context.portable_key_path.clone(),
    )
    .map_err(|error| error.to_string())?;
    let password = store
        .load_decrypted_password_by_id(password_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("saved password '{password_id}' was not found"))?
        .password
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("saved password '{password_id}' is empty or locked"))?;
    Ok(Some(password))
}

fn load_ssh_key_auth_with_context(
    context: &SshSessionConfigBuildContext,
    key_id: Option<&str>,
    auth_mode: &str,
) -> Result<Option<SshKeyAuthConfig>, String> {
    if auth_mode != "key" {
        return Ok(None);
    }
    let key_id = key_id
        .filter(|key_id| !key_id.trim().is_empty())
        .ok_or_else(|| "connection is set to key auth but has no key_id".to_string())?;
    let store = ConnectionStore::open_with_portable_key_path(
        &context.config_dir,
        context.portable_key_path.clone(),
    )
    .map_err(|error| error.to_string())?;
    let key = store
        .load_decrypted_ssh_key_by_id(key_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("SSH key '{key_id}' was not found"))?;
    let key_data = key
        .key_data
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("SSH key '{}' has no private key data", key.name))?;
    Ok(Some(SshKeyAuthConfig {
        key_data,
        cert_data: key.cert_data.filter(|value| !value.trim().is_empty()),
        passphrase: key.passphrase.filter(|value| !value.trim().is_empty()),
    }))
}

fn load_proxy_config_with_context(
    context: &SshSessionConfigBuildContext,
    connection: &SavedConnection,
) -> Result<Option<SshProxyConfig>, String> {
    let Some(proxy_id) = connection
        .network
        .as_ref()
        .and_then(|network| network.proxy_id.as_deref())
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(None);
    };
    let store = ConnectionStore::open_with_portable_key_path(
        &context.config_dir,
        context.portable_key_path.clone(),
    )
    .map_err(|error| error.to_string())?;
    let proxy = store
        .list_proxies()
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|proxy| proxy.id == proxy_id)
        .ok_or_else(|| format!("Proxy '{proxy_id}' was not found"))?;
    let protocol = match proxy.protocol.as_str() {
        "http" | "proxycommand" => proxy.protocol,
        _ => "socks5".to_string(),
    };
    Ok(Some(SshProxyConfig {
        protocol,
        host: proxy.host,
        port: proxy.port,
        command: proxy.command.filter(|value| !value.trim().is_empty()),
        username: proxy.username.filter(|value| !value.trim().is_empty()),
        password: proxy.password.filter(|value| !value.is_empty()),
    }))
}

fn load_proxy_jump_config_with_context(
    context: &SshSessionConfigBuildContext,
    connection: &SavedConnection,
    visited_proxy_jumps: &mut Vec<String>,
) -> Result<Option<Box<SshSessionConfig>>, String> {
    let Some(proxy_jump_id) = connection
        .network
        .as_ref()
        .and_then(|network| network.proxy_jump_id.as_deref())
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(None);
    };
    if visited_proxy_jumps
        .iter()
        .any(|visited| visited == proxy_jump_id)
    {
        return Err(format!(
            "ProxyJump chain contains a cycle at '{proxy_jump_id}'"
        ));
    }
    visited_proxy_jumps.push(proxy_jump_id.to_string());
    let store = ConnectionStore::open_with_portable_key_path(
        &context.config_dir,
        context.portable_key_path.clone(),
    )
    .map_err(|error| error.to_string())?;
    let jump_connection = store
        .get_connection(proxy_jump_id)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("ProxyJump connection '{proxy_jump_id}' was not found"))?;
    if !matches!(jump_connection.config, ConnectionType::Ssh { .. }) {
        return Err("Only SSH connections can be used as jump hosts".to_string());
    }
    let jump_config =
        build_ssh_session_config_with_context(&jump_connection, visited_proxy_jumps, context)?;
    visited_proxy_jumps.pop();
    Ok(Some(Box::new(jump_config)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nyaterm-desktop-{name}-{}-{}",
            std::process::id(),
            uuid()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn test_ssh_build_context(config_dir: PathBuf) -> SshSessionConfigBuildContext {
        SshSessionConfigBuildContext {
            config_dir: config_dir.clone(),
            portable_key_path: None,
            host_key_policy: "accept".to_string(),
            x11_display: String::new(),
            keep_alive_interval_secs: 30,
            host_key_prompts: Arc::new(HostKeyPromptBroker::default()),
            credential_prompts: Arc::new(CredentialPromptBroker::default()),
            otp_provider: Arc::new(NativeOtpProvider::new(config_dir, None)),
        }
    }

    #[test]
    fn ssh_password_loader_uses_decrypted_inline_password() {
        let dir = unique_temp_dir("ssh-inline-password");
        let context = test_ssh_build_context(dir.clone());
        let auth = ConnectionAuth {
            mode: "password".to_string(),
            password: Some("secret".to_string()),
            has_password: false,
            ..ConnectionAuth::default()
        };

        let password = load_ssh_connection_password_with_context(&context, &auth).unwrap();

        assert_eq!(password.as_deref(), Some("secret"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ssh_password_loader_resolves_saved_password_id() {
        let dir = unique_temp_dir("ssh-password-id");
        let store = ConnectionStore::open(&dir).expect("open store");
        let password_id = store
            .save_password(nyaterm_core::SavedPassword {
                id: "pw-1".to_string(),
                name: "Primary".to_string(),
                password: Some("stored-secret".to_string()),
                has_password: false,
            })
            .expect("save password");
        drop(store);
        let context = test_ssh_build_context(dir.clone());
        let auth = ConnectionAuth {
            mode: "password".to_string(),
            password_id: Some(password_id),
            ..ConnectionAuth::default()
        };

        let password = load_ssh_connection_password_with_context(&context, &auth).unwrap();

        assert_eq!(password.as_deref(), Some("stored-secret"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ssh_password_loader_rejects_locked_inline_password() {
        let dir = unique_temp_dir("ssh-locked-password");
        let context = test_ssh_build_context(dir.clone());
        let auth = ConnectionAuth {
            mode: "password".to_string(),
            password: Some("encrypted".to_string()),
            has_password: true,
            ..ConnectionAuth::default()
        };

        let error = load_ssh_connection_password_with_context(&context, &auth).unwrap_err();

        assert!(error.contains("locked"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ssh_session_config_uses_context_keep_alive_interval() {
        let dir = unique_temp_dir("ssh-keepalive");
        let mut context = test_ssh_build_context(dir.clone());
        context.keep_alive_interval_secs = 45;
        let connection = SavedConnection {
            id: "conn-1".to_string(),
            name: "SSH".to_string(),
            config: ConnectionType::Ssh {
                host: "example.com".to_string(),
                port: 22,
                username: "user".to_string(),
                backspace_mode: "del".to_string(),
                ai_execution_profile: AiExecutionProfile::Posix,
                x11_forwarding: false,
            },
            group_id: None,
            description: None,
            sort_order: 0,
            icon: None,
            auth: Some(ConnectionAuth {
                mode: "none".to_string(),
                ..ConnectionAuth::default()
            }),
            network: None,
            post_login: None,
            created_at_ms: None,
            updated_at_ms: None,
            last_used_at_ms: None,
        };

        let config =
            build_ssh_session_config_with_context(&connection, &mut Vec::new(), &context).unwrap();

        assert_eq!(config.keep_alive_interval_secs, 45);
        let _ = std::fs::remove_dir_all(dir);
    }
}

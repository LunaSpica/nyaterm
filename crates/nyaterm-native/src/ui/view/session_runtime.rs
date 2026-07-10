use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn start_local_session(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_session_name.is_some() {
            self.terminal_status = "wait for the pending session to finish connecting".to_string();
            cx.notify();
            return;
        }

        let config = LocalSessionConfig::default();
        match self.session_manager.create_local_session(config.clone()) {
            Ok(info) => {
                self.register_session(
                    &info.id,
                    SessionRuntimeMetadata {
                        ssh_config: None,
                        ssh_multiplex_key: None,
                        source_connection_id: None,
                        ai_execution_profile: AiExecutionProfile::Posix,
                        launch_config: SessionLaunchConfig::Local(config),
                    },
                );
                self.activate_session_id(&info.id);
                self.terminal_status = format!("running {}", short_id(&info.id));
                self.append_terminal_log(format!("\n# started local PTY {}\n", short_id(&info.id)));
                self.maybe_auto_start_recording(&info.id, &info.name);
                self.ensure_event_pump(window, cx);
            }
            Err(error) => {
                self.terminal_status = format!("failed to start local PTY: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn start_saved_connection(
        &mut self,
        connection: SavedConnection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_session_name.is_some() {
            self.terminal_status = "wait for the pending session to finish connecting".to_string();
            self.selected_nav = NavItem::Workspace;
            cx.notify();
            return;
        }

        match connection.config.clone() {
            ConnectionType::LocalTerminal {
                shell_path,
                shell_args,
                working_dir,
                ai_execution_profile,
            } => {
                let config = LocalSessionConfig {
                    name: connection.name.clone(),
                    shell_path: non_empty_string(shell_path),
                    shell_args: split_shell_args(&shell_args),
                    working_dir: working_dir
                        .filter(|value| !value.trim().is_empty())
                        .map(Into::into),
                    cols: 80,
                    rows: 24,
                };
                match self.session_manager.create_local_session(config.clone()) {
                    Ok(info) => self.activate_started_session(
                        connection.name,
                        info.id,
                        Some(connection.id),
                        ai_execution_profile,
                        SessionLaunchConfig::Local(config),
                        window,
                        cx,
                    ),
                    Err(error) => {
                        self.terminal_status = format!("failed to start local session: {error}");
                        self.selected_nav = NavItem::Workspace;
                        cx.notify();
                    }
                }
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
                match self.session_manager.create_telnet_session(config.clone()) {
                    Ok(info) => self.activate_started_session(
                        connection.name,
                        info.id,
                        Some(connection.id),
                        ai_execution_profile,
                        SessionLaunchConfig::Telnet(config),
                        window,
                        cx,
                    ),
                    Err(error) => {
                        self.terminal_status = format!("failed to start telnet session: {error}");
                        self.selected_nav = NavItem::Workspace;
                        cx.notify();
                    }
                }
            }
            ConnectionType::Ssh {
                ai_execution_profile,
                ..
            } => {
                self.ensure_event_pump(window, cx);
                let config = match self.build_ssh_session_config(&connection, &mut Vec::new()) {
                    Ok(config) => config,
                    Err(error) => {
                        self.terminal_status = format!("failed to prepare SSH session: {error}");
                        self.selected_nav = NavItem::Workspace;
                        cx.notify();
                        return;
                    }
                };
                self.begin_background_ssh_start(
                    connection.name,
                    config,
                    Some(connection.id),
                    ai_execution_profile,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
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
                match self.session_manager.create_serial_session(config.clone()) {
                    Ok(info) => self.activate_started_session(
                        connection.name,
                        info.id,
                        Some(connection.id),
                        ai_execution_profile,
                        SessionLaunchConfig::Serial(config),
                        window,
                        cx,
                    ),
                    Err(error) => {
                        self.terminal_status = format!("failed to start serial session: {error}");
                        self.selected_nav = NavItem::Workspace;
                        cx.notify();
                    }
                }
            }
        }
    }

    fn load_ssh_key_auth(
        &self,
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
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
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

    pub(in crate::ui::view) fn build_ssh_session_config(
        &self,
        connection: &SavedConnection,
        visited_proxy_jumps: &mut Vec<String>,
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
        let password = (!auth.has_password)
            .then_some(auth.password)
            .flatten()
            .filter(|value| !value.trim().is_empty());
        let key_auth = self.load_ssh_key_auth(auth.key_id.as_deref(), &auth.mode)?;
        let proxy_jump = self.load_proxy_jump_config(connection, visited_proxy_jumps)?;
        let proxy = self.load_proxy_config(connection)?;

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
            x11_display: self.settings.x11_display.clone(),
            cols: 80,
            rows: 24,
            host_key_verifier: Some(Arc::new(NativeHostKeyVerifier {
                config_dir: self.runtime.config_dir().to_path_buf(),
                portable_key_path: self.runtime.portable_key_path().map(ToOwned::to_owned),
                policy: self.settings.host_key_policy.clone(),
                prompt_broker: self.host_key_prompts.clone(),
            })),
            credential_provider: Some(self.credential_prompts.clone()),
            otp_provider: Some(self.otp_provider.clone()),
        })
    }

    fn load_proxy_config(
        &self,
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
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
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

    fn load_proxy_jump_config(
        &self,
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
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .map_err(|error| error.to_string())?;
        let jump_connection = store
            .get_connection(proxy_jump_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("ProxyJump connection '{proxy_jump_id}' was not found"))?;
        if !matches!(jump_connection.config, ConnectionType::Ssh { .. }) {
            return Err("Only SSH connections can be used as jump hosts".to_string());
        }
        let jump_config = self.build_ssh_session_config(&jump_connection, visited_proxy_jumps)?;
        visited_proxy_jumps.pop();
        Ok(Some(Box::new(jump_config)))
    }

    fn activate_started_session(
        &mut self,
        name: String,
        session_id: String,
        source_connection_id: Option<String>,
        ai_execution_profile: AiExecutionProfile,
        launch_config: SessionLaunchConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.register_session(
            &session_id,
            SessionRuntimeMetadata {
                ssh_config: None,
                ssh_multiplex_key: None,
                source_connection_id,
                ai_execution_profile,
                launch_config,
            },
        );
        self.activate_session_id(&session_id);
        self.terminal_status = format!("running {}", short_id(&session_id));
        self.append_terminal_log(format!("\n# started {name} ({})\n", short_id(&session_id)));
        self.selected_nav = NavItem::Workspace;
        self.maybe_auto_start_recording(&session_id, &name);
        self.ensure_event_pump(window, cx);
        cx.notify();
    }

    pub(in crate::ui::view) fn begin_background_ssh_start(
        &mut self,
        connection_name: String,
        config: SshSessionConfig,
        source_connection_id: Option<String>,
        ai_execution_profile: AiExecutionProfile,
        custom_name: Option<String>,
        tab_color: Option<u32>,
        after_session_id: Option<String>,
        insert_index: Option<usize>,
        seed_output: Option<String>,
        startup_command: Option<StartupCommandRequest>,
        cx: &mut Context<Self>,
    ) {
        self.pending_session_name = Some(connection_name.clone());
        self.pending_ssh_config = Some(config.clone());
        self.pending_ai_execution_profile = ai_execution_profile;
        self.pending_session_custom_name = custom_name;
        self.pending_session_tab_color = tab_color;
        self.pending_session_after_id = after_session_id;
        self.pending_session_insert_index = insert_index;
        self.pending_terminal_seed_output = seed_output;
        self.pending_startup_command = startup_command;
        self.pending_session_multiplex_key = None;
        self.pending_source_connection_id = source_connection_id;
        self.terminal_status = format!("connecting to {connection_name}");
        if self.active_session_id.is_none() {
            self.append_terminal_log(format!("\n# connecting to {connection_name}\n"));
        }
        self.selected_nav = NavItem::Workspace;

        let session_manager = self.session_manager.clone();
        let session_start_tx = self.session_start_tx.clone();
        std::thread::spawn(move || {
            let result = session_manager
                .create_ssh_session(config)
                .map(|info| SessionStartSuccess {
                    session_id: info.id,
                    multiplex_handle: None,
                })
                .map_err(|error| error.to_string());
            let _ = session_start_tx.send(SessionStartResult {
                connection_name,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn begin_background_multiplex_ssh_start(
        &mut self,
        connection_name: String,
        config: SshSessionConfig,
        source_connection_id: Option<String>,
        ai_execution_profile: AiExecutionProfile,
        custom_name: Option<String>,
        tab_color: Option<u32>,
        after_session_id: Option<String>,
        startup_command: Option<StartupCommandRequest>,
        existing_multiplex: Option<SshMultiplexHandle>,
        cx: &mut Context<Self>,
    ) {
        let multiplex_key = ssh_multiplex_key(&config);
        self.pending_session_name = Some(connection_name.clone());
        self.pending_ssh_config = Some(config.clone());
        self.pending_ai_execution_profile = ai_execution_profile;
        self.pending_session_custom_name = custom_name;
        self.pending_session_tab_color = tab_color;
        self.pending_session_after_id = after_session_id;
        self.pending_session_insert_index = None;
        self.pending_terminal_seed_output = None;
        self.pending_startup_command = startup_command;
        self.pending_session_multiplex_key = Some(multiplex_key.clone());
        self.pending_source_connection_id = source_connection_id;
        self.terminal_status = format!("multiplexing SSH session {connection_name}");
        self.selected_nav = NavItem::Workspace;

        let session_manager = self.session_manager.clone();
        let session_start_tx = self.session_start_tx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let multiplex = match existing_multiplex {
                    Some(handle) if !handle.is_closed() => handle,
                    _ => open_ssh_multiplex_handle(config.clone())
                        .map_err(|error| error.to_string())?,
                };
                let info = session_manager
                    .create_ssh_session_with_multiplex(config, multiplex.clone())
                    .map_err(|error| error.to_string())?;
                Ok(SessionStartSuccess {
                    session_id: info.id,
                    multiplex_handle: Some(multiplex),
                })
            })();
            let _ = session_start_tx.send(SessionStartResult {
                connection_name,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn send_probe_command(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.active_session_id.as_deref() else {
            self.terminal_status = "start a session first".to_string();
            cx.notify();
            return;
        };

        let command = if cfg!(target_os = "windows") {
            "echo nyaterm-native-ready\r\n"
        } else {
            "printf 'nyaterm-native-ready\\n'\n"
        };
        match self.session_manager.write(session_id, command.as_bytes()) {
            Ok(()) => {
                self.terminal_status = "probe command sent".to_string();
            }
            Err(error) => {
                self.terminal_status = format!("write failed: {error}");
            }
        }
        cx.notify();
    }

    pub(super) fn drain_session_start_events(&mut self, cx: &mut Context<Self>) {
        while let Ok(event) = self.session_start_rx.try_recv() {
            self.pending_session_name = None;
            match event.result {
                Ok(success) => {
                    let session_id = success.session_id;
                    let ssh_config = self.pending_ssh_config.take();
                    let launch_config = ssh_config
                        .clone()
                        .map(SessionLaunchConfig::Ssh)
                        .unwrap_or_else(|| SessionLaunchConfig::Ssh(SshSessionConfig::default()));
                    let ssh_multiplex_key = self.pending_session_multiplex_key.take();
                    if let (Some(key), Some(handle)) =
                        (ssh_multiplex_key.clone(), success.multiplex_handle)
                    {
                        self.ssh_multiplex_handles.insert(key, handle);
                    }
                    let source_connection_id = self.pending_source_connection_id.take();
                    self.register_session(
                        &session_id,
                        SessionRuntimeMetadata {
                            ssh_config,
                            ssh_multiplex_key,
                            source_connection_id,
                            ai_execution_profile: self.pending_ai_execution_profile,
                            launch_config,
                        },
                    );
                    if let Some(custom_name) = self.pending_session_custom_name.take() {
                        self.session_custom_names
                            .insert(session_id.clone(), custom_name);
                    }
                    if let Some(tab_color) = self.pending_session_tab_color.take() {
                        self.session_tab_colors
                            .insert(session_id.clone(), tab_color);
                    }
                    if let Some(seed_output) = self.pending_terminal_seed_output.take() {
                        self.terminal_views.insert(
                            session_id.clone(),
                            TerminalViewState::from_output(seed_output),
                        );
                    }
                    if let Some(after_session_id) = self.pending_session_after_id.take() {
                        self.move_session_after(&session_id, &after_session_id);
                    }
                    if let Some(insert_index) = self.pending_session_insert_index.take() {
                        self.move_session_to_index(&session_id, insert_index);
                    }
                    self.pending_ai_execution_profile = AiExecutionProfile::SendOnly;
                    self.activate_session_id(&session_id);
                    self.terminal_status = format!("running {}", short_id(&session_id));
                    self.append_terminal_log(format!(
                        "\n# started {} ({})\n",
                        event.connection_name,
                        short_id(&session_id)
                    ));
                    self.maybe_auto_start_recording(&session_id, &event.connection_name);
                    if let Some(startup_command) = self.pending_startup_command.take() {
                        self.schedule_startup_command(session_id.clone(), startup_command, cx);
                    }
                    self.apply_pending_workspace_split_for_duplicate(&session_id);
                    self.selected_nav = NavItem::Workspace;
                }
                Err(error) => {
                    self.pending_ssh_config = None;
                    self.pending_session_custom_name = None;
                    self.pending_session_tab_color = None;
                    self.pending_session_after_id = None;
                    self.pending_session_insert_index = None;
                    self.pending_terminal_seed_output = None;
                    self.pending_startup_command = None;
                    self.pending_session_multiplex_key = None;
                    self.pending_source_connection_id = None;
                    self.pending_workspace_split = None;
                    self.pending_ai_execution_profile = AiExecutionProfile::SendOnly;
                    self.active_ssh_config = None;
                    self.active_ai_execution_profile = AiExecutionProfile::SendOnly;
                    self.terminal_status =
                        format!("failed to start {}: {error}", event.connection_name);
                    if self.active_session_id.is_none() {
                        self.append_terminal_log(format!(
                            "\n# failed to start {}: {error}\n",
                            event.connection_name
                        ));
                    }
                    self.selected_nav = NavItem::Workspace;
                }
            }
        }
    }
}

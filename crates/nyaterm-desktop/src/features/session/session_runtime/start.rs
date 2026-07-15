use super::*;

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
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_session_name.is_some()
            && !matches!(connection.config, ConnectionType::Ssh { .. })
        {
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
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
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
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    cx,
                );
            }
            ConnectionType::Ssh {
                ai_execution_profile,
                ..
            } => {
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
                self.begin_background_session_start(
                    connection.name,
                    SessionLaunchConfig::Serial(config),
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
        }
    }

    pub(in crate::features) fn load_ssh_key_auth(
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

    pub(in crate::features) fn build_ssh_session_config(
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
            deferred_pty: true,
            cols: 80,
            rows: 24,
            pixel_width: 0,
            pixel_height: 0,
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

    pub(in crate::features) fn load_proxy_config(
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

}

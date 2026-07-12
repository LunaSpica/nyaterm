use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn set_network_tab(&mut self, tab: NetworkTab, cx: &mut Context<Self>) {
        self.network_tab = tab;
        self.network_move_picker = None;
        self.terminal_status = format!("network tab set to {}", tab.label());
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_network_section(
        &mut self,
        tab: NetworkTab,
        section_id: String,
        cx: &mut Context<Self>,
    ) {
        let key = network_section_key(tab, &section_id);
        if self.network_expanded_sections.remove(&key) {
            self.network_move_picker = None;
            self.terminal_status = format!("collapsed {} group", tab.label());
        } else {
            self.network_expanded_sections.insert(key);
            self.terminal_status = format!("expanded {} group", tab.label());
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn open_network_tunnel_editor(
        &mut self,
        tunnel_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tunnel = match tunnel_id.as_deref() {
            Some(id) => self.tunnels.iter().find(|tunnel| tunnel.id == id).cloned(),
            None => Some(TunnelConfig::default()),
        };
        let Some(tunnel) = tunnel else {
            self.terminal_status = "tunnel profile is no longer available".to_string();
            cx.notify();
            return;
        };

        self.network_tunnel_editor = Some(NetworkTunnelEditorState {
            id: tunnel_id,
            is_open: tunnel.is_open,
            name: tunnel.name,
            tunnel_type: match tunnel.tunnel_type.as_str() {
                "remote" | "dynamic" => tunnel.tunnel_type,
                _ => "local".to_string(),
            },
            connection_id: tunnel.connection_id,
            listen_port: if tunnel.listen_port == 0 {
                String::new()
            } else {
                tunnel.listen_port.to_string()
            },
            target_host: if tunnel.target_host.trim().is_empty() {
                "127.0.0.1".to_string()
            } else {
                tunnel.target_host
            },
            target_port: if tunnel.target_port == 0 {
                String::new()
            } else {
                tunnel.target_port.to_string()
            },
            auto_open: tunnel.auto_open,
            bind_localhost: tunnel.bind_localhost,
            group_id: tunnel.group_id,
            focused_field: NetworkTunnelEditorField::Name,
            error: None,
        });
        self.network_tab = NetworkTab::Tunnels;
        self.terminal_status = "tunnel editor opened".to_string();
        window.focus(&self.network_tunnel_editor_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn close_network_tunnel_editor(&mut self, cx: &mut Context<Self>) {
        self.network_tunnel_editor = None;
        self.terminal_status = "tunnel editor closed".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn focus_network_tunnel_editor_field(
        &mut self,
        field: NetworkTunnelEditorField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.network_tunnel_editor.as_mut() {
            editor.focused_field = field;
            editor.error = None;
        }
        window.focus(&self.network_tunnel_editor_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_network_tunnel_editor_key_down(
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
                self.close_network_tunnel_editor(cx);
                return;
            }
            "enter" => {
                self.save_network_tunnel_editor(cx);
                return;
            }
            "tab" if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                if let Some(editor) = self.network_tunnel_editor.as_mut() {
                    editor.focused_field = editor.focused_field.next(editor.is_dynamic());
                    editor.error = None;
                }
                cx.notify();
                return;
            }
            _ => {}
        }

        let Some(editor) = self.network_tunnel_editor.as_mut() else {
            return;
        };
        if keystroke.modifiers.platform || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                match editor.focused_field {
                    NetworkTunnelEditorField::Name => {
                        editor.name.pop();
                    }
                    NetworkTunnelEditorField::ListenPort => {
                        editor.listen_port.pop();
                    }
                    NetworkTunnelEditorField::TargetHost => {
                        editor.target_host.pop();
                    }
                    NetworkTunnelEditorField::TargetPort => {
                        editor.target_port.pop();
                    }
                }
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
                match editor.focused_field {
                    NetworkTunnelEditorField::Name => editor.name.push_str(input),
                    NetworkTunnelEditorField::ListenPort => {
                        editor
                            .listen_port
                            .extend(input.chars().filter(|character| character.is_ascii_digit()));
                    }
                    NetworkTunnelEditorField::TargetHost => editor.target_host.push_str(input),
                    NetworkTunnelEditorField::TargetPort => {
                        editor
                            .target_port
                            .extend(input.chars().filter(|character| character.is_ascii_digit()));
                    }
                }
                editor.error = None;
                cx.notify();
            }
        }
    }

    pub(in crate::ui::view) fn cycle_network_tunnel_type(&mut self, cx: &mut Context<Self>) {
        if let Some(editor) = self.network_tunnel_editor.as_mut() {
            editor.tunnel_type = match editor.tunnel_type.as_str() {
                "local" => "remote",
                "remote" => "dynamic",
                _ => "local",
            }
            .to_string();
            if editor.is_dynamic() {
                editor.focused_field = match editor.focused_field {
                    NetworkTunnelEditorField::TargetHost | NetworkTunnelEditorField::TargetPort => {
                        NetworkTunnelEditorField::ListenPort
                    }
                    field => field,
                };
            }
            editor.error = None;
            self.terminal_status = format!("tunnel type set to {}", editor.tunnel_type);
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn cycle_network_tunnel_connection(&mut self, cx: &mut Context<Self>) {
        let connection_ids = self
            .connections
            .iter()
            .filter(|connection| matches!(&connection.config, ConnectionType::Ssh { .. }))
            .map(|connection| connection.id.as_str())
            .collect::<Vec<_>>();
        let Some(editor) = self.network_tunnel_editor.as_mut() else {
            return;
        };
        editor.connection_id =
            next_network_group_id(editor.connection_id.as_deref(), connection_ids.into_iter());
        editor.error = None;
        self.terminal_status = "tunnel SSH connection changed".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn cycle_network_tunnel_group(&mut self, cx: &mut Context<Self>) {
        if let Some(editor) = self.network_tunnel_editor.as_mut() {
            editor.group_id = next_network_group_id(
                editor.group_id.as_deref(),
                self.tunnel_groups.iter().map(|group| group.id.as_str()),
            );
            editor.error = None;
            self.terminal_status = "tunnel group changed".to_string();
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn set_network_tunnel_bind_localhost(
        &mut self,
        bind_localhost: bool,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.network_tunnel_editor.as_mut() {
            editor.bind_localhost = bind_localhost;
            editor.error = None;
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn toggle_network_tunnel_auto_open(&mut self, cx: &mut Context<Self>) {
        if let Some(editor) = self.network_tunnel_editor.as_mut() {
            editor.auto_open = !editor.auto_open;
            editor.error = None;
            self.terminal_status = if editor.auto_open {
                "tunnel auto-open enabled"
            } else {
                "tunnel auto-open disabled"
            }
            .to_string();
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn save_network_tunnel_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.network_tunnel_editor.clone() else {
            self.terminal_status = "no tunnel editor is active".to_string();
            cx.notify();
            return;
        };

        let name = editor.name.trim().to_string();
        if name.is_empty() {
            self.set_network_tunnel_editor_error("Tunnel name is required", cx);
            return;
        }
        let Some(connection_id) = editor.connection_id.clone() else {
            self.set_network_tunnel_editor_error("SSH connection is required", cx);
            return;
        };
        if !self
            .connections
            .iter()
            .any(|connection| connection.id == connection_id)
        {
            self.set_network_tunnel_editor_error("Selected SSH connection is missing", cx);
            return;
        }
        let Some(listen_port) = parse_port(&editor.listen_port) else {
            self.set_network_tunnel_editor_error("Listen port must be 1-65535", cx);
            return;
        };
        let is_dynamic = editor.is_dynamic();
        let target_host = if is_dynamic {
            "127.0.0.1".to_string()
        } else {
            let value = editor.target_host.trim().to_string();
            if value.is_empty() {
                self.set_network_tunnel_editor_error("Target host is required", cx);
                return;
            }
            value
        };
        let target_port = if is_dynamic {
            0
        } else {
            let Some(port) = parse_port(&editor.target_port) else {
                self.set_network_tunnel_editor_error("Target port must be 1-65535", cx);
                return;
            };
            port
        };
        let group_id = editor.group_id.filter(|id| {
            self.tunnel_groups
                .iter()
                .any(|group| group.id.as_str() == id.as_str())
        });

        let id = editor.id.clone().unwrap_or_else(uuid);
        let mut next_tunnels = self.tunnels.clone();
        let tunnel = TunnelConfig {
            id: id.clone(),
            name: name.clone(),
            tunnel_type: editor.tunnel_type,
            connection_id: Some(connection_id),
            listen_port,
            target_host,
            target_port,
            is_open: editor.is_open,
            auto_open: editor.auto_open,
            bind_localhost: editor.bind_localhost,
            group_id,
        };
        if let Some(existing) = next_tunnels.iter_mut().find(|tunnel| tunnel.id == id) {
            *existing = tunnel;
        } else {
            next_tunnels.push(tunnel);
        }

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.replace_tunnels(&next_tunnels))
        {
            Ok(()) => {
                self.tunnels = next_tunnels;
                self.network_tunnel_editor = None;
                self.terminal_status = format!("tunnel '{name}' saved");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.terminal_status = format!("failed to save tunnel: {error}");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
                if let Some(editor) = self.network_tunnel_editor.as_mut() {
                    editor.error = Some(self.terminal_status.clone());
                }
            }
        }
        cx.notify();
    }

    fn set_network_tunnel_editor_error(
        &mut self,
        error: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        let error = error.into();
        if let Some(editor) = self.network_tunnel_editor.as_mut() {
            editor.error = Some(error.clone());
        }
        self.terminal_status = error;
        cx.notify();
    }

    pub(in crate::ui::view) fn open_network_proxy_editor(
        &mut self,
        proxy_id: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let proxy = match proxy_id.as_deref() {
            Some(id) => self.proxies.iter().find(|proxy| proxy.id == id).cloned(),
            None => Some(ProxyConfig::default()),
        };
        let Some(proxy) = proxy else {
            self.terminal_status = "proxy profile is no longer available".to_string();
            cx.notify();
            return;
        };

        self.network_proxy_editor = Some(NetworkProxyEditorState {
            id: proxy_id,
            name: proxy.name,
            protocol: match proxy.protocol.as_str() {
                "http" | "proxycommand" => proxy.protocol,
                _ => "socks5".to_string(),
            },
            host: if proxy.host.trim().is_empty() {
                "127.0.0.1".to_string()
            } else {
                proxy.host
            },
            port: if proxy.port == 0 {
                String::new()
            } else {
                proxy.port.to_string()
            },
            command: proxy.command.unwrap_or_default(),
            username: proxy.username.unwrap_or_default(),
            password: String::new(),
            existing_password: proxy.password,
            password_id: proxy.password_id,
            group_id: proxy.group_id,
            focused_field: NetworkProxyEditorField::Name,
            error: None,
        });
        self.network_tab = NetworkTab::Proxies;
        self.terminal_status = "proxy editor opened".to_string();
        window.focus(&self.network_proxy_editor_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn close_network_proxy_editor(&mut self, cx: &mut Context<Self>) {
        self.network_proxy_editor = None;
        self.terminal_status = "proxy editor closed".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn focus_network_proxy_editor_field(
        &mut self,
        field: NetworkProxyEditorField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(editor) = self.network_proxy_editor.as_mut() {
            editor.focused_field = field;
            editor.error = None;
        }
        window.focus(&self.network_proxy_editor_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_network_proxy_editor_key_down(
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
                self.close_network_proxy_editor(cx);
                return;
            }
            "enter" => {
                if keystroke.modifiers.shift
                    && let Some(editor) = self.network_proxy_editor.as_mut()
                    && editor.focused_field == NetworkProxyEditorField::Command
                {
                    editor.command.push('\n');
                    editor.error = None;
                    cx.notify();
                    return;
                }
                self.save_network_proxy_editor(cx);
                return;
            }
            "tab" if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                if let Some(editor) = self.network_proxy_editor.as_mut() {
                    editor.focused_field = editor.focused_field.next(editor.is_proxy_command());
                    editor.error = None;
                }
                cx.notify();
                return;
            }
            _ => {}
        }

        let Some(editor) = self.network_proxy_editor.as_mut() else {
            return;
        };
        if keystroke.modifiers.platform || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                match editor.focused_field {
                    NetworkProxyEditorField::Name => {
                        editor.name.pop();
                    }
                    NetworkProxyEditorField::Host => {
                        editor.host.pop();
                    }
                    NetworkProxyEditorField::Port => {
                        editor.port.pop();
                    }
                    NetworkProxyEditorField::Command => {
                        editor.command.pop();
                    }
                    NetworkProxyEditorField::Username => {
                        editor.username.pop();
                    }
                    NetworkProxyEditorField::Password => {
                        editor.password.pop();
                    }
                }
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
                match editor.focused_field {
                    NetworkProxyEditorField::Name => editor.name.push_str(input),
                    NetworkProxyEditorField::Host => editor.host.push_str(input),
                    NetworkProxyEditorField::Port => {
                        editor
                            .port
                            .extend(input.chars().filter(|character| character.is_ascii_digit()));
                    }
                    NetworkProxyEditorField::Command => editor.command.push_str(input),
                    NetworkProxyEditorField::Username => editor.username.push_str(input),
                    NetworkProxyEditorField::Password => editor.password.push_str(input),
                }
                editor.error = None;
                cx.notify();
            }
        }
    }

    pub(in crate::ui::view) fn cycle_network_proxy_protocol(&mut self, cx: &mut Context<Self>) {
        if let Some(editor) = self.network_proxy_editor.as_mut() {
            editor.protocol = match editor.protocol.as_str() {
                "socks5" => "http",
                "http" => "proxycommand",
                _ => "socks5",
            }
            .to_string();
            if editor.is_proxy_command() {
                editor.focused_field = match editor.focused_field {
                    NetworkProxyEditorField::Host
                    | NetworkProxyEditorField::Port
                    | NetworkProxyEditorField::Username
                    | NetworkProxyEditorField::Password => NetworkProxyEditorField::Command,
                    field => field,
                };
            } else if editor.focused_field == NetworkProxyEditorField::Command {
                editor.focused_field = NetworkProxyEditorField::Host;
            }
            editor.error = None;
            self.terminal_status = format!("proxy protocol set to {}", editor.protocol);
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn cycle_network_proxy_group(&mut self, cx: &mut Context<Self>) {
        if let Some(editor) = self.network_proxy_editor.as_mut() {
            editor.group_id = next_network_group_id(
                editor.group_id.as_deref(),
                self.proxy_groups.iter().map(|group| group.id.as_str()),
            );
            editor.error = None;
            self.terminal_status = "proxy group changed".to_string();
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn save_network_proxy_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.network_proxy_editor.clone() else {
            self.terminal_status = "no proxy editor is active".to_string();
            cx.notify();
            return;
        };

        let name = editor.name.trim().to_string();
        if name.is_empty() {
            self.set_network_proxy_editor_error("Proxy name is required", cx);
            return;
        }
        let is_command = editor.is_proxy_command();
        let command = if is_command {
            let command = editor.command.trim().to_string();
            if command.is_empty() {
                self.set_network_proxy_editor_error("ProxyCommand is required", cx);
                return;
            }
            Some(command)
        } else {
            None
        };
        let host = if is_command {
            editor.host.trim().to_string()
        } else {
            let host = editor.host.trim().to_string();
            if host.is_empty() {
                self.set_network_proxy_editor_error("Proxy host is required", cx);
                return;
            }
            host
        };
        let port = if is_command {
            editor.port.trim().parse::<u16>().unwrap_or(0)
        } else {
            let Some(port) = parse_port(&editor.port) else {
                self.set_network_proxy_editor_error("Proxy port must be 1-65535", cx);
                return;
            };
            port
        };
        let username = if editor.username.trim().is_empty() {
            None
        } else {
            Some(editor.username.trim().to_string())
        };
        let password = if editor.password.is_empty() {
            editor.existing_password
        } else {
            Some(editor.password)
        };
        let password_id = if password.is_some() {
            None
        } else {
            editor.password_id
        };
        let group_id = editor.group_id.filter(|id| {
            self.proxy_groups
                .iter()
                .any(|group| group.id.as_str() == id.as_str())
        });

        let id = editor.id.clone().unwrap_or_else(uuid);
        let proxy = ProxyConfig {
            id: id.clone(),
            name: name.clone(),
            protocol: editor.protocol,
            host,
            port,
            command,
            username,
            password,
            password_id,
            group_id,
        };
        let mut next_proxies = self.proxies.clone();
        if let Some(existing) = next_proxies.iter_mut().find(|proxy| proxy.id == id) {
            *existing = proxy;
        } else {
            next_proxies.push(proxy);
        }

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.replace_proxies(&next_proxies))
        {
            Ok(()) => {
                self.proxies = next_proxies;
                self.network_proxy_editor = None;
                self.terminal_status = format!("proxy '{name}' saved");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.terminal_status = format!("failed to save proxy: {error}");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
                if let Some(editor) = self.network_proxy_editor.as_mut() {
                    editor.error = Some(self.terminal_status.clone());
                }
            }
        }
        cx.notify();
    }

    fn set_network_proxy_editor_error(&mut self, error: impl Into<String>, cx: &mut Context<Self>) {
        let error = error.into();
        if let Some(editor) = self.network_proxy_editor.as_mut() {
            editor.error = Some(error.clone());
        }
        self.terminal_status = error;
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_tunnel_search_key_down(
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
                self.tunnel_search_draft.clear();
                self.terminal_status = "tunnel search cleared".to_string();
                cx.notify();
            }
            "backspace" if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                self.tunnel_search_draft.pop();
                cx.notify();
            }
            _ if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.tunnel_search_draft.push_str(input);
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    pub(in crate::ui::view) fn handle_proxy_search_key_down(
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
                self.proxy_search_draft.clear();
                self.terminal_status = "proxy search cleared".to_string();
                cx.notify();
            }
            "backspace" if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                self.proxy_search_draft.pop();
                cx.notify();
            }
            _ if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.proxy_search_draft.push_str(input);
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    pub(in crate::ui::view) fn open_network_group_editor(
        &mut self,
        tab: NetworkTab,
        group_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let name = match (tab, group_id.as_deref()) {
            (NetworkTab::Tunnels, Some(id)) => self
                .tunnel_groups
                .iter()
                .find(|group| group.id == id)
                .map(|group| group.name.clone()),
            (NetworkTab::Proxies, Some(id)) => self
                .proxy_groups
                .iter()
                .find(|group| group.id == id)
                .map(|group| group.name.clone()),
            (_, None) => Some(String::new()),
        };
        let Some(name) = name else {
            self.terminal_status = "network group is no longer available".to_string();
            cx.notify();
            return;
        };

        self.network_group_editor = Some(NetworkGroupEditorState {
            tab,
            id: group_id,
            name,
            error: None,
        });
        self.terminal_status = "network group editor opened".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn close_network_group_editor(&mut self, cx: &mut Context<Self>) {
        self.network_group_editor = None;
        self.terminal_status = "network group editor closed".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn handle_network_group_editor_key_down(
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
            "escape" => self.close_network_group_editor(cx),
            "enter" => self.save_network_group_editor(cx),
            "backspace" if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                if let Some(editor) = self.network_group_editor.as_mut() {
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
                    if let Some(editor) = self.network_group_editor.as_mut() {
                        editor.name.push_str(input);
                        editor.error = None;
                    }
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    pub(in crate::ui::view) fn save_network_group_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.network_group_editor.clone() else {
            self.terminal_status = "no network group editor is active".to_string();
            cx.notify();
            return;
        };
        let name = editor.name.trim().to_string();
        if name.is_empty() {
            if let Some(editor) = self.network_group_editor.as_mut() {
                editor.error = Some("Group name is required".to_string());
            }
            cx.notify();
            return;
        }

        match editor.tab {
            NetworkTab::Tunnels => self.save_tunnel_group(editor.id, name, cx),
            NetworkTab::Proxies => self.save_proxy_group(editor.id, name, cx),
        }
    }

    fn save_tunnel_group(
        &mut self,
        group_id: Option<String>,
        name: String,
        cx: &mut Context<Self>,
    ) {
        let mut groups = self.tunnel_groups.clone();
        if let Some(id) = group_id {
            let Some(group) = groups.iter_mut().find(|group| group.id == id) else {
                self.terminal_status = "tunnel group is no longer available".to_string();
                cx.notify();
                return;
            };
            group.name = name.clone();
        } else {
            groups.push(TunnelGroup {
                id: uuid(),
                name: name.clone(),
                sort_order: groups.len() as u32,
            });
        }

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.replace_tunnel_groups(&groups))
        {
            Ok(()) => {
                self.tunnel_groups = groups;
                self.network_group_editor = None;
                self.terminal_status = format!("tunnel group '{name}' saved");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.terminal_status = format!("failed to save tunnel group: {error}");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    fn save_proxy_group(&mut self, group_id: Option<String>, name: String, cx: &mut Context<Self>) {
        let mut groups = self.proxy_groups.clone();
        if let Some(id) = group_id {
            let Some(group) = groups.iter_mut().find(|group| group.id == id) else {
                self.terminal_status = "proxy group is no longer available".to_string();
                cx.notify();
                return;
            };
            group.name = name.clone();
        } else {
            groups.push(ProxyGroup {
                id: uuid(),
                name: name.clone(),
                sort_order: groups.len() as u32,
            });
        }

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.replace_proxy_groups(&groups))
        {
            Ok(()) => {
                self.proxy_groups = groups;
                self.network_group_editor = None;
                self.terminal_status = format!("proxy group '{name}' saved");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.terminal_status = format!("failed to save proxy group: {error}");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn open_network_group_delete_confirm(
        &mut self,
        tab: NetworkTab,
        id: String,
        label: String,
        item_count: usize,
        cx: &mut Context<Self>,
    ) {
        self.network_group_delete_confirm = Some(NetworkGroupDeleteConfirmState {
            tab,
            id,
            label,
            item_count,
        });
        self.terminal_status = "network group delete confirmation opened".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn cancel_network_group_delete(&mut self, cx: &mut Context<Self>) {
        self.network_group_delete_confirm = None;
        self.terminal_status = "network group delete cancelled".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn confirm_network_group_delete(&mut self, cx: &mut Context<Self>) {
        let Some(delete) = self.network_group_delete_confirm.clone() else {
            self.terminal_status = "no network group delete is pending".to_string();
            cx.notify();
            return;
        };

        match delete.tab {
            NetworkTab::Tunnels => self.delete_tunnel_group(delete.id, delete.label, cx),
            NetworkTab::Proxies => self.delete_proxy_group(delete.id, delete.label, cx),
        }
    }

    fn delete_tunnel_group(&mut self, group_id: String, label: String, cx: &mut Context<Self>) {
        let groups = self
            .tunnel_groups
            .iter()
            .filter(|group| group.id != group_id)
            .cloned()
            .collect::<Vec<_>>();
        let tunnels = self
            .tunnels
            .iter()
            .filter(|tunnel| tunnel.group_id.as_deref() != Some(group_id.as_str()))
            .cloned()
            .collect::<Vec<_>>();

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| {
            store.replace_tunnel_groups(&groups)?;
            store.replace_tunnels(&tunnels)
        }) {
            Ok(()) => {
                self.tunnel_groups = groups;
                self.tunnels = tunnels;
                self.network_group_delete_confirm = None;
                self.terminal_status = format!("tunnel group '{label}' deleted");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.terminal_status = format!("failed to delete tunnel group: {error}");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    fn delete_proxy_group(&mut self, group_id: String, label: String, cx: &mut Context<Self>) {
        let groups = self
            .proxy_groups
            .iter()
            .filter(|group| group.id != group_id)
            .cloned()
            .collect::<Vec<_>>();
        let proxies = self
            .proxies
            .iter()
            .filter(|proxy| proxy.group_id.as_deref() != Some(group_id.as_str()))
            .cloned()
            .collect::<Vec<_>>();

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| {
            store.replace_proxy_groups(&groups)?;
            store.replace_proxies(&proxies)
        }) {
            Ok(()) => {
                self.proxy_groups = groups;
                self.proxies = proxies;
                self.network_group_delete_confirm = None;
                self.terminal_status = format!("proxy group '{label}' deleted");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.terminal_status = format!("failed to delete proxy group: {error}");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn open_network_move_picker(
        &mut self,
        tab: NetworkTab,
        id: String,
        cx: &mut Context<Self>,
    ) {
        if self
            .network_move_picker
            .as_ref()
            .is_some_and(|picker| picker.tab == tab && picker.id == id)
        {
            self.network_move_picker = None;
            self.terminal_status = "network move menu closed".to_string();
        } else {
            self.network_move_picker = Some(NetworkMovePickerState { tab, id });
            self.terminal_status = format!("choose {} group", tab.label());
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn move_tunnel_to_group(
        &mut self,
        tunnel_id: String,
        group_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let group_id = group_id.filter(|id| {
            self.tunnel_groups
                .iter()
                .any(|group| group.id.as_str() == id.as_str())
        });
        let label = network_group_label(group_id.as_deref(), &self.tunnel_groups);
        self.move_tunnel_to_group_internal(tunnel_id, group_id, label, cx);
    }

    fn move_tunnel_to_group_internal(
        &mut self,
        tunnel_id: String,
        group_id: Option<String>,
        label: String,
        cx: &mut Context<Self>,
    ) {
        let mut next_tunnels = self.tunnels.clone();
        let Some(tunnel) = next_tunnels
            .iter_mut()
            .find(|tunnel| tunnel.id == tunnel_id)
        else {
            self.terminal_status = "tunnel profile is no longer available".to_string();
            cx.notify();
            return;
        };
        tunnel.group_id = group_id;

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.replace_tunnels(&next_tunnels))
        {
            Ok(()) => {
                self.tunnels = next_tunnels;
                self.network_move_picker = None;
                self.terminal_status = format!("tunnel moved to {label}");
                self.store_status.message = "tunnel group saved".to_string();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.terminal_status = format!("failed to move tunnel: {error}");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn move_proxy_to_group(
        &mut self,
        proxy_id: String,
        group_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let group_id = group_id.filter(|id| {
            self.proxy_groups
                .iter()
                .any(|group| group.id.as_str() == id.as_str())
        });
        let label = network_group_label(group_id.as_deref(), &self.proxy_groups);
        self.move_proxy_to_group_internal(proxy_id, group_id, label, cx);
    }

    fn move_proxy_to_group_internal(
        &mut self,
        proxy_id: String,
        group_id: Option<String>,
        label: String,
        cx: &mut Context<Self>,
    ) {
        let mut next_proxies = self.proxies.clone();
        let Some(proxy) = next_proxies.iter_mut().find(|proxy| proxy.id == proxy_id) else {
            self.terminal_status = "proxy profile is no longer available".to_string();
            cx.notify();
            return;
        };
        proxy.group_id = group_id;

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.replace_proxies(&next_proxies))
        {
            Ok(()) => {
                self.proxies = next_proxies;
                self.network_move_picker = None;
                self.terminal_status = format!("proxy moved to {label}");
                self.store_status.message = "proxy group saved".to_string();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.terminal_status = format!("failed to move proxy: {error}");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn open_network_delete_confirm(
        &mut self,
        tab: NetworkTab,
        id: String,
        label: String,
        cx: &mut Context<Self>,
    ) {
        self.network_delete_confirm = Some(NetworkDeleteConfirmState { tab, id, label });
        self.terminal_status = "network delete confirmation opened".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn cancel_network_delete(&mut self, cx: &mut Context<Self>) {
        self.network_delete_confirm = None;
        self.terminal_status = "network delete cancelled".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn confirm_network_delete(&mut self, cx: &mut Context<Self>) {
        let Some(delete) = self.network_delete_confirm.clone() else {
            self.terminal_status = "no network delete is pending".to_string();
            cx.notify();
            return;
        };

        match delete.tab {
            NetworkTab::Tunnels => self.delete_tunnel_profile(delete.id, delete.label, cx),
            NetworkTab::Proxies => self.delete_proxy_profile(delete.id, delete.label, cx),
        }
    }

    fn delete_tunnel_profile(&mut self, tunnel_id: String, label: String, cx: &mut Context<Self>) {
        if self.tunnel_manager.is_open(&tunnel_id).unwrap_or(false) {
            if let Err(error) = self.tunnel_manager.close(&tunnel_id) {
                self.terminal_status = format!("failed to close tunnel before delete: {error}");
                cx.notify();
                return;
            }
        }

        let mut next_tunnels = self.tunnels.clone();
        let before = next_tunnels.len();
        next_tunnels.retain(|tunnel| tunnel.id != tunnel_id);
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.replace_tunnels(&next_tunnels))
        {
            Ok(()) => {
                let deleted = next_tunnels.len() != before;
                self.tunnels = next_tunnels;
                self.pending_tunnels.retain(|id| id != &tunnel_id);
                self.network_delete_confirm = None;
                self.terminal_status = if deleted {
                    format!("tunnel '{label}' deleted")
                } else {
                    format!("tunnel '{label}' was already deleted")
                };
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = deleted;
            }
            Err(error) => {
                self.terminal_status = format!("failed to delete tunnel: {error}");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    fn delete_proxy_profile(&mut self, proxy_id: String, label: String, cx: &mut Context<Self>) {
        let mut next_proxies = self.proxies.clone();
        let before = next_proxies.len();
        next_proxies.retain(|proxy| proxy.id != proxy_id);
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.replace_proxies(&next_proxies))
        {
            Ok(()) => {
                let deleted = next_proxies.len() != before;
                self.proxies = next_proxies;
                self.network_delete_confirm = None;
                self.terminal_status = if deleted {
                    format!("proxy '{label}' deleted")
                } else {
                    format!("proxy '{label}' was already deleted")
                };
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = deleted;
            }
            Err(error) => {
                self.terminal_status = format!("failed to delete proxy: {error}");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn start_tunnel_job(
        &mut self,
        tunnel: TunnelConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_tunnels.iter().any(|id| id == &tunnel.id) {
            self.terminal_status = format!("tunnel {} is already pending", tunnel_name(&tunnel));
            cx.notify();
            return;
        }
        if self.tunnel_manager.is_open(&tunnel.id).unwrap_or(false) {
            self.terminal_status = format!("tunnel {} is already open", tunnel_name(&tunnel));
            cx.notify();
            return;
        }

        let Some(connection_id) = tunnel.connection_id.as_deref() else {
            self.terminal_status = format!("tunnel {} has no SSH connection", tunnel_name(&tunnel));
            cx.notify();
            return;
        };
        let Some(connection) = self
            .connections
            .iter()
            .find(|connection| connection.id == connection_id)
            .cloned()
        else {
            self.terminal_status = format!(
                "tunnel {} references missing connection {}",
                tunnel_name(&tunnel),
                connection_id
            );
            cx.notify();
            return;
        };
        let mode = match tunnel_mode(&tunnel) {
            Some(mode) => mode,
            None => {
                self.terminal_status = format!(
                    "tunnel {} mode '{}' is not native yet",
                    tunnel_name(&tunnel),
                    tunnel.tunnel_type
                );
                cx.notify();
                return;
            }
        };
        let ssh_config = match self.build_ssh_session_config(&connection, &mut Vec::new()) {
            Ok(config) => config,
            Err(error) => {
                self.terminal_status =
                    format!("failed to prepare tunnel {}: {error}", tunnel_name(&tunnel));
                cx.notify();
                return;
            }
        };
        let config = SshTunnelConfig {
            id: tunnel.id.clone(),
            ssh_config,
            mode,
            bind_host: if tunnel.bind_localhost {
                "127.0.0.1".to_string()
            } else {
                "0.0.0.0".to_string()
            },
            listen_port: tunnel.listen_port,
            target_host: matches!(mode, SshTunnelMode::Local | SshTunnelMode::Remote)
                .then_some(tunnel.target_host.clone()),
            target_port: matches!(mode, SshTunnelMode::Local | SshTunnelMode::Remote)
                .then_some(tunnel.target_port),
        };

        self.ensure_event_pump(window, cx);
        self.pending_tunnels.push(tunnel.id.clone());
        self.terminal_status = format!("opening tunnel {}", tunnel_name(&tunnel));
        let tunnel_manager = self.tunnel_manager.clone();
        let tunnel_tx = self.tunnel_tx.clone();
        std::thread::spawn(move || {
            let result = tunnel_manager
                .open(config)
                .map(TunnelJobOutput::Opened)
                .map_err(|error| error.to_string());
            let _ = tunnel_tx.send(TunnelJobResult {
                tunnel_id: tunnel.id,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::ui::view) fn close_tunnel_job(
        &mut self,
        tunnel_id: String,
        cx: &mut Context<Self>,
    ) {
        if self.pending_tunnels.iter().any(|id| id == &tunnel_id) {
            self.terminal_status = format!("tunnel {tunnel_id} is already pending");
            cx.notify();
            return;
        }
        if !self.tunnel_manager.is_open(&tunnel_id).unwrap_or(false) {
            self.terminal_status = format!("tunnel {tunnel_id} is not open");
            cx.notify();
            return;
        }

        self.pending_tunnels.push(tunnel_id.clone());
        self.terminal_status = format!("closing tunnel {tunnel_id}");
        let tunnel_manager = self.tunnel_manager.clone();
        let tunnel_tx = self.tunnel_tx.clone();
        std::thread::spawn(move || {
            let result = tunnel_manager
                .close(&tunnel_id)
                .map(|_| TunnelJobOutput::Closed)
                .map_err(|error| error.to_string());
            let _ = tunnel_tx.send(TunnelJobResult { tunnel_id, result });
        });
        cx.notify();
    }

    pub(super) fn drain_tunnel_events(&mut self) -> bool {
        let mut dirty = false;
        while let Ok(event) = self.tunnel_rx.try_recv() {
            dirty = true;
            self.pending_tunnels.retain(|id| id != &event.tunnel_id);
            match event.result {
                Ok(TunnelJobOutput::Opened(info)) => {
                    self.terminal_status = format!(
                        "tunnel {} open on {}:{}",
                        event.tunnel_id, info.bind_host, info.listen_port
                    );
                }
                Ok(TunnelJobOutput::Closed) => {
                    self.terminal_status = format!("tunnel {} closed", event.tunnel_id);
                }
                Err(error) => {
                    self.terminal_status = format!("tunnel {} failed: {error}", event.tunnel_id);
                }
            }
        }
        dirty
    }
}

fn next_network_group_id<'a>(
    current_group_id: Option<&str>,
    group_ids: impl Iterator<Item = &'a str>,
) -> Option<String> {
    let mut cycle = std::iter::once(None)
        .chain(group_ids.map(Some))
        .collect::<Vec<_>>();
    if cycle.is_empty() {
        return None;
    }
    let current_index = cycle
        .iter()
        .position(|group_id| *group_id == current_group_id)
        .unwrap_or(0);
    cycle
        .remove((current_index + 1) % cycle.len())
        .map(ToOwned::to_owned)
}

fn network_group_label<T>(group_id: Option<&str>, groups: &[T]) -> String
where
    T: NetworkGroupLike,
{
    group_id
        .and_then(|id| groups.iter().find(|group| group.network_group_id() == id))
        .map(|group| group.network_group_name().to_string())
        .unwrap_or_else(|| "Ungrouped".to_string())
}

fn network_section_key(tab: NetworkTab, section_id: &str) -> String {
    match tab {
        NetworkTab::Tunnels => format!("tunnel:{section_id}"),
        NetworkTab::Proxies => format!("proxy:{section_id}"),
    }
}

fn parse_port(value: &str) -> Option<u16> {
    let port = value.trim().parse::<u16>().ok()?;
    (port > 0).then_some(port)
}

trait NetworkGroupLike {
    fn network_group_id(&self) -> &str;
    fn network_group_name(&self) -> &str;
}

impl NetworkGroupLike for TunnelGroup {
    fn network_group_id(&self) -> &str {
        &self.id
    }

    fn network_group_name(&self) -> &str {
        &self.name
    }
}

impl NetworkGroupLike for ProxyGroup {
    fn network_group_id(&self) -> &str {
        &self.id
    }

    fn network_group_name(&self) -> &str {
        &self.name
    }
}

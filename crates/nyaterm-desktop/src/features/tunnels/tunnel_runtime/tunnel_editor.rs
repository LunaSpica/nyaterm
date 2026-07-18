use super::*;

impl NyaTermApp {
    pub(in crate::features) fn set_network_tab(&mut self, tab: NetworkTab, cx: &mut Context<Self>) {
        self.network_tab = tab;
        self.network_item_menu = None;
        self.network_move_picker = None;
        self.terminal_status = format!("network tab set to {}", tab.label());
        cx.notify();
    }

    pub(in crate::features) fn toggle_network_section(
        &mut self,
        tab: NetworkTab,
        section_id: String,
        cx: &mut Context<Self>,
    ) {
        self.network_item_menu = None;
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

    pub(in crate::features) fn open_network_tunnel_editor(
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
        self.network_item_menu = None;
        self.network_tab = NetworkTab::Tunnels;
        self.terminal_status = "tunnel editor opened".to_string();
        window.focus(&self.network_tunnel_editor_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_network_tunnel_editor(&mut self, cx: &mut Context<Self>) {
        self.network_tunnel_editor = None;
        self.terminal_status = "tunnel editor closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn focus_network_tunnel_editor_field(
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

    pub(in crate::features) fn handle_network_tunnel_editor_key_down(
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

    pub(in crate::features) fn cycle_network_tunnel_type(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn cycle_network_tunnel_connection(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn cycle_network_tunnel_group(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn set_network_tunnel_bind_localhost(
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

    pub(in crate::features) fn toggle_network_tunnel_auto_open(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn save_network_tunnel_editor(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn set_network_tunnel_editor_error(
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
}

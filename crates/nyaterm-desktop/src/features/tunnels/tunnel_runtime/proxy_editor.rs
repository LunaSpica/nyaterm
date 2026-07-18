use super::*;

impl NyaTermApp {
    pub(in crate::features) fn open_network_proxy_editor(
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
        self.network_item_menu = None;
        self.network_tab = NetworkTab::Proxies;
        self.terminal_status = "proxy editor opened".to_string();
        window.focus(&self.network_proxy_editor_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_network_proxy_editor(&mut self, cx: &mut Context<Self>) {
        self.network_proxy_editor = None;
        self.terminal_status = "proxy editor closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn focus_network_proxy_editor_field(
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

    pub(in crate::features) fn handle_network_proxy_editor_key_down(
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

    pub(in crate::features) fn cycle_network_proxy_protocol(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn cycle_network_proxy_group(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn save_network_proxy_editor(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn set_network_proxy_editor_error(
        &mut self,
        error: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        let error = error.into();
        if let Some(editor) = self.network_proxy_editor.as_mut() {
            editor.error = Some(error.clone());
        }
        self.terminal_status = error;
        cx.notify();
    }
}

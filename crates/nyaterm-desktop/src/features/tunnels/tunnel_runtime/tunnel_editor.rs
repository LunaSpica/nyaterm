use gpui::{Context, Window};
use nyaterm_core::{ConnectionStore, TunnelConfig, uuid};

use super::helpers::{network_section_key, parse_port};
use crate::features::NyaTermApp;
use crate::models::{NetworkTab, NetworkTunnelEditorField, NetworkTunnelEditorState};

impl NyaTermApp {
    pub(in crate::features) fn set_network_tab(&mut self, tab: NetworkTab, cx: &mut Context<Self>) {
        self.connection_state.set_network_tab(tab);
        self.shell.status = format!("network tab set to {}", tab.label());
        cx.notify();
    }

    pub(in crate::features) fn toggle_network_section(
        &mut self,
        tab: NetworkTab,
        section_id: String,
        cx: &mut Context<Self>,
    ) {
        let key = network_section_key(tab, &section_id);
        if self.connection_state.toggle_network_section(key) {
            self.shell.status = format!("expanded {} group", tab.label());
        } else {
            self.shell.status = format!("collapsed {} group", tab.label());
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
            Some(id) => self
                .tunnel_state
                .tunnels()
                .iter()
                .find(|tunnel| tunnel.id == id)
                .cloned(),
            None => Some(TunnelConfig::default()),
        };
        let Some(tunnel) = tunnel else {
            self.shell.status = "tunnel profile is no longer available".to_string();
            cx.notify();
            return;
        };

        self.connection_state
            .begin_network_tunnel_edit(NetworkTunnelEditorState {
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
        // The dialog's boxes own their text, so they have to be dropped for the
        // next tunnel to seed from its own values.
        self.forget_text_inputs("network.tunnel-editor.");
        self.shell.status = "tunnel editor opened".to_string();
        let tunnel_editor_focus = self.connection_state.network_tunnel_editor_focus_handle();
        window.focus(&tunnel_editor_focus);
        cx.notify();
    }

    pub(in crate::features) fn close_network_tunnel_editor(&mut self, cx: &mut Context<Self>) {
        self.connection_state.close_network_tunnel_editor();
        self.forget_text_inputs("network.tunnel-editor.");
        self.shell.status = "tunnel editor closed".to_string();
        cx.notify();
    }

    /// Apply an edit from one of the tunnel dialog's inputs.
    pub(in crate::features) fn apply_network_tunnel_editor_input(
        &mut self,
        field: &str,
        text: String,
        cx: &mut Context<Self>,
    ) {
        let field = match field {
            "name" => NetworkTunnelEditorField::Name,
            "listen-port" => NetworkTunnelEditorField::ListenPort,
            "target-host" => NetworkTunnelEditorField::TargetHost,
            "target-port" => NetworkTunnelEditorField::TargetPort,
            _ => return,
        };
        if self
            .connection_state
            .set_network_tunnel_editor_field(field, text)
        {
            cx.notify();
        }
    }

    pub(in crate::features) fn cycle_network_tunnel_type(&mut self, cx: &mut Context<Self>) {
        if let Some(tunnel_type) = self.connection_state.cycle_network_tunnel_type() {
            self.shell.status = format!("tunnel type set to {tunnel_type}");
        }
        cx.notify();
    }

    pub(in crate::features) fn cycle_network_tunnel_connection(&mut self, cx: &mut Context<Self>) {
        if self.connection_state.cycle_network_tunnel_connection() {
            self.shell.status = "tunnel SSH connection changed".to_string();
            cx.notify();
        }
    }

    pub(in crate::features) fn cycle_network_tunnel_group(&mut self, cx: &mut Context<Self>) {
        if self.connection_state.cycle_network_tunnel_group(
            self.tunnel_state
                .tunnel_groups()
                .iter()
                .map(|group| group.id.as_str()),
        ) {
            self.shell.status = "tunnel group changed".to_string();
        }
        cx.notify();
    }

    pub(in crate::features) fn set_network_tunnel_bind_localhost(
        &mut self,
        bind_localhost: bool,
        cx: &mut Context<Self>,
    ) {
        self.connection_state
            .set_network_tunnel_bind_localhost(bind_localhost);
        cx.notify();
    }

    pub(in crate::features) fn toggle_network_tunnel_auto_open(&mut self, cx: &mut Context<Self>) {
        if let Some(auto_open) = self.connection_state.toggle_network_tunnel_auto_open() {
            self.shell.status = if auto_open {
                "tunnel auto-open enabled"
            } else {
                "tunnel auto-open disabled"
            }
            .to_string();
        }
        cx.notify();
    }

    pub(in crate::features) fn save_network_tunnel_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.connection_state.active_network_tunnel_editor() else {
            self.shell.status = "no tunnel editor is active".to_string();
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
            .connection_state
            .connections()
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
        let group_id = editor
            .group_id
            .filter(|id| self.tunnel_state.has_tunnel_group(id));

        let id = editor.id.clone().unwrap_or_else(uuid);
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
        let next_tunnels = self.tunnel_state.tunnels_with_upsert(tunnel);

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.replace_tunnels(&next_tunnels))
        {
            Ok(()) => {
                self.tunnel_state.commit_tunnels(next_tunnels);
                self.connection_state.close_network_tunnel_editor();
                self.shell.status = format!("tunnel '{name}' saved");
                self.settings
                    .update_store_status(self.shell.status.clone(), true);
            }
            Err(error) => {
                self.shell.status = format!("failed to save tunnel: {error}");
                self.settings
                    .update_store_status(self.shell.status.clone(), false);
                self.connection_state
                    .set_network_tunnel_editor_error(self.shell.status.clone());
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
        self.connection_state
            .set_network_tunnel_editor_error(error.clone());
        self.shell.status = error;
        cx.notify();
    }
}

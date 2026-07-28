use gpui::{Context, Window};
use nyaterm_core::{ConnectionStore, TunnelConfig};
use nyaterm_transport::{SshTunnelConfig, SshTunnelMode};

use super::helpers::network_group_label;
use crate::features::{NyaTermApp, TunnelJobOutput, TunnelJobResult, tunnel_mode, tunnel_name};
use crate::models::{NetworkDeleteConfirmState, NetworkTab};

const TUNNEL_EVENT_DRAIN_LIMIT: usize = 32;

impl NyaTermApp {
    pub(in crate::features) fn toggle_network_item_menu(
        &mut self,
        tab: NetworkTab,
        id: String,
        cx: &mut Context<Self>,
    ) {
        self.connection_state.toggle_network_item_menu(tab, id);
        cx.notify();
    }

    pub(in crate::features) fn open_network_move_picker(
        &mut self,
        tab: NetworkTab,
        id: String,
        cx: &mut Context<Self>,
    ) {
        if self.connection_state.toggle_network_move_picker(tab, id) {
            self.terminal.view.status = format!("choose {} group", tab.label());
        } else {
            self.terminal.view.status = "network move menu closed".to_string();
        }
        cx.notify();
    }

    pub(in crate::features) fn move_tunnel_to_group(
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

    pub(in crate::features) fn move_tunnel_to_group_internal(
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
            self.terminal.view.status = "tunnel profile is no longer available".to_string();
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
                self.connection_state.close_network_move_picker();
                self.terminal.view.status = format!("tunnel moved to {label}");
                self.store_status.message = "tunnel group saved".to_string();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.terminal.view.status = format!("failed to move tunnel: {error}");
                self.store_status.message = self.terminal.view.status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn move_proxy_to_group(
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

    pub(in crate::features) fn move_proxy_to_group_internal(
        &mut self,
        proxy_id: String,
        group_id: Option<String>,
        label: String,
        cx: &mut Context<Self>,
    ) {
        let mut next_proxies = self.proxies.clone();
        let Some(proxy) = next_proxies.iter_mut().find(|proxy| proxy.id == proxy_id) else {
            self.terminal.view.status = "proxy profile is no longer available".to_string();
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
                self.connection_state.close_network_move_picker();
                self.terminal.view.status = format!("proxy moved to {label}");
                self.store_status.message = "proxy group saved".to_string();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.terminal.view.status = format!("failed to move proxy: {error}");
                self.store_status.message = self.terminal.view.status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn open_network_delete_confirm(
        &mut self,
        tab: NetworkTab,
        id: String,
        label: String,
        cx: &mut Context<Self>,
    ) {
        self.connection_state
            .open_network_delete_confirm(NetworkDeleteConfirmState { tab, id, label });
        self.terminal.view.status = "network delete confirmation opened".to_string();
        cx.notify();
    }

    pub(in crate::features) fn cancel_network_delete(&mut self, cx: &mut Context<Self>) {
        self.connection_state.close_network_delete_confirm();
        self.terminal.view.status = "network delete cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn confirm_network_delete(&mut self, cx: &mut Context<Self>) {
        let Some(delete) = self.connection_state.active_network_delete_confirm() else {
            self.terminal.view.status = "no network delete is pending".to_string();
            cx.notify();
            return;
        };

        match delete.tab {
            NetworkTab::Tunnels => self.delete_tunnel_profile(delete.id, delete.label, cx),
            NetworkTab::Proxies => self.delete_proxy_profile(delete.id, delete.label, cx),
        }
    }

    pub(in crate::features) fn delete_tunnel_profile(
        &mut self,
        tunnel_id: String,
        label: String,
        cx: &mut Context<Self>,
    ) {
        if self
            .tunnel_runtime
            .manager
            .is_open(&tunnel_id)
            .unwrap_or(false)
        {
            if let Err(error) = self.tunnel_runtime.manager.close(&tunnel_id) {
                self.terminal.view.status =
                    format!("failed to close tunnel before delete: {error}");
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
                self.tunnel_runtime.finish(&tunnel_id);
                self.connection_state
                    .remove_network_item_references(NetworkTab::Tunnels, &tunnel_id);
                self.terminal.view.status = if deleted {
                    format!("tunnel '{label}' deleted")
                } else {
                    format!("tunnel '{label}' was already deleted")
                };
                self.store_status.message = self.terminal.view.status.clone();
                self.store_status.ready = deleted;
            }
            Err(error) => {
                self.terminal.view.status = format!("failed to delete tunnel: {error}");
                self.store_status.message = self.terminal.view.status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn delete_proxy_profile(
        &mut self,
        proxy_id: String,
        label: String,
        cx: &mut Context<Self>,
    ) {
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
                self.connection_state
                    .remove_network_item_references(NetworkTab::Proxies, &proxy_id);
                self.terminal.view.status = if deleted {
                    format!("proxy '{label}' deleted")
                } else {
                    format!("proxy '{label}' was already deleted")
                };
                self.store_status.message = self.terminal.view.status.clone();
                self.store_status.ready = deleted;
            }
            Err(error) => {
                self.terminal.view.status = format!("failed to delete proxy: {error}");
                self.store_status.message = self.terminal.view.status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn start_tunnel_job(
        &mut self,
        tunnel: TunnelConfig,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.tunnel_runtime.is_pending(&tunnel.id) {
            self.terminal.view.status =
                format!("tunnel {} is already pending", tunnel_name(&tunnel));
            cx.notify();
            return;
        }
        if self
            .tunnel_runtime
            .manager
            .is_open(&tunnel.id)
            .unwrap_or(false)
        {
            self.terminal.view.status = format!("tunnel {} is already open", tunnel_name(&tunnel));
            cx.notify();
            return;
        }

        let Some(connection_id) = tunnel.connection_id.as_deref() else {
            self.terminal.view.status =
                format!("tunnel {} has no SSH connection", tunnel_name(&tunnel));
            cx.notify();
            return;
        };
        let Some(connection) = self
            .connections
            .iter()
            .find(|connection| connection.id == connection_id)
            .cloned()
        else {
            self.terminal.view.status = format!(
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
                self.terminal.view.status = format!(
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
                self.terminal.view.status =
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

        self.tunnel_runtime.mark_pending(tunnel.id.clone());
        self.terminal.view.status = format!("opening tunnel {}", tunnel_name(&tunnel));
        let tunnel_manager = self.tunnel_runtime.manager.clone();
        let tunnel_tx = self.tunnel_runtime.sender();
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

    pub(in crate::features) fn close_tunnel_job(
        &mut self,
        tunnel_id: String,
        cx: &mut Context<Self>,
    ) {
        if self.tunnel_runtime.is_pending(&tunnel_id) {
            self.terminal.view.status = format!("tunnel {tunnel_id} is already pending");
            cx.notify();
            return;
        }
        if !self
            .tunnel_runtime
            .manager
            .is_open(&tunnel_id)
            .unwrap_or(false)
        {
            self.terminal.view.status = format!("tunnel {tunnel_id} is not open");
            cx.notify();
            return;
        }

        self.tunnel_runtime.mark_pending(tunnel_id.clone());
        self.terminal.view.status = format!("closing tunnel {tunnel_id}");
        let tunnel_manager = self.tunnel_runtime.manager.clone();
        let tunnel_tx = self.tunnel_runtime.sender();
        std::thread::spawn(move || {
            let result = tunnel_manager
                .close(&tunnel_id)
                .map(|_| TunnelJobOutput::Closed)
                .map_err(|error| error.to_string());
            let _ = tunnel_tx.send(TunnelJobResult { tunnel_id, result });
        });
        cx.notify();
    }

    pub(in crate::features) fn drain_tunnel_events(&mut self) -> bool {
        if !self.tunnel_runtime.has_pending() {
            return false;
        }
        let mut dirty = false;
        for _ in 0..TUNNEL_EVENT_DRAIN_LIMIT {
            let Ok(event) = self.tunnel_runtime.try_recv() else {
                break;
            };
            dirty = true;
            self.tunnel_runtime.finish(&event.tunnel_id);
            match event.result {
                Ok(TunnelJobOutput::Opened(info)) => {
                    self.terminal.view.status = format!(
                        "tunnel {} open on {}:{}",
                        event.tunnel_id, info.bind_host, info.listen_port
                    );
                }
                Ok(TunnelJobOutput::Closed) => {
                    self.terminal.view.status = format!("tunnel {} closed", event.tunnel_id);
                }
                Err(error) => {
                    self.terminal.view.status =
                        format!("tunnel {} failed: {error}", event.tunnel_id);
                }
            }
        }
        dirty
    }
}

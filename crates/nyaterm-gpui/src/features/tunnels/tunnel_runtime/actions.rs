use super::*;

impl NyaTermApp {
    pub(in crate::features) fn open_network_move_picker(
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

    pub(in crate::features) fn open_network_delete_confirm(
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

    pub(in crate::features) fn cancel_network_delete(&mut self, cx: &mut Context<Self>) {
        self.network_delete_confirm = None;
        self.terminal_status = "network delete cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn confirm_network_delete(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn delete_tunnel_profile(&mut self, tunnel_id: String, label: String, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn delete_proxy_profile(&mut self, proxy_id: String, label: String, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn start_tunnel_job(
        &mut self,
        tunnel: TunnelConfig,
        _window: &mut Window,
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

    pub(in crate::features) fn close_tunnel_job(
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

    pub(in crate::features) fn drain_tunnel_events(&mut self) -> bool {
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

use super::*;

impl NyaTermApp {
    pub(in crate::features) fn open_network_group_editor(
        &mut self,
        tab: NetworkTab,
        group_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.network_item_menu = None;
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

    pub(in crate::features) fn close_network_group_editor(&mut self, cx: &mut Context<Self>) {
        self.network_group_editor = None;
        self.terminal_status = "network group editor closed".to_string();
        cx.notify();
    }

    pub(in crate::features) fn handle_network_group_editor_key_down(
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

    pub(in crate::features) fn save_network_group_editor(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn save_tunnel_group(
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

    pub(in crate::features) fn save_proxy_group(
        &mut self,
        group_id: Option<String>,
        name: String,
        cx: &mut Context<Self>,
    ) {
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

    pub(in crate::features) fn open_network_group_delete_confirm(
        &mut self,
        tab: NetworkTab,
        id: String,
        label: String,
        item_count: usize,
        cx: &mut Context<Self>,
    ) {
        self.network_item_menu = None;
        self.network_group_delete_confirm = Some(NetworkGroupDeleteConfirmState {
            tab,
            id,
            label,
            item_count,
        });
        self.terminal_status = "network group delete confirmation opened".to_string();
        cx.notify();
    }

    pub(in crate::features) fn cancel_network_group_delete(&mut self, cx: &mut Context<Self>) {
        self.network_group_delete_confirm = None;
        self.terminal_status = "network group delete cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn confirm_network_group_delete(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::features) fn delete_tunnel_group(
        &mut self,
        group_id: String,
        label: String,
        cx: &mut Context<Self>,
    ) {
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

    pub(in crate::features) fn delete_proxy_group(
        &mut self,
        group_id: String,
        label: String,
        cx: &mut Context<Self>,
    ) {
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
}

use gpui::Context;
use nyaterm_core::ConnectionStore;

use crate::features::NyaTermApp;
use crate::models::{NetworkGroupDeleteConfirmState, NetworkGroupEditorState, NetworkTab};

impl NyaTermApp {
    pub(in crate::features) fn open_network_group_editor(
        &mut self,
        tab: NetworkTab,
        group_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let name = match (tab, group_id.as_deref()) {
            (NetworkTab::Tunnels, Some(id)) => self
                .tunnel_state
                .tunnel_groups()
                .iter()
                .find(|group| group.id == id)
                .map(|group| group.name.clone()),
            (NetworkTab::Proxies, Some(id)) => self
                .tunnel_state
                .proxy_groups()
                .iter()
                .find(|group| group.id == id)
                .map(|group| group.name.clone()),
            (_, None) => Some(String::new()),
        };
        let Some(name) = name else {
            self.terminal.view.status = "network group is no longer available".to_string();
            cx.notify();
            return;
        };

        self.connection_state
            .begin_network_group_edit(NetworkGroupEditorState {
                tab,
                id: group_id,
                name,
                error: None,
            });
        // The box owns its text, so it has to be dropped for the next group to
        // seed from its own name.
        self.forget_text_inputs("network.group-editor.");
        self.terminal.view.status = "network group editor opened".to_string();
        cx.notify();
    }

    pub(in crate::features) fn close_network_group_editor(&mut self, cx: &mut Context<Self>) {
        self.connection_state.close_network_group_editor();
        self.forget_text_inputs("network.group-editor.");
        self.terminal.view.status = "network group editor closed".to_string();
        cx.notify();
    }

    /// Apply an edit from the group dialog's name box.
    pub(in crate::features) fn apply_network_group_editor_name(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        if self.connection_state.set_network_group_editor_name(text) {
            cx.notify();
        }
    }

    pub(in crate::features) fn save_network_group_editor(&mut self, cx: &mut Context<Self>) {
        let Some(editor) = self.connection_state.active_network_group_editor() else {
            self.terminal.view.status = "no network group editor is active".to_string();
            cx.notify();
            return;
        };
        let name = editor.name.trim().to_string();
        if name.is_empty() {
            self.connection_state
                .set_network_group_editor_error("Group name is required".to_string());
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
        let Some(groups) = self
            .tunnel_state
            .tunnel_groups_with_upsert(group_id.as_deref(), name.clone())
        else {
            self.terminal.view.status = "tunnel group is no longer available".to_string();
            cx.notify();
            return;
        };

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.replace_tunnel_groups(&groups))
        {
            Ok(()) => {
                self.tunnel_state.commit_tunnel_groups(groups);
                self.connection_state.close_network_group_editor();
                self.terminal.view.status = format!("tunnel group '{name}' saved");
                self.settings
                    .set_store_message(self.terminal.view.status.clone());
                self.settings.set_store_ready(true);
            }
            Err(error) => {
                self.terminal.view.status = format!("failed to save tunnel group: {error}");
                self.settings
                    .set_store_message(self.terminal.view.status.clone());
                self.settings.set_store_ready(false);
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
        let Some(groups) = self
            .tunnel_state
            .proxy_groups_with_upsert(group_id.as_deref(), name.clone())
        else {
            self.terminal.view.status = "proxy group is no longer available".to_string();
            cx.notify();
            return;
        };

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.replace_proxy_groups(&groups))
        {
            Ok(()) => {
                self.tunnel_state.commit_proxy_groups(groups);
                self.connection_state.close_network_group_editor();
                self.terminal.view.status = format!("proxy group '{name}' saved");
                self.settings
                    .set_store_message(self.terminal.view.status.clone());
                self.settings.set_store_ready(true);
            }
            Err(error) => {
                self.terminal.view.status = format!("failed to save proxy group: {error}");
                self.settings
                    .set_store_message(self.terminal.view.status.clone());
                self.settings.set_store_ready(false);
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
        self.connection_state
            .open_network_group_delete_confirm(NetworkGroupDeleteConfirmState {
                tab,
                id,
                label,
                item_count,
            });
        self.terminal.view.status = "network group delete confirmation opened".to_string();
        cx.notify();
    }

    pub(in crate::features) fn cancel_network_group_delete(&mut self, cx: &mut Context<Self>) {
        self.connection_state.close_network_group_delete_confirm();
        self.terminal.view.status = "network group delete cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn confirm_network_group_delete(&mut self, cx: &mut Context<Self>) {
        let Some(delete) = self.connection_state.active_network_group_delete_confirm() else {
            self.terminal.view.status = "no network group delete is pending".to_string();
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
        let removal = self.tunnel_state.without_tunnel_group(&group_id);

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| {
            store.replace_tunnel_groups(removal.groups())?;
            store.replace_tunnels(removal.tunnels())
        }) {
            Ok(()) => {
                let deleted_tunnel_ids = self.tunnel_state.commit_tunnel_group_removal(removal);
                self.connection_state.remove_network_group_references(
                    NetworkTab::Tunnels,
                    &group_id,
                    &deleted_tunnel_ids,
                );
                self.terminal.view.status = format!("tunnel group '{label}' deleted");
                self.settings
                    .set_store_message(self.terminal.view.status.clone());
                self.settings.set_store_ready(true);
            }
            Err(error) => {
                self.terminal.view.status = format!("failed to delete tunnel group: {error}");
                self.settings
                    .set_store_message(self.terminal.view.status.clone());
                self.settings.set_store_ready(false);
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
        let removal = self.tunnel_state.without_proxy_group(&group_id);

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| {
            store.replace_proxy_groups(removal.groups())?;
            store.replace_proxies(removal.proxies())
        }) {
            Ok(()) => {
                let deleted_proxy_ids = self.tunnel_state.commit_proxy_group_removal(removal);
                self.connection_state.remove_network_group_references(
                    NetworkTab::Proxies,
                    &group_id,
                    &deleted_proxy_ids,
                );
                self.terminal.view.status = format!("proxy group '{label}' deleted");
                self.settings
                    .set_store_message(self.terminal.view.status.clone());
                self.settings.set_store_ready(true);
            }
            Err(error) => {
                self.terminal.view.status = format!("failed to delete proxy group: {error}");
                self.settings
                    .set_store_message(self.terminal.view.status.clone());
                self.settings.set_store_ready(false);
            }
        }
        cx.notify();
    }
}

use super::*;
use nyaterm_domain::RestorableOpenTab;

impl NyaTermApp {
    pub(in crate::ui::view) fn persist_open_tabs(&mut self) {
        if !self.settings.startup_restore {
            return;
        }
        let tabs = self.serialize_open_tabs();
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_open_tabs(&tabs))
        {
            Ok(()) => {}
            Err(error) => {
                self.terminal_status = format!("failed to save open tabs: {error}");
            }
        }
        // Keep multi-leaf layout indexes aligned with the same ordered tab list.
        self.persist_terminal_window_layout();
    }

    pub(in crate::ui::view) fn serialize_open_tabs(&self) -> Vec<RestorableOpenTab> {
        self.ordered_sessions()
            .into_iter()
            .map(|session| {
                let metadata = self.session_metadata.get(&session.id);
                let connection_id = metadata.and_then(|meta| meta.source_connection_id.clone());
                let session_type = match metadata.map(|meta| &meta.launch_config) {
                    Some(SessionLaunchConfig::Ssh(_)) => "SSH",
                    Some(SessionLaunchConfig::Telnet(_)) => "Telnet",
                    Some(SessionLaunchConfig::Serial(_)) => "Serial",
                    Some(SessionLaunchConfig::Local(_)) | None => "Local",
                }
                .to_string();
                let custom_name = self.session_custom_names.get(&session.id).cloned();
                let tab_color = self
                    .session_tab_colors
                    .get(&session.id)
                    .map(|color| format!("#{color:06x}"));
                let title = self.session_display_name_by_info(&session);
                RestorableOpenTab {
                    title,
                    session_type,
                    connection_id,
                    custom_name,
                    tab_color,
                }
            })
            .collect()
    }

    pub(in crate::ui::view) fn try_restore_open_tabs(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.open_tabs_restored {
            return;
        }
        self.open_tabs_restored = true;
        if !self.settings.startup_restore {
            self.startup_restore_complete = true;
            return;
        }
        if !self.ordered_sessions().is_empty() {
            self.startup_restore_complete = true;
            return;
        }
        let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) else {
            self.startup_restore_complete = true;
            return;
        };
        let Ok(tabs) = store.load_open_tabs() else {
            self.startup_restore_complete = true;
            return;
        };
        if tabs.is_empty() {
            self.startup_restore_complete = true;
            return;
        }
        self.startup_restore_queue = tabs;
        self.terminal_status = format!(
            "restoring {} workspace tab(s)...",
            self.startup_restore_queue.len()
        );
        self.pump_startup_restore_queue(window, cx);
    }

    pub(in crate::ui::view) fn pump_startup_restore_queue(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.startup_restore_complete {
            return;
        }
        if self.pending_session_name.is_some() {
            return;
        }
        let Some(tab) = self.startup_restore_queue.first().cloned() else {
            self.finish_startup_restore(cx);
            return;
        };

        let started = self.start_restorable_open_tab(&tab, window, cx);
        self.startup_restore_queue.remove(0);
        if !started {
            // Keep draining sync failures until pending async work or queue empty.
            self.pump_startup_restore_queue(window, cx);
        }
    }

    fn finish_startup_restore(&mut self, cx: &mut Context<Self>) {
        if self.startup_restore_complete {
            return;
        }
        self.startup_restore_complete = true;
        // After all tabs reconnect, attempt multi-leaf then global pane layout restore.
        self.terminal_windows_restored = false;
        self.workspace_pane_layout_restored = false;
        self.try_restore_terminal_window_layout();
        self.try_restore_workspace_pane_layout();
        if self.terminal_windows_is_multi_leaf() {
            self.terminal_status = "restored workspace tabs and window layout".to_string();
        } else if self.workspace_split.as_ref().is_some_and(|root| root.is_split()) {
            self.terminal_status = "restored workspace tabs and pane layout".to_string();
        } else if !self.ordered_sessions().is_empty() {
            self.terminal_status = "restored workspace tabs".to_string();
        }
        if !self.ordered_sessions().is_empty() {
            self.persist_open_tabs();
        }
        cx.notify();
    }

    fn start_restorable_open_tab(
        &mut self,
        tab: &RestorableOpenTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let custom_name = tab
            .custom_name
            .clone()
            .filter(|value| !value.trim().is_empty());
        let tab_color = parse_restorable_tab_color(tab.tab_color.as_deref());
        let session_type = tab.session_type.to_ascii_lowercase();

        if let Some(connection_id) = tab.connection_id.as_ref().filter(|id| !id.is_empty()) {
            let connection = self
                .connections
                .iter()
                .find(|connection| &connection.id == connection_id)
                .cloned();
            let Some(connection) = connection else {
                self.terminal_status =
                    format!("restore skipped missing connection {connection_id}");
                return false;
            };
            match connection.config.clone() {
                ConnectionType::Ssh {
                    ai_execution_profile,
                    ..
                } => {
                    self.ensure_event_pump(window, cx);
                    let config = match self.build_ssh_session_config(&connection, &mut Vec::new()) {
                        Ok(config) => config,
                        Err(error) => {
                            self.terminal_status =
                                format!("restore SSH prepare failed: {error}");
                            return false;
                        }
                    };
                    self.begin_background_ssh_start(
                        connection.name,
                        config,
                        Some(connection.id),
                        ai_execution_profile,
                        custom_name,
                        tab_color,
                        None,
                        None,
                        None,
                        None,
                        cx,
                    );
                    return true;
                }
                _ => {
                    self.start_saved_connection(connection, window, cx);
                    if let Some(session_id) = self.active_session_id.clone() {
                        if let Some(name) = custom_name {
                            self.session_custom_names.insert(session_id.clone(), name);
                        }
                        if let Some(color) = tab_color {
                            self.session_tab_colors.insert(session_id, color);
                        }
                    }
                    return true;
                }
            }
        }

        if session_type == "local" || session_type.is_empty() {
            self.start_local_session(window, cx);
            if let Some(session_id) = self.active_session_id.clone() {
                if let Some(name) = custom_name {
                    self.session_custom_names.insert(session_id.clone(), name);
                }
                if let Some(color) = tab_color {
                    self.session_tab_colors.insert(session_id, color);
                }
            }
            return true;
        }

        self.terminal_status = format!(
            "restore skipped unsupported tab {} ({})",
            tab.title, tab.session_type
        );
        false
    }
}

fn parse_restorable_tab_color(value: Option<&str>) -> Option<u32> {
    let raw = value?.trim().trim_start_matches('#');
    if raw.len() != 6 {
        return None;
    }
    u32::from_str_radix(raw, 16).ok()
}

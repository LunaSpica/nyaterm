use super::*;

impl NyaTermApp {
    pub(in crate::features::pages::transfers) fn select_transfer_browser_entry(
        &mut self,
        path: String,
        cx: &mut Context<Self>,
    ) {
        self.transfer.browser.selected_remote_path = Some(path.clone());
        self.transfer.browser.selected_remote_paths.clear();
        self.transfer
            .browser
            .selected_remote_paths
            .insert(path.clone());
        self.transfer.paths.remote = path.clone();
        self.transfer.panel.focused_field = TransferInputField::Remote;
        self.terminal.view.status = format!("selected remote {path}");
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn select_transfer_browser_entry_from_click(
        &mut self,
        path: String,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.transfer.browser.focus);
        let modifiers = event.modifiers();
        if event.click_count() >= 2 && !modifiers.modified() {
            self.cancel_transfer_browser_pending_rename(cx);
            let entry = self
                .transfer
                .browser
                .entries
                .iter()
                .find(|entry| entry.path == path)
                .cloned();
            self.select_transfer_browser_entry(path, cx);
            if let Some(entry) = entry {
                if entry.file_type == SftpFileType::Directory {
                    self.open_transfer_browser_directory(entry.path, window, cx);
                } else {
                    self.open_transfer_default(entry, window, cx);
                }
            }
            return;
        }
    }

    pub(in crate::features::pages::transfers) fn handle_transfer_browser_entry_mouse_down(
        &mut self,
        path: String,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.transfer.browser.focus);
        self.transfer.panel.focused_field = TransferInputField::Remote;
        if self
            .transfer
            .browser
            .pending_rename
            .as_ref()
            .is_some_and(|pending| pending.path != path)
        {
            self.cancel_transfer_browser_pending_rename(cx);
        }

        let additive = event.modifiers.platform || event.modifiers.control;
        let range_anchor = event
            .modifiers
            .shift
            .then(|| self.transfer.browser.selected_remote_path.clone())
            .flatten()
            .or_else(|| {
                event
                    .modifiers
                    .shift
                    .then(|| {
                        self.transfer
                            .browser
                            .selected_remote_paths
                            .iter()
                            .next()
                            .cloned()
                    })
                    .flatten()
            });
        let anchor_path = range_anchor.clone().unwrap_or_else(|| path.clone());
        let base_selection = if additive {
            self.transfer.browser.selected_remote_paths.clone()
        } else {
            HashSet::new()
        };

        if let Some(anchor) = range_anchor {
            self.apply_transfer_browser_range(
                anchor,
                path.clone(),
                base_selection.clone(),
                additive,
                cx,
            );
        } else if additive {
            self.toggle_transfer_browser_entry_marked(path.clone(), cx);
        } else {
            self.select_transfer_browser_entry(path.clone(), cx);
        }

        self.transfer.browser.drag_selection = Some(TransferBrowserDragSelectionState {
            anchor_path,
            base_selection,
            additive,
        });
    }

    pub(in crate::features::pages::transfers) fn schedule_transfer_browser_name_rename(
        &mut self,
        path: String,
        was_single_selected_on_mouse_down: bool,
        event: &ClickEvent,
        cx: &mut Context<Self>,
    ) {
        let modifiers = event.modifiers();
        if !was_single_selected_on_mouse_down
            || event.click_count() != 1
            || modifiers.modified()
            || self.transfer.file_ops.rename.is_some()
        {
            if event.click_count() >= 2 || modifiers.modified() {
                self.cancel_transfer_browser_pending_rename(cx);
            }
            return;
        }

        if self.transfer.browser.selected_remote_path.as_deref() != Some(path.as_str())
            || self.transfer.browser.selected_remote_paths.len() != 1
            || !self.transfer.browser.selected_remote_paths.contains(&path)
        {
            return;
        }

        self.transfer.browser.pending_rename_token =
            self.transfer.browser.pending_rename_token.wrapping_add(1);
        let token = self.transfer.browser.pending_rename_token;
        self.transfer.browser.pending_rename = Some(TransferBrowserPendingRenameState {
            path: path.clone(),
            token,
        });
        cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(220)).await;
            let _ = this.update(cx, |this, cx| {
                let should_rename = this
                    .transfer
                    .browser
                    .pending_rename
                    .as_ref()
                    .is_some_and(|pending| pending.path == path && pending.token == token)
                    && this.transfer.browser.selected_remote_path.as_deref() == Some(path.as_str())
                    && this.transfer.browser.selected_remote_paths.len() == 1
                    && this.transfer.browser.selected_remote_paths.contains(&path)
                    && this.transfer.file_ops.rename.is_none();

                this.transfer.browser.pending_rename = None;
                if should_rename {
                    this.open_transfer_rename_for_path_after_delay(path, cx);
                } else {
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn cancel_transfer_browser_pending_rename(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if self.cancel_transfer_browser_pending_rename_without_notify() {
            cx.notify();
        }
    }

    pub(in crate::features) fn cancel_transfer_browser_pending_rename_without_notify(
        &mut self,
    ) -> bool {
        let cancelled = self.transfer.browser.pending_rename.take().is_some();
        if cancelled {
            self.transfer.browser.pending_rename_token =
                self.transfer.browser.pending_rename_token.wrapping_add(1);
        }
        cancelled
    }

    pub(in crate::features::pages::transfers) fn handle_transfer_browser_entry_mouse_move(
        &mut self,
        path: String,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        if !event.dragging() {
            self.transfer.browser.drag_selection = None;
            return;
        }

        let Some(drag_selection) = self.transfer.browser.drag_selection.clone() else {
            return;
        };

        self.apply_transfer_browser_range(
            drag_selection.anchor_path,
            path,
            drag_selection.base_selection,
            drag_selection.additive,
            cx,
        );
    }

    pub(in crate::features::pages::transfers) fn finish_transfer_browser_selection_drag(
        &mut self,
        _event: &MouseUpEvent,
        cx: &mut Context<Self>,
    ) {
        if self.transfer.browser.drag_selection.take().is_some() {
            cx.notify();
        }
    }

    pub(in crate::features::pages::transfers) fn select_transfer_browser_entry_from_context(
        &mut self,
        path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.transfer.browser.focus);
        self.transfer.browser.drag_selection = None;
        self.transfer.panel.focused_field = TransferInputField::Remote;
        if self.transfer.browser.selected_remote_paths.contains(&path) {
            self.transfer.browser.selected_remote_path = Some(path.clone());
            self.transfer.paths.remote = path;
            self.terminal.view.status = format!(
                "{} remote item(s) marked",
                self.transfer.browser.selected_remote_paths.len()
            );
            cx.notify();
            return;
        }

        self.select_transfer_browser_entry(path, cx);
    }

    pub(in crate::features::pages::transfers) fn open_transfer_browser_context_menu(
        &mut self,
        path: String,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_transfer_browser_entry_from_context(path.clone(), window, cx);
        let Some(entry) = self
            .transfer
            .browser
            .entries
            .iter()
            .find(|entry| entry.path == path)
            .cloned()
        else {
            self.transfer.browser.context_menu = None;
            cx.notify();
            return;
        };

        self.transfer.browser.context_menu = Some(TransferBrowserContextMenuState {
            path: entry.path,
            name: entry.name,
            is_parent: false,
            is_current_directory: false,
            is_directory: entry.file_type == SftpFileType::Directory,
            x: event.position.x,
            y: event.position.y,
        });
        self.transfer.browser.status = "file context menu opened".to_string();
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn open_transfer_browser_parent_context_menu(
        &mut self,
        current_path: String,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.transfer.browser.focus);
        self.transfer.browser.drag_selection = None;
        self.transfer.browser.selected_remote_path = None;
        self.transfer.browser.selected_remote_paths.clear();
        self.transfer.browser.context_menu = Some(TransferBrowserContextMenuState {
            path: remote_parent_path(&current_path),
            name: "..".to_string(),
            is_parent: true,
            is_current_directory: false,
            is_directory: true,
            x: event.position.x,
            y: event.position.y,
        });
        self.transfer.browser.status = "parent directory context menu opened".to_string();
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn open_transfer_browser_current_context_menu(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(&self.transfer.browser.focus);
        self.transfer.browser.drag_selection = None;
        self.transfer.browser.selected_remote_path = None;
        self.transfer.browser.selected_remote_paths.clear();
        let path = normalized_transfer_browser_path(&self.transfer.browser.path);
        self.transfer.browser.context_menu = Some(TransferBrowserContextMenuState {
            name: if path == "/" {
                "/".to_string()
            } else {
                remote_file_name(&path)
            },
            path,
            is_parent: false,
            is_current_directory: true,
            is_directory: true,
            x: event.position.x,
            y: event.position.y,
        });
        self.transfer.browser.status = "current directory context menu opened".to_string();
        cx.notify();
    }

    pub(in crate::features) fn close_transfer_browser_context_menu(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.transfer.browser.context_menu = None;
        cx.notify();
    }

    fn apply_transfer_browser_range(
        &mut self,
        anchor_path: String,
        target_path: String,
        base_selection: HashSet<String>,
        additive: bool,
        cx: &mut Context<Self>,
    ) {
        let entries = self.visible_transfer_browser_entries();
        let anchor_index = entries.iter().position(|entry| entry.path == anchor_path);
        let target_index = entries.iter().position(|entry| entry.path == target_path);

        let (Some(anchor_index), Some(target_index)) = (anchor_index, target_index) else {
            if additive {
                self.transfer.browser.selected_remote_paths = base_selection;
                cx.notify();
            } else {
                self.select_transfer_browser_entry(target_path, cx);
            }
            return;
        };

        let mut next_selection = if additive {
            base_selection
        } else {
            HashSet::new()
        };
        let start = anchor_index.min(target_index);
        let end = anchor_index.max(target_index);
        for entry in &entries[start..=end] {
            next_selection.insert(entry.path.clone());
        }

        self.transfer.browser.selected_remote_paths = next_selection;
        self.transfer.browser.selected_remote_path = Some(target_path.clone());
        self.transfer.paths.remote = target_path;
        self.transfer.panel.focused_field = TransferInputField::Remote;
        self.terminal.view.status = format!(
            "{} remote item(s) marked",
            self.transfer.browser.selected_remote_paths.len()
        );
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn toggle_transfer_browser_entry_marked(
        &mut self,
        path: String,
        cx: &mut Context<Self>,
    ) {
        if self.transfer.browser.selected_remote_paths.contains(&path) {
            self.transfer.browser.selected_remote_paths.remove(&path);
        } else {
            self.transfer
                .browser
                .selected_remote_paths
                .insert(path.clone());
        }
        self.transfer.browser.selected_remote_path = Some(path.clone());
        self.transfer.paths.remote = path.clone();
        self.transfer.panel.focused_field = TransferInputField::Remote;
        self.terminal.view.status = format!(
            "{} remote item(s) marked",
            self.transfer.browser.selected_remote_paths.len()
        );
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn select_all_visible_transfer_entries(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let entries = self.visible_transfer_browser_entries();
        self.transfer.browser.selected_remote_paths =
            entries.iter().map(|entry| entry.path.clone()).collect();
        if let Some(entry) = entries.first() {
            self.transfer.browser.selected_remote_path = Some(entry.path.clone());
            self.transfer.paths.remote = entry.path.clone();
        }
        self.terminal.view.status = format!(
            "{} remote item(s) marked",
            self.transfer.browser.selected_remote_paths.len()
        );
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn selected_transfer_path_part(
        &self,
        part: TransferPathPart,
    ) -> Option<String> {
        let path = self.transfer.browser.selected_remote_path.as_deref()?;
        Some(transfer_path_part_value(path, part))
    }

    pub(in crate::features::pages::transfers) fn copy_selected_transfer_path(
        &mut self,
        part: TransferPathPart,
        cx: &mut Context<Self>,
    ) {
        let Some(value) = self.selected_transfer_path_part(part) else {
            self.terminal.view.status = "select a remote item first".to_string();
            cx.notify();
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(value.clone()));
        self.terminal.view.status = format!("copied remote {}", part.label());
        self.transfer.browser.status = truncate_preview(&value, 92);
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn send_selected_transfer_path_to_terminal(
        &mut self,
        part: TransferPathPart,
        cx: &mut Context<Self>,
    ) {
        let Some(value) = self.selected_transfer_path_part(part) else {
            self.terminal.view.status = "select a remote item first".to_string();
            cx.notify();
            return;
        };
        if self.active_session_id.is_none() {
            self.terminal.view.status = "start a session before sending remote path".to_string();
            cx.notify();
            return;
        }
        if self.send_terminal_input(value.clone().into_bytes(), cx) {
            self.terminal.view.status = format!("sent remote {} to terminal", part.label());
            self.transfer.browser.status = truncate_preview(&value, 92);
            cx.notify();
        }
    }

    pub(in crate::features::pages::transfers) fn selected_transfer_entry(
        &self,
    ) -> Option<SftpFileEntry> {
        let selected = self.transfer.browser.selected_remote_path.as_deref()?;
        self.transfer
            .browser
            .entries
            .iter()
            .find(|entry| entry.path == selected)
            .cloned()
    }

    pub(in crate::features::pages::transfers) fn selected_transfer_entries(
        &self,
    ) -> Vec<SftpFileEntry> {
        if self.transfer.browser.selected_remote_paths.is_empty() {
            return self.selected_transfer_entry().into_iter().collect();
        }
        self.visible_transfer_browser_entries()
            .into_iter()
            .filter(|entry| {
                self.transfer
                    .browser
                    .selected_remote_paths
                    .contains(&entry.path)
            })
            .collect()
    }

    pub(in crate::features::pages::transfers) fn start_selected_sftp_download_jobs(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let entries = self.selected_transfer_entries();
        if entries.is_empty() {
            self.terminal.view.status = "mark remote items before downloading".to_string();
            cx.notify();
            return;
        }
        if self.settings.transfer_ask_save_location {
            let remote_paths = entries
                .into_iter()
                .map(|entry| entry.path)
                .collect::<Vec<_>>();
            self.prompt_transfer_download_directory_and_start(remote_paths, window, cx);
            return;
        }
        let base_local_path = self.normalized_transfer_local_path();
        let total = entries.len();
        for entry in entries {
            let local_path = if total == 1 {
                base_local_path.clone()
            } else {
                base_local_path.join(remote_file_name(&entry.path))
            };
            self.start_sftp_download_job_for_target(entry.path, local_path, window, cx);
        }
        self.terminal.view.status = format!("{total} SFTP download job(s) started");
        cx.notify();
    }
}

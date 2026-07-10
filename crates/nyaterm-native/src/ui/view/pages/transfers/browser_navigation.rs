use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn cache_transfer_browser_session(&mut self, session_id: &str) {
        if session_id.trim().is_empty()
            || !self
                .session_metadata
                .get(session_id)
                .is_some_and(|metadata| metadata.ssh_config.is_some())
        {
            return;
        }

        let current_path = normalized_transfer_browser_path(&self.transfer_browser_path);
        if current_path.is_empty() {
            return;
        }

        let mut history = self.transfer_browser_history.clone();
        if history.is_empty() {
            history.push_back(current_path.clone());
        }
        let history_index = self
            .transfer_browser_history_index
            .min(history.len().saturating_sub(1));

        let home_dir = normalized_transfer_browser_path(&self.transfer_browser_home_dir);
        let home_dir = if home_dir == "." {
            current_path.clone()
        } else {
            home_dir
        };

        self.transfer_browser_session_cache.insert(
            session_id.to_string(),
            TransferBrowserSessionCacheState {
                entries: self.transfer_browser_entries.clone(),
                current_path,
                home_dir,
                history,
                history_index,
                visited_history: self.transfer_browser_visited_history.clone(),
            },
        );
    }

    pub(in crate::ui::view) fn restore_transfer_browser_session_cache(
        &mut self,
        session_id: &str,
    ) -> bool {
        let Some(cache) = self.transfer_browser_session_cache.get(session_id).cloned() else {
            return false;
        };
        self.transfer_remote_path = cache.current_path.clone();
        self.transfer_browser_path = cache.current_path;
        self.transfer_browser_home_dir = cache.home_dir;
        self.transfer_browser_home_dir_pending = false;
        self.transfer_browser_path_draft.clear();
        self.transfer_browser_path_editing = false;
        self.transfer_browser_entries = cache.entries;
        self.transfer_browser_status = format!(
            "restored cached directory · {} item(s)",
            self.transfer_browser_entries.len()
        );
        self.transfer_browser_history = cache.history;
        self.transfer_browser_history_index = cache
            .history_index
            .min(self.transfer_browser_history.len().saturating_sub(1));
        self.transfer_browser_visited_history = cache.visited_history;
        self.transfer_selected_remote_path = None;
        self.transfer_selected_remote_paths.clear();
        self.transfer_browser_drag_selection = None;
        self.cancel_transfer_browser_pending_rename_without_notify();
        self.transfer_browser_context_menu = None;
        self.transfer_browser_favorites_menu = None;
        true
    }

    pub(in crate::ui::view) fn reset_transfer_browser_for_active_session(&mut self) {
        self.transfer_remote_path = ".".to_string();
        self.transfer_browser_path = ".".to_string();
        self.transfer_browser_home_dir.clear();
        self.transfer_browser_home_dir_pending = false;
        self.transfer_browser_path_draft.clear();
        self.transfer_browser_path_editing = false;
        self.transfer_browser_entries.clear();
        self.transfer_browser_status = if self.active_ssh_config.is_some() {
            "List a remote directory to browse files.".to_string()
        } else {
            "Start an SSH session to browse remote files.".to_string()
        };
        self.transfer_browser_history.clear();
        self.transfer_browser_history_index = 0;
        self.transfer_browser_visited_history.clear();
        self.transfer_selected_remote_path = None;
        self.transfer_selected_remote_paths.clear();
        self.transfer_browser_drag_selection = None;
        self.cancel_transfer_browser_pending_rename_without_notify();
        self.transfer_browser_context_menu = None;
        self.transfer_browser_favorites_menu = None;
    }

    pub(in crate::ui::view::pages::transfers) fn open_transfer_browser_directory(
        &mut self,
        path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_transfer_browser_directory_with_history(path, true, window, cx);
    }

    pub(in crate::ui::view::pages::transfers) fn open_transfer_browser_directory_with_history(
        &mut self,
        path: String,
        record_history: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.transfer_remote_path = path.clone();
        self.transfer_browser_path = path.clone();
        self.transfer_browser_path_draft.clear();
        self.transfer_browser_path_editing = false;
        self.transfer_selected_remote_path = None;
        if record_history {
            self.record_transfer_browser_history(path);
        } else {
            self.record_transfer_browser_visited_history(path);
        }
        self.transfer_browser_status = "Loading remote directory...".to_string();
        self.start_sftp_list_job(window, cx);
    }

    pub(in crate::ui::view::pages::transfers) fn open_transfer_remote_path_from_input(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = self.normalized_transfer_remote_path();
        self.open_transfer_browser_directory(path, window, cx);
    }

    pub(in crate::ui::view) fn open_transfer_browser_history(
        &mut self,
        delta: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.transfer_browser_history.is_empty() {
            self.transfer_browser_status = "directory history is empty".to_string();
            cx.notify();
            return;
        }
        let current = self.transfer_browser_history_index as isize;
        let next = current + delta;
        if next < 0 || next as usize >= self.transfer_browser_history.len() {
            self.transfer_browser_status = if delta > 0 {
                "no older directory history".to_string()
            } else {
                "no newer directory history".to_string()
            };
            cx.notify();
            return;
        }
        self.transfer_browser_history_index = next as usize;
        let Some(path) = self
            .transfer_browser_history
            .get(self.transfer_browser_history_index)
            .cloned()
        else {
            self.transfer_browser_status = "directory history entry is unavailable".to_string();
            cx.notify();
            return;
        };
        self.open_transfer_browser_directory_with_history(path, false, window, cx);
    }

    pub(in crate::ui::view::pages::transfers) fn record_transfer_browser_history(
        &mut self,
        path: String,
    ) {
        let path = normalized_transfer_browser_path(&path);
        if path.is_empty() {
            return;
        }
        self.transfer_browser_history
            .retain(|existing| existing != &path);
        self.transfer_browser_history.push_front(path.clone());
        self.transfer_browser_history.truncate(12);
        self.transfer_browser_history_index = 0;
        self.record_transfer_browser_visited_history(path);
    }

    pub(in crate::ui::view::pages::transfers) fn record_transfer_browser_visited_history(
        &mut self,
        path: String,
    ) {
        let path = normalized_transfer_browser_path(&path);
        if path.is_empty() {
            return;
        }
        self.transfer_browser_visited_history
            .retain(|existing| existing != &path);
        self.transfer_browser_visited_history.push_front(path);
        self.transfer_browser_visited_history.truncate(30);
    }

    pub(in crate::ui::view::pages::transfers) fn add_current_transfer_browser_favorite(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let path = normalized_transfer_browser_path(&self.transfer_browser_path);
        self.add_transfer_browser_favorite_path(path, cx);
    }

    pub(in crate::ui::view::pages::transfers) fn add_transfer_browser_favorite_path(
        &mut self,
        path: String,
        cx: &mut Context<Self>,
    ) {
        let path = normalized_transfer_browser_path(&path);
        if path.is_empty() {
            self.transfer_browser_status =
                "open or select a remote directory before adding a favorite".to_string();
            cx.notify();
            return;
        }
        let existed = self
            .transfer_browser_favorites
            .iter()
            .any(|existing| existing == &path);
        self.transfer_browser_favorites
            .retain(|existing| existing != &path);
        self.transfer_browser_favorites.push_front(path.clone());
        self.transfer_browser_favorites.truncate(12);
        self.transfer_browser_status = if existed {
            format!("favorite directory moved to front: {path}")
        } else {
            format!("favorite directory added: {path}")
        };
        self.persist_transfer_browser_favorites(cx);
        cx.notify();
    }

    pub(in crate::ui::view::pages::transfers) fn remove_current_transfer_browser_favorite(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let path = normalized_transfer_browser_path(&self.transfer_browser_path);
        self.remove_transfer_browser_favorite_path(path, cx);
    }

    pub(in crate::ui::view::pages::transfers) fn remove_transfer_browser_favorite_path(
        &mut self,
        path: String,
        cx: &mut Context<Self>,
    ) {
        let path = normalized_transfer_browser_path(&path);
        if path.is_empty() {
            self.transfer_browser_status = "favorite directory path is empty".to_string();
            cx.notify();
            return;
        }
        let previous_len = self.transfer_browser_favorites.len();
        self.transfer_browser_favorites
            .retain(|existing| existing != &path);
        self.transfer_browser_status = if self.transfer_browser_favorites.len() < previous_len {
            format!("favorite directory removed: {path}")
        } else {
            format!("favorite directory not found: {path}")
        };
        if self.transfer_browser_favorites.len() < previous_len {
            self.persist_transfer_browser_favorites(cx);
        }
        cx.notify();
    }

    pub(in crate::ui::view::pages::transfers) fn toggle_current_transfer_browser_favorite(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let path = normalized_transfer_browser_path(&self.transfer_browser_path);
        if self
            .transfer_browser_favorites
            .iter()
            .any(|entry| entry == &path)
        {
            self.remove_current_transfer_browser_favorite(cx);
        } else {
            self.add_current_transfer_browser_favorite(cx);
        }
    }

    pub(in crate::ui::view::pages::transfers) fn toggle_transfer_browser_auto_sync_cwd(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(connection_id) = self.active_transfer_browser_connection_id() else {
            self.transfer_browser_status = "Auto CWD requires a saved SSH connection".to_string();
            cx.notify();
            return;
        };
        let enabled = self
            .settings
            .ui_file_explorer_auto_sync_cwd_connection_ids
            .iter()
            .any(|id| id == &connection_id);
        if enabled {
            self.settings
                .ui_file_explorer_auto_sync_cwd_connection_ids
                .retain(|id| id != &connection_id);
            self.transfer_browser_status = "Auto CWD disabled for this connection".to_string();
            self.transfer_auto_sync_cwd_last_at = None;
        } else {
            self.settings
                .ui_file_explorer_auto_sync_cwd_connection_ids
                .retain(|id| id != &connection_id);
            self.settings
                .ui_file_explorer_auto_sync_cwd_connection_ids
                .push(connection_id);
            self.transfer_browser_status = "Auto CWD enabled for this connection".to_string();
            self.transfer_auto_sync_cwd_last_at = None;
        }
        self.persist_transfer_browser_ui_settings();
        if !enabled {
            self.start_transfer_sync_cwd_job(window, cx);
        } else {
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn transfer_browser_auto_sync_cwd_enabled(&self) -> bool {
        let Some(connection_id) = self.active_transfer_browser_connection_id() else {
            return false;
        };
        self.settings
            .ui_file_explorer_auto_sync_cwd_connection_ids
            .iter()
            .any(|id| id == &connection_id)
    }

    pub(in crate::ui::view) fn sync_transfer_browser_favorites_for_active_session(&mut self) {
        let Some(connection_id) = self.active_transfer_browser_connection_id() else {
            self.transfer_browser_favorites.clear();
            return;
        };
        self.transfer_browser_favorites = self
            .settings
            .ui_file_explorer_favorite_dirs_by_connection_id
            .get(&connection_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|path| normalized_transfer_browser_path(&path))
            .filter(|path| !path.trim().is_empty())
            .fold(VecDeque::<String>::new(), |mut paths, path| {
                if !paths.iter().any(|existing| existing == &path) {
                    paths.push_back(path);
                }
                paths
            });
        self.transfer_browser_favorites.truncate(12);
    }

    pub(in crate::ui::view::pages::transfers) fn persist_transfer_browser_favorites(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(connection_id) = self.active_transfer_browser_connection_id() else {
            self.transfer_browser_status =
                "favorite kept for this temporary session only".to_string();
            return;
        };
        let favorites = self
            .transfer_browser_favorites
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        if favorites.is_empty() {
            self.settings
                .ui_file_explorer_favorite_dirs_by_connection_id
                .remove(&connection_id);
        } else {
            self.settings
                .ui_file_explorer_favorite_dirs_by_connection_id
                .insert(connection_id, favorites);
        }
        self.persist_transfer_browser_ui_settings();
        cx.notify();
    }

    pub(in crate::ui::view::pages::transfers) fn persist_transfer_browser_ui_settings(&mut self) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_file_explorer_favorite_dirs(&self.settings))
        {
            Ok(settings) => {
                self.settings = settings;
                self.store_status.message = "file explorer favorites saved".to_string();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.store_status.message = format!("file explorer favorites save failed: {error}");
                self.store_status.ready = false;
                self.transfer_browser_status = self.store_status.message.clone();
            }
        }
    }

    pub(in crate::ui::view::pages::transfers) fn active_transfer_browser_connection_id(
        &self,
    ) -> Option<String> {
        let session_id = self.active_session_id.as_deref()?;
        self.session_metadata
            .get(session_id)?
            .source_connection_id
            .clone()
            .filter(|connection_id| !connection_id.trim().is_empty())
    }

    pub(in crate::ui::view::pages::transfers) fn open_transfer_parent_directory(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let current_path = normalized_transfer_browser_path(&self.transfer_browser_path);
        if current_path == "/" || current_path == "." {
            self.transfer_browser_status = "already at the top remote directory".to_string();
            cx.notify();
            return;
        }
        let parent = remote_parent_path(&current_path);
        if parent == current_path {
            self.transfer_browser_status = "remote parent directory is unavailable".to_string();
            cx.notify();
            return;
        }
        self.transfer_remote_path = parent.clone();
        self.transfer_browser_path = parent.clone();
        self.transfer_selected_remote_path = None;
        self.transfer_selected_remote_paths.clear();
        self.record_transfer_browser_history(parent);
        self.transfer_browser_status = "Loading parent directory...".to_string();
        self.start_sftp_list_job_with_select_after(Some(current_path), window, cx);
    }

    pub(in crate::ui::view::pages::transfers) fn refresh_transfer_browser(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = if self.transfer_browser_path.trim().is_empty() {
            self.normalized_transfer_remote_path()
        } else {
            self.transfer_browser_path.clone()
        };
        self.open_transfer_browser_directory(path, window, cx);
    }
}

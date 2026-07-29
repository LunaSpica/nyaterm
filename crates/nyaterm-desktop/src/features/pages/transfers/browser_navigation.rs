use gpui::{Context, Window};
use nyaterm_core::ConnectionStore;

use std::collections::VecDeque;

use crate::features::NyaTermApp;
use crate::models::{TransferBrowserNavigationSnapshot, TransferBrowserSessionCacheState};

use super::{normalized_transfer_browser_path, remote_file_name, remote_parent_path};

impl NyaTermApp {
    pub(in crate::features) fn cache_transfer_browser_session(&mut self, session_id: &str) {
        if session_id.trim().is_empty()
            || !self
                .session
                .metadata(session_id)
                .is_some_and(|metadata| metadata.ssh_config.is_some())
        {
            return;
        }

        let current_path = normalized_transfer_browser_path(&self.transfer.browser.path);
        if current_path.is_empty() {
            return;
        }

        let mut history = self.transfer.browser.history.clone();
        if history.is_empty() {
            history.push_back(current_path.clone());
        }
        let history_index = self
            .transfer
            .browser
            .history_index
            .min(history.len().saturating_sub(1));

        let home_dir = normalized_transfer_browser_path(&self.transfer.browser.home_dir);
        let home_dir = if home_dir == "." {
            current_path.clone()
        } else {
            home_dir
        };

        self.transfer.browser.session_cache.insert(
            session_id.to_string(),
            TransferBrowserSessionCacheState {
                entries: self.transfer.browser.entries.clone(),
                current_path,
                home_dir,
                history,
                history_index,
                visited_history: self.transfer.browser.visited_history.clone(),
            },
        );
    }

    pub(in crate::features) fn restore_transfer_browser_session_cache(
        &mut self,
        session_id: &str,
    ) -> bool {
        let Some(cache) = self.transfer.browser.session_cache.get(session_id).cloned() else {
            return false;
        };
        self.transfer.paths.remote = cache.current_path.clone();
        self.transfer.browser.path = cache.current_path;
        self.transfer.browser.home_dir = cache.home_dir;
        self.transfer.browser.home_dir_pending = false;
        self.transfer.browser.path_draft.clear();
        self.transfer.browser.path_editing = false;
        self.transfer.browser.entries = cache.entries;
        self.transfer.browser.loading = false;
        self.transfer.browser.error = None;
        self.transfer.browser.status = format!(
            "restored cached directory · {} item(s)",
            self.transfer.browser.entries.len()
        );
        self.transfer.browser.history = cache.history;
        self.transfer.browser.history_index = cache
            .history_index
            .min(self.transfer.browser.history.len().saturating_sub(1));
        self.transfer.browser.visited_history = cache.visited_history;
        self.transfer.browser.selected_remote_path = None;
        self.transfer.browser.selected_remote_paths.clear();
        self.transfer.browser.drag_selection = None;
        self.cancel_transfer_browser_pending_rename_without_notify();
        self.transfer.browser.context_menu = None;
        self.transfer.browser.favorites_menu = None;
        self.transfer.browser.path_menu = None;
        self.transfer.browser.upload_menu = None;
        true
    }

    pub(in crate::features) fn reset_transfer_browser_for_active_session(&mut self) {
        self.transfer.paths.remote = ".".to_string();
        self.transfer.browser.path = ".".to_string();
        self.transfer.browser.home_dir.clear();
        self.transfer.browser.home_dir_pending = false;
        self.transfer.browser.path_draft.clear();
        self.transfer.browser.path_editing = false;
        self.transfer.browser.entries.clear();
        self.transfer.browser.loading = false;
        self.transfer.browser.error = None;
        self.transfer.browser.status = if self.session.active_ssh_config().is_some() {
            "List a remote directory to browse files.".to_string()
        } else {
            "Start an SSH session to browse remote files.".to_string()
        };
        self.transfer.browser.history.clear();
        self.transfer.browser.history_index = 0;
        self.transfer.browser.visited_history.clear();
        self.transfer.browser.selected_remote_path = None;
        self.transfer.browser.selected_remote_paths.clear();
        self.transfer.browser.drag_selection = None;
        self.cancel_transfer_browser_pending_rename_without_notify();
        self.transfer.browser.context_menu = None;
        self.transfer.browser.favorites_menu = None;
        self.transfer.browser.path_menu = None;
        self.transfer.browser.upload_menu = None;
    }

    pub(in crate::features::pages::transfers) fn open_transfer_browser_directory(
        &mut self,
        path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rollback = self.prepare_transfer_browser_navigation();
        self.open_transfer_browser_directory_with_history_and_rollback(
            path, true, rollback, window, cx,
        );
    }

    fn open_transfer_browser_directory_with_history_and_rollback(
        &mut self,
        path: String,
        record_history: bool,
        rollback: TransferBrowserNavigationSnapshot,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.forget_text_inputs("transfer.browser.path");
        self.transfer.browser.list_offset = 0;
        self.transfer.paths.remote = path.clone();
        self.transfer.browser.path = path.clone();
        self.transfer.browser.path_draft.clear();
        self.transfer.browser.path_editing = false;
        self.transfer.browser.path_menu = None;
        self.transfer.browser.selected_remote_path = None;
        if record_history {
            self.record_transfer_browser_history(path);
        } else {
            self.record_transfer_browser_visited_history(path);
        }
        self.transfer.browser.status = "Loading remote directory...".to_string();
        self.transfer.browser.loading = true;
        self.transfer.browser.error = None;
        self.start_sftp_list_job(None, rollback, window, cx);
    }

    pub(in crate::features) fn open_transfer_browser_history(
        &mut self,
        delta: isize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rollback = self.prepare_transfer_browser_navigation();
        if self.transfer.browser.history.is_empty() {
            self.transfer.browser.status = "directory history is empty".to_string();
            cx.notify();
            return;
        }
        let current = self.transfer.browser.history_index as isize;
        let next = current + delta;
        if next < 0 || next as usize >= self.transfer.browser.history.len() {
            self.transfer.browser.status = if delta > 0 {
                "no older directory history".to_string()
            } else {
                "no newer directory history".to_string()
            };
            cx.notify();
            return;
        }
        self.transfer.browser.history_index = next as usize;
        let Some(path) = self
            .transfer
            .browser
            .history
            .get(self.transfer.browser.history_index)
            .cloned()
        else {
            self.transfer.browser.status = "directory history entry is unavailable".to_string();
            cx.notify();
            return;
        };
        self.open_transfer_browser_directory_with_history_and_rollback(
            path, false, rollback, window, cx,
        );
    }

    pub(in crate::features::pages::transfers) fn record_transfer_browser_history(
        &mut self,
        path: String,
    ) {
        let path = normalized_transfer_browser_path(&path);
        if path.is_empty() {
            return;
        }
        record_transfer_browser_history_entry(
            &mut self.transfer.browser.history,
            &mut self.transfer.browser.history_index,
            path.clone(),
        );
        self.record_transfer_browser_visited_history(path);
    }

    pub(in crate::features::pages::transfers) fn record_transfer_browser_visited_history(
        &mut self,
        path: String,
    ) {
        let path = normalized_transfer_browser_path(&path);
        if path.is_empty() {
            return;
        }
        self.transfer
            .browser
            .visited_history
            .retain(|existing| existing != &path);
        self.transfer.browser.visited_history.push_front(path);
        self.transfer.browser.visited_history.truncate(30);
    }

    pub(in crate::features::pages::transfers) fn add_current_transfer_browser_favorite(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let path = normalized_transfer_browser_path(&self.transfer.browser.path);
        self.add_transfer_browser_favorite_path(path, cx);
    }

    pub(in crate::features::pages::transfers) fn add_transfer_browser_favorite_path(
        &mut self,
        path: String,
        cx: &mut Context<Self>,
    ) {
        let path = normalized_transfer_browser_path(&path);
        if path.is_empty() {
            self.transfer.browser.status =
                "open or select a remote directory before adding a favorite".to_string();
            cx.notify();
            return;
        }
        let existed = self
            .transfer
            .browser
            .favorites
            .iter()
            .any(|existing| existing == &path);
        self.transfer
            .browser
            .favorites
            .retain(|existing| existing != &path);
        self.transfer.browser.favorites.push_front(path.clone());
        self.transfer.browser.favorites.truncate(12);
        self.transfer.browser.status = if existed {
            format!("favorite directory moved to front: {path}")
        } else {
            format!("favorite directory added: {path}")
        };
        self.persist_transfer_browser_favorites(cx);
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn remove_transfer_browser_favorite_path(
        &mut self,
        path: String,
        cx: &mut Context<Self>,
    ) {
        let path = normalized_transfer_browser_path(&path);
        if path.is_empty() {
            self.transfer.browser.status = "favorite directory path is empty".to_string();
            cx.notify();
            return;
        }
        let previous_len = self.transfer.browser.favorites.len();
        self.transfer
            .browser
            .favorites
            .retain(|existing| existing != &path);
        self.transfer.browser.status = if self.transfer.browser.favorites.len() < previous_len {
            format!("favorite directory removed: {path}")
        } else {
            format!("favorite directory not found: {path}")
        };
        if self.transfer.browser.favorites.len() < previous_len {
            self.persist_transfer_browser_favorites(cx);
        }
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn toggle_transfer_browser_auto_sync_cwd(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(connection_id) = self.active_transfer_browser_connection_id() else {
            self.transfer.browser.status = "Auto CWD requires a saved SSH connection".to_string();
            cx.notify();
            return;
        };
        let enabled = self
            .settings
            .summary
            .ui_file_explorer_auto_sync_cwd_connection_ids
            .iter()
            .any(|id| id == &connection_id);
        if enabled {
            self.settings
                .summary
                .ui_file_explorer_auto_sync_cwd_connection_ids
                .retain(|id| id != &connection_id);
            self.transfer.browser.status = "Auto CWD disabled for this connection".to_string();
            self.transfer.browser.auto_sync_cwd_last_at = None;
        } else {
            self.settings
                .summary
                .ui_file_explorer_auto_sync_cwd_connection_ids
                .retain(|id| id != &connection_id);
            self.settings
                .summary
                .ui_file_explorer_auto_sync_cwd_connection_ids
                .push(connection_id);
            self.transfer.browser.status = "Auto CWD enabled for this connection".to_string();
            self.transfer.browser.auto_sync_cwd_last_at = None;
        }
        self.persist_transfer_browser_ui_settings();
        if !enabled {
            self.start_transfer_sync_cwd_job(window, cx);
        } else {
            cx.notify();
        }
    }

    pub(in crate::features::pages::transfers) fn toggle_transfer_browser_hidden_files(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.settings.summary.ui_file_explorer_show_hidden_files =
            !self.settings.summary.ui_file_explorer_show_hidden_files;
        if !self.settings.summary.ui_file_explorer_show_hidden_files {
            self.transfer
                .browser
                .selected_remote_paths
                .retain(|path| !remote_file_name(path).starts_with('.'));
            if self
                .transfer
                .browser
                .selected_remote_path
                .as_deref()
                .is_some_and(|path| remote_file_name(path).starts_with('.'))
            {
                self.transfer.browser.selected_remote_path = None;
            }
        }
        self.transfer.browser.list_offset = 0;
        self.transfer.browser.status = if self.settings.summary.ui_file_explorer_show_hidden_files {
            "hidden files shown".to_string()
        } else {
            "hidden files hidden".to_string()
        };
        self.persist_transfer_browser_ui_settings();
        cx.notify();
    }

    pub(in crate::features) fn transfer_browser_auto_sync_cwd_enabled(&self) -> bool {
        let Some(connection_id) = self.active_transfer_browser_connection_id() else {
            return false;
        };
        self.settings
            .summary
            .ui_file_explorer_auto_sync_cwd_connection_ids
            .iter()
            .any(|id| id == &connection_id)
    }

    pub(in crate::features) fn sync_transfer_browser_favorites_for_active_session(&mut self) {
        let Some(connection_id) = self.active_transfer_browser_connection_id() else {
            self.transfer.browser.favorites.clear();
            return;
        };
        self.transfer.browser.favorites = self
            .settings
            .summary
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
        self.transfer.browser.favorites.truncate(12);
    }

    pub(in crate::features::pages::transfers) fn persist_transfer_browser_favorites(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(connection_id) = self.active_transfer_browser_connection_id() else {
            self.transfer.browser.status =
                "favorite kept for this temporary session only".to_string();
            return;
        };
        let favorites = self
            .transfer
            .browser
            .favorites
            .iter()
            .cloned()
            .collect::<Vec<_>>();
        if favorites.is_empty() {
            self.settings
                .summary
                .ui_file_explorer_favorite_dirs_by_connection_id
                .remove(&connection_id);
        } else {
            self.settings
                .summary
                .ui_file_explorer_favorite_dirs_by_connection_id
                .insert(connection_id, favorites);
        }
        self.persist_transfer_browser_ui_settings();
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn persist_transfer_browser_ui_settings(&mut self) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_file_explorer_favorite_dirs(&self.settings.summary))
        {
            Ok(settings) => {
                self.apply_gpui_settings(settings);
                self.settings.store_status.message = "file explorer favorites saved".to_string();
                self.settings.store_status.ready = true;
            }
            Err(error) => {
                self.settings.store_status.message =
                    format!("file explorer favorites save failed: {error}");
                self.settings.store_status.ready = false;
                self.transfer.browser.status = self.settings.store_status.message.clone();
            }
        }
    }

    pub(in crate::features::pages::transfers) fn active_transfer_browser_connection_id(
        &self,
    ) -> Option<String> {
        let session_id = self.session.active_id()?;
        self.session
            .metadata(session_id)?
            .source_connection_id
            .clone()
            .filter(|connection_id| !connection_id.trim().is_empty())
    }

    pub(in crate::features::pages::transfers) fn open_transfer_parent_directory(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rollback = self.prepare_transfer_browser_navigation();
        let current_path = normalized_transfer_browser_path(&self.transfer.browser.path);
        if current_path == "/" || current_path == "." {
            self.transfer.browser.status = "already at the top remote directory".to_string();
            cx.notify();
            return;
        }
        let parent = remote_parent_path(&current_path);
        if parent == current_path {
            self.transfer.browser.status = "remote parent directory is unavailable".to_string();
            cx.notify();
            return;
        }
        self.transfer.paths.remote = parent.clone();
        self.transfer.browser.path = parent.clone();
        self.transfer.browser.selected_remote_path = None;
        self.transfer.browser.selected_remote_paths.clear();
        self.record_transfer_browser_history(parent);
        self.transfer.browser.status = "Loading parent directory...".to_string();
        self.transfer.browser.loading = true;
        self.transfer.browser.error = None;
        self.start_sftp_list_job(Some(current_path), rollback, window, cx);
    }

    pub(in crate::features::pages::transfers) fn refresh_transfer_browser(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = if self.transfer.browser.path.trim().is_empty() {
            self.normalized_transfer_remote_path()
        } else {
            self.transfer.browser.path.clone()
        };
        self.open_transfer_browser_directory(path, window, cx);
    }
}

fn record_transfer_browser_history_entry(
    history: &mut VecDeque<String>,
    history_index: &mut usize,
    path: String,
) {
    if history.get(*history_index) == Some(&path) {
        return;
    }
    if !history.is_empty() {
        let current_index = (*history_index).min(history.len() - 1);
        history.drain(..current_index);
    }
    history.push_front(path);
    *history_index = 0;
}

impl NyaTermApp {
    fn prepare_transfer_browser_navigation(&mut self) -> TransferBrowserNavigationSnapshot {
        let session_key = self.session.active_id_owned().unwrap_or_default();
        let pending_job_id = self.transfer.browser.navigation_jobs.remove(&session_key);
        let stable_snapshot = pending_job_id
            .and_then(|job_id| self.transfer.browser.pending_navigations.remove(&job_id));
        if let Some(snapshot) = stable_snapshot {
            self.restore_transfer_browser_navigation(snapshot.clone());
            return snapshot;
        }
        self.capture_transfer_browser_navigation()
    }

    fn capture_transfer_browser_navigation(&self) -> TransferBrowserNavigationSnapshot {
        TransferBrowserNavigationSnapshot {
            remote_path: self.transfer.paths.remote.clone(),
            browser_path: self.transfer.browser.path.clone(),
            entries: self.transfer.browser.entries.clone(),
            loading: self.transfer.browser.loading,
            error: self.transfer.browser.error.clone(),
            status: self.transfer.browser.status.clone(),
            history: self.transfer.browser.history.clone(),
            history_index: self.transfer.browser.history_index,
            visited_history: self.transfer.browser.visited_history.clone(),
            selected_path: self.transfer.browser.selected_remote_path.clone(),
            selected_paths: self.transfer.browser.selected_remote_paths.clone(),
            list_offset: self.transfer.browser.list_offset,
        }
    }

    pub(in crate::features) fn restore_transfer_browser_navigation(
        &mut self,
        snapshot: TransferBrowserNavigationSnapshot,
    ) {
        self.transfer.paths.remote = snapshot.remote_path;
        self.transfer.browser.path = snapshot.browser_path;
        self.transfer.browser.entries = snapshot.entries;
        self.transfer.browser.loading = snapshot.loading;
        self.transfer.browser.error = snapshot.error;
        self.transfer.browser.status = snapshot.status;
        self.transfer.browser.history = snapshot.history;
        self.transfer.browser.history_index = snapshot.history_index;
        self.transfer.browser.visited_history = snapshot.visited_history;
        self.transfer.browser.selected_remote_path = snapshot.selected_path;
        self.transfer.browser.selected_remote_paths = snapshot.selected_paths;
        self.transfer.browser.list_offset = snapshot.list_offset;
    }
}

#[cfg(test)]
mod tests {
    use super::record_transfer_browser_history_entry;
    use std::collections::VecDeque;

    #[test]
    fn new_navigation_after_back_discards_forward_branch() {
        let mut history =
            VecDeque::from(["/three".to_string(), "/two".to_string(), "/one".to_string()]);
        let mut index = 1;

        record_transfer_browser_history_entry(&mut history, &mut index, "/four".to_string());

        assert_eq!(
            history,
            VecDeque::from(["/four".to_string(), "/two".to_string(), "/one".to_string(),])
        );
        assert_eq!(index, 0);
    }

    #[test]
    fn refreshing_current_history_entry_preserves_forward_navigation() {
        let original =
            VecDeque::from(["/three".to_string(), "/two".to_string(), "/one".to_string()]);
        let mut history = original.clone();
        let mut index = 1;

        record_transfer_browser_history_entry(&mut history, &mut index, "/two".to_string());

        assert_eq!(history, original);
        assert_eq!(index, 1);
    }

    #[test]
    fn revisiting_a_path_keeps_chronological_history() {
        let mut history = VecDeque::from(["/two".to_string(), "/one".to_string()]);
        let mut index = 0;

        record_transfer_browser_history_entry(&mut history, &mut index, "/one".to_string());

        assert_eq!(
            history,
            VecDeque::from(["/one".to_string(), "/two".to_string(), "/one".to_string(),])
        );
        assert_eq!(index, 0);
    }
}

use super::*;
use std::collections::{HashSet, VecDeque};

use nyaterm_transport::SftpFileType;

const TRANSFER_EVENT_DRAIN_LIMIT: usize = 256;

#[derive(Clone)]
struct TransferBrowserEventSnapshot {
    remote_path: String,
    browser_path: String,
    home_dir: String,
    home_dir_pending: bool,
    entries: Vec<SftpFileEntry>,
    loading: bool,
    error: Option<String>,
    status: String,
    history: VecDeque<String>,
    history_index: usize,
    visited_history: VecDeque<String>,
    selected_path: Option<String>,
    selected_paths: HashSet<String>,
}

impl NyaTermApp {
    fn transfer_browser_event_snapshot(&self) -> TransferBrowserEventSnapshot {
        TransferBrowserEventSnapshot {
            remote_path: self.transfer_remote_path.clone(),
            browser_path: self.transfer_browser_path.clone(),
            home_dir: self.transfer_browser_home_dir.clone(),
            home_dir_pending: self.transfer_browser_home_dir_pending,
            entries: self.transfer_browser_entries.clone(),
            loading: self.transfer_browser_loading,
            error: self.transfer_browser_error.clone(),
            status: self.transfer_browser_status.clone(),
            history: self.transfer_browser_history.clone(),
            history_index: self.transfer_browser_history_index,
            visited_history: self.transfer_browser_visited_history.clone(),
            selected_path: self.transfer_selected_remote_path.clone(),
            selected_paths: self.transfer_selected_remote_paths.clone(),
        }
    }

    fn restore_transfer_browser_event_snapshot(&mut self, snapshot: TransferBrowserEventSnapshot) {
        self.transfer_remote_path = snapshot.remote_path;
        self.transfer_browser_path = snapshot.browser_path;
        self.transfer_browser_home_dir = snapshot.home_dir;
        self.transfer_browser_home_dir_pending = snapshot.home_dir_pending;
        self.transfer_browser_entries = snapshot.entries;
        self.transfer_browser_loading = snapshot.loading;
        self.transfer_browser_error = snapshot.error;
        self.transfer_browser_status = snapshot.status;
        self.transfer_browser_history = snapshot.history;
        self.transfer_browser_history_index = snapshot.history_index;
        self.transfer_browser_visited_history = snapshot.visited_history;
        self.transfer_selected_remote_path = snapshot.selected_path;
        self.transfer_selected_remote_paths = snapshot.selected_paths;
    }

    fn load_transfer_browser_event_session(&mut self, session_id: &str) {
        let Some(cache) = self.transfer_browser_session_cache.get(session_id).cloned() else {
            self.transfer_remote_path = ".".to_string();
            self.transfer_browser_path = ".".to_string();
            self.transfer_browser_home_dir.clear();
            self.transfer_browser_home_dir_pending = false;
            self.transfer_browser_entries.clear();
            self.transfer_browser_loading = false;
            self.transfer_browser_error = None;
            self.transfer_browser_status.clear();
            self.transfer_browser_history.clear();
            self.transfer_browser_history_index = 0;
            self.transfer_browser_visited_history.clear();
            self.transfer_selected_remote_path = None;
            self.transfer_selected_remote_paths.clear();
            return;
        };

        self.transfer_remote_path = cache.current_path.clone();
        self.transfer_browser_path = cache.current_path;
        self.transfer_browser_home_dir = cache.home_dir;
        self.transfer_browser_home_dir_pending = false;
        self.transfer_browser_entries = cache.entries;
        self.transfer_browser_loading = false;
        self.transfer_browser_error = None;
        self.transfer_browser_status.clear();
        self.transfer_browser_history = cache.history;
        self.transfer_browser_history_index = cache.history_index;
        self.transfer_browser_visited_history = cache.visited_history;
        self.transfer_selected_remote_path = None;
        self.transfer_selected_remote_paths.clear();
    }

    pub(super) fn drain_transfer_events(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.transfer_jobs.is_empty() {
            return false;
        }
        let mut dirty = false;
        for _ in 0..TRANSFER_EVENT_DRAIN_LIMIT {
            let Ok(event) = self.transfer_rx.try_recv() else {
                break;
            };
            dirty = true;
            let Some(job_index) = self
                .transfer_jobs
                .iter()
                .position(|candidate| candidate.id == event.id)
            else {
                continue;
            };
            let event_id = event.id.clone();
            let job_session_id = self.transfer_jobs[job_index].session_id.clone();
            let navigation_job_key = matches!(
                &self.transfer_jobs[job_index].kind,
                TransferJobKind::ListDir { .. } | TransferJobKind::SyncCwd
            )
            .then(|| job_session_id.clone().unwrap_or_default());
            if transfer_navigation_job_is_stale(
                &self.transfer_browser_navigation_jobs,
                navigation_job_key.as_deref(),
                &event_id,
            ) {
                self.transfer_jobs.remove(job_index);
                continue;
            }
            let inactive_browser_snapshot = job_session_id
                .as_deref()
                .filter(|session_id| self.active_session_id.as_deref() != Some(*session_id))
                .map(|session_id| {
                    let snapshot = self.transfer_browser_event_snapshot();
                    self.load_transfer_browser_event_session(session_id);
                    snapshot
                });
            let job = &mut self.transfer_jobs[job_index];
            let mut external_sync_to_start: Option<(Option<String>, String, String, PathBuf)> =
                None;
            let mut external_sync_prompt_to_open: Option<String> = None;
            let mut zmodem_upload_after_probe: Option<(String, Vec<PathBuf>)> = None;
            let mut open_after_create: Option<SftpFileEntry> = None;
            let event_finished = matches!(&event.event, TransferJobEvent::Finished(_));
            let event_failed = matches!(&event.event, TransferJobEvent::Finished(Err(_)));
            let cleanup_internal_job_id = (event_finished
                && !job.is_user_transfer()
                && (!matches!(&job.kind, TransferJobKind::OpenExternal { .. }) || event_failed))
                .then(|| job.id.clone());
            match event.event {
                TransferJobEvent::Started { detail } => {
                    job.status = TransferJobStatus::Running;
                    job.detail = detail;
                    job.progress = None;
                    job.summary = None;
                }
                TransferJobEvent::ExternalModified {
                    remote_path,
                    local_path,
                } => {
                    job.detail = format!("External edit changed {}", local_path.display());
                    let watch_key = format!("{remote_path}\n{}", local_path.display());
                    if self.transfer_external_always_uploads.contains(&watch_key) {
                        external_sync_to_start = Some((
                            job_session_id.clone(),
                            job.id.clone(),
                            remote_path.clone(),
                            local_path.clone(),
                        ));
                    } else if let Some(session_id) = job_session_id.clone() {
                        let prompt_id = job.id.clone();
                        self.transfer_external_sync_prompts.insert(
                            prompt_id.clone(),
                            TransferExternalSyncPromptState {
                                session_id: Some(session_id),
                                job_id: job.id.clone(),
                                remote_path: remote_path.clone(),
                                local_path: local_path.clone(),
                            },
                        );
                        external_sync_prompt_to_open = Some(prompt_id);
                        self.terminal_status =
                            format!("external edit changed: {}", local_path.display());
                    }
                }
                TransferJobEvent::Progress(progress) => {
                    if job.status == TransferJobStatus::Running {
                        job.detail = format_transfer_progress(&progress);
                    }
                    job.progress = Some(progress);
                }
                TransferJobEvent::Finished(Ok(TransferJobOutput::Entries(entries))) => {
                    let (listed_path, select_after) = match &job.kind {
                        TransferJobKind::ListDir {
                            remote_path,
                            select_after,
                        } => (remote_path.clone(), select_after.clone()),
                        _ => (self.transfer_browser_path.clone(), None),
                    };
                    job.status = TransferJobStatus::Completed;
                    job.detail = format!("{} item(s)", entries.len());
                    self.transfer_browser_path = listed_path;
                    self.transfer_browser_entries = entries.clone();
                    self.transfer_browser_loading = false;
                    self.transfer_browser_error = None;
                    self.transfer_browser_status = job.detail.clone();
                    self.transfer_selected_remote_paths
                        .retain(|path| entries.iter().any(|entry| &entry.path == path));
                    if let Some(select_after) = select_after
                        && entries.iter().any(|entry| entry.path == select_after)
                    {
                        self.transfer_selected_remote_path = Some(select_after.clone());
                        self.transfer_selected_remote_paths.clear();
                        self.transfer_selected_remote_paths
                            .insert(select_after.clone());
                        self.transfer_remote_path = select_after;
                    }
                    job.entries = entries;
                    job.summary = None;
                    job.progress = None;
                    job.control = None;
                    self.terminal_status = format!("SFTP list completed: {}", job.detail);
                }
                TransferJobEvent::Finished(Ok(TransferJobOutput::HomeDir(home_dir))) => {
                    job.status = TransferJobStatus::Completed;
                    job.detail = format!("Home {home_dir}");
                    self.transfer_browser_home_dir = home_dir.clone();
                    self.transfer_browser_home_dir_pending = false;
                    self.transfer_browser_loading = false;
                    self.transfer_browser_error = None;
                    self.transfer_browser_status =
                        format!("remote home resolved: {}", truncate_preview(&home_dir, 72));
                    job.entries.clear();
                    job.summary = None;
                    job.progress = None;
                    job.control = None;
                    self.terminal_status = "SFTP remote home resolved".to_string();
                }
                TransferJobEvent::Finished(Ok(TransferJobOutput::CwdSynced {
                    remote_path,
                    entries,
                })) => {
                    job.status = TransferJobStatus::Completed;
                    job.detail = format!("Synced cwd {remote_path}");
                    self.transfer_remote_path = remote_path.clone();
                    self.transfer_browser_path = remote_path;
                    self.transfer_browser_entries = entries.clone();
                    self.transfer_browser_loading = false;
                    self.transfer_browser_error = None;
                    self.transfer_browser_status =
                        format!("remote exec cwd · {} item(s)", entries.len());
                    self.transfer_selected_remote_path = None;
                    self.transfer_selected_remote_paths.clear();
                    job.entries = entries;
                    job.summary = None;
                    job.progress = None;
                    job.control = None;
                    self.terminal_status = "SFTP cwd sync completed".to_string();
                }
                TransferJobEvent::Finished(Ok(TransferJobOutput::Renamed {
                    old_path,
                    new_path,
                    parent_path,
                    entries,
                })) => {
                    job.status = TransferJobStatus::Completed;
                    job.detail = format!("Renamed {old_path} -> {new_path}");
                    self.transfer_browser_path = parent_path.clone();
                    self.transfer_browser_entries = entries.clone();
                    self.transfer_browser_status = format!("{} item(s)", entries.len());
                    job.entries = entries;
                    job.summary = None;
                    job.progress = None;
                    job.control = None;
                    self.transfer_selected_remote_path = Some(new_path.clone());
                    self.transfer_selected_remote_paths.remove(&old_path);
                    self.transfer_selected_remote_paths.insert(new_path.clone());
                    self.transfer_remote_path = new_path.clone();
                    self.terminal_status =
                        format!("SFTP rename completed in {parent_path}: {new_path}");
                }
                TransferJobEvent::Finished(Ok(TransferJobOutput::Moved {
                    old_path,
                    new_path,
                    parent_path,
                    entries,
                })) => {
                    job.status = TransferJobStatus::Completed;
                    job.detail = format!("Moved {old_path} -> {new_path}");
                    self.transfer_browser_path = parent_path.clone();
                    self.transfer_browser_entries = entries.clone();
                    self.transfer_browser_status = format!("{} item(s)", entries.len());
                    job.entries = entries;
                    job.summary = None;
                    job.progress = None;
                    job.control = None;
                    self.transfer_selected_remote_path = Some(new_path.clone());
                    self.transfer_selected_remote_paths.remove(&old_path);
                    self.transfer_selected_remote_paths.insert(new_path.clone());
                    self.transfer_remote_path = new_path.clone();
                    self.terminal_status =
                        format!("SFTP move completed from {parent_path}: {new_path}");
                }
                TransferJobEvent::Finished(Ok(TransferJobOutput::Deleted {
                    remote_path,
                    parent_path,
                    entries,
                })) => {
                    job.status = TransferJobStatus::Completed;
                    job.detail = format!("Deleted {remote_path}");
                    self.transfer_browser_path = parent_path.clone();
                    self.transfer_browser_entries = entries.clone();
                    self.transfer_browser_status = format!("{} item(s)", entries.len());
                    job.entries = entries;
                    job.summary = None;
                    job.progress = None;
                    job.control = None;
                    if self.transfer_selected_remote_path.as_deref() == Some(remote_path.as_str()) {
                        self.transfer_selected_remote_path = None;
                    }
                    self.transfer_selected_remote_paths.remove(&remote_path);
                    self.terminal_status =
                        format!("SFTP delete completed in {parent_path}: {remote_path}");
                }
                TransferJobEvent::Finished(Ok(TransferJobOutput::CreatedDirectory {
                    remote_path,
                    parent_path,
                    entries,
                    open_after_create,
                })) => {
                    job.status = TransferJobStatus::Completed;
                    job.detail = format!("Created {remote_path}");
                    self.transfer_browser_path = if open_after_create {
                        remote_path.clone()
                    } else {
                        parent_path.clone()
                    };
                    self.transfer_browser_entries = entries.clone();
                    self.transfer_browser_status = format!("{} item(s)", entries.len());
                    job.entries = entries;
                    job.summary = None;
                    job.progress = None;
                    job.control = None;
                    self.transfer_selected_remote_path = Some(remote_path.clone());
                    self.transfer_selected_remote_paths.clear();
                    self.transfer_selected_remote_paths
                        .insert(remote_path.clone());
                    self.transfer_remote_path = remote_path.clone();
                    self.terminal_status =
                        format!("SFTP directory created in {parent_path}: {remote_path}");
                }
                TransferJobEvent::Finished(Ok(TransferJobOutput::CreatedFile {
                    remote_path,
                    parent_path,
                    entries,
                    open_after_create: should_open,
                })) => {
                    job.status = TransferJobStatus::Completed;
                    job.detail = format!("Created {remote_path}");
                    self.transfer_browser_path = parent_path.clone();
                    self.transfer_browser_entries = entries.clone();
                    self.transfer_browser_status = format!("{} item(s)", entries.len());
                    job.entries = entries.clone();
                    job.summary = None;
                    job.progress = None;
                    job.control = None;
                    self.transfer_selected_remote_path = Some(remote_path.clone());
                    self.transfer_selected_remote_paths.clear();
                    self.transfer_selected_remote_paths
                        .insert(remote_path.clone());
                    self.transfer_remote_path = remote_path.clone();
                    self.terminal_status =
                        format!("SFTP file created in {parent_path}: {remote_path}");
                    if should_open && job_session_id == self.active_session_id {
                        open_after_create = Some(
                            entries
                                .iter()
                                .find(|entry| entry.path == remote_path)
                                .cloned()
                                .unwrap_or_else(|| SftpFileEntry {
                                    name: remote_path
                                        .rsplit('/')
                                        .next()
                                        .unwrap_or(remote_path.as_str())
                                        .to_string(),
                                    path: remote_path.clone(),
                                    file_type: SftpFileType::File,
                                    size: Some(0),
                                    permissions: None,
                                    owner: String::new(),
                                    group: String::new(),
                                    modified_at: None,
                                }),
                        );
                    }
                }
                TransferJobEvent::Finished(Ok(TransferJobOutput::CreatedSymlink {
                    link_path,
                    target_path,
                    parent_path,
                    entries,
                })) => {
                    job.status = TransferJobStatus::Completed;
                    job.detail = format!("Linked {link_path} -> {target_path}");
                    self.transfer_browser_path = parent_path.clone();
                    self.transfer_browser_entries = entries.clone();
                    self.transfer_browser_status = format!("{} item(s)", entries.len());
                    job.entries = entries;
                    job.summary = None;
                    job.progress = None;
                    job.control = None;
                    self.transfer_selected_remote_path = Some(link_path.clone());
                    self.transfer_selected_remote_paths.clear();
                    self.transfer_selected_remote_paths
                        .insert(link_path.clone());
                    self.transfer_remote_path = link_path.clone();
                    self.terminal_status =
                        format!("SFTP symlink created in {parent_path}: {link_path}");
                }
                TransferJobEvent::Finished(Ok(TransferJobOutput::PropertiesLoaded {
                    remote_path,
                    properties,
                })) => {
                    job.status = TransferJobStatus::Completed;
                    job.detail = format!("Loaded properties for {remote_path}");
                    job.summary = None;
                    job.progress = None;
                    job.control = None;
                    if let Some(state) = self.transfer_properties.as_mut()
                        && state.session_id.as_deref() == job_session_id.as_deref()
                        && state.entry.path == remote_path
                    {
                        state.mode_value = properties
                            .permissions
                            .map(format_permissions_octal)
                            .unwrap_or_else(|| "0644".to_string());
                        state.owner_value = if properties.owner.is_empty() {
                            properties
                                .uid
                                .map(|uid| uid.to_string())
                                .unwrap_or_default()
                        } else {
                            properties.owner.clone()
                        };
                        state.group_value = if properties.group.is_empty() {
                            properties
                                .gid
                                .map(|gid| gid.to_string())
                                .unwrap_or_default()
                        } else {
                            properties.group.clone()
                        };
                        state.properties = Some(properties);
                        state.error = None;
                    }
                    self.transfer_browser_status = format!("properties loaded for {remote_path}");
                    self.terminal_status = format!("SFTP properties loaded: {remote_path}");
                }
                TransferJobEvent::Finished(Ok(TransferJobOutput::PropertiesUpdated {
                    remote_path,
                    parent_path,
                    properties,
                    entries,
                })) => {
                    job.status = TransferJobStatus::Completed;
                    job.detail = format!("Updated properties for {remote_path}");
                    self.transfer_browser_path = parent_path.clone();
                    self.transfer_browser_entries = entries.clone();
                    self.transfer_browser_status = format!("{} item(s)", entries.len());
                    job.entries = entries;
                    job.summary = None;
                    job.progress = None;
                    job.control = None;
                    self.transfer_selected_remote_path = Some(remote_path.clone());
                    self.transfer_selected_remote_paths.clear();
                    self.transfer_selected_remote_paths
                        .insert(remote_path.clone());
                    self.transfer_remote_path = remote_path.clone();
                    if let Some(state) = self.transfer_properties.as_mut()
                        && state.session_id.as_deref() == job_session_id.as_deref()
                        && state.entry.path == remote_path
                    {
                        state.mode_value = properties
                            .permissions
                            .map(format_permissions_octal)
                            .unwrap_or_else(|| state.mode_value.clone());
                        state.owner_value = if properties.owner.is_empty() {
                            properties
                                .uid
                                .map(|uid| uid.to_string())
                                .unwrap_or_default()
                        } else {
                            properties.owner.clone()
                        };
                        state.group_value = if properties.group.is_empty() {
                            properties
                                .gid
                                .map(|gid| gid.to_string())
                                .unwrap_or_default()
                        } else {
                            properties.group.clone()
                        };
                        state.properties = Some(properties);
                        state.saving = false;
                        state.error = None;
                    }
                    if self.transfer_properties.as_ref().is_some_and(|state| {
                        state.session_id.as_deref() == job_session_id.as_deref()
                            && state.entry.path == remote_path
                    }) {
                        self.transfer_properties = None;
                    }
                    self.terminal_status =
                        format!("SFTP properties updated in {parent_path}: {remote_path}");
                }
                TransferJobEvent::Finished(Ok(TransferJobOutput::EditorLoaded {
                    remote_path,
                    file,
                })) => {
                    job.status = TransferJobStatus::Completed;
                    job.detail = format!("Opened {remote_path}");
                    job.summary = None;
                    job.progress = None;
                    job.control = None;
                    if let Some(state) = self.transfer_editor.as_mut().and_then(|workspace| {
                        workspace.tab_mut(job_session_id.as_deref(), &remote_path)
                    }) {
                        state.content = file.content;
                        state.base_size = Some(file.size);
                        state.base_modified_at = Some(file.modified_at);
                        state.loading = false;
                        state.saving = false;
                        state.dirty = false;
                        state.conflict = false;
                        state.close_after_save = false;
                        state.reload_confirm = false;
                        state.error = None;
                    }
                    self.transfer_browser_status = format!("opened text file {remote_path}");
                    self.terminal_status = format!("SFTP text file opened: {remote_path}");
                }
                TransferJobEvent::Finished(Ok(TransferJobOutput::AiFileActionLoaded {
                    remote_path,
                    action_id,
                    action_name,
                    prompt,
                    file,
                })) => {
                    job.status = TransferJobStatus::Completed;
                    job.detail = format!("Prepared AI action {action_name} for {remote_path}");
                    job.summary = None;
                    job.progress = None;
                    job.control = None;

                    if job_session_id.as_deref() == self.active_session_id.as_deref() {
                        let mut context = self.ai_terminal_context();
                        context.selected_text = file.content;
                        context.cwd = Some(transfer_event_remote_parent_path(&remote_path));
                        self.ai_prepared_request = Some(AiPreparedRequest {
                            action: AiAction::CustomFileAction,
                            context,
                            source_label: format!("{action_name} · {remote_path}"),
                        });
                        self.ai_prompt_draft = prompt;
                        self.ai_response_preview = format!(
                            "Loaded {} byte(s) from {remote_path} for AI action {action_name}",
                            file.size
                        );
                        self.ai_status =
                            format!("AI file action ready: {action_name} ({action_id})");
                        self.ensure_panel_open(NavItem::AiAssistant);
                        self.ai_chat_focus_pending = true;
                        self.transfer_browser_status = format!("AI action ready for {remote_path}");
                        self.terminal_status =
                            format!("AI assistant opened for remote file: {remote_path}");
                    }
                }
                TransferJobEvent::Finished(Ok(TransferJobOutput::EditorSaved {
                    remote_path,
                    result,
                })) => {
                    job.status = TransferJobStatus::Completed;
                    job.detail = format!("Saved {remote_path}");
                    job.summary = None;
                    job.progress = None;
                    job.control = None;
                    let mut remove_saved_tab_id = None;
                    let mut close_editor_workspace = false;
                    if let Some(workspace) = self.transfer_editor.as_mut() {
                        let mut save_conflicted = false;
                        if let Some(state) =
                            workspace.tab_mut(job_session_id.as_deref(), &remote_path)
                        {
                            match result {
                                nyaterm_transport::SftpWriteTextResult::Saved {
                                    modified_at,
                                    size,
                                } => {
                                    if state.close_after_save {
                                        remove_saved_tab_id = Some(state.id.clone());
                                    }
                                    state.base_size = Some(size);
                                    state.base_modified_at = Some(modified_at);
                                    state.saving = false;
                                    state.dirty = false;
                                    state.conflict = false;
                                    state.close_after_save = false;
                                    state.reload_confirm = false;
                                    state.error = None;
                                    self.terminal_status =
                                        format!("SFTP text file saved: {remote_path}");
                                }
                                nyaterm_transport::SftpWriteTextResult::Conflict {
                                    modified_at,
                                    size,
                                } => {
                                    save_conflicted = true;
                                    state.base_size = Some(size);
                                    state.base_modified_at = Some(modified_at);
                                    state.saving = false;
                                    state.conflict = true;
                                    state.close_after_save = false;
                                    state.error =
                                        Some("Remote file changed before save.".to_string());
                                    self.terminal_status =
                                        format!("SFTP text save conflict: {remote_path}");
                                }
                            }
                        }
                        if save_conflicted {
                            workspace.close_after_save_all = false;
                            workspace.close_confirm = true;
                        }
                        if let Some(tab_id) = remove_saved_tab_id.as_deref() {
                            workspace.remove_tab(tab_id);
                            workspace.pending_close_tab_id = None;
                            workspace.close_confirm = false;
                        }
                        if workspace.close_after_save_all
                            && workspace.tabs.iter().all(|tab| !tab.dirty && !tab.saving)
                        {
                            close_editor_workspace = true;
                        }
                        if workspace.tabs.is_empty() {
                            close_editor_workspace = true;
                        }
                        if close_editor_workspace {
                            self.terminal_status =
                                format!("SFTP text file saved and closed: {remote_path}");
                        }
                    }
                    if close_editor_workspace {
                        self.transfer_editor = None;
                    }
                    self.transfer_browser_status =
                        format!("text editor save finished for {remote_path}");
                }
                TransferJobEvent::Finished(Ok(TransferJobOutput::ExternalOpened {
                    remote_path,
                    local_path,
                })) => {
                    job.status = TransferJobStatus::Completed;
                    job.detail = format!("Opened {}", local_path.display());
                    job.summary = None;
                    job.progress = None;
                    job.control = None;
                    self.transfer_browser_status = format!("opened external {remote_path}");
                    self.terminal_status =
                        format!("SFTP file opened externally: {}", local_path.display());
                }
                TransferJobEvent::Finished(Ok(TransferJobOutput::Summary(summary))) => {
                    job.status = TransferJobStatus::Completed;
                    job.detail = if summary.skipped {
                        "Skipped duplicate".to_string()
                    } else {
                        format!("{} transferred", format_file_size(Some(summary.bytes)))
                    };
                    job.entries.clear();
                    job.progress = Some(SftpTransferProgress {
                        remote_path: summary.remote_path.clone(),
                        local_path: summary.local_path.clone(),
                        bytes_transferred: summary.bytes,
                        total_bytes: Some(summary.bytes),
                        item_count_completed: job
                            .progress
                            .as_ref()
                            .and_then(|progress| progress.item_count_total),
                        item_count_total: job
                            .progress
                            .as_ref()
                            .and_then(|progress| progress.item_count_total),
                    });
                    job.summary = Some(summary);
                    self.terminal_status = format!("SFTP transfer completed: {}", job.detail);
                    job.control = None;
                }
                TransferJobEvent::Finished(Ok(TransferJobOutput::Uploaded {
                    summary,
                    parent_path,
                    entries,
                })) => {
                    job.status = TransferJobStatus::Completed;
                    job.detail = format!("{} uploaded", format_file_size(Some(summary.bytes)));
                    job.progress = Some(SftpTransferProgress {
                        remote_path: summary.remote_path.clone(),
                        local_path: summary.local_path.clone(),
                        bytes_transferred: summary.bytes,
                        total_bytes: Some(summary.bytes),
                        item_count_completed: job
                            .progress
                            .as_ref()
                            .and_then(|progress| progress.item_count_total),
                        item_count_total: job
                            .progress
                            .as_ref()
                            .and_then(|progress| progress.item_count_total),
                    });
                    job.summary = Some(summary.clone());
                    job.control = None;

                    if transfer_event_paths_match(&self.transfer_browser_path, &parent_path) {
                        self.transfer_browser_path = parent_path.clone();
                        self.transfer_browser_entries = entries.clone();
                        self.transfer_browser_status = format!("{} item(s)", entries.len());
                        self.transfer_selected_remote_path = Some(summary.remote_path.clone());
                        self.transfer_selected_remote_paths.clear();
                        self.transfer_selected_remote_paths
                            .insert(summary.remote_path.clone());
                        self.transfer_remote_path = summary.remote_path.clone();
                    } else {
                        self.transfer_browser_status =
                            format!("uploaded to {}", truncate_preview(&parent_path, 48));
                    }

                    job.entries = entries;
                    self.terminal_status =
                        format!("SFTP upload completed in {parent_path}: {}", job.detail);
                }
                TransferJobEvent::Finished(Ok(TransferJobOutput::ZmodemProbeReady {
                    session_id,
                    files,
                    probe_skipped,
                })) => {
                    job.status = TransferJobStatus::Completed;
                    job.detail = if probe_skipped {
                        format!("ZMODEM probe skipped; uploading {} file(s)", files.len())
                    } else {
                        format!("ZMODEM probe ready; uploading {} file(s)", files.len())
                    };
                    job.entries.clear();
                    job.summary = None;
                    job.progress = None;
                    job.control = None;
                    self.terminal_status = job.detail.clone();
                    if files.is_empty() {
                        self.terminal_status =
                            "ZMODEM upload cancelled — all conflicting files skipped".to_string();
                    } else {
                        zmodem_upload_after_probe = Some((session_id, files));
                    }
                }
                TransferJobEvent::Finished(Err(error)) => {
                    let browser_load_failed = matches!(
                        &job.kind,
                        TransferJobKind::ListDir { .. }
                            | TransferJobKind::ResolveHome
                            | TransferJobKind::SyncCwd
                    );
                    let property_remote_path = match &job.kind {
                        TransferJobKind::LoadProperties { remote_path }
                        | TransferJobKind::UpdateProperties { remote_path, .. }
                        | TransferJobKind::LoadEditor { remote_path }
                        | TransferJobKind::SaveEditor { remote_path }
                        | TransferJobKind::OpenExternal { remote_path, .. }
                        | TransferJobKind::AiFileAction { remote_path, .. } => {
                            Some(remote_path.clone())
                        }
                        _ => None,
                    };
                    if error == SFTP_TRANSFER_CANCELLED {
                        job.status = TransferJobStatus::Cancelled;
                        job.detail = "Cancelled".to_string();
                        self.terminal_status = format!("SFTP transfer cancelled: {}", job.id);
                    } else {
                        job.status = TransferJobStatus::Failed;
                        job.detail = error.clone();
                        self.terminal_status = format!("SFTP transfer failed: {error}");
                    }
                    if browser_load_failed {
                        self.transfer_browser_loading = false;
                        self.transfer_browser_error = self
                            .transfer_browser_entries
                            .is_empty()
                            .then_some(error.clone());
                    }
                    if let Some(remote_path) = property_remote_path.as_ref()
                        && let Some(state) = self.transfer_properties.as_mut()
                        && state.session_id.as_deref() == job_session_id.as_deref()
                        && state.entry.path == *remote_path
                    {
                        state.saving = false;
                        state.error = Some(error.clone());
                    }
                    if let Some(remote_path) = property_remote_path.as_ref()
                        && let Some(workspace) = self.transfer_editor.as_mut()
                        && let Some(state) =
                            workspace.tab_mut(job_session_id.as_deref(), remote_path)
                    {
                        state.loading = false;
                        state.saving = false;
                        state.close_after_save = false;
                        state.error = Some(error);
                        workspace.close_after_save_all = false;
                    }
                    job.summary = None;
                    job.control = None;
                }
            }
            if let Some((session_id, job_id, remote_path, local_path)) = external_sync_to_start {
                self.spawn_external_editor_sync_upload(session_id, job_id, remote_path, local_path);
            }
            if let Some((session_id, files)) = zmodem_upload_after_probe {
                self.begin_zmodem_upload_after_probe(session_id, files, cx);
            }
            if let Some(entry) = open_after_create
                && inactive_browser_snapshot.is_none()
                && self.active_session_id == job_session_id
            {
                self.open_transfer_default(entry, window, cx);
            }
            if let Some(snapshot) = inactive_browser_snapshot {
                if let Some(session_id) = job_session_id.as_deref() {
                    self.cache_transfer_browser_session(session_id);
                }
                self.restore_transfer_browser_event_snapshot(snapshot);
            }
            if event_finished
                && let Some(key) = navigation_job_key
                && self
                    .transfer_browser_navigation_jobs
                    .get(&key)
                    .is_some_and(|latest_id| latest_id == &event_id)
            {
                self.transfer_browser_navigation_jobs.remove(&key);
            }
            if let Some(job_id) = cleanup_internal_job_id {
                self.transfer_jobs.retain(|job| job.id != job_id);
            }
            if let Some(prompt_id) = external_sync_prompt_to_open {
                self.open_transfer_external_sync_window(prompt_id, cx);
            }
        }
        dirty
    }
}

fn transfer_event_paths_match(left: &str, right: &str) -> bool {
    transfer_event_normalized_path(left) == transfer_event_normalized_path(right)
}

fn transfer_event_remote_parent_path(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() || trimmed == "/" {
        return "/".to_string();
    }
    match trimmed.rsplit_once('/') {
        Some(("", _)) => "/".to_string(),
        Some((parent, _)) if !parent.is_empty() => parent.to_string(),
        _ => ".".to_string(),
    }
}

fn transfer_event_normalized_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        ".".to_string()
    } else if trimmed == "/" {
        "/".to_string()
    } else {
        trimmed.trim_end_matches('/').to_string()
    }
}

fn transfer_navigation_job_is_stale(
    latest_jobs: &HashMap<String, String>,
    session_key: Option<&str>,
    event_id: &str,
) -> bool {
    session_key.is_some_and(|key| {
        latest_jobs
            .get(key)
            .is_none_or(|latest_id| latest_id != event_id)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn superseded_transfer_navigation_result_is_stale_per_session() {
        let latest_jobs = HashMap::from([
            ("session-a".to_string(), "job-a2".to_string()),
            ("session-b".to_string(), "job-b1".to_string()),
        ]);

        assert!(transfer_navigation_job_is_stale(
            &latest_jobs,
            Some("session-a"),
            "job-a1"
        ));
        assert!(!transfer_navigation_job_is_stale(
            &latest_jobs,
            Some("session-a"),
            "job-a2"
        ));
        assert!(!transfer_navigation_job_is_stale(
            &latest_jobs,
            Some("session-b"),
            "job-b1"
        ));
        assert!(!transfer_navigation_job_is_stale(
            &latest_jobs,
            None,
            "unrelated-job"
        ));
    }
}

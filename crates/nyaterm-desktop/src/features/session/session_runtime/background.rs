use std::time::{Duration, Instant};

use gpui::Context;
use nyaterm_core::{AiExecutionProfile, ConnectionStore, uuid};
use nyaterm_transport::{
    LocalSessionConfig, SerialSessionConfig, SessionInfo, SessionKind, SessionManager,
    SshMultiplexHandle, SshSessionConfig, TelnetSessionConfig, open_ssh_multiplex_handle,
};

use super::super::state::{failed_session_start_display_name, pending_session_start_display_name};
use super::PendingSessionStartRegistration;
use crate::features::formatting::{session_kind_label, short_id, ssh_multiplex_key};
use crate::features::{
    FailedSessionStart, NyaTermApp, PendingSessionStart, SessionStartEventRequest,
    SessionStartResult, SessionStartSuccess,
};
use crate::models::{
    MainMode, NavItem, SessionLaunchConfig, SessionRuntimeMetadata, StartupCommandRequest,
    TerminalViewState,
};

const SESSION_START_EVENT_DRAIN_LIMIT: usize = 8;

impl NyaTermApp {
    pub(in crate::features) fn has_pending_session_start(&self) -> bool {
        self.session.start.has_pending()
    }

    pub(in crate::features) fn has_failed_session_start(&self) -> bool {
        self.session.start.has_failed()
    }

    pub(in crate::features) fn pending_session_display_name(&self) -> Option<String> {
        self.session.start.pending_display_name()
    }

    pub(in crate::features) fn active_failed_session(&self) -> Option<&FailedSessionStart> {
        self.session.start.active_failed()
    }

    pub(in crate::features) fn failed_session_display_name(&self) -> Option<String> {
        self.session.start.failed_display_name()
    }

    pub(in crate::features) fn select_pending_session_start(
        &mut self,
        request_id: String,
        cx: &mut Context<Self>,
    ) {
        if !self.session.start.select_pending(&request_id) {
            return;
        }
        self.shell.chrome.open_tabs_menu_open = false;
        self.shell.chrome.new_session_menu_open = false;
        self.shell.navigation.selected_nav = NavItem::Workspace;
        self.shell.navigation.main_mode = MainMode::Workspace;
        cx.notify();
    }

    pub(in crate::features) fn close_pending_session_start(
        &mut self,
        request_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(pending) = self.session.start.close_pending(&request_id) else {
            return;
        };
        self.terminal.view.status = format!(
            "cancelled connection {}",
            pending_session_start_display_name(&pending)
        );
        cx.notify();
    }

    pub(in crate::features) fn select_failed_session_start(
        &mut self,
        request_id: String,
        cx: &mut Context<Self>,
    ) {
        if !self.session.start.select_failed(&request_id) {
            return;
        }
        self.shell.chrome.open_tabs_menu_open = false;
        self.shell.chrome.new_session_menu_open = false;
        self.shell.navigation.selected_nav = NavItem::Workspace;
        self.shell.navigation.main_mode = MainMode::Workspace;
        cx.notify();
    }

    pub(in crate::features) fn close_failed_session_start(
        &mut self,
        request_id: String,
        cx: &mut Context<Self>,
    ) {
        let Some(failed) = self.session.start.close_failed(&request_id) else {
            return;
        };
        if !self.session.start.has_failed() {
            self.shell.chrome.last_connect_failure_name = None;
            self.shell.chrome.last_connect_failure_error = None;
        }
        self.terminal.view.status = format!(
            "closed failed connection {}",
            failed_session_start_display_name(&failed)
        );
        cx.notify();
    }

    pub(in crate::features) fn pending_session_status_source(&self) -> Option<(String, Instant)> {
        self.session.start.pending_status_source()
    }

    pub(in crate::features) fn register_pending_session_start(
        &mut self,
        registration: PendingSessionStartRegistration,
        cx: &mut Context<Self>,
    ) -> String {
        let request_id = uuid();
        let requested_at = Instant::now();
        let PendingSessionStartRegistration {
            connection_name,
            launch_config,
            kind,
            ai_execution_profile,
            custom_name,
            tab_color,
            after_session_id,
            insert_index,
            seed_output,
            startup_command,
            multiplex_key,
            source_connection_id,
            status_message,
            append_start_log,
        } = registration;

        let reconnecting = self.session.start.register_pending(
            request_id.clone(),
            PendingSessionStart {
                connection_name: connection_name.clone(),
                launch_config,
                requested_at,
                kind,
                ai_execution_profile,
                custom_name,
                tab_color,
                after_session_id,
                insert_index,
                seed_output,
                startup_command,
                multiplex_key,
                source_connection_id,
                reconnect_session_id: None,
            },
        );
        if !reconnecting {
            self.shell.chrome.last_connect_failure_name = None;
            self.shell.chrome.last_connect_failure_error = None;
        }
        self.terminal.view.status = status_message;
        // Status + connecting tab already show progress; avoid full terminal decode
        // work on the click path before the worker even starts.
        let _ = append_start_log;
        self.shell.navigation.selected_nav = NavItem::Workspace;
        self.shell.navigation.main_mode = MainMode::Workspace;
        cx.notify();
        request_id
    }

    pub(in crate::features) fn begin_background_session_start(
        &mut self,
        connection_name: String,
        launch_config: SessionLaunchConfig,
        source_connection_id: Option<String>,
        ai_execution_profile: AiExecutionProfile,
        custom_name: Option<String>,
        tab_color: Option<u32>,
        after_session_id: Option<String>,
        insert_index: Option<usize>,
        seed_output: Option<String>,
        startup_command: Option<StartupCommandRequest>,
        cx: &mut Context<Self>,
    ) {
        let kind = session_kind_for_launch_config(&launch_config);
        let request_id = self.register_pending_session_start(
            PendingSessionStartRegistration {
                connection_name: connection_name.clone(),
                launch_config: Some(launch_config.clone()),
                kind,
                ai_execution_profile,
                custom_name,
                tab_color,
                after_session_id,
                insert_index,
                seed_output,
                startup_command,
                multiplex_key: None,
                source_connection_id,
                status_message: format!("connecting to {connection_name}"),
                append_start_log: true,
            },
            cx,
        );

        let session_manager = self.session.manager.clone();
        let session_start_tx = self.session.start.sender();
        let request_id_for_worker = request_id.clone();
        std::thread::spawn(move || {
            let worker_started_at = Instant::now();
            let result = create_session_from_launch_config(&session_manager, launch_config.clone())
                .map(|session_info| SessionStartSuccess {
                    session_info,
                    multiplex_handle: None,
                    launch_config: Some(launch_config),
                })
                .map_err(|error| error.to_string());
            let worker_finished_at = Instant::now();
            let _ = session_start_tx.send(SessionStartResult {
                request_id: request_id_for_worker,
                connection_name,
                kind,
                worker_started_at,
                worker_finished_at,
                result,
            });
        });
    }

    pub(in crate::features) fn begin_background_ssh_start(
        &mut self,
        connection_name: String,
        mut config: SshSessionConfig,
        source_connection_id: Option<String>,
        ai_execution_profile: AiExecutionProfile,
        custom_name: Option<String>,
        tab_color: Option<u32>,
        after_session_id: Option<String>,
        insert_index: Option<usize>,
        seed_output: Option<String>,
        startup_command: Option<StartupCommandRequest>,
        cx: &mut Context<Self>,
    ) {
        config.deferred_pty = true;
        let geometry_session_hint = after_session_id
            .as_deref()
            .or(self.session.start.reconnect_target());
        if let Some(geometry) =
            self.desired_terminal_resize_geometry_for_session_hint(geometry_session_hint)
        {
            config.cols = geometry.cols;
            config.rows = geometry.rows;
            config.pixel_width = geometry.pixel_width;
            config.pixel_height = geometry.pixel_height;
        }
        let request_id = self.register_pending_session_start(
            PendingSessionStartRegistration {
                connection_name: connection_name.clone(),
                launch_config: Some(SessionLaunchConfig::Ssh(config.clone())),
                kind: SessionKind::Ssh,
                ai_execution_profile,
                custom_name,
                tab_color,
                after_session_id,
                insert_index,
                seed_output,
                startup_command,
                multiplex_key: None,
                source_connection_id,
                status_message: format!("connecting to {connection_name}"),
                append_start_log: true,
            },
            cx,
        );

        let session_manager = self.session.manager.clone();
        let session_start_tx = self.session.start.sender();
        let request_id_for_worker = request_id.clone();
        std::thread::spawn(move || {
            let worker_started_at = Instant::now();
            let result = session_manager
                .create_ssh_session(config.clone())
                .map(|info| SessionStartSuccess {
                    session_info: info,
                    multiplex_handle: None,
                    launch_config: Some(SessionLaunchConfig::Ssh(config)),
                })
                .map_err(|error| error.to_string());
            let worker_finished_at = Instant::now();
            let _ = session_start_tx.send(SessionStartResult {
                request_id: request_id_for_worker,
                connection_name,
                kind: SessionKind::Ssh,
                worker_started_at,
                worker_finished_at,
                result,
            });
        });
    }

    pub(in crate::features) fn begin_background_multiplex_ssh_start(
        &mut self,
        connection_name: String,
        mut config: SshSessionConfig,
        source_connection_id: Option<String>,
        ai_execution_profile: AiExecutionProfile,
        custom_name: Option<String>,
        tab_color: Option<u32>,
        after_session_id: Option<String>,
        startup_command: Option<StartupCommandRequest>,
        existing_multiplex: Option<SshMultiplexHandle>,
        cx: &mut Context<Self>,
    ) {
        config.deferred_pty = true;
        let geometry_session_hint = after_session_id
            .as_deref()
            .or(self.session.start.reconnect_target());
        if let Some(geometry) =
            self.desired_terminal_resize_geometry_for_session_hint(geometry_session_hint)
        {
            config.cols = geometry.cols;
            config.rows = geometry.rows;
            config.pixel_width = geometry.pixel_width;
            config.pixel_height = geometry.pixel_height;
        }
        let multiplex_key = ssh_multiplex_key(&config);
        let request_id = self.register_pending_session_start(
            PendingSessionStartRegistration {
                connection_name: connection_name.clone(),
                launch_config: Some(SessionLaunchConfig::Ssh(config.clone())),
                kind: SessionKind::Ssh,
                ai_execution_profile,
                custom_name,
                tab_color,
                after_session_id,
                insert_index: None,
                seed_output: None,
                startup_command,
                multiplex_key: Some(multiplex_key.clone()),
                source_connection_id,
                status_message: format!("multiplexing SSH session {connection_name}"),
                append_start_log: false,
            },
            cx,
        );

        let session_manager = self.session.manager.clone();
        let session_start_tx = self.session.start.sender();
        let request_id_for_worker = request_id.clone();
        std::thread::spawn(move || {
            let worker_started_at = Instant::now();
            let result = (|| {
                let multiplex = match existing_multiplex {
                    Some(handle) if !handle.is_closed() => handle,
                    _ => open_ssh_multiplex_handle(config.clone())
                        .map_err(|error| error.to_string())?,
                };
                let info = session_manager
                    .create_ssh_session_with_multiplex(config.clone(), multiplex.clone())
                    .map_err(|error| error.to_string())?;
                Ok(SessionStartSuccess {
                    session_info: info,
                    multiplex_handle: Some(multiplex),
                    launch_config: Some(SessionLaunchConfig::Ssh(config)),
                })
            })();
            let worker_finished_at = Instant::now();
            let _ = session_start_tx.send(SessionStartResult {
                request_id: request_id_for_worker,
                connection_name,
                kind: SessionKind::Ssh,
                worker_started_at,
                worker_finished_at,
                result,
            });
        });
    }

    pub(in crate::features) fn send_probe_command(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.session.active_id.clone() else {
            self.terminal.view.status = "start a session first".to_string();
            cx.notify();
            return;
        };
        if self.is_session_disconnected(&session_id) {
            self.terminal.view.status =
                "session disconnected — reconnect before probing".to_string();
            cx.notify();
            return;
        }

        let command = if cfg!(target_os = "windows") {
            "echo nyaterm-app-ready\r\n"
        } else {
            "printf 'nyaterm-app-ready\\n'\n"
        };
        match self.write_session_input_recorded(&session_id, command.as_bytes()) {
            Ok(()) => {
                self.terminal.view.status = "probe command sent".to_string();
            }
            Err(error) => {
                self.terminal.view.status = format!("write failed: {error}");
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn drain_session_start_events(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut dirty = false;
        for _ in 0..SESSION_START_EVENT_DRAIN_LIMIT {
            let Ok(event) = self.session.start.try_recv() else {
                break;
            };
            dirty = true;
            let request_id = event.request_id.clone();
            let (pending, was_active_pending) =
                match self.session.start.take_event_request(&event.request_id) {
                    SessionStartEventRequest::Cancelled => {
                        if let Ok(success) = event.result {
                            let session_id = success.session_info.id;
                            if let Err(error) = self.session.manager.close(&session_id) {
                                tracing::warn!(
                                    request_id = %request_id,
                                    session_id = %session_id,
                                    error = %error,
                                    "failed to close cancelled session start result"
                                );
                            }
                        }
                        tracing::debug!(
                            request_id = %request_id,
                            "discarded cancelled session start result"
                        );
                        continue;
                    }
                    SessionStartEventRequest::Pending {
                        pending,
                        was_active,
                    } => (pending, was_active),
                };
            let connection_name = event.connection_name.clone();
            let kind = pending
                .as_ref()
                .map(|pending| pending.kind)
                .unwrap_or(event.kind);
            let requested_at = pending.as_ref().map(|pending| pending.requested_at);
            let worker_duration = event
                .worker_finished_at
                .saturating_duration_since(event.worker_started_at);
            let worker_to_ui_duration =
                Instant::now().saturating_duration_since(event.worker_finished_at);
            match event.result {
                Ok(success) => {
                    let ui_register_started_at = Instant::now();
                    self.shell.chrome.last_connect_failure_name = None;
                    self.shell.chrome.last_connect_failure_error = None;
                    let session_info = success.session_info;
                    let session_id = session_info.id.clone();
                    let reconnect_session_id = pending
                        .as_ref()
                        .and_then(|pending| pending.reconnect_session_id.clone());
                    if reconnect_session_id
                        .as_deref()
                        .is_some_and(|stale_id| !self.session.metadata.contains_key(stale_id))
                    {
                        self.session.start.clear_reconnect_target();
                        if let Err(error) = self.session.manager.close(&session_id) {
                            tracing::warn!(
                                request_id = %request_id,
                                session_id = %session_id,
                                error = %error,
                                "failed to close stale reconnect result"
                            );
                        }
                        continue;
                    }
                    let launch_config = success
                        .launch_config
                        .or_else(|| {
                            pending
                                .as_ref()
                                .and_then(|pending| pending.launch_config.clone())
                        })
                        .unwrap_or_else(|| launch_config_for_session_info(&session_info));
                    let ssh_config = match &launch_config {
                        SessionLaunchConfig::Ssh(config) => Some(config.clone()),
                        _ => None,
                    };
                    let ssh_multiplex_key = pending
                        .as_ref()
                        .and_then(|pending| pending.multiplex_key.clone());
                    if let (Some(key), Some(handle)) =
                        (ssh_multiplex_key.clone(), success.multiplex_handle)
                    {
                        self.session.multiplex_handles.insert(key, handle);
                    }
                    let source_connection_id = pending
                        .as_ref()
                        .and_then(|pending| pending.source_connection_id.clone());
                    if let Some(connection_id) = source_connection_id.as_deref() {
                        match ConnectionStore::open_with_portable_key_path(
                            self.runtime.config_dir(),
                            self.runtime.portable_key_path().map(ToOwned::to_owned),
                        )
                        .and_then(|store| {
                            store.mark_connection_used(connection_id)?;
                            store.get_connection(connection_id)
                        }) {
                            Ok(Some(updated)) => {
                                if let Some(connection) = self
                                    .connection_catalog
                                    .connections
                                    .iter_mut()
                                    .find(|connection| connection.id == connection_id)
                                {
                                    *connection = updated;
                                }
                            }
                            Ok(None) => {}
                            Err(error) => tracing::warn!(
                                connection_id,
                                error = %error,
                                "failed to record recently used connection"
                            ),
                        }
                    }
                    let ai_execution_profile = pending
                        .as_ref()
                        .map(|pending| pending.ai_execution_profile)
                        .unwrap_or(AiExecutionProfile::SendOnly);
                    self.register_session(
                        &session_id,
                        SessionRuntimeMetadata {
                            ssh_config,
                            ssh_multiplex_key,
                            source_connection_id,
                            ai_execution_profile,
                            launch_config,
                            disconnected: false,
                        },
                    );
                    if let Some(custom_name) = pending
                        .as_ref()
                        .and_then(|pending| pending.custom_name.clone())
                    {
                        self.session
                            .custom_names
                            .insert(session_id.clone(), custom_name);
                    }
                    if let Some(tab_color) = pending.as_ref().and_then(|pending| pending.tab_color)
                    {
                        self.session
                            .tab_colors
                            .insert(session_id.clone(), tab_color);
                    }
                    if let Some(seed_output) = pending
                        .as_ref()
                        .and_then(|pending| pending.seed_output.clone())
                    {
                        self.seed_terminal_frame_session(&session_id, seed_output.clone());
                        self.terminal.view.views.insert(
                            session_id.clone(),
                            TerminalViewState::from_output_with_encoding(
                                seed_output,
                                &self.settings.summary.interaction_default_encoding,
                            ),
                        );
                    }
                    if let Some(after_session_id) = pending
                        .as_ref()
                        .and_then(|pending| pending.after_session_id.clone())
                    {
                        self.move_session_after(&session_id, &after_session_id);
                    }
                    if let Some(insert_index) =
                        pending.as_ref().and_then(|pending| pending.insert_index)
                    {
                        self.move_session_to_index(&session_id, insert_index);
                    }
                    if let Some(stale_id) = reconnect_session_id {
                        if stale_id != session_id {
                            self.migrate_reconnected_session_state(&stale_id, &session_id);
                            self.remove_session_state(&stale_id);
                            self.persist_workspace_pane_layout();
                            self.persist_terminal_window_layout();
                        }
                    }
                    let should_activate = self.session.start.complete_success(
                        pending
                            .as_ref()
                            .is_some_and(|pending| pending.reconnect_session_id.is_some()),
                        was_active_pending,
                        self.session.active_id.is_none(),
                    );
                    if should_activate {
                        self.activate_session_id(&session_id);
                    }
                    // First connected frames often land with a login banner burst.
                    // Enter degraded paint immediately so tab-strip/status repaint
                    // does not stack full terminal decorations on connect.
                    self.enter_connect_settle();
                    if let Some(view) = self.terminal.view.views.get_mut(&session_id) {
                        view.enter_render_degraded_mode();
                    }
                    self.terminal.view.status = format!(
                        "running {} · {}",
                        short_id(&session_id),
                        event.connection_name
                    );
                    // Do not append local log text through the full terminal decode path
                    // on connect success — that competes with the first SSH/PTY frames.
                    // Auto-recording file open is deferred to the idle plane.
                    if self.settings.summary.recording_auto_start {
                        self.recording.pending_auto_start =
                            Some((session_id.clone(), session_info.name.clone()));
                    }
                    if let Some(startup_command) =
                        pending.and_then(|pending| pending.startup_command)
                    {
                        self.schedule_startup_command(session_id.clone(), startup_command, cx);
                    }
                    self.apply_pending_workspace_split_for_duplicate(&session_id);
                    self.shell.navigation.selected_nav = NavItem::Workspace;
                    let ui_register_duration = ui_register_started_at.elapsed();
                    let request_to_ui_duration = requested_at
                        .map(|requested_at| requested_at.elapsed())
                        .unwrap_or(worker_duration + worker_to_ui_duration + ui_register_duration);
                    tracing::debug!(
                        diagnostic = "session_start",
                        request_id = %request_id,
                        connection_name = %connection_name,
                        kind = session_kind_label(kind),
                        session_id = %session_id,
                        worker_duration_ms = worker_duration.as_millis(),
                        worker_to_ui_ms = worker_to_ui_duration.as_millis(),
                        ui_register_ms = ui_register_duration.as_millis(),
                        request_to_ui_ms = request_to_ui_duration.as_millis(),
                        "session start completed"
                    );
                    if (worker_duration >= SESSION_START_SLOW_THRESHOLD
                        || request_to_ui_duration >= SESSION_START_SLOW_THRESHOLD
                        || ui_register_duration >= SESSION_START_SLOW_THRESHOLD)
                        && self.should_log_slow_diagnostic("session_start", Instant::now())
                    {
                        tracing::warn!(
                            diagnostic = "session_start",
                            request_id = %request_id,
                            connection_name = %connection_name,
                            kind = session_kind_label(kind),
                            session_id = %session_id,
                            worker_duration_ms = worker_duration.as_millis(),
                            worker_to_ui_ms = worker_to_ui_duration.as_millis(),
                            ui_register_ms = ui_register_duration.as_millis(),
                            request_to_ui_ms = request_to_ui_duration.as_millis(),
                            "slow session start"
                        );
                    }
                }
                Err(error) => {
                    let reconnect_session_id = pending
                        .as_ref()
                        .and_then(|pending| pending.reconnect_session_id.clone());
                    let reconnect_session_exists = reconnect_session_id
                        .as_deref()
                        .is_some_and(|session_id| self.session.metadata.contains_key(session_id));
                    let reconnect_failure = self.session.start.record_failure(
                        request_id.clone(),
                        pending,
                        error.clone(),
                        was_active_pending,
                        reconnect_session_exists,
                    );
                    if !reconnect_failure {
                        self.shell.chrome.last_connect_failure_name = Some(connection_name.clone());
                        self.shell.chrome.last_connect_failure_error = Some(error.clone());
                    }
                    self.terminal.view.status =
                        format!("failed to start {connection_name}: {error}");
                    if self.session.active_id.is_none() {
                        self.append_terminal_log(format!(
                            "\n# failed to start {}: {error}\n",
                            connection_name
                        ));
                    }
                    self.shell.navigation.selected_nav = NavItem::Workspace;
                    let request_to_ui_duration = requested_at
                        .map(|requested_at| requested_at.elapsed())
                        .unwrap_or(worker_duration + worker_to_ui_duration);
                    tracing::warn!(
                        diagnostic = "session_start",
                        request_id = %request_id,
                        connection_name = %connection_name,
                        kind = session_kind_label(kind),
                        worker_duration_ms = worker_duration.as_millis(),
                        worker_to_ui_ms = worker_to_ui_duration.as_millis(),
                        request_to_ui_ms = request_to_ui_duration.as_millis(),
                        error = %error,
                        "session start failed"
                    );
                }
            }
        }
        dirty
    }
}

fn session_kind_for_launch_config(config: &SessionLaunchConfig) -> SessionKind {
    match config {
        SessionLaunchConfig::Local(_) => SessionKind::LocalPty,
        SessionLaunchConfig::Ssh(_) => SessionKind::Ssh,
        SessionLaunchConfig::Telnet(config) if config.raw_tcp => SessionKind::RawTcp,
        SessionLaunchConfig::Telnet(_) => SessionKind::Telnet,
        SessionLaunchConfig::Serial(_) => SessionKind::Serial,
    }
}

const SESSION_START_SLOW_THRESHOLD: Duration = Duration::from_millis(500);

fn create_session_from_launch_config(
    session_manager: &SessionManager,
    launch_config: SessionLaunchConfig,
) -> Result<SessionInfo, nyaterm_transport::SessionError> {
    match launch_config {
        SessionLaunchConfig::Local(config) => session_manager.create_local_session(config),
        SessionLaunchConfig::Ssh(config) => session_manager.create_ssh_session(config),
        SessionLaunchConfig::Telnet(config) => session_manager.create_telnet_session(config),
        SessionLaunchConfig::Serial(config) => session_manager.create_serial_session(config),
    }
}

fn launch_config_for_session_info(info: &SessionInfo) -> SessionLaunchConfig {
    match info.kind {
        SessionKind::LocalPty => SessionLaunchConfig::Local(LocalSessionConfig {
            name: info.name.clone(),
            shell_path: None,
            shell_args: Vec::new(),
            working_dir: info.working_dir.clone(),
            cols: info.cols,
            rows: info.rows,
            pixel_width: 0,
            pixel_height: 0,
        }),
        SessionKind::Ssh => SessionLaunchConfig::Ssh(SshSessionConfig::default()),
        SessionKind::Telnet | SessionKind::RawTcp => {
            SessionLaunchConfig::Telnet(TelnetSessionConfig {
                name: info.name.clone(),
                host: String::new(),
                port: 23,
                raw_tcp: info.kind == SessionKind::RawTcp,
                enter_mode: nyaterm_transport::TelnetEnterMode::default(),
                force_character_at_a_time: false,
                send_naws: false,
                send_sga: false,
                cols: info.cols,
                rows: info.rows,
            })
        }
        SessionKind::Serial => SessionLaunchConfig::Serial(SerialSessionConfig {
            name: info.name.clone(),
            port_name: String::new(),
            baud_rate: 9600,
            data_bits: 8,
            parity: "none".to_string(),
            stop_bits: "1".to_string(),
            backspace_mode: "delete".to_string(),
        }),
    }
}

//! Grouped Remote page state: Docker, process table and host stats.
//!
//! Each of the three panes owns the same shape of refresh bookkeeping (job id,
//! owning session, pending flag, failure streak, last refresh instant) plus its
//! own view state. Keeping them in one struct per pane makes that symmetry
//! visible instead of spreading fifty-five prefixed fields across `NyaTermApp`.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::time::Instant;

use nyaterm_transport::{
    DockerComposeService, DockerContainerDetails, RemoteDockerOverview, RemoteProcess, RemoteStats,
};

use crate::features::formatting::docker_compose_project_key;
use crate::features::{DockerJobResult, ProcessJobResult, StatsJobResult};
use crate::models::{
    DockerConfirmState, DockerTab, RemoteProcessSignalConfirmState, RemoteProcessSortDirection,
    RemoteProcessSortKey,
};

pub(super) struct RemoteJobTicket<Event> {
    pub job_id: u64,
    pub tx: mpsc::Sender<Event>,
}

struct RemoteJobState<Event> {
    tx: mpsc::Sender<Event>,
    rx: mpsc::Receiver<Event>,
    pending: bool,
    job_id: u64,
    session_id: Option<String>,
    consecutive_refresh_failures: u8,
    last_refresh_at: Option<Instant>,
}

impl<Event> RemoteJobState<Event> {
    fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            tx,
            rx,
            pending: false,
            job_id: 0,
            session_id: None,
            consecutive_refresh_failures: 0,
            last_refresh_at: None,
        }
    }

    fn is_pending(&self) -> bool {
        self.pending
    }

    fn is_pending_for(&self, session_id: &str) -> bool {
        self.pending && self.session_id.as_deref() == Some(session_id)
    }

    fn last_refresh_at(&self) -> Option<Instant> {
        self.last_refresh_at
    }

    fn consecutive_refresh_failures(&self) -> u8 {
        self.consecutive_refresh_failures
    }

    fn begin(&mut self, session_id: String) -> RemoteJobTicket<Event> {
        self.job_id = self.job_id.wrapping_add(1).max(1);
        self.session_id = Some(session_id);
        self.pending = true;
        RemoteJobTicket {
            job_id: self.job_id,
            tx: self.tx.clone(),
        }
    }

    fn mark_refresh_started(&mut self) {
        self.last_refresh_at = Some(Instant::now());
    }

    fn try_recv(&self) -> Option<Event> {
        self.rx.try_recv().ok()
    }

    fn complete_if_matches(&mut self, job_id: u64, session_id: &str) -> bool {
        if self.job_id != job_id || self.session_id.as_deref() != Some(session_id) {
            return false;
        }
        self.pending = false;
        self.session_id = None;
        true
    }

    fn reset_refresh_failures(&mut self) {
        self.consecutive_refresh_failures = 0;
    }

    fn record_refresh_failure(&mut self, terminal: bool) -> u8 {
        self.consecutive_refresh_failures = if terminal {
            3
        } else {
            self.consecutive_refresh_failures.saturating_add(1)
        };
        self.consecutive_refresh_failures
    }

    fn reset_for_session_switch(&mut self) {
        self.pending = false;
        self.session_id = None;
        self.consecutive_refresh_failures = 0;
        self.last_refresh_at = None;
    }
}

pub(in crate::features) struct RemoteOpsFeatureState {
    pub docker: DockerPaneState,
    pub process: ProcessPaneState,
    pub stats: StatsPaneState,
}

/// Focus handles the Remote page needs at construction time.
pub(in crate::features) struct RemoteOpsFeatureFocus {}

pub(in crate::features) struct DockerPaneState {
    job: RemoteJobState<DockerJobResult>,
    pub overview: Option<RemoteDockerOverview>,
    pub status: String,
    pub details: Option<DockerContainerDetails>,
    pub details_container_id: Option<String>,
    pub details_last_refresh_at: Option<Instant>,
    pub confirm: Option<DockerConfirmState>,
    pub container_menu_id: Option<String>,
    pub compose_menu_id: Option<String>,
    pub tab: DockerTab,
    pub tab_menu_open: bool,
    pub header_menu_open: bool,
    pub search_draft: String,
    pub compose_expanded: HashSet<String>,
    pub compose_services: HashMap<String, Vec<DockerComposeService>>,
    pub compose_service_errors: HashMap<String, String>,
    pub list_offset: usize,
    pub resource_list_offset: usize,
}

pub(in crate::features) struct ProcessPaneState {
    job: RemoteJobState<ProcessJobResult>,
    pub items: Vec<RemoteProcess>,
    pub snapshot_loaded: bool,
    pub status: String,
    pub search_draft: String,
    pub sort_key: RemoteProcessSortKey,
    pub sort_direction: RemoteProcessSortDirection,
    pub list_offset: usize,
    pub selected_pid: Option<u32>,
    pub menu_pid: Option<u32>,
    pub nice_draft: String,
    pub signal_confirm: Option<RemoteProcessSignalConfirmState>,
}

pub(in crate::features) struct StatsPaneState {
    job: RemoteJobState<StatsJobResult>,
    pub data: Option<RemoteStats>,
    pub status: String,
    pub cpu_expanded: bool,
}

impl RemoteOpsFeatureState {
    pub(in crate::features) fn new(_focus: RemoteOpsFeatureFocus) -> Self {
        Self {
            docker: DockerPaneState {
                job: RemoteJobState::new(),
                overview: None,
                status: "start an SSH session to inspect Docker".to_string(),
                details: None,
                details_container_id: None,
                details_last_refresh_at: None,
                confirm: None,
                container_menu_id: None,
                compose_menu_id: None,
                tab: DockerTab::Containers,
                tab_menu_open: false,
                header_menu_open: false,
                search_draft: String::new(),
                compose_expanded: HashSet::new(),
                compose_services: HashMap::new(),
                compose_service_errors: HashMap::new(),
                list_offset: 0,
                resource_list_offset: 0,
            },
            process: ProcessPaneState {
                job: RemoteJobState::new(),
                items: Vec::new(),
                snapshot_loaded: false,
                status: "ready".to_string(),
                search_draft: String::new(),
                sort_key: RemoteProcessSortKey::Cpu,
                sort_direction: RemoteProcessSortDirection::Descending,
                list_offset: 0,
                selected_pid: None,
                menu_pid: None,
                nice_draft: "0".to_string(),
                signal_confirm: None,
            },
            stats: StatsPaneState {
                job: RemoteJobState::new(),
                data: None,
                status: "start an SSH session to inspect remote stats".to_string(),
                cpu_expanded: false,
            },
        }
    }

    pub(in crate::features) fn reset_for_session_switch(&mut self) {
        self.process.reset_for_session_switch();
        self.stats.reset_for_session_switch();
        self.docker.reset_for_session_switch();
    }
}

impl DockerPaneState {
    pub(in crate::features) fn is_pending(&self) -> bool {
        self.job.is_pending()
    }

    pub(in crate::features) fn last_refresh_at(&self) -> Option<Instant> {
        self.job.last_refresh_at()
    }

    pub(super) fn is_pending_for(&self, session_id: &str) -> bool {
        self.job.is_pending_for(session_id)
    }

    pub(super) fn begin_job(&mut self, session_id: String) -> RemoteJobTicket<DockerJobResult> {
        self.job.begin(session_id)
    }

    pub(super) fn mark_refresh_started(&mut self) {
        self.job.mark_refresh_started();
    }

    pub(super) fn next_event(&self) -> Option<DockerJobResult> {
        self.job.try_recv()
    }

    pub(super) fn complete_event(&mut self, job_id: u64, session_id: &str) -> bool {
        self.job.complete_if_matches(job_id, session_id)
    }

    pub(super) fn reset_refresh_failures(&mut self) {
        self.job.reset_refresh_failures();
    }

    pub(super) fn record_refresh_failure(&mut self) -> u8 {
        self.job.record_refresh_failure(false)
    }

    pub(in crate::features) fn set_tab(&mut self, tab: DockerTab) {
        self.container_menu_id = None;
        self.compose_menu_id = None;
        self.tab_menu_open = false;
        self.header_menu_open = false;
        if tab == DockerTab::Compose
            && self
                .overview
                .as_ref()
                .is_some_and(|overview| !overview.compose_available)
        {
            self.status = "Docker Compose is not available on this host".to_string();
            return;
        }
        self.tab = tab;
        self.list_offset = 0;
        self.resource_list_offset = 0;
        self.status = format!("Docker tab: {}", tab.label());
    }

    pub(in crate::features) fn toggle_tab_menu(&mut self) {
        self.tab_menu_open = !self.tab_menu_open;
    }

    pub(in crate::features) fn apply_search(&mut self, text: String) {
        self.search_draft = text;
        self.list_offset = 0;
        self.resource_list_offset = 0;
        self.status = "Docker search updated".to_string();
    }

    pub(in crate::features) fn close_details(&mut self) {
        self.details = None;
        self.details_container_id = None;
        self.details_last_refresh_at = None;
        self.status = "container details closed".to_string();
    }

    pub(in crate::features) fn request_confirm(&mut self, confirm: DockerConfirmState) {
        self.confirm = Some(confirm);
        self.status = "confirm Docker operation".to_string();
    }

    pub(in crate::features) fn cancel_confirm(&mut self) {
        self.confirm = None;
        self.status = "Docker operation cancelled".to_string();
    }

    pub(in crate::features) fn apply_overview(&mut self, overview: RemoteDockerOverview) {
        if let Some(details_id) = self.details_container_id.as_deref()
            && !overview
                .containers
                .iter()
                .any(|container| container.id == details_id)
        {
            self.details = None;
            self.details_container_id = None;
        }
        let active_compose_keys = overview
            .compose_projects
            .iter()
            .map(|project| {
                docker_compose_project_key(&project.name, Some(project.config_files.as_str()))
            })
            .collect::<HashSet<_>>();
        self.compose_expanded
            .retain(|key| active_compose_keys.contains(key));
        self.compose_services
            .retain(|key, _| active_compose_keys.contains(key));
        self.compose_service_errors
            .retain(|key, _| active_compose_keys.contains(key));
        self.overview = Some(overview);
    }

    fn reset_for_session_switch(&mut self) {
        self.job.reset_for_session_switch();
        self.overview = None;
        self.details = None;
        self.details_container_id = None;
        self.details_last_refresh_at = None;
        self.confirm = None;
        self.container_menu_id = None;
        self.compose_menu_id = None;
        self.compose_services.clear();
        self.compose_service_errors.clear();
        self.status = "start an SSH session to inspect Docker".to_string();
    }
}

impl ProcessPaneState {
    pub(in crate::features) fn is_pending(&self) -> bool {
        self.job.is_pending()
    }

    pub(in crate::features) fn last_refresh_at(&self) -> Option<Instant> {
        self.job.last_refresh_at()
    }

    pub(super) fn is_pending_for(&self, session_id: &str) -> bool {
        self.job.is_pending_for(session_id)
    }

    pub(super) fn begin_job(&mut self, session_id: String) -> RemoteJobTicket<ProcessJobResult> {
        self.job.begin(session_id)
    }

    pub(super) fn mark_refresh_started(&mut self) {
        self.job.mark_refresh_started();
    }

    pub(super) fn next_event(&self) -> Option<ProcessJobResult> {
        self.job.try_recv()
    }

    pub(super) fn complete_event(&mut self, job_id: u64, session_id: &str) -> bool {
        self.job.complete_if_matches(job_id, session_id)
    }

    pub(super) fn reset_refresh_failures(&mut self) {
        self.job.reset_refresh_failures();
    }

    pub(super) fn record_refresh_failure(&mut self, terminal: bool) -> u8 {
        self.job.record_refresh_failure(terminal)
    }

    pub(in crate::features) fn apply_search(&mut self, text: String) {
        self.search_draft = text;
        self.selected_pid = None;
        self.list_offset = 0;
    }

    pub(in crate::features) fn toggle_sort(&mut self, key: RemoteProcessSortKey) {
        if self.sort_key == key {
            self.sort_direction = self.sort_direction.reversed();
        } else {
            self.sort_key = key;
            self.sort_direction = match key {
                RemoteProcessSortKey::Cpu | RemoteProcessSortKey::Memory => {
                    RemoteProcessSortDirection::Descending
                }
                RemoteProcessSortKey::Pid
                | RemoteProcessSortKey::User
                | RemoteProcessSortKey::Command => RemoteProcessSortDirection::Ascending,
            };
        }
        self.list_offset = 0;
        self.status = format!(
            "sorted processes by {} {}",
            self.sort_key.label(),
            self.sort_direction.marker()
        );
    }

    pub(in crate::features) fn toggle_selection(&mut self, pid: u32) {
        self.menu_pid = None;
        self.selected_pid = (self.selected_pid != Some(pid)).then_some(pid);
        self.nice_draft = "0".to_string();
    }

    pub(in crate::features) fn apply_nice_input(&mut self, text: String) {
        let negative = text.starts_with('-');
        let digits: String = text.chars().filter(char::is_ascii_digit).take(3).collect();
        self.nice_draft = if negative {
            format!("-{digits}")
        } else {
            digits
        };
    }

    pub(in crate::features) fn validated_nice_draft(&mut self) -> Option<(u32, i32)> {
        let Some(pid) = self.selected_pid else {
            self.status = "select a process before applying nice".to_string();
            return None;
        };
        let Ok(nice) = self.nice_draft.trim().parse::<i32>() else {
            self.status = "nice must be an integer from -20 to 19".to_string();
            return None;
        };
        if !(-20..=19).contains(&nice) {
            self.status = "nice must be between -20 and 19".to_string();
            return None;
        }
        Some((pid, nice))
    }

    pub(in crate::features) fn request_signal(&mut self, pid: u32, signal: &'static str) -> bool {
        if signal != "KILL" {
            return false;
        }
        let command = self
            .items
            .iter()
            .find(|process| process.pid == pid)
            .map(|process| process.command_line.clone())
            .filter(|command| !command.trim().is_empty())
            .or_else(|| {
                self.items
                    .iter()
                    .find(|process| process.pid == pid)
                    .map(|process| process.command.clone())
            })
            .unwrap_or_else(|| "unknown process".to_string());
        self.signal_confirm = Some(RemoteProcessSignalConfirmState {
            pid,
            signal,
            command,
        });
        self.status = format!("confirm {signal} for pid {pid}");
        true
    }

    pub(in crate::features) fn cancel_signal_confirm(&mut self) {
        self.signal_confirm = None;
        self.status = "process signal cancelled".to_string();
    }

    pub(in crate::features) fn take_signal_confirm(
        &mut self,
    ) -> Option<RemoteProcessSignalConfirmState> {
        let confirm = self.signal_confirm.take();
        if confirm.is_none() {
            self.status = "no process signal pending".to_string();
        }
        confirm
    }

    pub(in crate::features) fn apply_processes(&mut self, processes: Vec<RemoteProcess>) {
        if self
            .selected_pid
            .is_some_and(|pid| !processes.iter().any(|process| process.pid == pid))
        {
            self.selected_pid = None;
            self.nice_draft = "0".to_string();
        }
        self.items = processes;
        self.snapshot_loaded = true;
    }

    fn reset_for_session_switch(&mut self) {
        self.job.reset_for_session_switch();
        self.items.clear();
        self.snapshot_loaded = false;
        self.selected_pid = None;
        self.menu_pid = None;
        self.signal_confirm = None;
        self.status = "ready".to_string();
    }
}

impl StatsPaneState {
    pub(in crate::features) fn is_pending(&self) -> bool {
        self.job.is_pending()
    }

    pub(in crate::features) fn last_refresh_at(&self) -> Option<Instant> {
        self.job.last_refresh_at()
    }

    pub(in crate::features) fn consecutive_refresh_failures(&self) -> u8 {
        self.job.consecutive_refresh_failures()
    }

    pub(super) fn is_pending_for(&self, session_id: &str) -> bool {
        self.job.is_pending_for(session_id)
    }

    pub(super) fn begin_job(&mut self, session_id: String) -> RemoteJobTicket<StatsJobResult> {
        self.job.begin(session_id)
    }

    pub(super) fn mark_refresh_started(&mut self) {
        self.job.mark_refresh_started();
    }

    pub(super) fn next_event(&self) -> Option<StatsJobResult> {
        self.job.try_recv()
    }

    pub(super) fn complete_event(&mut self, job_id: u64, session_id: &str) -> bool {
        self.job.complete_if_matches(job_id, session_id)
    }

    pub(super) fn reset_refresh_failures(&mut self) {
        self.job.reset_refresh_failures();
    }

    pub(super) fn record_refresh_failure(&mut self) -> u8 {
        self.job.record_refresh_failure(false)
    }

    pub(in crate::features) fn toggle_cpu_expanded(&mut self) {
        self.cpu_expanded = !self.cpu_expanded;
        self.status = if self.cpu_expanded {
            "showing per-core CPU usage".to_string()
        } else {
            "collapsed per-core CPU usage".to_string()
        };
    }

    fn reset_for_session_switch(&mut self) {
        self.job.reset_for_session_switch();
        self.data = None;
        self.status = "start an SSH session to inspect remote stats".to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::{RemoteJobState, RemoteOpsFeatureFocus, RemoteOpsFeatureState};

    #[test]
    fn remote_job_state_matches_job_and_session_before_completion() {
        let mut state = RemoteJobState::<u8>::new();
        let first = state.begin("session-a".to_string());
        first.tx.send(7).expect("receiver should stay owned");

        let second = state.begin("session-b".to_string());

        assert_eq!(state.try_recv(), Some(7));
        assert!(!state.complete_if_matches(first.job_id, "session-a"));
        assert!(state.is_pending_for("session-b"));
        assert!(!state.complete_if_matches(second.job_id, "session-a"));
        assert!(state.complete_if_matches(second.job_id, "session-b"));
        assert!(!state.is_pending());
    }

    #[test]
    fn remote_job_state_tracks_refresh_failures_and_resets_session_runtime() {
        let mut state = RemoteJobState::<u8>::new();
        state.begin("session-a".to_string());
        state.mark_refresh_started();
        assert_eq!(state.record_refresh_failure(false), 1);
        assert_eq!(state.record_refresh_failure(true), 3);

        state.reset_for_session_switch();

        assert!(!state.is_pending());
        assert!(state.last_refresh_at().is_none());
        assert_eq!(state.consecutive_refresh_failures(), 0);
    }

    #[test]
    fn remote_pane_transitions_keep_related_fields_in_sync() {
        let mut state = RemoteOpsFeatureState::new(RemoteOpsFeatureFocus {});
        state.process.selected_pid = Some(42);
        state.process.apply_nice_input("-1234x".to_string());
        assert_eq!(state.process.nice_draft, "-123");
        assert_eq!(state.process.validated_nice_draft(), None);
        assert_eq!(state.process.status, "nice must be between -20 and 19");

        state.process.toggle_selection(7);
        assert_eq!(state.process.selected_pid, Some(7));
        assert_eq!(state.process.nice_draft, "0");

        state.stats.toggle_cpu_expanded();
        assert!(state.stats.cpu_expanded);
        assert_eq!(state.stats.status, "showing per-core CPU usage");

        state.reset_for_session_switch();
        assert!(state.process.selected_pid.is_none());
        assert!(!state.stats.is_pending());
        assert!(state.docker.overview.is_none());
    }
}

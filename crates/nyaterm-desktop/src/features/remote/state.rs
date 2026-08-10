//! Grouped Remote page state: Docker, process table and host stats.
//!
//! Each of the three panes owns the same shape of refresh bookkeeping (job id,
//! owning session, pending flag, failure streak, last refresh instant) plus its
//! own view state. Keeping them in one struct per pane makes that symmetry
//! visible instead of spreading fifty-five prefixed fields across `NyaTermApp`.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, mpsc};
use std::time::Instant;

use nyaterm_transport::{
    DockerComposeService, DockerContainerDetails, RemoteDockerOverview, RemoteGpuOverview,
    RemoteNpuOverview, RemoteProcess, RemoteStats,
};

use crate::features::formatting::docker_compose_project_key;
use crate::features::{
    DockerJobResult, GpuJobResult, NpuJobResult, ProcessJobResult, StatsJobResult,
};
use crate::models::{DockerTab, RemoteProcessSortDirection, RemoteProcessSortKey};

pub(in crate::features) struct RemoteJobTicket<Event> {
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
    docker: DockerPaneState,
    process: ProcessPaneState,
    stats: StatsPaneState,
    gpu: AcceleratorPaneState<RemoteGpuOverview, GpuJobResult>,
    npu: AcceleratorPaneState<RemoteNpuOverview, NpuJobResult>,
}

/// Focus handles the Remote page needs at construction time.
pub(in crate::features) struct RemoteOpsFeatureFocus {}

struct DockerPaneState {
    job: RemoteJobState<DockerJobResult>,
    pub overview: Option<RemoteDockerOverview>,
    pub status: String,
    pub details: Option<DockerContainerDetails>,
    pub details_container_id: Option<String>,
    pub details_last_refresh_at: Option<Instant>,
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

struct ProcessPaneState {
    job: RemoteJobState<ProcessJobResult>,
    pub items: Arc<[RemoteProcess]>,
    pub snapshot_loaded: bool,
    pub status: String,
    pub search_draft: String,
    pub sort_key: RemoteProcessSortKey,
    pub sort_direction: RemoteProcessSortDirection,
    pub list_offset: usize,
    pub selected_pid: Option<u32>,
    pub menu_pid: Option<u32>,
    pub nice_draft: String,
}

struct StatsPaneState {
    job: RemoteJobState<StatsJobResult>,
    pub data: Option<RemoteStats>,
    pub status: String,
    pub cpu_expanded: bool,
}

struct AcceleratorPaneState<Data, Event> {
    job: RemoteJobState<Event>,
    pub data: Option<Data>,
    pub status: String,
    unavailable_sessions: HashSet<String>,
}

#[derive(Clone)]
pub(in crate::features) struct DockerPresentationState {
    pub overview: Option<RemoteDockerOverview>,
    pub status: String,
    pub details: Option<DockerContainerDetails>,
    pub details_container_id: Option<String>,
    pub container_menu_id: Option<String>,
    pub compose_menu_id: Option<String>,
    pub tab: DockerTab,
    pub tab_menu_open: bool,
    pub search_draft: String,
    pub compose_expanded: HashSet<String>,
    pub compose_services: HashMap<String, Vec<DockerComposeService>>,
    pub compose_service_errors: HashMap<String, String>,
    pub list_offset: usize,
    pub resource_list_offset: usize,
    pub pending: bool,
}

#[derive(Clone)]
pub(in crate::features) struct ProcessPresentationState {
    pub items: Arc<[RemoteProcess]>,
    pub snapshot_loaded: bool,
    pub status: String,
    pub search_draft: String,
    pub sort_key: RemoteProcessSortKey,
    pub sort_direction: RemoteProcessSortDirection,
    pub list_offset: usize,
    pub selected_pid: Option<u32>,
    pub menu_pid: Option<u32>,
    pub nice_draft: String,
    pub pending: bool,
}

#[derive(Clone)]
pub(in crate::features) struct StatsPresentationState {
    pub data: Option<RemoteStats>,
    pub status: String,
    pub cpu_expanded: bool,
    pub pending: bool,
    pub consecutive_refresh_failures: u8,
}

#[derive(Clone)]
pub(in crate::features) struct GpuPresentationState {
    pub data: Option<RemoteGpuOverview>,
    pub status: String,
    pub pending: bool,
    pub consecutive_refresh_failures: u8,
}

#[derive(Clone)]
pub(in crate::features) struct NpuPresentationState {
    pub data: Option<RemoteNpuOverview>,
    pub status: String,
    pub pending: bool,
    pub consecutive_refresh_failures: u8,
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
                items: Arc::from([]),
                snapshot_loaded: false,
                status: "ready".to_string(),
                search_draft: String::new(),
                sort_key: RemoteProcessSortKey::Cpu,
                sort_direction: RemoteProcessSortDirection::Descending,
                list_offset: 0,
                selected_pid: None,
                menu_pid: None,
                nice_draft: "0".to_string(),
            },
            stats: StatsPaneState {
                job: RemoteJobState::new(),
                data: None,
                status: "start an SSH session to inspect remote stats".to_string(),
                cpu_expanded: false,
            },
            gpu: AcceleratorPaneState::new("start an SSH session to inspect NVIDIA GPU"),
            npu: AcceleratorPaneState::new("start an SSH session to inspect Ascend NPU"),
        }
    }

    pub(in crate::features) fn reset_for_session_switch(&mut self) {
        self.process.reset_for_session_switch();
        self.stats.reset_for_session_switch();
        self.docker.reset_for_session_switch();
        self.gpu
            .reset_for_session_switch("start an SSH session to inspect NVIDIA GPU");
        self.npu
            .reset_for_session_switch("start an SSH session to inspect Ascend NPU");
    }

    pub(in crate::features) fn docker_presentation(&self) -> DockerPresentationState {
        DockerPresentationState {
            overview: self.docker.overview.clone(),
            status: self.docker.status.clone(),
            details: self.docker.details.clone(),
            details_container_id: self.docker.details_container_id.clone(),
            container_menu_id: self.docker.container_menu_id.clone(),
            compose_menu_id: self.docker.compose_menu_id.clone(),
            tab: self.docker.tab,
            tab_menu_open: self.docker.tab_menu_open,
            search_draft: self.docker.search_draft.clone(),
            compose_expanded: self.docker.compose_expanded.clone(),
            compose_services: self.docker.compose_services.clone(),
            compose_service_errors: self.docker.compose_service_errors.clone(),
            list_offset: self.docker.list_offset,
            resource_list_offset: self.docker.resource_list_offset,
            pending: self.docker.is_pending(),
        }
    }

    pub(in crate::features) fn process_presentation(&self) -> ProcessPresentationState {
        ProcessPresentationState {
            items: self.process.items.clone(),
            snapshot_loaded: self.process.snapshot_loaded,
            status: self.process.status.clone(),
            search_draft: self.process.search_draft.clone(),
            sort_key: self.process.sort_key,
            sort_direction: self.process.sort_direction,
            list_offset: self.process.list_offset,
            selected_pid: self.process.selected_pid,
            menu_pid: self.process.menu_pid,
            nice_draft: self.process.nice_draft.clone(),
            pending: self.process.is_pending(),
        }
    }

    pub(in crate::features) fn stats_presentation(&self) -> StatsPresentationState {
        StatsPresentationState {
            data: self.stats.data.clone(),
            status: self.stats.status.clone(),
            cpu_expanded: self.stats.cpu_expanded,
            pending: self.stats.is_pending(),
            consecutive_refresh_failures: self.stats.consecutive_refresh_failures(),
        }
    }

    pub(in crate::features) fn gpu_presentation(&self) -> GpuPresentationState {
        GpuPresentationState {
            data: self.gpu.data.clone(),
            status: self.gpu.status.clone(),
            pending: self.gpu.is_pending(),
            consecutive_refresh_failures: self.gpu.consecutive_refresh_failures(),
        }
    }

    pub(in crate::features) fn npu_presentation(&self) -> NpuPresentationState {
        NpuPresentationState {
            data: self.npu.data.clone(),
            status: self.npu.status.clone(),
            pending: self.npu.is_pending(),
            consecutive_refresh_failures: self.npu.consecutive_refresh_failures(),
        }
    }

    pub(in crate::features) fn docker_status(&self) -> &str {
        &self.docker.status
    }

    pub(in crate::features) fn set_docker_status(&mut self, status: impl Into<String>) {
        self.docker.status = status.into();
    }

    pub(in crate::features) fn process_status(&self) -> &str {
        &self.process.status
    }

    pub(in crate::features) fn set_process_status(&mut self, status: impl Into<String>) {
        self.process.status = status.into();
    }

    pub(in crate::features) fn stats_status(&self) -> &str {
        &self.stats.status
    }

    pub(in crate::features) fn set_stats_status(&mut self, status: impl Into<String>) {
        self.stats.status = status.into();
    }

    pub(in crate::features) fn gpu_status(&self) -> &str {
        &self.gpu.status
    }

    pub(in crate::features) fn set_gpu_status(&mut self, status: impl Into<String>) {
        self.gpu.status = status.into();
    }

    pub(in crate::features) fn npu_status(&self) -> &str {
        &self.npu.status
    }

    pub(in crate::features) fn set_npu_status(&mut self, status: impl Into<String>) {
        self.npu.status = status.into();
    }

    pub(in crate::features) fn has_pending_job(&self) -> bool {
        self.docker.is_pending()
            || self.process.is_pending()
            || self.stats.is_pending()
            || self.gpu.is_pending()
            || self.npu.is_pending()
    }

    pub(in crate::features) fn loaded_process_count(&self) -> Option<usize> {
        (self.process.snapshot_loaded && !self.process.items.is_empty())
            .then_some(self.process.items.len())
    }

    pub(in crate::features) fn docker_engine_version(&self) -> Option<String> {
        let overview = self
            .docker
            .overview
            .as_ref()
            .filter(|overview| overview.available)?;
        let version = overview.version.trim();
        Some(if version.is_empty() {
            "-".to_string()
        } else {
            version.to_string()
        })
    }

    pub(in crate::features) fn docker_can_prune(&self) -> bool {
        self.docker
            .overview
            .as_ref()
            .is_some_and(|overview| overview.available)
    }

    pub(in crate::features) fn docker_header_menu_open(&self) -> bool {
        self.docker.header_menu_open
    }

    pub(in crate::features) fn docker_is_pending(&self) -> bool {
        self.docker.is_pending()
    }

    pub(in crate::features) fn process_is_pending(&self) -> bool {
        self.process.is_pending()
    }

    pub(in crate::features) fn stats_is_pending(&self) -> bool {
        self.stats.is_pending()
    }

    pub(in crate::features) fn gpu_is_pending(&self) -> bool {
        self.gpu.is_pending()
    }

    pub(in crate::features) fn npu_is_pending(&self) -> bool {
        self.npu.is_pending()
    }

    pub(in crate::features) fn docker_last_refresh_at(&self) -> Option<Instant> {
        self.docker.last_refresh_at()
    }

    pub(in crate::features) fn process_last_refresh_at(&self) -> Option<Instant> {
        self.process.last_refresh_at()
    }

    pub(in crate::features) fn stats_last_refresh_at(&self) -> Option<Instant> {
        self.stats.last_refresh_at()
    }

    pub(in crate::features) fn gpu_last_refresh_at(&self) -> Option<Instant> {
        self.gpu.last_refresh_at()
    }

    pub(in crate::features) fn npu_last_refresh_at(&self) -> Option<Instant> {
        self.npu.last_refresh_at()
    }

    pub(in crate::features) fn docker_details_refresh(&self) -> Option<(String, Instant)> {
        let refresh = (
            self.docker.details_container_id.clone()?,
            self.docker.details_last_refresh_at?,
        );
        if self.docker.details.is_some() {
            Some(refresh)
        } else {
            None
        }
    }

    pub(in crate::features) fn set_docker_tab(&mut self, tab: DockerTab) {
        self.docker.set_tab(tab);
    }

    pub(in crate::features) fn toggle_docker_tab_menu(&mut self) {
        self.docker.toggle_tab_menu();
        if self.docker.tab_menu_open {
            self.docker.header_menu_open = false;
            self.docker.container_menu_id = None;
            self.docker.compose_menu_id = None;
        }
    }

    pub(in crate::features) fn toggle_docker_header_menu(&mut self) {
        self.docker.header_menu_open = !self.docker.header_menu_open;
        if self.docker.header_menu_open {
            self.docker.tab_menu_open = false;
            self.docker.container_menu_id = None;
            self.docker.compose_menu_id = None;
        }
    }

    pub(in crate::features) fn close_docker_menus(&mut self) {
        self.docker.tab_menu_open = false;
        self.docker.header_menu_open = false;
        self.docker.container_menu_id = None;
        self.docker.compose_menu_id = None;
    }

    pub(in crate::features) fn docker_menus_open(&self) -> bool {
        self.docker.tab_menu_open || self.docker.header_menu_open
    }

    pub(in crate::features) fn toggle_docker_container_menu(&mut self, id: String) {
        let open = self.docker.container_menu_id.as_deref() != Some(id.as_str());
        self.docker.container_menu_id = open.then_some(id);
        if open {
            self.docker.tab_menu_open = false;
            self.docker.header_menu_open = false;
            self.docker.compose_menu_id = None;
        }
    }

    pub(in crate::features) fn close_docker_container_menu(&mut self) {
        self.docker.container_menu_id = None;
    }

    pub(in crate::features) fn toggle_docker_compose_menu(&mut self, id: String) {
        let open = self.docker.compose_menu_id.as_deref() != Some(id.as_str());
        self.docker.compose_menu_id = open.then_some(id);
        if open {
            self.docker.tab_menu_open = false;
            self.docker.header_menu_open = false;
            self.docker.container_menu_id = None;
        }
    }

    pub(in crate::features) fn close_docker_compose_menu(&mut self) {
        self.docker.compose_menu_id = None;
    }

    pub(in crate::features) fn apply_docker_search(&mut self, text: String) {
        self.docker.apply_search(text);
    }

    pub(in crate::features) fn clamp_docker_list_offset(&mut self, max: usize) -> usize {
        self.docker.list_offset = self.docker.list_offset.min(max);
        self.docker.list_offset
    }

    pub(in crate::features) fn set_docker_list_offset(&mut self, offset: usize) -> bool {
        if self.docker.list_offset == offset {
            return false;
        }
        self.docker.list_offset = offset;
        true
    }

    pub(in crate::features) fn clamp_docker_resource_offset(&mut self, max: usize) -> usize {
        self.docker.resource_list_offset = self.docker.resource_list_offset.min(max);
        self.docker.resource_list_offset
    }

    pub(in crate::features) fn set_docker_resource_offset(&mut self, offset: usize) -> bool {
        if self.docker.resource_list_offset == offset {
            return false;
        }
        self.docker.resource_list_offset = offset;
        true
    }

    pub(in crate::features) fn close_docker_details(&mut self) {
        self.docker.close_details();
    }

    pub(in crate::features) fn toggle_compose_project(
        &mut self,
        key: String,
        project_name: &str,
    ) -> bool {
        if self.docker.compose_expanded.remove(&key) {
            self.docker.status = format!("collapsed compose project {project_name}");
            return false;
        }
        self.docker.compose_expanded.insert(key.clone());
        self.docker.status = format!("expanded compose project {project_name}");
        !self.docker.compose_services.contains_key(&key)
            && !self.docker.compose_service_errors.contains_key(&key)
    }

    pub(in crate::features) fn apply_process_search(&mut self, text: String) {
        self.process.apply_search(text);
    }

    pub(in crate::features) fn toggle_process_sort(&mut self, key: RemoteProcessSortKey) {
        self.process.toggle_sort(key);
    }

    pub(in crate::features) fn constrain_process_sort(
        &mut self,
        allow_memory: bool,
        allow_user: bool,
    ) -> RemoteProcessSortKey {
        if (!allow_user && self.process.sort_key == RemoteProcessSortKey::User)
            || (!allow_memory && self.process.sort_key == RemoteProcessSortKey::Memory)
        {
            self.process.sort_key = RemoteProcessSortKey::Cpu;
        }
        self.process.sort_key
    }

    pub(in crate::features) fn toggle_process_selection(&mut self, pid: u32) {
        self.process.toggle_selection(pid);
    }

    pub(in crate::features) fn toggle_process_menu(&mut self, pid: u32) {
        self.process.menu_pid = (self.process.menu_pid != Some(pid)).then_some(pid);
    }

    pub(in crate::features) fn close_process_menu(&mut self) {
        self.process.menu_pid = None;
    }

    pub(in crate::features) fn clamp_process_list_offset(&mut self, max: usize) -> usize {
        self.process.list_offset = self.process.list_offset.min(max);
        self.process.list_offset
    }

    pub(in crate::features) fn set_process_list_offset(&mut self, offset: usize) -> bool {
        if self.process.list_offset == offset {
            return false;
        }
        self.process.list_offset = offset;
        true
    }

    pub(in crate::features) fn apply_process_nice_input(&mut self, text: String) {
        self.process.apply_nice_input(text);
    }

    pub(in crate::features) fn validated_process_nice_draft(&mut self) -> Option<(u32, i32)> {
        self.process.validated_nice_draft()
    }

    pub(in crate::features) fn toggle_stats_cpu_expanded(&mut self) {
        self.stats.toggle_cpu_expanded();
    }

    pub(in crate::features) fn docker_is_pending_for(&self, session_id: &str) -> bool {
        self.docker.is_pending_for(session_id)
    }

    pub(in crate::features) fn begin_docker_job(
        &mut self,
        session_id: String,
    ) -> RemoteJobTicket<DockerJobResult> {
        self.docker.begin_job(session_id)
    }

    pub(in crate::features) fn mark_docker_refresh_started(&mut self) {
        self.docker.mark_refresh_started();
    }

    pub(in crate::features) fn next_docker_event(&self) -> Option<DockerJobResult> {
        self.docker.next_event()
    }

    pub(in crate::features) fn complete_docker_event(
        &mut self,
        job_id: u64,
        session_id: &str,
    ) -> bool {
        self.docker.complete_event(job_id, session_id)
    }

    pub(in crate::features) fn start_docker_container_action(&mut self, status: String) {
        self.docker.status = status;
        self.docker.details = None;
        self.docker.details_container_id = None;
    }

    pub(in crate::features) fn start_docker_details(
        &mut self,
        container_id: String,
        status: String,
    ) {
        self.docker.details_container_id = Some(container_id);
        self.docker.details_last_refresh_at = Some(Instant::now());
        self.docker.status = status;
    }

    pub(in crate::features) fn apply_docker_overview(&mut self, overview: RemoteDockerOverview) {
        self.docker.apply_overview(overview);
    }

    pub(in crate::features) fn apply_docker_details(
        &mut self,
        container_id: String,
        details: DockerContainerDetails,
    ) {
        self.docker.details = Some(details);
        self.docker.details_container_id = Some(container_id);
    }

    pub(in crate::features) fn clear_compose_service_error(&mut self, key: &str) {
        self.docker.compose_service_errors.remove(key);
    }

    pub(in crate::features) fn set_compose_services(
        &mut self,
        key: String,
        services: Vec<DockerComposeService>,
    ) {
        self.docker.compose_service_errors.remove(&key);
        self.docker.compose_services.insert(key, services);
    }

    pub(in crate::features) fn set_compose_service_error(&mut self, key: String, error: String) {
        self.docker.compose_services.remove(&key);
        self.docker.compose_service_errors.insert(key, error);
    }

    pub(in crate::features) fn reset_docker_refresh_failures(&mut self) {
        self.docker.reset_refresh_failures();
    }

    pub(in crate::features) fn record_docker_refresh_failure(&mut self) -> u8 {
        self.docker.record_refresh_failure()
    }

    pub(in crate::features) fn clear_docker_overview(&mut self) {
        self.docker.overview = None;
    }

    pub(in crate::features) fn process_is_pending_for(&self, session_id: &str) -> bool {
        self.process.is_pending_for(session_id)
    }

    pub(in crate::features) fn begin_process_job(
        &mut self,
        session_id: String,
    ) -> RemoteJobTicket<ProcessJobResult> {
        self.process.begin_job(session_id)
    }

    pub(in crate::features) fn mark_process_refresh_started(&mut self) {
        self.process.mark_refresh_started();
    }

    pub(in crate::features) fn next_process_event(&self) -> Option<ProcessJobResult> {
        self.process.next_event()
    }

    pub(in crate::features) fn complete_process_event(
        &mut self,
        job_id: u64,
        session_id: &str,
    ) -> bool {
        self.process.complete_event(job_id, session_id)
    }

    pub(in crate::features) fn reset_process_refresh_failures(&mut self) {
        self.process.reset_refresh_failures();
    }

    pub(in crate::features) fn record_process_refresh_failure(&mut self, terminal: bool) -> u8 {
        self.process.record_refresh_failure(terminal)
    }

    pub(in crate::features) fn clear_process_data(&mut self) {
        self.process.items = Arc::from([]);
        self.process.snapshot_loaded = false;
        self.process.selected_pid = None;
        self.process.menu_pid = None;
    }

    pub(in crate::features) fn apply_processes(&mut self, processes: Vec<RemoteProcess>) {
        self.process.apply_processes(processes);
    }

    pub(in crate::features) fn stats_is_pending_for(&self, session_id: &str) -> bool {
        self.stats.is_pending_for(session_id)
    }

    pub(in crate::features) fn begin_stats_job(
        &mut self,
        session_id: String,
    ) -> RemoteJobTicket<StatsJobResult> {
        self.stats.begin_job(session_id)
    }

    pub(in crate::features) fn mark_stats_refresh_started(&mut self) {
        self.stats.mark_refresh_started();
    }

    pub(in crate::features) fn next_stats_event(&self) -> Option<StatsJobResult> {
        self.stats.next_event()
    }

    pub(in crate::features) fn complete_stats_event(
        &mut self,
        job_id: u64,
        session_id: &str,
    ) -> bool {
        self.stats.complete_event(job_id, session_id)
    }

    pub(in crate::features) fn reset_stats_refresh_failures(&mut self) {
        self.stats.reset_refresh_failures();
    }

    pub(in crate::features) fn apply_stats(&mut self, stats: RemoteStats) {
        self.stats.data = Some(stats);
    }

    pub(in crate::features) fn record_stats_refresh_failure(&mut self) -> u8 {
        let failures = self.stats.record_refresh_failure();
        if failures >= 3 {
            self.stats.data = None;
        }
        failures
    }

    pub(in crate::features) fn gpu_is_pending_for(&self, session_id: &str) -> bool {
        self.gpu.is_pending_for(session_id)
    }

    pub(in crate::features) fn gpu_unavailable_for(&self, session_id: &str) -> bool {
        self.gpu.unavailable_for(session_id)
    }

    pub(in crate::features) fn begin_gpu_job(
        &mut self,
        session_id: String,
    ) -> RemoteJobTicket<GpuJobResult> {
        self.gpu.begin_job(session_id)
    }

    pub(in crate::features) fn mark_gpu_refresh_started(&mut self) {
        self.gpu.mark_refresh_started();
    }

    pub(in crate::features) fn next_gpu_event(&self) -> Option<GpuJobResult> {
        self.gpu.next_event()
    }

    pub(in crate::features) fn complete_gpu_event(
        &mut self,
        job_id: u64,
        session_id: &str,
    ) -> bool {
        self.gpu.complete_event(job_id, session_id)
    }

    pub(in crate::features) fn reset_gpu_refresh_failures(&mut self) {
        self.gpu.reset_refresh_failures();
    }

    pub(in crate::features) fn apply_gpu(&mut self, session_id: &str, overview: RemoteGpuOverview) {
        if overview.available {
            self.gpu.clear_unavailable(session_id);
        } else {
            self.gpu.mark_unavailable(session_id.to_string());
        }
        self.gpu.data = Some(overview);
    }

    pub(in crate::features) fn record_gpu_refresh_failure(&mut self) -> u8 {
        let failures = self.gpu.record_refresh_failure();
        if failures >= 3 {
            self.gpu.data = None;
        }
        failures
    }

    pub(in crate::features) fn npu_is_pending_for(&self, session_id: &str) -> bool {
        self.npu.is_pending_for(session_id)
    }

    pub(in crate::features) fn npu_unavailable_for(&self, session_id: &str) -> bool {
        self.npu.unavailable_for(session_id)
    }

    pub(in crate::features) fn begin_npu_job(
        &mut self,
        session_id: String,
    ) -> RemoteJobTicket<NpuJobResult> {
        self.npu.begin_job(session_id)
    }

    pub(in crate::features) fn mark_npu_refresh_started(&mut self) {
        self.npu.mark_refresh_started();
    }

    pub(in crate::features) fn next_npu_event(&self) -> Option<NpuJobResult> {
        self.npu.next_event()
    }

    pub(in crate::features) fn complete_npu_event(
        &mut self,
        job_id: u64,
        session_id: &str,
    ) -> bool {
        self.npu.complete_event(job_id, session_id)
    }

    pub(in crate::features) fn reset_npu_refresh_failures(&mut self) {
        self.npu.reset_refresh_failures();
    }

    pub(in crate::features) fn apply_npu(&mut self, session_id: &str, overview: RemoteNpuOverview) {
        if overview.available {
            self.npu.clear_unavailable(session_id);
        } else {
            self.npu.mark_unavailable(session_id.to_string());
        }
        self.npu.data = Some(overview);
    }

    pub(in crate::features) fn record_npu_refresh_failure(&mut self) -> u8 {
        let failures = self.npu.record_refresh_failure();
        if failures >= 3 {
            self.npu.data = None;
        }
        failures
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

    pub(in crate::features) fn apply_overview(&mut self, overview: RemoteDockerOverview) {
        if let Some(details_id) = self.details_container_id.as_deref()
            && !overview
                .containers
                .iter()
                .any(|container| container.id == details_id)
        {
            self.details = None;
            self.details_container_id = None;
            self.details_last_refresh_at = None;
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
        self.menu_pid = None;
        self.nice_draft = "0".to_string();
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

    pub(in crate::features) fn apply_processes(&mut self, processes: Vec<RemoteProcess>) {
        let contains_pid = |pid| processes.iter().any(|process| process.pid == pid);
        if self.selected_pid.is_some_and(|pid| !contains_pid(pid)) {
            self.selected_pid = None;
            self.nice_draft = "0".to_string();
        }
        if self.menu_pid.is_some_and(|pid| !contains_pid(pid)) {
            self.menu_pid = None;
        }
        self.items = processes.into();
        self.snapshot_loaded = true;
    }

    fn reset_for_session_switch(&mut self) {
        self.job.reset_for_session_switch();
        self.items = Arc::from([]);
        self.snapshot_loaded = false;
        self.selected_pid = None;
        self.menu_pid = None;
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

impl<Data, Event> AcceleratorPaneState<Data, Event> {
    fn new(status: &str) -> Self {
        Self {
            job: RemoteJobState::new(),
            data: None,
            status: status.to_string(),
            unavailable_sessions: HashSet::new(),
        }
    }

    fn is_pending(&self) -> bool {
        self.job.is_pending()
    }

    fn last_refresh_at(&self) -> Option<Instant> {
        self.job.last_refresh_at()
    }

    fn consecutive_refresh_failures(&self) -> u8 {
        self.job.consecutive_refresh_failures()
    }

    fn is_pending_for(&self, session_id: &str) -> bool {
        self.job.is_pending_for(session_id)
    }

    fn unavailable_for(&self, session_id: &str) -> bool {
        self.unavailable_sessions.contains(session_id)
    }

    fn mark_unavailable(&mut self, session_id: String) {
        self.unavailable_sessions.insert(session_id);
    }

    fn clear_unavailable(&mut self, session_id: &str) {
        self.unavailable_sessions.remove(session_id);
    }

    fn begin_job(&mut self, session_id: String) -> RemoteJobTicket<Event> {
        self.job.begin(session_id)
    }

    fn mark_refresh_started(&mut self) {
        self.job.mark_refresh_started();
    }

    fn next_event(&self) -> Option<Event> {
        self.job.try_recv()
    }

    fn complete_event(&mut self, job_id: u64, session_id: &str) -> bool {
        self.job.complete_if_matches(job_id, session_id)
    }

    fn reset_refresh_failures(&mut self) {
        self.job.reset_refresh_failures();
    }

    fn record_refresh_failure(&mut self) -> u8 {
        self.job.record_refresh_failure(false)
    }

    fn reset_for_session_switch(&mut self, status: &str) {
        self.job.reset_for_session_switch();
        self.data = None;
        self.status = status.to_string();
    }
}

#[cfg(test)]
mod tests {
    use nyaterm_transport::{DockerContainerDetails, RemoteDockerOverview, RemoteProcess};

    use super::{RemoteJobState, RemoteOpsFeatureFocus, RemoteOpsFeatureState};

    fn process(pid: u32) -> RemoteProcess {
        RemoteProcess {
            pid,
            ppid: 1,
            user: "user".to_string(),
            state: "S".to_string(),
            cpu_percent: 1.0,
            memory_percent: 2.0,
            rss_kb: 3,
            vsz_kb: 4,
            elapsed: "00:01".to_string(),
            command: "sleep".to_string(),
            command_line: "sleep 10".to_string(),
        }
    }

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
    fn docker_owner_excludes_menus_and_cleans_removed_details_and_compose_data() {
        let mut state = RemoteOpsFeatureState::new(RemoteOpsFeatureFocus {});

        state.toggle_docker_tab_menu();
        state.toggle_docker_header_menu();
        let presentation = state.docker_presentation();
        assert!(!presentation.tab_menu_open);
        assert!(state.docker_header_menu_open());

        state.toggle_docker_container_menu("container".to_string());
        state.toggle_docker_compose_menu("compose".to_string());
        let presentation = state.docker_presentation();
        assert!(!state.docker_header_menu_open());
        assert!(presentation.container_menu_id.is_none());
        assert_eq!(presentation.compose_menu_id.as_deref(), Some("compose"));

        state.start_docker_details("gone".to_string(), "loading".to_string());
        state.apply_docker_details("gone".to_string(), DockerContainerDetails::default());
        state.toggle_compose_project("old".to_string(), "old");
        state.set_compose_services("old".to_string(), Vec::new());
        state.toggle_compose_project("failed".to_string(), "failed");
        state.set_compose_service_error("failed".to_string(), "error".to_string());
        state.apply_docker_overview(RemoteDockerOverview::default());

        let presentation = state.docker_presentation();
        assert!(presentation.details.is_none());
        assert!(presentation.details_container_id.is_none());
        assert!(state.docker_details_refresh().is_none());
        assert!(presentation.compose_expanded.is_empty());
        assert!(presentation.compose_services.is_empty());
        assert!(presentation.compose_service_errors.is_empty());
    }

    #[test]
    fn process_owner_cleans_pid_scoped_interaction_when_results_change() {
        let mut state = RemoteOpsFeatureState::new(RemoteOpsFeatureFocus {});
        state.apply_processes(vec![process(42)]);
        state.toggle_process_selection(42);
        state.toggle_process_menu(42);
        state.apply_process_nice_input("-1234x".to_string());

        let presentation = state.process_presentation();
        assert_eq!(presentation.nice_draft, "-123");
        assert_eq!(presentation.selected_pid, Some(42));
        assert_eq!(presentation.menu_pid, Some(42));

        state.apply_processes(Vec::new());

        let presentation = state.process_presentation();
        assert!(presentation.selected_pid.is_none());
        assert!(presentation.menu_pid.is_none());
        assert_eq!(presentation.nice_draft, "0");
    }

    #[test]
    fn stats_owner_resets_session_runtime_without_losing_expansion_preference() {
        let mut state = RemoteOpsFeatureState::new(RemoteOpsFeatureFocus {});
        state.toggle_stats_cpu_expanded();
        state.begin_stats_job("session-a".to_string());

        state.reset_for_session_switch();

        let presentation = state.stats_presentation();
        assert!(!presentation.pending);
        assert!(presentation.data.is_none());
        assert!(presentation.cpu_expanded);
        assert_eq!(
            presentation.status,
            "start an SSH session to inspect remote stats"
        );
    }

    #[test]
    fn accelerator_owner_caches_unavailable_sessions_until_success() {
        let mut state = RemoteOpsFeatureState::new(RemoteOpsFeatureFocus {});
        let session_id = "session-a";

        state.apply_gpu(
            session_id,
            nyaterm_transport::RemoteGpuOverview {
                available: false,
                ..Default::default()
            },
        );

        assert!(state.gpu_unavailable_for(session_id));

        state.reset_for_session_switch();
        assert!(state.gpu_unavailable_for(session_id));

        state.apply_gpu(
            session_id,
            nyaterm_transport::RemoteGpuOverview {
                available: true,
                ..Default::default()
            },
        );

        assert!(!state.gpu_unavailable_for(session_id));
    }
}

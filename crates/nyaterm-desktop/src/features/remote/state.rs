//! Grouped Remote page state: Docker, process table and host stats.
//!
//! Each of the three panes owns the same shape of refresh bookkeeping (job id,
//! owning session, pending flag, failure streak, last refresh instant) plus its
//! own view state. Keeping them in one struct per pane makes that symmetry
//! visible instead of spreading fifty-five prefixed fields across `NyaTermApp`.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::time::Instant;

use gpui::FocusHandle;
use nyaterm_transport::{
    DockerComposeService, DockerContainerDetails, RemoteDockerOverview, RemoteProcess, RemoteStats,
};

use crate::features::{DockerJobResult, ProcessJobResult, StatsJobResult};
use crate::models::{
    DockerConfirmState, DockerTab, RemoteProcessSignalConfirmState, RemoteProcessSortDirection,
    RemoteProcessSortKey,
};

pub(in crate::features) struct RemoteOpsFeatureState {
    pub docker: DockerPaneState,
    pub process: ProcessPaneState,
    pub stats: StatsPaneState,
}

/// Focus handles the Remote page needs at construction time.
pub(in crate::features) struct RemoteOpsFeatureFocus {}

pub(in crate::features) struct DockerPaneState {
    pub tx: mpsc::Sender<DockerJobResult>,
    pub rx: mpsc::Receiver<DockerJobResult>,
    pub overview: Option<RemoteDockerOverview>,
    pub status: String,
    pub pending: bool,
    pub job_id: u64,
    pub job_session_id: Option<String>,
    pub consecutive_refresh_failures: u8,
    pub last_refresh_at: Option<Instant>,
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
    pub tx: mpsc::Sender<ProcessJobResult>,
    pub rx: mpsc::Receiver<ProcessJobResult>,
    pub items: Vec<RemoteProcess>,
    pub snapshot_loaded: bool,
    pub status: String,
    pub pending: bool,
    pub job_id: u64,
    pub job_session_id: Option<String>,
    pub consecutive_refresh_failures: u8,
    pub last_refresh_at: Option<Instant>,
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
    pub tx: mpsc::Sender<StatsJobResult>,
    pub rx: mpsc::Receiver<StatsJobResult>,
    pub data: Option<RemoteStats>,
    pub status: String,
    pub pending: bool,
    pub job_id: u64,
    pub job_session_id: Option<String>,
    pub consecutive_refresh_failures: u8,
    pub last_refresh_at: Option<Instant>,
    pub cpu_expanded: bool,
}

impl RemoteOpsFeatureState {
    pub(in crate::features) fn new(focus: RemoteOpsFeatureFocus) -> Self {
        let (docker_tx, docker_rx) = mpsc::channel();
        let (process_tx, process_rx) = mpsc::channel();
        let (stats_tx, stats_rx) = mpsc::channel();
        Self {
            docker: DockerPaneState {
                tx: docker_tx,
                rx: docker_rx,
                overview: None,
                status: "start an SSH session to inspect Docker".to_string(),
                pending: false,
                job_id: 0,
                job_session_id: None,
                consecutive_refresh_failures: 0,
                last_refresh_at: None,
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
                tx: process_tx,
                rx: process_rx,
                items: Vec::new(),
                snapshot_loaded: false,
                status: "ready".to_string(),
                pending: false,
                job_id: 0,
                job_session_id: None,
                consecutive_refresh_failures: 0,
                last_refresh_at: None,
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
                tx: stats_tx,
                rx: stats_rx,
                data: None,
                status: "start an SSH session to inspect remote stats".to_string(),
                pending: false,
                job_id: 0,
                job_session_id: None,
                consecutive_refresh_failures: 0,
                last_refresh_at: None,
                cpu_expanded: false,
            },
        }
    }
}

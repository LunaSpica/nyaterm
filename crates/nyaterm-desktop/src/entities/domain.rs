//! Domain-facing UI entity stores (snapshot projection from `NyaTermApp`).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsSnapshot {
    pub active_tab: String,
    pub has_master_password: bool,
    pub security_unlocked: bool,
    pub cloud_sync_enabled: bool,
    pub startup_restore: bool,
}

impl Default for SettingsSnapshot {
    fn default() -> Self {
        Self {
            active_tab: "General".into(),
            has_master_password: false,
            security_unlocked: true,
            cloud_sync_enabled: false,
            startup_restore: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct SettingsStore {
    snapshot: Option<SettingsSnapshot>,
}

impl SettingsStore {
    pub fn snapshot(&self) -> Option<&SettingsSnapshot> {
        self.snapshot.as_ref()
    }

    pub fn replace_snapshot(&mut self, snapshot: SettingsSnapshot) -> bool {
        if self.snapshot.as_ref() == Some(&snapshot) {
            return false;
        }
        self.snapshot = Some(snapshot);
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionsSnapshot {
    pub connection_count: usize,
    pub group_count: usize,
    pub search_active: bool,
    pub editor_open: bool,
    pub group_editor_open: bool,
    pub delete_confirm_open: bool,
    pub sort_mode: String,
}

impl Default for ConnectionsSnapshot {
    fn default() -> Self {
        Self {
            connection_count: 0,
            group_count: 0,
            search_active: false,
            editor_open: false,
            group_editor_open: false,
            delete_confirm_open: false,
            sort_mode: "Default".into(),
        }
    }
}

#[derive(Debug, Default)]
pub struct ConnectionsStore {
    snapshot: Option<ConnectionsSnapshot>,
}

impl ConnectionsStore {
    pub fn snapshot(&self) -> Option<&ConnectionsSnapshot> {
        self.snapshot.as_ref()
    }

    pub fn replace_snapshot(&mut self, snapshot: ConnectionsSnapshot) -> bool {
        if self.snapshot.as_ref() == Some(&snapshot) {
            return false;
        }
        self.snapshot = Some(snapshot);
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferSnapshot {
    pub job_count: usize,
    pub active_job_count: usize,
    pub browser_path: String,
    pub selected_count: usize,
    pub browser_busy: bool,
}

impl Default for TransferSnapshot {
    fn default() -> Self {
        Self {
            job_count: 0,
            active_job_count: 0,
            browser_path: String::new(),
            selected_count: 0,
            browser_busy: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct TransferStore {
    snapshot: Option<TransferSnapshot>,
}

impl TransferStore {
    pub fn snapshot(&self) -> Option<&TransferSnapshot> {
        self.snapshot.as_ref()
    }

    pub fn replace_snapshot(&mut self, snapshot: TransferSnapshot) -> bool {
        if self.snapshot.as_ref() == Some(&snapshot) {
            return false;
        }
        self.snapshot = Some(snapshot);
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiSnapshot {
    pub chat_pending: bool,
    pub message_count: usize,
    pub session_id: String,
    pub agent_active: bool,
}

impl Default for AiSnapshot {
    fn default() -> Self {
        Self {
            chat_pending: false,
            message_count: 0,
            session_id: String::new(),
            agent_active: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct AiStore {
    snapshot: Option<AiSnapshot>,
}

impl AiStore {
    pub fn snapshot(&self) -> Option<&AiSnapshot> {
        self.snapshot.as_ref()
    }

    pub fn replace_snapshot(&mut self, snapshot: AiSnapshot) -> bool {
        if self.snapshot.as_ref() == Some(&snapshot) {
            return false;
        }
        self.snapshot = Some(snapshot);
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudSyncSnapshot {
    pub enabled: bool,
    pub provider: String,
    pub conflict_active: bool,
    pub last_status: String,
}

impl Default for CloudSyncSnapshot {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: String::new(),
            conflict_active: false,
            last_status: String::new(),
        }
    }
}

#[derive(Debug, Default)]
pub struct CloudSyncStore {
    snapshot: Option<CloudSyncSnapshot>,
}

impl CloudSyncStore {
    pub fn snapshot(&self) -> Option<&CloudSyncSnapshot> {
        self.snapshot.as_ref()
    }

    pub fn replace_snapshot(&mut self, snapshot: CloudSyncSnapshot) -> bool {
        if self.snapshot.as_ref() == Some(&snapshot) {
            return false;
        }
        self.snapshot = Some(snapshot);
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteOpsSnapshot {
    pub process_count: usize,
    pub docker_tab: String,
    pub stats_ready: bool,
    pub confirm_open: bool,
}

impl Default for RemoteOpsSnapshot {
    fn default() -> Self {
        Self {
            process_count: 0,
            docker_tab: String::new(),
            stats_ready: false,
            confirm_open: false,
        }
    }
}

#[derive(Debug, Default)]
pub struct RemoteOpsStore {
    snapshot: Option<RemoteOpsSnapshot>,
}

impl RemoteOpsStore {
    pub fn snapshot(&self) -> Option<&RemoteOpsSnapshot> {
        self.snapshot.as_ref()
    }

    pub fn replace_snapshot(&mut self, snapshot: RemoteOpsSnapshot) -> bool {
        if self.snapshot.as_ref() == Some(&snapshot) {
            return false;
        }
        self.snapshot = Some(snapshot);
        true
    }
}

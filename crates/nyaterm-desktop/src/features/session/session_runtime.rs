use super::*;

use crate::models::StartupCommandRequest;

pub(in crate::features) struct PendingSessionStartRegistration {
    pub(in crate::features) connection_name: String,
    pub(in crate::features) launch_config: Option<SessionLaunchConfig>,
    pub(in crate::features) kind: SessionKind,
    pub(in crate::features) ai_execution_profile: AiExecutionProfile,
    pub(in crate::features) custom_name: Option<String>,
    pub(in crate::features) tab_color: Option<u32>,
    pub(in crate::features) after_session_id: Option<String>,
    pub(in crate::features) insert_index: Option<usize>,
    pub(in crate::features) seed_output: Option<String>,
    pub(in crate::features) startup_command: Option<StartupCommandRequest>,
    pub(in crate::features) multiplex_key: Option<String>,
    pub(in crate::features) source_connection_id: Option<String>,
    pub(in crate::features) status_message: String,
    pub(in crate::features) append_start_log: bool,
}

#[path = "session_runtime/background.rs"]
mod background;
#[path = "session_runtime/start.rs"]
mod start;

use super::*;

#[derive(Debug)]
pub(in crate::ui::view) struct SessionStartResult {
    pub(in crate::ui::view) connection_name: String,
    pub(in crate::ui::view) result: Result<SessionStartSuccess, String>,
}

#[derive(Debug)]
pub(in crate::ui::view) struct SessionStartSuccess {
    pub(in crate::ui::view) session_id: String,
    pub(in crate::ui::view) multiplex_handle: Option<SshMultiplexHandle>,
}

#[derive(Debug)]
pub(in crate::ui::view) struct TunnelJobResult {
    pub(in crate::ui::view) tunnel_id: String,
    pub(in crate::ui::view) result: Result<TunnelJobOutput, String>,
}

#[derive(Debug)]
pub(in crate::ui::view) enum TunnelJobOutput {
    Opened(SshTunnelInfo),
    Closed,
}

#[derive(Debug)]
pub(in crate::ui::view) struct ProcessJobResult {
    pub(in crate::ui::view) result: Result<ProcessJobOutput, String>,
}

#[derive(Debug)]
pub(in crate::ui::view) struct StatsJobResult {
    pub(in crate::ui::view) result: Result<RemoteStats, String>,
}

#[derive(Debug)]
pub(in crate::ui::view) struct TranslateJobResult {
    pub(in crate::ui::view) result: Result<TranslateResult, String>,
}

#[derive(Debug)]
pub(in crate::ui::view) struct UpdateJobResult {
    pub(in crate::ui::view) result: Result<NativeUpdateInfo, String>,
}

#[derive(Debug)]
pub(in crate::ui::view) struct DockerJobResult {
    pub(in crate::ui::view) result: Result<DockerJobOutput, String>,
}

#[derive(Debug)]
pub(in crate::ui::view) struct AiDiscoveryJobResult {
    pub(in crate::ui::view) profile_id: String,
    pub(in crate::ui::view) result: Result<Vec<AiModelDiscovery>, String>,
}

#[derive(Debug)]
pub(in crate::ui::view) struct AiChatJobResult {
    pub(in crate::ui::view) job_id: u64,
    pub(in crate::ui::view) session_id: String,
    pub(in crate::ui::view) result: Result<AiChatJobOutput, String>,
}

#[derive(Debug)]
pub(in crate::ui::view) enum AiChatWorkerEvent {
    Delta {
        job_id: u64,
        session_id: String,
        text_delta: String,
        reasoning_delta: Option<String>,
    },
    AgentToolCallDelta {
        job_id: u64,
        session_id: String,
        tool_name: Option<String>,
        arguments_delta_len: usize,
    },
    AgentBackgroundFinished {
        job_id: u64,
        state: AiAgentLoopState,
        result: Result<CommandObservation, String>,
    },
    Finished(AiChatJobResult),
}

#[derive(Debug)]
pub(in crate::ui::view) struct AiChatJobOutput {
    pub(in crate::ui::view) mode: AiMode,
    pub(in crate::ui::view) text: String,
    pub(in crate::ui::view) reasoning: Option<String>,
    pub(in crate::ui::view) command_cards: Vec<AiCommandCard>,
    pub(in crate::ui::view) auto_execute_first: bool,
    pub(in crate::ui::view) approval_note: Option<String>,
}

#[derive(Debug, Clone)]
pub(in crate::ui::view) struct AiAgentLoopState {
    pub(in crate::ui::view) ai_session_id: String,
    pub(in crate::ui::view) terminal_session_id: String,
    pub(in crate::ui::view) task_prompt: String,
    pub(in crate::ui::view) command: String,
    pub(in crate::ui::view) marker_id: Option<String>,
    pub(in crate::ui::view) background_job_id: Option<u64>,
    pub(in crate::ui::view) step_index: u16,
    pub(in crate::ui::view) max_steps: u16,
    pub(in crate::ui::view) output_start_len: usize,
    pub(in crate::ui::view) started_at: Instant,
    pub(in crate::ui::view) min_wait_until: Instant,
    pub(in crate::ui::view) timeout_at: Instant,
    pub(in crate::ui::view) last_seen_len: usize,
    pub(in crate::ui::view) stable_since: Instant,
}

#[derive(Debug, Clone)]
pub(in crate::ui::view) struct AiAgentStepView {
    pub(in crate::ui::view) step_index: u16,
    pub(in crate::ui::view) status: AiAgentStepStatus,
    pub(in crate::ui::view) title: String,
    pub(in crate::ui::view) detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ui::view) enum AiAgentStepStatus {
    Planning,
    Tool,
    NeedsApproval,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone)]
pub(in crate::ui::view) enum AiAgentBackgroundTarget {
    Ssh(SshSessionConfig),
    Local { working_dir: Option<PathBuf> },
}

#[derive(Debug)]
pub(in crate::ui::view) enum ProcessJobOutput {
    Listed(Vec<RemoteProcess>),
    Signalled {
        pid: u32,
        signal: String,
        processes: Vec<RemoteProcess>,
    },
    Reniced {
        pid: u32,
        nice: i32,
        processes: Vec<RemoteProcess>,
    },
}

#[derive(Debug)]
pub(in crate::ui::view) enum DockerJobOutput {
    Overview(RemoteDockerOverview),
    Details {
        container_id: String,
        details: DockerContainerDetails,
    },
    Logs {
        container_id: String,
        text: String,
    },
    ComposeServices {
        key: String,
        project_name: String,
        services: Vec<DockerComposeService>,
    },
    ComposeServiceAction {
        key: String,
        service_name: String,
        action: String,
        overview: RemoteDockerOverview,
        services: Vec<DockerComposeService>,
    },
    ComposeProjectAction {
        key: String,
        project_name: String,
        action: String,
        overview: RemoteDockerOverview,
        services: Option<Vec<DockerComposeService>>,
        service_error: Option<String>,
    },
    RefreshedAfterAction {
        label: String,
        overview: RemoteDockerOverview,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ui::view) enum ActivitySide {
    Left,
    Right,
}

use gpui::{
    App, ClickEvent, Context, FocusHandle, FontWeight, IntoElement, KeyDownEvent,
    PathPromptOptions, Render, SharedString, Timer, Window, div, prelude::*, px, rgb,
};
use nyaterm_domain::{
    AgentApprovalDecision, AgentCapturedOutput, AgentCommandExecutionMode,
    AgentOutputCaptureProcessor, AiAction, AiChatRequest, AiChatStreamDelta, AiCommandCard,
    AiContext, AiExecutionProfile, AiMessage, AiMessageRole, AiMode, AiModelDiscovery,
    AiProviderCredential, AiProviderKind, AiSettings, AppRuntime, AppSettingsSummary,
    AppendAiAuditRequest, CLOUD_SYNC_HISTORY_LIMIT, CloudSyncError, CloudSyncHistoryEntry,
    CloudSyncResult, CloudSyncSettings, CloudSyncState, CommandHistoryEntry, CommandObservation,
    ConfigBackupInfo, ConnectionStore, ConnectionType, DecryptedOtpEntry, DiagnosticsExportInfo,
    DiagnosticsExportOptions, DiagnosticsRuntimeSnapshot, GiteeSnippetHttpBackend,
    GithubGistHttpBackend, KeywordHighlightConfig, KnownHostCheck, LocalCloudSyncOptions,
    NativeServiceStatus, NativeServices, NativeUpdateInfo, QuickCommand, QuickCommandCategory,
    RiskLevel, RuntimeMode, SavedConnection, SnippetRemote, TranslateResult, TranslationSettings,
    TunnelConfig, agent_response_action, ai_model_id_for_credential, ai_model_id_for_provider,
    append_cloud_sync_history, assess_agent_command_risk, build_agent_capture_command,
    build_observation_message, decide_agent_command_execution, export_diagnostics_archive,
    merge_model_discoveries, now_rfc3339, parse_agent_model_output, parse_agent_tool_call,
    parse_model_output, pull_local_snapshot, pull_snapshot_with_remote, push_local_snapshot,
    push_snapshot_with_remote, read_cloud_sync_history, redact_context, redact_sensitive_text,
    search_command_sources, truncate_preview, uuid,
};
use nyaterm_migration::{LegacyProject, MigrationInventory};
use nyaterm_session::{
    DockerService, LocalSessionConfig, RecordingManager, RemoteCommandOutput, RemoteDockerOverview,
    RemoteProcess, RemoteStats, RemoteStatsService, SFTP_TRANSFER_CANCELLED, SerialSessionConfig,
    SessionEvent, SessionKind, SessionManager, SftpDuplicateDecision, SftpDuplicatePolicy,
    SftpDuplicateRequest, SftpDuplicateResolver, SftpFileEntry, SftpFileType, SftpService,
    SftpTransferControl, SftpTransferDirection, SftpTransferProgress, SftpTransferSummary,
    SshCredentialPrompt, SshCredentialPromptKind, SshCredentialPromptReason, SshCredentialProvider,
    SshHostKey, SshHostKeyDecision, SshHostKeyVerifier, SshKeyAuthConfig, SshOtpProvider,
    SshProcessService, SshSessionConfig, SshTunnelConfig, SshTunnelInfo, SshTunnelManager,
    SshTunnelMode, TelnetEnterMode, TelnetSessionConfig, TerminalHistorySearchRequest,
    run_local_command, safe_recording_name,
};
use nyaterm_terminal::TerminalScreen;

use crate::ai_http::{complete_native_chat, discover_openai_compatible_models, stream_native_chat};
use crate::cloud_sync_http::{
    NativeAliyunDriveRemote, NativeGoogleDriveRemote, NativeOneDriveRemote, NativeS3Remote,
    NativeSnippetHttpClient, NativeWebdavRemote,
};
use crate::translation_http::translate_text;
use crate::update_http::check_native_update;

use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const LEGACY_ROOT: &str = "nyaterm-tauri";
const INITIAL_TERMINAL_BANNER: &str = "$ nyaterm --native\nGPUI shell initialized.\nStart a local terminal or open a saved connection.\n";
const AI_AGENT_OBSERVATION_MIN_WAIT: Duration = Duration::from_millis(700);
const AI_AGENT_OBSERVATION_QUIET: Duration = Duration::from_millis(900);
const AI_AGENT_DEFAULT_STEP_TIMEOUT: Duration = Duration::from_millis(30_000);

pub struct NyaTermApp {
    runtime: AppRuntime,
    services: NativeServices,
    inventory: MigrationInventory,
    connections: Vec<SavedConnection>,
    tunnels: Vec<TunnelConfig>,
    quick_commands: Vec<QuickCommand>,
    quick_command_categories: Vec<QuickCommandCategory>,
    command_history: Vec<CommandHistoryEntry>,
    command_search_draft: String,
    command_search_focus: FocusHandle,
    keyword_highlights: KeywordHighlightConfig,
    settings: AppSettingsSummary,
    store_status: StoreStatus,
    session_manager: Arc<SessionManager>,
    recording_manager: Arc<RecordingManager>,
    recording_search_draft: String,
    recording_search_focus: FocusHandle,
    session_start_tx: mpsc::Sender<SessionStartResult>,
    session_start_rx: mpsc::Receiver<SessionStartResult>,
    tunnel_manager: Arc<SshTunnelManager>,
    tunnel_tx: mpsc::Sender<TunnelJobResult>,
    tunnel_rx: mpsc::Receiver<TunnelJobResult>,
    pending_tunnels: Vec<String>,
    process_tx: mpsc::Sender<ProcessJobResult>,
    process_rx: mpsc::Receiver<ProcessJobResult>,
    processes: Vec<RemoteProcess>,
    process_status: String,
    process_pending: bool,
    stats_tx: mpsc::Sender<StatsJobResult>,
    stats_rx: mpsc::Receiver<StatsJobResult>,
    remote_stats: Option<RemoteStats>,
    stats_status: String,
    stats_pending: bool,
    translate_tx: mpsc::Sender<TranslateJobResult>,
    translate_rx: mpsc::Receiver<TranslateJobResult>,
    translate_provider: String,
    translation_settings: TranslationSettings,
    translation_secret_draft: TranslationSecretDraft,
    translate_target_language: String,
    translate_input: String,
    translate_result: Option<TranslateResult>,
    translate_status: String,
    translate_pending: bool,
    translate_focus: FocusHandle,
    translate_focused_field: TranslateInputField,
    update_tx: mpsc::Sender<UpdateJobResult>,
    update_rx: mpsc::Receiver<UpdateJobResult>,
    update_status: String,
    update_info: Option<NativeUpdateInfo>,
    update_pending: bool,
    docker_tx: mpsc::Sender<DockerJobResult>,
    docker_rx: mpsc::Receiver<DockerJobResult>,
    docker_overview: Option<RemoteDockerOverview>,
    docker_status: String,
    docker_pending: bool,
    docker_logs: String,
    transfer_tx: mpsc::Sender<TransferJobResult>,
    transfer_rx: mpsc::Receiver<TransferJobResult>,
    transfer_jobs: Vec<TransferJobState>,
    transfer_remote_path: String,
    transfer_local_path: String,
    transfer_duplicate_policy: SftpDuplicatePolicy,
    transfer_path_prompt: Option<TransferPathPromptKind>,
    recording_path_prompt: Option<RecordingPathPromptKind>,
    config_path_prompt: Option<ConfigPathPromptKind>,
    diagnostics_path_prompt: Option<DiagnosticsPathPromptKind>,
    keyword_highlight_path_prompt: Option<KeywordHighlightPathPromptKind>,
    active_snapshot_password_prompt: Option<SnapshotPasswordPromptState>,
    cloud_sync_settings: CloudSyncSettings,
    cloud_sync_state: CloudSyncState,
    cloud_sync_history: Vec<CloudSyncHistoryEntry>,
    cloud_sync_conflict: Option<CloudSyncConflictState>,
    cloud_sync_secret_draft: CloudSyncSecretDraft,
    cloud_sync_status: String,
    cloud_sync_focus: FocusHandle,
    cloud_sync_focused_field: CloudSyncInputField,
    ai_settings: AiSettings,
    ai_model_draft: String,
    ai_base_url_draft: String,
    ai_secret_draft: String,
    ai_status: String,
    ai_session_count: usize,
    ai_message_count: usize,
    ai_audit_count: usize,
    ai_discovery_tx: mpsc::Sender<AiDiscoveryJobResult>,
    ai_discovery_rx: mpsc::Receiver<AiDiscoveryJobResult>,
    ai_discovery_pending: bool,
    ai_chat_tx: mpsc::Sender<AiChatWorkerEvent>,
    ai_chat_rx: mpsc::Receiver<AiChatWorkerEvent>,
    ai_chat_pending: bool,
    ai_chat_job_id: u64,
    ai_chat_cancel: Option<Arc<AtomicBool>>,
    ai_chat_session_id: String,
    ai_prompt_draft: String,
    ai_response_preview: String,
    ai_command_cards: Vec<AiCommandCard>,
    ai_agent_task_prompt: Option<String>,
    ai_agent_step_index: u16,
    ai_agent_loop: Option<AiAgentLoopState>,
    ai_agent_capture: AgentOutputCaptureProcessor,
    ai_agent_steps: Vec<AiAgentStepView>,
    ai_chat_focus: FocusHandle,
    ai_focus: FocusHandle,
    ai_focused_field: AiInputField,
    transfer_focus: FocusHandle,
    transfer_focused_field: TransferInputField,
    duplicate_prompts: Arc<SftpDuplicatePromptBroker>,
    active_duplicate_prompt: Option<SftpDuplicatePromptState>,
    pending_session_name: Option<String>,
    pending_ssh_config: Option<SshSessionConfig>,
    pending_ai_execution_profile: AiExecutionProfile,
    host_key_prompts: Arc<HostKeyPromptBroker>,
    active_host_key_prompt: Option<HostKeyPromptRequest>,
    credential_prompts: Arc<CredentialPromptBroker>,
    active_credential_prompt: Option<CredentialPromptState>,
    credential_focus: FocusHandle,
    snapshot_password_focus: FocusHandle,
    otp_provider: Arc<NativeOtpProvider>,
    active_session_id: Option<String>,
    active_ssh_config: Option<SshSessionConfig>,
    active_ai_execution_profile: AiExecutionProfile,
    terminal_focus: FocusHandle,
    terminal_output: String,
    terminal_screen: TerminalScreen,
    terminal_status: String,
    event_pump_started: bool,
    selected_nav: NavItem,
}

#[derive(Debug, Clone)]
struct StoreStatus {
    path: String,
    message: String,
    ready: bool,
}

struct NativeHostKeyVerifier {
    config_dir: PathBuf,
    portable_key_path: Option<PathBuf>,
    policy: String,
    prompt_broker: Arc<HostKeyPromptBroker>,
}

impl SshHostKeyVerifier for NativeHostKeyVerifier {
    fn verify(&self, host_key: &SshHostKey) -> Result<SshHostKeyDecision, String> {
        let store = ConnectionStore::open_with_portable_key_path(
            &self.config_dir,
            self.portable_key_path.clone(),
        )
        .map_err(|error| error.to_string())?;
        let line = format!(
            "{} {} {}",
            host_key.host_identifier, host_key.key_type, host_key.key_base64
        );
        match store
            .check_known_host(
                &host_key.host_identifier,
                &host_key.key_type,
                &host_key.key_base64,
            )
            .map_err(|error| error.to_string())?
        {
            KnownHostCheck::Match => Ok(SshHostKeyDecision::Accept),
            KnownHostCheck::UnknownHost if self.policy == "strict" => {
                Ok(SshHostKeyDecision::Reject(format!(
                    "unknown SSH host key for {} ({})",
                    host_key.host_identifier, host_key.fingerprint
                )))
            }
            KnownHostCheck::UnknownHost if self.policy == "prompt" => {
                match self
                    .prompt_broker
                    .request_decision(host_key.clone(), HostKeyPromptIssue::Unknown)
                {
                    Ok(HostKeyPromptChoice::Accept) => {
                        store
                            .upsert_known_host(&line)
                            .map_err(|error| error.to_string())?;
                        Ok(SshHostKeyDecision::Accept)
                    }
                    Ok(HostKeyPromptChoice::Reject) => Ok(SshHostKeyDecision::Reject(format!(
                        "unknown SSH host key rejected for {} ({})",
                        host_key.host_identifier, host_key.fingerprint
                    ))),
                    Err(error) => Ok(SshHostKeyDecision::Reject(error)),
                }
            }
            KnownHostCheck::UnknownHost => {
                store
                    .upsert_known_host(&line)
                    .map_err(|error| error.to_string())?;
                Ok(SshHostKeyDecision::Accept)
            }
            KnownHostCheck::HostSeen if self.policy == "accept" => {
                store
                    .replace_known_host_for_host(&host_key.host_identifier, &line)
                    .map_err(|error| error.to_string())?;
                Ok(SshHostKeyDecision::Accept)
            }
            KnownHostCheck::HostSeen if self.policy == "prompt" => {
                match self
                    .prompt_broker
                    .request_decision(host_key.clone(), HostKeyPromptIssue::Changed)
                {
                    Ok(HostKeyPromptChoice::Accept) => {
                        store
                            .replace_known_host_for_host(&host_key.host_identifier, &line)
                            .map_err(|error| error.to_string())?;
                        Ok(SshHostKeyDecision::Accept)
                    }
                    Ok(HostKeyPromptChoice::Reject) => Ok(SshHostKeyDecision::Reject(format!(
                        "changed SSH host key rejected for {} ({})",
                        host_key.host_identifier, host_key.fingerprint
                    ))),
                    Err(error) => Ok(SshHostKeyDecision::Reject(error)),
                }
            }
            KnownHostCheck::HostSeen => Ok(SshHostKeyDecision::Reject(format!(
                "SSH host key changed for {} ({})",
                host_key.host_identifier, host_key.fingerprint
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TotpUseRecord {
    code: String,
    time_step: u64,
}

#[derive(Debug)]
struct NativeOtpProvider {
    config_dir: PathBuf,
    portable_key_path: Option<PathBuf>,
    used_totp_codes: Mutex<HashMap<String, TotpUseRecord>>,
}

impl NativeOtpProvider {
    fn new(config_dir: PathBuf, portable_key_path: Option<PathBuf>) -> Self {
        Self {
            config_dir,
            portable_key_path,
            used_totp_codes: Mutex::new(HashMap::new()),
        }
    }

    fn load_entry(&self, otp_id: &str) -> Result<Option<DecryptedOtpEntry>, String> {
        let store = ConnectionStore::open_with_portable_key_path(
            &self.config_dir,
            self.portable_key_path.clone(),
        )
        .map_err(|error| error.to_string())?;
        store
            .load_decrypted_otp_entry_by_id(otp_id)
            .map_err(|error| error.to_string())
    }

    fn generate_totp_code(&self, entry: &DecryptedOtpEntry, now: u64) -> Result<TotpCode, String> {
        let (algorithm, secret, digits) = otp_material(entry)?;
        let period = if entry.period > 0 { entry.period } else { 30 };
        let totp = nyaterm_otp::Totp::new(
            algorithm,
            entry.issuer.clone(),
            entry.username.clone(),
            digits,
            period,
            secret,
        );
        let raw = totp.generate_at(now);
        Ok(TotpCode {
            code: format!("{:0>width$}", raw, width = digits as usize),
            time_step: now / period,
            period,
        })
    }

    fn generate_hotp_code(&self, entry: &DecryptedOtpEntry) -> Result<String, String> {
        let (algorithm, secret, digits) = otp_material(entry)?;
        let mut hotp = nyaterm_otp::Hotp::new(
            algorithm,
            entry.issuer.clone(),
            entry.username.clone(),
            digits,
            entry.counter,
            secret,
        );
        let raw = hotp.generate();
        Ok(format!("{:0>width$}", raw, width = digits as usize))
    }

    fn increment_counter(&self, otp_id: &str) -> Result<(), String> {
        let store = ConnectionStore::open_with_portable_key_path(
            &self.config_dir,
            self.portable_key_path.clone(),
        )
        .map_err(|error| error.to_string())?;
        store
            .increment_otp_counter(otp_id)
            .map_err(|error| error.to_string())
    }

    fn has_used_totp_code(&self, otp_id: &str, candidate: &TotpCode) -> Result<bool, String> {
        let used = self
            .used_totp_codes
            .lock()
            .map_err(|_| "TOTP use cache is poisoned".to_string())?;
        Ok(used.get(otp_id).is_some_and(|record| {
            record.code == candidate.code && record.time_step == candidate.time_step
        }))
    }

    fn record_totp_code(&self, otp_id: &str, candidate: &TotpCode) -> Result<(), String> {
        let mut used = self
            .used_totp_codes
            .lock()
            .map_err(|_| "TOTP use cache is poisoned".to_string())?;
        used.insert(
            otp_id.to_string(),
            TotpUseRecord {
                code: candidate.code.clone(),
                time_step: candidate.time_step,
            },
        );
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct TotpCode {
    code: String,
    time_step: u64,
    period: u64,
}

impl SshOtpProvider for NativeOtpProvider {
    fn request_otp_code(&self, otp_id: &str) -> Result<Option<String>, String> {
        let Some(entry) = self.load_entry(otp_id)? else {
            return Ok(None);
        };
        if entry.otp_type == "hotp" {
            let code = self.generate_hotp_code(&entry)?;
            self.increment_counter(otp_id)?;
            return Ok(Some(code));
        }

        let mut now = unix_seconds_now();
        let mut code = self.generate_totp_code(&entry, now)?;
        if self.has_used_totp_code(otp_id, &code)? {
            let wait = seconds_until_next_totp_step(now, code.period);
            std::thread::sleep(Duration::from_secs(wait));
            now = unix_seconds_now();
            code = self.generate_totp_code(&entry, now)?;
        }
        self.record_totp_code(otp_id, &code)?;
        Ok(Some(code.code))
    }
}

fn otp_material(
    entry: &DecryptedOtpEntry,
) -> Result<(nyaterm_otp::Algorithm, nyaterm_otp::Secret, u8), String> {
    let secret = entry
        .secret
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("OTP entry '{}' has no secret", entry.id))?;
    let algorithm = match entry.algorithm.as_str() {
        "SHA256" => nyaterm_otp::Algorithm::SHA256,
        "SHA512" => nyaterm_otp::Algorithm::SHA512,
        _ => nyaterm_otp::Algorithm::SHA1,
    };
    let secret = nyaterm_otp::Secret::from_base32(secret)
        .map_err(|error| format!("invalid OTP secret for '{}': {error:?}", entry.id))?;
    let digits = if entry.digits > 0 { entry.digits } else { 6 };
    Ok((algorithm, secret, digits))
}

fn unix_seconds_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn seconds_until_next_totp_step(now: u64, period: u64) -> u64 {
    let period = period.max(1);
    let remaining = period - (now % period);
    remaining.max(1)
}

#[derive(Debug)]
struct SessionStartResult {
    connection_name: String,
    result: Result<String, String>,
}

#[derive(Debug)]
struct TunnelJobResult {
    tunnel_id: String,
    result: Result<TunnelJobOutput, String>,
}

#[derive(Debug)]
enum TunnelJobOutput {
    Opened(SshTunnelInfo),
    Closed,
}

#[derive(Debug)]
struct ProcessJobResult {
    result: Result<ProcessJobOutput, String>,
}

#[derive(Debug)]
struct StatsJobResult {
    result: Result<RemoteStats, String>,
}

#[derive(Debug)]
struct TranslateJobResult {
    result: Result<TranslateResult, String>,
}

#[derive(Debug)]
struct UpdateJobResult {
    result: Result<NativeUpdateInfo, String>,
}

#[derive(Debug)]
struct DockerJobResult {
    result: Result<DockerJobOutput, String>,
}

#[derive(Debug)]
struct AiDiscoveryJobResult {
    profile_id: String,
    result: Result<Vec<AiModelDiscovery>, String>,
}

#[derive(Debug)]
struct AiChatJobResult {
    job_id: u64,
    session_id: String,
    result: Result<AiChatJobOutput, String>,
}

#[derive(Debug)]
enum AiChatWorkerEvent {
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
struct AiChatJobOutput {
    mode: AiMode,
    text: String,
    reasoning: Option<String>,
    command_cards: Vec<AiCommandCard>,
    auto_execute_first: bool,
    approval_note: Option<String>,
}

#[derive(Debug, Clone)]
struct AiAgentLoopState {
    ai_session_id: String,
    terminal_session_id: String,
    task_prompt: String,
    command: String,
    marker_id: Option<String>,
    background_job_id: Option<u64>,
    step_index: u16,
    max_steps: u16,
    output_start_len: usize,
    started_at: Instant,
    min_wait_until: Instant,
    timeout_at: Instant,
    last_seen_len: usize,
    stable_since: Instant,
}

#[derive(Debug, Clone)]
struct AiAgentStepView {
    step_index: u16,
    status: AiAgentStepStatus,
    title: String,
    detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AiAgentStepStatus {
    Planning,
    Tool,
    NeedsApproval,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone)]
enum AiAgentBackgroundTarget {
    Ssh(SshSessionConfig),
    Local { working_dir: Option<PathBuf> },
}

#[derive(Debug)]
enum ProcessJobOutput {
    Listed(Vec<RemoteProcess>),
    Signalled { pid: u32, signal: String },
    Reniced { pid: u32, nice: i32 },
}

#[derive(Debug)]
enum DockerJobOutput {
    Overview(RemoteDockerOverview),
    ContainerAction {
        container_id: String,
        action: String,
    },
    Logs {
        container_id: String,
        text: String,
    },
    Pruned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TransferJobKind {
    ListDir {
        remote_path: String,
    },
    Download {
        remote_path: String,
        local_path: PathBuf,
    },
    Upload {
        local_path: PathBuf,
        remote_path: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransferJobStatus {
    Running,
    Paused,
    Cancelling,
    Cancelled,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
struct TransferJobState {
    id: String,
    kind: TransferJobKind,
    status: TransferJobStatus,
    detail: String,
    entries: Vec<SftpFileEntry>,
    summary: Option<SftpTransferSummary>,
    progress: Option<SftpTransferProgress>,
    control: Option<SftpTransferControl>,
}

#[derive(Debug)]
struct TransferJobResult {
    id: String,
    event: TransferJobEvent,
}

#[derive(Debug)]
enum TransferJobEvent {
    Progress(SftpTransferProgress),
    Finished(Result<TransferJobOutput, String>),
}

#[derive(Debug)]
enum TransferJobOutput {
    Entries(Vec<SftpFileEntry>),
    Summary(SftpTransferSummary),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransferInputField {
    Remote,
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CloudSyncInputField {
    RemoteRoot,
    WebdavEndpoint,
    WebdavRoot,
    WebdavUsername,
    WebdavPassword,
    S3Endpoint,
    S3Bucket,
    S3Region,
    S3Root,
    S3AccessKeyId,
    S3SecretAccessKey,
    S3SessionToken,
    GoogleDriveRoot,
    GoogleDriveAccessToken,
    GoogleDriveRefreshToken,
    GoogleDriveClientId,
    GoogleDriveClientSecret,
    OneDriveRoot,
    OneDriveAccessToken,
    OneDriveRefreshToken,
    OneDriveClientId,
    OneDriveClientSecret,
    AliyunDriveRoot,
    AliyunDriveType,
    AliyunDriveAccessToken,
    AliyunDriveRefreshToken,
    AliyunDriveClientId,
    AliyunDriveClientSecret,
    GiteeEndpoint,
    GiteeGistId,
    GiteeToken,
    GithubGistId,
    GithubToken,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AiInputField {
    Model,
    BaseUrl,
    ApiKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TranslateInputField {
    TargetLanguage,
    Text,
    SettingsTargetLanguage,
    DeeplApiKey,
    BaiduAppId,
    BaiduAppKey,
    AliAppId,
    AliAppKey,
    YoudaoAppId,
    YoudaoAppKey,
}

impl TranslateInputField {
    fn is_settings_field(self) -> bool {
        matches!(
            self,
            Self::SettingsTargetLanguage
                | Self::DeeplApiKey
                | Self::BaiduAppId
                | Self::BaiduAppKey
                | Self::AliAppId
                | Self::AliAppKey
                | Self::YoudaoAppId
                | Self::YoudaoAppKey
        )
    }
}

#[derive(Debug, Clone, Default)]
struct CloudSyncSecretDraft {
    webdav_password: String,
    s3_access_key_id: String,
    s3_secret_access_key: String,
    s3_session_token: String,
    google_drive_access_token: String,
    google_drive_refresh_token: String,
    google_drive_client_secret: String,
    onedrive_access_token: String,
    onedrive_refresh_token: String,
    onedrive_client_secret: String,
    aliyun_drive_access_token: String,
    aliyun_drive_refresh_token: String,
    aliyun_drive_client_secret: String,
    gitee_token: String,
    github_token: String,
}

#[derive(Debug, Clone, Default)]
struct TranslationSecretDraft {
    deepl_api_key: String,
    baidu_app_key: String,
    ali_app_key: String,
    youdao_app_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransferPathPromptKind {
    UploadFile,
    UploadDirectory,
    DownloadDirectory,
}

#[derive(Debug)]
enum TransferPathPromptResult {
    Selected(PathBuf),
    Cancelled,
    Failed(String),
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecordingPathPromptKind {
    Start,
    SaveTranscript,
}

#[derive(Debug)]
enum RecordingPathPromptResult {
    Selected(PathBuf),
    Cancelled,
    Failed(String),
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigPathPromptKind {
    Export,
    Import,
    PortableExport,
    PortableImport,
    EncryptedPortableExport,
    EncryptedPortableImport,
}

#[derive(Debug)]
enum ConfigPathPromptResult {
    Exported(ConfigBackupInfo),
    Imported(ConfigBackupInfo),
    Cancelled,
    Failed(String),
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SnapshotPasswordPromptKind {
    Export,
    Import,
    CloudPush,
    CloudPull,
    CloudForcePush,
    CloudForcePull,
    CloudProviderPush,
    CloudProviderPull,
    CloudProviderForcePush,
    CloudProviderForcePull,
}

#[derive(Debug, Clone)]
struct SnapshotPasswordPromptState {
    kind: SnapshotPasswordPromptKind,
    value: String,
}

#[derive(Debug, Clone)]
struct CloudSyncConflictState {
    provider: String,
    message: String,
    provider_action: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiagnosticsPathPromptKind {
    Export,
}

#[derive(Debug)]
enum DiagnosticsPathPromptResult {
    Exported(DiagnosticsExportInfo),
    Cancelled,
    Failed(String),
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeywordHighlightPathPromptKind {
    Import,
}

#[derive(Debug)]
enum KeywordHighlightPathPromptResult {
    Imported {
        imported_rules: usize,
        updated_rules: usize,
        total_rules: usize,
    },
    Cancelled,
    Failed(String),
    Closed,
}

#[derive(Debug)]
struct SftpDuplicatePromptRequest {
    id: String,
    request: SftpDuplicateRequest,
    response_tx: mpsc::Sender<SftpDuplicateDecision>,
}

#[derive(Debug, Clone)]
struct SftpDuplicatePromptState {
    id: String,
    request: SftpDuplicateRequest,
    response_tx: mpsc::Sender<SftpDuplicateDecision>,
}

#[derive(Debug, Default)]
struct SftpDuplicatePromptBroker {
    pending: Mutex<VecDeque<SftpDuplicatePromptRequest>>,
}

impl SftpDuplicatePromptBroker {
    fn request_decision(
        &self,
        request: SftpDuplicateRequest,
    ) -> Result<SftpDuplicateDecision, String> {
        let (response_tx, response_rx) = mpsc::channel();
        let request = SftpDuplicatePromptRequest {
            id: sftp_duplicate_prompt_id(&request),
            request,
            response_tx,
        };
        self.pending
            .lock()
            .map_err(|_| "SFTP duplicate prompt queue is poisoned".to_string())?
            .push_back(request);

        response_rx
            .recv_timeout(Duration::from_secs(300))
            .map_err(|_| "SFTP duplicate prompt timed out".to_string())
    }

    fn pop_pending(&self) -> Option<SftpDuplicatePromptRequest> {
        self.pending
            .lock()
            .ok()
            .and_then(|mut pending| pending.pop_front())
    }
}

impl SftpDuplicateResolver for SftpDuplicatePromptBroker {
    fn resolve_duplicate(
        &self,
        request: &SftpDuplicateRequest,
    ) -> Result<SftpDuplicateDecision, String> {
        self.request_decision(request.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HostKeyPromptIssue {
    Unknown,
    Changed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HostKeyPromptChoice {
    Accept,
    Reject,
}

#[derive(Debug, Clone)]
struct HostKeyPromptRequest {
    id: String,
    host_key: SshHostKey,
    issue: HostKeyPromptIssue,
    response_tx: mpsc::Sender<HostKeyPromptChoice>,
}

#[derive(Debug, Default)]
struct HostKeyPromptBroker {
    pending: Mutex<VecDeque<HostKeyPromptRequest>>,
}

impl HostKeyPromptBroker {
    fn request_decision(
        &self,
        host_key: SshHostKey,
        issue: HostKeyPromptIssue,
    ) -> Result<HostKeyPromptChoice, String> {
        let (response_tx, response_rx) = mpsc::channel();
        let request = HostKeyPromptRequest {
            id: uuid_like_prompt_id(&host_key),
            host_key,
            issue,
            response_tx,
        };
        self.pending
            .lock()
            .map_err(|_| "host-key prompt queue is poisoned".to_string())?
            .push_back(request);

        response_rx
            .recv_timeout(Duration::from_secs(300))
            .map_err(|_| "SSH host-key prompt timed out".to_string())
    }

    fn pop_pending(&self) -> Option<HostKeyPromptRequest> {
        self.pending
            .lock()
            .ok()
            .and_then(|mut pending| pending.pop_front())
    }
}

#[derive(Debug)]
struct CredentialPromptRequest {
    id: String,
    prompt: SshCredentialPrompt,
    response_tx: mpsc::Sender<Option<String>>,
}

#[derive(Debug, Clone)]
struct CredentialPromptState {
    id: String,
    prompt: SshCredentialPrompt,
    response_tx: mpsc::Sender<Option<String>>,
    value: String,
}

#[derive(Debug, Default)]
struct CredentialPromptBroker {
    pending: Mutex<VecDeque<CredentialPromptRequest>>,
}

impl CredentialPromptBroker {
    fn request_secret(&self, prompt: SshCredentialPrompt) -> Result<Option<String>, String> {
        let (response_tx, response_rx) = mpsc::channel();
        let request = CredentialPromptRequest {
            id: credential_prompt_id(&prompt),
            prompt,
            response_tx,
        };
        self.pending
            .lock()
            .map_err(|_| "credential prompt queue is poisoned".to_string())?
            .push_back(request);

        response_rx
            .recv_timeout(Duration::from_secs(300))
            .map_err(|_| "SSH credential prompt timed out".to_string())
    }

    fn pop_pending(&self) -> Option<CredentialPromptRequest> {
        self.pending
            .lock()
            .ok()
            .and_then(|mut pending| pending.pop_front())
    }
}

impl SshCredentialProvider for CredentialPromptBroker {
    fn request_secret(&self, prompt: &SshCredentialPrompt) -> Result<Option<String>, String> {
        CredentialPromptBroker::request_secret(self, prompt.clone())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NavItem {
    Workspace,
    Connections,
    Tunnels,
    Stats,
    Processes,
    Docker,
    Translation,
    Transfers,
    Settings,
    Migration,
}

impl NyaTermApp {
    pub fn new(runtime: AppRuntime, cx: &mut Context<Self>) -> Self {
        let legacy = LegacyProject::new(LEGACY_ROOT);
        let inventory = nyaterm_migration::inventory(&legacy);
        let (session_start_tx, session_start_rx) = mpsc::channel();
        let (tunnel_tx, tunnel_rx) = mpsc::channel();
        let (process_tx, process_rx) = mpsc::channel();
        let (stats_tx, stats_rx) = mpsc::channel();
        let (translate_tx, translate_rx) = mpsc::channel();
        let (update_tx, update_rx) = mpsc::channel();
        let (docker_tx, docker_rx) = mpsc::channel();
        let (transfer_tx, transfer_rx) = mpsc::channel();
        let (ai_discovery_tx, ai_discovery_rx) = mpsc::channel();
        let (ai_chat_tx, ai_chat_rx) = mpsc::channel();
        let (
            connections,
            tunnels,
            quick_commands,
            quick_command_categories,
            command_history,
            keyword_highlights,
            settings,
            store_status,
            cloud_sync_settings,
            cloud_sync_state,
            translation_settings,
            ai_settings,
            ai_session_count,
            ai_message_count,
            ai_audit_count,
        ) = match ConnectionStore::open_with_portable_key_path(
            runtime.config_dir(),
            runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            Ok(store) => {
                let path = store.db_path().display().to_string();
                match store.load_sessions() {
                    Ok(config) => {
                        let settings = store.load_app_settings_summary().unwrap_or_default();
                        let tunnels = store.list_tunnels().unwrap_or_default();
                        let cloud_sync_settings =
                            store.load_cloud_sync_settings().unwrap_or_default();
                        let cloud_sync_state = store.load_cloud_sync_state().unwrap_or_default();
                        let translation_settings = store
                            .load_translation_settings()
                            .unwrap_or_else(|_| TranslationSettings {
                                target_language: settings.language.clone(),
                                ..TranslationSettings::default()
                            });
                        let quick_commands = store.load_quick_commands().unwrap_or_default();
                        let command_history = store.list_command_history(64).unwrap_or_default();
                        let keyword_highlights =
                            store.load_keyword_highlights().unwrap_or_default();
                        let ai_settings = store.load_ai_settings().unwrap_or_default();
                        let (ai_session_count, ai_message_count, ai_audit_count) =
                            ai_usage_counts(&store);
                        (
                            config.connections,
                            tunnels,
                            quick_commands.commands,
                            quick_commands.categories,
                            command_history,
                            keyword_highlights,
                            settings,
                            StoreStatus {
                                path,
                                message: "redb connection store online".to_string(),
                                ready: true,
                            },
                            cloud_sync_settings,
                            cloud_sync_state,
                            translation_settings,
                            ai_settings,
                            ai_session_count,
                            ai_message_count,
                            ai_audit_count,
                        )
                    }
                    Err(error) => (
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        KeywordHighlightConfig::default(),
                        AppSettingsSummary::default(),
                        StoreStatus {
                            path,
                            message: format!("failed to load sessions: {error}"),
                            ready: false,
                        },
                        CloudSyncSettings::default(),
                        CloudSyncState::default(),
                        TranslationSettings::default(),
                        AiSettings::default(),
                        0,
                        0,
                        0,
                    ),
                }
            }
            Err(error) => (
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                KeywordHighlightConfig::default(),
                AppSettingsSummary::default(),
                StoreStatus {
                    path: runtime
                        .config_dir()
                        .join("nyaterm.redb")
                        .display()
                        .to_string(),
                    message: format!("failed to open store: {error}"),
                    ready: false,
                },
                CloudSyncSettings::default(),
                CloudSyncState::default(),
                TranslationSettings::default(),
                AiSettings::default(),
                0,
                0,
                0,
            ),
        };
        let otp_provider = Arc::new(NativeOtpProvider::new(
            runtime.config_dir().to_path_buf(),
            runtime.portable_key_path().map(ToOwned::to_owned),
        ));
        let transfer_duplicate_policy =
            SftpDuplicatePolicy::from_legacy_value(&settings.transfer_duplicate_strategy);
        let recording_manager = Arc::new(RecordingManager::new());
        recording_manager.set_memory_limit(settings.recording_memory_limit_bytes as usize);
        let cloud_sync_history = read_cloud_sync_history(
            runtime.log_dir(),
            settings.diagnostics_retention_days,
            CLOUD_SYNC_HISTORY_LIMIT,
        )
        .unwrap_or_default();
        let (ai_model_draft, ai_base_url_draft) = ai_active_profile_drafts(&ai_settings);
        let translate_target_language = translation_settings.target_language.clone();

        Self {
            runtime,
            services: NativeServices::new(),
            inventory,
            connections,
            tunnels,
            quick_commands,
            quick_command_categories,
            command_history,
            command_search_draft: String::new(),
            command_search_focus: cx.focus_handle(),
            keyword_highlights,
            settings,
            store_status,
            session_manager: Arc::new(SessionManager::new()),
            recording_manager,
            recording_search_draft: String::new(),
            recording_search_focus: cx.focus_handle(),
            session_start_tx,
            session_start_rx,
            tunnel_manager: Arc::new(SshTunnelManager::new()),
            tunnel_tx,
            tunnel_rx,
            pending_tunnels: Vec::new(),
            process_tx,
            process_rx,
            processes: Vec::new(),
            process_status: "ready".to_string(),
            process_pending: false,
            stats_tx,
            stats_rx,
            remote_stats: None,
            stats_status: "start an SSH session to inspect remote stats".to_string(),
            stats_pending: false,
            translate_tx,
            translate_rx,
            translate_provider: "google".to_string(),
            translation_settings,
            translation_secret_draft: TranslationSecretDraft::default(),
            translate_target_language,
            translate_input: String::new(),
            translate_result: None,
            translate_status: "Google translation ready".to_string(),
            translate_pending: false,
            translate_focus: cx.focus_handle(),
            translate_focused_field: TranslateInputField::Text,
            update_tx,
            update_rx,
            update_status: format!("Current version {}", env!("CARGO_PKG_VERSION")),
            update_info: None,
            update_pending: false,
            docker_tx,
            docker_rx,
            docker_overview: None,
            docker_status: "start an SSH session to inspect Docker".to_string(),
            docker_pending: false,
            docker_logs: String::new(),
            transfer_tx,
            transfer_rx,
            transfer_jobs: Vec::new(),
            transfer_remote_path: ".".to_string(),
            transfer_local_path: "nyaterm-download.bin".to_string(),
            transfer_duplicate_policy,
            transfer_path_prompt: None,
            recording_path_prompt: None,
            config_path_prompt: None,
            diagnostics_path_prompt: None,
            keyword_highlight_path_prompt: None,
            active_snapshot_password_prompt: None,
            cloud_sync_settings,
            cloud_sync_state,
            cloud_sync_history,
            cloud_sync_conflict: None,
            cloud_sync_secret_draft: CloudSyncSecretDraft::default(),
            cloud_sync_status: "local provider ready".to_string(),
            cloud_sync_focus: cx.focus_handle(),
            cloud_sync_focused_field: CloudSyncInputField::RemoteRoot,
            ai_settings,
            ai_model_draft,
            ai_base_url_draft,
            ai_secret_draft: String::new(),
            ai_status: "AI settings ready".to_string(),
            ai_session_count,
            ai_message_count,
            ai_audit_count,
            ai_discovery_tx,
            ai_discovery_rx,
            ai_discovery_pending: false,
            ai_chat_tx,
            ai_chat_rx,
            ai_chat_pending: false,
            ai_chat_job_id: 0,
            ai_chat_cancel: None,
            ai_chat_session_id: format!("ai-session-{}", uuid()),
            ai_prompt_draft: String::new(),
            ai_response_preview: "Ask mode ready".to_string(),
            ai_command_cards: Vec::new(),
            ai_agent_task_prompt: None,
            ai_agent_step_index: 0,
            ai_agent_loop: None,
            ai_agent_capture: AgentOutputCaptureProcessor::new(),
            ai_agent_steps: Vec::new(),
            ai_chat_focus: cx.focus_handle(),
            ai_focus: cx.focus_handle(),
            ai_focused_field: AiInputField::Model,
            transfer_focus: cx.focus_handle(),
            transfer_focused_field: TransferInputField::Remote,
            duplicate_prompts: Arc::new(SftpDuplicatePromptBroker::default()),
            active_duplicate_prompt: None,
            pending_session_name: None,
            pending_ssh_config: None,
            pending_ai_execution_profile: AiExecutionProfile::SendOnly,
            host_key_prompts: Arc::new(HostKeyPromptBroker::default()),
            active_host_key_prompt: None,
            credential_prompts: Arc::new(CredentialPromptBroker::default()),
            active_credential_prompt: None,
            credential_focus: cx.focus_handle(),
            snapshot_password_focus: cx.focus_handle(),
            otp_provider,
            active_session_id: None,
            active_ssh_config: None,
            active_ai_execution_profile: AiExecutionProfile::SendOnly,
            terminal_focus: cx.focus_handle(),
            terminal_output: String::from(INITIAL_TERMINAL_BANNER),
            terminal_screen: initial_terminal_screen(),
            terminal_status: "idle".to_string(),
            event_pump_started: false,
            selected_nav: NavItem::Workspace,
        }
    }

    fn select(&mut self, item: NavItem, cx: &mut Context<Self>) {
        self.selected_nav = item;
        cx.notify();
    }

    fn start_local_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.active_session_id.is_some() || self.pending_session_name.is_some() {
            self.terminal_status = "session already running or connecting".to_string();
            cx.notify();
            return;
        }

        match self
            .session_manager
            .create_local_session(LocalSessionConfig::default())
        {
            Ok(info) => {
                self.active_session_id = Some(info.id.clone());
                self.active_ai_execution_profile = AiExecutionProfile::Posix;
                self.terminal_status = format!("running {}", short_id(&info.id));
                self.append_terminal_log(format!("\n# started local PTY {}\n", short_id(&info.id)));
                self.maybe_auto_start_recording(&info.id, &info.name);
                self.ensure_event_pump(window, cx);
            }
            Err(error) => {
                self.terminal_status = format!("failed to start local PTY: {error}");
            }
        }
        cx.notify();
    }

    fn start_saved_connection(
        &mut self,
        connection: SavedConnection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.active_session_id.is_some() || self.pending_session_name.is_some() {
            self.terminal_status = "close the active or pending session first".to_string();
            self.selected_nav = NavItem::Workspace;
            cx.notify();
            return;
        }

        match connection.config.clone() {
            ConnectionType::LocalTerminal {
                shell_path,
                shell_args,
                working_dir,
                ai_execution_profile,
            } => {
                let config = LocalSessionConfig {
                    name: connection.name.clone(),
                    shell_path: non_empty_string(shell_path),
                    shell_args: split_shell_args(&shell_args),
                    working_dir: working_dir
                        .filter(|value| !value.trim().is_empty())
                        .map(Into::into),
                    cols: 80,
                    rows: 24,
                };
                match self.session_manager.create_local_session(config) {
                    Ok(info) => self.activate_started_session(
                        connection.name,
                        info.id,
                        ai_execution_profile,
                        window,
                        cx,
                    ),
                    Err(error) => {
                        self.terminal_status = format!("failed to start local session: {error}");
                        self.selected_nav = NavItem::Workspace;
                        cx.notify();
                    }
                }
            }
            ConnectionType::Telnet {
                host,
                port,
                ai_execution_profile,
                raw_tcp_cli,
                enter_mode,
                force_character_at_a_time,
                send_naws,
                send_sga,
                ..
            } => {
                let config = TelnetSessionConfig {
                    name: connection.name.clone(),
                    host,
                    port,
                    raw_tcp: raw_tcp_cli,
                    enter_mode: parse_telnet_enter_mode(&enter_mode),
                    force_character_at_a_time,
                    send_naws,
                    send_sga,
                    cols: 80,
                    rows: 24,
                };
                match self.session_manager.create_telnet_session(config) {
                    Ok(info) => self.activate_started_session(
                        connection.name,
                        info.id,
                        ai_execution_profile,
                        window,
                        cx,
                    ),
                    Err(error) => {
                        self.terminal_status = format!("failed to start telnet session: {error}");
                        self.selected_nav = NavItem::Workspace;
                        cx.notify();
                    }
                }
            }
            ConnectionType::Ssh {
                ai_execution_profile,
                ..
            } => {
                self.ensure_event_pump(window, cx);
                let config = match self.build_ssh_session_config(&connection, &mut Vec::new()) {
                    Ok(config) => config,
                    Err(error) => {
                        self.terminal_status = format!("failed to prepare SSH session: {error}");
                        self.selected_nav = NavItem::Workspace;
                        cx.notify();
                        return;
                    }
                };
                self.begin_background_ssh_start(connection.name, config, ai_execution_profile, cx);
            }
            ConnectionType::Serial {
                port_name,
                baud_rate,
                data_bits,
                parity,
                stop_bits,
                ai_execution_profile,
                backspace_mode,
            } => {
                let config = SerialSessionConfig {
                    name: connection.name.clone(),
                    port_name,
                    baud_rate,
                    data_bits,
                    parity,
                    stop_bits,
                    backspace_mode,
                };
                match self.session_manager.create_serial_session(config) {
                    Ok(info) => self.activate_started_session(
                        connection.name,
                        info.id,
                        ai_execution_profile,
                        window,
                        cx,
                    ),
                    Err(error) => {
                        self.terminal_status = format!("failed to start serial session: {error}");
                        self.selected_nav = NavItem::Workspace;
                        cx.notify();
                    }
                }
            }
        }
    }

    fn load_ssh_key_auth(
        &self,
        key_id: Option<&str>,
        auth_mode: &str,
    ) -> Result<Option<SshKeyAuthConfig>, String> {
        if auth_mode != "key" {
            return Ok(None);
        }
        let key_id = key_id
            .filter(|key_id| !key_id.trim().is_empty())
            .ok_or_else(|| "connection is set to key auth but has no key_id".to_string())?;
        let store = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .map_err(|error| error.to_string())?;
        let key = store
            .load_decrypted_ssh_key_by_id(key_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("SSH key '{key_id}' was not found"))?;
        let key_data = key
            .key_data
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| format!("SSH key '{}' has no private key data", key.name))?;
        Ok(Some(SshKeyAuthConfig {
            key_data,
            cert_data: key.cert_data.filter(|value| !value.trim().is_empty()),
            passphrase: key.passphrase.filter(|value| !value.trim().is_empty()),
        }))
    }

    fn build_ssh_session_config(
        &self,
        connection: &SavedConnection,
        visited_proxy_jumps: &mut Vec<String>,
    ) -> Result<SshSessionConfig, String> {
        let ConnectionType::Ssh {
            host,
            port,
            username,
            backspace_mode,
            ai_execution_profile: _,
            x11_forwarding,
        } = connection.config.clone()
        else {
            return Err("only SSH connections can be used for SSH sessions".to_string());
        };
        let auth = connection.auth.clone().unwrap_or_default();
        let allow_none_auth = auth.mode == "none";
        let password = (!auth.has_password)
            .then_some(auth.password)
            .flatten()
            .filter(|value| !value.trim().is_empty());
        let key_auth = self.load_ssh_key_auth(auth.key_id.as_deref(), &auth.mode)?;
        let proxy_jump = self.load_proxy_jump_config(connection, visited_proxy_jumps)?;

        Ok(SshSessionConfig {
            name: connection.name.clone(),
            host,
            port,
            username,
            password,
            key_auth,
            otp_id: auth.otp_id.filter(|value| !value.trim().is_empty()),
            auto_fill_otp: auth.auto_fill_otp,
            proxy_jump,
            allow_none_auth,
            backspace_mode,
            term: "xterm-256color".to_string(),
            x11_forwarding,
            x11_display: self.settings.x11_display.clone(),
            cols: 80,
            rows: 24,
            host_key_verifier: Some(Arc::new(NativeHostKeyVerifier {
                config_dir: self.runtime.config_dir().to_path_buf(),
                portable_key_path: self.runtime.portable_key_path().map(ToOwned::to_owned),
                policy: self.settings.host_key_policy.clone(),
                prompt_broker: self.host_key_prompts.clone(),
            })),
            credential_provider: Some(self.credential_prompts.clone()),
            otp_provider: Some(self.otp_provider.clone()),
        })
    }

    fn load_proxy_jump_config(
        &self,
        connection: &SavedConnection,
        visited_proxy_jumps: &mut Vec<String>,
    ) -> Result<Option<Box<SshSessionConfig>>, String> {
        let Some(proxy_jump_id) = connection
            .network
            .as_ref()
            .and_then(|network| network.proxy_jump_id.as_deref())
            .filter(|value| !value.trim().is_empty())
        else {
            return Ok(None);
        };
        if visited_proxy_jumps
            .iter()
            .any(|visited| visited == proxy_jump_id)
        {
            return Err(format!(
                "ProxyJump chain contains a cycle at '{proxy_jump_id}'"
            ));
        }
        visited_proxy_jumps.push(proxy_jump_id.to_string());
        let store = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .map_err(|error| error.to_string())?;
        let jump_connection = store
            .get_connection(proxy_jump_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("ProxyJump connection '{proxy_jump_id}' was not found"))?;
        if !matches!(jump_connection.config, ConnectionType::Ssh { .. }) {
            return Err("Only SSH connections can be used as jump hosts".to_string());
        }
        let jump_config = self.build_ssh_session_config(&jump_connection, visited_proxy_jumps)?;
        visited_proxy_jumps.pop();
        Ok(Some(Box::new(jump_config)))
    }

    fn start_tunnel_job(
        &mut self,
        tunnel: TunnelConfig,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_tunnels.iter().any(|id| id == &tunnel.id) {
            self.terminal_status = format!("tunnel {} is already pending", tunnel_name(&tunnel));
            cx.notify();
            return;
        }
        if self.tunnel_manager.is_open(&tunnel.id).unwrap_or(false) {
            self.terminal_status = format!("tunnel {} is already open", tunnel_name(&tunnel));
            cx.notify();
            return;
        }

        let Some(connection_id) = tunnel.connection_id.as_deref() else {
            self.terminal_status = format!("tunnel {} has no SSH connection", tunnel_name(&tunnel));
            cx.notify();
            return;
        };
        let Some(connection) = self
            .connections
            .iter()
            .find(|connection| connection.id == connection_id)
            .cloned()
        else {
            self.terminal_status = format!(
                "tunnel {} references missing connection {}",
                tunnel_name(&tunnel),
                connection_id
            );
            cx.notify();
            return;
        };
        let mode = match tunnel_mode(&tunnel) {
            Some(mode) => mode,
            None => {
                self.terminal_status = format!(
                    "tunnel {} mode '{}' is not native yet",
                    tunnel_name(&tunnel),
                    tunnel.tunnel_type
                );
                cx.notify();
                return;
            }
        };
        let ssh_config = match self.build_ssh_session_config(&connection, &mut Vec::new()) {
            Ok(config) => config,
            Err(error) => {
                self.terminal_status =
                    format!("failed to prepare tunnel {}: {error}", tunnel_name(&tunnel));
                cx.notify();
                return;
            }
        };
        let config = SshTunnelConfig {
            id: tunnel.id.clone(),
            ssh_config,
            mode,
            bind_host: if tunnel.bind_localhost {
                "127.0.0.1".to_string()
            } else {
                "0.0.0.0".to_string()
            },
            listen_port: tunnel.listen_port,
            target_host: matches!(mode, SshTunnelMode::Local | SshTunnelMode::Remote)
                .then_some(tunnel.target_host.clone()),
            target_port: matches!(mode, SshTunnelMode::Local | SshTunnelMode::Remote)
                .then_some(tunnel.target_port),
        };

        self.ensure_event_pump(window, cx);
        self.pending_tunnels.push(tunnel.id.clone());
        self.terminal_status = format!("opening tunnel {}", tunnel_name(&tunnel));
        let tunnel_manager = self.tunnel_manager.clone();
        let tunnel_tx = self.tunnel_tx.clone();
        std::thread::spawn(move || {
            let result = tunnel_manager
                .open(config)
                .map(TunnelJobOutput::Opened)
                .map_err(|error| error.to_string());
            let _ = tunnel_tx.send(TunnelJobResult {
                tunnel_id: tunnel.id,
                result,
            });
        });
        cx.notify();
    }

    fn close_tunnel_job(&mut self, tunnel_id: String, cx: &mut Context<Self>) {
        if self.pending_tunnels.iter().any(|id| id == &tunnel_id) {
            self.terminal_status = format!("tunnel {tunnel_id} is already pending");
            cx.notify();
            return;
        }
        if !self.tunnel_manager.is_open(&tunnel_id).unwrap_or(false) {
            self.terminal_status = format!("tunnel {tunnel_id} is not open");
            cx.notify();
            return;
        }

        self.pending_tunnels.push(tunnel_id.clone());
        self.terminal_status = format!("closing tunnel {tunnel_id}");
        let tunnel_manager = self.tunnel_manager.clone();
        let tunnel_tx = self.tunnel_tx.clone();
        std::thread::spawn(move || {
            let result = tunnel_manager
                .close(&tunnel_id)
                .map(|_| TunnelJobOutput::Closed)
                .map_err(|error| error.to_string());
            let _ = tunnel_tx.send(TunnelJobResult { tunnel_id, result });
        });
        cx.notify();
    }

    fn refresh_processes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.process_status = "start an SSH session before listing processes".to_string();
            self.terminal_status = self.process_status.clone();
            cx.notify();
            return;
        };
        if self.process_pending {
            self.process_status = "process operation already running".to_string();
            cx.notify();
            return;
        }

        self.process_pending = true;
        self.process_status = "listing remote processes".to_string();
        self.ensure_event_pump(window, cx);
        let tx = self.process_tx.clone();
        std::thread::spawn(move || {
            let result = SshProcessService::new(config)
                .list_processes()
                .map(ProcessJobOutput::Listed)
                .map_err(|error| error.to_string());
            let _ = tx.send(ProcessJobResult { result });
        });
        cx.notify();
    }

    fn signal_process(
        &mut self,
        pid: u32,
        signal: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.process_status = "start an SSH session before signalling processes".to_string();
            self.terminal_status = self.process_status.clone();
            cx.notify();
            return;
        };
        if self.process_pending {
            self.process_status = "process operation already running".to_string();
            cx.notify();
            return;
        }

        self.process_pending = true;
        self.process_status = format!("sending {signal} to pid {pid}");
        self.ensure_event_pump(window, cx);
        let tx = self.process_tx.clone();
        std::thread::spawn(move || {
            let result = SshProcessService::new(config)
                .signal_process(pid, signal)
                .map(|_| ProcessJobOutput::Signalled {
                    pid,
                    signal: signal.to_string(),
                })
                .map_err(|error| error.to_string());
            let _ = tx.send(ProcessJobResult { result });
        });
        cx.notify();
    }

    fn renice_process(&mut self, pid: u32, nice: i32, window: &mut Window, cx: &mut Context<Self>) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.process_status = "start an SSH session before renicing processes".to_string();
            self.terminal_status = self.process_status.clone();
            cx.notify();
            return;
        };
        if self.process_pending {
            self.process_status = "process operation already running".to_string();
            cx.notify();
            return;
        }

        self.process_pending = true;
        self.process_status = format!("renicing pid {pid} to {nice}");
        self.ensure_event_pump(window, cx);
        let tx = self.process_tx.clone();
        std::thread::spawn(move || {
            let result = SshProcessService::new(config)
                .renice_process(pid, nice)
                .map(|_| ProcessJobOutput::Reniced { pid, nice })
                .map_err(|error| error.to_string());
            let _ = tx.send(ProcessJobResult { result });
        });
        cx.notify();
    }

    fn refresh_stats(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.stats_status = "start an SSH session before inspecting stats".to_string();
            self.terminal_status = self.stats_status.clone();
            cx.notify();
            return;
        };
        if self.stats_pending {
            self.stats_status = "stats refresh already running".to_string();
            cx.notify();
            return;
        }

        self.stats_pending = true;
        self.stats_status = "loading remote system stats".to_string();
        self.ensure_event_pump(window, cx);
        let tx = self.stats_tx.clone();
        std::thread::spawn(move || {
            let result = RemoteStatsService::new(config)
                .snapshot()
                .map_err(|error| error.to_string());
            let _ = tx.send(StatsJobResult { result });
        });
        cx.notify();
    }

    fn run_translation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.translate_pending {
            self.translate_status = "translation already running".to_string();
            cx.notify();
            return;
        }
        if self.translate_input.trim().is_empty() {
            self.translate_status = "type text before translating".to_string();
            cx.notify();
            return;
        }

        self.translate_pending = true;
        self.translate_status = format!("translating with {}", self.translate_provider);
        self.ensure_event_pump(window, cx);
        let tx = self.translate_tx.clone();
        let provider = self.translate_provider.clone();
        let target_language = self.translate_target_language.clone();
        let text = self.translate_input.clone();
        let settings = self.translation_settings.clone();
        std::thread::spawn(move || {
            let result = translate_text(&provider, &text, &target_language, &settings);
            let _ = tx.send(TranslateJobResult { result });
        });
        cx.notify();
    }

    fn set_translate_provider(&mut self, provider: &'static str, cx: &mut Context<Self>) {
        self.translate_provider = provider.to_string();
        self.translate_status = format!("translation provider set to {provider}");
        cx.notify();
    }

    fn save_translation_settings(&mut self, cx: &mut Context<Self>) {
        let mut next = self.translation_settings.clone();
        if !self.translation_secret_draft.deepl_api_key.is_empty() {
            next.deepl_api_key = self.translation_secret_draft.deepl_api_key.clone();
        }
        if !self.translation_secret_draft.baidu_app_key.is_empty() {
            next.baidu_app_key = self.translation_secret_draft.baidu_app_key.clone();
        }
        if !self.translation_secret_draft.ali_app_key.is_empty() {
            next.ali_app_key = self.translation_secret_draft.ali_app_key.clone();
        }
        if !self.translation_secret_draft.youdao_app_key.is_empty() {
            next.youdao_app_key = self.translation_secret_draft.youdao_app_key.clone();
        }

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_translation_settings(next))
        {
            Ok(saved) => {
                self.translation_settings = saved;
                self.translation_secret_draft = TranslationSecretDraft::default();
                self.translate_target_language = self.translation_settings.target_language.clone();
                self.translate_status = "translation settings saved".to_string();
                self.store_status.message = "translation settings saved".to_string();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.translate_status = format!("translation settings save failed: {error}");
                self.store_status.message = self.translate_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    fn clear_translation_secret(&mut self, provider: &'static str, cx: &mut Context<Self>) {
        match provider {
            "deepl" => {
                self.translation_settings.deepl_api_key.clear();
                self.translation_secret_draft.deepl_api_key.clear();
            }
            "baidu" => {
                self.translation_settings.baidu_app_key.clear();
                self.translation_secret_draft.baidu_app_key.clear();
            }
            "ali" => {
                self.translation_settings.ali_app_key.clear();
                self.translation_secret_draft.ali_app_key.clear();
            }
            "youdao" => {
                self.translation_settings.youdao_app_key.clear();
                self.translation_secret_draft.youdao_app_key.clear();
            }
            _ => {}
        }
        self.translate_status = format!("{provider} translation secret cleared; save to persist");
        cx.notify();
    }

    fn refresh_docker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status = "start an SSH session before inspecting Docker".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        self.docker_pending = true;
        self.docker_status = "loading Docker overview".to_string();
        self.ensure_event_pump(window, cx);
        let tx = self.docker_tx.clone();
        std::thread::spawn(move || {
            let result = DockerService::new(config)
                .overview()
                .map(DockerJobOutput::Overview)
                .map_err(|error| error.to_string());
            let _ = tx.send(DockerJobResult { result });
        });
        cx.notify();
    }

    fn docker_container_action(
        &mut self,
        container_id: String,
        action: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status = "start an SSH session before changing containers".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        self.docker_pending = true;
        self.docker_status = format!("Docker {action} {}", compact_id(&container_id));
        self.ensure_event_pump(window, cx);
        let tx = self.docker_tx.clone();
        std::thread::spawn(move || {
            let result = DockerService::new(config)
                .container_action(&container_id, action)
                .map(|_| DockerJobOutput::ContainerAction {
                    container_id,
                    action: action.to_string(),
                })
                .map_err(|error| error.to_string());
            let _ = tx.send(DockerJobResult { result });
        });
        cx.notify();
    }

    fn load_docker_logs(
        &mut self,
        container_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status = "start an SSH session before reading Docker logs".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        self.docker_pending = true;
        self.docker_status = format!("loading logs for {}", compact_id(&container_id));
        self.ensure_event_pump(window, cx);
        let tx = self.docker_tx.clone();
        std::thread::spawn(move || {
            let result = DockerService::new(config)
                .container_logs(&container_id, 200)
                .map(|output| DockerJobOutput::Logs {
                    container_id,
                    text: if output.stderr.trim().is_empty() {
                        output.stdout
                    } else {
                        format!("{}\n{}", output.stdout, output.stderr)
                    },
                })
                .map_err(|error| error.to_string());
            let _ = tx.send(DockerJobResult { result });
        });
        cx.notify();
    }

    fn prune_docker_system(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status = "start an SSH session before pruning Docker".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        self.docker_pending = true;
        self.docker_status = "running docker system prune".to_string();
        self.ensure_event_pump(window, cx);
        let tx = self.docker_tx.clone();
        std::thread::spawn(move || {
            let result = DockerService::new(config)
                .system_prune(false)
                .map(|_| DockerJobOutput::Pruned)
                .map_err(|error| error.to_string());
            let _ = tx.send(DockerJobResult { result });
        });
        cx.notify();
    }

    fn activate_started_session(
        &mut self,
        name: String,
        session_id: String,
        ai_execution_profile: AiExecutionProfile,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.active_session_id = Some(session_id.clone());
        self.active_ai_execution_profile = ai_execution_profile;
        self.terminal_status = format!("running {}", short_id(&session_id));
        self.append_terminal_log(format!("\n# started {name} ({})\n", short_id(&session_id)));
        self.selected_nav = NavItem::Workspace;
        self.maybe_auto_start_recording(&session_id, &name);
        self.ensure_event_pump(window, cx);
        cx.notify();
    }

    fn begin_background_ssh_start(
        &mut self,
        connection_name: String,
        config: SshSessionConfig,
        ai_execution_profile: AiExecutionProfile,
        cx: &mut Context<Self>,
    ) {
        self.pending_session_name = Some(connection_name.clone());
        self.pending_ssh_config = Some(config.clone());
        self.pending_ai_execution_profile = ai_execution_profile;
        self.terminal_status = format!("connecting to {connection_name}");
        self.append_terminal_log(format!("\n# connecting to {connection_name}\n"));
        self.selected_nav = NavItem::Workspace;

        let session_manager = self.session_manager.clone();
        let session_start_tx = self.session_start_tx.clone();
        std::thread::spawn(move || {
            let result = session_manager
                .create_ssh_session(config)
                .map(|info| info.id)
                .map_err(|error| error.to_string());
            let _ = session_start_tx.send(SessionStartResult {
                connection_name,
                result,
            });
        });
        cx.notify();
    }

    fn send_probe_command(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.active_session_id.as_deref() else {
            self.terminal_status = "start a session first".to_string();
            cx.notify();
            return;
        };

        let command = if cfg!(target_os = "windows") {
            "echo nyaterm-native-ready\r\n"
        } else {
            "printf 'nyaterm-native-ready\\n'\n"
        };
        match self.session_manager.write(session_id, command.as_bytes()) {
            Ok(()) => {
                self.terminal_status = "probe command sent".to_string();
            }
            Err(error) => {
                self.terminal_status = format!("write failed: {error}");
            }
        }
        cx.notify();
    }

    fn start_sftp_list_job(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.selected_nav = NavItem::Transfers;
            cx.notify();
            return;
        };
        self.ensure_event_pump(window, cx);
        let remote_path = self.normalized_transfer_remote_path();
        let id = self.next_transfer_id("sftp-list");
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
            kind: TransferJobKind::ListDir {
                remote_path: remote_path.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Listing {remote_path}"),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: None,
        });
        self.terminal_status = format!("SFTP list started for {remote_path}");
        let transfer_tx = self.transfer_tx.clone();
        std::thread::spawn(move || {
            let result = SftpService::new(config)
                .list_dir(&remote_path)
                .map(TransferJobOutput::Entries)
                .map_err(|error| error.to_string());
            let _ = transfer_tx.send(TransferJobResult {
                id,
                event: TransferJobEvent::Finished(result),
            });
        });
        cx.notify();
    }

    fn start_sftp_download_job(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.selected_nav = NavItem::Transfers;
            cx.notify();
            return;
        };
        let remote_path = self.normalized_transfer_remote_path();
        let local_path = self.normalized_transfer_local_path();
        let duplicate_policy = self.transfer_duplicate_policy;
        let duplicate_resolver = (duplicate_policy == SftpDuplicatePolicy::Ask)
            .then(|| self.duplicate_prompts.clone() as Arc<dyn SftpDuplicateResolver>);
        self.ensure_event_pump(window, cx);
        let id = self.next_transfer_id("sftp-download");
        let control = SftpTransferControl::new();
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
            kind: TransferJobKind::Download {
                remote_path: remote_path.clone(),
                local_path: local_path.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Downloading {remote_path}"),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: Some(control.clone()),
        });
        self.terminal_status = format!("SFTP download started for {remote_path}");
        let progress_tx = self.transfer_tx.clone();
        let finished_tx = self.transfer_tx.clone();
        std::thread::spawn(move || {
            let progress_id = id.clone();
            let result = SftpService::new(config)
                .download_path_with_progress_options_and_resolver(
                    &remote_path,
                    local_path,
                    control,
                    duplicate_policy,
                    duplicate_resolver,
                    move |progress| {
                        let _ = progress_tx.send(TransferJobResult {
                            id: progress_id.clone(),
                            event: TransferJobEvent::Progress(progress),
                        });
                    },
                )
                .map(TransferJobOutput::Summary)
                .map_err(|error| error.to_string());
            let _ = finished_tx.send(TransferJobResult {
                id,
                event: TransferJobEvent::Finished(result),
            });
        });
        cx.notify();
    }

    fn start_sftp_upload_job(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.terminal_status = "start an SSH session first".to_string();
            self.selected_nav = NavItem::Transfers;
            cx.notify();
            return;
        };
        let local_path = self.normalized_transfer_local_path();
        let remote_path = self.normalized_transfer_remote_path();
        let duplicate_policy = self.transfer_duplicate_policy;
        let duplicate_resolver = (duplicate_policy == SftpDuplicatePolicy::Ask)
            .then(|| self.duplicate_prompts.clone() as Arc<dyn SftpDuplicateResolver>);
        self.ensure_event_pump(window, cx);
        let id = self.next_transfer_id("sftp-upload");
        let control = SftpTransferControl::new();
        self.transfer_jobs.push(TransferJobState {
            id: id.clone(),
            kind: TransferJobKind::Upload {
                local_path: local_path.clone(),
                remote_path: remote_path.clone(),
            },
            status: TransferJobStatus::Running,
            detail: format!("Uploading {}", local_path.display()),
            entries: Vec::new(),
            summary: None,
            progress: None,
            control: Some(control.clone()),
        });
        self.terminal_status = format!("SFTP upload started for {}", local_path.display());
        let progress_tx = self.transfer_tx.clone();
        let finished_tx = self.transfer_tx.clone();
        std::thread::spawn(move || {
            let progress_id = id.clone();
            let result = SftpService::new(config)
                .upload_path_with_progress_options_and_resolver(
                    local_path,
                    &remote_path,
                    control,
                    duplicate_policy,
                    duplicate_resolver,
                    move |progress| {
                        let _ = progress_tx.send(TransferJobResult {
                            id: progress_id.clone(),
                            event: TransferJobEvent::Progress(progress),
                        });
                    },
                )
                .map(TransferJobOutput::Summary)
                .map_err(|error| error.to_string());
            let _ = finished_tx.send(TransferJobResult {
                id,
                event: TransferJobEvent::Finished(result),
            });
        });
        cx.notify();
    }

    fn cancel_transfer_job(&mut self, job_id: &str, cx: &mut Context<Self>) {
        let Some(job) = self
            .transfer_jobs
            .iter_mut()
            .find(|candidate| candidate.id == job_id)
        else {
            self.terminal_status = "transfer job not found".to_string();
            cx.notify();
            return;
        };

        if !matches!(
            job.status,
            TransferJobStatus::Running | TransferJobStatus::Paused
        ) {
            self.terminal_status = format!("transfer {} is not running", job.id);
            cx.notify();
            return;
        }

        let Some(control) = job.control.as_ref() else {
            self.terminal_status = format!("transfer {} cannot be cancelled", job.id);
            cx.notify();
            return;
        };

        control.cancel();
        job.status = TransferJobStatus::Cancelling;
        job.detail = "Cancelling".to_string();
        self.terminal_status = format!("SFTP transfer cancelling: {}", job.id);
        cx.notify();
    }

    fn pause_transfer_job(&mut self, job_id: &str, cx: &mut Context<Self>) {
        let Some(job) = self
            .transfer_jobs
            .iter_mut()
            .find(|candidate| candidate.id == job_id)
        else {
            self.terminal_status = "transfer job not found".to_string();
            cx.notify();
            return;
        };

        if job.status != TransferJobStatus::Running {
            self.terminal_status = format!("transfer {} is not running", job.id);
            cx.notify();
            return;
        }

        let Some(control) = job.control.as_ref() else {
            self.terminal_status = format!("transfer {} cannot be paused", job.id);
            cx.notify();
            return;
        };

        control.pause();
        job.status = TransferJobStatus::Paused;
        job.detail = "Paused".to_string();
        self.terminal_status = format!("SFTP transfer paused: {}", job.id);
        cx.notify();
    }

    fn resume_transfer_job(&mut self, job_id: &str, cx: &mut Context<Self>) {
        let Some(job) = self
            .transfer_jobs
            .iter_mut()
            .find(|candidate| candidate.id == job_id)
        else {
            self.terminal_status = "transfer job not found".to_string();
            cx.notify();
            return;
        };

        if job.status != TransferJobStatus::Paused {
            self.terminal_status = format!("transfer {} is not paused", job.id);
            cx.notify();
            return;
        }

        let Some(control) = job.control.as_ref() else {
            self.terminal_status = format!("transfer {} cannot be resumed", job.id);
            cx.notify();
            return;
        };

        control.resume();
        job.status = TransferJobStatus::Running;
        job.detail = "Resuming".to_string();
        self.terminal_status = format!("SFTP transfer resumed: {}", job.id);
        cx.notify();
    }

    fn next_transfer_id(&self, prefix: &str) -> String {
        format!("{prefix}-{}", self.transfer_jobs.len() + 1)
    }

    fn normalized_transfer_remote_path(&self) -> String {
        let value = self.transfer_remote_path.trim();
        if value.is_empty() {
            ".".to_string()
        } else {
            value.to_string()
        }
    }

    fn normalized_transfer_local_path(&self) -> PathBuf {
        let value = self.transfer_local_path.trim();
        if value.is_empty() {
            PathBuf::from("nyaterm-download.bin")
        } else {
            PathBuf::from(value)
        }
    }

    fn prompt_transfer_path(&mut self, kind: TransferPathPromptKind, cx: &mut Context<Self>) {
        if self.transfer_path_prompt.is_some() {
            self.terminal_status = "native path picker is already open".to_string();
            cx.notify();
            return;
        }

        let options = match kind {
            TransferPathPromptKind::UploadFile => PathPromptOptions {
                files: true,
                directories: false,
                multiple: false,
                prompt: Some(SharedString::from("Select upload file")),
            },
            TransferPathPromptKind::UploadDirectory => PathPromptOptions {
                files: false,
                directories: true,
                multiple: false,
                prompt: Some(SharedString::from("Select upload directory")),
            },
            TransferPathPromptKind::DownloadDirectory => PathPromptOptions {
                files: false,
                directories: true,
                multiple: false,
                prompt: Some(SharedString::from("Select download directory")),
            },
        };
        let remote_path = self.normalized_transfer_remote_path();
        let receiver = cx.prompt_for_paths(options);
        self.transfer_path_prompt = Some(kind);
        self.terminal_status = match kind {
            TransferPathPromptKind::UploadFile => "selecting upload file".to_string(),
            TransferPathPromptKind::UploadDirectory => "selecting upload directory".to_string(),
            TransferPathPromptKind::DownloadDirectory => "selecting download directory".to_string(),
        };
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(paths))) => paths
                    .into_iter()
                    .next()
                    .map(TransferPathPromptResult::Selected)
                    .unwrap_or(TransferPathPromptResult::Cancelled),
                Ok(Ok(None)) => TransferPathPromptResult::Cancelled,
                Ok(Err(error)) => TransferPathPromptResult::Failed(error.to_string()),
                Err(_) => TransferPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_transfer_path_prompt_result(kind, remote_path, result);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn apply_transfer_path_prompt_result(
        &mut self,
        kind: TransferPathPromptKind,
        remote_path: String,
        result: TransferPathPromptResult,
    ) {
        self.transfer_path_prompt = None;
        match result {
            TransferPathPromptResult::Selected(path) => {
                let selected = match kind {
                    TransferPathPromptKind::UploadFile
                    | TransferPathPromptKind::UploadDirectory => path,
                    TransferPathPromptKind::DownloadDirectory => {
                        path.join(download_file_name_from_remote_path(&remote_path))
                    }
                };
                self.transfer_local_path = selected.display().to_string();
                self.transfer_focused_field = TransferInputField::Local;
                self.terminal_status = match kind {
                    TransferPathPromptKind::UploadFile => "upload file selected".to_string(),
                    TransferPathPromptKind::UploadDirectory => {
                        "upload directory selected".to_string()
                    }
                    TransferPathPromptKind::DownloadDirectory => {
                        "download target selected".to_string()
                    }
                };
            }
            TransferPathPromptResult::Cancelled => {
                self.terminal_status = "path picker cancelled".to_string();
            }
            TransferPathPromptResult::Failed(error) => {
                self.terminal_status = format!("path picker failed: {error}");
            }
            TransferPathPromptResult::Closed => {
                self.terminal_status = "path picker closed before returning".to_string();
            }
        }
    }

    fn prompt_recording_path(&mut self, kind: RecordingPathPromptKind, cx: &mut Context<Self>) {
        if self.recording_path_prompt.is_some() {
            self.terminal_status = "recording path picker is already open".to_string();
            cx.notify();
            return;
        }
        let Some(session_id) = self.active_session_id.clone() else {
            self.terminal_status = "start a session before recording".to_string();
            cx.notify();
            return;
        };
        let session_name = self
            .active_session_name()
            .unwrap_or_else(|| "session".to_string());
        let target = recording_file_path(&self.settings, self.runtime.config_dir(), &session_name);
        let directory = target
            .parent()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| self.runtime.config_dir().to_path_buf());
        let file_name = target
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("nyaterm-recording.log");
        let receiver = cx.prompt_for_new_path(&directory, Some(file_name));
        self.recording_path_prompt = Some(kind);
        self.terminal_status = match kind {
            RecordingPathPromptKind::Start => "selecting recording path".to_string(),
            RecordingPathPromptKind::SaveTranscript => "selecting transcript path".to_string(),
        };
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(path))) => RecordingPathPromptResult::Selected(path),
                Ok(Ok(None)) => RecordingPathPromptResult::Cancelled,
                Ok(Err(error)) => RecordingPathPromptResult::Failed(error.to_string()),
                Err(_) => RecordingPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_recording_path_prompt_result(kind, session_id, result);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn apply_recording_path_prompt_result(
        &mut self,
        kind: RecordingPathPromptKind,
        session_id: String,
        result: RecordingPathPromptResult,
    ) {
        self.recording_path_prompt = None;
        match result {
            RecordingPathPromptResult::Selected(path) => match kind {
                RecordingPathPromptKind::Start => {
                    self.start_recording_to_path(&session_id, path.display().to_string());
                }
                RecordingPathPromptKind::SaveTranscript => {
                    self.save_transcript_to_path(&session_id, path.display().to_string());
                }
            },
            RecordingPathPromptResult::Cancelled => {
                self.terminal_status = match kind {
                    RecordingPathPromptKind::Start => "recording start cancelled".to_string(),
                    RecordingPathPromptKind::SaveTranscript => {
                        "transcript save cancelled".to_string()
                    }
                };
            }
            RecordingPathPromptResult::Failed(error) => {
                self.terminal_status = format!("recording path picker failed: {error}");
            }
            RecordingPathPromptResult::Closed => {
                self.terminal_status = "recording path picker closed before returning".to_string();
            }
        }
    }

    fn start_recording_to_path(&mut self, session_id: &str, path: String) {
        self.recording_manager
            .set_memory_limit(self.settings.recording_memory_limit_bytes as usize);
        match self.recording_manager.start(
            session_id,
            &path,
            self.settings.recording_include_io_labels,
            self.settings.recording_include_timestamps,
        ) {
            Ok(()) => {
                self.terminal_status = format!("recording started: {path}");
                self.append_terminal_log(format!("\n# recording started: {path}\n"));
            }
            Err(error) => {
                self.terminal_status = format!("recording start failed: {error}");
            }
        }
    }

    fn stop_active_recording(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.active_session_id.clone() else {
            self.terminal_status = "no active session to stop recording".to_string();
            cx.notify();
            return;
        };
        match self.recording_manager.stop(&session_id) {
            Ok(path) => {
                self.terminal_status = format!("recording saved: {path}");
                self.append_terminal_log(format!("\n# recording saved: {path}\n"));
            }
            Err(error) => {
                self.terminal_status = format!("recording stop failed: {error}");
            }
        }
        cx.notify();
    }

    fn save_transcript_to_path(&mut self, session_id: &str, path: String) {
        self.recording_manager
            .set_memory_limit(self.settings.recording_memory_limit_bytes as usize);
        match self.recording_manager.save_transcript(
            session_id,
            &path,
            self.settings.recording_include_io_labels,
            self.settings.recording_include_timestamps,
        ) {
            Ok(path) => {
                self.terminal_status = format!("transcript saved: {path}");
                self.append_terminal_log(format!("\n# transcript saved: {path}\n"));
            }
            Err(error) => {
                self.terminal_status = format!("transcript save failed: {error}");
            }
        }
    }

    fn maybe_auto_start_recording(&mut self, session_id: &str, session_name: &str) {
        if !self.settings.recording_auto_start {
            return;
        }
        let path = recording_file_path(&self.settings, self.runtime.config_dir(), session_name);
        self.start_recording_to_path(session_id, path.display().to_string());
    }

    fn active_session_name(&self) -> Option<String> {
        let session_id = self.active_session_id.as_deref()?;
        self.session_manager
            .list_sessions()
            .ok()?
            .into_iter()
            .find(|session| session.id == session_id)
            .map(|session| session.name)
    }

    fn send_terminal_input(&mut self, bytes: Vec<u8>, cx: &mut Context<Self>) {
        if bytes.is_empty() {
            return;
        }
        let Some(session_id) = self.active_session_id.as_deref() else {
            self.terminal_status = "start a session before typing".to_string();
            cx.notify();
            return;
        };
        match self.session_manager.write(session_id, &bytes) {
            Ok(()) => {
                self.recording_manager.write_input(session_id, &bytes);
                self.record_command_history_from_bytes(&bytes);
                self.terminal_status = format!("sent {} byte(s)", bytes.len());
            }
            Err(error) => {
                self.terminal_status = format!("input failed: {error}");
            }
        }
        cx.notify();
    }

    fn close_active_session(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self.active_session_id.take() else {
            self.terminal_status = "no active session".to_string();
            cx.notify();
            return;
        };
        self.active_ssh_config = None;
        self.active_ai_execution_profile = AiExecutionProfile::SendOnly;
        self.ai_agent_loop = None;
        self.ai_agent_capture = AgentOutputCaptureProcessor::new();

        match self.session_manager.close(&session_id) {
            Ok(()) => {
                self.recording_manager.cleanup_session(&session_id);
                self.terminal_status = "session closed".to_string();
                self.append_terminal_log(format!("\n# closed session {}\n", short_id(&session_id)));
            }
            Err(error) => {
                self.terminal_status = format!("close failed: {error}");
            }
        }
        cx.notify();
    }

    fn clear_terminal(&mut self, cx: &mut Context<Self>) {
        self.terminal_output.clear();
        self.terminal_screen.clear();
        self.terminal_status = "terminal cleared".to_string();
        cx.notify();
    }

    fn append_terminal_log(&mut self, text: impl AsRef<str>) {
        let text = text.as_ref();
        self.terminal_output.push_str(text);
        self.terminal_screen.advance(text.as_bytes());
        trim_terminal_output(&mut self.terminal_output);
    }

    fn append_terminal_bytes(&mut self, data: &[u8]) {
        self.terminal_screen.advance(data);
        self.terminal_output
            .push_str(&String::from_utf8_lossy(data));
        trim_terminal_output(&mut self.terminal_output);
    }

    fn update_host_key_policy(&mut self, policy: &'static str, cx: &mut Context<Self>) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_host_key_policy(policy))
        {
            Ok(settings) => {
                self.settings = settings;
                self.terminal_status = format!("host key policy set to {policy}");
                self.store_status.message = "settings saved".to_string();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.terminal_status = format!("failed to save host key policy: {error}");
                self.store_status.message = format!("settings save failed: {error}");
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    fn toggle_recording_auto_start(&mut self, cx: &mut Context<Self>) {
        self.settings.recording_auto_start = !self.settings.recording_auto_start;
        self.save_recording_settings(cx);
    }

    fn toggle_recording_io_labels(&mut self, cx: &mut Context<Self>) {
        self.settings.recording_include_io_labels = !self.settings.recording_include_io_labels;
        self.save_recording_settings(cx);
    }

    fn toggle_recording_timestamps(&mut self, cx: &mut Context<Self>) {
        self.settings.recording_include_timestamps = !self.settings.recording_include_timestamps;
        self.save_recording_settings(cx);
    }

    fn adjust_recording_memory_limit(&mut self, delta_mib: i64, cx: &mut Context<Self>) {
        let current_mib = (self.settings.recording_memory_limit_bytes / (1024 * 1024)).max(1);
        let next_mib = if delta_mib.is_negative() {
            current_mib.saturating_sub(delta_mib.unsigned_abs()).max(1)
        } else {
            current_mib.saturating_add(delta_mib as u64).min(512)
        };
        self.settings.recording_memory_limit_bytes = next_mib * 1024 * 1024;
        self.save_recording_settings(cx);
    }

    fn save_recording_settings(&mut self, cx: &mut Context<Self>) {
        self.recording_manager
            .set_memory_limit(self.settings.recording_memory_limit_bytes as usize);
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_recording_settings(&self.settings))
        {
            Ok(settings) => {
                self.settings = settings;
                self.recording_manager
                    .set_memory_limit(self.settings.recording_memory_limit_bytes as usize);
                self.store_status.message = "recording settings saved".to_string();
                self.store_status.ready = true;
                self.terminal_status = "recording settings saved".to_string();
            }
            Err(error) => {
                self.store_status.message = format!("recording settings save failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }

    fn toggle_keyword_highlights(&mut self, cx: &mut Context<Self>) {
        self.keyword_highlights.enabled = !self.keyword_highlights.enabled;
        self.save_keyword_highlights(cx);
    }

    fn toggle_keyword_highlights_wrapped(&mut self, cx: &mut Context<Self>) {
        self.keyword_highlights.across_wrapped_lines =
            !self.keyword_highlights.across_wrapped_lines;
        self.save_keyword_highlights(cx);
    }

    fn save_keyword_highlights(&mut self, cx: &mut Context<Self>) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_keyword_highlights(&self.keyword_highlights))
        {
            Ok(config) => {
                self.keyword_highlights = config;
                self.store_status.message = "keyword highlight settings saved".to_string();
                self.store_status.ready = true;
                self.terminal_status = "keyword highlight settings saved".to_string();
            }
            Err(error) => {
                self.store_status.message =
                    format!("keyword highlight settings save failed: {error}");
                self.store_status.ready = false;
                self.terminal_status = self.store_status.message.clone();
            }
        }
        cx.notify();
    }

    fn prompt_keyword_highlight_import(&mut self, cx: &mut Context<Self>) {
        if self.keyword_highlight_path_prompt.is_some() {
            self.terminal_status = "keyword highlight import picker is already open".to_string();
            cx.notify();
            return;
        }
        let options = PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from("Import keyword highlight JSON")),
        };
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        let receiver = cx.prompt_for_paths(options);
        self.keyword_highlight_path_prompt = Some(KeywordHighlightPathPromptKind::Import);
        self.terminal_status = "selecting keyword highlight import file".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(paths))) => match paths.into_iter().next() {
                    Some(path) => match std::fs::read_to_string(&path) {
                        Ok(raw) => match ConnectionStore::open_with_portable_key_path(
                            &config_dir,
                            portable_key_path.clone(),
                        )
                        .and_then(|store| store.import_keyword_highlights_json(&raw))
                        {
                            Ok((_, result)) => KeywordHighlightPathPromptResult::Imported {
                                imported_rules: result.imported_rules,
                                updated_rules: result.updated_rules,
                                total_rules: result.total_rules,
                            },
                            Err(error) => {
                                KeywordHighlightPathPromptResult::Failed(error.to_string())
                            }
                        },
                        Err(error) => KeywordHighlightPathPromptResult::Failed(error.to_string()),
                    },
                    None => KeywordHighlightPathPromptResult::Cancelled,
                },
                Ok(Ok(None)) => KeywordHighlightPathPromptResult::Cancelled,
                Ok(Err(error)) => KeywordHighlightPathPromptResult::Failed(error.to_string()),
                Err(_) => KeywordHighlightPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_keyword_highlight_import_result(result);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn apply_keyword_highlight_import_result(&mut self, result: KeywordHighlightPathPromptResult) {
        self.keyword_highlight_path_prompt = None;
        match result {
            KeywordHighlightPathPromptResult::Imported {
                imported_rules,
                updated_rules,
                total_rules,
            } => {
                self.refresh_keyword_highlights();
                self.terminal_status = format!(
                    "imported {imported_rules} keyword highlight rule(s), updated {updated_rules}, total {total_rules}"
                );
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = true;
            }
            KeywordHighlightPathPromptResult::Cancelled => {
                self.terminal_status = "keyword highlight import cancelled".to_string();
            }
            KeywordHighlightPathPromptResult::Failed(error) => {
                self.terminal_status = format!("keyword highlight import failed: {error}");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
            }
            KeywordHighlightPathPromptResult::Closed => {
                self.terminal_status =
                    "keyword highlight import picker closed before returning".to_string();
            }
        }
    }

    fn refresh_keyword_highlights(&mut self) {
        if let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            if let Ok(config) = store.load_keyword_highlights() {
                self.keyword_highlights = config;
            }
        }
    }

    fn update_cloud_sync_provider(&mut self, provider: &'static str, cx: &mut Context<Self>) {
        self.cloud_sync_settings.provider = provider.to_string();
        self.cloud_sync_status = format!("provider set to {provider}; save to persist");
        cx.notify();
    }

    fn toggle_cloud_sync_enabled(&mut self, cx: &mut Context<Self>) {
        self.cloud_sync_settings.enabled = !self.cloud_sync_settings.enabled;
        self.cloud_sync_status = if self.cloud_sync_settings.enabled {
            "cloud sync enabled; save to persist"
        } else {
            "cloud sync disabled; save to persist"
        }
        .to_string();
        cx.notify();
    }

    fn toggle_s3_virtual_host_style(&mut self, cx: &mut Context<Self>) {
        self.cloud_sync_settings.s3.virtual_host_style =
            !self.cloud_sync_settings.s3.virtual_host_style;
        self.cloud_sync_status = if self.cloud_sync_settings.s3.virtual_host_style {
            "S3 virtual-host style enabled; save to persist"
        } else {
            "S3 path-style URLs enabled; save to persist"
        }
        .to_string();
        cx.notify();
    }

    fn save_cloud_sync_settings(&mut self, cx: &mut Context<Self>) {
        let mut next = self.cloud_sync_settings.clone();
        if !self.cloud_sync_secret_draft.webdav_password.is_empty() {
            next.webdav.password = Some(self.cloud_sync_secret_draft.webdav_password.clone());
        }
        if !self.cloud_sync_secret_draft.s3_access_key_id.is_empty() {
            next.s3.access_key_id = Some(self.cloud_sync_secret_draft.s3_access_key_id.clone());
        }
        if !self.cloud_sync_secret_draft.s3_secret_access_key.is_empty() {
            next.s3.secret_access_key =
                Some(self.cloud_sync_secret_draft.s3_secret_access_key.clone());
        }
        if !self.cloud_sync_secret_draft.s3_session_token.is_empty() {
            next.s3.session_token = Some(self.cloud_sync_secret_draft.s3_session_token.clone());
        }
        if !self
            .cloud_sync_secret_draft
            .google_drive_access_token
            .is_empty()
        {
            next.google_drive.access_token = Some(
                self.cloud_sync_secret_draft
                    .google_drive_access_token
                    .clone(),
            );
        }
        if !self
            .cloud_sync_secret_draft
            .google_drive_refresh_token
            .is_empty()
        {
            next.google_drive.refresh_token = Some(
                self.cloud_sync_secret_draft
                    .google_drive_refresh_token
                    .clone(),
            );
        }
        if !self
            .cloud_sync_secret_draft
            .google_drive_client_secret
            .is_empty()
        {
            next.google_drive.client_secret = Some(
                self.cloud_sync_secret_draft
                    .google_drive_client_secret
                    .clone(),
            );
        }
        if !self
            .cloud_sync_secret_draft
            .onedrive_access_token
            .is_empty()
        {
            next.onedrive.access_token =
                Some(self.cloud_sync_secret_draft.onedrive_access_token.clone());
        }
        if !self
            .cloud_sync_secret_draft
            .onedrive_refresh_token
            .is_empty()
        {
            next.onedrive.refresh_token =
                Some(self.cloud_sync_secret_draft.onedrive_refresh_token.clone());
        }
        if !self
            .cloud_sync_secret_draft
            .onedrive_client_secret
            .is_empty()
        {
            next.onedrive.client_secret =
                Some(self.cloud_sync_secret_draft.onedrive_client_secret.clone());
        }
        if !self
            .cloud_sync_secret_draft
            .aliyun_drive_access_token
            .is_empty()
        {
            next.aliyun_drive.access_token = Some(
                self.cloud_sync_secret_draft
                    .aliyun_drive_access_token
                    .clone(),
            );
        }
        if !self
            .cloud_sync_secret_draft
            .aliyun_drive_refresh_token
            .is_empty()
        {
            next.aliyun_drive.refresh_token = Some(
                self.cloud_sync_secret_draft
                    .aliyun_drive_refresh_token
                    .clone(),
            );
        }
        if !self
            .cloud_sync_secret_draft
            .aliyun_drive_client_secret
            .is_empty()
        {
            next.aliyun_drive.client_secret = Some(
                self.cloud_sync_secret_draft
                    .aliyun_drive_client_secret
                    .clone(),
            );
        }
        if !self.cloud_sync_secret_draft.gitee_token.is_empty() {
            next.gitee_snippet.access_token =
                Some(self.cloud_sync_secret_draft.gitee_token.clone());
        }
        if !self.cloud_sync_secret_draft.github_token.is_empty() {
            next.github_gist.access_token = Some(self.cloud_sync_secret_draft.github_token.clone());
        }

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_cloud_sync_settings(next))
        {
            Ok(saved) => {
                self.cloud_sync_settings = saved;
                self.cloud_sync_secret_draft = CloudSyncSecretDraft::default();
                self.cloud_sync_status = "cloud sync settings saved".to_string();
                self.store_status.message = "cloud sync settings saved".to_string();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.cloud_sync_status = format!("cloud sync settings save failed: {error}");
                self.store_status.message = self.cloud_sync_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    fn update_ai_profile(&mut self, profile_id: &'static str, cx: &mut Context<Self>) {
        self.ai_settings.active_profile_id = profile_id.to_string();
        self.sync_ai_drafts_from_active_profile();
        self.ai_status = format!("AI provider set to {profile_id}; save to persist");
        cx.notify();
    }

    fn toggle_ai_enabled(&mut self, cx: &mut Context<Self>) {
        self.ai_settings.enabled = !self.ai_settings.enabled;
        self.ai_status = if self.ai_settings.enabled {
            "AI enabled; save to persist"
        } else {
            "AI disabled; save to persist"
        }
        .to_string();
        cx.notify();
    }

    fn set_ai_mode(&mut self, mode: AiMode, cx: &mut Context<Self>) {
        self.ai_settings.default_mode = mode;
        self.ai_status = "AI mode edited; save to persist".to_string();
        cx.notify();
    }

    fn set_ai_command_mode(&mut self, mode: AgentCommandExecutionMode, cx: &mut Context<Self>) {
        self.ai_settings.agent_command_execution_mode = mode;
        self.ai_status = "Agent command policy edited; save to persist".to_string();
        cx.notify();
    }

    fn toggle_ai_background_execution(&mut self, cx: &mut Context<Self>) {
        self.ai_settings.agent_background_execution_enabled =
            !self.ai_settings.agent_background_execution_enabled;
        self.ai_status = if self.ai_settings.agent_background_execution_enabled {
            "Agent background execution enabled; save to persist"
        } else {
            "Agent background execution disabled; save to persist"
        }
        .to_string();
        cx.notify();
    }

    fn begin_ai_chat_job(&mut self) -> (u64, Arc<AtomicBool>) {
        self.ai_chat_job_id = self.ai_chat_job_id.wrapping_add(1).max(1);
        let cancel = Arc::new(AtomicBool::new(false));
        self.ai_chat_cancel = Some(cancel.clone());
        (self.ai_chat_job_id, cancel)
    }

    fn upsert_ai_agent_step(
        &mut self,
        step_index: u16,
        status: AiAgentStepStatus,
        title: impl Into<String>,
        detail: impl Into<String>,
    ) {
        let title = title.into();
        let detail = detail.into();
        if let Some(step) = self
            .ai_agent_steps
            .iter_mut()
            .find(|step| step.step_index == step_index)
        {
            step.status = status;
            step.title = title;
            step.detail = detail;
        } else {
            self.ai_agent_steps.push(AiAgentStepView {
                step_index,
                status,
                title,
                detail,
            });
        }
        let overflow = self.ai_agent_steps.len().saturating_sub(8);
        if overflow > 0 {
            self.ai_agent_steps.drain(..overflow);
        }
    }

    fn cancel_ai_chat(&mut self, cx: &mut Context<Self>) {
        if let Some(cancel) = self.ai_chat_cancel.as_ref() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.ai_chat_job_id = self.ai_chat_job_id.wrapping_add(1).max(1);
        self.ai_chat_pending = false;
        self.ai_chat_cancel = None;
        let cancelled_step = self
            .ai_agent_loop
            .as_ref()
            .map(|state| state.step_index)
            .or_else(|| self.ai_agent_steps.last().map(|step| step.step_index));
        if let Some(state) = self.ai_agent_loop.take()
            && let Some(marker_id) = state.marker_id.as_deref()
        {
            self.ai_agent_capture.cancel(marker_id);
        }
        self.ai_agent_capture = AgentOutputCaptureProcessor::new();
        self.ai_agent_task_prompt = None;
        self.ai_command_cards.clear();
        self.ai_response_preview = "AI request cancelled".to_string();
        self.ai_status = "AI request cancelled".to_string();
        if let Some(step_index) = cancelled_step {
            self.upsert_ai_agent_step(
                step_index,
                AiAgentStepStatus::Cancelled,
                "Cancelled",
                "AI Agent request was cancelled",
            );
        }
        self.store_status.message = self.ai_status.clone();
        cx.notify();
    }

    fn save_ai_settings(&mut self, cx: &mut Context<Self>) {
        let mut next = self.ai_settings.clone();
        let active_id = next.active_profile_id.clone();
        let mut active_kind = None;
        let mut active_name = active_id.clone();
        let mut active_base_url = none_if_blank(&self.ai_base_url_draft);
        let active_model = self.ai_model_draft.trim().to_string();

        if let Some(profile) = next
            .provider_profiles
            .iter_mut()
            .find(|profile| profile.id == active_id)
        {
            profile.enabled = true;
            if !active_model.is_empty() {
                profile.model = active_model.clone();
            }
            profile.base_url = active_base_url.clone();
            if !self.ai_secret_draft.is_empty() {
                profile.api_key = Some(self.ai_secret_draft.clone());
            }
            active_kind = Some(profile.provider_kind.clone());
            active_name = profile.name.clone();
            active_base_url = profile.base_url.clone();
        }

        if let Some(kind) = active_kind.clone() {
            let credential = AiProviderCredential {
                id: active_id.clone(),
                name: active_name,
                provider_kind: kind.clone(),
                base_url: active_base_url.clone(),
                api_key: if self.ai_secret_draft.is_empty() {
                    next.provider_credentials
                        .iter()
                        .find(|credential| credential.id == active_id)
                        .and_then(|credential| credential.api_key.clone())
                } else {
                    Some(self.ai_secret_draft.clone())
                },
                enabled: true,
            };
            if let Some(existing) = next
                .provider_credentials
                .iter_mut()
                .find(|credential| credential.id == active_id)
            {
                *existing = credential;
            } else {
                next.provider_credentials.push(credential);
            }

            if !active_model.is_empty() {
                let model_id = if active_base_url
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                    || kind == AiProviderKind::OpenaiCompatible
                {
                    ai_model_id_for_credential(&active_id, &active_model)
                } else {
                    ai_model_id_for_provider(&kind, &active_model)
                };
                let model_index = next
                    .models
                    .iter()
                    .position(|model| model.credential_id.as_deref() == Some(active_id.as_str()))
                    .or_else(|| next.models.iter().position(|model| model.id == model_id));
                if let Some(model_index) = model_index {
                    let model = &mut next.models[model_index];
                    model.id = model_id.clone();
                    model.name = active_model.clone();
                    model.provider_kind = Some(kind);
                    model.credential_id = active_base_url
                        .as_deref()
                        .is_some_and(|value| !value.trim().is_empty())
                        .then(|| active_id.clone());
                    model.enabled = true;
                } else {
                    next.models.push(nyaterm_domain::AiModelConfigItem {
                        id: model_id.clone(),
                        name: active_model,
                        provider_kind: Some(kind),
                        credential_id: active_base_url
                            .as_deref()
                            .is_some_and(|value| !value.trim().is_empty())
                            .then(|| active_id.clone()),
                        enabled: true,
                        source: nyaterm_domain::AiModelSource::Manual,
                        last_seen_at: None,
                    });
                }
                next.default_model_id = Some(model_id);
            }
        }

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.save_ai_settings(next))
        {
            Ok(saved) => {
                self.ai_settings = saved;
                self.ai_secret_draft.clear();
                self.sync_ai_drafts_from_active_profile();
                self.refresh_ai_usage_counts();
                self.ai_status = "AI settings saved".to_string();
                self.store_status.message = "AI settings saved".to_string();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.ai_status = format!("AI settings save failed: {error}");
                self.store_status.message = self.ai_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    fn discover_ai_models(&mut self, cx: &mut Context<Self>) {
        if self.ai_discovery_pending {
            self.ai_status = "AI model discovery already running".to_string();
            cx.notify();
            return;
        }

        let credential = match self.active_ai_discovery_credential() {
            Ok(credential) => credential,
            Err(error) => {
                self.ai_status = error;
                cx.notify();
                return;
            }
        };
        if credential
            .base_url
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            self.ai_status = "AI model discovery requires a Base URL".to_string();
            cx.notify();
            return;
        }

        let settings = self.ai_settings.clone();
        let profile_id = credential.id.clone();
        let tx = self.ai_discovery_tx.clone();
        self.ai_discovery_pending = true;
        self.ai_status = "Discovering AI models...".to_string();
        std::thread::spawn(move || {
            let result = discover_openai_compatible_models(&settings, &credential);
            let _ = tx.send(AiDiscoveryJobResult { profile_id, result });
        });
        cx.notify();
    }

    fn active_ai_discovery_credential(&self) -> Result<AiProviderCredential, String> {
        let active_id = self.ai_settings.active_profile_id.as_str();
        let profile = self
            .ai_settings
            .provider_profiles
            .iter()
            .find(|profile| profile.id == active_id)
            .ok_or_else(|| format!("AI provider profile '{active_id}' is not configured"))?;
        let current_credential = self
            .ai_settings
            .provider_credentials
            .iter()
            .find(|credential| credential.id == active_id);

        Ok(AiProviderCredential {
            id: profile.id.clone(),
            name: profile.name.clone(),
            provider_kind: profile.provider_kind.clone(),
            base_url: none_if_blank(&self.ai_base_url_draft),
            api_key: if self.ai_secret_draft.is_empty() {
                current_credential
                    .and_then(|credential| credential.api_key.clone())
                    .or_else(|| profile.api_key.clone())
            } else {
                Some(self.ai_secret_draft.clone())
            },
            enabled: true,
        })
    }

    fn drain_ai_discovery_events(&mut self) {
        while let Ok(event) = self.ai_discovery_rx.try_recv() {
            self.ai_discovery_pending = false;
            match event.result {
                Ok(discoveries) if discoveries.is_empty() => {
                    self.ai_status = "AI discovery returned no models".to_string();
                }
                Ok(discoveries) => {
                    let count = self.apply_ai_model_discoveries(&event.profile_id, discoveries);
                    self.ai_status = format!("Discovered {count} AI model(s); save to persist");
                    self.store_status.message = self.ai_status.clone();
                    self.store_status.ready = true;
                }
                Err(error) => {
                    self.ai_status = format!("AI model discovery failed: {error}");
                    self.store_status.message = self.ai_status.clone();
                    self.store_status.ready = false;
                }
            }
        }
    }

    fn apply_ai_model_discoveries(
        &mut self,
        profile_id: &str,
        discoveries: Vec<AiModelDiscovery>,
    ) -> usize {
        let discoveries = merge_model_discoveries(discoveries);
        let first_discovery = discoveries.first().cloned();
        let discovered_ids: HashSet<String> =
            discoveries.iter().map(|model| model.id.clone()).collect();
        let last_seen_at = Some(now_rfc3339());

        for discovery in &discoveries {
            if let Some(model) = self
                .ai_settings
                .models
                .iter_mut()
                .find(|model| model.id == discovery.id)
            {
                model.name = discovery.name.clone();
                model.provider_kind = discovery.provider_kind.clone();
                model.credential_id = discovery.credential_id.clone();
                model.enabled = true;
                model.source = discovery.source.clone();
                model.last_seen_at = last_seen_at.clone();
            } else {
                self.ai_settings
                    .models
                    .push(nyaterm_domain::AiModelConfigItem {
                        id: discovery.id.clone(),
                        name: discovery.name.clone(),
                        provider_kind: discovery.provider_kind.clone(),
                        credential_id: discovery.credential_id.clone(),
                        enabled: true,
                        source: discovery.source.clone(),
                        last_seen_at: last_seen_at.clone(),
                    });
            }
        }

        if self.ai_settings.active_profile_id == profile_id {
            let draft_model_id = ai_model_id_for_credential(profile_id, self.ai_model_draft.trim());
            if discovered_ids.contains(&draft_model_id) {
                self.ai_settings.default_model_id = Some(draft_model_id);
            } else {
                let current_default_is_valid = self
                    .ai_settings
                    .default_model_id
                    .as_deref()
                    .is_some_and(|id| {
                        self.ai_settings
                            .models
                            .iter()
                            .any(|model| model.enabled && model.id == id)
                    });
                if !current_default_is_valid && let Some(first_discovery) = first_discovery.as_ref()
                {
                    self.ai_settings.default_model_id = Some(first_discovery.id.clone());
                }
                if self.ai_model_draft.trim().is_empty()
                    && let Some(first_discovery) = first_discovery.as_ref()
                {
                    self.ai_model_draft = first_discovery.name.clone();
                }
            }
        }

        discoveries.len()
    }

    fn start_ai_ask(&mut self, cx: &mut Context<Self>) {
        if self.ai_chat_pending {
            self.ai_response_preview = "AI request already running".to_string();
            cx.notify();
            return;
        }
        if self.ai_agent_loop.is_some() {
            self.ai_response_preview = "AI Agent step already running".to_string();
            self.ai_status = self.ai_response_preview.clone();
            cx.notify();
            return;
        }
        let prompt = self.ai_prompt_draft.trim().to_string();
        if prompt.is_empty() {
            self.ai_response_preview = "Enter a prompt first".to_string();
            cx.notify();
            return;
        }
        if !self.ai_settings.enabled {
            self.ai_response_preview = "AI assistant is disabled".to_string();
            cx.notify();
            return;
        }

        let settings = self.ai_settings.clone();
        let mode = settings.default_mode.clone();
        if mode == AiMode::Agent && self.active_session_id.is_none() {
            self.ai_response_preview =
                "Start a terminal session before running Agent mode".to_string();
            self.ai_status = self.ai_response_preview.clone();
            cx.notify();
            return;
        }
        let session_id = self.ai_chat_session_id.clone();
        let request = AiChatRequest {
            stream_id: None,
            session_id: Some(session_id.clone()),
            connection_id: self.active_session_id.clone(),
            terminal_session_id: self.active_session_id.clone(),
            mode: mode.clone(),
            model_id: settings.default_model_id.clone(),
            model_name: None,
            action: AiAction::GenerateCommand,
            user_input: prompt,
            context: self.ai_terminal_context(),
            options: Default::default(),
        };
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        let tx = self.ai_chat_tx.clone();
        let (job_id, cancel) = self.begin_ai_chat_job();

        if mode == AiMode::Agent {
            self.ai_agent_task_prompt = Some(request.user_input.clone());
            self.ai_agent_step_index = 0;
            self.ai_agent_steps.clear();
            self.upsert_ai_agent_step(
                0,
                AiAgentStepStatus::Planning,
                "Planning",
                truncate_preview(&request.user_input, 120),
            );
        } else {
            self.ai_agent_task_prompt = None;
            self.ai_agent_step_index = 0;
            self.ai_agent_loop = None;
            self.ai_agent_steps.clear();
        }
        self.ai_chat_pending = true;
        self.ai_response_preview = if mode == AiMode::Agent {
            "Running AI Agent step...".to_string()
        } else {
            "Running AI request...".to_string()
        };
        self.ai_command_cards.clear();
        self.ai_status = if mode == AiMode::Agent {
            "AI Agent step started".to_string()
        } else {
            "AI Ask request started".to_string()
        };
        std::thread::spawn(move || {
            let result = run_ai_ask_job(
                config_dir,
                portable_key_path,
                settings,
                request,
                Some(tx.clone()),
                cancel,
                job_id,
            );
            let _ = tx.send(AiChatWorkerEvent::Finished(AiChatJobResult {
                job_id,
                session_id,
                result,
            }));
        });
        cx.notify();
    }

    fn ai_terminal_context(&self) -> AiContext {
        let ssh = self.active_ssh_config.as_ref();
        let active_session = self.active_session_id.as_deref().and_then(|session_id| {
            self.session_manager
                .list_sessions()
                .ok()
                .and_then(|sessions| {
                    sessions
                        .into_iter()
                        .find(|session| session.id == session_id)
                })
        });
        AiContext {
            connection_name: ssh.map(|config| config.name.clone()),
            host: ssh.map(|config| config.host.clone()),
            port: ssh.map(|config| config.port),
            username: ssh.map(|config| config.username.clone()),
            cwd: active_session
                .as_ref()
                .and_then(|session| session.working_dir.as_ref())
                .map(|path| path.display().to_string()),
            os: None,
            arch: Some(std::env::consts::ARCH.to_string()),
            recent_output: recent_terminal_output(&self.terminal_output, 80),
            selected_text: String::new(),
            input_buffer: String::new(),
        }
    }

    fn handle_ai_prompt_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "enter" => self.start_ai_ask(cx),
            "backspace" => {
                self.ai_prompt_draft.pop();
                cx.notify();
            }
            "escape" => {
                self.ai_response_preview = "AI prompt blurred".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.ai_prompt_draft.push_str(input);
                    cx.notify();
                }
            }
        }
    }

    fn drain_ai_chat_events(&mut self, cx: &mut Context<Self>) {
        while let Ok(event) = self.ai_chat_rx.try_recv() {
            match event {
                AiChatWorkerEvent::Delta {
                    job_id,
                    session_id,
                    text_delta,
                    reasoning_delta,
                } => {
                    if job_id != self.ai_chat_job_id {
                        continue;
                    }
                    if self.ai_response_preview == "Running AI request..." {
                        self.ai_response_preview.clear();
                    }
                    self.ai_response_preview.push_str(&text_delta);
                    self.ai_response_preview = truncate_preview(&self.ai_response_preview, 320);
                    self.ai_status = if reasoning_delta
                        .as_deref()
                        .is_some_and(|delta| !delta.trim().is_empty())
                    {
                        "AI stream receiving; reasoning captured".to_string()
                    } else {
                        "AI stream receiving".to_string()
                    };
                    self.store_status.message = format!("AI session {session_id} streaming");
                    self.store_status.ready = true;
                    cx.notify();
                }
                AiChatWorkerEvent::AgentToolCallDelta {
                    job_id,
                    session_id,
                    tool_name,
                    arguments_delta_len,
                } => {
                    if job_id != self.ai_chat_job_id {
                        continue;
                    }
                    let tool_label = tool_name
                        .as_deref()
                        .filter(|name| !name.trim().is_empty())
                        .unwrap_or("tool");
                    self.ai_status = if arguments_delta_len == 0 {
                        format!("AI Agent selected {tool_label}")
                    } else {
                        format!(
                            "AI Agent streaming {tool_label} arguments (+{arguments_delta_len} chars)"
                        )
                    };
                    let step_index = self
                        .ai_agent_steps
                        .last()
                        .map(|step| step.step_index)
                        .unwrap_or(0);
                    self.upsert_ai_agent_step(
                        step_index,
                        AiAgentStepStatus::Tool,
                        format!("Tool {tool_label}"),
                        if arguments_delta_len == 0 {
                            "Provider selected an Agent tool".to_string()
                        } else {
                            format!("Streaming arguments (+{arguments_delta_len} chars)")
                        },
                    );
                    self.store_status.message =
                        format!("AI session {session_id} streaming Agent tool call");
                    self.store_status.ready = true;
                    cx.notify();
                }
                AiChatWorkerEvent::AgentBackgroundFinished {
                    job_id,
                    state,
                    result,
                } => {
                    if job_id != self.ai_chat_job_id {
                        continue;
                    }
                    self.ai_chat_cancel = None;
                    let Some(active_state) = self.ai_agent_loop.take() else {
                        continue;
                    };
                    if active_state.background_job_id != Some(job_id) {
                        self.ai_agent_loop = Some(active_state);
                        continue;
                    }
                    match result {
                        Ok(observation) => {
                            self.ai_status = match observation.exit_code {
                                Some(code) => {
                                    format!("AI Agent background command exited with {code}")
                                }
                                None => "AI Agent background command completed".to_string(),
                            };
                            self.upsert_ai_agent_step(
                                state.step_index,
                                AiAgentStepStatus::Completed,
                                "Observed",
                                observation_summary(&observation),
                            );
                            self.start_ai_agent_continuation(state, observation, cx);
                        }
                        Err(error) => {
                            self.ai_status = format!("AI Agent background command failed: {error}");
                            self.ai_response_preview = self.ai_status.clone();
                            self.upsert_ai_agent_step(
                                state.step_index,
                                AiAgentStepStatus::Failed,
                                "Failed",
                                truncate_preview(&error, 140),
                            );
                            self.store_status.message = self.ai_status.clone();
                            self.store_status.ready = false;
                            cx.notify();
                        }
                    }
                }
                AiChatWorkerEvent::Finished(event) => {
                    if event.job_id != self.ai_chat_job_id {
                        continue;
                    }
                    self.ai_chat_pending = false;
                    self.ai_chat_cancel = None;
                    match event.result {
                        Ok(output) => {
                            let command_count = output.command_cards.len();
                            self.ai_response_preview = if output.text.trim().is_empty() {
                                "AI returned an empty response".to_string()
                            } else {
                                truncate_preview(&output.text, 320)
                            };
                            let mode_label = if output.mode == AiMode::Agent {
                                "AI Agent"
                            } else {
                                "AI Ask"
                            };
                            self.ai_status = format!(
                                "{mode_label} completed; {} command card(s) parsed",
                                command_count
                            );
                            if output.reasoning.is_some() {
                                self.ai_status.push_str("; reasoning captured");
                            }
                            if let Some(note) = output.approval_note.as_deref() {
                                self.ai_status.push_str("; ");
                                self.ai_status.push_str(note);
                            }
                            let auto_execute_first = output.auto_execute_first;
                            let agent_step_index = self
                                .ai_agent_steps
                                .last()
                                .map(|step| step.step_index)
                                .unwrap_or(0);
                            if output.mode == AiMode::Agent {
                                let (step_status, step_title) = if command_count == 0 {
                                    (AiAgentStepStatus::Completed, "Final Answer")
                                } else if auto_execute_first {
                                    (AiAgentStepStatus::Running, "Auto Execute")
                                } else {
                                    (AiAgentStepStatus::NeedsApproval, "Needs Approval")
                                };
                                self.upsert_ai_agent_step(
                                    agent_step_index,
                                    step_status,
                                    step_title,
                                    truncate_preview(&output.text, 140),
                                );
                            }
                            self.ai_command_cards = output.command_cards;
                            self.store_status.message =
                                format!("AI session {} updated", event.session_id);
                            self.store_status.ready = true;
                            self.ai_prompt_draft.clear();
                            self.refresh_ai_usage_counts();
                            if output.mode == AiMode::Agent {
                                if command_count == 0 {
                                    self.ai_agent_loop = None;
                                    self.ai_agent_task_prompt = None;
                                } else if !auto_execute_first {
                                    self.ai_status.push_str("; awaiting command approval");
                                }
                            }
                            if auto_execute_first && !self.ai_command_cards.is_empty() {
                                self.apply_ai_command_card(0, true, cx);
                            }
                        }
                        Err(error) => {
                            self.ai_response_preview = format!("AI request failed: {error}");
                            self.ai_command_cards.clear();
                            self.ai_status = self.ai_response_preview.clone();
                            if self.ai_agent_task_prompt.is_some() {
                                let step_index = self
                                    .ai_agent_steps
                                    .last()
                                    .map(|step| step.step_index)
                                    .unwrap_or(0);
                                self.upsert_ai_agent_step(
                                    step_index,
                                    AiAgentStepStatus::Failed,
                                    "Failed",
                                    truncate_preview(&error, 140),
                                );
                            }
                            self.store_status.message = self.ai_status.clone();
                            self.store_status.ready = false;
                        }
                    }
                }
            }
        }
    }

    fn insert_ai_command_card(&mut self, index: usize, cx: &mut Context<Self>) {
        self.apply_ai_command_card(index, false, cx);
    }

    fn run_ai_command_card(&mut self, index: usize, cx: &mut Context<Self>) {
        self.apply_ai_command_card(index, true, cx);
    }

    fn save_ai_command_card(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(card) = self.ai_command_cards.get(index).cloned() else {
            self.ai_status = "AI command card is no longer available".to_string();
            cx.notify();
            return;
        };
        let command_text = card.command.trim();
        if command_text.is_empty() {
            self.ai_status = "AI command card has no command".to_string();
            cx.notify();
            return;
        }

        let result = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| {
            let config = store.load_quick_commands()?;
            let category_name = ai_command_card_category_name(&card);
            let existing_category = config
                .categories
                .iter()
                .find(|category| category.name == category_name)
                .cloned();
            let (category_id, new_category) = match existing_category {
                Some(category) => (category.id, None),
                None => {
                    let id = unique_quick_command_category_id(&config.categories, &category_name);
                    (
                        id.clone(),
                        Some(QuickCommandCategory {
                            id,
                            name: category_name,
                        }),
                    )
                }
            };
            let label = if card.title.trim().is_empty() {
                "AI Command".to_string()
            } else {
                card.title.trim().to_string()
            };
            let description = if card.explanation.trim().is_empty() {
                None
            } else {
                Some(card.explanation.trim().to_string())
            };
            store.upsert_quick_command(
                QuickCommand {
                    id: format!("ai-{}", uuid()),
                    label: label.clone(),
                    command: command_text.to_string(),
                    category_id: Some(category_id),
                    description,
                    color_tag: Some("blue".to_string()),
                    icon_tag: Some("terminal".to_string()),
                    pinned: Some(false),
                    execution_mode: Some("append".to_string()),
                    source: Some("ai".to_string()),
                    risk_level: card.risk_level.clone(),
                    updated_at: None,
                    created_at: None,
                    use_count: None,
                },
                new_category,
            )?;
            store
                .append_ai_audit(AppendAiAuditRequest {
                    connection_id: self.active_session_id.clone(),
                    action: "ai.save_quick_command".to_string(),
                    user_input: Some(self.ai_response_preview.clone()),
                    generated_command: Some(card.command.clone()),
                    risk_level: card.risk_level.clone(),
                    inserted_to_terminal: false,
                    executed: false,
                    blocked: false,
                })
                .map(|_| label)
        });

        match result {
            Ok(label) => {
                self.refresh_ai_usage_counts();
                self.refresh_quick_commands();
                self.ai_status = format!("Saved AI command card '{}' to Quick Commands", label);
                self.store_status.message = self.ai_status.clone();
                self.store_status.ready = true;
            }
            Err(error) => {
                self.ai_status = format!("Quick command save failed: {error}");
                self.store_status.message = self.ai_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    fn insert_quick_command(&mut self, index: usize, cx: &mut Context<Self>) {
        self.apply_quick_command(index, false, cx);
    }

    fn run_quick_command(&mut self, index: usize, cx: &mut Context<Self>) {
        self.apply_quick_command(index, true, cx);
    }

    fn insert_history_command(&mut self, index: usize, cx: &mut Context<Self>) {
        self.apply_history_command(index, false, cx);
    }

    fn run_history_command(&mut self, index: usize, cx: &mut Context<Self>) {
        self.apply_history_command(index, true, cx);
    }

    fn delete_history_command(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(entry) = self.command_history.get(index).cloned() else {
            self.terminal_status = "history command is no longer available".to_string();
            cx.notify();
            return;
        };
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.delete_command_history(&entry.command))
        {
            Ok(()) => {
                self.refresh_command_history();
                self.terminal_status = format!("deleted history command '{}'", entry.command);
            }
            Err(error) => {
                self.terminal_status = format!("history delete failed: {error}");
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
            }
        }
        cx.notify();
    }

    fn insert_command_search_result(&mut self, index: usize, cx: &mut Context<Self>) {
        self.apply_command_search_result(index, false, cx);
    }

    fn run_command_search_result(&mut self, index: usize, cx: &mut Context<Self>) {
        self.apply_command_search_result(index, true, cx);
    }

    fn apply_command_search_result(&mut self, index: usize, execute: bool, cx: &mut Context<Self>) {
        if self.active_session_id.is_none() {
            self.terminal_status =
                "start a terminal session before using command search".to_string();
            cx.notify();
            return;
        }
        let Some(result) = self.command_search_results().into_iter().nth(index) else {
            self.terminal_status = "command search result is no longer available".to_string();
            cx.notify();
            return;
        };
        let mut command = result.command.trim().to_string();
        if command.is_empty() {
            self.terminal_status = "command search result is empty".to_string();
            cx.notify();
            return;
        }
        if execute && !command.ends_with('\n') {
            command.push('\n');
        }
        self.send_terminal_input(command.into_bytes(), cx);
        self.terminal_status = if execute {
            format!("ran search result '{}'", result.display)
        } else {
            format!("inserted search result '{}'", result.display)
        };
        cx.notify();
    }

    fn apply_history_command(&mut self, index: usize, execute: bool, cx: &mut Context<Self>) {
        if self.active_session_id.is_none() {
            self.terminal_status = "start a terminal session before using history".to_string();
            cx.notify();
            return;
        }
        let Some(entry) = self.command_history.get(index).cloned() else {
            self.terminal_status = "history command is no longer available".to_string();
            cx.notify();
            return;
        };
        let mut command = entry.command.trim().to_string();
        if command.is_empty() {
            self.terminal_status = "history command is empty".to_string();
            cx.notify();
            return;
        }
        if execute && !command.ends_with('\n') {
            command.push('\n');
        }
        self.send_terminal_input(command.into_bytes(), cx);
        self.terminal_status = if execute {
            format!("ran history command '{}'", entry.command)
        } else {
            format!("inserted history command '{}'", entry.command)
        };
        cx.notify();
    }

    fn apply_quick_command(&mut self, index: usize, execute: bool, cx: &mut Context<Self>) {
        if self.active_session_id.is_none() {
            self.terminal_status =
                "start a terminal session before using a quick command".to_string();
            cx.notify();
            return;
        }
        let Some(command) = sorted_quick_commands(&self.quick_commands)
            .into_iter()
            .take(5)
            .nth(index)
        else {
            self.terminal_status = "quick command is no longer available".to_string();
            cx.notify();
            return;
        };
        let mut command_text = command.command.trim().to_string();
        if command_text.is_empty() {
            self.terminal_status = "quick command has no command text".to_string();
            cx.notify();
            return;
        }
        if execute && !command_text.ends_with('\n') {
            command_text.push('\n');
        }

        if let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            if let Err(error) = store.increment_quick_command_use_count(&command.id) {
                self.store_status.message =
                    format!("quick command use count update failed: {error}");
                self.store_status.ready = false;
            } else {
                self.refresh_quick_commands();
            }
        }

        self.send_terminal_input(command_text.into_bytes(), cx);
        self.terminal_status = if execute {
            format!("ran quick command '{}'", command.label)
        } else {
            format!("inserted quick command '{}'", command.label)
        };
        cx.notify();
    }

    fn apply_ai_command_card(&mut self, index: usize, execute: bool, cx: &mut Context<Self>) {
        if self.active_session_id.is_none() {
            self.ai_status = "Start a terminal session before using an AI command".to_string();
            cx.notify();
            return;
        }
        let Some(card) = self.ai_command_cards.get(index).cloned() else {
            self.ai_status = "AI command card is no longer available".to_string();
            cx.notify();
            return;
        };
        let mut command = card.command.trim().to_string();
        if command.is_empty() {
            self.ai_status = "AI command card has no command".to_string();
            cx.notify();
            return;
        }
        let should_continue_agent = execute && is_agent_command_card(&card);
        if should_continue_agent && self.ai_settings.agent_background_execution_enabled {
            match self.begin_ai_agent_background_execution(&card.command, cx) {
                Ok(()) => {
                    self.record_ai_command_card_audit(&card, true, false);
                    cx.notify();
                }
                Err(error) => {
                    self.ai_status = error;
                    let step_index = self
                        .ai_agent_steps
                        .last()
                        .map(|step| step.step_index)
                        .unwrap_or(0);
                    self.upsert_ai_agent_step(
                        step_index,
                        AiAgentStepStatus::Failed,
                        "Failed",
                        self.ai_status.clone(),
                    );
                    cx.notify();
                }
            }
            return;
        }
        if execute && !command.ends_with('\n') {
            command.push('\n');
        }
        let input_bytes = if should_continue_agent {
            match self.begin_ai_agent_observation(&card.command) {
                Ok(Some(wrapped_command)) => wrapped_command.into_bytes(),
                Ok(None) => command.clone().into_bytes(),
                Err(error) => {
                    self.ai_status = error;
                    let step_index = self
                        .ai_agent_steps
                        .last()
                        .map(|step| step.step_index)
                        .unwrap_or(0);
                    self.upsert_ai_agent_step(
                        step_index,
                        AiAgentStepStatus::Failed,
                        "Failed",
                        self.ai_status.clone(),
                    );
                    cx.notify();
                    return;
                }
            }
        } else {
            command.clone().into_bytes()
        };

        self.record_ai_command_card_audit(&card, execute, true);

        self.send_terminal_input(input_bytes, cx);
        self.ai_status = if should_continue_agent {
            if let Some(state) = self.ai_agent_loop.as_ref().cloned() {
                self.upsert_ai_agent_step(
                    state.step_index,
                    AiAgentStepStatus::Running,
                    "Running",
                    truncate_preview(&state.command, 140),
                );
                format!(
                    "AI Agent observing command output for step {}/{}",
                    state.step_index + 1,
                    state.max_steps
                )
            } else {
                format!("Ran AI command card '{}'", card.title)
            }
        } else if execute {
            format!("Ran AI command card '{}'", card.title)
        } else {
            format!("Inserted AI command card '{}'", card.title)
        };
        cx.notify();
    }

    fn record_ai_command_card_audit(
        &mut self,
        card: &AiCommandCard,
        execute: bool,
        inserted_to_terminal: bool,
    ) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| {
            store
                .append_ai_audit(AppendAiAuditRequest {
                    connection_id: self.active_session_id.clone(),
                    action: if execute {
                        "ai.command_card_run".to_string()
                    } else {
                        "ai.command_card_insert".to_string()
                    },
                    user_input: Some(self.ai_response_preview.clone()),
                    generated_command: Some(card.command.clone()),
                    risk_level: card.risk_level.clone(),
                    inserted_to_terminal,
                    executed: execute,
                    blocked: false,
                })
                .map(|_| ())
        }) {
            Ok(()) => {
                self.refresh_ai_usage_counts();
            }
            Err(error) => {
                self.store_status.message = format!("AI audit save failed: {error}");
                self.store_status.ready = false;
            }
        }
    }

    fn begin_ai_agent_observation(&mut self, command: &str) -> Result<Option<String>, String> {
        let Some(terminal_session_id) = self.active_session_id.clone() else {
            return Ok(None);
        };
        let task_prompt = self
            .ai_agent_task_prompt
            .clone()
            .unwrap_or_else(|| self.ai_response_preview.clone());
        let max_steps = self.ai_settings.max_agent_steps.unwrap_or(10).max(1);
        let step_index = self.ai_agent_step_index;
        if step_index.saturating_add(1) >= max_steps {
            self.ai_agent_loop = None;
            self.ai_status =
                format!("AI Agent reached max step limit ({max_steps}); review terminal output");
            return Ok(None);
        }
        self.ai_agent_step_index = self.ai_agent_step_index.saturating_add(1);
        let now = Instant::now();
        let timeout = self
            .ai_settings
            .agent_step_timeout_ms
            .map(Duration::from_millis)
            .unwrap_or(AI_AGENT_DEFAULT_STEP_TIMEOUT);
        let profile = self.active_ai_execution_profile();
        if profile == AiExecutionProfile::Disabled {
            return Err("AI Agent command execution is disabled for this session".to_string());
        }
        let marker_id = format!("agent-{}", uuid());
        let (marker_id, wrapped_command) =
            match build_agent_capture_command(profile, &marker_id, command.trim()) {
                Some(wrapped) => {
                    self.ai_agent_capture.register(marker_id.clone());
                    (Some(marker_id), Some(wrapped))
                }
                None => (None, None),
            };
        let output_start_len = self.terminal_output.len();
        self.ai_agent_loop = Some(AiAgentLoopState {
            ai_session_id: self.ai_chat_session_id.clone(),
            terminal_session_id,
            task_prompt,
            command: command.trim().to_string(),
            marker_id,
            background_job_id: None,
            step_index,
            max_steps,
            output_start_len,
            started_at: now,
            min_wait_until: now + AI_AGENT_OBSERVATION_MIN_WAIT,
            timeout_at: now + timeout,
            last_seen_len: output_start_len,
            stable_since: now,
        });
        self.ai_status = format!(
            "AI Agent observing command output for step {}/{}",
            step_index + 1,
            max_steps
        );
        self.upsert_ai_agent_step(
            step_index,
            AiAgentStepStatus::Running,
            "Running",
            truncate_preview(command.trim(), 140),
        );
        Ok(wrapped_command)
    }

    fn begin_ai_agent_background_execution(
        &mut self,
        command: &str,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let Some(terminal_session_id) = self.active_session_id.clone() else {
            return Err(
                "Start a terminal session before using AI Agent background execution".to_string(),
            );
        };
        let session = self
            .session_manager
            .list_sessions()
            .map_err(|error| error.to_string())?
            .into_iter()
            .find(|session| session.id == terminal_session_id)
            .ok_or_else(|| "Active terminal session was not found".to_string())?;
        let (target, target_label) = match session.kind {
            SessionKind::Ssh => {
                let config = self
                    .active_ssh_config
                    .clone()
                    .ok_or_else(|| "Active SSH session is missing its exec config".to_string())?;
                (AiAgentBackgroundTarget::Ssh(config), "SSH")
            }
            SessionKind::LocalPty => (
                AiAgentBackgroundTarget::Local {
                    working_dir: session.working_dir.clone(),
                },
                "local",
            ),
            SessionKind::Telnet | SessionKind::RawTcp | SessionKind::Serial => {
                return Err(format!(
                    "AI Agent background execution is not supported for {:?} sessions",
                    session.kind
                ));
            }
        };
        let task_prompt = self
            .ai_agent_task_prompt
            .clone()
            .unwrap_or_else(|| self.ai_response_preview.clone());
        let max_steps = self.ai_settings.max_agent_steps.unwrap_or(10).max(1);
        let step_index = self.ai_agent_step_index;
        if step_index.saturating_add(1) >= max_steps {
            self.ai_agent_loop = None;
            return Err(format!(
                "AI Agent reached max step limit ({max_steps}); review terminal output"
            ));
        }
        self.ai_agent_step_index = self.ai_agent_step_index.saturating_add(1);
        let now = Instant::now();
        let timeout = self
            .ai_settings
            .agent_step_timeout_ms
            .map(Duration::from_millis)
            .unwrap_or(AI_AGENT_DEFAULT_STEP_TIMEOUT);
        let (job_id, cancel) = self.begin_ai_chat_job();
        let state = AiAgentLoopState {
            ai_session_id: self.ai_chat_session_id.clone(),
            terminal_session_id,
            task_prompt,
            command: command.trim().to_string(),
            marker_id: None,
            background_job_id: Some(job_id),
            step_index,
            max_steps,
            output_start_len: self.terminal_output.len(),
            started_at: now,
            min_wait_until: now,
            timeout_at: now + timeout,
            last_seen_len: self.terminal_output.len(),
            stable_since: now,
        };
        self.ai_agent_loop = Some(state.clone());
        self.ai_status = format!(
            "AI Agent running {target_label} background command for step {}/{}",
            step_index + 1,
            max_steps
        );
        self.upsert_ai_agent_step(
            step_index,
            AiAgentStepStatus::Running,
            format!("{target_label} background"),
            truncate_preview(command.trim(), 140),
        );
        let tx = self.ai_chat_tx.clone();
        let command = state.command.clone();
        std::thread::spawn(move || {
            let started = Instant::now();
            let result = if ai_job_cancelled(&cancel) {
                Err("AI Agent background command cancelled".to_string())
            } else {
                match target {
                    AiAgentBackgroundTarget::Ssh(config) => SshProcessService::new(config)
                        .run_command(&command, timeout)
                        .map(|output| remote_command_observation(output, started))
                        .map_err(|error| error.to_string()),
                    AiAgentBackgroundTarget::Local { working_dir } => {
                        run_local_command(&command, working_dir, timeout)
                            .map(|output| remote_command_observation(output, started))
                            .map_err(|error| error.to_string())
                    }
                }
            };
            if !ai_job_cancelled(&cancel) {
                let _ = tx.send(AiChatWorkerEvent::AgentBackgroundFinished {
                    job_id,
                    state,
                    result,
                });
            }
        });
        cx.notify();
        Ok(())
    }

    fn drive_ai_agent_loop(&mut self, cx: &mut Context<Self>) {
        if self.ai_chat_pending {
            return;
        }
        let Some(state) = self.ai_agent_loop.as_mut() else {
            return;
        };
        if state.background_job_id.is_some() {
            return;
        }
        if self.active_session_id.as_deref() != Some(state.terminal_session_id.as_str()) {
            let step_index = state.step_index;
            self.ai_agent_loop = None;
            self.ai_status =
                "AI Agent loop stopped because the terminal session changed".to_string();
            self.upsert_ai_agent_step(
                step_index,
                AiAgentStepStatus::Failed,
                "Stopped",
                "Terminal session changed",
            );
            cx.notify();
            return;
        }

        let now = Instant::now();
        let current_len = self.terminal_output.len();
        if current_len != state.last_seen_len {
            state.last_seen_len = current_len;
            state.stable_since = now;
            return;
        }
        if now < state.min_wait_until {
            return;
        }
        let has_observed_output = current_len > state.output_start_len;
        let output_is_quiet = now.duration_since(state.stable_since) >= AI_AGENT_OBSERVATION_QUIET;
        let timed_out = now >= state.timeout_at;
        if timed_out && let Some(marker_id) = state.marker_id.clone() {
            let timeout_ms = now
                .duration_since(state.started_at)
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX);
            let command = state.command.clone();
            self.ai_agent_capture.cancel(&marker_id);
            let Some(state) = self.ai_agent_loop.take() else {
                return;
            };
            let observation = CommandObservation {
                output: "(command timed out; capture markers were not detected in terminal output)"
                    .to_string(),
                exit_code: None,
                duration_ms: timeout_ms,
            };
            self.ai_status = format!("AI Agent command capture timed out: {command}");
            self.upsert_ai_agent_step(
                state.step_index,
                AiAgentStepStatus::Failed,
                "Timed out",
                observation_summary(&observation),
            );
            self.start_ai_agent_continuation(state, observation, cx);
            return;
        }
        if !timed_out && (!has_observed_output || !output_is_quiet) {
            return;
        }
        if state.marker_id.is_some() {
            return;
        }

        let Some(state) = self.ai_agent_loop.take() else {
            return;
        };
        let output = if self.terminal_output.len() > state.output_start_len {
            self.terminal_output[state.output_start_len..].to_string()
        } else {
            String::new()
        };
        let duration_ms = now
            .duration_since(state.started_at)
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        let observation = CommandObservation {
            output,
            exit_code: None,
            duration_ms,
        };
        self.upsert_ai_agent_step(
            state.step_index,
            AiAgentStepStatus::Completed,
            "Observed",
            observation_summary(&observation),
        );
        self.start_ai_agent_continuation(state, observation, cx);
    }

    fn handle_ai_agent_captured_output(
        &mut self,
        captured: AgentCapturedOutput,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.ai_agent_loop.take() else {
            return;
        };
        if state.marker_id.as_deref() != Some(captured.marker_id.as_str()) {
            self.ai_agent_loop = Some(state);
            return;
        }
        let observation = CommandObservation {
            output: captured.output,
            exit_code: captured.exit_code,
            duration_ms: captured.duration_ms,
        };
        self.ai_status = match observation.exit_code {
            Some(code) => format!("AI Agent captured command output with exit code {code}"),
            None => "AI Agent captured command output".to_string(),
        };
        self.upsert_ai_agent_step(
            state.step_index,
            AiAgentStepStatus::Completed,
            "Observed",
            observation_summary(&observation),
        );
        self.start_ai_agent_continuation(state, observation, cx);
    }

    fn active_ai_execution_profile(&self) -> AiExecutionProfile {
        if self.active_ai_execution_profile != AiExecutionProfile::Auto {
            return self.active_ai_execution_profile;
        }
        let Some(session_id) = self.active_session_id.as_deref() else {
            return AiExecutionProfile::SendOnly;
        };
        self.session_manager
            .list_sessions()
            .ok()
            .and_then(|sessions| {
                sessions
                    .into_iter()
                    .find(|session| session.id == session_id)
            })
            .map(|session| match session.kind {
                SessionKind::LocalPty
                | SessionKind::Ssh
                | SessionKind::Telnet
                | SessionKind::RawTcp => AiExecutionProfile::Posix,
                SessionKind::Serial => AiExecutionProfile::SendOnly,
            })
            .unwrap_or(AiExecutionProfile::SendOnly)
    }

    fn start_ai_agent_continuation(
        &mut self,
        state: AiAgentLoopState,
        observation: CommandObservation,
        cx: &mut Context<Self>,
    ) {
        if self.ai_chat_pending {
            self.ai_agent_loop = Some(state);
            return;
        }
        let observation_message =
            build_observation_message(&observation, &state.command, &self.settings.language);
        let settings = self.ai_settings.clone();
        let request = AiChatRequest {
            stream_id: None,
            session_id: Some(state.ai_session_id.clone()),
            connection_id: self.active_session_id.clone(),
            terminal_session_id: self.active_session_id.clone(),
            mode: AiMode::Agent,
            model_id: settings.default_model_id.clone(),
            model_name: None,
            action: AiAction::GenerateCommand,
            user_input: format!(
                "Continue the same Agent task.\n\nOriginal task:\n{}\n\n{}",
                state.task_prompt, observation_message
            ),
            context: self.ai_terminal_context(),
            options: Default::default(),
        };
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        let tx = self.ai_chat_tx.clone();
        let session_id = state.ai_session_id;
        let (job_id, cancel) = self.begin_ai_chat_job();

        self.ai_chat_pending = true;
        self.ai_response_preview = format!(
            "Running AI Agent continuation step {}/{}...",
            state.step_index + 2,
            state.max_steps
        );
        self.ai_command_cards.clear();
        self.ai_status = self.ai_response_preview.clone();
        self.upsert_ai_agent_step(
            state.step_index.saturating_add(1),
            AiAgentStepStatus::Planning,
            "Planning",
            "Continuing from the latest command observation",
        );
        std::thread::spawn(move || {
            let result = run_ai_ask_job(
                config_dir,
                portable_key_path,
                settings,
                request,
                Some(tx.clone()),
                cancel,
                job_id,
            );
            let _ = tx.send(AiChatWorkerEvent::Finished(AiChatJobResult {
                job_id,
                session_id,
                result,
            }));
        });
        cx.notify();
    }

    fn sync_ai_drafts_from_active_profile(&mut self) {
        let (model, base_url) = ai_active_profile_drafts(&self.ai_settings);
        self.ai_model_draft = model;
        self.ai_base_url_draft = base_url;
        self.ai_secret_draft.clear();
    }

    fn refresh_ai_usage_counts(&mut self) {
        if let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            let (sessions, messages, audits) = ai_usage_counts(&store);
            self.ai_session_count = sessions;
            self.ai_message_count = messages;
            self.ai_audit_count = audits;
        }
    }

    fn refresh_quick_commands(&mut self) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.load_quick_commands())
        {
            Ok(config) => {
                self.quick_commands = config.commands;
                self.quick_command_categories = config.categories;
            }
            Err(error) => {
                self.store_status.message = format!("quick command refresh failed: {error}");
                self.store_status.ready = false;
            }
        }
    }

    fn refresh_command_history(&mut self) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.list_command_history(64))
        {
            Ok(history) => {
                self.command_history = history;
            }
            Err(error) => {
                self.store_status.message = format!("command history refresh failed: {error}");
                self.store_status.ready = false;
            }
        }
    }

    fn record_command_history_from_bytes(&mut self, bytes: &[u8]) {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return;
        };
        if !text.contains('\n') && !text.contains('\r') {
            return;
        }
        let submitted: Vec<String> = text
            .split(['\r', '\n'])
            .map(str::trim)
            .filter(|command| !command.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        if submitted.is_empty() {
            return;
        }
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            Ok(store) => {
                for command in submitted {
                    if let Err(error) = store.append_command_history(&command) {
                        self.store_status.message = format!("command history save failed: {error}");
                        self.store_status.ready = false;
                        return;
                    }
                }
                self.command_history = store.list_command_history(64).unwrap_or_default();
            }
            Err(error) => {
                self.store_status.message = format!("command history store failed: {error}");
                self.store_status.ready = false;
            }
        }
    }

    fn prompt_config_export(&mut self, cx: &mut Context<Self>) {
        if self.config_path_prompt.is_some() {
            self.terminal_status = "config path picker is already open".to_string();
            cx.notify();
            return;
        }

        let directory = self.runtime.config_dir().to_path_buf();
        let receiver = cx.prompt_for_new_path(&directory, Some("nyaterm-backup.redb"));
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        self.config_path_prompt = Some(ConfigPathPromptKind::Export);
        self.terminal_status = "selecting config backup destination".to_string();
        self.store_status.message = "selecting backup destination".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(path))) => match ConnectionStore::export_config_database(
                    &config_dir,
                    portable_key_path,
                    &path,
                ) {
                    Ok(info) => ConfigPathPromptResult::Exported(info),
                    Err(error) => ConfigPathPromptResult::Failed(error.to_string()),
                },
                Ok(Ok(None)) => ConfigPathPromptResult::Cancelled,
                Ok(Err(error)) => ConfigPathPromptResult::Failed(error.to_string()),
                Err(_) => ConfigPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_config_path_prompt_result(ConfigPathPromptKind::Export, result);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn prompt_portable_snapshot_export(&mut self, cx: &mut Context<Self>) {
        if self.config_path_prompt.is_some() {
            self.terminal_status = "config path picker is already open".to_string();
            cx.notify();
            return;
        }

        let directory = self.runtime.config_dir().to_path_buf();
        let receiver = cx.prompt_for_new_path(&directory, Some("nyaterm-backup.nya"));
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        self.config_path_prompt = Some(ConfigPathPromptKind::PortableExport);
        self.terminal_status = "selecting portable snapshot destination".to_string();
        self.store_status.message = "selecting .nya export destination".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(path))) => match ConnectionStore::export_portable_snapshot(
                    &config_dir,
                    portable_key_path,
                    &path,
                    "native-local",
                    env!("CARGO_PKG_VERSION"),
                ) {
                    Ok(info) => ConfigPathPromptResult::Exported(info),
                    Err(error) => ConfigPathPromptResult::Failed(error.to_string()),
                },
                Ok(Ok(None)) => ConfigPathPromptResult::Cancelled,
                Ok(Err(error)) => ConfigPathPromptResult::Failed(error.to_string()),
                Err(_) => ConfigPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_config_path_prompt_result(ConfigPathPromptKind::PortableExport, result);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn prompt_encrypted_portable_snapshot_export(&mut self, cx: &mut Context<Self>) {
        self.start_snapshot_password_prompt(SnapshotPasswordPromptKind::Export, cx);
    }

    fn prompt_config_import(&mut self, cx: &mut Context<Self>) {
        if self.config_path_prompt.is_some() {
            self.terminal_status = "config path picker is already open".to_string();
            cx.notify();
            return;
        }
        if self.active_session_id.is_some() || self.pending_session_name.is_some() {
            self.terminal_status = "close active session before importing config".to_string();
            cx.notify();
            return;
        }

        let options = PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from("Select config backup")),
        };
        let receiver = cx.prompt_for_paths(options);
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        self.config_path_prompt = Some(ConfigPathPromptKind::Import);
        self.terminal_status = "selecting config backup to import".to_string();
        self.store_status.message = "selecting backup source".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(paths))) => match paths.into_iter().next() {
                    Some(path) => match ConnectionStore::import_config_database(
                        &config_dir,
                        portable_key_path,
                        &path,
                    ) {
                        Ok(info) => ConfigPathPromptResult::Imported(info),
                        Err(error) => ConfigPathPromptResult::Failed(error.to_string()),
                    },
                    None => ConfigPathPromptResult::Cancelled,
                },
                Ok(Ok(None)) => ConfigPathPromptResult::Cancelled,
                Ok(Err(error)) => ConfigPathPromptResult::Failed(error.to_string()),
                Err(_) => ConfigPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_config_path_prompt_result(ConfigPathPromptKind::Import, result);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn prompt_portable_snapshot_import(&mut self, cx: &mut Context<Self>) {
        if self.config_path_prompt.is_some() {
            self.terminal_status = "config path picker is already open".to_string();
            cx.notify();
            return;
        }
        if self.active_session_id.is_some() || self.pending_session_name.is_some() {
            self.terminal_status = "close active session before importing config".to_string();
            cx.notify();
            return;
        }

        let options = PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from("Select .nya snapshot")),
        };
        let receiver = cx.prompt_for_paths(options);
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        self.config_path_prompt = Some(ConfigPathPromptKind::PortableImport);
        self.terminal_status = "selecting portable snapshot to import".to_string();
        self.store_status.message = "selecting .nya snapshot".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(paths))) => match paths.into_iter().next() {
                    Some(path) => match ConnectionStore::import_portable_snapshot(
                        &config_dir,
                        portable_key_path,
                        &path,
                    ) {
                        Ok(info) => ConfigPathPromptResult::Imported(info),
                        Err(error) => ConfigPathPromptResult::Failed(error.to_string()),
                    },
                    None => ConfigPathPromptResult::Cancelled,
                },
                Ok(Ok(None)) => ConfigPathPromptResult::Cancelled,
                Ok(Err(error)) => ConfigPathPromptResult::Failed(error.to_string()),
                Err(_) => ConfigPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_config_path_prompt_result(ConfigPathPromptKind::PortableImport, result);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn prompt_encrypted_portable_snapshot_import(&mut self, cx: &mut Context<Self>) {
        if self.active_session_id.is_some() || self.pending_session_name.is_some() {
            self.terminal_status = "close active session before importing config".to_string();
            cx.notify();
            return;
        }
        self.start_snapshot_password_prompt(SnapshotPasswordPromptKind::Import, cx);
    }

    fn start_snapshot_password_prompt(
        &mut self,
        kind: SnapshotPasswordPromptKind,
        cx: &mut Context<Self>,
    ) {
        if self.config_path_prompt.is_some() {
            self.terminal_status = "config path picker is already open".to_string();
            cx.notify();
            return;
        }
        self.active_snapshot_password_prompt = Some(SnapshotPasswordPromptState {
            kind,
            value: String::new(),
        });
        self.terminal_status = match kind {
            SnapshotPasswordPromptKind::Export => "enter password for encrypted .nya export",
            SnapshotPasswordPromptKind::Import => "enter password for encrypted .nya import",
            SnapshotPasswordPromptKind::CloudPush => "enter password for cloud sync push",
            SnapshotPasswordPromptKind::CloudPull => "enter password for cloud sync pull",
            SnapshotPasswordPromptKind::CloudForcePush => {
                "enter password for forced cloud sync push"
            }
            SnapshotPasswordPromptKind::CloudForcePull => {
                "enter password for forced cloud sync pull"
            }
            SnapshotPasswordPromptKind::CloudProviderPush => {
                "enter password for provider cloud sync push"
            }
            SnapshotPasswordPromptKind::CloudProviderPull => {
                "enter password for provider cloud sync pull"
            }
            SnapshotPasswordPromptKind::CloudProviderForcePush => {
                "enter password for forced provider cloud sync push"
            }
            SnapshotPasswordPromptKind::CloudProviderForcePull => {
                "enter password for forced provider cloud sync pull"
            }
        }
        .to_string();
        self.store_status.message = match kind {
            SnapshotPasswordPromptKind::CloudPush
            | SnapshotPasswordPromptKind::CloudPull
            | SnapshotPasswordPromptKind::CloudForcePush
            | SnapshotPasswordPromptKind::CloudForcePull
            | SnapshotPasswordPromptKind::CloudProviderPush
            | SnapshotPasswordPromptKind::CloudProviderPull
            | SnapshotPasswordPromptKind::CloudProviderForcePush
            | SnapshotPasswordPromptKind::CloudProviderForcePull => {
                "awaiting cloud sync password".to_string()
            }
            _ => "awaiting .nya master password".to_string(),
        };
        cx.notify();
    }

    fn submit_snapshot_password_prompt(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.active_snapshot_password_prompt.take() else {
            return;
        };
        let password = state.value.trim().to_string();
        if password.is_empty() {
            self.active_snapshot_password_prompt = Some(SnapshotPasswordPromptState {
                kind: state.kind,
                value: String::new(),
            });
            self.terminal_status = "master password is required for encrypted .nya".to_string();
            cx.notify();
            return;
        }

        match state.kind {
            SnapshotPasswordPromptKind::Export => {
                self.prompt_encrypted_portable_snapshot_export_path(password, cx);
            }
            SnapshotPasswordPromptKind::Import => {
                self.prompt_encrypted_portable_snapshot_import_path(password, cx);
            }
            SnapshotPasswordPromptKind::CloudPush => {
                self.run_local_cloud_sync_push(password, false, cx);
            }
            SnapshotPasswordPromptKind::CloudPull => {
                self.run_local_cloud_sync_pull(password, false, cx);
            }
            SnapshotPasswordPromptKind::CloudForcePush => {
                self.run_local_cloud_sync_push(password, true, cx);
            }
            SnapshotPasswordPromptKind::CloudForcePull => {
                self.run_local_cloud_sync_pull(password, true, cx);
            }
            SnapshotPasswordPromptKind::CloudProviderPush => {
                self.run_provider_cloud_sync_push(password, false, cx);
            }
            SnapshotPasswordPromptKind::CloudProviderPull => {
                self.run_provider_cloud_sync_pull(password, false, cx);
            }
            SnapshotPasswordPromptKind::CloudProviderForcePush => {
                self.run_provider_cloud_sync_push(password, true, cx);
            }
            SnapshotPasswordPromptKind::CloudProviderForcePull => {
                self.run_provider_cloud_sync_pull(password, true, cx);
            }
        }
    }

    fn cancel_snapshot_password_prompt(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.active_snapshot_password_prompt.take() else {
            return;
        };
        self.terminal_status = match state.kind {
            SnapshotPasswordPromptKind::Export => "encrypted .nya export cancelled".to_string(),
            SnapshotPasswordPromptKind::Import => "encrypted .nya import cancelled".to_string(),
            SnapshotPasswordPromptKind::CloudPush => "cloud sync push cancelled".to_string(),
            SnapshotPasswordPromptKind::CloudPull => "cloud sync pull cancelled".to_string(),
            SnapshotPasswordPromptKind::CloudForcePush => {
                "forced cloud sync push cancelled".to_string()
            }
            SnapshotPasswordPromptKind::CloudForcePull => {
                "forced cloud sync pull cancelled".to_string()
            }
            SnapshotPasswordPromptKind::CloudProviderPush => {
                "provider cloud sync push cancelled".to_string()
            }
            SnapshotPasswordPromptKind::CloudProviderPull => {
                "provider cloud sync pull cancelled".to_string()
            }
            SnapshotPasswordPromptKind::CloudProviderForcePush => {
                "forced provider cloud sync push cancelled".to_string()
            }
            SnapshotPasswordPromptKind::CloudProviderForcePull => {
                "forced provider cloud sync pull cancelled".to_string()
            }
        };
        self.store_status.message = "config picker cancelled".to_string();
        cx.notify();
    }

    fn handle_snapshot_password_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(state) = self.active_snapshot_password_prompt.as_mut() else {
            return;
        };
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "enter" => self.submit_snapshot_password_prompt(cx),
            "escape" => self.cancel_snapshot_password_prompt(cx),
            "backspace" => {
                state.value.pop();
                cx.notify();
            }
            _ => {
                if let Some(value) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|value| !value.is_empty())
                {
                    state.value.push_str(value);
                    cx.notify();
                }
            }
        }
    }

    fn prompt_encrypted_portable_snapshot_export_path(
        &mut self,
        master_password: String,
        cx: &mut Context<Self>,
    ) {
        let directory = self.runtime.config_dir().to_path_buf();
        let receiver = cx.prompt_for_new_path(&directory, Some("nyaterm-encrypted.nya"));
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        self.config_path_prompt = Some(ConfigPathPromptKind::EncryptedPortableExport);
        self.terminal_status = "selecting encrypted portable snapshot destination".to_string();
        self.store_status.message = "selecting encrypted .nya export destination".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(path))) => match ConnectionStore::export_encrypted_portable_snapshot(
                    &config_dir,
                    portable_key_path,
                    &path,
                    "native-local",
                    env!("CARGO_PKG_VERSION"),
                    &master_password,
                ) {
                    Ok(info) => ConfigPathPromptResult::Exported(info),
                    Err(error) => ConfigPathPromptResult::Failed(error.to_string()),
                },
                Ok(Ok(None)) => ConfigPathPromptResult::Cancelled,
                Ok(Err(error)) => ConfigPathPromptResult::Failed(error.to_string()),
                Err(_) => ConfigPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_config_path_prompt_result(
                    ConfigPathPromptKind::EncryptedPortableExport,
                    result,
                );
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn prompt_encrypted_portable_snapshot_import_path(
        &mut self,
        master_password: String,
        cx: &mut Context<Self>,
    ) {
        let options = PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(SharedString::from("Select encrypted .nya snapshot")),
        };
        let receiver = cx.prompt_for_paths(options);
        let config_dir = self.runtime.config_dir().to_path_buf();
        let portable_key_path = self.runtime.portable_key_path().map(ToOwned::to_owned);
        self.config_path_prompt = Some(ConfigPathPromptKind::EncryptedPortableImport);
        self.terminal_status = "selecting encrypted portable snapshot to import".to_string();
        self.store_status.message = "selecting encrypted .nya snapshot".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(paths))) => match paths.into_iter().next() {
                    Some(path) => match ConnectionStore::import_encrypted_portable_snapshot(
                        &config_dir,
                        portable_key_path,
                        &path,
                        &master_password,
                    ) {
                        Ok(info) => ConfigPathPromptResult::Imported(info),
                        Err(error) => ConfigPathPromptResult::Failed(error.to_string()),
                    },
                    None => ConfigPathPromptResult::Cancelled,
                },
                Ok(Ok(None)) => ConfigPathPromptResult::Cancelled,
                Ok(Err(error)) => ConfigPathPromptResult::Failed(error.to_string()),
                Err(_) => ConfigPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_config_path_prompt_result(
                    ConfigPathPromptKind::EncryptedPortableImport,
                    result,
                );
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn prompt_local_cloud_sync_push(&mut self, cx: &mut Context<Self>) {
        self.start_snapshot_password_prompt(SnapshotPasswordPromptKind::CloudPush, cx);
    }

    fn prompt_local_cloud_sync_pull(&mut self, cx: &mut Context<Self>) {
        if self.active_session_id.is_some() || self.pending_session_name.is_some() {
            self.terminal_status = "close active session before pulling cloud sync".to_string();
            cx.notify();
            return;
        }
        self.start_snapshot_password_prompt(SnapshotPasswordPromptKind::CloudPull, cx);
    }

    fn prompt_provider_cloud_sync_push(&mut self, cx: &mut Context<Self>) {
        self.start_snapshot_password_prompt(SnapshotPasswordPromptKind::CloudProviderPush, cx);
    }

    fn prompt_provider_cloud_sync_pull(&mut self, cx: &mut Context<Self>) {
        if self.active_session_id.is_some() || self.pending_session_name.is_some() {
            self.terminal_status =
                "close active session before pulling provider cloud sync".to_string();
            cx.notify();
            return;
        }
        self.start_snapshot_password_prompt(SnapshotPasswordPromptKind::CloudProviderPull, cx);
    }

    fn prompt_cloud_sync_force_push(&mut self, provider_action: bool, cx: &mut Context<Self>) {
        let kind = if provider_action {
            SnapshotPasswordPromptKind::CloudProviderForcePush
        } else {
            SnapshotPasswordPromptKind::CloudForcePush
        };
        self.start_snapshot_password_prompt(kind, cx);
    }

    fn prompt_cloud_sync_force_pull(&mut self, provider_action: bool, cx: &mut Context<Self>) {
        if self.active_session_id.is_some() || self.pending_session_name.is_some() {
            self.terminal_status = if provider_action {
                "close active session before force pulling provider cloud sync"
            } else {
                "close active session before force pulling cloud sync"
            }
            .to_string();
            cx.notify();
            return;
        }
        let kind = if provider_action {
            SnapshotPasswordPromptKind::CloudProviderForcePull
        } else {
            SnapshotPasswordPromptKind::CloudForcePull
        };
        self.start_snapshot_password_prompt(kind, cx);
    }

    fn dismiss_cloud_sync_conflict(&mut self, cx: &mut Context<Self>) {
        self.cloud_sync_conflict = None;
        self.cloud_sync_status = "cloud sync conflict dismissed".to_string();
        cx.notify();
    }

    fn capture_cloud_sync_conflict(
        &mut self,
        error: &CloudSyncError,
        provider: String,
        provider_action: bool,
    ) {
        if let CloudSyncError::Conflict(message) = error {
            self.cloud_sync_conflict = Some(CloudSyncConflictState {
                provider,
                message: message.clone(),
                provider_action,
            });
        }
    }

    fn run_local_cloud_sync_push(
        &mut self,
        master_password: String,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        let options = self.local_cloud_sync_options(master_password);
        let state = self.cloud_sync_state.clone();
        let started_at = Instant::now();
        self.cloud_sync_status = if force {
            "force pushing local cloud sync snapshot".to_string()
        } else {
            "pushing local cloud sync snapshot".to_string()
        };
        self.terminal_status = "cloud sync push started".to_string();
        cx.spawn(async move |this, cx| {
            let result = push_local_snapshot(&options, &state, force);
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(result) => {
                        this.cloud_sync_conflict = None;
                        let mut history = CloudSyncHistoryEntry::sync(
                            "success",
                            if force {
                                "manual_force_push"
                            } else {
                                "manual_push"
                            },
                            Some(result.status.provider.clone()),
                            result
                                .pointer
                                .as_ref()
                                .map(|pointer| pointer.revision_id.clone()),
                            result.status.message.clone(),
                        );
                        history.duration_ms = Some(started_at.elapsed().as_millis() as u64);
                        this.record_cloud_sync_history(&history);
                        this.refresh_cloud_sync_history();
                        this.cloud_sync_state = result.state;
                        this.cloud_sync_status = result.status.message.clone();
                        this.terminal_status = result.status.message;
                    }
                    Err(error) => {
                        let status = cloud_sync_history_status(&error);
                        this.cloud_sync_status = format!("push failed: {error}");
                        this.capture_cloud_sync_conflict(
                            &error,
                            "local_directory".to_string(),
                            false,
                        );
                        this.terminal_status = this.cloud_sync_status.clone();
                        let mut history = CloudSyncHistoryEntry::sync(
                            status,
                            if force {
                                "manual_force_push"
                            } else {
                                "manual_push"
                            },
                            Some("local_directory".to_string()),
                            None,
                            this.cloud_sync_status.clone(),
                        );
                        history.duration_ms = Some(started_at.elapsed().as_millis() as u64);
                        this.record_cloud_sync_history(&history);
                        this.refresh_cloud_sync_history();
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn run_local_cloud_sync_pull(
        &mut self,
        master_password: String,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        let options = self.local_cloud_sync_options(master_password);
        let state = self.cloud_sync_state.clone();
        let started_at = Instant::now();
        self.cloud_sync_status = if force {
            "force pulling local cloud sync snapshot".to_string()
        } else {
            "pulling local cloud sync snapshot".to_string()
        };
        self.terminal_status = "cloud sync pull started".to_string();
        cx.spawn(async move |this, cx| {
            let result = pull_local_snapshot(&options, &state, force);
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(result) => {
                        this.cloud_sync_conflict = None;
                        let mut history = CloudSyncHistoryEntry::sync(
                            "success",
                            if force {
                                "manual_force_pull"
                            } else {
                                "manual_pull"
                            },
                            Some(result.status.provider.clone()),
                            result
                                .pointer
                                .as_ref()
                                .map(|pointer| pointer.revision_id.clone()),
                            result.status.message.clone(),
                        );
                        history.duration_ms = Some(started_at.elapsed().as_millis() as u64);
                        this.record_cloud_sync_history(&history);
                        this.refresh_cloud_sync_history();
                        this.cloud_sync_state = result.state;
                        this.cloud_sync_status = result.status.message.clone();
                        this.terminal_status = result.status.message;
                        this.refresh_store_from_runtime();
                    }
                    Err(error) => {
                        let status = cloud_sync_history_status(&error);
                        this.cloud_sync_status = format!("pull failed: {error}");
                        this.capture_cloud_sync_conflict(
                            &error,
                            "local_directory".to_string(),
                            false,
                        );
                        this.terminal_status = this.cloud_sync_status.clone();
                        let mut history = CloudSyncHistoryEntry::sync(
                            status,
                            if force {
                                "manual_force_pull"
                            } else {
                                "manual_pull"
                            },
                            Some("local_directory".to_string()),
                            None,
                            this.cloud_sync_status.clone(),
                        );
                        history.duration_ms = Some(started_at.elapsed().as_millis() as u64);
                        this.record_cloud_sync_history(&history);
                        this.refresh_cloud_sync_history();
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn run_provider_cloud_sync_push(
        &mut self,
        master_password: String,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        let options = self.local_cloud_sync_options(master_password);
        let state = self.cloud_sync_state.clone();
        let settings = self.cloud_sync_settings.clone();
        let provider = configured_cloud_sync_provider(&settings);
        let started_at = Instant::now();
        self.cloud_sync_status = if force {
            format!("force pushing provider cloud sync snapshot via {provider}")
        } else {
            format!("pushing provider cloud sync snapshot via {provider}")
        };
        self.terminal_status = "provider cloud sync push started".to_string();
        cx.spawn(async move |this, cx| {
            let result = push_provider_snapshot(&settings, &options, &state, force);
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(result) => {
                        this.cloud_sync_conflict = None;
                        let mut history = CloudSyncHistoryEntry::sync(
                            "success",
                            if force {
                                "manual_provider_force_push"
                            } else {
                                "manual_provider_push"
                            },
                            Some(result.status.provider.clone()),
                            result
                                .pointer
                                .as_ref()
                                .map(|pointer| pointer.revision_id.clone()),
                            result.status.message.clone(),
                        );
                        history.duration_ms = Some(started_at.elapsed().as_millis() as u64);
                        this.record_cloud_sync_history(&history);
                        this.refresh_cloud_sync_history();
                        this.cloud_sync_state = result.state;
                        this.cloud_sync_status = result.status.message.clone();
                        this.terminal_status = result.status.message;
                    }
                    Err(error) => {
                        let status = cloud_sync_history_status(&error);
                        this.cloud_sync_status = format!("provider push failed: {error}");
                        this.capture_cloud_sync_conflict(
                            &error,
                            configured_cloud_sync_provider(&settings),
                            true,
                        );
                        this.terminal_status = this.cloud_sync_status.clone();
                        let mut history = CloudSyncHistoryEntry::sync(
                            status,
                            if force {
                                "manual_provider_force_push"
                            } else {
                                "manual_provider_push"
                            },
                            Some(configured_cloud_sync_provider(&settings)),
                            None,
                            this.cloud_sync_status.clone(),
                        );
                        history.duration_ms = Some(started_at.elapsed().as_millis() as u64);
                        this.record_cloud_sync_history(&history);
                        this.refresh_cloud_sync_history();
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn run_provider_cloud_sync_pull(
        &mut self,
        master_password: String,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        let options = self.local_cloud_sync_options(master_password);
        let state = self.cloud_sync_state.clone();
        let settings = self.cloud_sync_settings.clone();
        let provider = configured_cloud_sync_provider(&settings);
        let started_at = Instant::now();
        self.cloud_sync_status = if force {
            format!("force pulling provider cloud sync snapshot via {provider}")
        } else {
            format!("pulling provider cloud sync snapshot via {provider}")
        };
        self.terminal_status = "provider cloud sync pull started".to_string();
        cx.spawn(async move |this, cx| {
            let result = pull_provider_snapshot(&settings, &options, &state, force);
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(result) => {
                        this.cloud_sync_conflict = None;
                        let mut history = CloudSyncHistoryEntry::sync(
                            "success",
                            if force {
                                "manual_provider_force_pull"
                            } else {
                                "manual_provider_pull"
                            },
                            Some(result.status.provider.clone()),
                            result
                                .pointer
                                .as_ref()
                                .map(|pointer| pointer.revision_id.clone()),
                            result.status.message.clone(),
                        );
                        history.duration_ms = Some(started_at.elapsed().as_millis() as u64);
                        this.record_cloud_sync_history(&history);
                        this.refresh_cloud_sync_history();
                        this.cloud_sync_state = result.state;
                        this.cloud_sync_status = result.status.message.clone();
                        this.terminal_status = result.status.message;
                        this.refresh_store_from_runtime();
                    }
                    Err(error) => {
                        let status = cloud_sync_history_status(&error);
                        this.cloud_sync_status = format!("provider pull failed: {error}");
                        this.capture_cloud_sync_conflict(
                            &error,
                            configured_cloud_sync_provider(&settings),
                            true,
                        );
                        this.terminal_status = this.cloud_sync_status.clone();
                        let mut history = CloudSyncHistoryEntry::sync(
                            status,
                            if force {
                                "manual_provider_force_pull"
                            } else {
                                "manual_provider_pull"
                            },
                            Some(configured_cloud_sync_provider(&settings)),
                            None,
                            this.cloud_sync_status.clone(),
                        );
                        history.duration_ms = Some(started_at.elapsed().as_millis() as u64);
                        this.record_cloud_sync_history(&history);
                        this.refresh_cloud_sync_history();
                    }
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn local_cloud_sync_options(&self, master_password: String) -> LocalCloudSyncOptions {
        LocalCloudSyncOptions {
            config_dir: self.runtime.config_dir().to_path_buf(),
            portable_key_path: self.runtime.portable_key_path().map(ToOwned::to_owned),
            remote_dir: self.runtime.config_dir().join("cloud-sync-local"),
            remote_root: self.cloud_sync_settings.remote_root.clone(),
            device_id: self.cloud_sync_state.device_id.clone(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            master_password,
            enabled: true,
        }
    }

    fn record_cloud_sync_history(&mut self, entry: &CloudSyncHistoryEntry) {
        if let Err(error) = append_cloud_sync_history(self.runtime.log_dir(), entry) {
            self.cloud_sync_status = format!("{}; history log failed: {error}", entry.message);
        }
    }

    fn refresh_cloud_sync_history(&mut self) {
        self.cloud_sync_history = read_cloud_sync_history(
            self.runtime.log_dir(),
            self.settings.diagnostics_retention_days,
            CLOUD_SYNC_HISTORY_LIMIT,
        )
        .unwrap_or_default();
    }

    fn apply_config_path_prompt_result(
        &mut self,
        kind: ConfigPathPromptKind,
        result: ConfigPathPromptResult,
    ) {
        self.config_path_prompt = None;
        match result {
            ConfigPathPromptResult::Exported(info) => {
                self.store_status.path = info.database_path.display().to_string();
                self.store_status.message = match kind {
                    ConfigPathPromptKind::PortableExport => {
                        format!("exported {} byte .nya snapshot", info.bytes)
                    }
                    ConfigPathPromptKind::EncryptedPortableExport => {
                        format!("exported {} byte encrypted .nya snapshot", info.bytes)
                    }
                    _ => format!("exported {} byte config backup", info.bytes),
                };
                self.store_status.ready = true;
                self.terminal_status = match kind {
                    ConfigPathPromptKind::PortableExport => {
                        format!(
                            "portable snapshot exported to {}",
                            info.backup_path.display()
                        )
                    }
                    ConfigPathPromptKind::EncryptedPortableExport => {
                        format!(
                            "encrypted portable snapshot exported to {}",
                            info.backup_path.display()
                        )
                    }
                    _ => format!("config exported to {}", info.backup_path.display()),
                };
            }
            ConfigPathPromptResult::Imported(info) => {
                self.refresh_store_from_runtime();
                let safety = info
                    .safety_backup_path
                    .as_ref()
                    .map(|path| format!("; previous db saved to {}", path.display()))
                    .unwrap_or_default();
                self.store_status.message = match kind {
                    ConfigPathPromptKind::PortableImport => {
                        format!("imported {} byte .nya snapshot{safety}", info.bytes)
                    }
                    ConfigPathPromptKind::EncryptedPortableImport => {
                        format!(
                            "imported {} byte encrypted .nya snapshot{safety}",
                            info.bytes
                        )
                    }
                    _ => format!("imported {} byte config backup{safety}", info.bytes),
                };
                self.store_status.ready = true;
                self.terminal_status = match kind {
                    ConfigPathPromptKind::PortableImport => {
                        format!(
                            "portable snapshot imported from {}",
                            info.backup_path.display()
                        )
                    }
                    ConfigPathPromptKind::EncryptedPortableImport => {
                        format!(
                            "encrypted portable snapshot imported from {}",
                            info.backup_path.display()
                        )
                    }
                    _ => format!("config imported from {}", info.backup_path.display()),
                };
            }
            ConfigPathPromptResult::Cancelled => {
                self.terminal_status = match kind {
                    ConfigPathPromptKind::Export => "config export cancelled".to_string(),
                    ConfigPathPromptKind::Import => "config import cancelled".to_string(),
                    ConfigPathPromptKind::PortableExport => {
                        "portable snapshot export cancelled".to_string()
                    }
                    ConfigPathPromptKind::PortableImport => {
                        "portable snapshot import cancelled".to_string()
                    }
                    ConfigPathPromptKind::EncryptedPortableExport => {
                        "encrypted portable snapshot export cancelled".to_string()
                    }
                    ConfigPathPromptKind::EncryptedPortableImport => {
                        "encrypted portable snapshot import cancelled".to_string()
                    }
                };
                self.store_status.message = "config picker cancelled".to_string();
            }
            ConfigPathPromptResult::Failed(error) => {
                self.terminal_status = match kind {
                    ConfigPathPromptKind::Export => format!("config export failed: {error}"),
                    ConfigPathPromptKind::Import => format!("config import failed: {error}"),
                    ConfigPathPromptKind::PortableExport => {
                        format!("portable snapshot export failed: {error}")
                    }
                    ConfigPathPromptKind::PortableImport => {
                        format!("portable snapshot import failed: {error}")
                    }
                    ConfigPathPromptKind::EncryptedPortableExport => {
                        format!("encrypted portable snapshot export failed: {error}")
                    }
                    ConfigPathPromptKind::EncryptedPortableImport => {
                        format!("encrypted portable snapshot import failed: {error}")
                    }
                };
                self.store_status.message = self.terminal_status.clone();
                self.store_status.ready = false;
            }
            ConfigPathPromptResult::Closed => {
                self.terminal_status = "config path picker closed before returning".to_string();
                self.store_status.message = "config picker closed".to_string();
            }
        }
    }

    fn refresh_store_from_runtime(&mut self) {
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            Ok(store) => {
                let path = store.db_path().display().to_string();
                match store.load_sessions() {
                    Ok(config) => {
                        self.connections = config.connections;
                        self.tunnels = store.list_tunnels().unwrap_or_default();
                        let quick_commands = store.load_quick_commands().unwrap_or_default();
                        self.quick_commands = quick_commands.commands;
                        self.quick_command_categories = quick_commands.categories;
                        self.command_history = store.list_command_history(64).unwrap_or_default();
                        self.keyword_highlights =
                            store.load_keyword_highlights().unwrap_or_default();
                        self.settings = store.load_app_settings_summary().unwrap_or_default();
                        self.translation_settings = store
                            .load_translation_settings()
                            .unwrap_or_else(|_| TranslationSettings {
                                target_language: self.settings.language.clone(),
                                ..TranslationSettings::default()
                            });
                        self.translation_secret_draft = TranslationSecretDraft::default();
                        self.translate_target_language =
                            self.translation_settings.target_language.clone();
                        self.recording_manager
                            .set_memory_limit(self.settings.recording_memory_limit_bytes as usize);
                        self.cloud_sync_settings = store
                            .load_cloud_sync_settings()
                            .unwrap_or_else(|_| self.cloud_sync_settings.clone());
                        self.cloud_sync_state = store
                            .load_cloud_sync_state()
                            .unwrap_or_else(|_| self.cloud_sync_state.clone());
                        self.transfer_duplicate_policy = SftpDuplicatePolicy::from_legacy_value(
                            &self.settings.transfer_duplicate_strategy,
                        );
                        self.store_status = StoreStatus {
                            path,
                            message: "redb connection store online".to_string(),
                            ready: true,
                        };
                    }
                    Err(error) => {
                        self.connections.clear();
                        self.tunnels.clear();
                        self.quick_commands.clear();
                        self.quick_command_categories.clear();
                        self.command_history.clear();
                        self.keyword_highlights = KeywordHighlightConfig::default();
                        self.settings = AppSettingsSummary::default();
                        self.translation_settings = TranslationSettings::default();
                        self.translation_secret_draft = TranslationSecretDraft::default();
                        self.translate_target_language =
                            self.translation_settings.target_language.clone();
                        self.store_status = StoreStatus {
                            path,
                            message: format!("failed to load sessions: {error}"),
                            ready: false,
                        };
                    }
                }
            }
            Err(error) => {
                self.connections.clear();
                self.tunnels.clear();
                self.quick_commands.clear();
                self.quick_command_categories.clear();
                self.command_history.clear();
                self.settings = AppSettingsSummary::default();
                self.translation_settings = TranslationSettings::default();
                self.translation_secret_draft = TranslationSecretDraft::default();
                self.translate_target_language = self.translation_settings.target_language.clone();
                self.store_status = StoreStatus {
                    path: self
                        .runtime
                        .config_dir()
                        .join("nyaterm.redb")
                        .display()
                        .to_string(),
                    message: format!("failed to open store: {error}"),
                    ready: false,
                };
            }
        }
    }

    fn reveal_log_dir(&mut self, cx: &mut Context<Self>) {
        match std::fs::create_dir_all(self.runtime.log_dir()) {
            Ok(()) => {
                cx.reveal_path(self.runtime.log_dir());
                self.terminal_status =
                    format!("opened log directory {}", self.runtime.log_dir().display());
            }
            Err(error) => {
                self.terminal_status = format!("failed to prepare log directory: {error}");
            }
        }
        cx.notify();
    }

    fn prompt_diagnostics_export(&mut self, cx: &mut Context<Self>) {
        if self.diagnostics_path_prompt.is_some() {
            self.terminal_status = "diagnostics path picker is already open".to_string();
            cx.notify();
            return;
        }

        let directory = self.runtime.log_dir().to_path_buf();
        let receiver = cx.prompt_for_new_path(&directory, Some("nyaterm-diagnostics.zip"));
        let runtime = self.runtime.clone();
        let options = self.diagnostics_export_options();
        self.diagnostics_path_prompt = Some(DiagnosticsPathPromptKind::Export);
        self.terminal_status = "selecting diagnostics export destination".to_string();
        cx.spawn(async move |this, cx| {
            let result = match receiver.await {
                Ok(Ok(Some(path))) => match export_diagnostics_archive(&runtime, &options, &path) {
                    Ok(info) => DiagnosticsPathPromptResult::Exported(info),
                    Err(error) => DiagnosticsPathPromptResult::Failed(error.to_string()),
                },
                Ok(Ok(None)) => DiagnosticsPathPromptResult::Cancelled,
                Ok(Err(error)) => DiagnosticsPathPromptResult::Failed(error.to_string()),
                Err(_) => DiagnosticsPathPromptResult::Closed,
            };
            let _ = this.update(cx, |this, cx| {
                this.apply_diagnostics_path_prompt_result(result);
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn apply_diagnostics_path_prompt_result(&mut self, result: DiagnosticsPathPromptResult) {
        self.diagnostics_path_prompt = None;
        match result {
            DiagnosticsPathPromptResult::Exported(info) => {
                self.terminal_status = format!(
                    "diagnostics exported to {} ({} log file(s), {} bytes)",
                    info.output_path.display(),
                    info.log_files,
                    info.bytes
                );
            }
            DiagnosticsPathPromptResult::Cancelled => {
                self.terminal_status = "diagnostics export cancelled".to_string();
            }
            DiagnosticsPathPromptResult::Failed(error) => {
                self.terminal_status = format!("diagnostics export failed: {error}");
            }
            DiagnosticsPathPromptResult::Closed => {
                self.terminal_status =
                    "diagnostics path picker closed before returning".to_string();
            }
        }
    }

    fn diagnostics_export_options(&self) -> DiagnosticsExportOptions {
        DiagnosticsExportOptions {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            language: self.settings.language.clone(),
            log_level: self.settings.diagnostics_level.clone(),
            retention_days: self.settings.diagnostics_retention_days,
            runtime_snapshot: self.diagnostics_runtime_snapshot(),
        }
    }

    fn diagnostics_runtime_snapshot(&self) -> DiagnosticsRuntimeSnapshot {
        let sessions = self.session_manager.list_sessions().unwrap_or_default();
        let mut local_sessions = 0;
        let mut ssh_sessions = 0;
        let mut telnet_sessions = 0;
        let mut raw_tcp_sessions = 0;
        let mut serial_sessions = 0;
        for session in &sessions {
            match session.kind {
                SessionKind::LocalPty => local_sessions += 1,
                SessionKind::Ssh => ssh_sessions += 1,
                SessionKind::Telnet => telnet_sessions += 1,
                SessionKind::RawTcp => raw_tcp_sessions += 1,
                SessionKind::Serial => serial_sessions += 1,
            }
        }

        let open_tunnels = self
            .tunnel_manager
            .list()
            .map(|items| items.len())
            .unwrap_or(0);
        let mut running_transfers = 0;
        let mut paused_transfers = 0;
        let mut completed_transfers = 0;
        let mut failed_transfers = 0;
        for job in &self.transfer_jobs {
            match job.status {
                TransferJobStatus::Running | TransferJobStatus::Cancelling => {
                    running_transfers += 1
                }
                TransferJobStatus::Paused => paused_transfers += 1,
                TransferJobStatus::Completed => completed_transfers += 1,
                TransferJobStatus::Failed => failed_transfers += 1,
                TransferJobStatus::Cancelled => {}
            }
        }

        DiagnosticsRuntimeSnapshot {
            active_sessions: sessions.len(),
            local_sessions,
            ssh_sessions,
            telnet_sessions,
            raw_tcp_sessions,
            serial_sessions,
            open_tunnels,
            pending_tunnels: self.pending_tunnels.len(),
            saved_connections: self.connections.len(),
            saved_tunnels: self.tunnels.len(),
            running_transfers,
            paused_transfers,
            completed_transfers,
            failed_transfers,
        }
    }

    fn resolve_host_key_prompt(
        &mut self,
        request_id: String,
        choice: HostKeyPromptChoice,
        cx: &mut Context<Self>,
    ) {
        let Some(request) = self.active_host_key_prompt.take() else {
            self.terminal_status = "no SSH host key prompt is active".to_string();
            cx.notify();
            return;
        };

        if request.id != request_id {
            self.active_host_key_prompt = Some(request);
            self.terminal_status = "SSH host key prompt changed before response".to_string();
            cx.notify();
            return;
        }

        let host = request.host_key.host_identifier.clone();
        let _ = request.response_tx.send(choice);
        self.terminal_status = match choice {
            HostKeyPromptChoice::Accept => format!("accepted SSH host key for {host}"),
            HostKeyPromptChoice::Reject => format!("rejected SSH host key for {host}"),
        };
        cx.notify();
    }

    fn resolve_duplicate_prompt(
        &mut self,
        request_id: String,
        decision: SftpDuplicateDecision,
        cx: &mut Context<Self>,
    ) {
        let Some(prompt) = self.active_duplicate_prompt.take() else {
            self.terminal_status = "no SFTP duplicate prompt is active".to_string();
            cx.notify();
            return;
        };

        if prompt.id != request_id {
            self.active_duplicate_prompt = Some(prompt);
            self.terminal_status = "SFTP duplicate prompt changed before response".to_string();
            cx.notify();
            return;
        }

        let target = prompt.request.target_path.clone();
        let _ = prompt.response_tx.send(decision);
        self.terminal_status = format!(
            "SFTP duplicate decision for {target}: {}",
            duplicate_decision_label(decision)
        );
        cx.notify();
    }

    fn submit_credential_prompt(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.active_credential_prompt.take() else {
            return;
        };
        let host = credential_prompt_target(&state.prompt);
        let _ = state.response_tx.send(Some(state.value));
        self.terminal_status = format!("submitted SSH credential for {host}");
        cx.notify();
    }

    fn cancel_credential_prompt(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.active_credential_prompt.take() else {
            return;
        };
        let host = credential_prompt_target(&state.prompt);
        let _ = state.response_tx.send(None);
        self.terminal_status = format!("cancelled SSH credential prompt for {host}");
        cx.notify();
    }

    fn handle_credential_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let Some(state) = self.active_credential_prompt.as_mut() else {
            return;
        };
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "enter" => {
                self.submit_credential_prompt(cx);
            }
            "escape" => {
                self.cancel_credential_prompt(cx);
            }
            "backspace" => {
                state.value.pop();
                cx.notify();
            }
            _ => {
                if let Some(value) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|value| !value.is_empty())
                {
                    state.value.push_str(value);
                    cx.notify();
                }
            }
        }
    }

    fn handle_transfer_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        let value = match self.transfer_focused_field {
            TransferInputField::Remote => &mut self.transfer_remote_path,
            TransferInputField::Local => &mut self.transfer_local_path,
        };
        match keystroke.key.as_str() {
            "backspace" => {
                value.pop();
                cx.notify();
            }
            "escape" => {
                self.terminal_status = "transfer input blurred".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    value.push_str(input);
                    cx.notify();
                }
            }
        }
    }

    fn handle_command_search_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                self.command_search_draft.pop();
                cx.notify();
            }
            "escape" => {
                self.command_search_draft.clear();
                self.terminal_status = "command search cleared".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.command_search_draft.push_str(input);
                    cx.notify();
                }
            }
        }
    }

    fn command_search_results(&self) -> Vec<nyaterm_domain::FuzzyResult> {
        search_command_sources(
            &self.command_history,
            &self.quick_commands,
            &self.command_search_draft,
            8,
            Some(1),
            Some(512),
        )
    }

    fn handle_recording_search_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                self.recording_search_draft.pop();
                cx.notify();
            }
            "escape" => {
                self.recording_search_draft.clear();
                self.terminal_status = "recording search cleared".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.recording_search_draft.push_str(input);
                    cx.notify();
                }
            }
        }
    }

    fn recording_search_results(
        &self,
    ) -> Result<nyaterm_session::TerminalHistorySearchResponse, String> {
        let Some(session_id) = self.active_session_id.clone() else {
            return Ok(nyaterm_session::TerminalHistorySearchResponse {
                total: 0,
                elapsed_ms: 0,
                truncated: false,
                results: Vec::new(),
            });
        };
        self.recording_manager
            .search_history(TerminalHistorySearchRequest {
                session_id,
                query: self.recording_search_draft.trim().to_string(),
                case_sensitive: false,
                regex: false,
                whole_word: false,
                limit: Some(8),
                context_before: Some(1),
                context_after: Some(1),
                max_lines: None,
            })
            .map_err(|error| error.to_string())
    }

    fn handle_cloud_sync_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                self.cloud_sync_input_value_mut().pop();
                self.cloud_sync_status = "cloud sync settings edited".to_string();
                cx.notify();
            }
            "escape" => {
                self.cloud_sync_status = "cloud sync input blurred".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.cloud_sync_input_value_mut().push_str(input);
                    self.cloud_sync_status = "cloud sync settings edited".to_string();
                    cx.notify();
                }
            }
        }
    }

    fn cloud_sync_input_value_mut(&mut self) -> &mut String {
        match self.cloud_sync_focused_field {
            CloudSyncInputField::RemoteRoot => &mut self.cloud_sync_settings.remote_root,
            CloudSyncInputField::WebdavEndpoint => &mut self.cloud_sync_settings.webdav.endpoint,
            CloudSyncInputField::WebdavRoot => &mut self.cloud_sync_settings.webdav.root,
            CloudSyncInputField::WebdavUsername => &mut self.cloud_sync_settings.webdav.username,
            CloudSyncInputField::WebdavPassword => {
                &mut self.cloud_sync_secret_draft.webdav_password
            }
            CloudSyncInputField::S3Endpoint => &mut self.cloud_sync_settings.s3.endpoint,
            CloudSyncInputField::S3Bucket => &mut self.cloud_sync_settings.s3.bucket,
            CloudSyncInputField::S3Region => &mut self.cloud_sync_settings.s3.region,
            CloudSyncInputField::S3Root => &mut self.cloud_sync_settings.s3.root,
            CloudSyncInputField::S3AccessKeyId => {
                &mut self.cloud_sync_secret_draft.s3_access_key_id
            }
            CloudSyncInputField::S3SecretAccessKey => {
                &mut self.cloud_sync_secret_draft.s3_secret_access_key
            }
            CloudSyncInputField::S3SessionToken => {
                &mut self.cloud_sync_secret_draft.s3_session_token
            }
            CloudSyncInputField::GoogleDriveRoot => &mut self.cloud_sync_settings.google_drive.root,
            CloudSyncInputField::GoogleDriveAccessToken => {
                &mut self.cloud_sync_secret_draft.google_drive_access_token
            }
            CloudSyncInputField::GoogleDriveRefreshToken => {
                &mut self.cloud_sync_secret_draft.google_drive_refresh_token
            }
            CloudSyncInputField::GoogleDriveClientId => {
                let value = self
                    .cloud_sync_settings
                    .google_drive
                    .client_id
                    .get_or_insert_with(String::new);
                value
            }
            CloudSyncInputField::GoogleDriveClientSecret => {
                &mut self.cloud_sync_secret_draft.google_drive_client_secret
            }
            CloudSyncInputField::OneDriveRoot => &mut self.cloud_sync_settings.onedrive.root,
            CloudSyncInputField::OneDriveAccessToken => {
                &mut self.cloud_sync_secret_draft.onedrive_access_token
            }
            CloudSyncInputField::OneDriveRefreshToken => {
                &mut self.cloud_sync_secret_draft.onedrive_refresh_token
            }
            CloudSyncInputField::OneDriveClientId => {
                let value = self
                    .cloud_sync_settings
                    .onedrive
                    .client_id
                    .get_or_insert_with(String::new);
                value
            }
            CloudSyncInputField::OneDriveClientSecret => {
                &mut self.cloud_sync_secret_draft.onedrive_client_secret
            }
            CloudSyncInputField::AliyunDriveRoot => &mut self.cloud_sync_settings.aliyun_drive.root,
            CloudSyncInputField::AliyunDriveType => {
                &mut self.cloud_sync_settings.aliyun_drive.drive_type
            }
            CloudSyncInputField::AliyunDriveAccessToken => {
                &mut self.cloud_sync_secret_draft.aliyun_drive_access_token
            }
            CloudSyncInputField::AliyunDriveRefreshToken => {
                &mut self.cloud_sync_secret_draft.aliyun_drive_refresh_token
            }
            CloudSyncInputField::AliyunDriveClientId => {
                let value = self
                    .cloud_sync_settings
                    .aliyun_drive
                    .client_id
                    .get_or_insert_with(String::new);
                value
            }
            CloudSyncInputField::AliyunDriveClientSecret => {
                &mut self.cloud_sync_secret_draft.aliyun_drive_client_secret
            }
            CloudSyncInputField::GiteeEndpoint => {
                &mut self.cloud_sync_settings.gitee_snippet.api_endpoint
            }
            CloudSyncInputField::GiteeGistId => &mut self.cloud_sync_settings.gitee_snippet.gist_id,
            CloudSyncInputField::GiteeToken => &mut self.cloud_sync_secret_draft.gitee_token,
            CloudSyncInputField::GithubGistId => &mut self.cloud_sync_settings.github_gist.gist_id,
            CloudSyncInputField::GithubToken => &mut self.cloud_sync_secret_draft.github_token,
        }
    }

    fn handle_ai_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                self.ai_input_value_mut().pop();
                self.ai_status = "AI settings edited".to_string();
                cx.notify();
            }
            "escape" => {
                self.ai_status = "AI input blurred".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.ai_input_value_mut().push_str(input);
                    self.ai_status = "AI settings edited".to_string();
                    cx.notify();
                }
            }
        }
    }

    fn ai_input_value_mut(&mut self) -> &mut String {
        match self.ai_focused_field {
            AiInputField::Model => &mut self.ai_model_draft,
            AiInputField::BaseUrl => &mut self.ai_base_url_draft,
            AiInputField::ApiKey => &mut self.ai_secret_draft,
        }
    }

    fn handle_translate_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }
        let settings_field = self.translate_focused_field.is_settings_field();

        match keystroke.key.as_str() {
            "backspace" => {
                self.translate_input_value_mut().pop();
                self.translate_status = if settings_field {
                    "translation settings edited".to_string()
                } else {
                    "translation input edited".to_string()
                };
                cx.notify();
            }
            "enter" if self.translate_focused_field == TranslateInputField::Text => {
                self.translate_input.push('\n');
                self.translate_status = "translation input edited".to_string();
                cx.notify();
            }
            "escape" => {
                self.translate_status = if settings_field {
                    "translation settings input blurred".to_string()
                } else {
                    "translation input blurred".to_string()
                };
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.translate_input_value_mut().push_str(input);
                    self.translate_status = if settings_field {
                        "translation settings edited".to_string()
                    } else {
                        "translation input edited".to_string()
                    };
                    cx.notify();
                }
            }
        }
    }

    fn translate_input_value_mut(&mut self) -> &mut String {
        match self.translate_focused_field {
            TranslateInputField::TargetLanguage => &mut self.translate_target_language,
            TranslateInputField::Text => &mut self.translate_input,
            TranslateInputField::SettingsTargetLanguage => {
                &mut self.translation_settings.target_language
            }
            TranslateInputField::DeeplApiKey => &mut self.translation_secret_draft.deepl_api_key,
            TranslateInputField::BaiduAppId => &mut self.translation_settings.baidu_app_id,
            TranslateInputField::BaiduAppKey => &mut self.translation_secret_draft.baidu_app_key,
            TranslateInputField::AliAppId => &mut self.translation_settings.ali_app_id,
            TranslateInputField::AliAppKey => &mut self.translation_secret_draft.ali_app_key,
            TranslateInputField::YoudaoAppId => &mut self.translation_settings.youdao_app_id,
            TranslateInputField::YoudaoAppKey => &mut self.translation_secret_draft.youdao_app_key,
        }
    }

    fn drain_session_start_events(&mut self) {
        while let Ok(event) = self.session_start_rx.try_recv() {
            self.pending_session_name = None;
            match event.result {
                Ok(session_id) => {
                    self.active_ssh_config = self.pending_ssh_config.take();
                    self.active_ai_execution_profile = self.pending_ai_execution_profile;
                    self.pending_ai_execution_profile = AiExecutionProfile::SendOnly;
                    self.active_session_id = Some(session_id.clone());
                    self.terminal_status = format!("running {}", short_id(&session_id));
                    self.append_terminal_log(format!(
                        "\n# started {} ({})\n",
                        event.connection_name,
                        short_id(&session_id)
                    ));
                    self.maybe_auto_start_recording(&session_id, &event.connection_name);
                    self.selected_nav = NavItem::Workspace;
                }
                Err(error) => {
                    self.pending_ssh_config = None;
                    self.pending_ai_execution_profile = AiExecutionProfile::SendOnly;
                    self.active_ssh_config = None;
                    self.active_ai_execution_profile = AiExecutionProfile::SendOnly;
                    self.terminal_status =
                        format!("failed to start {}: {error}", event.connection_name);
                    self.append_terminal_log(format!(
                        "\n# failed to start {}: {error}\n",
                        event.connection_name
                    ));
                    self.selected_nav = NavItem::Workspace;
                }
            }
        }
    }

    fn drain_transfer_events(&mut self) {
        while let Ok(event) = self.transfer_rx.try_recv() {
            let Some(job) = self
                .transfer_jobs
                .iter_mut()
                .find(|candidate| candidate.id == event.id)
            else {
                continue;
            };
            match event.event {
                TransferJobEvent::Progress(progress) => {
                    if job.status == TransferJobStatus::Running {
                        job.detail = format_transfer_progress(&progress);
                    }
                    job.progress = Some(progress);
                }
                TransferJobEvent::Finished(Ok(TransferJobOutput::Entries(entries))) => {
                    job.status = TransferJobStatus::Completed;
                    job.detail = format!("{} item(s)", entries.len());
                    job.entries = entries;
                    job.summary = None;
                    job.progress = None;
                    job.control = None;
                    self.terminal_status = format!("SFTP list completed: {}", job.detail);
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
                    });
                    job.summary = Some(summary);
                    self.terminal_status = format!("SFTP transfer completed: {}", job.detail);
                    job.control = None;
                }
                TransferJobEvent::Finished(Err(error)) => {
                    if error == SFTP_TRANSFER_CANCELLED {
                        job.status = TransferJobStatus::Cancelled;
                        job.detail = "Cancelled".to_string();
                        self.terminal_status = format!("SFTP transfer cancelled: {}", job.id);
                    } else {
                        job.status = TransferJobStatus::Failed;
                        job.detail = error.clone();
                        self.terminal_status = format!("SFTP transfer failed: {error}");
                    }
                    job.summary = None;
                    job.control = None;
                }
            }
        }
    }

    fn drain_tunnel_events(&mut self) {
        while let Ok(event) = self.tunnel_rx.try_recv() {
            self.pending_tunnels.retain(|id| id != &event.tunnel_id);
            match event.result {
                Ok(TunnelJobOutput::Opened(info)) => {
                    self.terminal_status = format!(
                        "tunnel {} open on {}:{}",
                        event.tunnel_id, info.bind_host, info.listen_port
                    );
                }
                Ok(TunnelJobOutput::Closed) => {
                    self.terminal_status = format!("tunnel {} closed", event.tunnel_id);
                }
                Err(error) => {
                    self.terminal_status = format!("tunnel {} failed: {error}", event.tunnel_id);
                }
            }
        }
    }

    fn drain_process_events(&mut self) {
        while let Ok(event) = self.process_rx.try_recv() {
            self.process_pending = false;
            match event.result {
                Ok(ProcessJobOutput::Listed(processes)) => {
                    self.process_status = format!("loaded {} remote process(es)", processes.len());
                    self.terminal_status = self.process_status.clone();
                    self.processes = processes;
                }
                Ok(ProcessJobOutput::Signalled { pid, signal }) => {
                    self.process_status = format!("sent {signal} to pid {pid}");
                    self.terminal_status = self.process_status.clone();
                }
                Ok(ProcessJobOutput::Reniced { pid, nice }) => {
                    self.process_status = format!("reniced pid {pid} to {nice}");
                    self.terminal_status = self.process_status.clone();
                }
                Err(error) => {
                    self.process_status = format!("process operation failed: {error}");
                    self.terminal_status = self.process_status.clone();
                }
            }
        }
    }

    fn drain_docker_events(&mut self) {
        while let Ok(event) = self.docker_rx.try_recv() {
            self.docker_pending = false;
            match event.result {
                Ok(DockerJobOutput::Overview(overview)) => {
                    self.docker_status = if overview.available {
                        format!(
                            "Docker {} · {} container(s)",
                            if overview.version.trim().is_empty() {
                                "available".to_string()
                            } else {
                                overview.version.clone()
                            },
                            overview.containers.len()
                        )
                    } else {
                        "Docker is not available on this SSH host".to_string()
                    };
                    self.terminal_status = self.docker_status.clone();
                    self.docker_overview = Some(overview);
                }
                Ok(DockerJobOutput::ContainerAction {
                    container_id,
                    action,
                }) => {
                    self.docker_status = format!("Docker {action} {}", compact_id(&container_id));
                    self.terminal_status = self.docker_status.clone();
                    self.docker_overview = None;
                }
                Ok(DockerJobOutput::Logs { container_id, text }) => {
                    self.docker_status = format!("loaded logs for {}", compact_id(&container_id));
                    self.terminal_status = self.docker_status.clone();
                    self.docker_logs = truncate_preview(&text, 4000);
                }
                Ok(DockerJobOutput::Pruned) => {
                    self.docker_status = "Docker system prune completed".to_string();
                    self.terminal_status = self.docker_status.clone();
                    self.docker_overview = None;
                }
                Err(error) => {
                    self.docker_status = format!("Docker operation failed: {error}");
                    self.terminal_status = self.docker_status.clone();
                }
            }
        }
    }

    fn drain_stats_events(&mut self) {
        while let Ok(event) = self.stats_rx.try_recv() {
            self.stats_pending = false;
            match event.result {
                Ok(stats) => {
                    self.stats_status = format!(
                        "loaded stats for {} · load {:.2}/{:.2}/{:.2}",
                        if stats.system.hostname.trim().is_empty() {
                            "remote host"
                        } else {
                            stats.system.hostname.as_str()
                        },
                        stats.load.load1,
                        stats.load.load5,
                        stats.load.load15
                    );
                    self.terminal_status = self.stats_status.clone();
                    self.remote_stats = Some(stats);
                }
                Err(error) => {
                    self.stats_status = format!("stats refresh failed: {error}");
                    self.terminal_status = self.stats_status.clone();
                }
            }
        }
    }

    fn drain_translate_events(&mut self) {
        while let Ok(event) = self.translate_rx.try_recv() {
            self.translate_pending = false;
            match event.result {
                Ok(result) => {
                    self.translate_status = format!(
                        "translated {} character(s) from {}",
                        result.original.chars().count(),
                        result.detected_language
                    );
                    self.terminal_status = self.translate_status.clone();
                    self.translate_result = Some(result);
                }
                Err(error) => {
                    self.translate_status = format!("translation failed: {error}");
                    self.terminal_status = self.translate_status.clone();
                }
            }
        }
    }

    fn start_update_check(&mut self, cx: &mut Context<Self>) {
        if self.update_pending {
            self.update_status = "update check already running".to_string();
            cx.notify();
            return;
        }
        self.update_pending = true;
        self.update_status = "checking GitHub releases...".to_string();
        self.update_info = None;
        let tx = self.update_tx.clone();
        std::thread::spawn(move || {
            let result = check_native_update();
            let _ = tx.send(UpdateJobResult { result });
        });
        cx.notify();
    }

    fn drain_update_events(&mut self) {
        while let Ok(event) = self.update_rx.try_recv() {
            self.update_pending = false;
            match event.result {
                Ok(info) => {
                    self.update_status = if info.available {
                        format!(
                            "update available: {} -> {}",
                            info.current_version, info.latest_version
                        )
                    } else {
                        format!("NyaTerm is up to date ({})", info.current_version)
                    };
                    self.terminal_status = self.update_status.clone();
                    self.update_info = Some(info);
                }
                Err(error) => {
                    self.update_status = format!("update check failed: {error}");
                    self.terminal_status = self.update_status.clone();
                    self.update_info = None;
                }
            }
        }
    }

    fn drain_host_key_prompts(&mut self) {
        if self.active_host_key_prompt.is_some() {
            return;
        }

        if let Some(request) = self.host_key_prompts.pop_pending() {
            self.terminal_status = format!(
                "SSH host key decision required for {}",
                request.host_key.host_identifier
            );
            self.active_host_key_prompt = Some(request);
        }
    }

    fn drain_credential_prompts(&mut self) {
        if self.active_credential_prompt.is_some() {
            return;
        }

        if let Some(request) = self.credential_prompts.pop_pending() {
            self.terminal_status = format!(
                "SSH credential required for {}",
                credential_prompt_target(&request.prompt)
            );
            self.active_credential_prompt = Some(CredentialPromptState {
                id: request.id,
                prompt: request.prompt,
                response_tx: request.response_tx,
                value: String::new(),
            });
        }
    }

    fn drain_duplicate_prompts(&mut self) {
        if self.active_duplicate_prompt.is_some() {
            return;
        }

        if let Some(request) = self.duplicate_prompts.pop_pending() {
            self.terminal_status = format!(
                "SFTP duplicate decision required for {}",
                request.request.target_path
            );
            self.active_duplicate_prompt = Some(SftpDuplicatePromptState {
                id: request.id,
                request: request.request,
                response_tx: request.response_tx,
            });
        }
    }

    fn drain_session_events(&mut self, cx: &mut Context<Self>) {
        let Ok(events) = self.session_manager.drain_events(64) else {
            self.terminal_status = "failed to drain session events".to_string();
            return;
        };

        for event in events {
            match event {
                SessionEvent::Output { session_id, data } => {
                    if self.active_session_id.as_deref() == Some(session_id.as_str()) {
                        let text = String::from_utf8_lossy(&data);
                        if self.ai_agent_capture.has_active() {
                            let result = self.ai_agent_capture.process(&text);
                            if !result.visible_text.is_empty() {
                                self.recording_manager
                                    .write_output(&session_id, &result.visible_text);
                                self.append_terminal_log(&result.visible_text);
                            }
                            for captured in result.completed {
                                self.handle_ai_agent_captured_output(captured, cx);
                            }
                        } else {
                            self.recording_manager.write_output(&session_id, &text);
                            self.append_terminal_bytes(&data);
                        }
                    }
                }
                SessionEvent::Exited { session_id } => {
                    if self.active_session_id.as_deref() == Some(session_id.as_str()) {
                        self.recording_manager.cleanup_session(&session_id);
                        self.active_session_id = None;
                        self.active_ssh_config = None;
                        self.active_ai_execution_profile = AiExecutionProfile::SendOnly;
                        self.ai_agent_loop = None;
                        self.ai_agent_capture = AgentOutputCaptureProcessor::new();
                        self.terminal_status = "session exited".to_string();
                        self.append_terminal_log("\n# session exited\n");
                    }
                }
                SessionEvent::Error {
                    session_id,
                    message,
                } => {
                    if session_id.is_empty()
                        || self.active_session_id.as_deref() == Some(session_id.as_str())
                    {
                        self.terminal_status = format!("session error: {message}");
                        self.append_terminal_log(format!("\n# session error: {message}\n"));
                    }
                }
            }
        }
        self.drain_session_start_events();
        self.drain_tunnel_events();
        self.drain_process_events();
        self.drain_stats_events();
        self.drain_translate_events();
        self.drain_update_events();
        self.drain_docker_events();
        self.drain_transfer_events();
        self.drain_ai_discovery_events();
        self.drain_ai_chat_events(cx);
        self.drive_ai_agent_loop(cx);
        self.drain_host_key_prompts();
        self.drain_credential_prompts();
        self.drain_duplicate_prompts();
    }

    fn ensure_event_pump(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.event_pump_started {
            return;
        }
        self.event_pump_started = true;

        window
            .spawn(cx, async move |cx| {
                loop {
                    Timer::after(Duration::from_millis(50)).await;
                    let keep_running = cx
                        .update_root(|root, _window, cx| {
                            let Ok(view) = root.downcast::<NyaTermApp>() else {
                                return false;
                            };
                            view.update(cx, |this, cx| {
                                this.drain_session_events(cx);
                                cx.notify();
                                this.event_pump_started
                            })
                        })
                        .unwrap_or(false);
                    if !keep_running {
                        break;
                    }
                }
            })
            .detach();
    }
}

impl Render for NyaTermApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(rgb(0x111318))
            .text_color(rgb(0xe7e9ee))
            .font_family("Inter")
            .child(
                div()
                    .flex()
                    .flex_col()
                    .size_full()
                    .child(self.title_bar())
                    .child(
                        div()
                            .flex()
                            .flex_1()
                            .min_h_0()
                            .child(self.sidebar(cx))
                            .child(self.main_surface(cx)),
                    ),
            )
    }
}

impl NyaTermApp {
    fn title_bar(&self) -> impl IntoElement {
        div()
            .h(px(42.))
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .border_b_1()
            .border_color(rgb(0x262a33))
            .bg(rgb(0x151820))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(logo_mark())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("NyaTerm Native"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x8f98aa))
                                    .child("GPUI desktop workspace"),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(status_pill("native", rgb(0x6ee7b7), rgb(0x14352b)))
                    .child(status_pill("no webview", rgb(0xfbbf24), rgb(0x3a2f14))),
            )
    }

    fn sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(224.))
            .flex_none()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .border_r_1()
            .border_color(rgb(0x252a35))
            .bg(rgb(0x171a22))
            .child(self.nav_button("Workspace", NavItem::Workspace, cx))
            .child(self.nav_button("Connections", NavItem::Connections, cx))
            .child(self.nav_button("Tunnels", NavItem::Tunnels, cx))
            .child(self.nav_button("Stats", NavItem::Stats, cx))
            .child(self.nav_button("Processes", NavItem::Processes, cx))
            .child(self.nav_button("Docker", NavItem::Docker, cx))
            .child(self.nav_button("Translation", NavItem::Translation, cx))
            .child(self.nav_button("Transfers", NavItem::Transfers, cx))
            .child(self.nav_button("Settings", NavItem::Settings, cx))
            .child(self.nav_button("Migration", NavItem::Migration, cx))
            .child(div().flex_1())
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2d3442))
                    .bg(rgb(0x10131a))
                    .p_3()
                    .child(div().text_xs().text_color(rgb(0x8f98aa)).child("Runtime"))
                    .child(div().mt_1().text_sm().child(match self.runtime.mode() {
                        RuntimeMode::Portable => "Portable",
                        RuntimeMode::Installed => "Installed",
                    }))
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(rgb(0x8f98aa))
                            .child(self.runtime.config_dir().display().to_string()),
                    ),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(if self.store_status.ready {
                        rgb(0x244638)
                    } else {
                        rgb(0x4a2525)
                    })
                    .bg(rgb(0x10131a))
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x8f98aa))
                            .child("Config Store"),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_sm()
                            .text_color(if self.store_status.ready {
                                rgb(0x6ee7b7)
                            } else {
                                rgb(0xfca5a5)
                            })
                            .child(self.store_status.message.clone()),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(rgb(0x8f98aa))
                            .child(self.store_status.path.clone()),
                    ),
            )
    }

    fn nav_button(
        &self,
        label: &'static str,
        item: NavItem,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.selected_nav == item;
        div()
            .id(SharedString::from(format!("nav-{label}")))
            .h(px(36.))
            .px_3()
            .flex()
            .items_center()
            .rounded_md()
            .cursor_pointer()
            .text_sm()
            .when(selected, |this| {
                this.bg(rgb(0x2b3342)).text_color(rgb(0xffffff))
            })
            .when(!selected, |this| {
                this.text_color(rgb(0xaeb7c8))
                    .hover(|hover| hover.bg(rgb(0x202632)).text_color(rgb(0xffffff)))
            })
            .child(label)
            .on_click(cx.listener(move |this, _, _, cx| this.select(item, cx)))
    }

    fn main_surface(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .bg(rgb(0x0f1117))
            .child(match self.selected_nav {
                NavItem::Workspace => self.workspace_view(cx).into_any_element(),
                NavItem::Connections => self.connections_view(cx).into_any_element(),
                NavItem::Tunnels => self.tunnels_view(cx).into_any_element(),
                NavItem::Stats => self.stats_view(cx).into_any_element(),
                NavItem::Processes => self.processes_view(cx).into_any_element(),
                NavItem::Docker => self.docker_view(cx).into_any_element(),
                NavItem::Translation => self.translation_view(cx).into_any_element(),
                NavItem::Transfers => self.transfers_view(cx).into_any_element(),
                NavItem::Settings => self.settings_view(cx).into_any_element(),
                NavItem::Migration => self.migration_view().into_any_element(),
            })
    }

    fn workspace_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut workspace = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x2a3140))
            .bg(rgb(0x0b0d12))
            .child(
                div()
                    .h(px(40.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .border_b_1()
                    .border_color(rgb(0x252b37))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Terminal Workspace"),
                    )
                    .child(status_pill(
                        status_label(&self.terminal_status),
                        rgb(0x93c5fd),
                        rgb(0x17253b),
                    )),
            );

        if let Some(prompt) = self.active_host_key_prompt.clone() {
            workspace = workspace.child(self.host_key_prompt_banner(prompt, cx));
        }
        if let Some(prompt) = self.active_credential_prompt.clone() {
            workspace = workspace.child(self.credential_prompt_banner(prompt, cx));
        }

        div()
            .flex()
            .flex_1()
            .min_h_0()
            .p_4()
            .gap_4()
            .child(workspace.child(self.terminal_canvas(cx)))
            .child(self.right_panel(cx))
    }

    fn stats_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let can_refresh = self.active_ssh_config.is_some() && !self.stats_pending;
        let stats = self.remote_stats.clone().unwrap_or_default();
        let memory_total = stats.memory.used.saturating_add(stats.memory.available);
        let memory_percent = if memory_total > 0 {
            stats.memory.used as f64 / memory_total as f64 * 100.
        } else {
            0.
        };
        let disk_summary = stats
            .disks
            .iter()
            .max_by_key(|disk| disk.use_percent)
            .map(|disk| format!("{} {}%", disk.mount, disk.use_percent))
            .unwrap_or_else(|| "n/a".to_string());
        let net_summary = stats
            .networks
            .iter()
            .map(|net| net.rx_bytes_per_sec + net.tx_bytes_per_sec)
            .fold(0.0, f64::max);

        let mut networks = div().flex().flex_col().gap_2();
        if self.remote_stats.is_none() {
            networks = networks.child(empty_panel(if self.active_ssh_config.is_some() {
                "No stats snapshot loaded."
            } else {
                "Start an SSH session to inspect remote stats."
            }));
        } else if stats.networks.is_empty() {
            networks = networks.child(empty_panel("No active physical network interfaces found."));
        } else {
            for network in stats.networks.iter().take(8) {
                networks = networks.child(stats_resource_row(
                    &network.nic,
                    &format!(
                        "{} · rx {} · tx {}",
                        network.state,
                        format_rate(network.rx_bytes_per_sec),
                        format_rate(network.tx_bytes_per_sec)
                    ),
                    (network.rx_bytes_per_sec + network.tx_bytes_per_sec) / net_summary.max(1.0),
                ));
            }
        }

        let mut disks = div().flex().flex_col().gap_2();
        if self.remote_stats.is_some() && stats.disks.is_empty() {
            disks = disks.child(empty_panel("No mounted block devices found."));
        } else {
            for disk in stats.disks.iter().take(8) {
                disks = disks.child(stats_resource_row(
                    &disk.mount,
                    &format!(
                        "{} · {} free of {}",
                        disk.device,
                        format_file_size(Some(disk.available)),
                        format_file_size(Some(disk.total))
                    ),
                    disk.use_percent as f64 / 100.,
                ));
            }
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_5()
            .gap_4()
            .child(section_header(
                "Stats",
                "Native SSH exec system snapshot for the active remote session.",
            ))
            .child(
                div()
                    .grid()
                    .grid_cols(6)
                    .gap_3()
                    .child(metric(
                        "SSH",
                        if self.active_ssh_config.is_some() {
                            "ready".to_string()
                        } else {
                            "none".to_string()
                        },
                    ))
                    .child(metric(
                        "Host",
                        if stats.system.hostname.trim().is_empty() {
                            "n/a".to_string()
                        } else {
                            truncate_preview(&stats.system.hostname, 28)
                        },
                    ))
                    .child(metric("CPU", format!("{:.1}%", stats.cpu.usage)))
                    .child(metric("Load", format!("{:.2}", stats.load.load1)))
                    .child(metric("Memory", format!("{memory_percent:.0}%")))
                    .child(metric("Disk", disk_summary)),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xe5edf7))
                                    .child(self.stats_status.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .when(!can_refresh, |this| this.opacity(0.45))
                                    .child(small_button(
                                        "stats-refresh",
                                        "Refresh",
                                        cx.listener(|this, _, window, cx| {
                                            this.refresh_stats(window, cx);
                                        }),
                                    )),
                            ),
                    ),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(3)
                    .gap_3()
                    .child(
                        div()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x2a3140))
                            .bg(rgb(0x151923))
                            .p_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("System"),
                            )
                            .child(capability_line(
                                "OS",
                                truncate_preview(&stats.system.os, 52),
                            ))
                            .child(capability_line("Arch", stats.system.arch.clone()))
                            .child(capability_line(
                                "Uptime",
                                format_uptime(stats.system.uptime_sec),
                            ))
                            .child(capability_line(
                                "CPU Model",
                                truncate_preview(&stats.cpu.model, 52),
                            ))
                            .child(capability_line("Cores", stats.cpu.cores.to_string())),
                    )
                    .child(
                        div()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x2a3140))
                            .bg(rgb(0x151923))
                            .p_4()
                            .child(div().text_sm().font_weight(FontWeight(700.)).child("Load"))
                            .child(capability_line("1 min", format!("{:.2}", stats.load.load1)))
                            .child(capability_line("5 min", format!("{:.2}", stats.load.load5)))
                            .child(capability_line(
                                "15 min",
                                format!("{:.2}", stats.load.load15),
                            ))
                            .child(capability_line(
                                "Per Core",
                                stats
                                    .cpu
                                    .per_core
                                    .iter()
                                    .take(8)
                                    .map(|usage| format!("{usage:.0}%"))
                                    .collect::<Vec<_>>()
                                    .join(" "),
                            )),
                    )
                    .child(
                        div()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x2a3140))
                            .bg(rgb(0x151923))
                            .p_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("Memory"),
                            )
                            .child(capability_line(
                                "Used",
                                format_file_size(Some(stats.memory.used)),
                            ))
                            .child(capability_line(
                                "Available",
                                format_file_size(Some(stats.memory.available)),
                            ))
                            .child(capability_line(
                                "Cached",
                                format_file_size(Some(stats.memory.cached)),
                            ))
                            .child(stats_progress_bar(memory_percent / 100.)),
                    ),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_3()
                    .child(
                        div()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x2a3140))
                            .bg(rgb(0x151923))
                            .p_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("Network"),
                            )
                            .child(networks),
                    )
                    .child(
                        div()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x2a3140))
                            .bg(rgb(0x151923))
                            .p_4()
                            .child(div().text_sm().font_weight(FontWeight(700.)).child("Disks"))
                            .child(disks),
                    ),
            )
    }

    fn processes_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let can_list = self.active_ssh_config.is_some() && !self.process_pending;
        let mut rows = div().mt_3().flex().flex_col().gap_2();
        if self.processes.is_empty() {
            rows = rows.child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x10151e))
                    .p_4()
                    .text_sm()
                    .text_color(rgb(0xaeb7c8))
                    .child(if self.active_ssh_config.is_some() {
                        "No process snapshot loaded."
                    } else {
                        "Start an SSH session to list remote processes."
                    }),
            );
        } else {
            let mut processes = self.processes.clone();
            processes.sort_by(|left, right| {
                right
                    .cpu_percent
                    .partial_cmp(&left.cpu_percent)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(left.pid.cmp(&right.pid))
            });
            for process in processes.into_iter().take(24) {
                rows = rows.child(process_row(
                    &process,
                    cx.listener({
                        let pid = process.pid;
                        move |this, _, window, cx| {
                            this.signal_process(pid, "TERM", window, cx);
                        }
                    }),
                    cx.listener({
                        let pid = process.pid;
                        move |this, _, window, cx| {
                            this.signal_process(pid, "KILL", window, cx);
                        }
                    }),
                    cx.listener({
                        let pid = process.pid;
                        move |this, _, window, cx| {
                            this.renice_process(pid, 5, window, cx);
                        }
                    }),
                    cx.listener({
                        let pid = process.pid;
                        move |this, _, window, cx| {
                            this.renice_process(pid, -5, window, cx);
                        }
                    }),
                ));
            }
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_5()
            .gap_4()
            .child(section_header(
                "Processes",
                "Native SSH exec process inspector for the active remote session.",
            ))
            .child(
                div()
                    .grid()
                    .grid_cols(4)
                    .gap_3()
                    .child(metric(
                        "SSH",
                        if self.active_ssh_config.is_some() {
                            "ready".to_string()
                        } else {
                            "none".to_string()
                        },
                    ))
                    .child(metric("Processes", self.processes.len().to_string()))
                    .child(metric(
                        "CPU Top",
                        self.processes
                            .iter()
                            .max_by(|left, right| {
                                left.cpu_percent
                                    .partial_cmp(&right.cpu_percent)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .map(|process| format!("{:.1}%", process.cpu_percent))
                            .unwrap_or_else(|| "0.0%".to_string()),
                    ))
                    .child(metric(
                        "Status",
                        if self.process_pending {
                            "running".to_string()
                        } else {
                            "idle".to_string()
                        },
                    )),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xe5edf7))
                                    .child(self.process_status.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .when(!can_list, |this| this.opacity(0.45))
                                    .child(small_button(
                                        "process-refresh",
                                        "Refresh",
                                        cx.listener(|this, _, window, cx| {
                                            this.refresh_processes(window, cx);
                                        }),
                                    )),
                            ),
                    ),
            )
            .child(rows)
    }

    fn docker_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let can_run = self.active_ssh_config.is_some() && !self.docker_pending;
        let overview = self.docker_overview.clone().unwrap_or_default();
        let mut rows = div().mt_3().flex().flex_col().gap_2();
        if self.docker_overview.is_none() {
            rows = rows.child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x10151e))
                    .p_4()
                    .text_sm()
                    .text_color(rgb(0xaeb7c8))
                    .child(if self.active_ssh_config.is_some() {
                        "No Docker snapshot loaded."
                    } else {
                        "Start an SSH session to inspect remote Docker."
                    }),
            );
        } else if !overview.available {
            rows = rows.child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x10151e))
                    .p_4()
                    .text_sm()
                    .text_color(rgb(0xaeb7c8))
                    .child("Docker is not installed or the daemon is not reachable."),
            );
        } else if overview.containers.is_empty() {
            rows = rows.child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x10151e))
                    .p_4()
                    .text_sm()
                    .text_color(rgb(0xaeb7c8))
                    .child("No containers found."),
            );
        } else {
            let mut containers = overview.containers.clone();
            containers.sort_by(|left, right| {
                docker_state_rank(&left.state)
                    .cmp(&docker_state_rank(&right.state))
                    .then(left.name.cmp(&right.name))
            });
            for container in containers.into_iter().take(24) {
                let container_id = container.id.clone();
                rows = rows.child(
                    div()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(0x2a3140))
                        .bg(rgb(0x151923))
                        .p_3()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_3()
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_weight(FontWeight(700.))
                                                .text_color(rgb(0xe5edf7))
                                                .child(truncate_preview(&container.name, 48)),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_family("JetBrains Mono")
                                                .text_color(rgb(0x98a3b8))
                                                .child(format!(
                                                    "{} · {}",
                                                    compact_id(&container.id),
                                                    truncate_preview(&container.image, 64)
                                                )),
                                        ),
                                )
                                .child(status_pill(
                                    docker_state_label(&container.state),
                                    docker_state_color(&container.state),
                                    rgb(0x17233a),
                                )),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0xaeb7c8))
                                .line_height(px(18.))
                                .child(truncate_preview(&container.status, 120)),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0x64748b))
                                .child(truncate_preview(&container.ports, 120)),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_end()
                                .gap_1()
                                .child(small_button(
                                    format!("docker-logs-{}", compact_id(&container_id)),
                                    "Logs",
                                    cx.listener({
                                        let container_id = container_id.clone();
                                        move |this, _, window, cx| {
                                            this.load_docker_logs(container_id.clone(), window, cx);
                                        }
                                    }),
                                ))
                                .child(small_button(
                                    format!("docker-start-{}", compact_id(&container_id)),
                                    "Start",
                                    cx.listener({
                                        let container_id = container_id.clone();
                                        move |this, _, window, cx| {
                                            this.docker_container_action(
                                                container_id.clone(),
                                                "start",
                                                window,
                                                cx,
                                            );
                                        }
                                    }),
                                ))
                                .child(small_button(
                                    format!("docker-stop-{}", compact_id(&container_id)),
                                    "Stop",
                                    cx.listener({
                                        let container_id = container_id.clone();
                                        move |this, _, window, cx| {
                                            this.docker_container_action(
                                                container_id.clone(),
                                                "stop",
                                                window,
                                                cx,
                                            );
                                        }
                                    }),
                                ))
                                .child(small_button(
                                    format!("docker-restart-{}", compact_id(&container_id)),
                                    "Restart",
                                    cx.listener({
                                        let container_id = container_id.clone();
                                        move |this, _, window, cx| {
                                            this.docker_container_action(
                                                container_id.clone(),
                                                "restart",
                                                window,
                                                cx,
                                            );
                                        }
                                    }),
                                )),
                        ),
                );
            }
        }

        let logs = if self.docker_logs.trim().is_empty() {
            "No logs loaded.".to_string()
        } else {
            self.docker_logs.clone()
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_5()
            .gap_4()
            .child(section_header(
                "Docker",
                "Native SSH exec Docker manager for the active remote session.",
            ))
            .child(
                div()
                    .grid()
                    .grid_cols(6)
                    .gap_3()
                    .child(metric(
                        "SSH",
                        if self.active_ssh_config.is_some() {
                            "ready".to_string()
                        } else {
                            "none".to_string()
                        },
                    ))
                    .child(metric(
                        "Docker",
                        if overview.available {
                            "available".to_string()
                        } else {
                            "unknown".to_string()
                        },
                    ))
                    .child(metric(
                        "Version",
                        if overview.version.trim().is_empty() {
                            "n/a".to_string()
                        } else {
                            overview.version.clone()
                        },
                    ))
                    .child(metric("Containers", overview.containers.len().to_string()))
                    .child(metric("Images", overview.images.len().to_string()))
                    .child(metric(
                        "Compose",
                        if overview.compose_available {
                            overview.compose_projects.len().to_string()
                        } else {
                            "off".to_string()
                        },
                    )),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xe5edf7))
                                    .child(self.docker_status.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .when(!can_run, |this| this.opacity(0.45))
                                    .child(small_button(
                                        "docker-refresh",
                                        "Refresh",
                                        cx.listener(|this, _, window, cx| {
                                            this.refresh_docker(window, cx);
                                        }),
                                    ))
                                    .child(small_button(
                                        "docker-prune",
                                        "Prune",
                                        cx.listener(|this, _, window, cx| {
                                            this.prune_docker_system(window, cx);
                                        }),
                                    )),
                            ),
                    ),
            )
            .child(rows)
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_3()
                    .child(
                        div()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x2a3140))
                            .bg(rgb(0x151923))
                            .p_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("Resources"),
                            )
                            .child(capability_line(
                                "Volumes",
                                overview.volumes.len().to_string(),
                            ))
                            .child(capability_line(
                                "Networks",
                                overview.networks.len().to_string(),
                            ))
                            .child(capability_line(
                                "Compose Projects",
                                overview.compose_projects.len().to_string(),
                            )),
                    )
                    .child(
                        div()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x2a3140))
                            .bg(rgb(0x151923))
                            .p_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("Recent Logs"),
                            )
                            .child(
                                div()
                                    .mt_3()
                                    .max_h(px(180.))
                                    .overflow_hidden()
                                    .font_family("JetBrains Mono")
                                    .text_xs()
                                    .line_height(px(18.))
                                    .text_color(rgb(0xaeb7c8))
                                    .child(logs),
                            ),
                    ),
            )
    }

    fn translation_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let can_translate = !self.translate_pending && !self.translate_input.trim().is_empty();
        let input_value = if self.translate_input.is_empty() {
            " ".to_string()
        } else {
            self.translate_input.clone()
        };
        let target_value = if self.translate_target_language.is_empty() {
            " ".to_string()
        } else {
            self.translate_target_language.clone()
        };
        let result_text = self
            .translate_result
            .as_ref()
            .map(|result| result.translated.clone())
            .unwrap_or_else(|| "No translation yet.".to_string());
        let detected = self
            .translate_result
            .as_ref()
            .map(|result| result.detected_language.clone())
            .unwrap_or_else(|| "auto".to_string());
        let credential_status = match self.translate_provider.as_str() {
            "google" | "microsoft" => "not required".to_string(),
            "deepl" => configured_status(&self.translation_settings.deepl_api_key),
            "baidu" => configured_pair_status(
                &self.translation_settings.baidu_app_id,
                &self.translation_settings.baidu_app_key,
            ),
            "ali" => configured_pair_status(
                &self.translation_settings.ali_app_id,
                &self.translation_settings.ali_app_key,
            ),
            "youdao" => configured_pair_status(
                &self.translation_settings.youdao_app_id,
                &self.translation_settings.youdao_app_key,
            ),
            _ => "unsupported".to_string(),
        };
        let mut provider_controls = div().flex().items_center().gap_2().flex_wrap();
        for provider in ["google", "microsoft", "deepl", "baidu", "ali", "youdao"] {
            let selected = self.translate_provider == provider;
            provider_controls =
                provider_controls.child(div().when(!selected, |this| this.opacity(0.58)).child(
                    small_button(
                        format!("translate-provider-{provider}"),
                        provider,
                        cx.listener(move |this, _, _, cx| {
                            this.set_translate_provider(provider, cx);
                        }),
                    ),
                ));
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_5()
            .gap_4()
            .child(section_header(
                "Translation",
                "Native translation for selected terminal text or manual input.",
            ))
            .child(
                div()
                    .grid()
                    .grid_cols(4)
                    .gap_3()
                    .child(metric("Provider", self.translate_provider.clone()))
                    .child(metric("Target", self.translate_target_language.clone()))
                    .child(metric("Detected", detected))
                    .child(metric("Credentials", credential_status)),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(provider_controls)
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xe5edf7))
                                    .child(self.translate_status.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .when(!can_translate, |this| this.opacity(0.45))
                                    .child(small_button(
                                        "translate-run",
                                        "Translate",
                                        cx.listener(|this, _, window, cx| {
                                            this.run_translation(window, cx);
                                        }),
                                    )),
                            ),
                    ),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(3)
                    .gap_3()
                    .child(
                        div()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x2a3140))
                            .bg(rgb(0x151923))
                            .p_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("Target"),
                            )
                            .child(
                                div()
                                    .id(SharedString::from("translate-target-input"))
                                    .mt_3()
                                    .h(px(36.))
                                    .px_3()
                                    .flex()
                                    .items_center()
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(rgb(0x303848))
                                    .bg(rgb(0x10151e))
                                    .font_family("JetBrains Mono")
                                    .text_sm()
                                    .child(target_value)
                                    .track_focus(&self.translate_focus)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.translate_focused_field =
                                            TranslateInputField::TargetLanguage;
                                        window.focus(&this.translate_focus);
                                        cx.notify();
                                    }))
                                    .on_key_down(cx.listener(
                                        |this, event: &KeyDownEvent, _, cx| {
                                            this.handle_translate_key_down(event, cx);
                                        },
                                    )),
                            )
                            .child(
                                div()
                                    .mt_3()
                                    .text_xs()
                                    .text_color(rgb(0x98a3b8))
                                    .child("Examples: zh-CN, en, ja, ko, fr, de."),
                            ),
                    )
                    .child(
                        div()
                            .col_span(2)
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x2a3140))
                            .bg(rgb(0x151923))
                            .p_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("Source Text"),
                            )
                            .child(
                                div()
                                    .id(SharedString::from("translate-source-input"))
                                    .mt_3()
                                    .min_h(px(150.))
                                    .p_3()
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(rgb(0x303848))
                                    .bg(rgb(0x10151e))
                                    .font_family("JetBrains Mono")
                                    .text_sm()
                                    .line_height(px(18.))
                                    .whitespace_normal()
                                    .child(input_value)
                                    .track_focus(&self.translate_focus)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.translate_focused_field = TranslateInputField::Text;
                                        window.focus(&this.translate_focus);
                                        cx.notify();
                                    }))
                                    .on_key_down(cx.listener(
                                        |this, event: &KeyDownEvent, _, cx| {
                                            this.handle_translate_key_down(event, cx);
                                        },
                                    )),
                            ),
                    ),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Result"),
                    )
                    .child(
                        div()
                            .mt_3()
                            .min_h(px(160.))
                            .p_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(0x303848))
                            .bg(rgb(0x10151e))
                            .text_sm()
                            .line_height(px(20.))
                            .text_color(rgb(0xdbeafe))
                            .child(result_text),
                    ),
            )
    }

    fn transfers_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let ssh_status = if self.active_ssh_config.is_some() {
            "SSH session ready"
        } else if self.pending_ssh_config.is_some() {
            "SSH session connecting"
        } else {
            "No SSH session"
        };
        let can_transfer = self.active_ssh_config.is_some();
        let remote_value = if self.transfer_remote_path.is_empty() {
            " ".to_string()
        } else {
            self.transfer_remote_path.clone()
        };
        let local_value = if self.transfer_local_path.is_empty() {
            " ".to_string()
        } else {
            self.transfer_local_path.clone()
        };
        let mut jobs = div().mt_3().flex().flex_col().gap_2();
        if self.transfer_jobs.is_empty() {
            jobs = jobs.child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x10151e))
                    .p_4()
                    .text_sm()
                    .text_color(rgb(0xaeb7c8))
                    .child("Queue is empty."),
            );
        } else {
            for job in self.transfer_jobs.iter().rev().take(8) {
                let status_color = match job.status {
                    TransferJobStatus::Running => rgb(0xfacc15),
                    TransferJobStatus::Paused => rgb(0x93c5fd),
                    TransferJobStatus::Cancelling => rgb(0xfbbf24),
                    TransferJobStatus::Cancelled => rgb(0x94a3b8),
                    TransferJobStatus::Completed => rgb(0x34d399),
                    TransferJobStatus::Failed => rgb(0xfb7185),
                };
                let mut status_action = div().flex().items_center().gap_1();
                if job.status == TransferJobStatus::Running && job.control.is_some() {
                    let job_id = job.id.clone();
                    status_action = status_action.child(small_button(
                        format!("transfer-pause-{job_id}"),
                        "Pause",
                        cx.listener(move |this, _, _, cx| {
                            this.pause_transfer_job(&job_id, cx);
                        }),
                    ));
                }
                if job.status == TransferJobStatus::Paused && job.control.is_some() {
                    let job_id = job.id.clone();
                    status_action = status_action.child(small_button(
                        format!("transfer-resume-{job_id}"),
                        "Resume",
                        cx.listener(move |this, _, _, cx| {
                            this.resume_transfer_job(&job_id, cx);
                        }),
                    ));
                }
                if matches!(
                    job.status,
                    TransferJobStatus::Running | TransferJobStatus::Paused
                ) && job.control.is_some()
                {
                    let job_id = job.id.clone();
                    status_action = status_action.child(small_button(
                        format!("transfer-cancel-{job_id}"),
                        "Cancel",
                        cx.listener(move |this, _, _, cx| {
                            this.cancel_transfer_job(&job_id, cx);
                        }),
                    ));
                }
                let mut entries = div().mt_2().flex().flex_col().gap_1();
                for entry in job.entries.iter().take(6) {
                    entries = entries.child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .text_xs()
                            .text_color(rgb(0xaeb7c8))
                            .child(entry_kind_label(entry.file_type))
                            .child(div().flex_1().min_w_0().child(entry.name.clone()))
                            .child(format_file_size(entry.size)),
                    );
                }
                if let Some(summary) = job.summary.as_ref() {
                    entries =
                        entries.child(div().mt_2().text_xs().text_color(rgb(0xaeb7c8)).child(
                            format!(
                                "{} -> {}",
                                summary.remote_path,
                                summary.local_path.display()
                            ),
                        ));
                }
                let progress = job
                    .progress
                    .as_ref()
                    .map(transfer_progress_bar)
                    .unwrap_or_else(|| div().into_any_element());
                jobs = jobs.child(
                    div()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(0x2a3140))
                        .bg(rgb(0x151923))
                        .p_4()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_3()
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(rgb(0xe5edf7))
                                                .child(transfer_job_title(&job.kind)),
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(rgb(0xaeb7c8))
                                                .child(job.detail.clone()),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight(700.))
                                                .text_color(status_color)
                                                .child(transfer_status_label(job.status)),
                                        )
                                        .child(status_action),
                                ),
                        )
                        .child(progress)
                        .child(entries),
                );
            }
        }

        let mut view = div()
            .size_full()
            .p_5()
            .child(section_header("Transfers", "No active transfer jobs."));
        if let Some(prompt) = self.active_duplicate_prompt.clone() {
            view = view.child(self.duplicate_prompt_banner(prompt, cx));
        }
        view = view.child(
            div()
                .mt_4()
                .rounded_md()
                .border_1()
                .border_color(rgb(0x2a3140))
                .bg(rgb(0x151923))
                .p_4()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(div().text_sm().text_color(rgb(0xe5edf7)).child("Queue"))
                        .child(div().text_xs().text_color(rgb(0xaeb7c8)).child(ssh_status)),
                )
                .child(
                    div()
                        .grid()
                        .grid_cols(2)
                        .gap_2()
                        .child(
                            transfer_input(
                                "transfer-remote-path",
                                "Remote",
                                remote_value,
                                self.transfer_focused_field == TransferInputField::Remote,
                            )
                            .track_focus(&self.transfer_focus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.transfer_focused_field = TransferInputField::Remote;
                                window.focus(&this.transfer_focus);
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(
                                |this, event: &KeyDownEvent, _, cx| {
                                    cx.stop_propagation();
                                    this.handle_transfer_key_down(event, cx);
                                },
                            )),
                        )
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(
                                    transfer_input(
                                        "transfer-local-path",
                                        "Local",
                                        local_value,
                                        self.transfer_focused_field == TransferInputField::Local,
                                    )
                                    .flex_1()
                                    .track_focus(&self.transfer_focus)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.transfer_focused_field = TransferInputField::Local;
                                        window.focus(&this.transfer_focus);
                                        cx.notify();
                                    }))
                                    .on_key_down(cx.listener(
                                        |this, event: &KeyDownEvent, _, cx| {
                                            cx.stop_propagation();
                                            this.handle_transfer_key_down(event, cx);
                                        },
                                    )),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(small_button(
                                            "transfer-pick-download-dir",
                                            "Save To",
                                            cx.listener(|this, _, _, cx| {
                                                this.prompt_transfer_path(
                                                    TransferPathPromptKind::DownloadDirectory,
                                                    cx,
                                                );
                                            }),
                                        ))
                                        .child(
                                            div()
                                                .flex()
                                                .gap_1()
                                                .child(small_button(
                                                    "transfer-pick-upload-file",
                                                    "File",
                                                    cx.listener(|this, _, _, cx| {
                                                        this.prompt_transfer_path(
                                                            TransferPathPromptKind::UploadFile,
                                                            cx,
                                                        );
                                                    }),
                                                ))
                                                .child(small_button(
                                                    "transfer-pick-upload-dir",
                                                    "Dir",
                                                    cx.listener(|this, _, _, cx| {
                                                        this.prompt_transfer_path(
                                                            TransferPathPromptKind::UploadDirectory,
                                                            cx,
                                                        );
                                                    }),
                                                )),
                                        ),
                                ),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .child(div().text_xs().text_color(rgb(0x98a3b8)).child("Duplicate"))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(policy_button(
                                    "transfer-policy-ask",
                                    "Ask",
                                    self.transfer_duplicate_policy == SftpDuplicatePolicy::Ask,
                                    cx.listener(|this, _, _, cx| {
                                        this.transfer_duplicate_policy = SftpDuplicatePolicy::Ask;
                                        this.terminal_status =
                                            "transfer duplicate policy: ask".to_string();
                                        cx.notify();
                                    }),
                                ))
                                .child(policy_button(
                                    "transfer-policy-overwrite",
                                    "Overwrite",
                                    self.transfer_duplicate_policy
                                        == SftpDuplicatePolicy::Overwrite,
                                    cx.listener(|this, _, _, cx| {
                                        this.transfer_duplicate_policy =
                                            SftpDuplicatePolicy::Overwrite;
                                        this.terminal_status =
                                            "transfer duplicate policy: overwrite".to_string();
                                        cx.notify();
                                    }),
                                ))
                                .child(policy_button(
                                    "transfer-policy-skip",
                                    "Skip",
                                    self.transfer_duplicate_policy == SftpDuplicatePolicy::Skip,
                                    cx.listener(|this, _, _, cx| {
                                        this.transfer_duplicate_policy = SftpDuplicatePolicy::Skip;
                                        this.terminal_status =
                                            "transfer duplicate policy: skip".to_string();
                                        cx.notify();
                                    }),
                                ))
                                .child(policy_button(
                                    "transfer-policy-rename",
                                    "Rename",
                                    self.transfer_duplicate_policy == SftpDuplicatePolicy::Rename,
                                    cx.listener(|this, _, _, cx| {
                                        this.transfer_duplicate_policy =
                                            SftpDuplicatePolicy::Rename;
                                        this.terminal_status =
                                            "transfer duplicate policy: rename".to_string();
                                        cx.notify();
                                    }),
                                )),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .when(!can_transfer, |this| this.opacity(0.45))
                        .child(small_button(
                            "transfer-list",
                            "List",
                            cx.listener(|this, _, window, cx| {
                                this.start_sftp_list_job(window, cx);
                            }),
                        ))
                        .child(small_button(
                            "transfer-download",
                            "Download",
                            cx.listener(|this, _, window, cx| {
                                this.start_sftp_download_job(window, cx);
                            }),
                        ))
                        .child(small_button(
                            "transfer-upload",
                            "Upload",
                            cx.listener(|this, _, window, cx| {
                                this.start_sftp_upload_job(window, cx);
                            }),
                        )),
                ),
        );
        view = view.child(jobs);

        view
    }

    fn duplicate_prompt_banner(
        &mut self,
        prompt: SftpDuplicatePromptState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let overwrite_id = prompt.id.clone();
        let skip_id = prompt.id.clone();
        let rename_id = prompt.id.clone();
        let direction = match prompt.request.direction {
            SftpTransferDirection::Download => "Download duplicate",
            SftpTransferDirection::Upload => "Upload duplicate",
        };
        let kind = if prompt.request.is_directory {
            "directory"
        } else {
            "file"
        };

        div()
            .mt_4()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x7c5d1f))
            .bg(rgb(0x201a0c))
            .p_3()
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child(direction),
                            )
                            .child(
                                div().text_xs().text_color(rgb(0xcbd5e1)).child(format!(
                                    "Target {kind}: {}",
                                    prompt.request.target_path
                                )),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x98a3b8))
                                    .child(format!("Source: {}", prompt.request.source_path)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                format!("duplicate-overwrite-{overwrite_id}"),
                                "Overwrite",
                                cx.listener(move |this, _, _, cx| {
                                    this.resolve_duplicate_prompt(
                                        overwrite_id.clone(),
                                        SftpDuplicateDecision::Overwrite,
                                        cx,
                                    );
                                }),
                            ))
                            .child(small_button(
                                format!("duplicate-skip-{skip_id}"),
                                "Skip",
                                cx.listener(move |this, _, _, cx| {
                                    this.resolve_duplicate_prompt(
                                        skip_id.clone(),
                                        SftpDuplicateDecision::Skip,
                                        cx,
                                    );
                                }),
                            ))
                            .child(small_button(
                                format!("duplicate-rename-{rename_id}"),
                                "Rename",
                                cx.listener(move |this, _, _, cx| {
                                    this.resolve_duplicate_prompt(
                                        rename_id.clone(),
                                        SftpDuplicateDecision::Rename,
                                        cx,
                                    );
                                }),
                            )),
                    ),
            )
    }

    fn host_key_prompt_banner(
        &mut self,
        prompt: HostKeyPromptRequest,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let accept_id = prompt.id.clone();
        let reject_id = prompt.id.clone();
        let tone = match prompt.issue {
            HostKeyPromptIssue::Unknown => "Unknown SSH host key",
            HostKeyPromptIssue::Changed => "Changed SSH host key",
        };
        let action = match prompt.issue {
            HostKeyPromptIssue::Unknown => "Accept will add this key to known_hosts.",
            HostKeyPromptIssue::Changed => "Accept will replace the stored key for this host.",
        };

        div()
            .mx_3()
            .mt_3()
            .rounded_md()
            .border_1()
            .border_color(match prompt.issue {
                HostKeyPromptIssue::Unknown => rgb(0x7c5d1f),
                HostKeyPromptIssue::Changed => rgb(0x7f1d1d),
            })
            .bg(match prompt.issue {
                HostKeyPromptIssue::Unknown => rgb(0x201a0c),
                HostKeyPromptIssue::Changed => rgb(0x211111),
            })
            .p_3()
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_sm().font_weight(FontWeight(700.)).child(tone))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0xcbd5e1))
                                    .child(prompt.host_key.host_identifier.clone()),
                            )
                            .child(div().text_xs().text_color(rgb(0x98a3b8)).child(format!(
                                "{} {}",
                                prompt.host_key.key_type, prompt.host_key.fingerprint
                            )))
                            .child(div().text_xs().text_color(rgb(0x98a3b8)).child(action)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                format!("host-key-reject-{reject_id}"),
                                "Reject",
                                cx.listener(move |this, _, _, cx| {
                                    this.resolve_host_key_prompt(
                                        reject_id.clone(),
                                        HostKeyPromptChoice::Reject,
                                        cx,
                                    );
                                }),
                            ))
                            .child(small_button(
                                format!("host-key-accept-{accept_id}"),
                                "Accept",
                                cx.listener(move |this, _, _, cx| {
                                    this.resolve_host_key_prompt(
                                        accept_id.clone(),
                                        HostKeyPromptChoice::Accept,
                                        cx,
                                    );
                                }),
                            )),
                    ),
            )
    }

    fn credential_prompt_banner(
        &mut self,
        prompt: CredentialPromptState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let title = match prompt.prompt.kind {
            SshCredentialPromptKind::Password => "SSH Password",
            SshCredentialPromptKind::KeyPassphrase => "SSH Key Passphrase",
            SshCredentialPromptKind::KeyboardInteractive => "SSH Verification",
        };
        let reason = match prompt.prompt.reason {
            SshCredentialPromptReason::MissingPassword => "Password is required.",
            SshCredentialPromptReason::PasswordRejected => "Previous password was rejected.",
            SshCredentialPromptReason::KeyPassphraseRequired => {
                "Passphrase is required to unlock the key."
            }
            SshCredentialPromptReason::KeyboardInteractive => {
                "Server requested keyboard-interactive verification."
            }
        };
        let display_value = if prompt.value.is_empty() {
            " ".to_string()
        } else if prompt.prompt.echo {
            prompt.value.clone()
        } else {
            "*".repeat(prompt.value.chars().count())
        };
        let mut details = div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().text_sm().font_weight(FontWeight(700.)).child(title))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(0xcbd5e1))
                    .child(credential_prompt_target(&prompt.prompt)),
            )
            .child(div().text_xs().text_color(rgb(0x98a3b8)).child(reason));
        if let Some(prompt_text) = prompt
            .prompt
            .prompt_text
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            details = details.child(
                div()
                    .mt_1()
                    .text_xs()
                    .text_color(rgb(0xe2e8f0))
                    .child(prompt_text.to_string()),
            );
        }

        div()
            .mx_3()
            .mt_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x365f87))
            .bg(rgb(0x101a26))
            .p_3()
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_3()
                    .child(details)
                    .child(
                        div()
                            .id(SharedString::from(format!(
                                "credential-input-{}",
                                prompt.id
                            )))
                            .w(px(240.))
                            .h(px(32.))
                            .px_3()
                            .flex()
                            .items_center()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(0x4b6f97))
                            .bg(rgb(0x07111d))
                            .font_family("JetBrains Mono")
                            .text_sm()
                            .track_focus(&self.credential_focus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                window.focus(&this.credential_focus);
                                this.terminal_status = "credential prompt focused".to_string();
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                cx.stop_propagation();
                                this.handle_credential_key_down(event, cx);
                            }))
                            .child(display_value),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                format!("credential-cancel-{}", prompt.id),
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_credential_prompt(cx);
                                }),
                            ))
                            .child(small_button(
                                format!("credential-submit-{}", prompt.id),
                                "Submit",
                                cx.listener(|this, _, _, cx| {
                                    this.submit_credential_prompt(cx);
                                }),
                            )),
                    ),
            )
    }

    fn snapshot_password_prompt_banner(
        &mut self,
        prompt: SnapshotPasswordPromptState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let title = match prompt.kind {
            SnapshotPasswordPromptKind::Export => "Encrypted Snapshot Export",
            SnapshotPasswordPromptKind::Import => "Encrypted Snapshot Import",
            SnapshotPasswordPromptKind::CloudPush => "Cloud Sync Push",
            SnapshotPasswordPromptKind::CloudPull => "Cloud Sync Pull",
            SnapshotPasswordPromptKind::CloudForcePush => "Force Cloud Sync Push",
            SnapshotPasswordPromptKind::CloudForcePull => "Force Cloud Sync Pull",
            SnapshotPasswordPromptKind::CloudProviderPush => "Provider Sync Push",
            SnapshotPasswordPromptKind::CloudProviderPull => "Provider Sync Pull",
            SnapshotPasswordPromptKind::CloudProviderForcePush => "Force Provider Sync Push",
            SnapshotPasswordPromptKind::CloudProviderForcePull => "Force Provider Sync Pull",
        };
        let masked = if prompt.value.is_empty() {
            " ".to_string()
        } else {
            "*".repeat(prompt.value.chars().count())
        };

        div()
            .mt_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x365f87))
            .bg(rgb(0x101a26))
            .p_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(div().text_sm().font_weight(FontWeight(700.)).child(title))
                            .child(div().text_xs().text_color(rgb(0x98a3b8)).child(
                                match prompt.kind {
                                    SnapshotPasswordPromptKind::CloudPush
                                    | SnapshotPasswordPromptKind::CloudPull
                                    | SnapshotPasswordPromptKind::CloudForcePush
                                    | SnapshotPasswordPromptKind::CloudForcePull
                                    | SnapshotPasswordPromptKind::CloudProviderPush
                                    | SnapshotPasswordPromptKind::CloudProviderPull
                                    | SnapshotPasswordPromptKind::CloudProviderForcePush
                                    | SnapshotPasswordPromptKind::CloudProviderForcePull => {
                                        "Password encrypts or decrypts this cloud snapshot."
                                    }
                                    _ => "Password is used only for this .nya operation.",
                                },
                            )),
                    )
                    .child(
                        div()
                            .id(SharedString::from("snapshot-password-input"))
                            .w(px(240.))
                            .h(px(32.))
                            .px_3()
                            .flex()
                            .items_center()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(0x4b6f97))
                            .bg(rgb(0x07111d))
                            .font_family("JetBrains Mono")
                            .text_sm()
                            .track_focus(&self.snapshot_password_focus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                window.focus(&this.snapshot_password_focus);
                                this.terminal_status =
                                    "snapshot password prompt focused".to_string();
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                cx.stop_propagation();
                                this.handle_snapshot_password_key_down(event, cx);
                            }))
                            .child(masked),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                "snapshot-password-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_snapshot_password_prompt(cx);
                                }),
                            ))
                            .child(small_button(
                                "snapshot-password-submit",
                                "Submit",
                                cx.listener(|this, _, _, cx| {
                                    this.submit_snapshot_password_prompt(cx);
                                }),
                            )),
                    ),
            )
    }

    fn cloud_sync_input(
        &mut self,
        id: &'static str,
        label: &'static str,
        value: String,
        field: CloudSyncInputField,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        transfer_input(
            id,
            label,
            if value.is_empty() {
                " ".to_string()
            } else {
                value
            },
            self.cloud_sync_focused_field == field,
        )
        .track_focus(&self.cloud_sync_focus)
        .on_click(cx.listener(move |this, _, window, cx| {
            this.cloud_sync_focused_field = field;
            window.focus(&this.cloud_sync_focus);
            cx.notify();
        }))
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
            cx.stop_propagation();
            this.handle_cloud_sync_key_down(event, cx);
        }))
    }

    fn cloud_sync_conflict_banner(
        &mut self,
        conflict: CloudSyncConflictState,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let provider_action = conflict.provider_action;
        let local_hash = self
            .cloud_sync_state
            .last_synced_payload_hash
            .as_deref()
            .map(compact_id)
            .unwrap_or_else(|| "unsynced".to_string());
        let remote_revision = self
            .cloud_sync_state
            .last_applied_remote_revision
            .as_deref()
            .map(compact_id)
            .unwrap_or_else(|| "unknown".to_string());

        div()
            .mt_3()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x8a5f1c))
            .bg(rgb(0x1f1a10))
            .p_3()
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .text_color(rgb(0xfacc15))
                                    .child("Cloud Sync Conflict"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0xe2e8f0))
                                    .child(conflict.message),
                            )
                            .child(div().text_xs().text_color(rgb(0x98a3b8)).child(format!(
                                "{} · local {} · remote {}",
                                conflict.provider, local_hash, remote_revision
                            ))),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                "cloud-conflict-force-push",
                                "Force Push",
                                cx.listener(move |this, _, _, cx| {
                                    this.prompt_cloud_sync_force_push(provider_action, cx);
                                }),
                            ))
                            .child(small_button(
                                "cloud-conflict-force-pull",
                                "Force Pull",
                                cx.listener(move |this, _, _, cx| {
                                    this.prompt_cloud_sync_force_pull(provider_action, cx);
                                }),
                            ))
                            .child(small_button(
                                "cloud-conflict-dismiss",
                                "Dismiss",
                                cx.listener(|this, _, _, cx| {
                                    this.dismiss_cloud_sync_conflict(cx);
                                }),
                            )),
                    ),
            )
    }

    fn ai_input(
        &mut self,
        id: &'static str,
        label: &'static str,
        value: String,
        field: AiInputField,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        transfer_input(
            id,
            label,
            if value.is_empty() {
                " ".to_string()
            } else {
                value
            },
            self.ai_focused_field == field,
        )
        .track_focus(&self.ai_focus)
        .on_click(cx.listener(move |this, _, window, cx| {
            this.ai_focused_field = field;
            window.focus(&this.ai_focus);
            cx.notify();
        }))
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
            cx.stop_propagation();
            this.handle_ai_key_down(event, cx);
        }))
    }

    fn translation_input(
        &mut self,
        id: &'static str,
        label: &'static str,
        value: String,
        field: TranslateInputField,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        transfer_input(
            id,
            label,
            if value.is_empty() {
                " ".to_string()
            } else {
                value
            },
            self.translate_focused_field == field,
        )
        .track_focus(&self.translate_focus)
        .on_click(cx.listener(move |this, _, window, cx| {
            this.translate_focused_field = field;
            window.focus(&this.translate_focus);
            cx.notify();
        }))
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
            cx.stop_propagation();
            this.handle_translate_key_down(event, cx);
        }))
    }

    fn translation_settings_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let translation_target_value = self.translation_settings.target_language.clone();
        let deepl_key_value = cloud_secret_display(
            &self.translation_secret_draft.deepl_api_key,
            &none_if_blank(&self.translation_settings.deepl_api_key),
        );
        let baidu_app_id_value = self.translation_settings.baidu_app_id.clone();
        let baidu_key_value = cloud_secret_display(
            &self.translation_secret_draft.baidu_app_key,
            &none_if_blank(&self.translation_settings.baidu_app_key),
        );
        let ali_app_id_value = self.translation_settings.ali_app_id.clone();
        let ali_key_value = cloud_secret_display(
            &self.translation_secret_draft.ali_app_key,
            &none_if_blank(&self.translation_settings.ali_app_key),
        );
        let youdao_app_id_value = self.translation_settings.youdao_app_id.clone();
        let youdao_key_value = cloud_secret_display(
            &self.translation_secret_draft.youdao_app_key,
            &none_if_blank(&self.translation_settings.youdao_app_key),
        );
        let translation_secret_count = [
            &self.translation_settings.deepl_api_key,
            &self.translation_settings.baidu_app_key,
            &self.translation_settings.ali_app_key,
            &self.translation_settings.youdao_app_key,
        ]
        .iter()
        .filter(|value| !value.trim().is_empty())
        .count();

        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x2a3140))
            .bg(rgb(0x151923))
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Translation"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x98a3b8))
                            .child(self.translate_status.clone()),
                    ),
            )
            .child(
                div()
                    .mt_3()
                    .grid()
                    .grid_cols(4)
                    .gap_3()
                    .child(metric("Provider", self.translate_provider.clone()))
                    .child(metric(
                        "Target",
                        self.translation_settings.target_language.clone(),
                    ))
                    .child(metric("Secrets", translation_secret_count.to_string()))
                    .child(metric(
                        "Runtime",
                        if self.translate_pending {
                            "running".to_string()
                        } else {
                            "idle".to_string()
                        },
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(policy_button(
                        "translation-provider-google-settings",
                        "Google",
                        self.translate_provider == "google",
                        cx.listener(|this, _, _, cx| {
                            this.set_translate_provider("google", cx);
                        }),
                    ))
                    .child(policy_button(
                        "translation-provider-microsoft-settings",
                        "Microsoft",
                        self.translate_provider == "microsoft",
                        cx.listener(|this, _, _, cx| {
                            this.set_translate_provider("microsoft", cx);
                        }),
                    ))
                    .child(policy_button(
                        "translation-provider-deepl-settings",
                        "DeepL",
                        self.translate_provider == "deepl",
                        cx.listener(|this, _, _, cx| {
                            this.set_translate_provider("deepl", cx);
                        }),
                    ))
                    .child(policy_button(
                        "translation-provider-baidu-settings",
                        "Baidu",
                        self.translate_provider == "baidu",
                        cx.listener(|this, _, _, cx| {
                            this.set_translate_provider("baidu", cx);
                        }),
                    ))
                    .child(policy_button(
                        "translation-provider-ali-settings",
                        "Ali",
                        self.translate_provider == "ali",
                        cx.listener(|this, _, _, cx| {
                            this.set_translate_provider("ali", cx);
                        }),
                    ))
                    .child(policy_button(
                        "translation-provider-youdao-settings",
                        "Youdao",
                        self.translate_provider == "youdao",
                        cx.listener(|this, _, _, cx| {
                            this.set_translate_provider("youdao", cx);
                        }),
                    ))
                    .child(small_button(
                        "translation-settings-save",
                        "Save",
                        cx.listener(|this, _, _, cx| {
                            this.save_translation_settings(cx);
                        }),
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .grid()
                    .grid_cols(3)
                    .gap_2()
                    .child(self.translation_input(
                        "translation-target-language",
                        "Target",
                        translation_target_value,
                        TranslateInputField::SettingsTargetLanguage,
                        cx,
                    ))
                    .child(self.translation_input(
                        "translation-deepl-key",
                        "DeepL Key",
                        deepl_key_value,
                        TranslateInputField::DeeplApiKey,
                        cx,
                    ))
                    .child(self.translation_input(
                        "translation-baidu-app-id",
                        "Baidu App ID",
                        baidu_app_id_value,
                        TranslateInputField::BaiduAppId,
                        cx,
                    ))
                    .child(self.translation_input(
                        "translation-baidu-app-key",
                        "Baidu App Key",
                        baidu_key_value,
                        TranslateInputField::BaiduAppKey,
                        cx,
                    ))
                    .child(self.translation_input(
                        "translation-ali-app-id",
                        "Ali App ID",
                        ali_app_id_value,
                        TranslateInputField::AliAppId,
                        cx,
                    ))
                    .child(self.translation_input(
                        "translation-ali-app-key",
                        "Ali App Key",
                        ali_key_value,
                        TranslateInputField::AliAppKey,
                        cx,
                    ))
                    .child(self.translation_input(
                        "translation-youdao-app-id",
                        "Youdao App ID",
                        youdao_app_id_value,
                        TranslateInputField::YoudaoAppId,
                        cx,
                    ))
                    .child(self.translation_input(
                        "translation-youdao-app-key",
                        "Youdao App Key",
                        youdao_key_value,
                        TranslateInputField::YoudaoAppKey,
                        cx,
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(small_button(
                        "translation-clear-deepl",
                        "Clear DeepL",
                        cx.listener(|this, _, _, cx| {
                            this.clear_translation_secret("deepl", cx);
                        }),
                    ))
                    .child(small_button(
                        "translation-clear-baidu",
                        "Clear Baidu",
                        cx.listener(|this, _, _, cx| {
                            this.clear_translation_secret("baidu", cx);
                        }),
                    ))
                    .child(small_button(
                        "translation-clear-ali",
                        "Clear Ali",
                        cx.listener(|this, _, _, cx| {
                            this.clear_translation_secret("ali", cx);
                        }),
                    ))
                    .child(small_button(
                        "translation-clear-youdao",
                        "Clear Youdao",
                        cx.listener(|this, _, _, cx| {
                            this.clear_translation_secret("youdao", cx);
                        }),
                    )),
            )
    }

    fn connections_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut list = div().flex().flex_col().gap_2();
        if self.connections.is_empty() {
            list = list.child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .text_color(rgb(0xaeb7c8))
                    .child("No saved connections were found in the native runtime directory yet."),
            );
        } else {
            for connection in &self.connections {
                let connection_for_click = connection.clone();
                list = list.child(connection_row(
                    connection,
                    cx.listener(move |this, _, window, cx| {
                        this.start_saved_connection(connection_for_click.clone(), window, cx);
                    }),
                ));
            }
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_5()
            .gap_4()
            .child(section_header(
                "Connections",
                "Compatible with the saved connection schema from the Tauri app.",
            ))
            .child(list)
    }

    fn tunnels_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let open_tunnels = self
            .tunnel_manager
            .list()
            .unwrap_or_default()
            .into_iter()
            .map(|info| (info.id.clone(), info))
            .collect::<HashMap<_, _>>();
        let mut list = div().flex().flex_col().gap_2();
        if self.tunnels.is_empty() {
            list = list.child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .text_color(rgb(0xaeb7c8))
                    .child("No saved tunnels were found in the native runtime directory yet."),
            );
        } else {
            for tunnel in self.tunnels.clone() {
                let tunnel_for_open = tunnel.clone();
                let tunnel_id_for_close = tunnel.id.clone();
                let open_info = open_tunnels.get(&tunnel.id).cloned();
                let pending = self.pending_tunnels.iter().any(|id| id == &tunnel.id);
                let connection_label = tunnel
                    .connection_id
                    .as_deref()
                    .and_then(|id| {
                        self.connections
                            .iter()
                            .find(|connection| connection.id == id)
                            .map(|connection| connection.name.clone())
                    })
                    .unwrap_or_else(|| "No connection".to_string());
                list = list.child(tunnel_row(
                    &tunnel,
                    connection_label,
                    open_info,
                    pending,
                    cx.listener(move |this, _, window, cx| {
                        this.start_tunnel_job(tunnel_for_open.clone(), window, cx);
                    }),
                    cx.listener(move |this, _, _, cx| {
                        this.close_tunnel_job(tunnel_id_for_close.clone(), cx);
                    }),
                ));
            }
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_5()
            .gap_4()
            .child(section_header(
                "Tunnels",
                "Legacy tunnel profiles backed by native SSH forwarding services.",
            ))
            .child(
                div()
                    .grid()
                    .grid_cols(3)
                    .gap_3()
                    .child(metric("Profiles", self.tunnels.len().to_string()))
                    .child(metric("Open", open_tunnels.len().to_string()))
                    .child(metric("Pending", self.pending_tunnels.len().to_string())),
            )
            .child(list)
    }

    fn settings_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot_prompt = self.active_snapshot_password_prompt.clone();
        let backup_snapshot_prompt = snapshot_prompt
            .as_ref()
            .filter(|prompt| {
                matches!(
                    prompt.kind,
                    SnapshotPasswordPromptKind::Export | SnapshotPasswordPromptKind::Import
                )
            })
            .cloned();
        let cloud_snapshot_prompt = snapshot_prompt
            .as_ref()
            .filter(|prompt| {
                matches!(
                    prompt.kind,
                    SnapshotPasswordPromptKind::CloudPush
                        | SnapshotPasswordPromptKind::CloudPull
                        | SnapshotPasswordPromptKind::CloudForcePush
                        | SnapshotPasswordPromptKind::CloudForcePull
                        | SnapshotPasswordPromptKind::CloudProviderPush
                        | SnapshotPasswordPromptKind::CloudProviderPull
                        | SnapshotPasswordPromptKind::CloudProviderForcePush
                        | SnapshotPasswordPromptKind::CloudProviderForcePull
                )
            })
            .cloned();
        let cloud_remote_path = self
            .runtime
            .config_dir()
            .join("cloud-sync-local")
            .join(&self.cloud_sync_settings.remote_root)
            .display()
            .to_string();
        let cloud_provider_label = format!("local / {}", self.cloud_sync_settings.provider);
        let cloud_last_revision = self
            .cloud_sync_state
            .last_applied_remote_revision
            .as_deref()
            .map(compact_id)
            .unwrap_or_else(|| "none".to_string());
        let cloud_last_hash = self
            .cloud_sync_state
            .last_synced_payload_hash
            .as_deref()
            .map(compact_id)
            .unwrap_or_else(|| "none".to_string());
        let cloud_history_empty = self.cloud_sync_history.is_empty();
        let cloud_history_0 = self.cloud_sync_history.first().cloned();
        let cloud_history_1 = self.cloud_sync_history.get(1).cloned();
        let cloud_history_2 = self.cloud_sync_history.get(2).cloned();
        let cloud_conflict = self.cloud_sync_conflict.clone();
        let active_cloud_provider = configured_cloud_sync_provider(&self.cloud_sync_settings);
        let webdav_password_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.webdav_password,
            &self.cloud_sync_settings.webdav.password,
        );
        let s3_access_key_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.s3_access_key_id,
            &self.cloud_sync_settings.s3.access_key_id,
        );
        let s3_secret_key_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.s3_secret_access_key,
            &self.cloud_sync_settings.s3.secret_access_key,
        );
        let s3_session_token_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.s3_session_token,
            &self.cloud_sync_settings.s3.session_token,
        );
        let google_drive_access_token_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.google_drive_access_token,
            &self.cloud_sync_settings.google_drive.access_token,
        );
        let google_drive_refresh_token_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.google_drive_refresh_token,
            &self.cloud_sync_settings.google_drive.refresh_token,
        );
        let google_drive_client_secret_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.google_drive_client_secret,
            &self.cloud_sync_settings.google_drive.client_secret,
        );
        let onedrive_access_token_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.onedrive_access_token,
            &self.cloud_sync_settings.onedrive.access_token,
        );
        let onedrive_refresh_token_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.onedrive_refresh_token,
            &self.cloud_sync_settings.onedrive.refresh_token,
        );
        let onedrive_client_secret_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.onedrive_client_secret,
            &self.cloud_sync_settings.onedrive.client_secret,
        );
        let aliyun_drive_access_token_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.aliyun_drive_access_token,
            &self.cloud_sync_settings.aliyun_drive.access_token,
        );
        let aliyun_drive_refresh_token_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.aliyun_drive_refresh_token,
            &self.cloud_sync_settings.aliyun_drive.refresh_token,
        );
        let aliyun_drive_client_secret_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.aliyun_drive_client_secret,
            &self.cloud_sync_settings.aliyun_drive.client_secret,
        );
        let gitee_token_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.gitee_token,
            &self.cloud_sync_settings.gitee_snippet.access_token,
        );
        let github_token_value = cloud_secret_display(
            &self.cloud_sync_secret_draft.github_token,
            &self.cloud_sync_settings.github_gist.access_token,
        );
        let active_ai_profile_id = self.ai_settings.active_profile_id.clone();
        let active_ai_api_key = ai_active_profile_api_key(&self.ai_settings);
        let ai_key_value = cloud_secret_display(&self.ai_secret_draft, &active_ai_api_key);
        let enabled_ai_models = self
            .ai_settings
            .models
            .iter()
            .filter(|model| model.enabled)
            .count();
        let ai_default_model = self
            .ai_settings
            .default_model_id
            .as_deref()
            .map(compact_id)
            .unwrap_or_else(|| "none".to_string());
        let ai_discovery_label = if self.ai_discovery_pending {
            "Pending"
        } else {
            "Discover"
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_5()
            .gap_4()
            .child(section_header(
                "Settings",
                "Native settings backed by the legacy-compatible redb document.",
            ))
            .child(
                div()
                    .grid()
                    .grid_cols(3)
                    .gap_3()
                    .child(metric("Theme", self.settings.theme.clone()))
                    .child(metric("Language", self.settings.language.clone()))
                    .child(metric(
                        "Terminal Font",
                        format!(
                            "{} {}",
                            self.settings.terminal_font_family, self.settings.terminal_font_size
                        ),
                    ))
                    .child(metric(
                        "Transfer Policy",
                        self.settings.transfer_duplicate_strategy.clone(),
                    ))
                    .child(metric(
                        "X11 Display",
                        if self.settings.x11_display.trim().is_empty() {
                            "auto".to_string()
                        } else {
                            self.settings.x11_display.clone()
                        },
                    )),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(div().text_sm().font_weight(FontWeight(700.)).child("AI"))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x98a3b8))
                                    .child(self.ai_status.clone()),
                            ),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(6)
                            .gap_3()
                            .child(metric(
                                "State",
                                if self.ai_settings.enabled {
                                    "enabled".to_string()
                                } else {
                                    "disabled".to_string()
                                },
                            ))
                            .child(metric("Models", enabled_ai_models.to_string()))
                            .child(metric("Default", ai_default_model))
                            .child(metric("Sessions", self.ai_session_count.to_string()))
                            .child(metric("Messages", self.ai_message_count.to_string()))
                            .child(metric("Audit", self.ai_audit_count.to_string())),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(policy_button(
                                "ai-provider-openai",
                                "OpenAI",
                                active_ai_profile_id == "openai",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("openai", cx);
                                }),
                            ))
                            .child(policy_button(
                                "ai-provider-anthropic",
                                "Anthropic",
                                active_ai_profile_id == "anthropic",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("anthropic", cx);
                                }),
                            ))
                            .child(policy_button(
                                "ai-provider-gemini",
                                "Gemini",
                                active_ai_profile_id == "gemini",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("gemini", cx);
                                }),
                            ))
                            .child(policy_button(
                                "ai-provider-deepseek",
                                "DeepSeek",
                                active_ai_profile_id == "deepseek",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("deepseek", cx);
                                }),
                            ))
                            .child(policy_button(
                                "ai-provider-ollama",
                                "Ollama",
                                active_ai_profile_id == "ollama",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("ollama", cx);
                                }),
                            ))
                            .child(policy_button(
                                "ai-provider-xai",
                                "xAI",
                                active_ai_profile_id == "xai",
                                cx.listener(|this, _, _, cx| {
                                    this.update_ai_profile("xai", cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .mt_2()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(policy_button(
                                "ai-mode-ask",
                                "Ask",
                                self.ai_settings.default_mode == AiMode::Ask,
                                cx.listener(|this, _, _, cx| {
                                    this.set_ai_mode(AiMode::Ask, cx);
                                }),
                            ))
                            .child(policy_button(
                                "ai-mode-agent",
                                "Agent",
                                self.ai_settings.default_mode == AiMode::Agent,
                                cx.listener(|this, _, _, cx| {
                                    this.set_ai_mode(AiMode::Agent, cx);
                                }),
                            ))
                            .child(policy_button(
                                "ai-command-confirm",
                                "Confirm",
                                self.ai_settings.agent_command_execution_mode
                                    == AgentCommandExecutionMode::ConfirmEach,
                                cx.listener(|this, _, _, cx| {
                                    this.set_ai_command_mode(
                                        AgentCommandExecutionMode::ConfirmEach,
                                        cx,
                                    );
                                }),
                            ))
                            .child(policy_button(
                                "ai-command-smart",
                                "Smart",
                                self.ai_settings.agent_command_execution_mode
                                    == AgentCommandExecutionMode::Smart,
                                cx.listener(|this, _, _, cx| {
                                    this.set_ai_command_mode(AgentCommandExecutionMode::Smart, cx);
                                }),
                            ))
                            .child(policy_button(
                                "ai-command-auto",
                                "Auto",
                                self.ai_settings.agent_command_execution_mode
                                    == AgentCommandExecutionMode::Auto,
                                cx.listener(|this, _, _, cx| {
                                    this.set_ai_command_mode(AgentCommandExecutionMode::Auto, cx);
                                }),
                            ))
                            .child(policy_button(
                                "ai-agent-bg-exec",
                                "BG Exec",
                                self.ai_settings.agent_background_execution_enabled,
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_ai_background_execution(cx);
                                }),
                            ))
                            .child(small_button(
                                "ai-enabled",
                                if self.ai_settings.enabled {
                                    "Enabled"
                                } else {
                                    "Disabled"
                                },
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_ai_enabled(cx);
                                }),
                            ))
                            .child(small_button(
                                "ai-save",
                                "Save",
                                cx.listener(|this, _, _, cx| {
                                    this.save_ai_settings(cx);
                                }),
                            ))
                            .child(small_button(
                                "ai-discover",
                                ai_discovery_label,
                                cx.listener(|this, _, _, cx| {
                                    this.discover_ai_models(cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(3)
                            .gap_2()
                            .child(self.ai_input(
                                "ai-model",
                                "Model",
                                self.ai_model_draft.clone(),
                                AiInputField::Model,
                                cx,
                            ))
                            .child(self.ai_input(
                                "ai-base-url",
                                "Base URL",
                                self.ai_base_url_draft.clone(),
                                AiInputField::BaseUrl,
                                cx,
                            ))
                            .child(self.ai_input(
                                "ai-api-key",
                                "API Key",
                                ai_key_value,
                                AiInputField::ApiKey,
                                cx,
                            )),
                    ),
            )
            .child(self.translation_settings_section(cx))
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Keyword Highlights"),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(3)
                            .gap_3()
                            .child(metric(
                                "State",
                                if self.keyword_highlights.enabled {
                                    "enabled".to_string()
                                } else {
                                    "disabled".to_string()
                                },
                            ))
                            .child(metric(
                                "Rules",
                                self.keyword_highlights.rules.len().to_string(),
                            ))
                            .child(metric(
                                "Active",
                                self.keyword_highlights
                                    .rules
                                    .iter()
                                    .filter(|rule| rule.enabled)
                                    .count()
                                    .to_string(),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                "settings-keyword-highlights-enabled",
                                if self.keyword_highlights.enabled {
                                    "Enabled"
                                } else {
                                    "Disabled"
                                },
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_keyword_highlights(cx);
                                }),
                            ))
                            .child(small_button(
                                "settings-keyword-highlights-wrap",
                                if self.keyword_highlights.across_wrapped_lines {
                                    "Wrap On"
                                } else {
                                    "Wrap Off"
                                },
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_keyword_highlights_wrapped(cx);
                                }),
                            ))
                            .child(small_button(
                                "settings-keyword-highlights-import",
                                "Import",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_keyword_highlight_import(cx);
                                }),
                            ))
                            .child(div().text_xs().text_color(rgb(0x98a3b8)).child(
                                match self.keyword_highlight_path_prompt {
                                    Some(KeywordHighlightPathPromptKind::Import) => {
                                        "selecting import file"
                                    }
                                    None => "legacy JSON import",
                                },
                            )),
                    ),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("SSH Host Keys"),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(policy_button(
                                "settings-host-key-prompt",
                                "Prompt",
                                self.settings.host_key_policy == "prompt",
                                cx.listener(|this, _, _, cx| {
                                    this.update_host_key_policy("prompt", cx);
                                }),
                            ))
                            .child(policy_button(
                                "settings-host-key-strict",
                                "Strict",
                                self.settings.host_key_policy == "strict",
                                cx.listener(|this, _, _, cx| {
                                    this.update_host_key_policy("strict", cx);
                                }),
                            ))
                            .child(policy_button(
                                "settings-host-key-accept",
                                "Accept",
                                self.settings.host_key_policy == "accept",
                                cx.listener(|this, _, _, cx| {
                                    this.update_host_key_policy("accept", cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .text_xs()
                            .text_color(rgb(0x98a3b8))
                            .child(format!("Current policy: {}", self.settings.host_key_policy)),
                    ),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Recording"),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(3)
                            .gap_3()
                            .child(metric(
                                "Path",
                                if self.settings.recording_path.trim().is_empty() {
                                    self.runtime
                                        .config_dir()
                                        .join("recordings")
                                        .display()
                                        .to_string()
                                } else {
                                    self.settings.recording_path.clone()
                                },
                            ))
                            .child(metric(
                                "Memory",
                                format!(
                                    "{} MiB",
                                    (self.settings.recording_memory_limit_bytes / (1024 * 1024))
                                        .max(1)
                                ),
                            ))
                            .child(metric(
                                "Format",
                                format!(
                                    "{} / {}",
                                    if self.settings.recording_include_io_labels {
                                        "labels"
                                    } else {
                                        "plain"
                                    },
                                    if self.settings.recording_include_timestamps {
                                        "timestamps"
                                    } else {
                                        "no time"
                                    }
                                ),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                "settings-recording-auto",
                                if self.settings.recording_auto_start {
                                    "Auto On"
                                } else {
                                    "Auto Off"
                                },
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_recording_auto_start(cx);
                                }),
                            ))
                            .child(small_button(
                                "settings-recording-labels",
                                if self.settings.recording_include_io_labels {
                                    "Labels On"
                                } else {
                                    "Labels Off"
                                },
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_recording_io_labels(cx);
                                }),
                            ))
                            .child(small_button(
                                "settings-recording-timestamps",
                                if self.settings.recording_include_timestamps {
                                    "Time On"
                                } else {
                                    "Time Off"
                                },
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_recording_timestamps(cx);
                                }),
                            ))
                            .child(small_button(
                                "settings-recording-memory-minus",
                                "-1 MiB",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_recording_memory_limit(-1, cx);
                                }),
                            ))
                            .child(small_button(
                                "settings-recording-memory-plus",
                                "+1 MiB",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_recording_memory_limit(1, cx);
                                }),
                            )),
                    ),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Config Backup"),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                "settings-config-export",
                                "Export",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_config_export(cx);
                                }),
                            ))
                            .child(small_button(
                                "settings-config-import",
                                "Import",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_config_import(cx);
                                }),
                            ))
                            .child(div().text_xs().text_color(rgb(0x98a3b8)).child(
                                match self.config_path_prompt {
                                    Some(ConfigPathPromptKind::Export) => "selecting export path",
                                    Some(ConfigPathPromptKind::Import) => "selecting import path",
                                    Some(ConfigPathPromptKind::PortableExport) => {
                                        "selecting .nya export path"
                                    }
                                    Some(ConfigPathPromptKind::PortableImport) => {
                                        "selecting .nya import path"
                                    }
                                    Some(ConfigPathPromptKind::EncryptedPortableExport) => {
                                        "selecting encrypted .nya export path"
                                    }
                                    Some(ConfigPathPromptKind::EncryptedPortableImport) => {
                                        "selecting encrypted .nya import path"
                                    }
                                    None => "native redb backup",
                                },
                            )),
                    )
                    .child(
                        div()
                            .mt_2()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                "settings-portable-export",
                                "Export .nya",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_portable_snapshot_export(cx);
                                }),
                            ))
                            .child(small_button(
                                "settings-portable-import",
                                "Import .nya",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_portable_snapshot_import(cx);
                                }),
                            ))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x98a3b8))
                                    .child("legacy portable snapshot"),
                            ),
                    )
                    .child(
                        div()
                            .mt_2()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                "settings-encrypted-portable-export",
                                "Encrypt .nya",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_encrypted_portable_snapshot_export(cx);
                                }),
                            ))
                            .child(small_button(
                                "settings-encrypted-portable-import",
                                "Decrypt .nya",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_encrypted_portable_snapshot_import(cx);
                                }),
                            ))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x98a3b8))
                                    .child("AES-GCM master password"),
                            ),
                    )
                    .when_some(backup_snapshot_prompt, |this, prompt| {
                        this.child(self.snapshot_password_prompt_banner(prompt, cx))
                    })
                    .child(
                        div()
                            .mt_3()
                            .text_xs()
                            .text_color(rgb(0x98a3b8))
                            .child(self.store_status.path.clone()),
                    ),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("Cloud Sync"),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x98a3b8))
                                    .child(self.cloud_sync_status.clone()),
                            ),
                    )
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(3)
                            .gap_2()
                            .child(compact_setting_state("Provider", cloud_provider_label))
                            .child(compact_setting_state("Revision", cloud_last_revision))
                            .child(compact_setting_state("Hash", cloud_last_hash)),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(policy_button(
                                "cloud-provider-local",
                                "Local",
                                active_cloud_provider == "local_directory",
                                cx.listener(|this, _, _, cx| {
                                    this.update_cloud_sync_provider("local_directory", cx);
                                }),
                            ))
                            .child(policy_button(
                                "cloud-provider-webdav",
                                "WebDAV",
                                active_cloud_provider == "webdav",
                                cx.listener(|this, _, _, cx| {
                                    this.update_cloud_sync_provider("webdav", cx);
                                }),
                            ))
                            .child(policy_button(
                                "cloud-provider-s3",
                                "S3",
                                active_cloud_provider == "s3",
                                cx.listener(|this, _, _, cx| {
                                    this.update_cloud_sync_provider("s3", cx);
                                }),
                            ))
                            .child(policy_button(
                                "cloud-provider-google-drive",
                                "Drive",
                                active_cloud_provider == "google_drive",
                                cx.listener(|this, _, _, cx| {
                                    this.update_cloud_sync_provider("google_drive", cx);
                                }),
                            ))
                            .child(policy_button(
                                "cloud-provider-onedrive",
                                "OneDrive",
                                active_cloud_provider == "onedrive",
                                cx.listener(|this, _, _, cx| {
                                    this.update_cloud_sync_provider("onedrive", cx);
                                }),
                            ))
                            .child(policy_button(
                                "cloud-provider-aliyun-drive",
                                "Aliyun",
                                active_cloud_provider == "aliyun_drive",
                                cx.listener(|this, _, _, cx| {
                                    this.update_cloud_sync_provider("aliyun_drive", cx);
                                }),
                            ))
                            .child(policy_button(
                                "cloud-provider-gitee",
                                "Gitee",
                                active_cloud_provider == "gitee_snippet",
                                cx.listener(|this, _, _, cx| {
                                    this.update_cloud_sync_provider("gitee_snippet", cx);
                                }),
                            ))
                            .child(policy_button(
                                "cloud-provider-github",
                                "GitHub",
                                active_cloud_provider == "github_gist",
                                cx.listener(|this, _, _, cx| {
                                    this.update_cloud_sync_provider("github_gist", cx);
                                }),
                            ))
                            .child(small_button(
                                "cloud-sync-enabled",
                                if self.cloud_sync_settings.enabled {
                                    "Enabled"
                                } else {
                                    "Disabled"
                                },
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_cloud_sync_enabled(cx);
                                }),
                            ))
                            .child(small_button(
                                "cloud-sync-save",
                                "Save",
                                cx.listener(|this, _, _, cx| {
                                    this.save_cloud_sync_settings(cx);
                                }),
                            )),
                    )
                    .child(div().mt_3().child(self.cloud_sync_input(
                        "cloud-sync-remote-root",
                        "Remote Root",
                        self.cloud_sync_settings.remote_root.clone(),
                        CloudSyncInputField::RemoteRoot,
                        cx,
                    )))
                    .when(active_cloud_provider == "webdav", |this| {
                        this.child(
                            div()
                                .mt_2()
                                .grid()
                                .grid_cols(2)
                                .gap_2()
                                .child(self.cloud_sync_input(
                                    "cloud-webdav-endpoint",
                                    "Endpoint",
                                    self.cloud_sync_settings.webdav.endpoint.clone(),
                                    CloudSyncInputField::WebdavEndpoint,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-webdav-root",
                                    "Root",
                                    self.cloud_sync_settings.webdav.root.clone(),
                                    CloudSyncInputField::WebdavRoot,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-webdav-username",
                                    "Username",
                                    self.cloud_sync_settings.webdav.username.clone(),
                                    CloudSyncInputField::WebdavUsername,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-webdav-password",
                                    "Password",
                                    webdav_password_value,
                                    CloudSyncInputField::WebdavPassword,
                                    cx,
                                )),
                        )
                    })
                    .when(active_cloud_provider == "s3", |this| {
                        this.child(
                            div()
                                .mt_2()
                                .grid()
                                .grid_cols(3)
                                .gap_2()
                                .child(self.cloud_sync_input(
                                    "cloud-s3-endpoint",
                                    "Endpoint",
                                    self.cloud_sync_settings.s3.endpoint.clone(),
                                    CloudSyncInputField::S3Endpoint,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-s3-bucket",
                                    "Bucket",
                                    self.cloud_sync_settings.s3.bucket.clone(),
                                    CloudSyncInputField::S3Bucket,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-s3-region",
                                    "Region",
                                    self.cloud_sync_settings.s3.region.clone(),
                                    CloudSyncInputField::S3Region,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-s3-root",
                                    "S3 Root",
                                    self.cloud_sync_settings.s3.root.clone(),
                                    CloudSyncInputField::S3Root,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-s3-access-key",
                                    "Access Key",
                                    s3_access_key_value,
                                    CloudSyncInputField::S3AccessKeyId,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-s3-secret-key",
                                    "Secret Key",
                                    s3_secret_key_value,
                                    CloudSyncInputField::S3SecretAccessKey,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-s3-session-token",
                                    "Session Token",
                                    s3_session_token_value,
                                    CloudSyncInputField::S3SessionToken,
                                    cx,
                                ))
                                .child(small_button(
                                    "cloud-s3-url-style",
                                    if self.cloud_sync_settings.s3.virtual_host_style {
                                        "Virtual Host"
                                    } else {
                                        "Path Style"
                                    },
                                    cx.listener(|this, _, _, cx| {
                                        this.toggle_s3_virtual_host_style(cx);
                                    }),
                                )),
                        )
                    })
                    .when(active_cloud_provider == "google_drive", |this| {
                        this.child(
                            div()
                                .mt_2()
                                .grid()
                                .grid_cols(3)
                                .gap_2()
                                .child(self.cloud_sync_input(
                                    "cloud-google-drive-root",
                                    "Drive Root",
                                    self.cloud_sync_settings.google_drive.root.clone(),
                                    CloudSyncInputField::GoogleDriveRoot,
                                    cx,
                                ))
                                .child(
                                    self.cloud_sync_input(
                                        "cloud-google-drive-client-id",
                                        "Client ID",
                                        self.cloud_sync_settings
                                            .google_drive
                                            .client_id
                                            .clone()
                                            .unwrap_or_default(),
                                        CloudSyncInputField::GoogleDriveClientId,
                                        cx,
                                    ),
                                )
                                .child(self.cloud_sync_input(
                                    "cloud-google-drive-client-secret",
                                    "Client Secret",
                                    google_drive_client_secret_value,
                                    CloudSyncInputField::GoogleDriveClientSecret,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-google-drive-access-token",
                                    "Access Token",
                                    google_drive_access_token_value,
                                    CloudSyncInputField::GoogleDriveAccessToken,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-google-drive-refresh-token",
                                    "Refresh Token",
                                    google_drive_refresh_token_value,
                                    CloudSyncInputField::GoogleDriveRefreshToken,
                                    cx,
                                )),
                        )
                    })
                    .when(active_cloud_provider == "onedrive", |this| {
                        this.child(
                            div()
                                .mt_2()
                                .grid()
                                .grid_cols(3)
                                .gap_2()
                                .child(self.cloud_sync_input(
                                    "cloud-onedrive-root",
                                    "OneDrive Root",
                                    self.cloud_sync_settings.onedrive.root.clone(),
                                    CloudSyncInputField::OneDriveRoot,
                                    cx,
                                ))
                                .child(
                                    self.cloud_sync_input(
                                        "cloud-onedrive-client-id",
                                        "Client ID",
                                        self.cloud_sync_settings
                                            .onedrive
                                            .client_id
                                            .clone()
                                            .unwrap_or_default(),
                                        CloudSyncInputField::OneDriveClientId,
                                        cx,
                                    ),
                                )
                                .child(self.cloud_sync_input(
                                    "cloud-onedrive-client-secret",
                                    "Client Secret",
                                    onedrive_client_secret_value,
                                    CloudSyncInputField::OneDriveClientSecret,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-onedrive-access-token",
                                    "Access Token",
                                    onedrive_access_token_value,
                                    CloudSyncInputField::OneDriveAccessToken,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-onedrive-refresh-token",
                                    "Refresh Token",
                                    onedrive_refresh_token_value,
                                    CloudSyncInputField::OneDriveRefreshToken,
                                    cx,
                                )),
                        )
                    })
                    .when(active_cloud_provider == "aliyun_drive", |this| {
                        this.child(
                            div()
                                .mt_2()
                                .grid()
                                .grid_cols(3)
                                .gap_2()
                                .child(self.cloud_sync_input(
                                    "cloud-aliyun-drive-root",
                                    "Aliyun Root",
                                    self.cloud_sync_settings.aliyun_drive.root.clone(),
                                    CloudSyncInputField::AliyunDriveRoot,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-aliyun-drive-type",
                                    "Drive Type",
                                    self.cloud_sync_settings.aliyun_drive.drive_type.clone(),
                                    CloudSyncInputField::AliyunDriveType,
                                    cx,
                                ))
                                .child(
                                    self.cloud_sync_input(
                                        "cloud-aliyun-drive-client-id",
                                        "Client ID",
                                        self.cloud_sync_settings
                                            .aliyun_drive
                                            .client_id
                                            .clone()
                                            .unwrap_or_default(),
                                        CloudSyncInputField::AliyunDriveClientId,
                                        cx,
                                    ),
                                )
                                .child(self.cloud_sync_input(
                                    "cloud-aliyun-drive-client-secret",
                                    "Client Secret",
                                    aliyun_drive_client_secret_value,
                                    CloudSyncInputField::AliyunDriveClientSecret,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-aliyun-drive-access-token",
                                    "Access Token",
                                    aliyun_drive_access_token_value,
                                    CloudSyncInputField::AliyunDriveAccessToken,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-aliyun-drive-refresh-token",
                                    "Refresh Token",
                                    aliyun_drive_refresh_token_value,
                                    CloudSyncInputField::AliyunDriveRefreshToken,
                                    cx,
                                )),
                        )
                    })
                    .when(active_cloud_provider == "gitee_snippet", |this| {
                        this.child(
                            div()
                                .mt_2()
                                .grid()
                                .grid_cols(3)
                                .gap_2()
                                .child(self.cloud_sync_input(
                                    "cloud-gitee-endpoint",
                                    "API Endpoint",
                                    self.cloud_sync_settings.gitee_snippet.api_endpoint.clone(),
                                    CloudSyncInputField::GiteeEndpoint,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-gitee-gist",
                                    "Snippet ID",
                                    self.cloud_sync_settings.gitee_snippet.gist_id.clone(),
                                    CloudSyncInputField::GiteeGistId,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-gitee-token",
                                    "Token",
                                    gitee_token_value,
                                    CloudSyncInputField::GiteeToken,
                                    cx,
                                )),
                        )
                    })
                    .when(active_cloud_provider == "github_gist", |this| {
                        this.child(
                            div()
                                .mt_2()
                                .grid()
                                .grid_cols(2)
                                .gap_2()
                                .child(self.cloud_sync_input(
                                    "cloud-github-gist",
                                    "Gist ID",
                                    self.cloud_sync_settings.github_gist.gist_id.clone(),
                                    CloudSyncInputField::GithubGistId,
                                    cx,
                                ))
                                .child(self.cloud_sync_input(
                                    "cloud-github-token",
                                    "Token",
                                    github_token_value,
                                    CloudSyncInputField::GithubToken,
                                    cx,
                                )),
                        )
                    })
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                "settings-cloud-sync-push",
                                "Push Local",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_local_cloud_sync_push(cx);
                                }),
                            ))
                            .child(small_button(
                                "settings-cloud-sync-pull",
                                "Pull Local",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_local_cloud_sync_pull(cx);
                                }),
                            ))
                            .child(div().text_xs().text_color(rgb(0x98a3b8)).child(format!(
                                "device {}",
                                compact_id(&self.cloud_sync_state.device_id)
                            ))),
                    )
                    .child(
                        div()
                            .mt_2()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                "settings-provider-cloud-sync-push",
                                "Push Provider",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_provider_cloud_sync_push(cx);
                                }),
                            ))
                            .child(small_button(
                                "settings-provider-cloud-sync-pull",
                                "Pull Provider",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_provider_cloud_sync_pull(cx);
                                }),
                            ))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x98a3b8))
                                    .child(self.cloud_sync_settings.provider.clone()),
                            ),
                    )
                    .when_some(cloud_conflict, |this, conflict| {
                        this.child(self.cloud_sync_conflict_banner(conflict, cx))
                    })
                    .when_some(cloud_snapshot_prompt, |this, prompt| {
                        this.child(self.snapshot_password_prompt_banner(prompt, cx))
                    })
                    .child(
                        div()
                            .mt_3()
                            .text_xs()
                            .text_color(rgb(0x98a3b8))
                            .child(cloud_remote_path),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight(700.))
                                    .text_color(rgb(0xcbd5e1))
                                    .child("Recent History"),
                            )
                            .when(cloud_history_empty, |this| {
                                this.child(
                                    div()
                                        .rounded_sm()
                                        .border_1()
                                        .border_color(rgb(0x273244))
                                        .bg(rgb(0x111722))
                                        .p_3()
                                        .text_xs()
                                        .text_color(rgb(0x98a3b8))
                                        .child("No sync runs recorded"),
                                )
                            })
                            .when_some(cloud_history_0, |this, entry| {
                                this.child(cloud_sync_history_row(entry))
                            })
                            .when_some(cloud_history_1, |this, entry| {
                                this.child(cloud_sync_history_row(entry))
                            })
                            .when_some(cloud_history_2, |this, entry| {
                                this.child(cloud_sync_history_row(entry))
                            }),
                    ),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Diagnostics / Updates"),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                "settings-diagnostics-export",
                                "Export",
                                cx.listener(|this, _, _, cx| {
                                    this.prompt_diagnostics_export(cx);
                                }),
                            ))
                            .child(small_button(
                                "settings-diagnostics-logs",
                                "Logs",
                                cx.listener(|this, _, _, cx| {
                                    this.reveal_log_dir(cx);
                                }),
                            ))
                            .child(div().text_xs().text_color(rgb(0x98a3b8)).child(
                                match self.diagnostics_path_prompt {
                                    Some(DiagnosticsPathPromptKind::Export) => {
                                        "selecting export path"
                                    }
                                    None => "native diagnostics",
                                },
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .border_t_1()
                            .border_color(rgb(0x2a3140))
                            .pt_3()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight(700.))
                                            .text_color(rgb(0xe5edf7))
                                            .child("Native Update Check"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(0x98a3b8))
                                            .line_height(px(18.))
                                            .child(truncate_preview(&self.update_status, 120)),
                                    ),
                            )
                            .child(small_button(
                                "settings-update-check",
                                if self.update_pending {
                                    "Checking"
                                } else {
                                    "Check"
                                },
                                cx.listener(|this, _, _, cx| {
                                    this.start_update_check(cx);
                                }),
                            )),
                    )
                    .when_some(self.update_info.clone(), |this, info| {
                        let release_url = info.html_url.clone().unwrap_or_else(|| {
                            "https://github.com/nyakang/nyaterm/releases".to_string()
                        });
                        let notes = info.release_notes.unwrap_or_default();
                        this.child(
                            div()
                                .mt_2()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .text_xs()
                                .text_color(rgb(0x98a3b8))
                                .child(format!(
                                    "Latest: {}{}",
                                    info.latest_version,
                                    info.release_date
                                        .as_deref()
                                        .map(|date| format!(" · {date}"))
                                        .unwrap_or_default()
                                ))
                                .child(release_url)
                                .when(!notes.trim().is_empty(), |this| {
                                    this.child(truncate_preview(&notes, 180))
                                }),
                        )
                    })
                    .child(
                        div()
                            .mt_3()
                            .text_xs()
                            .text_color(rgb(0x98a3b8))
                            .child(self.runtime.log_dir().display().to_string()),
                    ),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_3()
                    .child(setting_state(
                        "Startup Restore",
                        if self.settings.startup_restore {
                            "enabled"
                        } else {
                            "disabled"
                        },
                    ))
                    .child(setting_state(
                        "Confirm On Close",
                        if self.settings.confirm_on_close {
                            "enabled"
                        } else {
                            "disabled"
                        },
                    )),
            )
    }

    fn migration_view(&self) -> impl IntoElement {
        let mut capabilities = div().flex().flex_col().gap_2();
        for capability in self.services.capabilities() {
            capabilities = capabilities.child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_4()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_3()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child(capability.area),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x98a3b8))
                                    .child(capability.note),
                            ),
                    )
                    .child(service_status(capability.status)),
            );
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_5()
            .gap_4()
            .child(section_header(
                "Migration",
                "Inventory of the ignored Tauri source and the native replacement boundary.",
            ))
            .child(
                div()
                    .grid()
                    .grid_cols(4)
                    .gap_3()
                    .child(metric(
                        "Legacy source",
                        if self.inventory.exists {
                            "found"
                        } else {
                            "missing"
                        },
                    ))
                    .child(metric("Rust files", self.inventory.rust_files.to_string()))
                    .child(metric(
                        "Frontend files",
                        self.inventory.frontend_files.to_string(),
                    ))
                    .child(metric(
                        "Command modules",
                        self.inventory.command_modules.to_string(),
                    )),
            )
            .child(capabilities)
    }

    fn ai_ask_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let agent_mode = self.ai_settings.default_mode == AiMode::Agent;
        let ai_running = self.ai_chat_pending || self.ai_agent_loop.is_some();
        let action_label = if ai_running {
            "Cancel"
        } else if agent_mode {
            "Agent"
        } else {
            "Ask"
        };
        let mut command_rows = div().mt_3().flex().flex_col().gap_2();
        for (index, card) in self.ai_command_cards.iter().cloned().take(3).enumerate() {
            let risk = risk_label(card.risk_level.as_ref());
            let title = if card.title.trim().is_empty() {
                "Command".to_string()
            } else {
                card.title.clone()
            };
            command_rows = command_rows.child(
                div()
                    .border_t_1()
                    .border_color(rgb(0x2a3140))
                    .pt_2()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .min_w_0()
                                    .text_xs()
                                    .font_weight(FontWeight(700.))
                                    .text_color(rgb(0xe5edf7))
                                    .child(title),
                            )
                            .child(status_pill(risk, rgb(0xfacc15), rgb(0x3a2f14))),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0xaeb7c8))
                            .line_height(px(18.))
                            .child(truncate_preview(&card.command, 120)),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x64748b))
                                    .child(truncate_preview(&card.explanation, 80)),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(small_button(
                                        format!("ai-command-save-{index}"),
                                        "Save",
                                        cx.listener(move |this, _, _, cx| {
                                            this.save_ai_command_card(index, cx);
                                        }),
                                    ))
                                    .child(small_button(
                                        format!("ai-command-insert-{index}"),
                                        "Insert",
                                        cx.listener(move |this, _, _, cx| {
                                            this.insert_ai_command_card(index, cx);
                                        }),
                                    ))
                                    .child(small_button(
                                        format!("ai-command-run-{index}"),
                                        "Run",
                                        cx.listener(move |this, _, _, cx| {
                                            this.run_ai_command_card(index, cx);
                                        }),
                                    )),
                            ),
                    ),
            );
        }
        let mut agent_step_rows = div();
        if agent_mode || !self.ai_agent_steps.is_empty() {
            agent_step_rows = agent_step_rows
                .mt_3()
                .border_t_1()
                .border_color(rgb(0x2a3140))
                .pt_2()
                .flex()
                .flex_col()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(0xe5edf7))
                        .child("Agent Steps"),
                );
            if self.ai_agent_steps.is_empty() {
                agent_step_rows = agent_step_rows.child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x98a3b8))
                        .child("No Agent steps yet."),
                );
            } else {
                for step in self.ai_agent_steps.iter().cloned().rev().take(8).rev() {
                    let (label, fg, bg) = ai_agent_step_status_style(step.status);
                    agent_step_rows = agent_step_rows.child(
                        div()
                            .flex()
                            .items_start()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight(700.))
                                            .text_color(rgb(0xe5edf7))
                                            .child(format!(
                                                "{}. {}",
                                                step.step_index.saturating_add(1),
                                                truncate_preview(&step.title, 40)
                                            )),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(0x98a3b8))
                                            .line_height(px(18.))
                                            .child(truncate_preview(&step.detail, 120)),
                                    ),
                            )
                            .child(status_pill(label, rgb(fg), rgb(bg))),
                    );
                }
            }
        }
        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x2a3140))
            .bg(rgb(0x151923))
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child(if agent_mode { "AI Agent" } else { "AI Ask" }),
                    )
                    .child(status_pill(
                        if ai_running { "running" } else { "ready" },
                        if ai_running {
                            rgb(0xfacc15)
                        } else {
                            rgb(0x6ee7b7)
                        },
                        if ai_running {
                            rgb(0x3a2f14)
                        } else {
                            rgb(0x12342a)
                        },
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .text_xs()
                    .text_color(rgb(0xaeb7c8))
                    .line_height(px(18.))
                    .child(self.ai_response_preview.clone()),
            )
            .child(agent_step_rows)
            .child(
                transfer_input(
                    "ai-ask-prompt",
                    "Prompt",
                    self.ai_prompt_draft.clone(),
                    true,
                )
                .mt_3()
                .track_focus(&self.ai_chat_focus)
                .on_click(cx.listener(|this, _, window, cx| {
                    window.focus(&this.ai_chat_focus);
                    cx.notify();
                }))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.handle_ai_prompt_key_down(event, cx);
                })),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(0x98a3b8))
                            .child(compact_id(&self.ai_chat_session_id)),
                    )
                    .child(small_button(
                        "ai-ask-run",
                        action_label,
                        cx.listener(|this, _, _, cx| {
                            if this.ai_chat_pending || this.ai_agent_loop.is_some() {
                                this.cancel_ai_chat(cx);
                            } else {
                                this.start_ai_ask(cx);
                            }
                        }),
                    )),
            )
            .child(command_rows)
    }

    fn command_center_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let sessions = self.session_manager.list_sessions().unwrap_or_default();
        let active_label = self
            .active_session_id
            .as_deref()
            .map(compact_id)
            .unwrap_or_else(|| "none".to_string());
        let provider = configured_cloud_sync_provider(&self.cloud_sync_settings);
        let provider_action = provider != "local_directory";
        let sync_label = if provider_action { "Provider" } else { "Local" };
        let mut session_rows = div().mt_3().flex().flex_col().gap_2();
        if sessions.is_empty() {
            session_rows = session_rows.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x98a3b8))
                    .child("No active runtime sessions."),
            );
        } else {
            for session in sessions.into_iter().take(3) {
                let is_active = self.active_session_id.as_deref() == Some(session.id.as_str());
                session_rows = session_rows.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_2()
                        .border_t_1()
                        .border_color(rgb(0x2a3140))
                        .pt_2()
                        .child(
                            div()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(0xe5edf7))
                                        .child(truncate_preview(&session.name, 42)),
                                )
                                .child(div().text_xs().text_color(rgb(0x98a3b8)).child(format!(
                                    "{} · {}",
                                    session_kind_label(session.kind),
                                    compact_id(&session.id)
                                ))),
                        )
                        .child(status_pill(
                            if is_active { "active" } else { "open" },
                            if is_active {
                                rgb(0x6ee7b7)
                            } else {
                                rgb(0x93c5fd)
                            },
                            if is_active {
                                rgb(0x12342a)
                            } else {
                                rgb(0x17233a)
                            },
                        )),
                );
            }
        }

        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x2a3140))
            .bg(rgb(0x151923))
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Command Center"),
                    )
                    .child(status_pill("native", rgb(0x6ee7b7), rgb(0x12342a))),
            )
            .child(
                div()
                    .mt_3()
                    .grid()
                    .grid_cols(2)
                    .gap_2()
                    .child(metric("Active", active_label))
                    .child(metric("Sync", provider)),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(small_button(
                        "command-center-new-session",
                        "New",
                        cx.listener(|this, _, _, cx| {
                            this.select(NavItem::Connections, cx);
                        }),
                    ))
                    .child(small_button(
                        "command-center-active-sessions",
                        "Sessions",
                        cx.listener(|this, _, _, cx| {
                            this.select(NavItem::Workspace, cx);
                        }),
                    ))
                    .child(small_button(
                        "command-center-settings",
                        "Settings",
                        cx.listener(|this, _, _, cx| {
                            this.select(NavItem::Settings, cx);
                        }),
                    ))
                    .child(small_button(
                        "command-center-update-check",
                        "Updates",
                        cx.listener(|this, _, _, cx| {
                            this.start_update_check(cx);
                        }),
                    )),
            )
            .child(
                div()
                    .mt_2()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(small_button(
                        "command-center-sync-push",
                        "Push",
                        cx.listener(move |this, _, _, cx| {
                            if provider_action {
                                this.prompt_provider_cloud_sync_push(cx);
                            } else {
                                this.prompt_local_cloud_sync_push(cx);
                            }
                        }),
                    ))
                    .child(small_button(
                        "command-center-sync-pull",
                        "Pull",
                        cx.listener(move |this, _, _, cx| {
                            if provider_action {
                                this.prompt_provider_cloud_sync_pull(cx);
                            } else {
                                this.prompt_local_cloud_sync_pull(cx);
                            }
                        }),
                    ))
                    .child(small_button(
                        "command-center-sync-history",
                        "History",
                        cx.listener(|this, _, _, cx| {
                            this.select(NavItem::Settings, cx);
                        }),
                    ))
                    .child(small_button(
                        "command-center-migration",
                        "Migration",
                        cx.listener(|this, _, _, cx| {
                            this.select(NavItem::Migration, cx);
                        }),
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .text_xs()
                    .text_color(rgb(0x98a3b8))
                    .line_height(px(18.))
                    .child(format!(
                        "{sync_label} sync · {} · {}",
                        truncate_preview(&self.cloud_sync_status, 84),
                        truncate_preview(&self.update_status, 84)
                    )),
            )
            .child(session_rows)
    }

    fn quick_commands_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let commands = sorted_quick_commands(&self.quick_commands);
        let mut rows = div().mt_3().flex().flex_col().gap_2();
        if commands.is_empty() {
            rows = rows.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x98a3b8))
                    .line_height(px(18.))
                    .child("No quick commands saved yet."),
            );
        } else {
            for (index, command) in commands.into_iter().take(5).enumerate() {
                let category =
                    quick_command_category_label(&self.quick_command_categories, &command);
                let meta = format!(
                    "{} · used {}",
                    category,
                    command.use_count.unwrap_or_default()
                );
                rows = rows.child(
                    div()
                        .border_t_1()
                        .border_color(rgb(0x2a3140))
                        .pt_2()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .min_w_0()
                                        .text_xs()
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(0xe5edf7))
                                        .child(truncate_preview(&command.label, 42)),
                                )
                                .child(div().text_xs().text_color(rgb(0x64748b)).child(meta)),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(0xaeb7c8))
                                .line_height(px(18.))
                                .child(truncate_preview(&command.command, 120)),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_end()
                                .gap_1()
                                .child(small_button(
                                    format!("quick-command-insert-{index}"),
                                    "Insert",
                                    cx.listener(move |this, _, _, cx| {
                                        this.insert_quick_command(index, cx);
                                    }),
                                ))
                                .child(small_button(
                                    format!("quick-command-run-{index}"),
                                    "Run",
                                    cx.listener(move |this, _, _, cx| {
                                        this.run_quick_command(index, cx);
                                    }),
                                )),
                        ),
                );
            }
        }

        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x2a3140))
            .bg(rgb(0x151923))
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Quick Commands"),
                    )
                    .child(small_button(
                        "quick-command-refresh",
                        "Refresh",
                        cx.listener(|this, _, _, cx| {
                            this.refresh_quick_commands();
                            cx.notify();
                        }),
                    )),
            )
            .child(rows)
    }

    fn recording_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let active_session_id = self.active_session_id.clone();
        let is_recording = active_session_id
            .as_deref()
            .is_some_and(|session_id| self.recording_manager.is_recording(session_id));
        let search = self.recording_search_results();
        let mut rows = div().mt_3().flex().flex_col().gap_2();
        match search {
            Ok(response) if response.results.is_empty() => {
                rows = rows.child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x98a3b8))
                        .line_height(px(18.))
                        .child(if self.recording_search_draft.trim().is_empty() {
                            "Type to search captured terminal history."
                        } else {
                            "No transcript matches."
                        }),
                );
            }
            Ok(response) => {
                rows = rows.child(div().text_xs().text_color(rgb(0x64748b)).child(format!(
                    "{} match(es) · {} ms",
                    response.total, response.elapsed_ms
                )));
                for result in response.results.into_iter().take(4) {
                    let meta = format!("{} · line {}", result.source, result.line_number);
                    rows = rows.child(
                        div()
                            .border_t_1()
                            .border_color(rgb(0x2a3140))
                            .pt_2()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    .child(
                                        div()
                                            .min_w_0()
                                            .text_xs()
                                            .font_family("JetBrains Mono")
                                            .text_color(rgb(0xe5edf7))
                                            .child(truncate_preview(&result.preview, 120)),
                                    )
                                    .child(div().text_xs().text_color(rgb(0x64748b)).child(meta)),
                            )
                            .child(div().text_xs().text_color(rgb(0x98a3b8)).child(format!(
                                "context {} before / {} after",
                                result.before.len(),
                                result.after.len()
                            ))),
                    );
                }
            }
            Err(error) => {
                rows = rows.child(
                    div()
                        .text_xs()
                        .text_color(rgb(0xfca5a5))
                        .line_height(px(18.))
                        .child(format!("Search failed: {error}")),
                );
            }
        }

        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x2a3140))
            .bg(rgb(0x151923))
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Recording"),
                    )
                    .child(status_pill(
                        if is_recording { "recording" } else { "idle" },
                        if is_recording {
                            rgb(0xfca5a5)
                        } else {
                            rgb(0x93c5fd)
                        },
                        if is_recording {
                            rgb(0x3a1717)
                        } else {
                            rgb(0x17233a)
                        },
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(small_button(
                        "recording-start",
                        "Start",
                        cx.listener(|this, _, _, cx| {
                            this.prompt_recording_path(RecordingPathPromptKind::Start, cx);
                        }),
                    ))
                    .child(small_button(
                        "recording-stop",
                        "Stop",
                        cx.listener(|this, _, _, cx| {
                            this.stop_active_recording(cx);
                        }),
                    ))
                    .child(small_button(
                        "recording-save-transcript",
                        "Save",
                        cx.listener(|this, _, _, cx| {
                            this.prompt_recording_path(RecordingPathPromptKind::SaveTranscript, cx);
                        }),
                    )),
            )
            .child(
                transfer_input(
                    "recording-search-input",
                    "Transcript Search",
                    self.recording_search_draft.clone(),
                    true,
                )
                .mt_3()
                .track_focus(&self.recording_search_focus)
                .on_click(cx.listener(|this, _, window, cx| {
                    window.focus(&this.recording_search_focus);
                    cx.notify();
                }))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.handle_recording_search_key_down(event, cx);
                })),
            )
            .child(rows)
    }

    fn command_search_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let results = self.command_search_results();
        let mut rows = div().mt_3().flex().flex_col().gap_2();
        if results.is_empty() {
            rows = rows.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x98a3b8))
                    .line_height(px(18.))
                    .child("No matches."),
            );
        } else {
            for (index, result) in results.into_iter().enumerate() {
                let meta = format!(
                    "{} · {}",
                    command_source_label(&result.source),
                    result.score
                );
                rows = rows.child(
                    div()
                        .border_t_1()
                        .border_color(rgb(0x2a3140))
                        .pt_2()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .min_w_0()
                                        .text_xs()
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(0xe5edf7))
                                        .child(truncate_preview(&result.display, 44)),
                                )
                                .child(div().text_xs().text_color(rgb(0x64748b)).child(meta)),
                        )
                        .child(
                            div()
                                .font_family("JetBrains Mono")
                                .text_xs()
                                .text_color(rgb(0xaeb7c8))
                                .line_height(px(18.))
                                .child(truncate_preview(&result.command, 120)),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_end()
                                .gap_1()
                                .child(small_button(
                                    format!("command-search-insert-{index}"),
                                    "Insert",
                                    cx.listener(move |this, _, _, cx| {
                                        this.insert_command_search_result(index, cx);
                                    }),
                                ))
                                .child(small_button(
                                    format!("command-search-run-{index}"),
                                    "Run",
                                    cx.listener(move |this, _, _, cx| {
                                        this.run_command_search_result(index, cx);
                                    }),
                                )),
                        ),
                );
            }
        }

        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x2a3140))
            .bg(rgb(0x151923))
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Command Search"),
                    )
                    .child(status_pill(
                        if self.command_search_draft.trim().is_empty() {
                            "idle"
                        } else {
                            "matched"
                        },
                        rgb(0x93c5fd),
                        rgb(0x17233a),
                    )),
            )
            .child(
                transfer_input(
                    "command-search-input",
                    "Search",
                    self.command_search_draft.clone(),
                    true,
                )
                .mt_3()
                .track_focus(&self.command_search_focus)
                .on_click(cx.listener(|this, _, window, cx| {
                    window.focus(&this.command_search_focus);
                    cx.notify();
                }))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.handle_command_search_key_down(event, cx);
                })),
            )
            .child(rows)
    }

    fn command_history_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut rows = div().mt_3().flex().flex_col().gap_2();
        if self.command_history.is_empty() {
            rows = rows.child(
                div()
                    .text_xs()
                    .text_color(rgb(0x98a3b8))
                    .line_height(px(18.))
                    .child("No command history yet."),
            );
        } else {
            for (index, entry) in self.command_history.iter().cloned().take(8).enumerate() {
                let meta = format!("used {}", entry.use_count);
                rows = rows.child(
                    div()
                        .border_t_1()
                        .border_color(rgb(0x2a3140))
                        .pt_2()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div()
                                        .min_w_0()
                                        .text_xs()
                                        .text_color(rgb(0xaeb7c8))
                                        .font_family("JetBrains Mono")
                                        .child(truncate_preview(&entry.command, 120)),
                                )
                                .child(div().text_xs().text_color(rgb(0x64748b)).child(meta)),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_end()
                                .gap_1()
                                .child(small_button(
                                    format!("history-command-insert-{index}"),
                                    "Insert",
                                    cx.listener(move |this, _, _, cx| {
                                        this.insert_history_command(index, cx);
                                    }),
                                ))
                                .child(small_button(
                                    format!("history-command-run-{index}"),
                                    "Run",
                                    cx.listener(move |this, _, _, cx| {
                                        this.run_history_command(index, cx);
                                    }),
                                ))
                                .child(small_button(
                                    format!("history-command-delete-{index}"),
                                    "Delete",
                                    cx.listener(move |this, _, _, cx| {
                                        this.delete_history_command(index, cx);
                                    }),
                                )),
                        ),
                );
            }
        }

        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x2a3140))
            .bg(rgb(0x151923))
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Command History"),
                    )
                    .child(small_button(
                        "command-history-refresh",
                        "Refresh",
                        cx.listener(|this, _, _, cx| {
                            this.refresh_command_history();
                            cx.notify();
                        }),
                    )),
            )
            .child(rows)
    }

    fn right_panel(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(320.))
            .flex_none()
            .flex()
            .flex_col()
            .gap_3()
            .child(self.command_center_panel(cx))
            .child(self.ai_ask_panel(cx))
            .child(self.recording_panel(cx))
            .child(self.command_search_panel(cx))
            .child(self.quick_commands_panel(cx))
            .child(self.command_history_panel(cx))
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Session Types"),
                    )
                    .child(capability_line("SSH", "native baseline ready"))
                    .child(capability_line("Local PTY", "native service ready"))
                    .child(capability_line("Telnet", "native service ready"))
                    .child(capability_line("Raw TCP", "native service ready"))
                    .child(capability_line("Serial", "native service ready")),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Runtime Sessions"),
                    )
                    .child(
                        div().mt_2().text_3xl().font_weight(FontWeight(800.)).child(
                            self.session_manager
                                .list_sessions()
                                .map(|sessions| sessions.len().to_string())
                                .unwrap_or_else(|_| "0".to_string()),
                        ),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_xs()
                            .text_color(rgb(0x98a3b8))
                            .child("Managed by the Tauri-free nyaterm-session service."),
                    ),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Imported Profiles"),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_3xl()
                            .font_weight(FontWeight(800.))
                            .child(self.connections.len().to_string()),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_xs()
                            .text_color(rgb(0x98a3b8))
                            .child("Loaded from native runtime configuration when present."),
                    ),
            )
    }
}

impl NyaTermApp {
    fn terminal_canvas(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut output = div().flex().flex_col().gap_1();
        for line in self.terminal_screen.lines() {
            let line = if line.is_empty() {
                " ".to_string()
            } else {
                line
            };
            output = output.child(terminal_line_element(&line, &self.keyword_highlights));
        }

        div()
            .flex_1()
            .min_h_0()
            .p_4()
            .font_family("JetBrains Mono")
            .text_sm()
            .text_color(rgb(0xc8d3f5))
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x222836))
                    .bg(rgb(0x07090d))
                    .size_full()
                    .flex()
                    .flex_col()
                    .track_focus(&self.terminal_focus)
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                        cx.stop_propagation();
                        if let Some(bytes) = terminal_key_bytes(event) {
                            this.send_terminal_input(bytes, cx);
                        }
                    }))
                    .child(
                        div()
                            .h(px(42.))
                            .flex()
                            .items_center()
                            .gap_2()
                            .px_3()
                            .border_b_1()
                            .border_color(rgb(0x222836))
                            .child(small_button(
                                "terminal-start-local",
                                "Start Local",
                                cx.listener(|this, _, window, cx| {
                                    this.start_local_session(window, cx);
                                }),
                            ))
                            .child(small_button(
                                "terminal-probe",
                                "Probe",
                                cx.listener(|this, _, _, cx| {
                                    this.send_probe_command(cx);
                                }),
                            ))
                            .child(small_button(
                                "terminal-close",
                                "Close",
                                cx.listener(|this, _, _, cx| {
                                    this.close_active_session(cx);
                                }),
                            ))
                            .child(small_button(
                                "terminal-clear",
                                "Clear",
                                cx.listener(|this, _, _, cx| {
                                    this.clear_terminal(cx);
                                }),
                            ))
                            .child(div().flex_1())
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x98a3b8))
                                    .child(self.terminal_status.clone()),
                            ),
                    )
                    .child(
                        div()
                            .id(SharedString::from("terminal-output"))
                            .flex_1()
                            .min_h_0()
                            .p_4()
                            .overflow_hidden()
                            .on_click(cx.listener(|this, _, window, cx| {
                                window.focus(&this.terminal_focus);
                                this.terminal_status = "terminal focused".to_string();
                                cx.notify();
                            }))
                            .child(output),
                    ),
            )
    }
}

struct TerminalHighlightSpan {
    text: String,
    color: Option<u32>,
}

fn terminal_line_element(line: &str, config: &KeywordHighlightConfig) -> impl IntoElement {
    let spans = keyword_highlight_spans(line, config);
    let mut row = div()
        .flex()
        .flex_row()
        .items_center()
        .min_h(px(18.))
        .line_height(px(18.))
        .whitespace_nowrap();

    for span in spans {
        let mut child = div()
            .line_height(px(18.))
            .whitespace_nowrap()
            .child(span.text);
        if let Some(color) = span.color {
            child = child.text_color(rgb(color)).bg(rgb(0x16202e));
        }
        row = row.child(child);
    }

    row
}

fn keyword_highlight_spans(
    line: &str,
    config: &KeywordHighlightConfig,
) -> Vec<TerminalHighlightSpan> {
    if !config.enabled || config.rules.is_empty() || line.is_empty() {
        return vec![TerminalHighlightSpan {
            text: line.to_string(),
            color: None,
        }];
    }

    let lowered = line.to_ascii_lowercase();
    let mut spans = Vec::new();
    let mut cursor = 0;
    while cursor < line.len() {
        let mut best: Option<(usize, usize, u32)> = None;
        for rule in config.rules.iter().filter(|rule| rule.enabled) {
            let color = parse_hex_rgb(&rule.color_dark).unwrap_or(0x79c0ff);
            for pattern in rule.patterns.iter().map(|pattern| pattern.trim()) {
                if pattern.is_empty() {
                    continue;
                }
                let needle = pattern.to_ascii_lowercase();
                if let Some(relative_start) = lowered[cursor..].find(&needle) {
                    let start = cursor + relative_start;
                    let end = start + needle.len();
                    let replace = best
                        .map(|(best_start, best_end, _)| {
                            start < best_start || (start == best_start && end > best_end)
                        })
                        .unwrap_or(true);
                    if replace {
                        best = Some((start, end, color));
                    }
                }
            }
        }

        let Some((start, end, color)) = best else {
            spans.push(TerminalHighlightSpan {
                text: line[cursor..].to_string(),
                color: None,
            });
            break;
        };
        if start > cursor {
            spans.push(TerminalHighlightSpan {
                text: line[cursor..start].to_string(),
                color: None,
            });
        }
        spans.push(TerminalHighlightSpan {
            text: line[start..end].to_string(),
            color: Some(color),
        });
        cursor = end;
    }

    if spans.is_empty() {
        spans.push(TerminalHighlightSpan {
            text: " ".to_string(),
            color: None,
        });
    }
    spans
}

fn parse_hex_rgb(value: &str) -> Option<u32> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if hex.len() != 6 {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

fn logo_mark() -> impl IntoElement {
    div()
        .size(px(26.))
        .rounded_md()
        .bg(rgb(0x6ee7b7))
        .shadow_sm()
        .child(
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgb(0x062018))
                .font_weight(FontWeight(900.))
                .child("N"),
        )
}

fn status_pill(
    label: &'static str,
    fg: impl Into<gpui::Hsla>,
    bg: impl Into<gpui::Hsla>,
) -> impl IntoElement {
    div()
        .rounded_sm()
        .px_2()
        .py_1()
        .text_xs()
        .text_color(fg.into())
        .bg(bg.into())
        .child(label)
}

fn ai_agent_step_status_style(status: AiAgentStepStatus) -> (&'static str, u32, u32) {
    match status {
        AiAgentStepStatus::Planning => ("planning", 0x93c5fd, 0x17233a),
        AiAgentStepStatus::Tool => ("tool", 0xc4b5fd, 0x2b2142),
        AiAgentStepStatus::NeedsApproval => ("review", 0xfacc15, 0x3a2f14),
        AiAgentStepStatus::Running => ("running", 0x6ee7b7, 0x12342a),
        AiAgentStepStatus::Completed => ("done", 0x86efac, 0x12301f),
        AiAgentStepStatus::Failed => ("failed", 0xfca5a5, 0x3a1717),
        AiAgentStepStatus::Cancelled => ("cancelled", 0xcbd5e1, 0x273244),
    }
}

fn empty_panel(text: &'static str) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x10151e))
        .p_4()
        .text_sm()
        .text_color(rgb(0xaeb7c8))
        .child(text)
}

fn stats_resource_row(label: &str, detail: &str, ratio: f64) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x10151e))
        .p_3()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .min_w_0()
                        .text_sm()
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(0xe5edf7))
                        .child(truncate_preview(label, 36)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x98a3b8))
                        .child(format!("{:.0}%", ratio.clamp(0., 1.) * 100.)),
                ),
        )
        .child(
            div()
                .mt_1()
                .text_xs()
                .text_color(rgb(0x98a3b8))
                .child(truncate_preview(detail, 96)),
        )
        .child(stats_progress_bar(ratio))
}

fn stats_progress_bar(ratio: f64) -> impl IntoElement {
    let ratio = ratio.clamp(0., 1.);
    div()
        .mt_3()
        .h(px(6.))
        .w_full()
        .overflow_hidden()
        .rounded_sm()
        .bg(rgb(0x242b38))
        .child(
            div()
                .h(px(6.))
                .w(px(220. * ratio as f32))
                .rounded_sm()
                .bg(if ratio >= 0.9 {
                    rgb(0xfb7185)
                } else if ratio >= 0.75 {
                    rgb(0xfacc15)
                } else {
                    rgb(0x38bdf8)
                }),
        )
}

fn format_rate(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1024. * 1024. {
        format!("{:.1} MiB/s", bytes_per_sec / 1024. / 1024.)
    } else if bytes_per_sec >= 1024. {
        format!("{:.1} KiB/s", bytes_per_sec / 1024.)
    } else {
        format!("{bytes_per_sec:.0} B/s")
    }
}

fn format_uptime(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {minutes}m")
    } else {
        format!("{minutes}m")
    }
}

fn risk_label(risk: Option<&RiskLevel>) -> &'static str {
    match risk {
        Some(RiskLevel::Low) => "Low",
        Some(RiskLevel::Medium) => "Medium",
        Some(RiskLevel::High) => "High",
        Some(RiskLevel::Critical) => "Critical",
        None => "Unrated",
    }
}

fn sorted_quick_commands(commands: &[QuickCommand]) -> Vec<QuickCommand> {
    let mut commands = commands.to_vec();
    commands.sort_by(|left, right| {
        right
            .pinned
            .unwrap_or_default()
            .cmp(&left.pinned.unwrap_or_default())
            .then_with(|| {
                right
                    .use_count
                    .unwrap_or_default()
                    .cmp(&left.use_count.unwrap_or_default())
            })
            .then_with(|| {
                right
                    .updated_at
                    .unwrap_or_default()
                    .cmp(&left.updated_at.unwrap_or_default())
            })
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
    });
    commands
}

fn quick_command_category_label(
    categories: &[QuickCommandCategory],
    command: &QuickCommand,
) -> String {
    command
        .category_id
        .as_deref()
        .and_then(|id| categories.iter().find(|category| category.id == id))
        .map(|category| category.name.clone())
        .unwrap_or_else(|| "Unsorted".to_string())
}

fn command_source_label(source: &str) -> &'static str {
    match source {
        "quickCommand" => "quick",
        "history" => "history",
        _ => "command",
    }
}

fn ai_command_card_category_name(card: &AiCommandCard) -> String {
    card.category
        .as_deref()
        .map(str::trim)
        .filter(|category| !category.is_empty())
        .unwrap_or("AI Generated")
        .to_string()
}

fn unique_quick_command_category_id(
    categories: &[QuickCommandCategory],
    category_name: &str,
) -> String {
    let base = format!("ai-{}", quick_command_slug(category_name));
    if !categories.iter().any(|category| category.id == base) {
        return base;
    }
    for suffix in 2.. {
        let candidate = format!("{base}-{suffix}");
        if !categories.iter().any(|category| category.id == candidate) {
            return candidate;
        }
    }
    unreachable!("unbounded suffix search always returns")
}

fn quick_command_slug(input: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_was_dash = false;
        } else if matches!(ch, '-' | '_' | ' ' | '\t' | '\n' | '\r') && !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "commands".to_string()
    } else {
        slug.to_string()
    }
}

fn recording_file_path(
    settings: &AppSettingsSummary,
    config_dir: &std::path::Path,
    session_name: &str,
) -> PathBuf {
    let base_dir = if settings.recording_path.trim().is_empty() {
        config_dir.join("recordings")
    } else {
        PathBuf::from(settings.recording_path.trim())
    };
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    base_dir.join(format!(
        "recording-{}-{timestamp_ms}.log",
        safe_recording_name(session_name)
    ))
}

fn docker_state_rank(state: &str) -> u8 {
    match state.trim().to_ascii_lowercase().as_str() {
        "running" => 0,
        "restarting" | "paused" => 1,
        "created" => 2,
        "exited" | "dead" => 3,
        _ => 4,
    }
}

fn docker_state_label(state: &str) -> &'static str {
    match state.trim().to_ascii_lowercase().as_str() {
        "running" => "running",
        "restarting" => "restart",
        "paused" => "paused",
        "created" => "created",
        "exited" => "exited",
        "dead" => "dead",
        _ => "unknown",
    }
}

fn docker_state_color(state: &str) -> gpui::Hsla {
    match state.trim().to_ascii_lowercase().as_str() {
        "running" => rgb(0x6ee7b7).into(),
        "restarting" | "paused" => rgb(0xfbbf24).into(),
        "created" => rgb(0x93c5fd).into(),
        "exited" | "dead" => rgb(0xfca5a5).into(),
        _ => rgb(0x98a3b8).into(),
    }
}

fn session_kind_label(kind: SessionKind) -> &'static str {
    match kind {
        SessionKind::LocalPty => "local",
        SessionKind::Ssh => "ssh",
        SessionKind::Telnet => "telnet",
        SessionKind::RawTcp => "raw tcp",
        SessionKind::Serial => "serial",
    }
}

fn service_status(status: NativeServiceStatus) -> impl IntoElement {
    match status {
        NativeServiceStatus::Ready => {
            status_pill("ready", rgb(0x6ee7b7), rgb(0x12342a)).into_any_element()
        }
        NativeServiceStatus::Porting => {
            status_pill("porting", rgb(0xfbbf24), rgb(0x3a2f14)).into_any_element()
        }
        NativeServiceStatus::Blocked => {
            status_pill("replace", rgb(0xfca5a5), rgb(0x3a1717)).into_any_element()
        }
    }
}

fn cloud_sync_history_status(error: &CloudSyncError) -> &'static str {
    match error {
        CloudSyncError::Conflict(_) => "conflict",
        _ => "failed",
    }
}

fn push_provider_snapshot(
    settings: &CloudSyncSettings,
    options: &LocalCloudSyncOptions,
    state: &CloudSyncState,
    force: bool,
) -> Result<CloudSyncResult, CloudSyncError> {
    match settings.provider.as_str() {
        "webdav" => {
            let remote = NativeWebdavRemote::new(&settings.webdav)?;
            push_snapshot_with_remote(options, &remote, state, force)
        }
        "s3" => {
            let remote = NativeS3Remote::new(&settings.s3)?;
            push_snapshot_with_remote(options, &remote, state, force)
        }
        "google_drive" => {
            let remote = NativeGoogleDriveRemote::new(&settings.google_drive)?;
            push_snapshot_with_remote(options, &remote, state, force)
        }
        "onedrive" => {
            let remote = NativeOneDriveRemote::new(&settings.onedrive)?;
            push_snapshot_with_remote(options, &remote, state, force)
        }
        "aliyun_drive" => {
            let remote = NativeAliyunDriveRemote::new(&settings.aliyun_drive)?;
            push_snapshot_with_remote(options, &remote, state, force)
        }
        "gitee_snippet" => {
            let backend = GiteeSnippetHttpBackend::new(
                &settings.gitee_snippet,
                NativeSnippetHttpClient::new()?,
            )?;
            let remote = SnippetRemote::new("gitee_snippet", backend);
            push_snapshot_with_remote(options, &remote, state, force)
        }
        "github_gist" => {
            let backend =
                GithubGistHttpBackend::new(&settings.github_gist, NativeSnippetHttpClient::new()?)?;
            let remote = SnippetRemote::new("github_gist", backend);
            push_snapshot_with_remote(options, &remote, state, force)
        }
        "local_directory" => push_local_snapshot(options, state, force),
        provider => Err(CloudSyncError::Remote(format!(
            "native cloud provider '{provider}' is not wired yet"
        ))),
    }
}

fn pull_provider_snapshot(
    settings: &CloudSyncSettings,
    options: &LocalCloudSyncOptions,
    state: &CloudSyncState,
    force: bool,
) -> Result<CloudSyncResult, CloudSyncError> {
    match settings.provider.as_str() {
        "webdav" => {
            let remote = NativeWebdavRemote::new(&settings.webdav)?;
            pull_snapshot_with_remote(options, &remote, state, force)
        }
        "s3" => {
            let remote = NativeS3Remote::new(&settings.s3)?;
            pull_snapshot_with_remote(options, &remote, state, force)
        }
        "google_drive" => {
            let remote = NativeGoogleDriveRemote::new(&settings.google_drive)?;
            pull_snapshot_with_remote(options, &remote, state, force)
        }
        "onedrive" => {
            let remote = NativeOneDriveRemote::new(&settings.onedrive)?;
            pull_snapshot_with_remote(options, &remote, state, force)
        }
        "aliyun_drive" => {
            let remote = NativeAliyunDriveRemote::new(&settings.aliyun_drive)?;
            pull_snapshot_with_remote(options, &remote, state, force)
        }
        "gitee_snippet" => {
            let backend = GiteeSnippetHttpBackend::new(
                &settings.gitee_snippet,
                NativeSnippetHttpClient::new()?,
            )?;
            let remote = SnippetRemote::new("gitee_snippet", backend);
            pull_snapshot_with_remote(options, &remote, state, force)
        }
        "github_gist" => {
            let backend =
                GithubGistHttpBackend::new(&settings.github_gist, NativeSnippetHttpClient::new()?)?;
            let remote = SnippetRemote::new("github_gist", backend);
            pull_snapshot_with_remote(options, &remote, state, force)
        }
        "local_directory" => pull_local_snapshot(options, state, force),
        provider => Err(CloudSyncError::Remote(format!(
            "native cloud provider '{provider}' is not wired yet"
        ))),
    }
}

fn configured_cloud_sync_provider(settings: &CloudSyncSettings) -> String {
    let provider = settings.provider.trim();
    if provider.is_empty() {
        "local_directory".to_string()
    } else {
        provider.to_string()
    }
}

fn is_agent_command_card(card: &AiCommandCard) -> bool {
    card.id.starts_with("agent-")
        || card
            .category
            .as_deref()
            .is_some_and(|category| category == "AI Agent")
}

fn run_ai_ask_job(
    config_dir: PathBuf,
    portable_key_path: Option<PathBuf>,
    settings: AiSettings,
    mut request: AiChatRequest,
    stream_tx: Option<mpsc::Sender<AiChatWorkerEvent>>,
    cancel: Arc<AtomicBool>,
    job_id: u64,
) -> Result<AiChatJobOutput, String> {
    if ai_job_cancelled(&cancel) {
        return Err("AI request cancelled".to_string());
    }
    if settings.redaction_enabled {
        redact_context(&mut request.context);
        request.user_input = redact_sensitive_text(&request.user_input);
    }
    let session_id = request
        .session_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("ai-session-{}", uuid()));
    request.session_id = Some(session_id.clone());

    let store = ConnectionStore::open_with_portable_key_path(&config_dir, portable_key_path)
        .map_err(|error| error.to_string())?;
    let history = store.load_ai_history().map_err(|error| error.to_string())?;
    if settings.record_history {
        store
            .append_ai_user_message(
                &session_id,
                request.connection_id.clone(),
                request.user_input.clone(),
            )
            .map_err(|error| error.to_string())?;
    }
    if ai_job_cancelled(&cancel) {
        return Err("AI request cancelled".to_string());
    }

    let completion = if matches!(request.mode, AiMode::Ask | AiMode::Agent) {
        let delta_session_id = session_id.clone();
        let stream_cancel = cancel.clone();
        let stream_mode = request.mode.clone();
        stream_native_chat(&settings, &request, &history.messages, |delta| {
            if ai_job_cancelled(&stream_cancel) {
                return;
            }
            if delta.done {
                return;
            }
            if let Some(tx) = stream_tx.as_ref() {
                let AiChatStreamDelta {
                    text_delta,
                    reasoning_delta,
                    tool_call_deltas,
                    done: _,
                } = delta;
                if !text_delta.is_empty() || reasoning_delta.is_some() {
                    let _ = tx.send(AiChatWorkerEvent::Delta {
                        job_id,
                        session_id: delta_session_id.clone(),
                        text_delta,
                        reasoning_delta,
                    });
                }
                if stream_mode == AiMode::Agent {
                    for tool_delta in tool_call_deltas {
                        let _ = tx.send(AiChatWorkerEvent::AgentToolCallDelta {
                            job_id,
                            session_id: delta_session_id.clone(),
                            tool_name: tool_delta.name_delta,
                            arguments_delta_len: tool_delta.arguments_delta.len(),
                        });
                    }
                }
            }
        })?
    } else {
        if ai_job_cancelled(&cancel) {
            return Err("AI request cancelled".to_string());
        }
        complete_native_chat(&settings, &request, &history.messages)?
    };
    if ai_job_cancelled(&cancel) {
        return Err("AI request cancelled".to_string());
    }
    let output = if request.mode == AiMode::Agent {
        ai_agent_job_output(
            &settings,
            &request,
            completion.text,
            completion.reasoning_content,
            completion.tool_calls,
        )?
    } else {
        let (text, reasoning, command_cards) =
            parse_model_output(&completion.text, completion.reasoning_content);
        AiChatJobOutput {
            mode: AiMode::Ask,
            text,
            reasoning,
            command_cards,
            auto_execute_first: false,
            approval_note: None,
        }
    };
    if settings.record_history {
        store
            .append_ai_message(AiMessage {
                id: format!("msg-{}", uuid()),
                session_id,
                role: AiMessageRole::Assistant,
                content: output.text.clone(),
                created_at: now_rfc3339(),
                reasoning_content: output.reasoning.clone(),
                command_cards: output.command_cards.clone(),
            })
            .map_err(|error| error.to_string())?;
    }

    Ok(output)
}

fn ai_job_cancelled(cancel: &Arc<AtomicBool>) -> bool {
    cancel.load(Ordering::Relaxed)
}

fn remote_command_observation(output: RemoteCommandOutput, started: Instant) -> CommandObservation {
    CommandObservation {
        output: merge_command_output(&output.stdout, &output.stderr),
        exit_code: output
            .exit_status
            .and_then(|status| i32::try_from(status).ok()),
        duration_ms: started.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
    }
}

fn observation_summary(observation: &CommandObservation) -> String {
    let status = observation
        .exit_code
        .map(|code| format!("exit {code}"))
        .unwrap_or_else(|| "exit unknown".to_string());
    let preview = truncate_preview(observation.output.trim(), 100);
    if preview.is_empty() {
        format!("{status}; {} ms; no output", observation.duration_ms)
    } else {
        format!("{status}; {} ms; {preview}", observation.duration_ms)
    }
}

fn merge_command_output(stdout: &str, stderr: &str) -> String {
    let stdout = stdout.trim_end();
    let stderr = stderr.trim_end();
    match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => String::new(),
        (false, true) => stdout.to_string(),
        (true, false) => stderr.to_string(),
        (false, false) => format!("{stdout}\n{stderr}"),
    }
}

fn ai_agent_job_output(
    settings: &AiSettings,
    request: &AiChatRequest,
    text: String,
    reasoning: Option<String>,
    tool_calls: Vec<nyaterm_domain::AiToolCall>,
) -> Result<AiChatJobOutput, String> {
    let parsed = if tool_calls.is_empty() {
        parse_agent_model_output(&text).map_err(|error| error.to_string())?
    } else {
        parse_agent_tool_call(&tool_calls)
            .or_else(|_| parse_agent_model_output(&text))
            .map_err(|error| error.to_string())?
    };
    let action = agent_response_action(&parsed);
    if action == "final_answer" {
        let answer = parsed
            .answer
            .as_deref()
            .map(str::trim)
            .filter(|answer| !answer.is_empty())
            .unwrap_or("Agent finished without a final answer")
            .to_string();
        return Ok(AiChatJobOutput {
            mode: AiMode::Agent,
            text: answer,
            reasoning: Some(parsed.thought).or(reasoning),
            command_cards: Vec::new(),
            auto_execute_first: false,
            approval_note: None,
        });
    }

    let command = parsed
        .command
        .as_deref()
        .map(str::trim)
        .filter(|command| !command.is_empty())
        .ok_or_else(|| "Agent returned execute_command without a command".to_string())?;
    let assessment = assess_agent_command_risk(&parsed, command);
    let (decision, approval_note) = decide_agent_command_execution(settings, &assessment);
    let explanation = parsed
        .thought
        .trim()
        .is_empty()
        .then(|| "Agent requested command execution".to_string())
        .unwrap_or_else(|| parsed.thought.trim().to_string());
    let card = AiCommandCard {
        id: format!("agent-{}", uuid()),
        title: "Agent Command".to_string(),
        command: command.to_string(),
        explanation,
        risk_level: Some(assessment.effective_risk),
        risk_reason: assessment.risk_reason,
        expected_effect: "Run the next native Agent step in the active terminal".to_string(),
        rollback: Some("Review terminal output before running additional Agent steps".to_string()),
        category: Some("AI Agent".to_string()),
        references: request
            .terminal_session_id
            .as_ref()
            .map(|id| vec![format!("terminal:{id}")])
            .unwrap_or_default(),
    };
    let auto_execute_first = decision == AgentApprovalDecision::Auto;
    let approval_note = if auto_execute_first {
        Some("agent policy allows automatic execution".to_string())
    } else {
        approval_note
    };
    let text = approval_note
        .as_deref()
        .map(|note| format!("Agent proposed `{}`; {note}", card.command))
        .unwrap_or_else(|| format!("Agent proposed `{}`", card.command));

    Ok(AiChatJobOutput {
        mode: AiMode::Agent,
        text,
        reasoning: Some(parsed.thought).or(reasoning),
        command_cards: vec![card],
        auto_execute_first,
        approval_note,
    })
}

fn ai_active_profile_drafts(settings: &AiSettings) -> (String, String) {
    settings
        .provider_profiles
        .iter()
        .find(|profile| profile.id == settings.active_profile_id)
        .map(|profile| {
            (
                profile.model.clone(),
                profile.base_url.clone().unwrap_or_default(),
            )
        })
        .unwrap_or_default()
}

fn ai_active_profile_api_key(settings: &AiSettings) -> Option<String> {
    settings
        .provider_credentials
        .iter()
        .find(|credential| credential.id == settings.active_profile_id)
        .and_then(|credential| credential.api_key.clone())
        .or_else(|| {
            settings
                .provider_profiles
                .iter()
                .find(|profile| profile.id == settings.active_profile_id)
                .and_then(|profile| profile.api_key.clone())
        })
}

fn ai_usage_counts(store: &ConnectionStore) -> (usize, usize, usize) {
    let history = store.load_ai_history().unwrap_or_default();
    let audit_count = store
        .list_ai_audit_logs(None)
        .map(|logs| logs.len())
        .unwrap_or_default();
    (history.sessions.len(), history.messages.len(), audit_count)
}

fn none_if_blank(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn recent_terminal_output(output: &str, max_lines: usize) -> String {
    let lines = output.lines().collect::<Vec<_>>();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}

fn cloud_secret_display(draft: &str, current: &Option<String>) -> String {
    if !draft.is_empty() {
        "*".repeat(draft.chars().count())
    } else if current
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        "set".to_string()
    } else {
        " ".to_string()
    }
}

fn section_header(title: &'static str, detail: &'static str) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_2xl().font_weight(FontWeight(800.)).child(title))
        .child(div().text_sm().text_color(rgb(0x98a3b8)).child(detail))
}

fn metric(label: &'static str, value: impl Into<SharedString>) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x151923))
        .p_4()
        .child(div().text_xs().text_color(rgb(0x98a3b8)).child(label))
        .child(
            div()
                .mt_2()
                .text_2xl()
                .font_weight(FontWeight(800.))
                .child(value.into()),
        )
}

fn setting_state(label: &'static str, value: &'static str) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x151923))
        .p_4()
        .child(div().text_xs().text_color(rgb(0x98a3b8)).child(label))
        .child(
            div()
                .mt_2()
                .text_lg()
                .font_weight(FontWeight(700.))
                .child(value),
        )
}

fn compact_setting_state(label: &'static str, value: String) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x111722))
        .p_3()
        .child(div().text_xs().text_color(rgb(0x98a3b8)).child(label))
        .child(
            div()
                .mt_1()
                .text_sm()
                .font_weight(FontWeight(700.))
                .child(value),
        )
}

fn cloud_sync_history_row(entry: CloudSyncHistoryEntry) -> impl IntoElement {
    let status_color = match entry.status.as_str() {
        "success" => rgb(0x86efac),
        "conflict" => rgb(0xfacc15),
        "failed" => rgb(0xfca5a5),
        _ => rgb(0xcbd5e1),
    };
    let provider = entry
        .provider
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unknown");
    let revision = entry
        .revision
        .as_deref()
        .map(compact_id)
        .unwrap_or_else(|| "no revision".to_string());
    let duration = entry
        .duration_ms
        .map(|value| format!(" / {value} ms"))
        .unwrap_or_default();
    let meta = format!("{} / {provider} / {revision}{duration}", entry.trigger);

    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x273244))
        .bg(rgb(0x111722))
        .p_3()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight(700.))
                        .text_color(status_color)
                        .child(entry.status),
                )
                .child(div().text_xs().text_color(rgb(0x98a3b8)).child(entry.kind)),
        )
        .child(
            div()
                .mt_1()
                .text_sm()
                .font_weight(FontWeight(700.))
                .child(entry.message),
        )
        .child(div().mt_1().text_xs().text_color(rgb(0x98a3b8)).child(meta))
}

fn compact_id(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.chars().count() <= 12 {
        trimmed.to_string()
    } else {
        let prefix: String = trimmed.chars().take(8).collect();
        let suffix: String = trimmed
            .chars()
            .rev()
            .take(4)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("{prefix}..{suffix}")
    }
}

fn policy_button(
    id: &'static str,
    label: &'static str,
    selected: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id))
        .h(px(30.))
        .px_3()
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(if selected {
            rgb(0x4ade80)
        } else {
            rgb(0x303848)
        })
        .bg(if selected {
            rgb(0x173823)
        } else {
            rgb(0x151b27)
        })
        .text_color(if selected {
            rgb(0xbbf7d0)
        } else {
            rgb(0xdbeafe)
        })
        .text_xs()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x223047)))
        .child(label)
        .on_click(on_click)
}

fn connection_row(
    connection: &SavedConnection,
    on_connect: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let can_connect = matches!(
        connection.config,
        ConnectionType::Ssh { .. }
            | ConnectionType::LocalTerminal { .. }
            | ConnectionType::Telnet { .. }
            | ConnectionType::Serial { .. }
    );
    let action = if can_connect {
        small_button(format!("connect-{}", connection.id), "Connect", on_connect).into_any_element()
    } else {
        status_pill("porting", rgb(0xfbbf24), rgb(0x3a2f14)).into_any_element()
    };

    div()
        .flex()
        .items_center()
        .justify_between()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x151923))
        .p_3()
        .hover(|this| this.bg(rgb(0x1c2230)))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight(700.))
                        .child(connection.name.clone()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x98a3b8))
                        .child(connection.endpoint()),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(status_pill(
                    connection.kind_label(),
                    rgb(0x93c5fd),
                    rgb(0x17253b),
                ))
                .child(action),
        )
}

fn process_row(
    process: &RemoteProcess,
    on_term: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_kill: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_nice_down: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_nice_up: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x151923))
        .p_3()
        .hover(|this| this.bg(rgb(0x1c2230)))
        .child(
            div()
                .w(px(74.))
                .flex_none()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight(700.))
                        .child(process.pid.to_string()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x98a3b8))
                        .child(format!("ppid {}", process.ppid)),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight(700.))
                        .child(process.command.clone()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x98a3b8))
                        .child(process.command_line.clone()),
                ),
        )
        .child(process_stat("CPU", format!("{:.1}%", process.cpu_percent)))
        .child(process_stat(
            "MEM",
            format!("{:.1}%", process.memory_percent),
        ))
        .child(process_stat(
            "RSS",
            format_file_size(Some(process.rss_kb.saturating_mul(1024))),
        ))
        .child(process_stat("USER", process.user.clone()))
        .child(process_stat("STATE", process.state.clone()))
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(small_button(
                    format!("process-term-{}", process.pid),
                    "TERM",
                    on_term,
                ))
                .child(small_button(
                    format!("process-kill-{}", process.pid),
                    "KILL",
                    on_kill,
                ))
                .child(small_button(
                    format!("process-nice-down-{}", process.pid),
                    "Nice -5",
                    on_nice_down,
                ))
                .child(small_button(
                    format!("process-nice-up-{}", process.pid),
                    "Nice +5",
                    on_nice_up,
                )),
        )
}

fn process_stat(label: &'static str, value: impl Into<SharedString>) -> impl IntoElement {
    div()
        .w(px(70.))
        .flex_none()
        .flex()
        .flex_col()
        .gap_1()
        .child(div().text_xs().text_color(rgb(0x64748b)).child(label))
        .child(
            div()
                .text_xs()
                .text_color(rgb(0xcbd5e1))
                .child(value.into()),
        )
}

fn tunnel_row(
    tunnel: &TunnelConfig,
    connection_label: String,
    open_info: Option<SshTunnelInfo>,
    pending: bool,
    on_open: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let supported = tunnel_mode(tunnel).is_some();
    let is_open = open_info.is_some();
    let status = if pending {
        "pending"
    } else if is_open {
        "open"
    } else if supported {
        "closed"
    } else {
        "porting"
    };
    let status_color = if pending {
        rgb(0xfacc15)
    } else if is_open {
        rgb(0x6ee7b7)
    } else if supported {
        rgb(0x93c5fd)
    } else {
        rgb(0xfbbf24)
    };
    let status_bg = if pending {
        rgb(0x3a2f14)
    } else if is_open {
        rgb(0x12342a)
    } else if supported {
        rgb(0x17253b)
    } else {
        rgb(0x3a2f14)
    };
    let action = if pending {
        status_pill("pending", rgb(0xfacc15), rgb(0x3a2f14)).into_any_element()
    } else if is_open {
        small_button(format!("close-tunnel-{}", tunnel.id), "Close", on_close).into_any_element()
    } else if supported {
        small_button(format!("open-tunnel-{}", tunnel.id), "Open", on_open).into_any_element()
    } else {
        status_pill("porting", rgb(0xfbbf24), rgb(0x3a2f14)).into_any_element()
    };
    let listen = open_info
        .as_ref()
        .map(|info| format!("{}:{}", info.bind_host, info.listen_port))
        .unwrap_or_else(|| {
            format!(
                "{}:{}",
                if tunnel.bind_localhost {
                    "127.0.0.1"
                } else {
                    "0.0.0.0"
                },
                tunnel.listen_port
            )
        });

    div()
        .flex()
        .items_center()
        .justify_between()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x151923))
        .p_3()
        .hover(|this| this.bg(rgb(0x1c2230)))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight(700.))
                        .child(tunnel_name(tunnel)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x98a3b8))
                        .child(tunnel_endpoint(tunnel, &listen)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x98a3b8))
                        .child(connection_label),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(status_pill(
                    tunnel_mode_label(tunnel),
                    rgb(0x93c5fd),
                    rgb(0x17253b),
                ))
                .child(status_pill(status, status_color, status_bg))
                .child(action),
        )
}

fn capability_line(label: &'static str, value: impl Into<SharedString>) -> impl IntoElement {
    div()
        .mt_2()
        .flex()
        .items_center()
        .justify_between()
        .text_sm()
        .child(div().text_color(rgb(0xcbd5e1)).child(label))
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x98a3b8))
                .child(value.into()),
        )
}

fn small_button(
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x303848))
        .bg(rgb(0x151b27))
        .text_color(rgb(0xdbeafe))
        .text_xs()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x223047)))
        .child(label)
        .on_click(on_click)
}

fn transfer_input(
    id: impl Into<String>,
    label: &'static str,
    value: String,
    active: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(SharedString::from(id.into()))
        .h(px(46.))
        .px_3()
        .py_2()
        .flex()
        .flex_col()
        .gap_1()
        .rounded_sm()
        .border_1()
        .border_color(if active { rgb(0x4b6f97) } else { rgb(0x303848) })
        .bg(rgb(0x0d1320))
        .cursor_pointer()
        .child(div().text_xs().text_color(rgb(0x8f98aa)).child(label))
        .child(
            div()
                .font_family("JetBrains Mono")
                .text_xs()
                .text_color(rgb(0xe5edf7))
                .child(value),
        )
}

fn terminal_key_bytes(event: &KeyDownEvent) -> Option<Vec<u8>> {
    let keystroke = &event.keystroke;
    if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.function {
        return None;
    }

    if keystroke.modifiers.control {
        return control_key_bytes(&keystroke.key);
    }

    match keystroke.key.as_str() {
        "enter" => return Some(b"\n".to_vec()),
        "backspace" => return Some(vec![0x7f]),
        "tab" => return Some(b"\t".to_vec()),
        "escape" => return Some(vec![0x1b]),
        "up" => return Some(b"\x1b[A".to_vec()),
        "down" => return Some(b"\x1b[B".to_vec()),
        "right" => return Some(b"\x1b[C".to_vec()),
        "left" => return Some(b"\x1b[D".to_vec()),
        "home" => return Some(b"\x1b[H".to_vec()),
        "end" => return Some(b"\x1b[F".to_vec()),
        "delete" => return Some(b"\x1b[3~".to_vec()),
        "pageup" => return Some(b"\x1b[5~".to_vec()),
        "pagedown" => return Some(b"\x1b[6~".to_vec()),
        _ => {}
    }

    keystroke
        .key_char
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(|value| value.as_bytes().to_vec())
}

fn control_key_bytes(key: &str) -> Option<Vec<u8>> {
    let byte = match key {
        "space" => 0x00,
        "left_bracket" | "[" => 0x1b,
        "backslash" | "\\" => 0x1c,
        "right_bracket" | "]" => 0x1d,
        "6" => 0x1e,
        "slash" | "/" => 0x1f,
        value if value.len() == 1 => {
            let byte = value.as_bytes()[0].to_ascii_lowercase();
            if byte.is_ascii_lowercase() {
                byte - b'a' + 1
            } else {
                return None;
            }
        }
        _ => return None,
    };
    Some(vec![byte])
}

fn trim_terminal_output(output: &mut String) {
    const MAX_BYTES: usize = 64 * 1024;
    if output.len() <= MAX_BYTES {
        return;
    }
    let drain_to = output
        .char_indices()
        .find_map(|(index, _)| (index >= output.len() - MAX_BYTES).then_some(index))
        .unwrap_or(0);
    output.drain(..drain_to);
}

fn initial_terminal_screen() -> TerminalScreen {
    let mut screen = TerminalScreen::default();
    screen.advance(INITIAL_TERMINAL_BANNER.as_bytes());
    screen
}

fn short_id(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

fn status_label(status: &str) -> &'static str {
    if status.starts_with("running") {
        "session running"
    } else if status.contains("failed") || status.contains("error") {
        "session attention"
    } else {
        "session ready"
    }
}

fn non_empty_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn split_shell_args(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_telnet_enter_mode(value: &str) -> TelnetEnterMode {
    match value {
        "crlf" => TelnetEnterMode::Crlf,
        "lf" => TelnetEnterMode::Lf,
        _ => TelnetEnterMode::Cr,
    }
}

fn download_file_name_from_remote_path(remote_path: &str) -> String {
    remote_path
        .trim()
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty() && *name != ".")
        .unwrap_or("nyaterm-download.bin")
        .to_string()
}

fn tunnel_mode(tunnel: &TunnelConfig) -> Option<SshTunnelMode> {
    match tunnel.tunnel_type.as_str() {
        "local" => Some(SshTunnelMode::Local),
        "remote" => Some(SshTunnelMode::Remote),
        "dynamic" => Some(SshTunnelMode::Dynamic),
        _ => None,
    }
}

fn tunnel_mode_label(tunnel: &TunnelConfig) -> &'static str {
    match tunnel.tunnel_type.as_str() {
        "local" => "Local",
        "remote" => "Remote",
        "dynamic" => "SOCKS5",
        _ => "Tunnel",
    }
}

fn tunnel_name(tunnel: &TunnelConfig) -> String {
    if tunnel.name.trim().is_empty() {
        tunnel.id.clone()
    } else {
        tunnel.name.clone()
    }
}

fn tunnel_endpoint(tunnel: &TunnelConfig, listen: &str) -> String {
    match tunnel.tunnel_type.as_str() {
        "dynamic" => format!("{listen} SOCKS5"),
        "remote" => format!(
            "remote {} -> {}:{}",
            tunnel.listen_port, tunnel.target_host, tunnel.target_port
        ),
        _ => format!("{listen} -> {}:{}", tunnel.target_host, tunnel.target_port),
    }
}

fn transfer_job_title(kind: &TransferJobKind) -> String {
    match kind {
        TransferJobKind::ListDir { remote_path } => format!("List {remote_path}"),
        TransferJobKind::Download {
            remote_path,
            local_path,
        } => format!("Download {remote_path} -> {}", local_path.display()),
        TransferJobKind::Upload {
            local_path,
            remote_path,
        } => format!("Upload {} -> {remote_path}", local_path.display()),
    }
}

fn transfer_status_label(status: TransferJobStatus) -> &'static str {
    match status {
        TransferJobStatus::Running => "Running",
        TransferJobStatus::Paused => "Paused",
        TransferJobStatus::Cancelling => "Cancelling",
        TransferJobStatus::Cancelled => "Cancelled",
        TransferJobStatus::Completed => "Done",
        TransferJobStatus::Failed => "Failed",
    }
}

fn transfer_progress_bar(progress: &SftpTransferProgress) -> gpui::AnyElement {
    let percent = progress
        .total_bytes
        .filter(|total| *total > 0)
        .map(|total| progress.bytes_transferred as f32 / total as f32)
        .unwrap_or(0.)
        .clamp(0., 1.);

    div()
        .mt_3()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .h(px(6.))
                .w_full()
                .overflow_hidden()
                .rounded_sm()
                .bg(rgb(0x242b38))
                .child(
                    div()
                        .h(px(6.))
                        .w(px(260. * percent))
                        .rounded_sm()
                        .bg(rgb(0x38bdf8)),
                ),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x98a3b8))
                .child(format_transfer_progress(progress)),
        )
        .into_any_element()
}

fn format_transfer_progress(progress: &SftpTransferProgress) -> String {
    let transferred = format_file_size(Some(progress.bytes_transferred));
    match progress.total_bytes.filter(|total| *total > 0) {
        Some(total) => {
            let percent = (progress.bytes_transferred as f64 / total as f64 * 100.).clamp(0., 100.);
            format!(
                "{transferred} / {} ({percent:.0}%)",
                format_file_size(Some(total))
            )
        }
        None => format!("{transferred} transferred"),
    }
}

fn entry_kind_label(file_type: SftpFileType) -> &'static str {
    match file_type {
        SftpFileType::Directory => "dir",
        SftpFileType::File => "file",
        SftpFileType::Symlink => "link",
        SftpFileType::Other => "node",
    }
}

fn format_file_size(size: Option<u64>) -> String {
    let Some(size) = size else {
        return String::new();
    };
    if size >= 1024 * 1024 {
        format!("{:.1} MiB", size as f64 / 1024. / 1024.)
    } else if size >= 1024 {
        format!("{:.1} KiB", size as f64 / 1024.)
    } else {
        format!("{size} B")
    }
}

fn configured_status(secret: &str) -> String {
    if secret.trim().is_empty() {
        "missing".to_string()
    } else {
        "configured".to_string()
    }
}

fn configured_pair_status(id: &str, secret: &str) -> String {
    if id.trim().is_empty() || secret.trim().is_empty() {
        "missing".to_string()
    } else {
        "configured".to_string()
    }
}

fn uuid_like_prompt_id(host_key: &SshHostKey) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    host_key.host_identifier.hash(&mut hasher);
    host_key.key_type.hash(&mut hasher);
    host_key.key_base64.hash(&mut hasher);
    format!("hk-{:016x}", hasher.finish())
}

fn credential_prompt_id(prompt: &SshCredentialPrompt) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    prompt.connection_name.hash(&mut hasher);
    prompt.host.hash(&mut hasher);
    prompt.port.hash(&mut hasher);
    prompt.username.hash(&mut hasher);
    prompt.kind.hash(&mut hasher);
    prompt.reason.hash(&mut hasher);
    prompt.attempt.hash(&mut hasher);
    prompt.prompt_text.hash(&mut hasher);
    prompt.echo.hash(&mut hasher);
    format!("cred-{:016x}", hasher.finish())
}

fn sftp_duplicate_prompt_id(request: &SftpDuplicateRequest) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    request.direction.hash(&mut hasher);
    request.source_path.hash(&mut hasher);
    request.target_path.hash(&mut hasher);
    request.is_directory.hash(&mut hasher);
    format!("sftp-dup-{:016x}", hasher.finish())
}

fn duplicate_decision_label(decision: SftpDuplicateDecision) -> &'static str {
    match decision {
        SftpDuplicateDecision::Overwrite => "overwrite",
        SftpDuplicateDecision::Skip => "skip",
        SftpDuplicateDecision::Rename => "rename",
    }
}

fn credential_prompt_target(prompt: &SshCredentialPrompt) -> String {
    format!(
        "{}@{}:{} (attempt {})",
        prompt.username, prompt.host, prompt.port, prompt.attempt
    )
}

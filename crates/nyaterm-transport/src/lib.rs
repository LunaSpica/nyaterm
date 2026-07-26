use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener as StdTcpListener, TcpStream};
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread::JoinHandle;
use std::time::Duration;

use portable_pty::{Child, CommandBuilder, MasterPty, PtySize, native_pty_system};
use russh::keys::{PrivateKeyWithHashAlg, PublicKeyBase64};
use russh::{ChannelMsg, Disconnect, MethodKind, client};
use russh_sftp::client::SftpSession;
use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::{mpsc as tokio_mpsc, oneshot};

mod recording;
mod session_types;
mod sftp;
mod x11;

pub use x11::{
    X11AuthRewriter, X11DisplayTarget, X11ForwardingConfig, effective_x11_display,
    prepare_x11_forwarding, resolve_x11_display_spec, resolve_x11_display_targets,
    rewrite_x11_auth_setup_packet,
};
use x11::{X11ChannelOpen, X11Forwarder, enable_x11_failed_message, spawn_x11_forwarder};
mod sftp_transfer_types;
mod trzsz;
mod zmodem;

pub use session_types::{
    SessionDrain, SessionDrainStats, SessionError, SessionEvent, SessionInfo, SessionKind,
    TerminalTransport,
};
pub use sftp_transfer_types::{
    SFTP_TRANSFER_DEFAULT_BUFFER_SIZE, SFTP_TRANSFER_MAX_BUFFER_SIZE, SFTP_TRANSFER_MAX_RETRIES,
    SFTP_TRANSFER_MIN_BUFFER_SIZE, SftpDuplicateDecision, SftpDuplicatePolicy,
    SftpDuplicateRequest, SftpDuplicateResolver, SftpTransferDirection, SftpTransferOptions,
    SftpTransferProgress, SftpTransferSummary,
};
pub use trzsz::{
    TrzszAction, TrzszConfig, TrzszDetectResult, TrzszDetector, TrzszDownloadEngine,
    TrzszDownloadError, TrzszDownloadEvent, TrzszDownloadStep, TrzszFilteredOutput, TrzszMode,
    TrzszOutputEvent, TrzszOutputScan, TrzszProtocolFilteredOutput, TrzszProtocolFrame,
    TrzszProtocolPayload, TrzszProtocolStream, TrzszTransferEvent, TrzszTransferPhase,
    TrzszTransferState, TrzszTrigger, TrzszUploadEngine, TrzszUploadEntry, TrzszUploadError,
    TrzszUploadEvent, TrzszUploadPayload, TrzszUploadSource, TrzszUploadStep,
    build_trzsz_action_frame, build_trzsz_config_frame, build_trzsz_integer_frame,
    build_trzsz_string_frame, parse_trzsz_action_frame, parse_trzsz_config_frame,
    parse_trzsz_json_frame, parse_trzsz_protocol_frame, trzsz_fail_response,
};
pub use zmodem::{
    ZmodemAction, ZmodemDetectResult, ZmodemDetector, ZmodemDirection, ZmodemEvent, ZmodemTransfer,
    start_zmodem_transfer,
};
mod stats;

mod docker;

pub use docker::{
    DOCKER_COMPOSE_PROJECTS_SCRIPT, DOCKER_IMAGES_SCRIPT, DOCKER_NETWORKS_SCRIPT,
    DOCKER_OVERVIEW_SCRIPT, DOCKER_VOLUMES_SCRIPT, DockerComposeProject, DockerComposeService,
    DockerComposeServiceContainer, DockerContainer, DockerContainerDetails, DockerContainerMount,
    DockerContainerNetwork, DockerContainerStats, DockerImage, DockerNetwork, DockerService,
    DockerVolume, RemoteDockerOverview, docker_container_details_script, parse_compose_projects,
    parse_compose_services_output, parse_docker_container_details_output,
    parse_docker_images_output, parse_docker_networks_output, parse_docker_overview_output,
    parse_docker_stats_output, parse_docker_volumes_output,
};
pub use recording::{
    DEFAULT_HISTORY_SEARCH_LIMIT, DEFAULT_HISTORY_SEARCH_LINES, DEFAULT_MEMORY_LIMIT_BYTES,
    MAX_HISTORY_SEARCH_LINES, RecordingError, RecordingManager, TerminalHistorySearchRequest,
    TerminalHistorySearchResponse, TerminalHistorySearchResult, safe_recording_name,
};
pub use stats::{
    CpuInfo, DiskInfo, LoadInfo, MemoryInfo, NetworkInfo, RemoteStats, RemoteStatsService,
    SYSINFO_SCRIPT, SystemInfo, parse_stats_output,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSessionConfig {
    pub name: String,
    pub shell_path: Option<String>,
    pub shell_args: Vec<String>,
    pub working_dir: Option<PathBuf>,
    pub cols: u16,
    pub rows: u16,
    /// Total terminal pixel width (cols * cell_width). Zero means unknown.
    pub pixel_width: u16,
    /// Total terminal pixel height (rows * cell_height). Zero means unknown.
    pub pixel_height: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TelnetEnterMode {
    Crlf,
    Cr,
    Lf,
}

impl Default for TelnetEnterMode {
    fn default() -> Self {
        Self::Cr
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelnetSessionConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub raw_tcp: bool,
    pub enter_mode: TelnetEnterMode,
    pub force_character_at_a_time: bool,
    pub send_naws: bool,
    pub send_sga: bool,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialSessionConfig {
    pub name: String,
    pub port_name: String,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub parity: String,
    pub stop_bits: String,
    pub backspace_mode: String,
}

#[derive(Clone)]
pub struct SshSessionConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub key_auth: Option<SshKeyAuthConfig>,
    pub otp_id: Option<String>,
    pub auto_fill_otp: bool,
    pub proxy_jump: Option<Box<SshSessionConfig>>,
    pub proxy: Option<SshProxyConfig>,
    pub allow_none_auth: bool,
    pub backspace_mode: String,
    pub term: String,
    pub x11_forwarding: bool,
    pub x11_display: String,
    pub deferred_pty: bool,
    /// Seconds between SSH keepalive packets. Zero disables keepalive.
    pub keep_alive_interval_secs: u32,
    pub cols: u16,
    pub rows: u16,
    /// Total terminal pixel width (cols * cell_width). Zero means unknown.
    pub pixel_width: u16,
    /// Total terminal pixel height (rows * cell_height). Zero means unknown.
    pub pixel_height: u16,
    pub host_key_verifier: Option<Arc<dyn SshHostKeyVerifier>>,
    pub credential_provider: Option<Arc<dyn SshCredentialProvider>>,
    pub otp_provider: Option<Arc<dyn SshOtpProvider>>,
}

#[derive(Clone, PartialEq, Eq)]
pub struct SshProxyConfig {
    pub protocol: String,
    pub host: String,
    pub port: u16,
    pub command: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl std::fmt::Debug for SshProxyConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SshProxyConfig")
            .field("protocol", &self.protocol)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("command", &self.command)
            .field("username", &self.username)
            .field("password", &self.password.as_ref().map(|_| "<redacted>"))
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshKeyAuthConfig {
    pub key_data: String,
    pub cert_data: Option<String>,
    pub passphrase: Option<String>,
}

impl std::fmt::Debug for SshSessionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SshSessionConfig")
            .field("name", &self.name)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field("password", &self.password.as_ref().map(|_| "<redacted>"))
            .field("key_auth", &self.key_auth.as_ref().map(|_| "<redacted>"))
            .field("otp_id", &self.otp_id)
            .field("auto_fill_otp", &self.auto_fill_otp)
            .field("proxy_jump", &self.proxy_jump.is_some())
            .field("proxy", &self.proxy)
            .field("allow_none_auth", &self.allow_none_auth)
            .field("backspace_mode", &self.backspace_mode)
            .field("term", &self.term)
            .field("x11_forwarding", &self.x11_forwarding)
            .field("x11_display", &self.x11_display)
            .field("deferred_pty", &self.deferred_pty)
            .field("keep_alive_interval_secs", &self.keep_alive_interval_secs)
            .field("cols", &self.cols)
            .field("rows", &self.rows)
            .field("pixel_width", &self.pixel_width)
            .field("pixel_height", &self.pixel_height)
            .field("host_key_verifier", &self.host_key_verifier.is_some())
            .field("credential_provider", &self.credential_provider.is_some())
            .field("otp_provider", &self.otp_provider.is_some())
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshHostKey {
    pub host: String,
    pub port: u16,
    pub host_identifier: String,
    pub key_type: String,
    pub key_base64: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SshHostKeyDecision {
    Accept,
    Reject(String),
}

pub trait SshHostKeyVerifier: Send + Sync {
    fn verify(&self, host_key: &SshHostKey) -> Result<SshHostKeyDecision, String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SshCredentialPromptKind {
    Password,
    KeyPassphrase,
    KeyboardInteractive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SshCredentialPromptReason {
    MissingPassword,
    PasswordRejected,
    KeyPassphraseRequired,
    KeyboardInteractive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshCredentialPrompt {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub connection_name: String,
    pub kind: SshCredentialPromptKind,
    pub reason: SshCredentialPromptReason,
    pub attempt: u32,
    pub prompt_text: Option<String>,
    pub echo: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SshKeyboardInteractivePrompt {
    pub prompt: String,
    pub echo: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SshKeyboardInteractiveRequest {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub connection_name: String,
    pub name: String,
    pub instructions: String,
    pub round: u32,
    pub prompts: Vec<SshKeyboardInteractivePrompt>,
    pub otp_id: Option<String>,
}

pub trait SshCredentialProvider: Send + Sync {
    fn request_secret(&self, prompt: &SshCredentialPrompt) -> Result<Option<String>, String>;

    fn request_keyboard_interactive(
        &self,
        request: &SshKeyboardInteractiveRequest,
    ) -> Result<Option<Vec<String>>, String> {
        let prompt_count = request.prompts.len();
        let mut responses = Vec::with_capacity(prompt_count);
        for (index, prompt) in request.prompts.iter().enumerate() {
            let response = self.request_secret(&SshCredentialPrompt {
                host: request.host.clone(),
                port: request.port,
                username: request.username.clone(),
                connection_name: request.connection_name.clone(),
                kind: SshCredentialPromptKind::KeyboardInteractive,
                reason: SshCredentialPromptReason::KeyboardInteractive,
                attempt: request.round,
                prompt_text: Some(format_keyboard_interactive_prompt(
                    &request.name,
                    &request.instructions,
                    &prompt.prompt,
                    index,
                    prompt_count,
                )),
                echo: prompt.echo,
            })?;
            let Some(response) = response else {
                return Ok(None);
            };
            responses.push(response);
        }
        Ok(Some(responses))
    }
}

pub trait SshOtpProvider: Send + Sync {
    fn request_otp_code(&self, otp_id: &str) -> Result<Option<String>, String>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SshTunnelMode {
    Local,
    Remote,
    Dynamic,
}

#[derive(Clone)]
pub struct SshTunnelConfig {
    pub id: String,
    pub ssh_config: SshSessionConfig,
    pub mode: SshTunnelMode,
    pub bind_host: String,
    pub listen_port: u16,
    pub target_host: Option<String>,
    pub target_port: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshMultiplexInfo {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub proxy: Option<SshProxyConfig>,
    pub jump_count: usize,
}

#[derive(Clone)]
pub struct SshMultiplexHandle {
    inner: Arc<SshMultiplexInner>,
}

type SharedSshHandle = Arc<tokio::sync::Mutex<client::Handle<SshClientHandler>>>;
type ForwardedTcpIpRegistry = Arc<tokio::sync::Mutex<ForwardedTcpIpDispatch>>;

struct SshMultiplexInner {
    runtime: Arc<tokio::runtime::Runtime>,
    target: SharedSshHandle,
    jumps: Vec<SharedSshHandle>,
    info: SshMultiplexInfo,
    forwarded_tcpip: ForwardedTcpIpRegistry,
    closed: AtomicBool,
}

#[derive(Default)]
struct ForwardedTcpIpDispatch {
    fallback: Option<tokio_mpsc::UnboundedSender<ForwardedTcpIpChannel>>,
    by_listener: HashMap<(String, u32), tokio_mpsc::UnboundedSender<ForwardedTcpIpChannel>>,
}

impl std::fmt::Debug for SshMultiplexHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SshMultiplexHandle")
            .field("info", &self.inner.info)
            .field("closed", &self.is_closed())
            .finish()
    }
}

impl SshMultiplexHandle {
    pub fn info(&self) -> SshMultiplexInfo {
        self.inner.info.clone()
    }

    pub fn jump_count(&self) -> usize {
        self.inner.info.jump_count
    }

    pub fn is_closed(&self) -> bool {
        self.inner.closed.load(Ordering::Relaxed)
    }

    pub fn matches_config(&self, config: &SshSessionConfig) -> bool {
        self.inner.info.host == config.host
            && self.inner.info.port == config.port
            && self.inner.info.username == config.username
            && self.inner.info.proxy == config.proxy
    }

    pub fn ensure_matches_config(&self, config: &SshSessionConfig) -> anyhow::Result<()> {
        if self.matches_config(config) {
            return Ok(());
        }
        let info = &self.inner.info;
        anyhow::bail!(
            "SSH multiplex handle targets {}@{}:{}, but operation targets {}@{}:{}",
            info.username,
            info.host,
            info.port,
            config.username,
            config.host,
            config.port
        )
    }

    pub fn disconnect(&self) -> anyhow::Result<()> {
        if self.inner.closed.swap(true, Ordering::Relaxed) {
            return Ok(());
        }
        let target = self.inner.target.clone();
        let jumps = self.inner.jumps.clone();
        self.inner.runtime.block_on(async move {
            let _ = target
                .lock()
                .await
                .disconnect(Disconnect::ByApplication, "ssh multiplex closed", "en")
                .await;
            for jump in jumps {
                let _ = jump
                    .lock()
                    .await
                    .disconnect(Disconnect::ByApplication, "ssh multiplex closed", "en")
                    .await;
            }
            Ok(())
        })
    }

    fn target_handle(&self) -> SharedSshHandle {
        self.inner.target.clone()
    }

    fn forwarded_tcpip_registry(&self) -> ForwardedTcpIpRegistry {
        self.inner.forwarded_tcpip.clone()
    }

    fn block_on<T, F>(&self, operation: F) -> anyhow::Result<T>
    where
        F: Future<Output = anyhow::Result<T>> + Send + 'static,
        T: Send + 'static,
    {
        if self.is_closed() {
            anyhow::bail!("SSH multiplex handle is closed");
        }
        self.inner.runtime.block_on(operation)
    }
}

fn forwarded_tcpip_sender_for(
    dispatch: &ForwardedTcpIpDispatch,
    connected_address: &str,
    connected_port: u32,
) -> Option<tokio_mpsc::UnboundedSender<ForwardedTcpIpChannel>> {
    dispatch
        .by_listener
        .get(&(connected_address.to_string(), connected_port))
        .or(dispatch.fallback.as_ref())
        .cloned()
}

pub fn open_ssh_multiplex_handle(config: SshSessionConfig) -> anyhow::Result<SshMultiplexHandle> {
    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("nyaterm-ssh-multiplex")
            .build()
            .map_err(|error| anyhow::anyhow!("failed to start SSH multiplex runtime: {error}"))?,
    );
    let forwarded_tcpip = Arc::new(tokio::sync::Mutex::new(ForwardedTcpIpDispatch::default()));
    let (target, jumps) = runtime.block_on(open_authenticated_ssh_handle_with_sender_registry(
        &config,
        Some(forwarded_tcpip.clone()),
        None,
    ))?;
    let info = SshMultiplexInfo {
        name: config.name,
        host: config.host,
        port: config.port,
        username: config.username,
        proxy: config.proxy,
        jump_count: jumps.len(),
    };
    Ok(SshMultiplexHandle {
        inner: Arc::new(SshMultiplexInner {
            runtime,
            target: Arc::new(tokio::sync::Mutex::new(target)),
            jumps: jumps
                .into_iter()
                .map(|jump| Arc::new(tokio::sync::Mutex::new(jump)))
                .collect(),
            info,
            forwarded_tcpip,
            closed: AtomicBool::new(false),
        }),
    })
}

impl std::fmt::Debug for SshTunnelConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SshTunnelConfig")
            .field("id", &self.id)
            .field("ssh_config", &self.ssh_config)
            .field("mode", &self.mode)
            .field("bind_host", &self.bind_host)
            .field("listen_port", &self.listen_port)
            .field("target_host", &self.target_host)
            .field("target_port", &self.target_port)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshTunnelInfo {
    pub id: String,
    pub mode: SshTunnelMode,
    pub bind_host: String,
    pub listen_port: u16,
    pub target_host: Option<String>,
    pub target_port: Option<u16>,
}

#[derive(Debug, Default)]
pub struct SshTunnelManager {
    active: Mutex<HashMap<String, SshTunnelHandle>>,
}

#[derive(Debug)]
struct SshTunnelHandle {
    info: SshTunnelInfo,
    shutdown_tx: Option<oneshot::Sender<()>>,
    worker_thread: Option<JoinHandle<()>>,
}

struct ForwardedTcpIpChannel {
    channel: russh::Channel<client::Msg>,
    connected_address: String,
    connected_port: u32,
    originator_address: String,
    originator_port: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SftpFileType {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SftpFileEntry {
    pub name: String,
    pub path: String,
    pub file_type: SftpFileType,
    pub size: Option<u64>,
    pub permissions: Option<u32>,
    pub owner: String,
    pub group: String,
    pub modified_at: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SftpFileProperties {
    pub name: String,
    pub path: String,
    pub file_type: SftpFileType,
    pub size: Option<u64>,
    pub permissions: Option<u32>,
    pub permissions_symbolic: String,
    pub owner: String,
    pub group: String,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub modified_at: Option<u32>,
    pub accessed_at: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SftpAttributeUpdate {
    pub mode: Option<u32>,
    pub owner: Option<String>,
    pub group: Option<String>,
    pub recursive: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SftpRemoteTextFile {
    pub path: String,
    pub content: String,
    pub size: u64,
    pub modified_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SftpWriteTextResult {
    Saved { modified_at: u64, size: u64 },
    Conflict { modified_at: u64, size: u64 },
}

#[derive(Debug, Clone, Default)]
pub struct SftpTransferControl {
    cancelled: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
}

impl SftpTransferControl {
    pub fn new() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
        self.paused.store(false, Ordering::Relaxed);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    pub fn pause(&self) {
        if !self.is_cancelled() {
            self.paused.store(true, Ordering::Relaxed);
        }
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::Relaxed);
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn check_cancelled(&self) -> anyhow::Result<()> {
        if self.is_cancelled() {
            anyhow::bail!(SFTP_TRANSFER_CANCELLED);
        }
        Ok(())
    }

    async fn wait_if_paused(&self) -> anyhow::Result<()> {
        self.check_cancelled()?;
        while self.is_paused() {
            tokio::time::sleep(Duration::from_millis(100)).await;
            self.check_cancelled()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SftpService {
    config: SshSessionConfig,
    multiplex: Option<SshMultiplexHandle>,
}

pub const SFTP_TRANSFER_CANCELLED: &str = "SFTP transfer cancelled";
pub const PROCESS_LIST_UNSUPPORTED_MARKER: &str = "NYATERM_PROCESS_UNSUPPORTED";
pub const PROCESS_LIST_UNSUPPORTED_ERROR: &str =
    "Process listing is unsupported on this remote host";
const PROCESS_TIMEOUT: Duration = Duration::from_secs(10);
const MIT_MAGIC_COOKIE: &str = "MIT-MAGIC-COOKIE-1";
const XAUTH_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, PartialEq)]
pub struct RemoteProcess {
    pub pid: u32,
    pub ppid: u32,
    pub user: String,
    pub state: String,
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub rss_kb: u64,
    pub vsz_kb: u64,
    pub elapsed: String,
    pub command: String,
    pub command_line: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteCommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_status: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct SshProcessService {
    config: SshSessionConfig,
    multiplex: Option<SshMultiplexHandle>,
}

pub const PROCESS_LIST_SCRIPT: &str = r#"sh -s <<'NYATERM_PROCESS_SCRIPT'
LC_ALL=C
export LC_ALL

unsupported() {
  echo "NYATERM_PROCESS_UNSUPPORTED"
  exit 42
}

clean() {
  printf "%s" "$1" | tr "\011\012\015" "   "
}

emit() {
  pid=$(clean "$1")
  ppid=$(clean "$2")
  user=$(clean "$3")
  stat=$(clean "$4")
  cpu=$(clean "$5")
  mem=$(clean "$6")
  rss=$(clean "$7")
  vsz=$(clean "$8")
  etime=$(clean "$9")
  comm=$(clean "${10}")
  args=$(clean "${11}")

  [ -n "$pid" ] || return 0
  [ -n "$ppid" ] || ppid=0
  [ -n "$user" ] || user=-
  [ -n "$stat" ] || stat=-
  [ -n "$cpu" ] || cpu=0
  [ -n "$mem" ] || mem=0
  [ -n "$rss" ] || rss=0
  [ -n "$vsz" ] || vsz=0
  [ -n "$etime" ] || etime=-
  [ -n "$comm" ] || comm=-
  [ -n "$args" ] || args=$comm

  printf "PROCESS\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n" \
    "$pid" "$ppid" "$user" "$stat" "$cpu" "$mem" "$rss" "$vsz" "$etime" "$comm" "$args"
}

parse_ps_full() {
  awk '
  function clean(value) { gsub(/[\t\r\n]/, " ", value); return value }
  NF >= 10 && $1 ~ /^[0-9]+$/ {
    args = ""
    for (i = 11; i <= NF; i++) args = args (args == "" ? "" : " ") $i
    if (args == "") args = $10
    printf "PROCESS\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n", \
      clean($1), clean($2), clean($3), clean($4), clean($5), clean($6), \
      clean($7), clean($8), clean($9), clean($10), clean(args)
  }'
}

parse_ps_basic() {
  awk '
  function clean(value) { gsub(/[\t\r\n]/, " ", value); return value }
  NR == 1 && toupper($1) == "PID" { next }
  NF >= 6 && $1 ~ /^[0-9]+$/ {
    args = ""
    for (i = 7; i <= NF; i++) args = args (args == "" ? "" : " ") $i
    if (args == "") args = $6
    printf "PROCESS\t%s\t%s\t%s\t%s\t0\t0\t0\t%s\t-\t%s\t%s\n", \
      clean($1), clean($2), clean($3), clean($4), clean($5), clean($6), clean(args)
  }'
}

parse_ps_minimal() {
  awk '
  function clean(value) { gsub(/[\t\r\n]/, " ", value); return value }
  NR == 1 && toupper($1) == "PID" { next }
  $1 ~ /^[0-9]+$/ {
    pid = $1; ppid = 0; user = "-"; stat = "-"; vsz = 0; start = 2
    if ($2 ~ /^[0-9]+$/) {
      ppid = $2; start = 3
      if (NF >= 3 && $3 !~ /^[0-9]+$/) { user = $3; start = 4 }
    } else if (NF >= 2) {
      user = $2; start = 3
    }
    if (NF >= start && $(start) ~ /^[0-9]+$/ && NF >= start + 1 && $(start + 1) ~ /^[A-Za-z]/) {
      vsz = $(start); stat = $(start + 1); start += 2
    } else if (NF >= start && $(start) ~ /^[A-Za-z][A-Za-z+<NsSlL]*$/) {
      stat = $(start); start += 1
    }
    args = ""
    for (i = start; i <= NF; i++) args = args (args == "" ? "" : " ") $i
    if (args == "") args = "-"
    comm = args; sub(/[ ].*$/, "", comm)
    printf "PROCESS\t%s\t%s\t%s\t%s\t0\t0\t0\t%s\t-\t%s\t%s\n", \
      clean(pid), clean(ppid), clean(user), clean(stat), clean(vsz), clean(comm), clean(args)
  }'
}

emit_proc() {
  [ -d /proc ] || return 1
  found=0
  mem_total=$(awk '/^MemTotal:/ { print $2; exit }' /proc/meminfo 2>/dev/null)
  [ -n "$mem_total" ] || mem_total=0

  for proc_dir in /proc/[0-9]*; do
    [ -r "$proc_dir/status" ] || continue
    pid=${proc_dir##*/}
    case "$pid" in *[!0-9]*|"") continue ;; esac
    status=$(awk '
      /^Name:/ { name=$2 }
      /^State:/ { state=$2 }
      /^PPid:/ { ppid=$2 }
      /^Uid:/ { uid=$2 }
      /^VmRSS:/ { rss=$2 }
      /^VmSize:/ { vsz=$2 }
      END {
        if (name == "") name="-"; if (state == "") state="-"; if (ppid == "") ppid=0
        if (uid == "") uid=0; if (rss == "") rss=0; if (vsz == "") vsz=0
        printf "%s\t%s\t%s\t%s\t%s\t%s\n", name, state, ppid, uid, rss, vsz
      }' "$proc_dir/status" 2>/dev/null)
    [ -n "$status" ] || continue
    old_ifs=$IFS; IFS="	"; set -- $status; IFS=$old_ifs
    comm=$1; stat=$2; ppid=$3; uid=$4; rss=$5; vsz=$6; user=$uid
    if [ -r /etc/passwd ]; then
      resolved_user=$(awk -F: -v uid="$uid" '$3 == uid { print $1; exit }' /etc/passwd 2>/dev/null)
      [ -n "$resolved_user" ] && user=$resolved_user
    fi
    if [ -r "$proc_dir/cmdline" ]; then
      args=$(tr "\000" " " <"$proc_dir/cmdline" 2>/dev/null)
    else
      args=
    fi
    [ -n "$args" ] || args=$comm
    mem=$(awk -v rss="$rss" -v total="$mem_total" 'BEGIN { if (total > 0) printf "%.1f", (rss * 100) / total; else printf "0"; }')
    emit "$pid" "$ppid" "$user" "$stat" "0" "$mem" "$rss" "$vsz" "-" "$comm" "$args"
    found=1
  done
  [ "$found" -eq 1 ]
}

if command -v ps >/dev/null 2>&1; then
  rows=$(ps -eo pid=,ppid=,user=,stat=,pcpu=,pmem=,rss=,vsz=,etime=,comm=,args= --no-headers 2>/dev/null | parse_ps_full)
  [ -n "$rows" ] && { printf "%s\n" "$rows"; exit 0; }
  rows=$(ps -axo pid=,ppid=,user=,stat=,pcpu=,pmem=,rss=,vsz=,etime=,comm=,command= 2>/dev/null | parse_ps_full)
  [ -n "$rows" ] && { printf "%s\n" "$rows"; exit 0; }
  rows=$(ps -o pid,ppid,user,stat,vsz,comm,args 2>/dev/null | parse_ps_basic)
  [ -n "$rows" ] && { printf "%s\n" "$rows"; exit 0; }
  rows=$(ps w 2>/dev/null | parse_ps_minimal)
  [ -n "$rows" ] && { printf "%s\n" "$rows"; exit 0; }
  rows=$(ps 2>/dev/null | parse_ps_minimal)
  [ -n "$rows" ] && { printf "%s\n" "$rows"; exit 0; }
fi

if emit_proc; then
  exit 0
fi

unsupported
NYATERM_PROCESS_SCRIPT
"#;

impl Default for SerialSessionConfig {
    fn default() -> Self {
        Self {
            name: "Serial".to_string(),
            port_name: String::new(),
            baud_rate: 115_200,
            data_bits: 8,
            parity: "none".to_string(),
            stop_bits: "1".to_string(),
            backspace_mode: "ctrl_h".to_string(),
        }
    }
}

impl Default for SshSessionConfig {
    fn default() -> Self {
        Self {
            name: "SSH".to_string(),
            host: String::new(),
            port: 22,
            username: "root".to_string(),
            password: None,
            key_auth: None,
            otp_id: None,
            auto_fill_otp: false,
            proxy_jump: None,
            proxy: None,
            allow_none_auth: false,
            backspace_mode: "del".to_string(),
            term: "xterm-256color".to_string(),
            x11_forwarding: false,
            x11_display: String::new(),
            deferred_pty: false,
            keep_alive_interval_secs: 30,
            cols: 80,
            rows: 24,
            pixel_width: 0,
            pixel_height: 0,
            host_key_verifier: None,
            credential_provider: None,
            otp_provider: None,
        }
    }
}

impl SshTunnelManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&self, config: SshTunnelConfig) -> anyhow::Result<SshTunnelInfo> {
        self.open_inner(config, None)
    }

    pub fn open_with_multiplex(
        &self,
        config: SshTunnelConfig,
        multiplex: SshMultiplexHandle,
    ) -> anyhow::Result<SshTunnelInfo> {
        multiplex.ensure_matches_config(&config.ssh_config)?;
        self.open_inner(config, Some(multiplex))
    }

    fn open_inner(
        &self,
        config: SshTunnelConfig,
        multiplex: Option<SshMultiplexHandle>,
    ) -> anyhow::Result<SshTunnelInfo> {
        if let Some(info) = self
            .active
            .lock()
            .map_err(|_| anyhow::anyhow!("SSH tunnel registry lock is poisoned"))?
            .get(&config.id)
            .map(|handle| handle.info.clone())
        {
            return Ok(info);
        }

        validate_tunnel_config(&config)?;
        let bind_host = normalized_bind_host(&config.bind_host);
        let (listener, actual_port) = match config.mode {
            SshTunnelMode::Local | SshTunnelMode::Dynamic => {
                let listener = StdTcpListener::bind((bind_host.as_str(), config.listen_port))
                    .map_err(|error| {
                        anyhow::anyhow!(
                            "failed to bind tunnel listener {}:{}: {error}",
                            bind_host,
                            config.listen_port
                        )
                    })?;
                listener.set_nonblocking(true)?;
                let actual_port = listener.local_addr()?.port();
                (Some(listener), actual_port)
            }
            SshTunnelMode::Remote => (None, config.listen_port),
        };
        let info = SshTunnelInfo {
            id: config.id.clone(),
            mode: config.mode,
            bind_host,
            listen_port: actual_port,
            target_host: config.target_host.clone(),
            target_port: config.target_port,
        };
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let worker_info = info.clone();
        let worker_thread = std::thread::spawn(move || {
            run_tunnel_worker(
                config,
                listener,
                worker_info,
                shutdown_rx,
                ready_tx,
                multiplex,
            );
        });

        match ready_rx.recv_timeout(Duration::from_secs(30)) {
            Ok(Ok(info)) => {
                self.active
                    .lock()
                    .map_err(|_| anyhow::anyhow!("SSH tunnel registry lock is poisoned"))?
                    .insert(
                        info.id.clone(),
                        SshTunnelHandle {
                            info: info.clone(),
                            shutdown_tx: Some(shutdown_tx),
                            worker_thread: Some(worker_thread),
                        },
                    );
                Ok(info)
            }
            Ok(Err(error)) => {
                let _ = shutdown_tx.send(());
                let _ = worker_thread.join();
                Err(anyhow::anyhow!(error))
            }
            Err(error) => {
                let _ = shutdown_tx.send(());
                let _ = worker_thread.join();
                Err(anyhow::anyhow!("SSH tunnel startup timed out: {error}"))
            }
        }
    }

    pub fn close(&self, tunnel_id: &str) -> anyhow::Result<()> {
        let Some(mut handle) = self
            .active
            .lock()
            .map_err(|_| anyhow::anyhow!("SSH tunnel registry lock is poisoned"))?
            .remove(tunnel_id)
        else {
            return Ok(());
        };
        if let Some(shutdown_tx) = handle.shutdown_tx.take() {
            let _ = shutdown_tx.send(());
        }
        if let Some(worker_thread) = handle.worker_thread.take() {
            let _ = worker_thread.join();
        }
        Ok(())
    }

    pub fn is_open(&self, tunnel_id: &str) -> anyhow::Result<bool> {
        Ok(self
            .active
            .lock()
            .map_err(|_| anyhow::anyhow!("SSH tunnel registry lock is poisoned"))?
            .contains_key(tunnel_id))
    }

    pub fn list(&self) -> anyhow::Result<Vec<SshTunnelInfo>> {
        Ok(self
            .active
            .lock()
            .map_err(|_| anyhow::anyhow!("SSH tunnel registry lock is poisoned"))?
            .values()
            .map(|handle| handle.info.clone())
            .collect())
    }
}

impl Default for TelnetSessionConfig {
    fn default() -> Self {
        Self {
            name: "Telnet".to_string(),
            host: String::new(),
            port: 23,
            raw_tcp: false,
            enter_mode: TelnetEnterMode::Cr,
            force_character_at_a_time: false,
            send_naws: true,
            send_sga: true,
            cols: 80,
            rows: 24,
        }
    }
}

impl SshProcessService {
    pub fn new(config: SshSessionConfig) -> Self {
        Self {
            config,
            multiplex: None,
        }
    }

    pub fn with_multiplex(
        config: SshSessionConfig,
        multiplex: SshMultiplexHandle,
    ) -> anyhow::Result<Self> {
        multiplex.ensure_matches_config(&config)?;
        Ok(Self {
            config,
            multiplex: Some(multiplex),
        })
    }

    fn exec_command_bytes(
        &self,
        command: Vec<u8>,
        timeout: Duration,
    ) -> anyhow::Result<RemoteCommandOutput> {
        if let Some(multiplex) = self.multiplex.clone() {
            return multiplex.block_on(exec_ssh_command_with_multiplex(
                multiplex.clone(),
                command,
                timeout,
            ));
        }
        run_ssh_exec_operation(exec_ssh_command(self.config.clone(), command, timeout))
    }

    pub fn list_processes(&self) -> anyhow::Result<Vec<RemoteProcess>> {
        let output =
            self.exec_command_bytes(PROCESS_LIST_SCRIPT.as_bytes().to_vec(), PROCESS_TIMEOUT)?;
        if is_process_list_unsupported(&output.stdout)
            || is_process_list_unsupported(&output.stderr)
        {
            anyhow::bail!(PROCESS_LIST_UNSUPPORTED_ERROR);
        }
        let output = ensure_remote_command_success(output, "Failed to list processes")?;
        if is_process_list_unsupported(&output.stdout)
            || is_process_list_unsupported(&output.stderr)
        {
            anyhow::bail!(PROCESS_LIST_UNSUPPORTED_ERROR);
        }
        Ok(parse_process_output(&output.stdout))
    }

    pub fn signal_process(
        &self,
        pid: u32,
        signal: impl AsRef<str>,
    ) -> anyhow::Result<RemoteCommandOutput> {
        let signal = normalize_process_signal(signal.as_ref())?;
        let output = self.exec_command_bytes(
            format!("kill -{signal} -- {pid}").into_bytes(),
            PROCESS_TIMEOUT,
        )?;
        ensure_remote_command_success(output, "Failed to signal process")
    }

    pub fn renice_process(&self, pid: u32, nice: i32) -> anyhow::Result<RemoteCommandOutput> {
        if !(-20..=19).contains(&nice) {
            anyhow::bail!("Nice value must be between -20 and 19");
        }
        let output = self.exec_command_bytes(
            format!("renice -n {nice} -p {pid}").into_bytes(),
            PROCESS_TIMEOUT,
        )?;
        ensure_remote_command_success(output, "Failed to renice process")
    }

    pub fn run_command(
        &self,
        command: impl AsRef<str>,
        timeout: Duration,
    ) -> anyhow::Result<RemoteCommandOutput> {
        self.exec_command_bytes(command.as_ref().as_bytes().to_vec(), timeout)
    }
}

pub fn run_local_command(
    command: impl AsRef<str>,
    cwd: Option<PathBuf>,
    timeout: Duration,
) -> anyhow::Result<RemoteCommandOutput> {
    let command = command.as_ref().to_string();
    run_ssh_exec_operation(async move {
        tokio::time::timeout(timeout, async move {
            let mut child = local_shell_command(&command);
            if let Some(cwd) = cwd.filter(|value| !value.as_os_str().is_empty()) {
                child.current_dir(cwd);
            }
            child.kill_on_drop(true);
            child.stdout(Stdio::piped());
            child.stderr(Stdio::piped());
            let output = child
                .output()
                .await
                .map_err(|error| anyhow::anyhow!("failed to run local command: {error}"))?;
            Ok(RemoteCommandOutput {
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                exit_status: output
                    .status
                    .code()
                    .and_then(|code| u32::try_from(code).ok()),
            })
        })
        .await
        .map_err(|_| anyhow::anyhow!("local command timed out"))?
    })
}

#[cfg(windows)]
fn local_shell_command(command: &str) -> tokio::process::Command {
    let mut child = tokio::process::Command::new("cmd");
    child.args(["/C", command]);
    child
}

#[cfg(not(windows))]
fn local_shell_command(command: &str) -> tokio::process::Command {
    let mut child = tokio::process::Command::new("sh");
    child.args(["-lc", command]);
    child
}

impl Default for LocalSessionConfig {
    fn default() -> Self {
        Self {
            name: "Local Terminal".to_string(),
            shell_path: None,
            shell_args: Vec::new(),
            working_dir: None,
            cols: 80,
            rows: 24,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}

pub struct SessionManager {
    sessions: Mutex<HashMap<String, ManagedSession>>,
    event_queue: SessionEventQueue,
}

const SESSION_EVENT_QUEUE_OUTPUT_LIMIT: usize = 8 * 1024 * 1024;
const SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT: usize = 256 * 1024;

#[derive(Clone)]
struct SessionEventQueue {
    inner: Arc<Mutex<SessionEventQueueInner>>,
}

#[derive(Default)]
struct SessionEventQueueInner {
    events: VecDeque<SessionEvent>,
    queued_output_bytes: usize,
}

impl SessionEventQueue {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SessionEventQueueInner::default())),
        }
    }

    fn push(&self, event: SessionEvent) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.push(event);
    }

    fn drain(&self, max_events: usize) -> SessionDrain {
        self.drain_with_output_budget(max_events, None)
    }

    fn drain_with_output_budget(
        &self,
        max_events: usize,
        max_output_bytes: Option<usize>,
    ) -> SessionDrain {
        let Ok(mut inner) = self.inner.lock() else {
            return SessionDrain::default();
        };
        inner.drain(max_events, max_output_bytes)
    }
}

impl SessionEventQueueInner {
    fn push(&mut self, event: SessionEvent) {
        match event {
            SessionEvent::Output {
                session_id,
                mut data,
            } => {
                if data.is_empty() {
                    return;
                }
                let mut leading_drop = 0usize;
                if data.len() > SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT {
                    leading_drop = data.len() - SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT;
                    data.drain(..leading_drop);
                }
                if leading_drop > 0 {
                    self.push_output_drop_event(session_id.clone(), leading_drop);
                }
                self.queued_output_bytes = self.queued_output_bytes.saturating_add(data.len());
                self.events.push_back(SessionEvent::Output {
                    session_id: session_id.clone(),
                    data,
                });
                self.enforce_output_limit();
            }
            other => self.events.push_back(other),
        }
    }

    fn push_output_drop_event(&mut self, session_id: String, bytes: usize) {
        if bytes == 0 {
            return;
        }
        if let Some(SessionEvent::OutputDropped {
            session_id: last_session_id,
            bytes: last_bytes,
        }) = self.events.back_mut()
            && last_session_id == &session_id
        {
            *last_bytes = last_bytes.saturating_add(bytes);
            return;
        }
        self.events
            .push_back(SessionEvent::OutputDropped { session_id, bytes });
    }

    fn insert_output_drop_event(&mut self, index: usize, session_id: String, bytes: usize) {
        if bytes == 0 {
            return;
        }
        if index > 0
            && let Some(SessionEvent::OutputDropped {
                session_id: previous_session_id,
                bytes: previous_bytes,
            }) = self.events.get_mut(index - 1)
            && previous_session_id == &session_id
        {
            *previous_bytes = previous_bytes.saturating_add(bytes);
            return;
        }
        let index = index.min(self.events.len());
        self.events
            .insert(index, SessionEvent::OutputDropped { session_id, bytes });
    }

    fn enforce_output_limit(&mut self) {
        while self.queued_output_bytes > SESSION_EVENT_QUEUE_OUTPUT_LIMIT {
            let excess = self.queued_output_bytes - SESSION_EVENT_QUEUE_OUTPUT_LIMIT;
            let Some(index) = self
                .events
                .iter()
                .position(|event| matches!(event, SessionEvent::Output { .. }))
            else {
                self.queued_output_bytes = 0;
                break;
            };
            let mut remove_event = false;
            let mut dropped: Option<(String, usize)> = None;
            if let Some(SessionEvent::Output { session_id, data }) = self.events.get_mut(index) {
                let remove = excess.min(data.len());
                let dropped_session_id = session_id.clone();
                data.drain(..remove);
                self.queued_output_bytes = self.queued_output_bytes.saturating_sub(remove);
                remove_event = data.is_empty();
                dropped = Some((dropped_session_id, remove));
            }
            if let Some((session_id, bytes)) = dropped {
                if remove_event {
                    self.events.remove(index);
                    self.insert_output_drop_event(index, session_id, bytes);
                } else {
                    self.insert_output_drop_event(index, session_id, bytes);
                }
            } else if remove_event {
                self.events.remove(index);
            }
        }
    }

    fn drain(&mut self, max_events: usize, max_output_bytes: Option<usize>) -> SessionDrain {
        let mut events = Vec::new();
        let mut stats = SessionDrainStats::default();
        for _ in 0..max_events {
            if let Some(max_output_bytes) = max_output_bytes {
                if stats.drained_output_bytes >= max_output_bytes && stats.drained_events > 0 {
                    break;
                }
                let remaining_output_budget =
                    max_output_bytes.saturating_sub(stats.drained_output_bytes);
                if remaining_output_budget == 0 && stats.drained_events > 0 {
                    break;
                }
                if remaining_output_budget == 0
                    && matches!(self.events.front(), Some(SessionEvent::Output { .. }))
                {
                    break;
                }
                if let Some(SessionEvent::Output { session_id, data }) = self.events.front_mut() {
                    let take = data.len().min(remaining_output_budget);
                    if data.len() > take {
                        let remaining = data.split_off(take);
                        let chunk = std::mem::replace(data, remaining);
                        let session_id = session_id.clone();
                        stats.drained_events = stats.drained_events.saturating_add(1);
                        stats.drained_output_bytes =
                            stats.drained_output_bytes.saturating_add(chunk.len());
                        self.queued_output_bytes =
                            self.queued_output_bytes.saturating_sub(chunk.len());
                        events.push(SessionEvent::Output {
                            session_id,
                            data: chunk,
                        });
                        continue;
                    }
                }
            }
            let Some(event) = self.events.pop_front() else {
                break;
            };
            stats.drained_events = stats.drained_events.saturating_add(1);
            match &event {
                SessionEvent::Output { data, .. } => {
                    stats.drained_output_bytes =
                        stats.drained_output_bytes.saturating_add(data.len());
                    self.queued_output_bytes = self.queued_output_bytes.saturating_sub(data.len());
                }
                SessionEvent::OutputDropped { bytes, .. } => {
                    stats.dropped_output_bytes = stats.dropped_output_bytes.saturating_add(*bytes);
                }
                SessionEvent::Exited { .. } | SessionEvent::Error { .. } => {}
            }
            events.push(event);
        }
        stats.queued_events = self.events.len();
        stats.queued_output_bytes = self.queued_output_bytes;
        SessionDrain { events, stats }
    }
}

enum ManagedSession {
    Local(LocalPtyTransport),
    Ssh(SshChannelTransport),
    Tcp(TelnetTransport),
    Serial(SerialTransport),
}

pub struct LocalPtyTransport {
    info: SessionInfo,
    master: Box<dyn MasterPty + Send>,
    writer: QueuedTransportWriter,
    child: Box<dyn Child + Send + Sync>,
    reader_thread: Option<JoinHandle<()>>,
}

pub struct TelnetTransport {
    info: SessionInfo,
    writer: QueuedTransportWriter,
    reader_stream: TcpStream,
    config: TelnetSessionConfig,
    reader_thread: Option<JoinHandle<()>>,
}

pub struct SshChannelTransport {
    info: SessionInfo,
    command_tx: tokio_mpsc::UnboundedSender<SshCommand>,
    backspace_as_bs: bool,
    worker_thread: Option<JoinHandle<()>>,
}

pub struct SerialTransport {
    info: SessionInfo,
    writer: QueuedTransportWriter,
    backspace_as_bs: bool,
    stop_reader: Arc<AtomicBool>,
    reader_thread: Option<JoinHandle<()>>,
}

struct QueuedTransportWriter {
    command_tx: mpsc::Sender<TransportWriterCommand>,
    worker_thread: Option<JoinHandle<()>>,
}

enum TransportWriterCommand {
    Write(Vec<u8>),
    Close,
}

impl QueuedTransportWriter {
    fn spawn<W>(
        session_id: String,
        writer: W,
        flush_each_byte: bool,
        event_queue: SessionEventQueue,
    ) -> Self
    where
        W: Write + Send + 'static,
    {
        let (command_tx, command_rx) = mpsc::channel();
        let worker_thread = std::thread::spawn(move || {
            run_transport_writer(session_id, writer, flush_each_byte, command_rx, event_queue)
        });
        Self {
            command_tx,
            worker_thread: Some(worker_thread),
        }
    }

    fn write(&self, data: Vec<u8>) -> anyhow::Result<()> {
        if data.is_empty() {
            return Ok(());
        }
        self.command_tx
            .send(TransportWriterCommand::Write(data))
            .map_err(|_| anyhow::anyhow!("transport writer stopped"))
    }

    fn close(&mut self) {
        let _ = self.command_tx.send(TransportWriterCommand::Close);
        if let Some(worker_thread) = self.worker_thread.take() {
            let _ = worker_thread.join();
        }
    }
}

fn run_transport_writer<W>(
    session_id: String,
    mut writer: W,
    flush_each_byte: bool,
    command_rx: mpsc::Receiver<TransportWriterCommand>,
    event_queue: SessionEventQueue,
) where
    W: Write,
{
    while let Ok(command) = command_rx.recv() {
        match command {
            TransportWriterCommand::Write(data) => {
                let write_result = if flush_each_byte {
                    data.iter().try_for_each(|byte| {
                        writer
                            .write_all(std::slice::from_ref(byte))
                            .and_then(|_| writer.flush())
                    })
                } else {
                    writer.write_all(&data).and_then(|_| writer.flush())
                };
                if let Err(error) = write_result {
                    send_session_error(&event_queue, &session_id, error);
                    break;
                }
            }
            TransportWriterCommand::Close => break,
        }
    }
}

enum SshCommand {
    Write(Vec<u8>),
    Resize {
        cols: u16,
        rows: u16,
        pixel_width: u16,
        pixel_height: u16,
    },
    Close,
}

struct OpenSshShellSession {
    handle: Option<client::Handle<SshClientHandler>>,
    channel: russh::Channel<client::Msg>,
    jump_handles: Vec<client::Handle<SshClientHandler>>,
    disconnect_on_close: bool,
    x11_forwarder: Option<X11Forwarder>,
    local_notice: Option<Vec<u8>>,
}

enum SshShellHandle {
    Dedicated(client::Handle<SshClientHandler>),
    Multiplexed(SharedSshHandle),
}

struct PendingOpenSshShellSession {
    handle: SshShellHandle,
    jump_handles: Vec<client::Handle<SshClientHandler>>,
    disconnect_on_close: bool,
    x11_config: Option<X11ForwardingConfig>,
    x11_rx: Option<tokio_mpsc::UnboundedReceiver<X11ChannelOpen>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SshPtyDimensions {
    cols: u16,
    rows: u16,
    pixel_width: u16,
    pixel_height: u16,
}

fn local_pty_size(cols: u16, rows: u16, pixel_width: u16, pixel_height: u16) -> PtySize {
    PtySize {
        rows,
        cols,
        pixel_width,
        pixel_height,
    }
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            event_queue: SessionEventQueue::new(),
        }
    }

    pub fn create_local_session(
        &self,
        config: LocalSessionConfig,
    ) -> Result<SessionInfo, SessionError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(local_pty_size(
                config.cols,
                config.rows,
                config.pixel_width,
                config.pixel_height,
            ))
            .map_err(SessionError::OpenPty)?;

        let mut command = build_command(&config);
        configure_environment(&mut command);
        if let Some(working_dir) = &config.working_dir {
            command.cwd(working_dir);
        }

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(SessionError::CloneReader)?;
        let writer = pair
            .master
            .take_writer()
            .map_err(SessionError::TakeWriter)?;
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(SessionError::Spawn)?;
        drop(pair.slave);

        let info = SessionInfo {
            id: session_id.clone(),
            name: config.name,
            kind: SessionKind::LocalPty,
            working_dir: config.working_dir.clone(),
            cols: config.cols,
            rows: config.rows,
        };
        let reader_thread =
            spawn_reader_thread(session_id.clone(), reader, self.event_queue.clone());
        let writer = QueuedTransportWriter::spawn(
            session_id.clone(),
            writer,
            false,
            self.event_queue.clone(),
        );
        let session = LocalPtyTransport {
            info: info.clone(),
            master: pair.master,
            writer,
            child,
            reader_thread: Some(reader_thread),
        };

        self.sessions
            .lock()
            .map_err(|_| SessionError::LockPoisoned)?
            .insert(session_id, ManagedSession::Local(session));

        Ok(info)
    }

    pub fn create_telnet_session(
        &self,
        config: TelnetSessionConfig,
    ) -> Result<SessionInfo, SessionError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let addr = format!("{}:{}", config.host, config.port);
        let stream = TcpStream::connect(&addr).map_err(|source| SessionError::ConnectTcp {
            addr: addr.clone(),
            source,
        })?;
        stream.set_nodelay(true).ok();
        stream
            .set_read_timeout(Some(Duration::from_millis(250)))
            .ok();

        let mut writer = stream
            .try_clone()
            .map_err(|source| SessionError::CloneTcp {
                session_id: session_id.clone(),
                source,
            })?;
        let response_writer = stream
            .try_clone()
            .map_err(|source| SessionError::CloneTcp {
                session_id: session_id.clone(),
                source,
            })?;

        if let Some(naws) = maybe_build_naws(config.cols, config.rows, &config) {
            writer.write_all(&naws).ok();
            writer.flush().ok();
        }

        let info = SessionInfo {
            id: session_id.clone(),
            name: config.name.clone(),
            kind: if config.raw_tcp {
                SessionKind::RawTcp
            } else {
                SessionKind::Telnet
            },
            working_dir: None,
            cols: config.cols,
            rows: config.rows,
        };

        let reader_thread = spawn_tcp_reader_thread(
            session_id.clone(),
            stream
                .try_clone()
                .map_err(|source| SessionError::CloneTcp {
                    session_id: session_id.clone(),
                    source,
                })?,
            response_writer,
            config.clone(),
            self.event_queue.clone(),
        );
        let writer = QueuedTransportWriter::spawn(
            session_id.clone(),
            writer,
            config.force_character_at_a_time,
            self.event_queue.clone(),
        );

        let session = TelnetTransport {
            info: info.clone(),
            writer,
            reader_stream: stream,
            config,
            reader_thread: Some(reader_thread),
        };

        self.sessions
            .lock()
            .map_err(|_| SessionError::LockPoisoned)?
            .insert(session_id, ManagedSession::Tcp(session));

        Ok(info)
    }

    pub fn create_ssh_session(
        &self,
        config: SshSessionConfig,
    ) -> Result<SessionInfo, SessionError> {
        self.create_ssh_session_inner(config, None)
    }

    pub fn create_ssh_session_with_multiplex(
        &self,
        config: SshSessionConfig,
        multiplex: SshMultiplexHandle,
    ) -> Result<SessionInfo, SessionError> {
        multiplex
            .ensure_matches_config(&config)
            .map_err(|source| SessionError::CreateSsh {
                addr: format!("{}:{}", config.host, config.port),
                source,
            })?;
        self.create_ssh_session_inner(config, Some(multiplex))
    }

    fn create_ssh_session_inner(
        &self,
        config: SshSessionConfig,
        multiplex: Option<SshMultiplexHandle>,
    ) -> Result<SessionInfo, SessionError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let addr = format!("{}:{}", config.host, config.port);
        let (command_tx, command_rx) = tokio_mpsc::unbounded_channel();
        let (ready_tx, ready_rx) = mpsc::channel();
        let event_queue = self.event_queue.clone();
        let worker_config = config.clone();
        let worker_session_id = session_id.clone();
        let worker_thread = std::thread::spawn(move || {
            run_ssh_worker(
                worker_session_id,
                worker_config,
                command_rx,
                ready_tx,
                event_queue,
                multiplex,
            );
        });

        match ready_rx.recv() {
            Ok(Ok(())) => {}
            Ok(Err(message)) => {
                let _ = worker_thread.join();
                return Err(SessionError::CreateSsh {
                    addr,
                    source: anyhow::anyhow!(message),
                });
            }
            Err(error) => {
                let _ = worker_thread.join();
                return Err(SessionError::CreateSsh {
                    addr,
                    source: anyhow::anyhow!("SSH worker exited before readiness: {error}"),
                });
            }
        }

        let info = SessionInfo {
            id: session_id.clone(),
            name: config.name,
            kind: SessionKind::Ssh,
            working_dir: None,
            cols: config.cols,
            rows: config.rows,
        };
        let session = SshChannelTransport {
            info: info.clone(),
            command_tx,
            backspace_as_bs: config.backspace_mode == "ctrl_h",
            worker_thread: Some(worker_thread),
        };

        self.sessions
            .lock()
            .map_err(|_| SessionError::LockPoisoned)?
            .insert(session_id, ManagedSession::Ssh(session));

        Ok(info)
    }

    pub fn create_serial_session(
        &self,
        config: SerialSessionConfig,
    ) -> Result<SessionInfo, SessionError> {
        let session_id = uuid::Uuid::new_v4().to_string();
        let port = open_serial_port(&config).map_err(|source| SessionError::OpenSerial {
            port_name: config.port_name.clone(),
            source,
        })?;
        let reader = port
            .try_clone()
            .map_err(|source| SessionError::CloneSerial {
                session_id: session_id.clone(),
                source,
            })?;

        let info = SessionInfo {
            id: session_id.clone(),
            name: config.name,
            kind: SessionKind::Serial,
            working_dir: None,
            cols: 80,
            rows: 24,
        };
        let stop_reader = Arc::new(AtomicBool::new(false));
        let reader_thread = spawn_serial_reader_thread(
            session_id.clone(),
            reader,
            stop_reader.clone(),
            self.event_queue.clone(),
        );
        let writer =
            QueuedTransportWriter::spawn(session_id.clone(), port, false, self.event_queue.clone());
        let session = SerialTransport {
            info: info.clone(),
            writer,
            backspace_as_bs: config.backspace_mode == "ctrl_h",
            stop_reader,
            reader_thread: Some(reader_thread),
        };

        self.sessions
            .lock()
            .map_err(|_| SessionError::LockPoisoned)?
            .insert(session_id, ManagedSession::Serial(session));

        Ok(info)
    }

    pub fn list_serial_ports(&self) -> Result<Vec<String>, SessionError> {
        let mut ports = serialport::available_ports()
            .map_err(|source| SessionError::OpenSerial {
                port_name: "<list>".to_string(),
                source,
            })?
            .into_iter()
            .map(|port| port.port_name)
            .collect::<Vec<_>>();
        ports.sort_unstable();
        Ok(ports)
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionInfo>, SessionError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionError::LockPoisoned)?
            .values()
            .map(ManagedSession::info)
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
        Ok(sessions)
    }

    pub fn write(&self, session_id: &str, data: &[u8]) -> Result<(), SessionError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionError::LockPoisoned)?;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;
        session.write(data).map_err(|source| SessionError::Write {
            session_id: session_id.to_string(),
            source,
        })
    }

    pub fn resize(&self, session_id: &str, cols: u16, rows: u16) -> Result<(), SessionError> {
        self.resize_with_pixels(session_id, cols, rows, 0, 0)
    }

    /// Resize the live session, including total pixel dimensions when known.
    /// Pixel size is used by local PTY masters and SSH `window-change` / `request_pty`.
    pub fn resize_with_pixels(
        &self,
        session_id: &str,
        cols: u16,
        rows: u16,
        pixel_width: u16,
        pixel_height: u16,
    ) -> Result<(), SessionError> {
        let mut sessions = self
            .sessions
            .lock()
            .map_err(|_| SessionError::LockPoisoned)?;
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;
        session
            .resize(cols, rows, pixel_width, pixel_height)
            .map_err(|source| SessionError::Resize {
                session_id: session_id.to_string(),
                source,
            })
    }

    pub fn close(&self, session_id: &str) -> Result<(), SessionError> {
        let mut session = self
            .sessions
            .lock()
            .map_err(|_| SessionError::LockPoisoned)?
            .remove(session_id)
            .ok_or_else(|| SessionError::NotFound(session_id.to_string()))?;
        session.close();
        Ok(())
    }

    pub fn try_recv_event(&self) -> Result<Option<SessionEvent>, SessionError> {
        Ok(self.event_queue.drain(1).events.into_iter().next())
    }

    pub fn drain_events(&self, max_events: usize) -> Result<SessionDrain, SessionError> {
        Ok(self.event_queue.drain(max_events))
    }

    pub fn drain_events_with_output_budget(
        &self,
        max_events: usize,
        max_output_bytes: usize,
    ) -> Result<SessionDrain, SessionError> {
        Ok(self
            .event_queue
            .drain_with_output_budget(max_events, Some(max_output_bytes)))
    }
}

impl ManagedSession {
    fn info(&self) -> SessionInfo {
        match self {
            Self::Local(session) => session.info.clone(),
            Self::Ssh(session) => session.info.clone(),
            Self::Tcp(session) => session.info.clone(),
            Self::Serial(session) => session.info.clone(),
        }
    }

    fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        match self {
            Self::Local(session) => session.write(data),
            Self::Tcp(session) => session.write(data),
            Self::Ssh(session) => session.write(data),
            Self::Serial(session) => session.write(data),
        }
    }

    fn resize(
        &mut self,
        cols: u16,
        rows: u16,
        pixel_width: u16,
        pixel_height: u16,
    ) -> anyhow::Result<()> {
        match self {
            Self::Local(session) => session.resize(cols, rows, pixel_width, pixel_height),
            Self::Tcp(session) => session.resize(cols, rows, pixel_width, pixel_height),
            Self::Ssh(session) => session.resize(cols, rows, pixel_width, pixel_height),
            Self::Serial(session) => session.resize(cols, rows, pixel_width, pixel_height),
        }
    }

    fn close(&mut self) {
        match self {
            Self::Local(session) => {
                let _ = session.close();
            }
            Self::Tcp(session) => {
                let _ = session.close();
            }
            Self::Ssh(session) => {
                let _ = session.close();
            }
            Self::Serial(session) => {
                let _ = session.close();
            }
        }
    }
}

impl TerminalTransport for LocalPtyTransport {
    fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        self.writer.write(data.to_vec())
    }

    fn resize(
        &mut self,
        cols: u16,
        rows: u16,
        pixel_width: u16,
        pixel_height: u16,
    ) -> anyhow::Result<()> {
        self.master
            .resize(local_pty_size(cols, rows, pixel_width, pixel_height))?;
        self.info.cols = cols;
        self.info.rows = rows;
        Ok(())
    }

    fn close(&mut self) -> anyhow::Result<()> {
        let _ = self.child.kill();
        self.writer.close();
        if let Some(reader_thread) = self.reader_thread.take() {
            let _ = reader_thread.join();
        }
        Ok(())
    }
}

impl TerminalTransport for TelnetTransport {
    fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        let data = normalize_telnet_input(data, &self.config);
        self.writer.write(data)
    }

    fn resize(
        &mut self,
        cols: u16,
        rows: u16,
        _pixel_width: u16,
        _pixel_height: u16,
    ) -> anyhow::Result<()> {
        self.info.cols = cols;
        self.info.rows = rows;
        if let Some(naws) = maybe_build_naws(cols, rows, &self.config) {
            self.writer.write(naws)?;
        }
        Ok(())
    }

    fn close(&mut self) -> anyhow::Result<()> {
        let _ = self.reader_stream.shutdown(Shutdown::Both);
        self.writer.close();
        if let Some(reader_thread) = self.reader_thread.take() {
            let _ = reader_thread.join();
        }
        Ok(())
    }
}

impl TerminalTransport for SshChannelTransport {
    fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        let data = if self.backspace_as_bs {
            remap_del_to_bs(data)
        } else {
            data.to_vec()
        };
        self.command_tx
            .send(SshCommand::Write(data))
            .map_err(|_| anyhow::anyhow!("SSH worker stopped"))?;
        Ok(())
    }

    fn resize(
        &mut self,
        cols: u16,
        rows: u16,
        pixel_width: u16,
        pixel_height: u16,
    ) -> anyhow::Result<()> {
        self.info.cols = cols;
        self.info.rows = rows;
        self.command_tx
            .send(SshCommand::Resize {
                cols,
                rows,
                pixel_width,
                pixel_height,
            })
            .map_err(|_| anyhow::anyhow!("SSH worker stopped"))?;
        Ok(())
    }

    fn close(&mut self) -> anyhow::Result<()> {
        let _ = self.command_tx.send(SshCommand::Close);
        if let Some(worker_thread) = self.worker_thread.take() {
            let _ = worker_thread.join();
        }
        Ok(())
    }
}

impl TerminalTransport for SerialTransport {
    fn write(&mut self, data: &[u8]) -> anyhow::Result<()> {
        let data = if self.backspace_as_bs {
            remap_del_to_bs(data)
        } else {
            data.to_vec()
        };
        self.writer.write(data)
    }

    fn resize(
        &mut self,
        cols: u16,
        rows: u16,
        _pixel_width: u16,
        _pixel_height: u16,
    ) -> anyhow::Result<()> {
        self.info.cols = cols;
        self.info.rows = rows;
        Ok(())
    }

    fn close(&mut self) -> anyhow::Result<()> {
        self.stop_reader.store(true, Ordering::Relaxed);
        self.writer.close();
        if let Some(reader_thread) = self.reader_thread.take() {
            let _ = reader_thread.join();
        }
        Ok(())
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

fn spawn_reader_thread(
    session_id: String,
    mut reader: Box<dyn Read + Send>,
    event_queue: SessionEventQueue,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    event_queue.push(SessionEvent::Exited {
                        session_id: session_id.clone(),
                        reason: "reader reached EOF".to_string(),
                    });
                    break;
                }
                Ok(read) => {
                    event_queue.push(SessionEvent::Output {
                        session_id: session_id.clone(),
                        data: buffer[..read].to_vec(),
                    });
                }
                Err(error) => {
                    event_queue.push(SessionEvent::Error {
                        session_id: session_id.clone(),
                        message: error.to_string(),
                    });
                    break;
                }
            }
        }
    })
}

fn spawn_tcp_reader_thread(
    session_id: String,
    mut reader: TcpStream,
    mut response_writer: TcpStream,
    config: TelnetSessionConfig,
    event_queue: SessionEventQueue,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buffer = [0_u8; 8192];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => {
                    event_queue.push(SessionEvent::Exited {
                        session_id: session_id.clone(),
                        reason: if config.raw_tcp {
                            "raw TCP peer closed connection".to_string()
                        } else {
                            "telnet peer closed connection".to_string()
                        },
                    });
                    break;
                }
                Ok(read) => {
                    let visible = if config.raw_tcp {
                        unescape_iac_iac(&buffer[..read])
                    } else {
                        strip_telnet_commands(&buffer[..read], &mut |command, option| {
                            let response = negotiate_response(
                                command,
                                option,
                                config.send_naws,
                                config.send_sga,
                            );
                            if !response.is_empty() {
                                let _ = response_writer.write_all(&response);
                                let _ = response_writer.flush();
                            }
                        })
                    };
                    if !visible.is_empty() {
                        event_queue.push(SessionEvent::Output {
                            session_id: session_id.clone(),
                            data: visible,
                        });
                    }
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    continue;
                }
                Err(error) => {
                    event_queue.push(SessionEvent::Error {
                        session_id: session_id.clone(),
                        message: error.to_string(),
                    });
                    break;
                }
            }
        }
    })
}

fn run_ssh_worker(
    session_id: String,
    config: SshSessionConfig,
    command_rx: tokio_mpsc::UnboundedReceiver<SshCommand>,
    ready_tx: mpsc::Sender<Result<(), String>>,
    event_queue: SessionEventQueue,
    multiplex: Option<SshMultiplexHandle>,
) {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("nyaterm-ssh")
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = ready_tx.send(Err(format!("failed to start SSH runtime: {error}")));
            return;
        }
    };

    runtime.block_on(async move {
        if config.deferred_pty {
            run_deferred_ssh_worker(
                session_id,
                config,
                command_rx,
                ready_tx,
                event_queue,
                multiplex,
            )
            .await;
            return;
        }

        let open_session = match open_ssh_shell(&config, multiplex.as_ref()).await {
            Ok(session) => {
                let _ = ready_tx.send(Ok(()));
                session
            }
            Err(error) => {
                let _ = ready_tx.send(Err(error.to_string()));
                return;
            }
        };
        run_open_ssh_shell_session(
            session_id,
            open_session,
            command_rx,
            event_queue,
            VecDeque::new(),
        )
        .await;
    });
}

async fn run_deferred_ssh_worker(
    session_id: String,
    config: SshSessionConfig,
    mut command_rx: tokio_mpsc::UnboundedReceiver<SshCommand>,
    ready_tx: mpsc::Sender<Result<(), String>>,
    event_queue: SessionEventQueue,
    multiplex: Option<SshMultiplexHandle>,
) {
    let pending_session = match open_pending_ssh_shell(&config, multiplex.as_ref()).await {
        Ok(session) => {
            let _ = ready_tx.send(Ok(()));
            session
        }
        Err(error) => {
            let _ = ready_tx.send(Err(error.to_string()));
            return;
        }
    };
    let mut pending_session = Some(pending_session);
    let mut dimensions = SshPtyDimensions::from_config(&config);
    let mut pending_writes = VecDeque::new();
    let mut fallback = Box::pin(tokio::time::sleep(Duration::from_millis(750)));

    loop {
        tokio::select! {
            command = command_rx.recv() => {
                match command {
                    Some(SshCommand::Write(data)) => {
                        pending_writes.push_back(data);
                    }
                    Some(SshCommand::Resize {
                        cols,
                        rows,
                        pixel_width,
                        pixel_height,
                    }) => {
                        dimensions = SshPtyDimensions::new(cols, rows, pixel_width, pixel_height);
                        break;
                    }
                    Some(SshCommand::Close) | None => {
                        if let Some(session) = pending_session.take() {
                            disconnect_pending_ssh_shell(session).await;
                        }
                        return;
                    }
                }
            }
            _ = &mut fallback => {
                break;
            }
        }
    }

    let Some(pending_session) = pending_session.take() else {
        return;
    };
    if drain_deferred_ssh_open_commands(&mut command_rx, &mut dimensions, &mut pending_writes) {
        disconnect_pending_ssh_shell(pending_session).await;
        return;
    }
    match open_ssh_shell_from_pending(&config, pending_session, dimensions).await {
        Ok(open_session) => {
            run_open_ssh_shell_session(
                session_id,
                open_session,
                command_rx,
                event_queue,
                pending_writes,
            )
            .await;
        }
        Err(error) => {
            send_session_error(&event_queue, &session_id, error);
        }
    }
}

fn drain_deferred_ssh_open_commands(
    command_rx: &mut tokio_mpsc::UnboundedReceiver<SshCommand>,
    dimensions: &mut SshPtyDimensions,
    pending_writes: &mut VecDeque<Vec<u8>>,
) -> bool {
    loop {
        match command_rx.try_recv() {
            Ok(SshCommand::Write(data)) => {
                pending_writes.push_back(data);
            }
            Ok(SshCommand::Resize {
                cols,
                rows,
                pixel_width,
                pixel_height,
            }) => {
                *dimensions = SshPtyDimensions::new(cols, rows, pixel_width, pixel_height);
            }
            Ok(SshCommand::Close) => return true,
            Err(tokio_mpsc::error::TryRecvError::Empty) => return false,
            Err(tokio_mpsc::error::TryRecvError::Disconnected) => return true,
        }
    }
}

async fn run_open_ssh_shell_session(
    session_id: String,
    open_session: OpenSshShellSession,
    mut command_rx: tokio_mpsc::UnboundedReceiver<SshCommand>,
    event_queue: SessionEventQueue,
    mut pending_writes: VecDeque<Vec<u8>>,
) {
    let OpenSshShellSession {
        handle,
        mut channel,
        jump_handles,
        disconnect_on_close,
        x11_forwarder,
        local_notice,
    } = open_session;
    if let Some(notice) = local_notice {
        event_queue.push(SessionEvent::Output {
            session_id: session_id.clone(),
            data: notice,
        });
    }
    if let Some(forwarder) = x11_forwarder {
        spawn_x11_forwarder(event_queue.clone(), session_id.clone(), forwarder);
    }

    while let Some(data) = pending_writes.pop_front() {
        if let Err(error) = channel.data_bytes(data).await {
            send_session_error(&event_queue, &session_id, error);
            disconnect_open_ssh_shell(handle, jump_handles, disconnect_on_close).await;
            return;
        }
    }

    loop {
        tokio::select! {
            command = command_rx.recv() => {
                match command {
                    Some(SshCommand::Write(data)) => {
                        if let Err(error) = channel.data_bytes(data).await {
                            send_session_error(&event_queue, &session_id, error);
                            break;
                        }
                    }
                    Some(SshCommand::Resize {
                        cols,
                        rows,
                        pixel_width,
                        pixel_height,
                    }) => {
                        if let Err(error) = channel
                            .window_change(
                                cols.into(),
                                rows.into(),
                                pixel_width.into(),
                                pixel_height.into(),
                            )
                            .await
                        {
                            send_session_error(&event_queue, &session_id, error);
                            break;
                        }
                    }
                    Some(SshCommand::Close) | None => {
                        let _ = channel.eof().await;
                        let _ = channel.close().await;
                        break;
                    }
                }
            }
            message = channel.wait() => {
                match message {
                    Some(ChannelMsg::Data { data }) | Some(ChannelMsg::ExtendedData { data, .. }) => {
                        event_queue.push(SessionEvent::Output {
                            session_id: session_id.clone(),
                            data: data.to_vec(),
                        });
                    }
                    Some(ChannelMsg::ExitStatus { exit_status }) => {
                        event_queue.push(SessionEvent::Exited {
                            session_id: session_id.clone(),
                            reason: format!("SSH channel exit status {exit_status}"),
                        });
                        break;
                    }
                    Some(ChannelMsg::Eof) => {
                        event_queue.push(SessionEvent::Exited {
                            session_id: session_id.clone(),
                            reason: "SSH channel EOF".to_string(),
                        });
                        break;
                    }
                    Some(ChannelMsg::Close) => {
                        event_queue.push(SessionEvent::Exited {
                            session_id: session_id.clone(),
                            reason: "SSH channel closed by remote".to_string(),
                        });
                        break;
                    }
                    None => {
                        event_queue.push(SessionEvent::Exited {
                            session_id: session_id.clone(),
                            reason: "SSH connection task ended".to_string(),
                        });
                        break;
                    }
                    Some(_) => {}
                }
            }
        }
    }

    disconnect_open_ssh_shell(handle, jump_handles, disconnect_on_close).await;
}

async fn open_ssh_shell(
    config: &SshSessionConfig,
    multiplex: Option<&SshMultiplexHandle>,
) -> anyhow::Result<OpenSshShellSession> {
    let pending = open_pending_ssh_shell(config, multiplex).await?;
    open_ssh_shell_from_pending(config, pending, SshPtyDimensions::from_config(config)).await
}

async fn open_pending_ssh_shell(
    config: &SshSessionConfig,
    multiplex: Option<&SshMultiplexHandle>,
) -> anyhow::Result<PendingOpenSshShellSession> {
    let x11_config = if config.x11_forwarding {
        Some(prepare_x11_forwarding(&config.x11_display).await)
    } else {
        None
    };
    let (x11_tx, x11_rx) = if x11_config.is_some() {
        let (tx, rx) = tokio_mpsc::unbounded_channel();
        (Some(tx), Some(rx))
    } else {
        (None, None)
    };
    let (handle, jump_handles, disconnect_on_close) = if let Some(multiplex) = multiplex {
        multiplex.ensure_matches_config(config)?;
        if x11_tx.is_some() {
            anyhow::bail!("X11 forwarding is not supported for multiplexed SSH shell sessions");
        }
        let handle = multiplex.target_handle();
        (SshShellHandle::Multiplexed(handle), Vec::new(), false)
    } else {
        let (handle, jump_handles) =
            open_authenticated_ssh_handle_with_channel_senders(config, None, x11_tx).await?;
        (SshShellHandle::Dedicated(handle), jump_handles, true)
    };

    Ok(PendingOpenSshShellSession {
        handle,
        jump_handles,
        disconnect_on_close,
        x11_config,
        x11_rx,
    })
}

async fn open_ssh_shell_from_pending(
    config: &SshSessionConfig,
    pending: PendingOpenSshShellSession,
    dimensions: SshPtyDimensions,
) -> anyhow::Result<OpenSshShellSession> {
    let PendingOpenSshShellSession {
        mut handle,
        jump_handles,
        disconnect_on_close,
        x11_config,
        x11_rx,
    } = pending;
    let channel = match &mut handle {
        SshShellHandle::Dedicated(handle) => handle.channel_open_session().await?,
        SshShellHandle::Multiplexed(handle) => handle.lock().await.channel_open_session().await?,
    };
    let (x11_forwarder, local_notice) = if let (Some(config), Some(rx)) = (x11_config, x11_rx) {
        match channel
            .request_x11(true, false, MIT_MAGIC_COOKIE, &config.fake_cookie_hex, 0)
            .await
        {
            Ok(()) => (Some(X11Forwarder { rx, config }), None),
            Err(_) => (None, Some(enable_x11_failed_message().into_bytes())),
        }
    } else {
        (None, None)
    };
    channel
        .request_pty(
            false,
            &config.term,
            dimensions.cols.into(),
            dimensions.rows.into(),
            dimensions.pixel_width.into(),
            dimensions.pixel_height.into(),
            &[],
        )
        .await?;
    channel.request_shell(true).await?;
    let handle = match handle {
        SshShellHandle::Dedicated(handle) => Some(handle),
        SshShellHandle::Multiplexed(_) => None,
    };
    Ok(OpenSshShellSession {
        handle,
        channel,
        jump_handles,
        disconnect_on_close,
        x11_forwarder,
        local_notice,
    })
}

async fn disconnect_pending_ssh_shell(session: PendingOpenSshShellSession) {
    if session.disconnect_on_close {
        if let SshShellHandle::Dedicated(handle) = session.handle {
            let _ = handle
                .disconnect(Disconnect::ByApplication, "session closed", "en")
                .await;
        }
        for jump_handle in session.jump_handles {
            let _ = jump_handle
                .disconnect(Disconnect::ByApplication, "session closed", "en")
                .await;
        }
    }
}

async fn disconnect_open_ssh_shell(
    handle: Option<client::Handle<SshClientHandler>>,
    jump_handles: Vec<client::Handle<SshClientHandler>>,
    disconnect_on_close: bool,
) {
    if disconnect_on_close {
        if let Some(handle) = handle {
            let _ = handle
                .disconnect(Disconnect::ByApplication, "session closed", "en")
                .await;
        }
        for jump_handle in jump_handles {
            let _ = jump_handle
                .disconnect(Disconnect::ByApplication, "session closed", "en")
                .await;
        }
    }
}

impl SshPtyDimensions {
    fn new(cols: u16, rows: u16, pixel_width: u16, pixel_height: u16) -> Self {
        Self {
            cols: cols.max(1),
            rows: rows.max(1),
            pixel_width,
            pixel_height,
        }
    }

    fn from_config(config: &SshSessionConfig) -> Self {
        Self::new(
            config.cols,
            config.rows,
            config.pixel_width,
            config.pixel_height,
        )
    }
}

fn ssh_client_config(config: &SshSessionConfig) -> Arc<russh::client::Config> {
    let keepalive_interval = if config.keep_alive_interval_secs == 0 {
        None
    } else {
        Some(Duration::from_secs(u64::from(
            config.keep_alive_interval_secs,
        )))
    };
    Arc::new(russh::client::Config {
        inactivity_timeout: None,
        keepalive_interval,
        keepalive_max: 3,
        ..Default::default()
    })
}

fn validate_tunnel_config(config: &SshTunnelConfig) -> anyhow::Result<()> {
    if config.id.trim().is_empty() {
        anyhow::bail!("SSH tunnel id is required");
    }
    match config.mode {
        SshTunnelMode::Local | SshTunnelMode::Remote => {
            if config
                .target_host
                .as_deref()
                .is_none_or(|host| host.trim().is_empty())
            {
                anyhow::bail!("{:?} SSH tunnel requires a target host", config.mode);
            }
            if config.target_port.unwrap_or(0) == 0 {
                anyhow::bail!("{:?} SSH tunnel requires a target port", config.mode);
            }
        }
        SshTunnelMode::Dynamic => {}
    }
    Ok(())
}

fn normalized_bind_host(bind_host: &str) -> String {
    let bind_host = bind_host.trim();
    if bind_host.is_empty() {
        "127.0.0.1".to_string()
    } else {
        bind_host.to_string()
    }
}

fn run_tunnel_worker(
    config: SshTunnelConfig,
    listener: Option<StdTcpListener>,
    mut info: SshTunnelInfo,
    shutdown_rx: oneshot::Receiver<()>,
    ready_tx: mpsc::Sender<Result<SshTunnelInfo, String>>,
    multiplex: Option<SshMultiplexHandle>,
) {
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("nyaterm-ssh-tunnel")
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            let _ = ready_tx.send(Err(format!("failed to start SSH tunnel runtime: {error}")));
            return;
        }
    };

    runtime.block_on(async move {
        let (forwarded_tx, forwarded_rx) = if config.mode == SshTunnelMode::Remote {
            let (tx, rx) = tokio_mpsc::unbounded_channel();
            (Some(tx), Some(rx))
        } else {
            (None, None)
        };
        let (handle, jump_handles, forwarded_registry, disconnect_on_close) =
            match multiplex.as_ref() {
                Some(multiplex) => (
                    multiplex.target_handle(),
                    Vec::new(),
                    Some(multiplex.forwarded_tcpip_registry()),
                    false,
                ),
                None => {
                    match open_authenticated_ssh_handle_with_forwarded_tx(
                        &config.ssh_config,
                        forwarded_tx.clone(),
                    )
                    .await
                    {
                        Ok((handle, jumps)) => (
                            Arc::new(tokio::sync::Mutex::new(handle)),
                            jumps
                                .into_iter()
                                .map(|jump| Arc::new(tokio::sync::Mutex::new(jump)))
                                .collect(),
                            None,
                            true,
                        ),
                        Err(error) => {
                            let _ = ready_tx.send(Err(error.to_string()));
                            return;
                        }
                    }
                }
            };

        match config.mode {
            SshTunnelMode::Local => {
                let Some(listener) = listener else {
                    let _ =
                        ready_tx.send(Err("local SSH tunnel listener was not created".to_string()));
                    return;
                };
                let listener = match tokio::net::TcpListener::from_std(listener) {
                    Ok(listener) => listener,
                    Err(error) => {
                        let _ =
                            ready_tx.send(Err(format!("failed to adopt tunnel listener: {error}")));
                        return;
                    }
                };
                let target_host = config.target_host.unwrap_or_default();
                let target_port = config.target_port.unwrap_or_default();
                let _ = ready_tx.send(Ok(info));
                run_local_tunnel_loop(
                    listener,
                    handle.clone(),
                    target_host,
                    target_port,
                    shutdown_rx,
                )
                .await;
            }
            SshTunnelMode::Remote => {
                let target_host = config.target_host.unwrap_or_default();
                let target_port = config.target_port.unwrap_or_default();
                let actual_port = match handle
                    .lock()
                    .await
                    .tcpip_forward(&info.bind_host, info.listen_port.into())
                    .await
                {
                    Ok(0) => info.listen_port,
                    Ok(port) => port.try_into().unwrap_or(info.listen_port),
                    Err(error) => {
                        let _ = ready_tx.send(Err(format!(
                            "failed to request remote SSH tunnel {}:{}: {error}",
                            info.bind_host, info.listen_port
                        )));
                        return;
                    }
                };
                info.listen_port = actual_port;
                if let (Some(registry), Some(tx)) = (forwarded_registry.as_ref(), forwarded_tx) {
                    registry
                        .lock()
                        .await
                        .by_listener
                        .insert((info.bind_host.clone(), info.listen_port.into()), tx);
                }
                let _ = ready_tx.send(Ok(info.clone()));
                run_remote_tunnel_loop(
                    handle.clone(),
                    info.bind_host.clone(),
                    info.listen_port,
                    target_host,
                    target_port,
                    forwarded_rx.expect("remote tunnel receiver"),
                    shutdown_rx,
                )
                .await;
                if let Some(registry) = forwarded_registry.as_ref() {
                    registry
                        .lock()
                        .await
                        .by_listener
                        .remove(&(info.bind_host.clone(), info.listen_port.into()));
                }
            }
            SshTunnelMode::Dynamic => {
                let Some(listener) = listener else {
                    let _ = ready_tx.send(Err(
                        "dynamic SSH tunnel listener was not created".to_string()
                    ));
                    return;
                };
                let listener = match tokio::net::TcpListener::from_std(listener) {
                    Ok(listener) => listener,
                    Err(error) => {
                        let _ =
                            ready_tx.send(Err(format!("failed to adopt tunnel listener: {error}")));
                        return;
                    }
                };
                let _ = ready_tx.send(Ok(info));
                run_dynamic_tunnel_loop(listener, handle.clone(), shutdown_rx).await;
            }
        }

        if disconnect_on_close {
            let _ = handle
                .lock()
                .await
                .disconnect(Disconnect::ByApplication, "tunnel closed", "en")
                .await;
            for jump_handle in jump_handles {
                let _ = jump_handle
                    .lock()
                    .await
                    .disconnect(Disconnect::ByApplication, "tunnel closed", "en")
                    .await;
            }
        } else {
            drop(jump_handles);
        }
    });
}

async fn run_local_tunnel_loop(
    listener: tokio::net::TcpListener,
    ssh_handle: Arc<tokio::sync::Mutex<client::Handle<SshClientHandler>>>,
    target_host: String,
    target_port: u16,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = &mut shutdown_rx => break,
            accepted = listener.accept() => {
                let Ok((local_stream, peer_addr)) = accepted else {
                    continue;
                };
                let ssh_handle = ssh_handle.clone();
                let target_host = target_host.clone();
                tokio::spawn(async move {
                    let _ = forward_tcp_stream_over_ssh(
                        local_stream,
                        ssh_handle,
                        target_host,
                        target_port,
                        peer_addr,
                    )
                    .await;
                });
            }
        }
    }
}

async fn run_dynamic_tunnel_loop(
    listener: tokio::net::TcpListener,
    ssh_handle: Arc<tokio::sync::Mutex<client::Handle<SshClientHandler>>>,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = &mut shutdown_rx => break,
            accepted = listener.accept() => {
                let Ok((mut local_stream, peer_addr)) = accepted else {
                    continue;
                };
                let ssh_handle = ssh_handle.clone();
                tokio::spawn(async move {
                    let Ok((target_host, target_port)) = read_socks5_connect_request(&mut local_stream).await else {
                        let _ = local_stream.shutdown().await;
                        return;
                    };
                    let _ = forward_tcp_stream_over_ssh(
                        local_stream,
                        ssh_handle,
                        target_host,
                        target_port,
                        peer_addr,
                    )
                    .await;
                });
            }
        }
    }
}

async fn run_remote_tunnel_loop(
    ssh_handle: Arc<tokio::sync::Mutex<client::Handle<SshClientHandler>>>,
    listen_addr: String,
    listen_port: u16,
    target_host: String,
    target_port: u16,
    mut forwarded_rx: tokio_mpsc::UnboundedReceiver<ForwardedTcpIpChannel>,
    mut shutdown_rx: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = &mut shutdown_rx => break,
            forwarded = forwarded_rx.recv() => {
                let Some(forwarded) = forwarded else {
                    break;
                };
                let target_host = target_host.clone();
                tokio::spawn(async move {
                    let _ = forward_remote_channel_to_target(
                        forwarded,
                        target_host,
                        target_port,
                    )
                    .await;
                });
            }
        }
    }

    let _ = ssh_handle
        .lock()
        .await
        .cancel_tcpip_forward(&listen_addr, listen_port.into())
        .await;
}

async fn forward_remote_channel_to_target(
    forwarded: ForwardedTcpIpChannel,
    target_host: String,
    target_port: u16,
) -> anyhow::Result<()> {
    let ForwardedTcpIpChannel {
        channel,
        connected_address,
        connected_port,
        originator_address,
        originator_port,
    } = forwarded;
    let _forward_context = (
        connected_address,
        connected_port,
        originator_address,
        originator_port,
    );
    let mut local_stream =
        tokio::net::TcpStream::connect((target_host.as_str(), target_port)).await?;
    let mut channel_stream = channel.into_stream();
    let _ = tokio::io::copy_bidirectional(&mut local_stream, &mut channel_stream).await?;
    Ok(())
}

async fn forward_tcp_stream_over_ssh(
    mut local_stream: tokio::net::TcpStream,
    ssh_handle: Arc<tokio::sync::Mutex<client::Handle<SshClientHandler>>>,
    target_host: String,
    target_port: u16,
    peer_addr: SocketAddr,
) -> anyhow::Result<()> {
    let channel = {
        let handle = ssh_handle.lock().await;
        handle
            .channel_open_direct_tcpip(
                target_host,
                target_port.into(),
                peer_addr.ip().to_string(),
                peer_addr.port().into(),
            )
            .await?
    };
    let mut channel_stream = channel.into_stream();
    let _ = tokio::io::copy_bidirectional(&mut local_stream, &mut channel_stream).await?;
    Ok(())
}

async fn read_socks5_connect_request<S>(stream: &mut S) -> anyhow::Result<(String, u16)>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut greeting = [0_u8; 2];
    stream.read_exact(&mut greeting).await?;
    if greeting[0] != 0x05 || greeting[1] == 0 {
        anyhow::bail!("invalid SOCKS5 greeting");
    }
    let mut methods = vec![0_u8; greeting[1] as usize];
    stream.read_exact(&mut methods).await?;
    if !methods.contains(&0x00) {
        stream.write_all(&[0x05, 0xff]).await?;
        anyhow::bail!("SOCKS5 client did not offer no-auth method");
    }
    stream.write_all(&[0x05, 0x00]).await?;

    let mut header = [0_u8; 4];
    stream.read_exact(&mut header).await?;
    if header[0] != 0x05 || header[1] != 0x01 || header[2] != 0x00 {
        write_socks5_reply(stream, 0x07).await?;
        anyhow::bail!("unsupported SOCKS5 request");
    }
    let target_host = match header[3] {
        0x01 => {
            let mut addr = [0_u8; 4];
            stream.read_exact(&mut addr).await?;
            std::net::Ipv4Addr::from(addr).to_string()
        }
        0x03 => {
            let mut len = [0_u8; 1];
            stream.read_exact(&mut len).await?;
            let mut domain = vec![0_u8; len[0] as usize];
            stream.read_exact(&mut domain).await?;
            String::from_utf8(domain)
                .map_err(|_| anyhow::anyhow!("SOCKS5 domain is not valid UTF-8"))?
        }
        0x04 => {
            let mut addr = [0_u8; 16];
            stream.read_exact(&mut addr).await?;
            std::net::Ipv6Addr::from(addr).to_string()
        }
        _ => {
            write_socks5_reply(stream, 0x08).await?;
            anyhow::bail!("unsupported SOCKS5 address type");
        }
    };
    let mut port_bytes = [0_u8; 2];
    stream.read_exact(&mut port_bytes).await?;
    let target_port = u16::from_be_bytes(port_bytes);
    write_socks5_reply(stream, 0x00).await?;
    Ok((target_host, target_port))
}

async fn write_socks5_reply<S>(stream: &mut S, code: u8) -> std::io::Result<()>
where
    S: AsyncWrite + Unpin,
{
    stream
        .write_all(&[0x05, code, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await
}

fn run_sftp_operation<T, F>(operation: F) -> anyhow::Result<T>
where
    T: Send + 'static,
    F: Future<Output = anyhow::Result<T>> + Send + 'static,
{
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("nyaterm-sftp")
        .build()
        .map_err(|error| anyhow::anyhow!("failed to start SFTP runtime: {error}"))?;
    runtime.block_on(operation)
}

pub(crate) fn run_ssh_exec_operation<T, F>(operation: F) -> anyhow::Result<T>
where
    T: Send + 'static,
    F: Future<Output = anyhow::Result<T>> + Send + 'static,
{
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_name("nyaterm-ssh-exec")
        .build()
        .map_err(|error| anyhow::anyhow!("failed to start SSH exec runtime: {error}"))?;
    runtime.block_on(operation)
}

pub(crate) async fn exec_ssh_command(
    config: SshSessionConfig,
    command: Vec<u8>,
    timeout: Duration,
) -> anyhow::Result<RemoteCommandOutput> {
    tokio::time::timeout(timeout, async move {
        let (handle, jump_handles) = open_authenticated_ssh_handle(&config).await?;
        let channel = open_exec_channel_on_handle(&handle, command).await?;
        let output = collect_exec_channel(channel).await?;
        let _ = handle
            .disconnect(Disconnect::ByApplication, "ssh exec completed", "en")
            .await;
        for jump_handle in jump_handles {
            let _ = jump_handle
                .disconnect(Disconnect::ByApplication, "ssh exec completed", "en")
                .await;
        }

        Ok(output)
    })
    .await
    .map_err(|_| anyhow::anyhow!("remote command timed out"))?
}

async fn exec_ssh_command_with_multiplex(
    multiplex: SshMultiplexHandle,
    command: Vec<u8>,
    timeout: Duration,
) -> anyhow::Result<RemoteCommandOutput> {
    tokio::time::timeout(timeout, async move {
        let handle = multiplex.target_handle();
        let channel = {
            let handle = handle.lock().await;
            open_exec_channel_on_handle(&handle, command).await?
        };
        collect_exec_channel(channel).await
    })
    .await
    .map_err(|_| anyhow::anyhow!("remote command timed out"))?
}

async fn open_exec_channel_on_handle(
    handle: &client::Handle<SshClientHandler>,
    command: Vec<u8>,
) -> anyhow::Result<russh::Channel<client::Msg>> {
    let channel = handle
        .channel_open_session()
        .await
        .map_err(|error| anyhow::anyhow!("failed to open exec channel: {error}"))?;
    channel
        .exec(true, command)
        .await
        .map_err(|error| anyhow::anyhow!("failed to execute remote command: {error}"))?;
    Ok(channel)
}

async fn collect_exec_channel(
    mut channel: russh::Channel<client::Msg>,
) -> anyhow::Result<RemoteCommandOutput> {
    let mut stdout = String::new();
    let mut stderr = String::new();
    let mut exit_status = None;
    loop {
        match channel.wait().await {
            Some(ChannelMsg::Data { data }) => {
                stdout.push_str(&String::from_utf8_lossy(&data));
            }
            Some(ChannelMsg::ExtendedData { data, .. }) => {
                stderr.push_str(&String::from_utf8_lossy(&data));
            }
            Some(ChannelMsg::ExitStatus {
                exit_status: status,
            }) => {
                exit_status = Some(status);
            }
            Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => break,
            Some(_) => {}
        }
    }

    let _ = channel.close().await;
    Ok(RemoteCommandOutput {
        stdout,
        stderr,
        exit_status,
    })
}

pub(crate) fn ensure_remote_command_success(
    output: RemoteCommandOutput,
    context: &str,
) -> anyhow::Result<RemoteCommandOutput> {
    if matches!(output.exit_status, Some(0) | None) {
        return Ok(output);
    }

    let stderr = output.stderr.trim();
    let stdout = output.stdout.trim();
    let detail = if !stderr.is_empty() {
        stderr
    } else if !stdout.is_empty() {
        stdout
    } else {
        "remote command failed"
    };

    anyhow::bail!("{context}: {detail}")
}

pub fn normalize_process_signal(signal: &str) -> anyhow::Result<&'static str> {
    match signal.trim().to_ascii_uppercase().as_str() {
        "TERM" | "SIGTERM" | "15" => Ok("TERM"),
        "KILL" | "SIGKILL" | "9" => Ok("KILL"),
        "HUP" | "SIGHUP" | "1" => Ok("HUP"),
        "STOP" | "SIGSTOP" | "19" => Ok("STOP"),
        "CONT" | "SIGCONT" | "18" => Ok("CONT"),
        _ => anyhow::bail!("Unsupported process signal"),
    }
}

pub fn is_process_list_unsupported(output: &str) -> bool {
    output
        .lines()
        .any(|line| line.trim() == PROCESS_LIST_UNSUPPORTED_MARKER)
}

pub fn parse_process_output(output: &str) -> Vec<RemoteProcess> {
    output
        .lines()
        .filter_map(|line| {
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() < 12 || cols[0] != "PROCESS" {
                return None;
            }

            Some(RemoteProcess {
                pid: cols[1].parse().ok()?,
                ppid: cols[2].parse().unwrap_or(0),
                user: cols[3].to_string(),
                state: cols[4].to_string(),
                cpu_percent: cols[5].parse().unwrap_or(0.0),
                memory_percent: cols[6].parse().unwrap_or(0.0),
                rss_kb: cols[7].parse().unwrap_or(0),
                vsz_kb: cols[8].parse().unwrap_or(0),
                elapsed: cols[9].to_string(),
                command: cols[10].to_string(),
                command_line: cols[11..].join("\t"),
            })
        })
        .collect()
}

struct OpenSftpSession {
    sftp: SftpSession,
    connection: OpenSftpConnection,
}

enum OpenSftpConnection {
    Dedicated {
        handle: client::Handle<SshClientHandler>,
        jump_handles: Vec<client::Handle<SshClientHandler>>,
    },
    Multiplex,
}

async fn open_sftp_session(
    config: &SshSessionConfig,
    multiplex: Option<&SshMultiplexHandle>,
) -> anyhow::Result<OpenSftpSession> {
    let (channel, connection) = if let Some(multiplex) = multiplex {
        multiplex.ensure_matches_config(config)?;
        let handle = multiplex.target_handle();
        let channel = {
            let handle = handle.lock().await;
            tokio::time::timeout(Duration::from_secs(30), handle.channel_open_session())
                .await
                .map_err(|_| anyhow::anyhow!("SFTP channel open timed out"))??
        };
        (channel, OpenSftpConnection::Multiplex)
    } else {
        let (handle, jump_handles) = open_authenticated_ssh_handle(config).await?;
        let channel = tokio::time::timeout(Duration::from_secs(30), handle.channel_open_session())
            .await
            .map_err(|_| anyhow::anyhow!("SFTP channel open timed out"))??;
        (
            channel,
            OpenSftpConnection::Dedicated {
                handle,
                jump_handles,
            },
        )
    };
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|error| anyhow::anyhow!("failed to start SFTP subsystem: {error}"))?;
    let sftp = tokio::time::timeout(
        Duration::from_secs(30),
        SftpSession::new(channel.into_stream()),
    )
    .await
    .map_err(|_| anyhow::anyhow!("SFTP initialization timed out"))??;
    Ok(OpenSftpSession { sftp, connection })
}

async fn close_sftp_session(session: OpenSftpSession) {
    let OpenSftpSession { sftp, connection } = session;
    let _ = sftp.close().await;
    if let OpenSftpConnection::Dedicated {
        handle,
        jump_handles,
    } = connection
    {
        let _ = handle
            .disconnect(Disconnect::ByApplication, "sftp session closed", "en")
            .await;
        for jump_handle in jump_handles {
            let _ = jump_handle
                .disconnect(Disconnect::ByApplication, "sftp session closed", "en")
                .await;
        }
    }
}

type SshHandleChain = (
    client::Handle<SshClientHandler>,
    Vec<client::Handle<SshClientHandler>>,
);

fn open_authenticated_ssh_handle(
    config: &SshSessionConfig,
) -> Pin<Box<dyn Future<Output = anyhow::Result<SshHandleChain>> + Send + '_>> {
    open_authenticated_ssh_handle_with_channel_senders(config, None, None)
}

fn open_authenticated_ssh_handle_with_forwarded_tx(
    config: &SshSessionConfig,
    forwarded_tcpip_tx: Option<tokio_mpsc::UnboundedSender<ForwardedTcpIpChannel>>,
) -> Pin<Box<dyn Future<Output = anyhow::Result<SshHandleChain>> + Send + '_>> {
    open_authenticated_ssh_handle_with_channel_senders(config, forwarded_tcpip_tx, None)
}

fn open_authenticated_ssh_handle_with_channel_senders(
    config: &SshSessionConfig,
    forwarded_tcpip_tx: Option<tokio_mpsc::UnboundedSender<ForwardedTcpIpChannel>>,
    x11_tx: Option<tokio_mpsc::UnboundedSender<X11ChannelOpen>>,
) -> Pin<Box<dyn Future<Output = anyhow::Result<SshHandleChain>> + Send + '_>> {
    let forwarded_tcpip = forwarded_tcpip_tx.map(|tx| {
        Arc::new(tokio::sync::Mutex::new(ForwardedTcpIpDispatch {
            fallback: Some(tx),
            by_listener: HashMap::new(),
        }))
    });
    open_authenticated_ssh_handle_with_sender_registry(config, forwarded_tcpip, x11_tx)
}

fn open_authenticated_ssh_handle_with_sender_registry(
    config: &SshSessionConfig,
    forwarded_tcpip: Option<ForwardedTcpIpRegistry>,
    x11_tx: Option<tokio_mpsc::UnboundedSender<X11ChannelOpen>>,
) -> Pin<Box<dyn Future<Output = anyhow::Result<SshHandleChain>> + Send + '_>> {
    Box::pin(async move {
        if let Some(jump_config) = config.proxy_jump.as_deref() {
            let (jump_handle, mut jump_handles) =
                open_authenticated_ssh_handle(jump_config).await?;
            let direct_channel = tokio::time::timeout(
                Duration::from_secs(30),
                jump_handle.channel_open_direct_tcpip(
                    &config.host,
                    config.port.into(),
                    "127.0.0.1",
                    0,
                ),
            )
            .await
            .map_err(|_| anyhow::anyhow!("SSH ProxyJump direct-tcpip open timed out"))??;
            let mut handle = tokio::time::timeout(
                Duration::from_secs(30),
                client::connect_stream(
                    ssh_client_config(config),
                    direct_channel.into_stream(),
                    SshClientHandler {
                        host: config.host.clone(),
                        port: config.port,
                        verifier: config.host_key_verifier.clone(),
                        forwarded_tcpip: forwarded_tcpip.clone(),
                        x11_tx: x11_tx.clone(),
                    },
                ),
            )
            .await
            .map_err(|_| anyhow::anyhow!("SSH ProxyJump target connection timed out"))??;
            authenticate_ssh(&mut handle, config).await?;
            jump_handles.push(jump_handle);
            return Ok((handle, jump_handles));
        }

        let mut handle = tokio::time::timeout(
            Duration::from_secs(30),
            connect_ssh_transport(config, forwarded_tcpip.clone(), x11_tx.clone()),
        )
        .await
        .map_err(|_| anyhow::anyhow!("SSH connection timed out"))??;

        authenticate_ssh(&mut handle, config).await?;
        Ok((handle, Vec::new()))
    })
}

async fn connect_ssh_transport(
    config: &SshSessionConfig,
    forwarded_tcpip: Option<ForwardedTcpIpRegistry>,
    x11_tx: Option<tokio_mpsc::UnboundedSender<X11ChannelOpen>>,
) -> anyhow::Result<client::Handle<SshClientHandler>> {
    let handler = SshClientHandler {
        host: config.host.clone(),
        port: config.port,
        verifier: config.host_key_verifier.clone(),
        forwarded_tcpip,
        x11_tx,
    };
    let Some(proxy) = config.proxy.as_ref() else {
        return client::connect(
            ssh_client_config(config),
            (config.host.as_str(), config.port),
            handler,
        )
        .await
        .map_err(|error| anyhow::anyhow!("SSH connection failed: {error}"));
    };

    match proxy.protocol.as_str() {
        "socks5" => {
            let proxy_addr = format!("{}:{}", proxy.host, proxy.port);
            let target = (config.host.as_str(), config.port);
            let stream = match (
                proxy.username.as_deref().filter(|value| !value.is_empty()),
                proxy.password.as_deref().filter(|value| !value.is_empty()),
            ) {
                (Some(username), Some(password)) => {
                    tokio_socks::tcp::Socks5Stream::connect_with_password(
                        proxy_addr.as_str(),
                        target,
                        username,
                        password,
                    )
                    .await
                }
                _ => tokio_socks::tcp::Socks5Stream::connect(proxy_addr.as_str(), target).await,
            }
            .map_err(|error| anyhow::anyhow!("SOCKS5 proxy connection failed: {error}"))?;
            client::connect_stream(ssh_client_config(config), stream.into_inner(), handler)
                .await
                .map_err(|error| anyhow::anyhow!("SSH connection via SOCKS5 proxy failed: {error}"))
        }
        "http" => {
            let proxy_addr = format!("{}:{}", proxy.host, proxy.port);
            let mut stream = tokio::net::TcpStream::connect(&proxy_addr)
                .await
                .map_err(|error| anyhow::anyhow!("HTTP proxy connection failed: {error}"))?;
            match (
                proxy.username.as_deref().filter(|value| !value.is_empty()),
                proxy.password.as_deref().filter(|value| !value.is_empty()),
            ) {
                (Some(username), Some(password)) => {
                    async_http_proxy::http_connect_tokio_with_basic_auth(
                        &mut stream,
                        &config.host,
                        config.port,
                        username,
                        password,
                    )
                    .await
                }
                _ => {
                    async_http_proxy::http_connect_tokio(&mut stream, &config.host, config.port)
                        .await
                }
            }
            .map_err(|error| anyhow::anyhow!("HTTP proxy tunnel failed: {error}"))?;
            client::connect_stream(ssh_client_config(config), stream, handler)
                .await
                .map_err(|error| anyhow::anyhow!("SSH connection via HTTP proxy failed: {error}"))
        }
        "proxycommand" => {
            let stream = open_proxy_command_stream(
                proxy.command.as_deref(),
                &config.host,
                config.port,
                &config.username,
            )
            .await?;
            client::connect_stream(ssh_client_config(config), stream, handler)
                .await
                .map_err(|error| anyhow::anyhow!("SSH connection via ProxyCommand failed: {error}"))
        }
        other => anyhow::bail!("unsupported SSH proxy protocol '{other}'"),
    }
}

async fn open_proxy_command_stream(
    template: Option<&str>,
    host: &str,
    port: u16,
    username: &str,
) -> anyhow::Result<ProxyCommandStream> {
    let command = expand_proxy_command(template, host, port, username)?;
    let mut process = system_shell_command(&command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| anyhow::anyhow!("ProxyCommand failed to start: {error}"))?;

    let stdin = process
        .stdin
        .take()
        .ok_or_else(|| anyhow::anyhow!("ProxyCommand stdin unavailable"))?;
    let stdout = process
        .stdout
        .take()
        .ok_or_else(|| anyhow::anyhow!("ProxyCommand stdout unavailable"))?;

    if let Some(mut stderr) = process.stderr.take() {
        tokio::spawn(async move {
            let mut buffer = [0_u8; 1024];
            loop {
                match stderr.read(&mut buffer).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {}
                }
            }
        });
    }

    tokio::spawn(async move {
        let _ = process.wait().await;
    });

    Ok(ProxyCommandStream { stdout, stdin })
}

struct ProxyCommandStream {
    stdout: tokio::process::ChildStdout,
    stdin: tokio::process::ChildStdin,
}

impl AsyncRead for ProxyCommandStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Pin::new(&mut self.stdout).poll_read(cx, buf)
    }
}

impl AsyncWrite for ProxyCommandStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        Pin::new(&mut self.stdin).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Pin::new(&mut self.stdin).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        Pin::new(&mut self.stdin).poll_shutdown(cx)
    }
}

fn expand_proxy_command(
    template: Option<&str>,
    host: &str,
    port: u16,
    username: &str,
) -> anyhow::Result<String> {
    let template = template.unwrap_or_default().trim();
    if template.is_empty() {
        anyhow::bail!("ProxyCommand is empty");
    }

    let quoted_host = local_shell_quote(host);
    let port = port.to_string();
    let quoted_port = local_shell_quote(&port);
    let quoted_username = local_shell_quote(username);

    let mut output = String::with_capacity(template.len());
    let mut chars = template.chars();
    while let Some(ch) = chars.next() {
        if ch != '%' {
            output.push(ch);
            continue;
        }
        match chars.next() {
            Some('%') => output.push('%'),
            Some('h') => output.push_str(&quoted_host),
            Some('p') => output.push_str(&quoted_port),
            Some('r') => output.push_str(&quoted_username),
            Some(other) => {
                output.push('%');
                output.push(other);
            }
            None => output.push('%'),
        }
    }

    Ok(output)
}

#[cfg(windows)]
fn system_shell_command(command: &str) -> tokio::process::Command {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    let mut cmd = tokio::process::Command::new("cmd");
    cmd.arg("/C").arg(command);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

#[cfg(not(windows))]
fn system_shell_command(command: &str) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new("sh");
    cmd.arg("-c").arg(command);
    cmd
}

#[cfg(windows)]
fn local_shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "\"\"".to_string();
    }

    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-' | ':' | '@' | '%'))
    {
        return value.to_string();
    }

    let escaped = value.replace('"', "\"\"");
    format!("\"{escaped}\"")
}

#[cfg(not(windows))]
fn local_shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    format!("'{}'", value.replace('\'', "'\\''"))
}

async fn authenticate_ssh(
    handle: &mut client::Handle<SshClientHandler>,
    config: &SshSessionConfig,
) -> anyhow::Result<()> {
    if let Some(key_auth) = config.key_auth.as_ref() {
        return authenticate_ssh_key(handle, config, key_auth).await;
    }

    if let Some(password) = config
        .password
        .as_deref()
        .filter(|password| !password.is_empty())
    {
        let auth_result = authenticate_password(handle, config, password).await?;
        if auth_result.success() {
            return Ok(());
        }
        if try_keyboard_interactive_after_auth_result(handle, config, &auth_result).await? {
            return Ok(());
        }
        return authenticate_password_with_prompt(
            handle,
            config,
            SshCredentialPromptReason::PasswordRejected,
        )
        .await;
    } else if config.allow_none_auth {
        let auth_result = tokio::time::timeout(
            Duration::from_secs(30),
            handle.authenticate_none(config.username.clone()),
        )
        .await
        .map_err(|_| anyhow::anyhow!("SSH none authentication timed out"))??;
        if auth_result.success() {
            return Ok(());
        }
        if try_keyboard_interactive_after_auth_result(handle, config, &auth_result).await? {
            return Ok(());
        }
        anyhow::bail!("SSH none authentication rejected by server");
    } else {
        authenticate_password_with_prompt(
            handle,
            config,
            SshCredentialPromptReason::MissingPassword,
        )
        .await
    }
}

async fn authenticate_ssh_key(
    handle: &mut client::Handle<SshClientHandler>,
    config: &SshSessionConfig,
    key_auth: &SshKeyAuthConfig,
) -> anyhow::Result<()> {
    let key = decode_ssh_key_with_prompt(config, key_auth)?;
    let hash_alg = tokio::time::timeout(Duration::from_secs(30), handle.best_supported_rsa_hash())
        .await
        .ok()
        .and_then(Result::ok)
        .flatten()
        .flatten();
    let cert = key_auth
        .cert_data
        .as_deref()
        .map(russh::keys::Certificate::from_openssh)
        .transpose()
        .map_err(|error| anyhow::anyhow!("failed to decode OpenSSH certificate: {error}"))?;

    let auth_result = if let Some(cert) = cert {
        tokio::time::timeout(
            Duration::from_secs(30),
            handle.authenticate_openssh_cert(config.username.clone(), Arc::new(key), cert),
        )
        .await
        .map_err(|_| anyhow::anyhow!("SSH certificate authentication timed out"))??
    } else {
        tokio::time::timeout(
            Duration::from_secs(30),
            handle.authenticate_publickey(
                config.username.clone(),
                PrivateKeyWithHashAlg::new(Arc::new(key), hash_alg),
            ),
        )
        .await
        .map_err(|_| anyhow::anyhow!("SSH public-key authentication timed out"))??
    };

    if auth_result.success() {
        Ok(())
    } else if try_keyboard_interactive_after_auth_result(handle, config, &auth_result).await? {
        Ok(())
    } else {
        anyhow::bail!("SSH public-key authentication rejected by server")
    }
}

async fn authenticate_password(
    handle: &mut client::Handle<SshClientHandler>,
    config: &SshSessionConfig,
    password: &str,
) -> anyhow::Result<client::AuthResult> {
    tokio::time::timeout(
        Duration::from_secs(30),
        handle.authenticate_password(config.username.clone(), password.to_string()),
    )
    .await
    .map_err(|_| anyhow::anyhow!("SSH password authentication timed out"))?
    .map_err(anyhow::Error::from)
}

async fn authenticate_password_with_prompt(
    handle: &mut client::Handle<SshClientHandler>,
    config: &SshSessionConfig,
    reason: SshCredentialPromptReason,
) -> anyhow::Result<()> {
    for attempt in 1..=3 {
        let Some(password) = request_runtime_secret(
            config,
            SshCredentialPromptKind::Password,
            reason,
            attempt,
            None,
            false,
        )?
        else {
            anyhow::bail!("SSH password prompt was cancelled");
        };
        if password.is_empty() {
            continue;
        }
        let auth_result = authenticate_password(handle, config, &password).await?;
        if auth_result.success() {
            return Ok(());
        }
        if try_keyboard_interactive_after_auth_result(handle, config, &auth_result).await? {
            return Ok(());
        }
    }
    anyhow::bail!("SSH password authentication rejected by server")
}

const MAX_KEYBOARD_INTERACTIVE_RESTARTS: u32 = 8;

async fn try_keyboard_interactive_after_auth_result(
    handle: &mut client::Handle<SshClientHandler>,
    config: &SshSessionConfig,
    auth_result: &client::AuthResult,
) -> anyhow::Result<bool> {
    match auth_result {
        client::AuthResult::Success => Ok(true),
        client::AuthResult::Failure {
            remaining_methods,
            partial_success: _,
        } if remaining_methods.contains(&MethodKind::KeyboardInteractive) => {
            finish_keyboard_interactive(handle, config).await?;
            Ok(true)
        }
        client::AuthResult::Failure { .. } => Ok(false),
    }
}

async fn finish_keyboard_interactive(
    handle: &mut client::Handle<SshClientHandler>,
    config: &SshSessionConfig,
) -> anyhow::Result<()> {
    let mut step = tokio::time::timeout(
        Duration::from_secs(30),
        handle.authenticate_keyboard_interactive_start(config.username.clone(), None),
    )
    .await
    .map_err(|_| anyhow::anyhow!("SSH keyboard-interactive authentication timed out"))??;
    let mut round = 0_u32;
    let mut restart_count = 0_u32;

    loop {
        match step {
            client::KeyboardInteractiveAuthResponse::Success => return Ok(()),
            client::KeyboardInteractiveAuthResponse::Failure {
                remaining_methods,
                partial_success,
            } => {
                if partial_success
                    && remaining_methods.contains(&MethodKind::KeyboardInteractive)
                    && restart_count < MAX_KEYBOARD_INTERACTIVE_RESTARTS
                {
                    restart_count = restart_count.saturating_add(1);
                    step = tokio::time::timeout(
                        Duration::from_secs(30),
                        handle
                            .authenticate_keyboard_interactive_start(config.username.clone(), None),
                    )
                    .await
                    .map_err(|_| {
                        anyhow::anyhow!(
                            "SSH keyboard-interactive restart timed out after partial success"
                        )
                    })??;
                    continue;
                }

                anyhow::bail!(
                    "SSH keyboard-interactive authentication rejected by server (remaining methods: {:?}, partial success: {})",
                    remaining_methods,
                    partial_success
                );
            }
            client::KeyboardInteractiveAuthResponse::InfoRequest {
                name,
                instructions,
                prompts,
            } => {
                round = round.saturating_add(1);
                let prompt_count = prompts.len();
                let responses = if prompts.is_empty() {
                    Vec::new()
                } else if let Some(password) = config
                    .password
                    .as_deref()
                    .filter(|_| should_auto_fill_password_prompts(&prompts))
                {
                    vec![password.to_string()]
                } else if config.auto_fill_otp && should_auto_fill_otp_prompts(&prompts) {
                    match request_otp_response(config, prompt_count)? {
                        Some(responses) => responses,
                        None => request_keyboard_interactive_responses(
                            config,
                            &name,
                            &instructions,
                            prompts,
                            round,
                        )?,
                    }
                } else {
                    request_keyboard_interactive_responses(
                        config,
                        &name,
                        &instructions,
                        prompts,
                        round,
                    )?
                };

                step = tokio::time::timeout(
                    Duration::from_secs(30),
                    handle.authenticate_keyboard_interactive_respond(responses),
                )
                .await
                .map_err(|_| anyhow::anyhow!("SSH keyboard-interactive response timed out"))??;
            }
        }
    }
}

fn request_keyboard_interactive_responses(
    config: &SshSessionConfig,
    name: &str,
    instructions: &str,
    prompts: Vec<client::Prompt>,
    round: u32,
) -> anyhow::Result<Vec<String>> {
    let prompt_count = prompts.len();
    let Some(provider) = config.credential_provider.as_ref() else {
        anyhow::bail!("SSH runtime credential prompt is unavailable");
    };
    let request = SshKeyboardInteractiveRequest {
        host: config.host.clone(),
        port: config.port,
        username: config.username.clone(),
        connection_name: config.name.clone(),
        name: name.to_string(),
        instructions: instructions.to_string(),
        round,
        prompts: prompts
            .into_iter()
            .map(|prompt| SshKeyboardInteractivePrompt {
                prompt: prompt.prompt,
                echo: prompt.echo,
            })
            .collect(),
        otp_id: config.otp_id.clone(),
    };
    let Some(responses) = provider
        .request_keyboard_interactive(&request)
        .map_err(|error| anyhow::anyhow!("SSH keyboard-interactive prompt failed: {error}"))?
    else {
        anyhow::bail!("SSH keyboard-interactive prompt was cancelled");
    };
    if responses.len() != prompt_count {
        anyhow::bail!(
            "SSH keyboard-interactive prompt returned {} responses for {prompt_count} prompts",
            responses.len()
        );
    }
    Ok(responses)
}

fn request_otp_response(
    config: &SshSessionConfig,
    prompt_count: usize,
) -> anyhow::Result<Option<Vec<String>>> {
    let Some(otp_id) = config.otp_id.as_deref().filter(|otp_id| !otp_id.is_empty()) else {
        return Ok(None);
    };
    let Some(provider) = config.otp_provider.as_ref() else {
        return Ok(None);
    };
    let Some(code) = provider
        .request_otp_code(otp_id)
        .map_err(|error| anyhow::anyhow!("SSH OTP auto-fill failed: {error}"))?
    else {
        return Ok(None);
    };
    Ok(Some(vec![code; prompt_count]))
}

fn format_keyboard_interactive_prompt(
    name: &str,
    instructions: &str,
    prompt: &str,
    index: usize,
    prompt_count: usize,
) -> String {
    let mut parts = Vec::new();
    if !name.trim().is_empty() {
        parts.push(name.trim().to_string());
    }
    if !instructions.trim().is_empty() {
        parts.push(instructions.trim().to_string());
    }
    if !prompt.trim().is_empty() {
        parts.push(prompt.trim().to_string());
    } else if prompt_count > 1 {
        parts.push(format!("Response {} of {}", index + 1, prompt_count));
    } else {
        parts.push("Response".to_string());
    }
    parts.join("\n")
}

fn should_auto_fill_password_prompts(prompts: &[client::Prompt]) -> bool {
    prompts.len() == 1
        && !prompts[0].echo
        && is_password_keyboard_interactive_prompt(&prompts[0].prompt)
}

fn should_auto_fill_otp_prompts(prompts: &[client::Prompt]) -> bool {
    prompts.len() == 1 && is_otp_keyboard_interactive_prompt(&prompts[0].prompt)
}

fn is_otp_keyboard_interactive_prompt(prompt: &str) -> bool {
    let normalized = prompt.to_lowercase();
    let selection_markers = [
        "select",
        "choose",
        "choice",
        "option",
        "method",
        "delivery",
        "send to",
        "send via",
        "push",
        "sms/email",
        "sms or email",
        "email or sms",
        "选择",
        "请选择",
        "选项",
        "方式",
        "方法",
        "发送到",
        "发送至",
    ];
    if selection_markers
        .iter()
        .any(|marker| normalized.contains(marker))
    {
        return false;
    }

    [
        "otp",
        "totp",
        "hotp",
        "2fa",
        "mfa",
        "one-time",
        "one time",
        "verification code",
        "authentication code",
        "auth code",
        "authenticator",
        "passcode",
        "token",
        "验证码",
        "校验码",
        "动态码",
        "动态密码",
        "动态口令",
        "一次性",
        "令牌",
        "双因素",
        "二次",
        "两步",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn is_password_keyboard_interactive_prompt(prompt: &str) -> bool {
    let normalized = prompt.to_lowercase();
    let additional_factor_markers = [
        "otp",
        "totp",
        "hotp",
        "2fa",
        "mfa",
        "one-time",
        "one time",
        "verification",
        "authentication code",
        "auth code",
        "authenticator",
        "passcode",
        "token",
        "code",
        "验证码",
        "校验码",
        "动态码",
        "动态密码",
        "动态口令",
        "一次性",
        "令牌",
        "双因素",
        "二次",
        "两步",
    ];
    if additional_factor_markers
        .iter()
        .any(|marker| normalized.contains(marker))
    {
        return false;
    }

    ["password", "passphrase", "密码", "口令"]
        .iter()
        .any(|marker| normalized.contains(marker))
}

fn decode_ssh_key_with_prompt(
    config: &SshSessionConfig,
    key_auth: &SshKeyAuthConfig,
) -> anyhow::Result<russh::keys::PrivateKey> {
    match russh::keys::decode_secret_key(&key_auth.key_data, key_auth.passphrase.as_deref()) {
        Ok(key) => return Ok(key),
        Err(error) if config.credential_provider.is_none() => {
            anyhow::bail!("failed to decode SSH private key: {error}");
        }
        Err(_) => {}
    }

    for attempt in 1..=3 {
        let Some(passphrase) = request_runtime_secret(
            config,
            SshCredentialPromptKind::KeyPassphrase,
            SshCredentialPromptReason::KeyPassphraseRequired,
            attempt,
            None,
            false,
        )?
        else {
            anyhow::bail!("SSH key passphrase prompt was cancelled");
        };
        match russh::keys::decode_secret_key(&key_auth.key_data, Some(&passphrase)) {
            Ok(key) => return Ok(key),
            Err(error) if attempt == 3 => {
                anyhow::bail!("failed to decode SSH private key: {error}");
            }
            Err(_) => {}
        }
    }

    anyhow::bail!("failed to decode SSH private key")
}

fn request_runtime_secret(
    config: &SshSessionConfig,
    kind: SshCredentialPromptKind,
    reason: SshCredentialPromptReason,
    attempt: u32,
    prompt_text: Option<String>,
    echo: bool,
) -> anyhow::Result<Option<String>> {
    let Some(provider) = config.credential_provider.as_ref() else {
        anyhow::bail!("SSH runtime credential prompt is unavailable");
    };
    provider
        .request_secret(&SshCredentialPrompt {
            host: config.host.clone(),
            port: config.port,
            username: config.username.clone(),
            connection_name: config.name.clone(),
            kind,
            reason,
            attempt,
            prompt_text,
            echo,
        })
        .map_err(|error| anyhow::anyhow!("SSH runtime credential prompt failed: {error}"))
}

struct SshClientHandler {
    host: String,
    port: u16,
    verifier: Option<Arc<dyn SshHostKeyVerifier>>,
    forwarded_tcpip: Option<ForwardedTcpIpRegistry>,
    x11_tx: Option<tokio_mpsc::UnboundedSender<X11ChannelOpen>>,
}

impl client::Handler for SshClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        let Some(verifier) = &self.verifier else {
            return Ok(false);
        };
        let host_identifier = ssh_host_identifier(&self.host, self.port);
        let host_key = SshHostKey {
            host: self.host.clone(),
            port: self.port,
            host_identifier,
            key_type: server_public_key.algorithm().to_string(),
            key_base64: server_public_key.public_key_base64(),
            fingerprint: server_public_key
                .fingerprint(Default::default())
                .to_string(),
        };
        match verifier.verify(&host_key) {
            Ok(SshHostKeyDecision::Accept) => Ok(true),
            Ok(SshHostKeyDecision::Reject(_)) | Err(_) => Ok(false),
        }
    }

    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: russh::Channel<client::Msg>,
        connected_address: &str,
        connected_port: u32,
        originator_address: &str,
        originator_port: u32,
        reply: russh::client::ChannelOpenHandle,
        _session: &mut russh::client::Session,
    ) -> Result<(), Self::Error> {
        let Some(registry) = self.forwarded_tcpip.as_ref() else {
            reply
                .reject(russh::ChannelOpenFailure::AdministrativelyProhibited)
                .await;
            return Ok(());
        };
        let dispatch = registry.lock().await;
        let tx = forwarded_tcpip_sender_for(&dispatch, connected_address, connected_port);
        drop(dispatch);
        let Some(tx) = tx else {
            reply
                .reject(russh::ChannelOpenFailure::AdministrativelyProhibited)
                .await;
            return Ok(());
        };
        if tx
            .send(ForwardedTcpIpChannel {
                channel,
                connected_address: connected_address.to_string(),
                connected_port,
                originator_address: originator_address.to_string(),
                originator_port,
            })
            .is_ok()
        {
            reply.accept().await;
        } else {
            reply
                .reject(russh::ChannelOpenFailure::AdministrativelyProhibited)
                .await;
        }
        Ok(())
    }

    async fn server_channel_open_x11(
        &mut self,
        channel: russh::Channel<client::Msg>,
        originator_address: &str,
        originator_port: u32,
        reply: russh::client::ChannelOpenHandle,
        _session: &mut russh::client::Session,
    ) -> Result<(), Self::Error> {
        let Some(tx) = self.x11_tx.as_ref() else {
            reply
                .reject(russh::ChannelOpenFailure::AdministrativelyProhibited)
                .await;
            return Ok(());
        };
        if tx
            .send(X11ChannelOpen {
                channel,
                originator_address: originator_address.to_string(),
                originator_port,
            })
            .is_ok()
        {
            reply.accept().await;
        } else {
            reply
                .reject(russh::ChannelOpenFailure::AdministrativelyProhibited)
                .await;
        }
        Ok(())
    }
}

fn ssh_host_identifier(host: &str, port: u16) -> String {
    if port == 22 {
        host.to_string()
    } else {
        format!("[{host}]:{port}")
    }
}

fn send_session_error(
    event_queue: &SessionEventQueue,
    session_id: &str,
    error: impl std::fmt::Display,
) {
    event_queue.push(SessionEvent::Error {
        session_id: session_id.to_string(),
        message: error.to_string(),
    });
}

fn spawn_serial_reader_thread(
    session_id: String,
    mut reader: Box<dyn SerialPort>,
    stop_reader: Arc<AtomicBool>,
    event_queue: SessionEventQueue,
) -> JoinHandle<()> {
    std::thread::spawn(move || {
        let mut buffer = [0_u8; 4096];
        while !stop_reader.load(Ordering::Relaxed) {
            match reader.read(&mut buffer) {
                Ok(0) => continue,
                Ok(read) => {
                    event_queue.push(SessionEvent::Output {
                        session_id: session_id.clone(),
                        data: buffer[..read].to_vec(),
                    });
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    continue;
                }
                Err(error) => {
                    event_queue.push(SessionEvent::Error {
                        session_id: session_id.clone(),
                        message: error.to_string(),
                    });
                    break;
                }
            }
        }
    })
}

fn remap_del_to_bs(data: &[u8]) -> Vec<u8> {
    data.iter()
        .map(|byte| if *byte == 0x7f { 0x08 } else { *byte })
        .collect()
}

fn open_serial_port(config: &SerialSessionConfig) -> serialport::Result<Box<dyn SerialPort>> {
    serialport::new(&config.port_name, config.baud_rate)
        .data_bits(parse_data_bits(config.data_bits))
        .parity(parse_parity(&config.parity))
        .stop_bits(parse_stop_bits(&config.stop_bits))
        .flow_control(FlowControl::None)
        .timeout(Duration::from_millis(10))
        .open()
}

fn parse_data_bits(value: u8) -> DataBits {
    match value {
        5 => DataBits::Five,
        6 => DataBits::Six,
        7 => DataBits::Seven,
        _ => DataBits::Eight,
    }
}

fn parse_parity(value: &str) -> Parity {
    match value {
        "odd" => Parity::Odd,
        "even" => Parity::Even,
        _ => Parity::None,
    }
}

fn parse_stop_bits(value: &str) -> StopBits {
    match value {
        "2" => StopBits::Two,
        _ => StopBits::One,
    }
}

const IAC: u8 = 255;
const WILL: u8 = 251;
const WONT: u8 = 252;
const DO: u8 = 253;
const DONT: u8 = 254;
const SB: u8 = 250;
const SE: u8 = 240;
const OPT_ECHO: u8 = 1;
const OPT_SUPPRESS_GO_AHEAD: u8 = 3;
const OPT_NAWS: u8 = 31;

fn negotiate_response(command: u8, option: u8, send_naws: bool, send_sga: bool) -> Vec<u8> {
    match command {
        WILL => {
            if option == OPT_ECHO || (send_sga && option == OPT_SUPPRESS_GO_AHEAD) {
                vec![IAC, DO, option]
            } else {
                vec![IAC, DONT, option]
            }
        }
        DO => {
            if send_naws && option == OPT_NAWS {
                vec![IAC, WILL, option]
            } else {
                vec![IAC, WONT, option]
            }
        }
        WONT => vec![IAC, DONT, option],
        DONT => vec![IAC, WONT, option],
        _ => vec![],
    }
}

fn maybe_build_naws(cols: u16, rows: u16, config: &TelnetSessionConfig) -> Option<Vec<u8>> {
    if config.raw_tcp || !config.send_naws {
        return None;
    }
    Some(vec![
        IAC,
        SB,
        OPT_NAWS,
        (cols >> 8) as u8,
        (cols & 0xff) as u8,
        (rows >> 8) as u8,
        (rows & 0xff) as u8,
        IAC,
        SE,
    ])
}

fn unescape_iac_iac(data: &[u8]) -> Vec<u8> {
    let mut visible = Vec::with_capacity(data.len());
    let mut index = 0;
    while index < data.len() {
        if data[index] == IAC && index + 1 < data.len() && data[index + 1] == IAC {
            visible.push(IAC);
            index += 2;
        } else {
            visible.push(data[index]);
            index += 1;
        }
    }
    visible
}

fn strip_telnet_commands(data: &[u8], on_negotiate: &mut impl FnMut(u8, u8)) -> Vec<u8> {
    let mut visible = Vec::with_capacity(data.len());
    let mut index = 0;
    while index < data.len() {
        if data[index] == IAC && index + 1 < data.len() {
            let command = data[index + 1];
            match command {
                IAC => {
                    visible.push(IAC);
                    index += 2;
                }
                WILL | WONT | DO | DONT => {
                    if index + 2 < data.len() {
                        on_negotiate(command, data[index + 2]);
                        index += 3;
                    } else {
                        index += 2;
                    }
                }
                SB => {
                    index += 2;
                    while index < data.len() {
                        if data[index] == IAC && index + 1 < data.len() && data[index + 1] == SE {
                            index += 2;
                            break;
                        }
                        index += 1;
                    }
                }
                _ => index += 2,
            }
        } else {
            visible.push(data[index]);
            index += 1;
        }
    }
    visible
}

fn normalize_telnet_input(data: &[u8], config: &TelnetSessionConfig) -> Vec<u8> {
    if config.raw_tcp {
        return data.to_vec();
    }
    let newline = match config.enter_mode {
        TelnetEnterMode::Crlf => b"\r\n".as_slice(),
        TelnetEnterMode::Cr => b"\r".as_slice(),
        TelnetEnterMode::Lf => b"\n".as_slice(),
    };
    let mut normalized = Vec::with_capacity(data.len());
    for byte in data {
        match *byte {
            b'\n' | b'\r' => normalized.extend_from_slice(newline),
            IAC => normalized.extend_from_slice(&[IAC, IAC]),
            _ => normalized.push(*byte),
        }
    }
    normalized
}

fn build_command(config: &LocalSessionConfig) -> CommandBuilder {
    let shell = config
        .shell_path
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(default_shell);
    let mut command = CommandBuilder::new(&shell);
    if config.shell_args.is_empty() && cfg!(not(target_os = "windows")) {
        if should_use_interactive_login_args(&shell) {
            command.args(["--login", "-i"]);
        }
    } else {
        command.args(config.shell_args.iter().map(String::as_str));
    }
    command
}

fn configure_environment(command: &mut CommandBuilder) {
    command.env("TERM", "xterm-256color");
    if cfg!(target_os = "macos") {
        command.env("LANG", utf8_env_or("LANG", "en_US.UTF-8"));
        command.env("LC_CTYPE", utf8_env_or("LC_CTYPE", "UTF-8"));
    }
}

fn utf8_env_or(name: &str, fallback: &str) -> String {
    std::env::var(name)
        .ok()
        .filter(|value| {
            let normalized = value.to_ascii_lowercase().replace('_', "-");
            normalized.contains("utf-8") || normalized.contains("utf8")
        })
        .unwrap_or_else(|| fallback.to_string())
}

fn default_shell() -> String {
    if cfg!(target_os = "windows") {
        std::env::var("COMSPEC").unwrap_or_else(|_| "powershell.exe".to_string())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }
}

fn should_use_interactive_login_args(program: &str) -> bool {
    let name = program
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(program)
        .to_ascii_lowercase();
    matches!(name.as_str(), "bash" | "zsh" | "fish")
}

pub type SharedSessionManager = Arc<SessionManager>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    struct GatedWriter {
        started_tx: mpsc::SyncSender<()>,
        release_rx: mpsc::Receiver<()>,
        output: Arc<Mutex<Vec<u8>>>,
    }

    impl Write for GatedWriter {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            let _ = self.started_tx.send(());
            let _ = self.release_rx.recv();
            self.output
                .lock()
                .expect("output lock")
                .extend_from_slice(data);
            Ok(data.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    struct WriteCapture(Arc<Mutex<Vec<Vec<u8>>>>);

    impl Write for WriteCapture {
        fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .expect("write capture lock")
                .push(data.to_vec());
            Ok(data.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn queued_transport_writer_returns_before_blocking_write_completes() {
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        let output = Arc::new(Mutex::new(Vec::new()));
        let mut writer = QueuedTransportWriter::spawn(
            "queued".to_string(),
            GatedWriter {
                started_tx,
                release_rx,
                output: output.clone(),
            },
            false,
            SessionEventQueue::new(),
        );

        writer.write(b"input".to_vec()).expect("queue input");
        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("background write should start");
        assert!(output.lock().expect("output lock").is_empty());

        release_tx.send(()).expect("release writer");
        writer.close();
        assert_eq!(*output.lock().expect("output lock"), b"input");
    }

    #[test]
    fn queued_transport_writer_preserves_character_at_a_time_mode() {
        let writes = Arc::new(Mutex::new(Vec::new()));
        let mut writer = QueuedTransportWriter::spawn(
            "character-mode".to_string(),
            WriteCapture(writes.clone()),
            true,
            SessionEventQueue::new(),
        );

        writer.write(b"abc".to_vec()).expect("queue input");
        writer.close();

        assert_eq!(
            *writes.lock().expect("write capture lock"),
            vec![vec![b'a'], vec![b'b'], vec![b'c']]
        );
    }

    #[test]
    fn local_session_echoes_output() {
        if cfg!(target_os = "windows") {
            return;
        }

        let manager = SessionManager::new();
        let info = manager
            .create_local_session(LocalSessionConfig {
                name: "test".to_string(),
                shell_path: Some("/bin/sh".to_string()),
                shell_args: Vec::new(),
                working_dir: None,
                cols: 80,
                rows: 24,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("local session");

        manager
            .write(&info.id, b"printf nyaterm-transport-ready\\n\n")
            .expect("write");

        let output = collect_output(&manager, &info.id, Duration::from_secs(3));
        manager.close(&info.id).expect("close");

        assert!(
            String::from_utf8_lossy(&output).contains("nyaterm-transport-ready"),
            "output was: {}",
            String::from_utf8_lossy(&output)
        );
    }

    #[test]
    fn local_session_info_preserves_working_dir() {
        if cfg!(target_os = "windows") {
            return;
        }

        let dir = std::env::temp_dir().join(format!("nyaterm-local-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let manager = SessionManager::new();
        let info = manager
            .create_local_session(LocalSessionConfig {
                name: "cwd-test".to_string(),
                shell_path: Some("/bin/sh".to_string()),
                shell_args: Vec::new(),
                working_dir: Some(dir.clone()),
                cols: 80,
                rows: 24,
                pixel_width: 0,
                pixel_height: 0,
            })
            .expect("local session");
        let sessions = manager.list_sessions().expect("sessions");
        manager.close(&info.id).expect("close");
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(sessions[0].working_dir.as_ref(), Some(&dir));
    }

    #[test]
    fn local_background_command_uses_working_dir_and_exit_code() {
        if cfg!(target_os = "windows") {
            return;
        }

        let dir = std::env::temp_dir().join(format!("nyaterm-local-bg-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let output = run_local_command(
            "printf ready > marker.txt; printf output; exit 7",
            Some(dir.clone()),
            Duration::from_secs(3),
        )
        .expect("local command");
        let marker = std::fs::read_to_string(dir.join("marker.txt")).expect("marker");
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(marker, "ready");
        assert_eq!(output.stdout, "output");
        assert_eq!(output.exit_status, Some(7));
    }

    #[test]
    fn resize_updates_session_info() {
        if cfg!(target_os = "windows") {
            return;
        }

        let manager = SessionManager::new();
        let info = manager
            .create_local_session(LocalSessionConfig {
                shell_path: Some("/bin/sh".to_string()),
                ..Default::default()
            })
            .expect("local session");
        manager.resize(&info.id, 120, 32).expect("resize");
        let sessions = manager.list_sessions().expect("sessions");
        manager.close(&info.id).expect("close");

        assert_eq!(sessions[0].cols, 120);
        assert_eq!(sessions[0].rows, 32);
    }

    #[test]
    fn raw_tcp_session_echoes_output() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let port = listener.local_addr().expect("addr").port();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .expect("timeout");
            let mut buffer = [0_u8; 64];
            let read = stream.read(&mut buffer).expect("read");
            stream.write_all(b"echo:").expect("prefix");
            stream.write_all(&buffer[..read]).expect("echo");
        });

        let manager = SessionManager::new();
        let info = manager
            .create_telnet_session(TelnetSessionConfig {
                name: "raw".to_string(),
                host: "127.0.0.1".to_string(),
                port,
                raw_tcp: true,
                ..Default::default()
            })
            .expect("raw tcp");

        manager.write(&info.id, b"hello").expect("write");
        let output = collect_output_until(&manager, &info.id, "echo:hello", Duration::from_secs(3));
        manager.close(&info.id).expect("close");
        server.join().expect("server");

        assert!(String::from_utf8_lossy(&output).contains("echo:hello"));
    }

    #[test]
    fn telnet_session_negotiates_and_strips_iac() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let port = listener.local_addr().expect("addr").port();
        let (tx, rx) = mpsc::channel();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .expect("timeout");
            stream
                .write_all(&[IAC, WILL, OPT_SUPPRESS_GO_AHEAD, b'o', b'k'])
                .expect("write greeting");

            let mut seen = Vec::new();
            let started = Instant::now();
            while started.elapsed() < Duration::from_secs(3) {
                let mut buffer = [0_u8; 64];
                match stream.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => {
                        seen.extend_from_slice(&buffer[..read]);
                        if seen
                            .windows(3)
                            .any(|window| window == [IAC, DO, OPT_SUPPRESS_GO_AHEAD])
                        {
                            break;
                        }
                    }
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                        ) =>
                    {
                        continue;
                    }
                    Err(error) => panic!("server read failed: {error}"),
                }
            }
            tx.send(seen).expect("send seen");
        });

        let manager = SessionManager::new();
        let info = manager
            .create_telnet_session(TelnetSessionConfig {
                name: "telnet".to_string(),
                host: "127.0.0.1".to_string(),
                port,
                raw_tcp: false,
                send_sga: true,
                ..Default::default()
            })
            .expect("telnet");

        let output = collect_output_until(&manager, &info.id, "ok", Duration::from_secs(3));
        manager.close(&info.id).expect("close");
        server.join().expect("server");
        let seen = rx.recv().expect("seen");

        assert_eq!(String::from_utf8_lossy(&output), "ok");
        assert!(
            seen.windows(3)
                .any(|window| { window == [IAC, DO, OPT_SUPPRESS_GO_AHEAD] })
        );
    }

    #[test]
    fn serial_invalid_port_reports_open_error() {
        let manager = SessionManager::new();
        let port_name = if cfg!(target_os = "windows") {
            r"\\.\NyaTermMissingPort".to_string()
        } else {
            "/dev/nyaterm-missing-port".to_string()
        };

        let error = manager
            .create_serial_session(SerialSessionConfig {
                port_name: port_name.clone(),
                ..Default::default()
            })
            .expect_err("invalid port should not open");

        match error {
            SessionError::OpenSerial {
                port_name: actual, ..
            } => assert_eq!(actual, port_name),
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn serial_backspace_mode_remaps_delete_to_ctrl_h() {
        assert_eq!(remap_del_to_bs(b"a\x7fb"), b"a\x08b");
    }

    #[test]
    fn ssh_refused_connection_reports_create_error() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
        let port = listener.local_addr().expect("addr").port();
        drop(listener);

        let manager = SessionManager::new();
        let error = manager
            .create_ssh_session(SshSessionConfig {
                name: "ssh".to_string(),
                host: "127.0.0.1".to_string(),
                port,
                username: "tester".to_string(),
                password: Some("secret".to_string()),
                ..Default::default()
            })
            .expect_err("closed port should not open");

        match error {
            SessionError::CreateSsh { addr, .. } => assert_eq!(addr, format!("127.0.0.1:{port}")),
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn ssh_host_identifier_uses_openssh_port_format() {
        assert_eq!(ssh_host_identifier("example.com", 22), "example.com");
        assert_eq!(
            ssh_host_identifier("example.com", 2222),
            "[example.com]:2222"
        );
    }

    #[test]
    fn tunnel_config_validation_matches_tunnel_modes() {
        let mut config = SshTunnelConfig {
            id: "tunnel-1".to_string(),
            ssh_config: SshSessionConfig::default(),
            mode: SshTunnelMode::Local,
            bind_host: String::new(),
            listen_port: 0,
            target_host: None,
            target_port: None,
        };
        assert!(
            validate_tunnel_config(&config)
                .expect_err("missing target host")
                .to_string()
                .contains("target host")
        );

        config.target_host = Some("127.0.0.1".to_string());
        assert!(
            validate_tunnel_config(&config)
                .expect_err("missing target port")
                .to_string()
                .contains("target port")
        );

        config.target_port = Some(8080);
        validate_tunnel_config(&config).expect("local tunnel");

        config.mode = SshTunnelMode::Dynamic;
        config.target_host = None;
        config.target_port = None;
        validate_tunnel_config(&config).expect("dynamic tunnel");

        config.mode = SshTunnelMode::Remote;
        assert!(
            validate_tunnel_config(&config)
                .expect_err("remote missing target")
                .to_string()
                .contains("target host")
        );
        config.target_host = Some("127.0.0.1".to_string());
        config.target_port = Some(5432);
        validate_tunnel_config(&config).expect("remote tunnel");
    }

    #[test]
    fn forwarded_tcpip_dispatch_prefers_listener_specific_sender() {
        let (fallback_tx, _fallback_rx) = tokio_mpsc::unbounded_channel();
        let (specific_tx, _specific_rx) = tokio_mpsc::unbounded_channel();
        let dispatch = ForwardedTcpIpDispatch {
            fallback: Some(fallback_tx.clone()),
            by_listener: HashMap::from([(("127.0.0.1".to_string(), 2022), specific_tx.clone())]),
        };

        let exact =
            forwarded_tcpip_sender_for(&dispatch, "127.0.0.1", 2022).expect("specific sender");
        assert!(exact.same_channel(&specific_tx));

        let fallback =
            forwarded_tcpip_sender_for(&dispatch, "127.0.0.1", 2200).expect("fallback sender");
        assert!(fallback.same_channel(&fallback_tx));

        let empty = ForwardedTcpIpDispatch::default();
        assert!(forwarded_tcpip_sender_for(&empty, "127.0.0.1", 2022).is_none());
    }

    #[test]
    fn socks5_connect_parser_accepts_domain_requests() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        runtime.block_on(async {
            let (mut client, mut server) = tokio::io::duplex(128);
            let parser =
                tokio::spawn(async move { read_socks5_connect_request(&mut server).await });

            client
                .write_all(&[0x05, 0x01, 0x00])
                .await
                .expect("greeting");
            let mut method_reply = [0_u8; 2];
            client
                .read_exact(&mut method_reply)
                .await
                .expect("method reply");
            assert_eq!(method_reply, [0x05, 0x00]);

            let domain = b"example.com";
            let mut request = vec![0x05, 0x01, 0x00, 0x03, domain.len() as u8];
            request.extend_from_slice(domain);
            request.extend_from_slice(&443_u16.to_be_bytes());
            client.write_all(&request).await.expect("connect request");
            let mut connect_reply = [0_u8; 10];
            client
                .read_exact(&mut connect_reply)
                .await
                .expect("connect reply");
            assert_eq!(connect_reply, [0x05, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0]);

            let (host, port) = parser.await.expect("parser task").expect("parsed");
            assert_eq!(host, "example.com");
            assert_eq!(port, 443);
        });
    }

    #[test]
    fn process_parser_reads_legacy_rows() {
        let rows =
            "PROCESS\t42\t1\troot\tSs\t0.4\t1.2\t1234\t5678\t01:02\tsshd\t/usr/sbin/sshd -D\n";

        let processes = parse_process_output(rows);

        assert_eq!(processes.len(), 1);
        assert_eq!(processes[0].pid, 42);
        assert_eq!(processes[0].ppid, 1);
        assert_eq!(processes[0].user, "root");
        assert_eq!(processes[0].cpu_percent, 0.4);
        assert_eq!(processes[0].command_line, "/usr/sbin/sshd -D");
    }

    #[test]
    fn process_parser_preserves_command_lines_containing_tabs() {
        let rows = "PROCESS\t9\t1\troot\tS\t0\t0\t1\t2\t-\tawk\tawk\twith\ttabs\n";

        let processes = parse_process_output(rows);

        assert_eq!(processes.len(), 1);
        assert_eq!(processes[0].command_line, "awk\twith\ttabs");
    }

    #[test]
    fn process_parser_detects_unsupported_marker() {
        assert!(is_process_list_unsupported(
            "warning\nNYATERM_PROCESS_UNSUPPORTED\n"
        ));
        assert!(!is_process_list_unsupported(
            "PROCESS\t1\t0\troot\tS\t0\t0\t0\t0\t-\tsh\tsh\n"
        ));
    }

    #[test]
    fn process_signal_normalization_matches_legacy_allowlist() {
        assert_eq!(normalize_process_signal("sigterm").unwrap(), "TERM");
        assert_eq!(normalize_process_signal("9").unwrap(), "KILL");
        assert_eq!(normalize_process_signal("cont").unwrap(), "CONT");
        assert!(normalize_process_signal("USR1").is_err());
    }

    #[test]
    fn ssh_config_debug_redacts_password() {
        let config = SshSessionConfig {
            password: Some("super-secret".to_string()),
            ..Default::default()
        };
        let debug = format!("{config:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("super-secret"));
    }

    #[test]
    fn ssh_config_debug_redacts_key_material() {
        let config = SshSessionConfig {
            key_auth: Some(SshKeyAuthConfig {
                key_data: "-----BEGIN PRIVATE KEY-----secret-key".to_string(),
                cert_data: Some("ssh-ed25519-cert-v01@openssh.com secret-cert".to_string()),
                passphrase: Some("key-passphrase".to_string()),
            }),
            ..Default::default()
        };
        let debug = format!("{config:?}");
        assert!(debug.contains("<redacted>"));
        assert!(!debug.contains("secret-key"));
        assert!(!debug.contains("secret-cert"));
        assert!(!debug.contains("key-passphrase"));
    }

    #[test]
    fn ssh_config_debug_redacts_proxy_password() {
        let config = SshSessionConfig {
            proxy: Some(SshProxyConfig {
                protocol: "socks5".to_string(),
                host: "127.0.0.1".to_string(),
                port: 1080,
                command: None,
                username: Some("proxy-user".to_string()),
                password: Some("proxy-secret".to_string()),
            }),
            ..Default::default()
        };
        let debug = format!("{config:?}");
        assert!(debug.contains("<redacted>"));
        assert!(debug.contains("proxy-user"));
        assert!(!debug.contains("proxy-secret"));
    }

    #[test]
    fn ssh_client_config_disables_idle_timeout_and_maps_keepalive() {
        let config = SshSessionConfig {
            keep_alive_interval_secs: 45,
            ..Default::default()
        };

        let client_config = ssh_client_config(&config);

        assert_eq!(client_config.inactivity_timeout, None);
        assert_eq!(
            client_config.keepalive_interval,
            Some(Duration::from_secs(45))
        );
        assert_eq!(client_config.keepalive_max, 3);

        let disabled = SshSessionConfig {
            keep_alive_interval_secs: 0,
            ..Default::default()
        };
        let disabled_client_config = ssh_client_config(&disabled);

        assert_eq!(disabled_client_config.inactivity_timeout, None);
        assert_eq!(disabled_client_config.keepalive_interval, None);
    }

    #[test]
    fn local_config_defaults_to_unknown_pixel_dimensions() {
        let config = LocalSessionConfig::default();
        assert_eq!(config.cols, 80);
        assert_eq!(config.rows, 24);
        assert_eq!(config.pixel_width, 0);
        assert_eq!(config.pixel_height, 0);
    }

    #[test]
    fn local_pty_size_preserves_cell_and_pixel_dimensions() {
        let size = local_pty_size(132, 43, 1056, 688);
        assert_eq!(size.cols, 132);
        assert_eq!(size.rows, 43);
        assert_eq!(size.pixel_width, 1056);
        assert_eq!(size.pixel_height, 688);
    }

    #[test]
    fn ssh_pty_dimensions_clamp_to_positive_cells() {
        let dimensions = SshPtyDimensions::new(0, 0, 0, 0);
        assert_eq!(dimensions.cols, 1);
        assert_eq!(dimensions.rows, 1);
        assert_eq!(dimensions.pixel_width, 0);
        assert_eq!(dimensions.pixel_height, 0);

        let dimensions = SshPtyDimensions::new(132, 43, 1056, 688);
        assert_eq!(dimensions.cols, 132);
        assert_eq!(dimensions.rows, 43);
        assert_eq!(dimensions.pixel_width, 1056);
        assert_eq!(dimensions.pixel_height, 688);
    }

    #[test]
    fn ssh_pty_dimensions_use_config_size() {
        let config = SshSessionConfig {
            cols: 101,
            rows: 37,
            pixel_width: 808,
            pixel_height: 592,
            ..Default::default()
        };
        let dimensions = SshPtyDimensions::from_config(&config);
        assert_eq!(dimensions.cols, 101);
        assert_eq!(dimensions.rows, 37);
        assert_eq!(dimensions.pixel_width, 808);
        assert_eq!(dimensions.pixel_height, 592);
    }

    #[test]
    fn deferred_ssh_open_drain_keeps_writes_and_latest_resize() {
        let (tx, mut rx) = tokio_mpsc::unbounded_channel();
        tx.send(SshCommand::Write(b"before".to_vec())).unwrap();
        tx.send(SshCommand::Resize {
            cols: 100,
            rows: 30,
            pixel_width: 800,
            pixel_height: 600,
        })
        .unwrap();
        tx.send(SshCommand::Resize {
            cols: 132,
            rows: 43,
            pixel_width: 1056,
            pixel_height: 688,
        })
        .unwrap();
        tx.send(SshCommand::Write(b"after".to_vec())).unwrap();

        let mut dimensions = SshPtyDimensions::new(80, 24, 0, 0);
        let mut pending_writes = VecDeque::new();
        let should_close =
            drain_deferred_ssh_open_commands(&mut rx, &mut dimensions, &mut pending_writes);

        assert!(!should_close);
        assert_eq!(dimensions, SshPtyDimensions::new(132, 43, 1056, 688));
        assert_eq!(
            pending_writes.into_iter().collect::<Vec<_>>(),
            vec![b"before".to_vec(), b"after".to_vec()]
        );
    }

    #[test]
    fn deferred_ssh_open_drain_closes_before_shell_open() {
        let (tx, mut rx) = tokio_mpsc::unbounded_channel();
        tx.send(SshCommand::Write(b"queued".to_vec())).unwrap();
        tx.send(SshCommand::Close).unwrap();

        let mut dimensions = SshPtyDimensions::new(80, 24, 0, 0);
        let mut pending_writes = VecDeque::new();
        let should_close =
            drain_deferred_ssh_open_commands(&mut rx, &mut dimensions, &mut pending_writes);

        assert!(should_close);
        assert_eq!(
            pending_writes.into_iter().collect::<Vec<_>>(),
            vec![b"queued".to_vec()]
        );
    }

    #[test]
    fn deferred_ssh_open_drain_closes_on_disconnected_command_channel() {
        let (tx, mut rx) = tokio_mpsc::unbounded_channel();
        drop(tx);

        let mut dimensions = SshPtyDimensions::new(80, 24, 0, 0);
        let mut pending_writes = VecDeque::new();

        assert!(drain_deferred_ssh_open_commands(
            &mut rx,
            &mut dimensions,
            &mut pending_writes
        ));
        assert!(pending_writes.is_empty());
    }

    #[test]
    fn proxy_command_expansion_replaces_ssh_tokens() {
        let expanded = expand_proxy_command(
            Some("nc %h %p --user %r --literal %%"),
            "host name",
            2222,
            "user'name",
        )
        .expect("expanded command");

        #[cfg(windows)]
        {
            assert!(expanded.contains("\"host name\""));
            assert!(expanded.contains("2222"));
            assert!(expanded.contains("\"user'name\""));
        }
        #[cfg(not(windows))]
        {
            assert!(expanded.contains("'host name'"));
            assert!(expanded.contains("'2222'"));
            assert!(expanded.contains("'user'\\''name'"));
        }
        assert!(expanded.contains("--literal %"));
    }

    #[test]
    fn keyboard_interactive_prompt_classification_is_conservative() {
        let password_prompts = vec![client::Prompt {
            prompt: "Password: ".to_string(),
            echo: false,
        }];
        assert!(should_auto_fill_password_prompts(&password_prompts));
        assert!(!should_auto_fill_otp_prompts(&password_prompts));

        let otp_prompts = vec![client::Prompt {
            prompt: "Verification code: ".to_string(),
            echo: false,
        }];
        assert!(should_auto_fill_otp_prompts(&otp_prompts));
        assert!(!should_auto_fill_password_prompts(&otp_prompts));

        let selection_prompts = vec![client::Prompt {
            prompt: "Choose MFA method: ".to_string(),
            echo: true,
        }];
        assert!(!should_auto_fill_otp_prompts(&selection_prompts));
        assert!(!should_auto_fill_password_prompts(&selection_prompts));
    }

    #[derive(Default)]
    struct BatchKeyboardInteractiveProvider {
        requests: Mutex<Vec<SshKeyboardInteractiveRequest>>,
    }

    impl SshCredentialProvider for BatchKeyboardInteractiveProvider {
        fn request_secret(&self, _prompt: &SshCredentialPrompt) -> Result<Option<String>, String> {
            panic!("batch keyboard-interactive should not fall back to request_secret")
        }

        fn request_keyboard_interactive(
            &self,
            request: &SshKeyboardInteractiveRequest,
        ) -> Result<Option<Vec<String>>, String> {
            self.requests.lock().unwrap().push(request.clone());
            Ok(Some(vec!["alice".to_string(), "123456".to_string()]))
        }
    }

    #[test]
    fn keyboard_interactive_requests_are_delivered_as_one_challenge() {
        let provider = Arc::new(BatchKeyboardInteractiveProvider::default());
        let config = SshSessionConfig {
            name: "Production".to_string(),
            host: "host.example.com".to_string(),
            port: 2222,
            username: "root".to_string(),
            otp_id: Some("otp-1".to_string()),
            credential_provider: Some(provider.clone()),
            ..SshSessionConfig::default()
        };
        let responses = request_keyboard_interactive_responses(
            &config,
            "Login verification",
            "Complete both fields",
            vec![
                client::Prompt {
                    prompt: "Account:".to_string(),
                    echo: true,
                },
                client::Prompt {
                    prompt: "Code:".to_string(),
                    echo: false,
                },
            ],
            2,
        )
        .expect("keyboard-interactive responses");

        assert_eq!(responses, ["alice", "123456"]);
        let requests = provider.requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        assert_eq!(request.connection_name, "Production");
        assert_eq!(request.name, "Login verification");
        assert_eq!(request.instructions, "Complete both fields");
        assert_eq!(request.round, 2);
        assert_eq!(request.otp_id.as_deref(), Some("otp-1"));
        assert_eq!(request.prompts.len(), 2);
        assert!(request.prompts[0].echo);
        assert!(!request.prompts[1].echo);
    }

    #[derive(Default)]
    struct LegacyCredentialProvider {
        prompts: Mutex<Vec<SshCredentialPrompt>>,
    }

    impl SshCredentialProvider for LegacyCredentialProvider {
        fn request_secret(&self, prompt: &SshCredentialPrompt) -> Result<Option<String>, String> {
            self.prompts.lock().unwrap().push(prompt.clone());
            Ok(Some("response".to_string()))
        }
    }

    #[test]
    fn keyboard_interactive_default_provider_keeps_single_prompt_compatibility() {
        let provider = LegacyCredentialProvider::default();
        let request = SshKeyboardInteractiveRequest {
            host: "host.example.com".to_string(),
            port: 22,
            username: "root".to_string(),
            connection_name: "Production".to_string(),
            name: "Verification".to_string(),
            instructions: "Use both factors".to_string(),
            round: 3,
            prompts: vec![
                SshKeyboardInteractivePrompt {
                    prompt: "Password:".to_string(),
                    echo: false,
                },
                SshKeyboardInteractivePrompt {
                    prompt: "Code:".to_string(),
                    echo: false,
                },
            ],
            otp_id: None,
        };

        let responses = provider
            .request_keyboard_interactive(&request)
            .expect("fallback responses")
            .expect("not cancelled");

        assert_eq!(responses, ["response", "response"]);
        let prompts = provider.prompts.lock().unwrap();
        assert_eq!(prompts.len(), 2);
        assert_eq!(prompts[0].attempt, 3);
        assert_eq!(
            prompts[0].prompt_text.as_deref(),
            Some("Verification\nUse both factors\nPassword:")
        );
    }

    struct ShortKeyboardInteractiveProvider;

    impl SshCredentialProvider for ShortKeyboardInteractiveProvider {
        fn request_secret(&self, _prompt: &SshCredentialPrompt) -> Result<Option<String>, String> {
            Ok(None)
        }

        fn request_keyboard_interactive(
            &self,
            _request: &SshKeyboardInteractiveRequest,
        ) -> Result<Option<Vec<String>>, String> {
            Ok(Some(vec!["only-one".to_string()]))
        }
    }

    #[test]
    fn keyboard_interactive_rejects_response_count_mismatches() {
        let config = SshSessionConfig {
            credential_provider: Some(Arc::new(ShortKeyboardInteractiveProvider)),
            ..SshSessionConfig::default()
        };
        let error = request_keyboard_interactive_responses(
            &config,
            "Verification",
            "",
            vec![
                client::Prompt {
                    prompt: "First:".to_string(),
                    echo: true,
                },
                client::Prompt {
                    prompt: "Second:".to_string(),
                    echo: false,
                },
            ],
            1,
        )
        .expect_err("response count mismatch");

        assert!(error.to_string().contains("1 responses for 2 prompts"));
    }

    fn collect_output(manager: &SessionManager, session_id: &str, timeout: Duration) -> Vec<u8> {
        collect_output_until(manager, session_id, "nyaterm-transport-ready", timeout)
    }

    fn collect_output_until(
        manager: &SessionManager,
        session_id: &str,
        needle: &str,
        timeout: Duration,
    ) -> Vec<u8> {
        let started = Instant::now();
        let mut output = Vec::new();
        while started.elapsed() < timeout {
            for event in manager.drain_events(16).expect("events").events {
                match event {
                    SessionEvent::Output {
                        session_id: event_session_id,
                        data,
                    } if event_session_id == session_id => output.extend(data),
                    SessionEvent::OutputDropped { .. } => {}
                    SessionEvent::Error { message, .. } => panic!("session error: {message}"),
                    _ => {}
                }
            }
            if String::from_utf8_lossy(&output).contains(needle) {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        output
    }

    #[test]
    fn session_event_queue_keeps_consecutive_output_chunks_separate() {
        let queue = SessionEventQueue::new();
        queue.push(SessionEvent::Output {
            session_id: "a".to_string(),
            data: b"hello ".to_vec(),
        });
        queue.push(SessionEvent::Output {
            session_id: "a".to_string(),
            data: b"world".to_vec(),
        });

        let drain = queue.drain(8);
        assert_eq!(drain.events.len(), 2);
        assert_eq!(drain.stats.drained_output_bytes, 11);
        assert_eq!(drain.stats.queued_output_bytes, 0);
        match &drain.events[0] {
            SessionEvent::Output { session_id, data } => {
                assert_eq!(session_id, "a");
                assert_eq!(data, b"hello ");
            }
            event => panic!("unexpected event: {event:?}"),
        }
        match &drain.events[1] {
            SessionEvent::Output { session_id, data } => {
                assert_eq!(session_id, "a");
                assert_eq!(data, b"world");
            }
            event => panic!("unexpected event: {event:?}"),
        }
    }

    #[test]
    fn session_event_queue_keeps_sessions_separate() {
        let queue = SessionEventQueue::new();
        queue.push(SessionEvent::Output {
            session_id: "a".to_string(),
            data: b"a1".to_vec(),
        });
        queue.push(SessionEvent::Output {
            session_id: "b".to_string(),
            data: b"b1".to_vec(),
        });
        queue.push(SessionEvent::Output {
            session_id: "a".to_string(),
            data: b"a2".to_vec(),
        });

        let drain = queue.drain(8);
        assert_eq!(drain.events.len(), 3);
        assert!(matches!(
            &drain.events[0],
            SessionEvent::Output { session_id, data } if session_id == "a" && data == b"a1"
        ));
        assert!(matches!(
            &drain.events[1],
            SessionEvent::Output { session_id, data } if session_id == "b" && data == b"b1"
        ));
        assert!(matches!(
            &drain.events[2],
            SessionEvent::Output { session_id, data } if session_id == "a" && data == b"a2"
        ));
    }

    #[test]
    fn session_event_queue_respects_output_drain_budget() {
        let queue = SessionEventQueue::new();
        queue.push(SessionEvent::Output {
            session_id: "a".to_string(),
            data: vec![b'a'; 128],
        });
        queue.push(SessionEvent::Output {
            session_id: "b".to_string(),
            data: vec![b'b'; 128],
        });

        let drain = queue.drain_with_output_budget(8, Some(200));
        assert_eq!(drain.events.len(), 2);
        assert_eq!(drain.stats.drained_output_bytes, 200);
        assert_eq!(drain.stats.queued_output_bytes, 56);
        assert!(matches!(
            &drain.events[0],
            SessionEvent::Output { session_id, data } if session_id == "a" && data.len() == 128
        ));
        assert!(matches!(
            &drain.events[1],
            SessionEvent::Output { session_id, data } if session_id == "b" && data.len() == 72
        ));

        let drain = queue.drain_with_output_budget(8, Some(200));
        assert_eq!(drain.events.len(), 1);
        assert_eq!(drain.stats.drained_output_bytes, 56);
        assert_eq!(drain.stats.queued_output_bytes, 0);
    }

    #[test]
    fn session_event_queue_zero_output_budget_does_not_drain_output() {
        let queue = SessionEventQueue::new();
        queue.push(SessionEvent::Output {
            session_id: "a".to_string(),
            data: b"hello".to_vec(),
        });

        let drain = queue.drain_with_output_budget(8, Some(0));
        assert!(drain.events.is_empty());
        assert_eq!(drain.stats.drained_output_bytes, 0);
        assert_eq!(drain.stats.queued_output_bytes, 5);

        let drain = queue.drain_with_output_budget(8, Some(8));
        assert_eq!(drain.events.len(), 1);
        assert_eq!(drain.stats.drained_output_bytes, 5);
        assert_eq!(drain.stats.queued_output_bytes, 0);
    }

    #[test]
    fn session_event_queue_zero_output_budget_can_drain_drop_marker() {
        let queue = SessionEventQueue::new();
        queue.push(SessionEvent::Output {
            session_id: "a".to_string(),
            data: vec![b'x'; SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT + 32],
        });

        let drain = queue.drain_with_output_budget(8, Some(0));
        assert_eq!(drain.events.len(), 1);
        assert!(matches!(
            &drain.events[0],
            SessionEvent::OutputDropped { session_id, bytes } if session_id == "a" && *bytes == 32
        ));
        assert_eq!(drain.stats.drained_output_bytes, 0);
        assert_eq!(
            drain.stats.queued_output_bytes,
            SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT
        );

        let drain = queue.drain_with_output_budget(8, Some(8));
        assert_eq!(drain.events.len(), 1);
        assert_eq!(drain.stats.drained_output_bytes, 8);
        assert_eq!(
            drain.stats.queued_output_bytes,
            SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT - 8
        );
    }

    #[test]
    fn session_event_queue_trims_oversized_output_and_reports_drop() {
        let queue = SessionEventQueue::new();
        queue.push(SessionEvent::Output {
            session_id: "a".to_string(),
            data: vec![b'x'; SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT + 32],
        });

        let drain = queue.drain(8);
        assert_eq!(drain.events.len(), 2);
        assert!(matches!(
            &drain.events[0],
            SessionEvent::OutputDropped { session_id, bytes } if session_id == "a" && *bytes == 32
        ));
        assert!(matches!(
            &drain.events[1],
            SessionEvent::Output { data, .. } if data.len() == SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT
        ));
        assert_eq!(drain.stats.dropped_output_bytes, 32);
    }

    #[test]
    fn session_event_queue_keeps_adjacent_output_chunks_separate() {
        let queue = SessionEventQueue::new();
        queue.push(SessionEvent::Output {
            session_id: "a".to_string(),
            data: vec![b'a'; SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT - 8],
        });
        queue.push(SessionEvent::Output {
            session_id: "a".to_string(),
            data: vec![b'b'; 16],
        });

        let drain = queue.drain(8);
        assert_eq!(drain.events.len(), 2);
        assert!(matches!(
            &drain.events[0],
            SessionEvent::Output { session_id, data } if session_id == "a"
                && data.len() == SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT - 8
                && data[0] == b'a'
        ));
        assert!(matches!(
            &drain.events[1],
            SessionEvent::Output { session_id, data } if session_id == "a"
                && data.len() == 16
                && *data.last().unwrap() == b'b'
        ));
        assert_eq!(drain.stats.dropped_output_bytes, 0);
    }

    #[test]
    fn session_event_queue_reports_global_limit_drops_for_trimmed_session() {
        let queue = SessionEventQueue::new();
        let event_count =
            (SESSION_EVENT_QUEUE_OUTPUT_LIMIT / SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT) + 2;
        for index in 0..event_count {
            queue.push(SessionEvent::Output {
                session_id: format!("session-{index}"),
                data: vec![b'x'; SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT],
            });
        }

        let drain = queue.drain(event_count + 8);
        let dropped = drain
            .events
            .iter()
            .filter_map(|event| match event {
                SessionEvent::OutputDropped { session_id, bytes } => {
                    Some((session_id.as_str(), *bytes))
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(
            dropped,
            vec![
                ("session-0", SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT),
                ("session-1", SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT),
            ]
        );
        assert_eq!(
            drain.stats.drained_output_bytes,
            SESSION_EVENT_QUEUE_OUTPUT_LIMIT
        );
        assert_eq!(
            drain.stats.dropped_output_bytes,
            SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT * 2
        );
    }
}

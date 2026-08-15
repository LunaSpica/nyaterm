use std::fmt;
use std::future::Future;
use std::marker::PhantomData;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, mpsc};
use std::task::{Context, Poll};

use futures::channel::oneshot;
use nyaterm_core::{
    AiSettings, AppSettingsSummary, CloudSyncSettings, CloudSyncState, CommandHistoryEntry,
    ConnectionStore, Group, KeywordHighlightConfig, OtpEntry, ProxyConfig, ProxyGroup,
    QuickCommand, QuickCommandCategory, SavedConnection, SavedCredential, SavedPassword, SshKey,
    StorageError, TranslationSettings, TunnelConfig, TunnelGroup,
};

const STORE_QUEUE_CAPACITY: usize = 256;

pub type RequestId = u64;

#[derive(Clone)]
pub struct StoreConfig {
    pub config_dir: PathBuf,
    pub portable_key_path: Option<PathBuf>,
}

impl fmt::Debug for StoreConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StoreConfig")
            .field("config_dir", &self.config_dir)
            .field(
                "portable_key_path",
                &self.portable_key_path.as_ref().map(|_| "<configured>"),
            )
            .finish()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreDomain {
    Bootstrap,
    Settings,
    Connections,
    Commands,
    Security,
    Tunnels,
    CloudSync,
    Sessions,
    Ai,
    Terminal,
    Transfers,
    Barrier,
}

#[derive(Clone, PartialEq, Eq)]
pub struct StoreOperationError {
    category: &'static str,
    message: String,
}

impl StoreOperationError {
    pub fn category(&self) -> &'static str {
        self.category
    }

    pub fn user_message(&self) -> &str {
        &self.message
    }

    fn unavailable(message: impl Into<String>) -> Self {
        Self {
            category: "unavailable",
            message: message.into(),
        }
    }
}

impl From<StorageError> for StoreOperationError {
    fn from(error: StorageError) -> Self {
        let category = match &error {
            StorageError::CreateDir { .. } => "create_dir",
            StorageError::Open { .. } => "open",
            StorageError::Crypto(_) | StorageError::MissingMasterKey => "crypto",
            StorageError::InvalidData(_) | StorageError::PortableSnapshotEntity { .. } => {
                "invalid_data"
            }
            _ => "storage",
        };
        Self {
            category,
            message: error.to_string(),
        }
    }
}

impl fmt::Debug for StoreOperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StoreOperationError")
            .field("category", &self.category)
            .field("message", &"<redacted>")
            .finish()
    }
}

impl fmt::Display for StoreOperationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for StoreOperationError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreSubmitError {
    QueueFull,
    Disconnected,
}

impl fmt::Display for StoreSubmitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueueFull => formatter.write_str("the storage request queue is full"),
            Self::Disconnected => formatter.write_str("the storage worker is unavailable"),
        }
    }
}

impl std::error::Error for StoreSubmitError {}

pub trait StoreRequest: Send + 'static {
    type Response: Send + 'static;

    fn domain(&self) -> StoreDomain;

    fn execute(self, store: &ConnectionStore) -> Result<Self::Response, StorageError>;
}

pub struct StoreFnRequest<F, T> {
    domain: StoreDomain,
    operation: F,
    response: PhantomData<fn() -> T>,
}

pub fn store_request<F, T>(domain: StoreDomain, operation: F) -> StoreFnRequest<F, T>
where
    F: FnOnce(&ConnectionStore) -> Result<T, StorageError> + Send + 'static,
    T: Send + 'static,
{
    StoreFnRequest {
        domain,
        operation,
        response: PhantomData,
    }
}

impl<F, T> StoreRequest for StoreFnRequest<F, T>
where
    F: FnOnce(&ConnectionStore) -> Result<T, StorageError> + Send + 'static,
    T: Send + 'static,
{
    type Response = T;

    fn domain(&self) -> StoreDomain {
        self.domain
    }

    fn execute(self, store: &ConnectionStore) -> Result<Self::Response, StorageError> {
        (self.operation)(store)
    }
}

pub struct StoreEvent<T> {
    pub request_id: RequestId,
    pub domain: StoreDomain,
    pub generation: u64,
    pub outcome: Result<T, StoreOperationError>,
}

impl<T> fmt::Debug for StoreEvent<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("StoreEvent")
            .field("request_id", &self.request_id)
            .field("domain", &self.domain)
            .field("generation", &self.generation)
            .field("outcome", &self.outcome.as_ref().map(|_| "<value>"))
            .finish()
    }
}

pub struct StoreTask<T> {
    request_id: RequestId,
    receiver: oneshot::Receiver<StoreEvent<T>>,
}

impl<T> StoreTask<T> {
    pub fn request_id(&self) -> RequestId {
        self.request_id
    }
}

impl<T> Future for StoreTask<T> {
    type Output = StoreEvent<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.receiver).poll(cx) {
            Poll::Ready(Ok(event)) => Poll::Ready(event),
            Poll::Ready(Err(_)) => Poll::Ready(StoreEvent {
                request_id: self.request_id,
                domain: StoreDomain::Barrier,
                generation: 0,
                outcome: Err(StoreOperationError::unavailable(
                    "the storage worker stopped before returning a result",
                )),
            }),
            Poll::Pending => Poll::Pending,
        }
    }
}

type WorkerJob = Box<dyn FnOnce(Result<&ConnectionStore, &StoreOperationError>) + Send>;

enum WorkerMessage {
    Execute(WorkerJob),
}

#[derive(Clone)]
pub struct StoreUiClient {
    sender: mpsc::SyncSender<WorkerMessage>,
    next_request_id: Arc<AtomicU64>,
}

impl StoreUiClient {
    pub fn try_submit<R: StoreRequest>(
        &self,
        generation: u64,
        request: R,
    ) -> Result<StoreTask<R::Response>, StoreSubmitError> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let domain = request.domain();
        let (sender, receiver) = oneshot::channel();
        let job = Box::new(
            move |store: Result<&ConnectionStore, &StoreOperationError>| {
                let outcome = match store {
                    Ok(store) => request.execute(store).map_err(StoreOperationError::from),
                    Err(error) => Err(error.clone()),
                };
                let _ = sender.send(StoreEvent {
                    request_id,
                    domain,
                    generation,
                    outcome,
                });
            },
        );
        match self.sender.try_send(WorkerMessage::Execute(job)) {
            Ok(()) => Ok(StoreTask {
                request_id,
                receiver,
            }),
            Err(mpsc::TrySendError::Full(_)) => Err(StoreSubmitError::QueueFull),
            Err(mpsc::TrySendError::Disconnected(_)) => Err(StoreSubmitError::Disconnected),
        }
    }
}

#[derive(Clone)]
pub struct StoreBlockingClient {
    sender: mpsc::SyncSender<WorkerMessage>,
    next_request_id: Arc<AtomicU64>,
}

impl StoreBlockingClient {
    pub fn request<R: StoreRequest>(
        &self,
        generation: u64,
        request: R,
    ) -> Result<StoreEvent<R::Response>, StoreSubmitError> {
        let request_id = self.next_request_id.fetch_add(1, Ordering::Relaxed);
        let domain = request.domain();
        let (sender, receiver) = mpsc::sync_channel(1);
        let job = Box::new(
            move |store: Result<&ConnectionStore, &StoreOperationError>| {
                let outcome = match store {
                    Ok(store) => request.execute(store).map_err(StoreOperationError::from),
                    Err(error) => Err(error.clone()),
                };
                let _ = sender.send(StoreEvent {
                    request_id,
                    domain,
                    generation,
                    outcome,
                });
            },
        );
        self.sender
            .send(WorkerMessage::Execute(job))
            .map_err(|_| StoreSubmitError::Disconnected)?;
        receiver.recv().map_err(|_| StoreSubmitError::Disconnected)
    }
}

pub struct StoreRuntime {
    ui_client: StoreUiClient,
    blocking_client: StoreBlockingClient,
}

impl StoreRuntime {
    pub fn spawn(config: StoreConfig) -> Result<Self, std::io::Error> {
        let (sender, receiver) = mpsc::sync_channel(STORE_QUEUE_CAPACITY);
        let next_request_id = Arc::new(AtomicU64::new(1));
        std::thread::Builder::new()
            .name("nyaterm-store".to_string())
            .spawn(move || store_worker(config, receiver))?;
        Ok(Self {
            ui_client: StoreUiClient {
                sender: sender.clone(),
                next_request_id: next_request_id.clone(),
            },
            blocking_client: StoreBlockingClient {
                sender: sender.clone(),
                next_request_id,
            },
        })
    }

    pub fn ui_client(&self) -> StoreUiClient {
        self.ui_client.clone()
    }

    pub fn blocking_client(&self) -> StoreBlockingClient {
        self.blocking_client.clone()
    }
}

fn store_worker(config: StoreConfig, receiver: mpsc::Receiver<WorkerMessage>) {
    let store =
        ConnectionStore::open_with_portable_key_path(config.config_dir, config.portable_key_path)
            .map_err(StoreOperationError::from);
    while let Ok(message) = receiver.recv() {
        match message {
            WorkerMessage::Execute(job) => job(store.as_ref()),
        }
    }
}

pub struct BootstrapSnapshot {
    pub database_path: PathBuf,
    pub connections: Vec<SavedConnection>,
    pub connection_groups: Vec<Group>,
    pub ssh_keys: Vec<SshKey>,
    pub otp_entries: Vec<OtpEntry>,
    pub saved_passwords: Vec<SavedPassword>,
    pub saved_credentials: Vec<SavedCredential>,
    pub tunnels: Vec<TunnelConfig>,
    pub tunnel_groups: Vec<TunnelGroup>,
    pub proxies: Vec<ProxyConfig>,
    pub proxy_groups: Vec<ProxyGroup>,
    pub quick_commands: Vec<QuickCommand>,
    pub quick_command_categories: Vec<QuickCommandCategory>,
    pub command_history: Vec<CommandHistoryEntry>,
    pub keyword_highlights: KeywordHighlightConfig,
    pub settings: AppSettingsSummary,
    pub cloud_sync_settings: CloudSyncSettings,
    pub cloud_sync_state: CloudSyncState,
    pub translation_settings: TranslationSettings,
    pub ai_settings: AiSettings,
    pub ai_session_count: usize,
    pub ai_message_count: usize,
    pub ai_audit_count: usize,
}

pub struct LoadBootstrap;

impl StoreRequest for LoadBootstrap {
    type Response = BootstrapSnapshot;

    fn domain(&self) -> StoreDomain {
        StoreDomain::Bootstrap
    }

    fn execute(self, store: &ConnectionStore) -> Result<Self::Response, StorageError> {
        let sessions = store.load_sessions()?;
        let quick_commands = store.load_quick_commands()?;
        let ai_history = store.load_ai_history()?;
        Ok(BootstrapSnapshot {
            database_path: store.db_path().to_path_buf(),
            connections: sessions.connections,
            connection_groups: sessions.groups,
            ssh_keys: store.list_ssh_keys()?,
            otp_entries: store.list_otp_entries()?,
            saved_passwords: store.list_passwords()?,
            saved_credentials: store.list_credentials()?,
            tunnels: store.list_tunnels()?,
            tunnel_groups: store.list_tunnel_groups()?,
            proxies: store.list_proxies()?,
            proxy_groups: store.list_proxy_groups()?,
            quick_commands: quick_commands.commands,
            quick_command_categories: quick_commands.categories,
            command_history: store.list_command_history(64)?,
            keyword_highlights: store.load_keyword_highlights()?,
            settings: store.load_app_settings_summary()?,
            cloud_sync_settings: store.load_cloud_sync_settings()?,
            cloud_sync_state: store.load_cloud_sync_state()?,
            translation_settings: store.load_translation_settings()?,
            ai_settings: store.load_ai_settings()?,
            ai_session_count: ai_history.sessions.len(),
            ai_message_count: ai_history.messages.len(),
            ai_audit_count: store.list_ai_audit_logs(None)?.len(),
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FlushBarrier;

impl StoreRequest for FlushBarrier {
    type Response = ();

    fn domain(&self) -> StoreDomain {
        StoreDomain::Barrier
    }

    fn execute(self, _store: &ConnectionStore) -> Result<Self::Response, StorageError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::{FlushBarrier, LoadBootstrap, StoreConfig, StoreRuntime};

    fn temp_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "nyaterm-store-runtime-{label}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ))
    }

    use std::path::PathBuf;

    #[test]
    fn bootstrap_and_barrier_run_on_the_store_worker() {
        let config_dir = temp_dir("bootstrap");
        let runtime = StoreRuntime::spawn(StoreConfig {
            config_dir: config_dir.clone(),
            portable_key_path: None,
        })
        .expect("spawn runtime");
        let client = runtime.blocking_client();
        let bootstrap = client
            .request(0, LoadBootstrap)
            .expect("receive bootstrap")
            .outcome
            .expect("load bootstrap");
        assert!(bootstrap.connections.is_empty());
        client
            .request(0, FlushBarrier)
            .expect("receive barrier")
            .outcome
            .expect("flush barrier");
        drop(runtime);
        std::fs::remove_dir_all(config_dir).ok();
    }

    #[test]
    fn request_ids_are_monotonic_across_clients() {
        let config_dir = temp_dir("request-ids");
        let runtime = StoreRuntime::spawn(StoreConfig {
            config_dir: config_dir.clone(),
            portable_key_path: None,
        })
        .expect("spawn runtime");
        let first = runtime
            .blocking_client()
            .request(0, FlushBarrier)
            .expect("first request");
        let second = runtime
            .blocking_client()
            .request(0, FlushBarrier)
            .expect("second request");
        assert!(second.request_id > first.request_id);
        drop(runtime);
        std::fs::remove_dir_all(config_dir).ok();
    }
}

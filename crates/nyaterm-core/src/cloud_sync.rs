use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::{
    Engine,
    engine::general_purpose::{STANDARD as BASE64_STANDARD, URL_SAFE_NO_PAD},
};
use hmac::{Hmac, Mac, digest::KeyInit as HmacKeyInit};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use time::{OffsetDateTime, macros::format_description};

use crate::{PortableSnapshotError, RawPortableSnapshot};

mod gc;
mod protocol;

pub use gc::{
    SYNC_SNAPSHOT_GC_GRACE_PERIOD, SYNC_SNAPSHOT_KEEP_RECENT, cleanup_sync_snapshots_with_remote,
};

type HmacSha256 = Hmac<Sha256>;

pub const SYNC_CURRENT_FILE: &str = "sync/current.redb.enc";
pub const SYNC_LATEST_FILE: &str = "sync/latest.redb";
pub const SYNC_SNAPSHOTS_DIR: &str = "sync/snapshots/";
pub const MASKED_SECRET_VALUE: &str = "__SET__";
pub const CLOUD_SYNC_HISTORY_DOMAIN: &str = "cloud_sync.history";
pub const CLOUD_SYNC_HISTORY_EVENT: &str = "entry";
pub const CLOUD_SYNC_HISTORY_LIMIT: usize = 100;
pub const REMOTE_SYNC_POINTER_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Error)]
pub enum CloudSyncError {
    #[error("cloud sync is disabled")]
    Disabled,
    #[error("cloud sync conflict detected: {}", .0.message)]
    Conflict(Box<CloudConflictPreview>),
    #[error("remote snapshot is newer than local state; pull first")]
    RemoteNewer,
    #[error("no remote sync snapshot found")]
    NoRemoteSnapshot,
    #[error("no newer remote sync snapshot is available")]
    NoNewerRemoteSnapshot,
    #[error(
        "remote sync metadata is inconsistent: latest points to {revision} but the referenced snapshot is missing"
    )]
    SnapshotMissing { revision: String },
    #[error(
        "remote sync snapshot revision mismatch: latest points to {pointer_revision} but snapshot contains {snapshot_revision}"
    )]
    RevisionMismatch {
        pointer_revision: String,
        snapshot_revision: String,
    },
    #[error("remote sync snapshot hash mismatch: expected {expected} but got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error(
        "remote sync was updated by another device: expected {expected_revision:?} but found {actual_revision:?}"
    )]
    ConcurrentUpdate {
        expected_revision: Option<String>,
        actual_revision: Option<String>,
    },
    #[error("remote sync snapshot {revision} is corrupted")]
    CorruptedSnapshot { revision: String },
    #[error("invalid remote path '{path}'")]
    InvalidRemotePath { path: String },
    #[error("cloud sync remote error: {0}")]
    Remote(String),
    #[error("failed to create cloud sync directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to read cloud sync file {path}: {source}")]
    ReadFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write cloud sync file {path}: {source}")]
    WriteFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("local cloud-sync store error: {0}")]
    LocalStore(String),
    #[error("portable snapshot error: {0}")]
    PortableSnapshot(#[from] PortableSnapshotError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("base64 error: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteSyncPointer {
    #[serde(default = "default_remote_sync_pointer_schema_version")]
    pub schema_version: u32,
    pub revision_id: String,
    pub created_at_ms: u64,
    pub payload_hash: String,
    pub device_id: String,
    pub app_version: String,
}

fn default_remote_sync_pointer_schema_version() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudSyncState {
    #[serde(default = "uuid_v4")]
    pub device_id: String,
    #[serde(default)]
    pub last_synced_payload_hash: Option<String>,
    #[serde(default)]
    pub last_applied_remote_revision: Option<String>,
    #[serde(default)]
    pub last_checked_at_ms: Option<u64>,
    #[serde(default)]
    pub last_synced_at_ms: Option<u64>,
}

impl Default for CloudSyncState {
    fn default() -> Self {
        Self {
            device_id: uuid_v4(),
            last_synced_payload_hash: None,
            last_applied_remote_revision: None,
            last_checked_at_ms: None,
            last_synced_at_ms: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CloudConflictKind {
    #[default]
    ContentConflict,
    RemoteInconsistent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudConflictPreview {
    pub detected_at_ms: u64,
    pub provider: String,
    #[serde(default)]
    pub kind: CloudConflictKind,
    pub local_payload_hash: String,
    pub remote_payload_hash: String,
    pub remote_revision: String,
    pub remote_created_at_ms: u64,
    pub remote_device_id: String,
    #[serde(default)]
    pub recovery_revision: Option<String>,
    #[serde(default)]
    pub recovery_payload_hash: Option<String>,
    #[serde(default)]
    pub recovery_created_at_ms: Option<u64>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudSyncStatus {
    pub enabled: bool,
    pub provider: String,
    pub state: String,
    pub message: String,
    pub current_operation: Option<String>,
    pub last_checked_at_ms: Option<u64>,
    pub last_synced_at_ms: Option<u64>,
    pub conflict: Option<CloudConflictPreview>,
}

impl Default for CloudSyncStatus {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "local_directory".to_string(),
            state: "idle".to_string(),
            message: String::new(),
            current_operation: None,
            last_checked_at_ms: None,
            last_synced_at_ms: None,
            conflict: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudSyncHistoryEntry {
    #[serde(default = "uuid_v4")]
    pub id: String,
    pub timestamp_ms: u64,
    pub kind: String,
    pub status: String,
    pub trigger: String,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub revision: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    pub message: String,
}

impl CloudSyncHistoryEntry {
    pub fn sync(
        status: impl Into<String>,
        trigger: impl Into<String>,
        provider: Option<String>,
        revision: Option<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid_v4(),
            timestamp_ms: current_time_ms(),
            kind: "sync".to_string(),
            status: status.into(),
            trigger: trigger.into(),
            provider,
            revision,
            duration_ms: None,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct WebdavSyncSettings {
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub root: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct S3SyncSettings {
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub bucket: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub root: String,
    #[serde(default)]
    pub access_key_id: Option<String>,
    #[serde(default)]
    pub secret_access_key: Option<String>,
    #[serde(default)]
    pub session_token: Option<String>,
    #[serde(default)]
    pub virtual_host_style: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GiteeSnippetSyncSettings {
    #[serde(default = "default_gitee_api_endpoint")]
    pub api_endpoint: String,
    #[serde(default)]
    pub gist_id: String,
    #[serde(default)]
    pub access_token: Option<String>,
}

impl Default for GiteeSnippetSyncSettings {
    fn default() -> Self {
        Self {
            api_endpoint: default_gitee_api_endpoint(),
            gist_id: String::new(),
            access_token: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct OAuthDriveSyncSettings {
    #[serde(default)]
    pub root: String,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AliyunDriveSyncSettings {
    #[serde(default)]
    pub root: String,
    #[serde(default)]
    pub access_token: Option<String>,
    #[serde(default)]
    pub refresh_token: Option<String>,
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default = "default_aliyun_drive_type")]
    pub drive_type: String,
}

impl Default for AliyunDriveSyncSettings {
    fn default() -> Self {
        Self {
            root: String::new(),
            access_token: None,
            refresh_token: None,
            client_id: None,
            client_secret: None,
            drive_type: default_aliyun_drive_type(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct GithubGistSyncSettings {
    #[serde(default)]
    pub gist_id: String,
    #[serde(default)]
    pub access_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudSyncSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default = "default_remote_root")]
    pub remote_root: String,
    #[serde(default = "default_device_name")]
    pub device_name: String,
    #[serde(default = "default_true")]
    pub auto_check_on_startup: bool,
    #[serde(default = "default_true")]
    pub auto_push_on_change: bool,
    #[serde(default = "default_true")]
    pub auto_pull_remote_changes: bool,
    #[serde(default = "default_sync_debounce_seconds")]
    pub sync_debounce_seconds: u64,
    #[serde(default)]
    pub webdav: WebdavSyncSettings,
    #[serde(default)]
    pub s3: S3SyncSettings,
    #[serde(default)]
    pub gitee_snippet: GiteeSnippetSyncSettings,
    #[serde(default)]
    pub google_drive: OAuthDriveSyncSettings,
    #[serde(default)]
    pub onedrive: OAuthDriveSyncSettings,
    #[serde(default)]
    pub aliyun_drive: AliyunDriveSyncSettings,
    #[serde(default)]
    pub github_gist: GithubGistSyncSettings,
}

impl Default for CloudSyncSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: default_provider(),
            remote_root: default_remote_root(),
            device_name: default_device_name(),
            auto_check_on_startup: true,
            auto_push_on_change: true,
            auto_pull_remote_changes: true,
            sync_debounce_seconds: default_sync_debounce_seconds(),
            webdav: WebdavSyncSettings::default(),
            s3: S3SyncSettings::default(),
            gitee_snippet: GiteeSnippetSyncSettings::default(),
            google_drive: OAuthDriveSyncSettings::default(),
            onedrive: OAuthDriveSyncSettings::default(),
            aliyun_drive: AliyunDriveSyncSettings::default(),
            github_gist: GithubGistSyncSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudRemoteCheckDecision {
    UpToDate,
    LocalChanged,
    AutoPull,
    RemoteAvailable,
    Conflict,
}

pub fn decide_cloud_remote_check(
    state: &CloudSyncState,
    local_hash: &str,
    remote: &RemoteSyncPointer,
    allow_auto_pull: bool,
) -> CloudRemoteCheckDecision {
    if remote.payload_hash == local_hash {
        return CloudRemoteCheckDecision::UpToDate;
    }

    let local_changed = state
        .last_synced_payload_hash
        .as_deref()
        .is_none_or(|hash| hash != local_hash);
    let remote_changed = state
        .last_applied_remote_revision
        .as_deref()
        .is_none_or(|revision| revision != remote.revision_id);

    match (remote_changed, local_changed, allow_auto_pull) {
        (true, true, _) => CloudRemoteCheckDecision::Conflict,
        (true, false, true) => CloudRemoteCheckDecision::AutoPull,
        (true, false, false) => CloudRemoteCheckDecision::RemoteAvailable,
        (false, true, _) => CloudRemoteCheckDecision::LocalChanged,
        (false, false, _) => CloudRemoteCheckDecision::UpToDate,
    }
}

pub fn mask_cloud_sync_settings(mut settings: CloudSyncSettings) -> CloudSyncSettings {
    settings.webdav.password = mask_secret(settings.webdav.password);
    settings.s3.access_key_id = mask_secret(settings.s3.access_key_id);
    settings.s3.secret_access_key = mask_secret(settings.s3.secret_access_key);
    settings.s3.session_token = mask_secret(settings.s3.session_token);
    settings.gitee_snippet.access_token = mask_secret(settings.gitee_snippet.access_token);
    mask_oauth_drive_settings(&mut settings.google_drive);
    mask_oauth_drive_settings(&mut settings.onedrive);
    mask_aliyun_drive_settings(&mut settings.aliyun_drive);
    settings.github_gist.access_token = mask_secret(settings.github_gist.access_token);
    settings
}

pub fn merge_masked_cloud_sync_settings(
    current: &CloudSyncSettings,
    mut next: CloudSyncSettings,
) -> CloudSyncSettings {
    next.webdav.password = merge_secret(
        current.webdav.password.as_ref(),
        next.webdav.password.as_ref(),
    );
    next.s3.access_key_id = merge_secret(
        current.s3.access_key_id.as_ref(),
        next.s3.access_key_id.as_ref(),
    );
    next.s3.secret_access_key = merge_secret(
        current.s3.secret_access_key.as_ref(),
        next.s3.secret_access_key.as_ref(),
    );
    next.s3.session_token = merge_secret(
        current.s3.session_token.as_ref(),
        next.s3.session_token.as_ref(),
    );
    next.gitee_snippet.access_token = merge_secret(
        current.gitee_snippet.access_token.as_ref(),
        next.gitee_snippet.access_token.as_ref(),
    );
    merge_oauth_drive_settings(&current.google_drive, &mut next.google_drive);
    merge_oauth_drive_settings(&current.onedrive, &mut next.onedrive);
    merge_aliyun_drive_settings(&current.aliyun_drive, &mut next.aliyun_drive);
    next.github_gist.access_token = merge_secret(
        current.github_gist.access_token.as_ref(),
        next.github_gist.access_token.as_ref(),
    );
    next
}

#[derive(Debug, Clone)]
pub struct LocalCloudSyncOptions {
    pub config_dir: PathBuf,
    pub portable_key_path: Option<PathBuf>,
    pub remote_dir: PathBuf,
    pub remote_root: String,
    pub device_id: String,
    pub app_version: String,
    pub master_password: String,
    pub enabled: bool,
}

fn default_provider() -> String {
    "webdav".to_string()
}

fn default_remote_root() -> String {
    "nyaterm".to_string()
}

fn default_gitee_api_endpoint() -> String {
    "https://gitee.com/api/v5".to_string()
}

fn default_aliyun_drive_type() -> String {
    "resource".to_string()
}

fn default_device_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "This Device".to_string())
}

fn default_sync_debounce_seconds() -> u64 {
    15
}

fn default_true() -> bool {
    true
}

fn mask_oauth_drive_settings(settings: &mut OAuthDriveSyncSettings) {
    settings.access_token = mask_secret(settings.access_token.take());
    settings.refresh_token = mask_secret(settings.refresh_token.take());
    settings.client_secret = mask_secret(settings.client_secret.take());
}

fn mask_aliyun_drive_settings(settings: &mut AliyunDriveSyncSettings) {
    settings.access_token = mask_secret(settings.access_token.take());
    settings.refresh_token = mask_secret(settings.refresh_token.take());
    settings.client_secret = mask_secret(settings.client_secret.take());
}

fn merge_oauth_drive_settings(current: &OAuthDriveSyncSettings, next: &mut OAuthDriveSyncSettings) {
    next.access_token = merge_secret(current.access_token.as_ref(), next.access_token.as_ref());
    next.refresh_token = merge_secret(current.refresh_token.as_ref(), next.refresh_token.as_ref());
    next.client_secret = merge_secret(current.client_secret.as_ref(), next.client_secret.as_ref());
}

fn merge_aliyun_drive_settings(
    current: &AliyunDriveSyncSettings,
    next: &mut AliyunDriveSyncSettings,
) {
    next.access_token = merge_secret(current.access_token.as_ref(), next.access_token.as_ref());
    next.refresh_token = merge_secret(current.refresh_token.as_ref(), next.refresh_token.as_ref());
    next.client_secret = merge_secret(current.client_secret.as_ref(), next.client_secret.as_ref());
}

fn mask_secret(value: Option<String>) -> Option<String> {
    value.and_then(|secret| {
        if secret.is_empty() {
            None
        } else {
            Some(MASKED_SECRET_VALUE.to_string())
        }
    })
}

fn merge_secret(current: Option<&String>, incoming: Option<&String>) -> Option<String> {
    match incoming.map(String::as_str) {
        Some(MASKED_SECRET_VALUE) | None => current.cloned(),
        Some("") => None,
        Some(value) => Some(value.to_string()),
    }
}

#[derive(Debug, Clone)]
pub struct CloudSyncResult {
    pub state: CloudSyncState,
    pub status: CloudSyncStatus,
    pub pointer: Option<RemoteSyncPointer>,
    pub backup: Option<CloudSyncBackupInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloudSyncBackupInfo {
    pub database_path: PathBuf,
    pub safety_backup_path: Option<PathBuf>,
}

pub trait CloudSyncRemote {
    fn provider(&self) -> &'static str;
    fn create_dir(&self, path: &str) -> Result<(), CloudSyncError>;
    fn read_if_exists(&self, path: &str) -> Result<Option<Vec<u8>>, CloudSyncError>;
    fn write(&self, path: &str, bytes: &[u8]) -> Result<(), CloudSyncError>;
    fn delete(&self, path: &str) -> Result<(), CloudSyncError>;
    fn list_files(&self, path: &str) -> Result<Vec<String>, CloudSyncError>;
}

pub trait CloudLocalStore: Send + Sync {
    fn encode_sync_snapshot(
        &self,
        snapshot: &RawPortableSnapshot,
        master_password: &str,
    ) -> Result<Vec<u8>, CloudSyncError>;

    fn decode_sync_snapshot(
        &self,
        bytes: &[u8],
        master_password: &str,
    ) -> Result<RawPortableSnapshot, CloudSyncError>;

    fn encode_sync_pointer(&self, pointer: &RemoteSyncPointer) -> Result<Vec<u8>, CloudSyncError>;

    fn decode_sync_pointer(&self, bytes: &[u8]) -> Result<RemoteSyncPointer, CloudSyncError>;

    fn build_sync_snapshot(
        &self,
        options: &LocalCloudSyncOptions,
    ) -> Result<RawPortableSnapshot, CloudSyncError>;

    fn apply_sync_snapshot(
        &self,
        options: &LocalCloudSyncOptions,
        snapshot: &RawPortableSnapshot,
    ) -> Result<CloudSyncBackupInfo, CloudSyncError>;

    fn persist_cloud_sync_state(&self, state: &CloudSyncState) -> Result<(), CloudSyncError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum S3HttpMethod {
    Get,
    Head,
    Put,
    Delete,
}

impl S3HttpMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct S3SignedRequest {
    pub method: S3HttpMethod,
    pub url: String,
    pub headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct S3ObjectTarget {
    url: String,
    host: String,
    canonical_uri: String,
}

pub fn s3_payload_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub fn build_s3_signed_request(
    settings: &S3SyncSettings,
    method: S3HttpMethod,
    path: &str,
    payload_sha256: &str,
    timestamp: SystemTime,
) -> Result<S3SignedRequest, CloudSyncError> {
    build_s3_signed_request_with_query(
        settings,
        method,
        path,
        &BTreeMap::new(),
        payload_sha256,
        timestamp,
    )
}

pub fn build_s3_signed_request_with_query(
    settings: &S3SyncSettings,
    method: S3HttpMethod,
    path: &str,
    query: &BTreeMap<String, String>,
    payload_sha256: &str,
    timestamp: SystemTime,
) -> Result<S3SignedRequest, CloudSyncError> {
    let access_key_id = required_secret(
        settings.access_key_id.as_deref(),
        "S3 access key ID is required",
    )?;
    let secret_access_key = required_secret(
        settings.secret_access_key.as_deref(),
        "S3 secret access key is required",
    )?;
    let region = s3_region(settings);
    let (short_date, amz_date) = s3_timestamp(timestamp)?;
    let target = s3_object_target(settings, path)?;
    let canonical_query = query
        .iter()
        .map(|(key, value)| {
            format!(
                "{}={}",
                s3_percent_encode_segment(key),
                s3_percent_encode_segment(value)
            )
        })
        .collect::<Vec<_>>()
        .join("&");

    let mut headers = BTreeMap::from([
        ("host".to_string(), target.host.clone()),
        (
            "x-amz-content-sha256".to_string(),
            payload_sha256.to_string(),
        ),
        ("x-amz-date".to_string(), amz_date.clone()),
    ]);
    if let Some(session_token) = settings
        .session_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        headers.insert(
            "x-amz-security-token".to_string(),
            session_token.to_string(),
        );
    }

    let canonical_headers = headers
        .iter()
        .map(|(name, value)| format!("{name}:{}\n", normalize_s3_header_value(value)))
        .collect::<String>();
    let signed_headers = headers.keys().cloned().collect::<Vec<_>>().join(";");
    let canonical_request = format!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        method.as_str(),
        target.canonical_uri,
        canonical_query,
        canonical_headers,
        signed_headers,
        payload_sha256
    );
    let credential_scope = format!("{short_date}/{region}/s3/aws4_request");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{}",
        s3_payload_sha256(canonical_request.as_bytes())
    );
    let signing_key = s3_signing_key(&secret_access_key, &short_date, &region)?;
    let signature = hex::encode(s3_hmac(&signing_key, &string_to_sign)?);
    headers.insert(
        "authorization".to_string(),
        format!(
            "AWS4-HMAC-SHA256 Credential={access_key_id}/{credential_scope}, SignedHeaders={signed_headers}, Signature={signature}"
        ),
    );

    Ok(S3SignedRequest {
        method,
        url: if canonical_query.is_empty() {
            target.url
        } else {
            format!("{}?{canonical_query}", target.url)
        },
        headers,
    })
}

fn s3_object_target(
    settings: &S3SyncSettings,
    path: &str,
) -> Result<S3ObjectTarget, CloudSyncError> {
    let bucket = settings.bucket.trim();
    if bucket.is_empty() {
        return Err(CloudSyncError::Remote("S3 bucket is required".to_string()));
    }
    if bucket.contains('/') {
        return Err(CloudSyncError::Remote(
            "S3 bucket must not contain path separators".to_string(),
        ));
    }

    let endpoint = s3_endpoint(settings);
    let endpoint = split_s3_endpoint(&endpoint)?;
    let object_key = remote_path(&settings.root, path);
    let object_path = s3_encode_path(&object_key);

    if settings.virtual_host_style {
        let host = format!("{bucket}.{}", endpoint.host);
        let canonical_uri = join_s3_paths(&endpoint.base_path, &object_path, false);
        return Ok(S3ObjectTarget {
            url: format!("{}://{}{}", endpoint.scheme, host, canonical_uri),
            host,
            canonical_uri,
        });
    }

    let canonical_uri = join_s3_paths(
        &endpoint.base_path,
        &format!("{}/{}", s3_encode_path(bucket), object_path),
        false,
    );
    Ok(S3ObjectTarget {
        url: format!("{}://{}{}", endpoint.scheme, endpoint.host, canonical_uri),
        host: endpoint.host,
        canonical_uri,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct S3EndpointParts {
    scheme: String,
    host: String,
    base_path: String,
}

fn s3_endpoint(settings: &S3SyncSettings) -> String {
    let endpoint = settings.endpoint.trim().trim_end_matches('/');
    if !endpoint.is_empty() {
        endpoint.to_string()
    } else {
        format!("https://s3.{}.amazonaws.com", s3_region(settings))
    }
}

fn split_s3_endpoint(endpoint: &str) -> Result<S3EndpointParts, CloudSyncError> {
    let Some((scheme, rest)) = endpoint.split_once("://") else {
        return Err(CloudSyncError::Remote(
            "S3 endpoint must include http:// or https://".to_string(),
        ));
    };
    let scheme = scheme.trim().to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(CloudSyncError::Remote(format!(
            "S3 endpoint scheme '{scheme}' is not supported"
        )));
    }
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host = host.trim().to_ascii_lowercase();
    if host.is_empty() {
        return Err(CloudSyncError::Remote(
            "S3 endpoint host is required".to_string(),
        ));
    }
    Ok(S3EndpointParts {
        scheme,
        host,
        base_path: s3_encode_path(path),
    })
}

fn s3_region(settings: &S3SyncSettings) -> String {
    if settings.region.trim().is_empty() {
        "us-east-1".to_string()
    } else {
        settings.region.trim().to_string()
    }
}

fn s3_timestamp(timestamp: SystemTime) -> Result<(String, String), CloudSyncError> {
    let timestamp: OffsetDateTime = timestamp.into();
    let short_date = timestamp
        .format(format_description!("[year][month][day]"))
        .map_err(|error| CloudSyncError::Remote(format!("failed to format S3 date: {error}")))?;
    let amz_date = timestamp
        .format(format_description!(
            "[year][month][day]T[hour][minute][second]Z"
        ))
        .map_err(|error| {
            CloudSyncError::Remote(format!("failed to format S3 x-amz-date: {error}"))
        })?;
    Ok((short_date, amz_date))
}

fn s3_signing_key(secret: &str, short_date: &str, region: &str) -> Result<Vec<u8>, CloudSyncError> {
    let date_key = s3_hmac(format!("AWS4{secret}").as_bytes(), short_date)?;
    let region_key = s3_hmac(&date_key, region)?;
    let service_key = s3_hmac(&region_key, "s3")?;
    s3_hmac(&service_key, "aws4_request")
}

fn s3_hmac(key: &[u8], value: &str) -> Result<Vec<u8>, CloudSyncError> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|error| CloudSyncError::Remote(format!("failed to create S3 HMAC: {error}")))?;
    mac.update(value.as_bytes());
    Ok(mac.finalize().into_bytes().to_vec())
}

fn normalize_s3_header_value(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn join_s3_paths(left: &str, right: &str, trailing_slash: bool) -> String {
    let left = left.trim_matches('/');
    let right = right.trim_matches('/');
    let mut path = match (left.is_empty(), right.is_empty()) {
        (true, true) => "/".to_string(),
        (true, false) => format!("/{right}"),
        (false, true) => format!("/{left}"),
        (false, false) => format!("/{left}/{right}"),
    };
    if trailing_slash && !path.ends_with('/') {
        path.push('/');
    }
    path
}

fn s3_encode_path(path: &str) -> String {
    path.trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(s3_percent_encode_segment)
        .collect::<Vec<_>>()
        .join("/")
}

fn s3_percent_encode_segment(segment: &str) -> String {
    let mut encoded = String::new();
    for byte in segment.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

pub const SNIPPET_REMOTE_FILE_PREFIX: &str = "nyaterm-";
pub const SNIPPET_REMOTE_FILE_SUFFIX: &str = ".blob";

pub trait SnippetBlobBackend {
    fn fetch_blob(&self, filename: &str) -> Result<Option<String>, CloudSyncError>;
    fn patch_blobs(&self, files: BTreeMap<String, Option<String>>) -> Result<(), CloudSyncError>;
    fn list_blob_names(&self) -> Result<Vec<String>, CloudSyncError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnippetHttpMethod {
    Get,
    Patch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnippetHttpRequest {
    pub method: SnippetHttpMethod,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub query: BTreeMap<String, String>,
    pub json_body: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnippetHttpResponse {
    pub status: u16,
    pub body: String,
}

pub trait SnippetHttpClient {
    fn send(&self, request: SnippetHttpRequest) -> Result<SnippetHttpResponse, CloudSyncError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SnippetHttpDocument {
    #[serde(default)]
    pub files: BTreeMap<String, SnippetHttpFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SnippetHttpFile {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub raw_url: Option<String>,
    #[serde(default)]
    pub truncated: bool,
}

pub struct GiteeSnippetHttpBackend<C> {
    client: C,
    api_endpoint: String,
    gist_id: String,
    access_token: String,
}

impl<C> GiteeSnippetHttpBackend<C> {
    pub fn new(settings: &GiteeSnippetSyncSettings, client: C) -> Result<Self, CloudSyncError> {
        let api_endpoint = settings
            .api_endpoint
            .trim()
            .trim_end_matches('/')
            .to_string();
        let gist_id = settings.gist_id.trim().to_string();
        let access_token = required_secret(
            settings.access_token.as_deref(),
            "Gitee snippet access token is required",
        )?;
        if api_endpoint.is_empty() {
            return Err(CloudSyncError::Remote(
                "Gitee API endpoint is required".to_string(),
            ));
        }
        if gist_id.is_empty() {
            return Err(CloudSyncError::Remote(
                "Gitee snippet ID is required".to_string(),
            ));
        }
        Ok(Self {
            client,
            api_endpoint,
            gist_id,
            access_token,
        })
    }
}

impl<C> SnippetBlobBackend for GiteeSnippetHttpBackend<C>
where
    C: SnippetHttpClient,
{
    fn fetch_blob(&self, filename: &str) -> Result<Option<String>, CloudSyncError> {
        if let Ok(content) = self.fetch_raw_filename(filename) {
            return Ok(Some(content));
        }

        let document = self.fetch_document()?;
        let Some(file) = document.files.get(filename) else {
            return Ok(None);
        };
        if let Some(content) = non_empty_optional(&file.content) {
            return Ok(Some(content.to_string()));
        }
        self.fetch_raw_file(filename, file).map(Some)
    }

    fn patch_blobs(&self, files: BTreeMap<String, Option<String>>) -> Result<(), CloudSyncError> {
        let response = self.client.send(SnippetHttpRequest {
            method: SnippetHttpMethod::Patch,
            url: join_url(&self.api_endpoint, &format!("gists/{}", self.gist_id)),
            headers: BTreeMap::new(),
            query: BTreeMap::new(),
            json_body: Some(gitee_snippet_patch_body(&self.access_token, files)),
        })?;
        ensure_snippet_http_success("Gitee snippet", response.status, &response.body)
    }

    fn list_blob_names(&self) -> Result<Vec<String>, CloudSyncError> {
        Ok(self.fetch_document()?.files.into_keys().collect())
    }
}

impl<C> GiteeSnippetHttpBackend<C>
where
    C: SnippetHttpClient,
{
    fn fetch_document(&self) -> Result<SnippetHttpDocument, CloudSyncError> {
        let response = self.client.send(
            self.gitee_get_request(format!("{}/gists/{}", self.api_endpoint, self.gist_id)),
        )?;
        let body = ensure_snippet_http_text("Gitee snippet", response)?;
        serde_json::from_str(&body).map_err(CloudSyncError::Json)
    }

    fn fetch_raw_filename(&self, filename: &str) -> Result<String, CloudSyncError> {
        let response = self.client.send(self.gitee_get_request(format!(
            "{}/gists/{}/raw/{}",
            self.api_endpoint, self.gist_id, filename
        )))?;
        ensure_snippet_http_text("Gitee snippet", response)
    }

    fn fetch_raw_file(
        &self,
        filename: &str,
        file: &SnippetHttpFile,
    ) -> Result<String, CloudSyncError> {
        let url = non_empty_optional(&file.raw_url)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| {
                format!(
                    "{}/gists/{}/raw/{}",
                    self.api_endpoint, self.gist_id, filename
                )
            });
        let response = self.client.send(self.gitee_get_request(url))?;
        ensure_snippet_http_text("Gitee snippet", response)
    }

    fn gitee_get_request(&self, url: String) -> SnippetHttpRequest {
        let mut query = BTreeMap::new();
        query.insert("access_token".to_string(), self.access_token.clone());
        SnippetHttpRequest {
            method: SnippetHttpMethod::Get,
            url,
            headers: BTreeMap::new(),
            query,
            json_body: None,
        }
    }
}

pub struct GithubGistHttpBackend<C> {
    client: C,
    gist_id: String,
    access_token: String,
}

impl<C> GithubGistHttpBackend<C> {
    pub fn new(settings: &GithubGistSyncSettings, client: C) -> Result<Self, CloudSyncError> {
        let gist_id = settings.gist_id.trim().to_string();
        let access_token = required_secret(
            settings.access_token.as_deref(),
            "GitHub Gist access token is required",
        )?;
        if gist_id.is_empty() {
            return Err(CloudSyncError::Remote(
                "GitHub Gist ID is required".to_string(),
            ));
        }
        Ok(Self {
            client,
            gist_id,
            access_token,
        })
    }
}

impl<C> SnippetBlobBackend for GithubGistHttpBackend<C>
where
    C: SnippetHttpClient,
{
    fn fetch_blob(&self, filename: &str) -> Result<Option<String>, CloudSyncError> {
        let document = self.fetch_document()?;
        let Some(file) = document.files.get(filename) else {
            return Ok(None);
        };
        if !file.truncated
            && let Some(content) = non_empty_optional(&file.content)
        {
            return Ok(Some(content.to_string()));
        }
        self.fetch_raw_file(file).map(Some)
    }

    fn patch_blobs(&self, files: BTreeMap<String, Option<String>>) -> Result<(), CloudSyncError> {
        let request = self.github_patch_request(github_gist_patch_body(files));
        let response = self.client.send(request.clone())?;
        if github_gist_update_conflict_is_retryable(response.status, &response.body) {
            let retry = self.client.send(request)?;
            return ensure_snippet_http_success("GitHub Gist", retry.status, &retry.body);
        }
        ensure_snippet_http_success("GitHub Gist", response.status, &response.body)
    }

    fn list_blob_names(&self) -> Result<Vec<String>, CloudSyncError> {
        Ok(self.fetch_document()?.files.into_keys().collect())
    }
}

impl<C> GithubGistHttpBackend<C>
where
    C: SnippetHttpClient,
{
    fn fetch_document(&self) -> Result<SnippetHttpDocument, CloudSyncError> {
        let response = self.client.send(
            self.github_get_request(format!("https://api.github.com/gists/{}", self.gist_id)),
        )?;
        let body = ensure_snippet_http_text("GitHub Gist", response)?;
        serde_json::from_str(&body).map_err(CloudSyncError::Json)
    }

    fn fetch_raw_file(&self, file: &SnippetHttpFile) -> Result<String, CloudSyncError> {
        let raw_url = non_empty_optional(&file.raw_url).ok_or_else(|| {
            CloudSyncError::Remote("GitHub Gist file raw URL is missing".to_string())
        })?;
        let response = self
            .client
            .send(self.github_get_request(raw_url.to_string()))?;
        ensure_snippet_http_text("GitHub Gist", response)
    }

    fn github_get_request(&self, url: String) -> SnippetHttpRequest {
        SnippetHttpRequest {
            method: SnippetHttpMethod::Get,
            url,
            headers: self.github_headers(),
            query: BTreeMap::new(),
            json_body: None,
        }
    }

    fn github_patch_request(&self, body: serde_json::Value) -> SnippetHttpRequest {
        SnippetHttpRequest {
            method: SnippetHttpMethod::Patch,
            url: format!("https://api.github.com/gists/{}", self.gist_id),
            headers: self.github_headers(),
            query: BTreeMap::new(),
            json_body: Some(body),
        }
    }

    fn github_headers(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            (
                "Authorization".to_string(),
                format!("Bearer {}", self.access_token),
            ),
            (
                "Accept".to_string(),
                "application/vnd.github+json".to_string(),
            ),
            ("X-GitHub-Api-Version".to_string(), "2022-11-28".to_string()),
            ("User-Agent".to_string(), "NyaTerm".to_string()),
        ])
    }
}

pub fn gitee_snippet_patch_body(
    access_token: &str,
    files: BTreeMap<String, Option<String>>,
) -> serde_json::Value {
    serde_json::json!({
        "access_token": access_token,
        "files": snippet_patch_file_values(files),
    })
}

pub fn github_gist_patch_body(files: BTreeMap<String, Option<String>>) -> serde_json::Value {
    serde_json::json!({
        "files": snippet_patch_file_values(files),
    })
}

pub fn github_gist_update_conflict_is_retryable(status: u16, body: &str) -> bool {
    status == 409 && body.contains("Gist cannot be updated")
}

fn snippet_patch_file_values(
    files: BTreeMap<String, Option<String>>,
) -> serde_json::Map<String, serde_json::Value> {
    files
        .into_iter()
        .map(|(filename, content)| {
            let value = content
                .map(|content| serde_json::json!({ "content": content }))
                .unwrap_or(serde_json::Value::Null);
            (filename, value)
        })
        .collect()
}

fn ensure_snippet_http_text(
    provider: &str,
    response: SnippetHttpResponse,
) -> Result<String, CloudSyncError> {
    ensure_snippet_http_success(provider, response.status, &response.body)?;
    Ok(response.body)
}

fn ensure_snippet_http_success(
    provider: &str,
    status: u16,
    body: &str,
) -> Result<(), CloudSyncError> {
    if (200..300).contains(&status) {
        return Ok(());
    }
    Err(CloudSyncError::Remote(format!(
        "{provider} request failed ({status}): {}",
        body.trim()
    )))
}

fn required_secret(value: Option<&str>, message: &str) -> Result<String, CloudSyncError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| CloudSyncError::Remote(message.to_string()))
}

fn non_empty_optional(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|value| !value.trim().is_empty())
}

fn join_url(base: &str, child: &str) -> String {
    format!(
        "{}/{}",
        base.trim_end_matches('/'),
        child.trim_start_matches('/')
    )
}

pub struct SnippetRemote<B> {
    provider: &'static str,
    backend: B,
}

impl<B> SnippetRemote<B> {
    pub fn new(provider: &'static str, backend: B) -> Self {
        Self { provider, backend }
    }
}

impl<B> CloudSyncRemote for SnippetRemote<B>
where
    B: SnippetBlobBackend,
{
    fn provider(&self) -> &'static str {
        self.provider
    }

    fn create_dir(&self, _path: &str) -> Result<(), CloudSyncError> {
        Ok(())
    }

    fn read_if_exists(&self, path: &str) -> Result<Option<Vec<u8>>, CloudSyncError> {
        let filename = snippet_remote_filename(path);
        self.backend
            .fetch_blob(&filename)?
            .map(|content| decode_snippet_blob(&content))
            .transpose()
    }

    fn write(&self, path: &str, bytes: &[u8]) -> Result<(), CloudSyncError> {
        let mut files = std::collections::BTreeMap::new();
        files.insert(
            snippet_remote_filename(path),
            Some(encode_snippet_blob(bytes)),
        );
        self.backend.patch_blobs(files)
    }

    fn delete(&self, path: &str) -> Result<(), CloudSyncError> {
        let mut files = BTreeMap::new();
        files.insert(snippet_remote_filename(path), None);
        self.backend.patch_blobs(files)
    }

    fn list_files(&self, path: &str) -> Result<Vec<String>, CloudSyncError> {
        let prefix = path.trim_start_matches('/');
        Ok(self
            .backend
            .list_blob_names()?
            .into_iter()
            .filter_map(|filename| snippet_remote_path(&filename))
            .filter(|remote_path| remote_path.starts_with(prefix))
            .collect())
    }
}

pub fn snippet_remote_filename(path: &str) -> String {
    format!(
        "{SNIPPET_REMOTE_FILE_PREFIX}{}{SNIPPET_REMOTE_FILE_SUFFIX}",
        URL_SAFE_NO_PAD.encode(path.as_bytes())
    )
}

pub fn snippet_remote_path(filename: &str) -> Option<String> {
    let encoded = filename
        .strip_prefix(SNIPPET_REMOTE_FILE_PREFIX)?
        .strip_suffix(SNIPPET_REMOTE_FILE_SUFFIX)?;
    let bytes = URL_SAFE_NO_PAD.decode(encoded).ok()?;
    String::from_utf8(bytes).ok()
}

pub fn encode_snippet_blob(bytes: &[u8]) -> String {
    BASE64_STANDARD.encode(bytes)
}

pub fn decode_snippet_blob(content: &str) -> Result<Vec<u8>, CloudSyncError> {
    Ok(BASE64_STANDARD.decode(content.trim())?)
}

#[derive(Debug, Clone)]
pub struct LocalDirectoryRemote {
    root_dir: PathBuf,
}

impl LocalDirectoryRemote {
    pub fn new(root_dir: impl Into<PathBuf>) -> Self {
        Self {
            root_dir: root_dir.into(),
        }
    }

    fn path_for(&self, path: &str) -> Result<PathBuf, CloudSyncError> {
        let mut local = self.root_dir.clone();
        for component in Path::new(path).components() {
            match component {
                Component::Normal(part) => local.push(part),
                Component::CurDir => {}
                _ => {
                    return Err(CloudSyncError::InvalidRemotePath {
                        path: path.to_string(),
                    });
                }
            }
        }
        Ok(local)
    }
}

impl CloudSyncRemote for LocalDirectoryRemote {
    fn provider(&self) -> &'static str {
        "local_directory"
    }

    fn create_dir(&self, path: &str) -> Result<(), CloudSyncError> {
        let path = self.path_for(path)?;
        std::fs::create_dir_all(&path).map_err(|source| CloudSyncError::CreateDir {
            path: path.clone(),
            source,
        })
    }

    fn read_if_exists(&self, path: &str) -> Result<Option<Vec<u8>>, CloudSyncError> {
        let path = self.path_for(path)?;
        if !path.exists() {
            return Ok(None);
        }
        std::fs::read(&path)
            .map(Some)
            .map_err(|source| CloudSyncError::ReadFile { path, source })
    }

    fn write(&self, path: &str, bytes: &[u8]) -> Result<(), CloudSyncError> {
        let path = self.path_for(path)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| CloudSyncError::CreateDir {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        std::fs::write(&path, bytes).map_err(|source| CloudSyncError::WriteFile { path, source })
    }

    fn delete(&self, path: &str) -> Result<(), CloudSyncError> {
        let path = self.path_for(path)?;
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(CloudSyncError::WriteFile { path, source }),
        }
    }

    fn list_files(&self, path: &str) -> Result<Vec<String>, CloudSyncError> {
        let directory = self.path_for(path)?;
        if !directory.exists() {
            return Ok(Vec::new());
        }
        let prefix = path.trim().trim_matches('/');
        std::fs::read_dir(&directory)
            .map_err(|source| CloudSyncError::ReadFile {
                path: directory.clone(),
                source,
            })?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                entry
                    .file_type()
                    .ok()
                    .filter(|kind| kind.is_file())
                    .map(|_| entry.file_name().to_string_lossy().into_owned())
            })
            .map(|name| Ok(remote_path(prefix, &name)))
            .collect()
    }
}

pub fn remote_path(base_root: &str, child: &str) -> String {
    let root = base_root.trim().trim_matches('/');
    let child = child.trim().trim_start_matches('/');
    if root.is_empty() {
        child.to_string()
    } else if child.is_empty() {
        root.to_string()
    } else {
        format!("{root}/{child}")
    }
}

pub fn drive_remote_segments(base_root: &str, child: &str) -> Vec<String> {
    remote_path(base_root, child)
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub fn google_drive_query_literal(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('\'', "\\'");
    format!("'{escaped}'")
}

pub fn legacy_sync_snapshot_file(revision: &str) -> String {
    format!("{SYNC_SNAPSHOTS_DIR}{revision}.redb.enc")
}

pub fn append_cloud_sync_history(
    log_dir: impl AsRef<Path>,
    entry: &CloudSyncHistoryEntry,
) -> Result<(), CloudSyncError> {
    let log_dir = log_dir.as_ref();
    std::fs::create_dir_all(log_dir).map_err(|source| CloudSyncError::CreateDir {
        path: log_dir.to_path_buf(),
        source,
    })?;
    let path = log_dir.join(cloud_sync_history_log_file_name());
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|source| CloudSyncError::WriteFile {
            path: path.clone(),
            source,
        })?;
    let line = serde_json::to_string(&cloud_sync_history_log_value(entry))?;
    writeln!(file, "{line}").map_err(|source| CloudSyncError::WriteFile { path, source })
}

pub fn read_cloud_sync_history(
    log_dir: impl AsRef<Path>,
    retention_days: u32,
    limit: usize,
) -> Result<Vec<CloudSyncHistoryEntry>, CloudSyncError> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let log_dir = log_dir.as_ref();
    let mut entries = Vec::new();
    for path in collect_cloud_sync_history_log_files(log_dir, retention_days)? {
        let content = match std::fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => continue,
        };
        for line in content.lines().rev() {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            if let Some(entry) = parse_cloud_sync_history_entry(&value) {
                entries.push(entry);
                if entries.len() >= limit {
                    entries.sort_by_key(|entry| std::cmp::Reverse(entry.timestamp_ms));
                    return Ok(entries);
                }
            }
        }
    }
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.timestamp_ms));
    Ok(entries)
}

pub fn push_local_snapshot(
    local_store: &dyn CloudLocalStore,
    options: &LocalCloudSyncOptions,
    state: &CloudSyncState,
    force: bool,
) -> Result<CloudSyncResult, CloudSyncError> {
    let remote = LocalDirectoryRemote::new(options.remote_dir.clone());
    push_snapshot_with_remote(local_store, options, &remote, state, force)
}

pub fn push_snapshot_with_remote(
    local_store: &dyn CloudLocalStore,
    options: &LocalCloudSyncOptions,
    remote: &dyn CloudSyncRemote,
    state: &CloudSyncState,
    force: bool,
) -> Result<CloudSyncResult, CloudSyncError> {
    ensure_enabled(options)?;
    ensure_remote_layout(remote, &options.remote_root)?;
    let mut next_state = normalized_state(state, &options.device_id);
    let mut snapshot = local_store.build_sync_snapshot(options)?;
    snapshot.recalculate_hash()?;
    let local_hash = snapshot.meta.payload_hash.clone();
    let latest = load_sync_pointer_from_remote(local_store, remote, &options.remote_root)?;

    if let Some(remote_pointer) = &latest
        && remote_pointer.payload_hash == local_hash
    {
        match protocol::resolve_remote_snapshot(local_store, remote, options, remote_pointer)? {
            protocol::RemoteSnapshotResolution::Current(_)
            | protocol::RemoteSnapshotResolution::LegacyMigrated(_) => {}
            protocol::RemoteSnapshotResolution::Inconsistent {
                pointer,
                recovery_candidate,
            } => {
                return Err(CloudSyncError::Conflict(Box::new(
                    remote_inconsistent_preview(
                        remote.provider(),
                        &local_hash,
                        &pointer,
                        &recovery_candidate,
                    ),
                )));
            }
        }
        next_state.last_synced_payload_hash = Some(local_hash);
        next_state.last_applied_remote_revision = Some(remote_pointer.revision_id.clone());
        next_state.last_checked_at_ms = Some(current_time_ms());
        let result = result(
            next_state,
            remote.provider(),
            "idle",
            "Cloud sync is already up to date",
            latest,
            None,
            None,
        );
        local_store.persist_cloud_sync_state(&result.state)?;
        return Ok(result);
    }

    let remote_changed = latest.as_ref().is_some_and(|remote| {
        next_state
            .last_applied_remote_revision
            .as_deref()
            .is_none_or(|revision| revision != remote.revision_id)
    });
    let local_changed = next_state
        .last_synced_payload_hash
        .as_deref()
        .is_none_or(|hash| hash != local_hash);

    if remote_changed && !force {
        let remote_pointer = latest.expect("remote changed requires remote pointer");
        if local_changed {
            let conflict =
                conflict_preview(options, remote.provider(), &local_hash, &remote_pointer);
            return Err(CloudSyncError::Conflict(Box::new(conflict)));
        }
        return Err(CloudSyncError::RemoteNewer);
    }

    protocol::upload_sync_snapshot(local_store, remote, options, &snapshot)?;
    let pointer = protocol::pointer_from_snapshot(&snapshot);
    protocol::read_snapshot_for_pointer(local_store, remote, options, &pointer)?;
    if !force {
        protocol::ensure_remote_head_unchanged(
            local_store,
            remote,
            &options.remote_root,
            latest.as_ref(),
        )?;
    }
    write_sync_pointer(local_store, remote, &options.remote_root, &pointer)?;
    let _ = protocol::write_current_sync_snapshot_compat(local_store, remote, options, &snapshot);
    next_state.last_synced_payload_hash = Some(pointer.payload_hash.clone());
    next_state.last_applied_remote_revision = Some(pointer.revision_id.clone());
    next_state.last_synced_at_ms = Some(current_time_ms());
    next_state.last_checked_at_ms = Some(current_time_ms());
    let result = result(
        next_state,
        remote.provider(),
        "idle",
        "Cloud sync snapshot uploaded",
        Some(pointer),
        None,
        None,
    );
    local_store.persist_cloud_sync_state(&result.state)?;
    Ok(result)
}

pub fn pull_local_snapshot(
    local_store: &dyn CloudLocalStore,
    options: &LocalCloudSyncOptions,
    state: &CloudSyncState,
    force: bool,
) -> Result<CloudSyncResult, CloudSyncError> {
    let remote = LocalDirectoryRemote::new(options.remote_dir.clone());
    pull_snapshot_with_remote(local_store, options, &remote, state, force)
}

pub fn pull_snapshot_with_remote(
    local_store: &dyn CloudLocalStore,
    options: &LocalCloudSyncOptions,
    remote: &dyn CloudSyncRemote,
    state: &CloudSyncState,
    force: bool,
) -> Result<CloudSyncResult, CloudSyncError> {
    ensure_enabled(options)?;
    ensure_remote_layout(remote, &options.remote_root)?;
    let latest = load_sync_pointer_from_remote(local_store, remote, &options.remote_root)?
        .ok_or(CloudSyncError::NoRemoteSnapshot)?;
    let mut next_state = normalized_state(state, &options.device_id);
    let mut local_snapshot = local_store.build_sync_snapshot(options)?;
    local_snapshot.recalculate_hash()?;
    let remote_snapshot =
        match protocol::resolve_remote_snapshot(local_store, remote, options, &latest)? {
            protocol::RemoteSnapshotResolution::Current(snapshot)
            | protocol::RemoteSnapshotResolution::LegacyMigrated(snapshot) => snapshot,
            protocol::RemoteSnapshotResolution::Inconsistent {
                pointer,
                recovery_candidate,
            } => {
                return Err(CloudSyncError::Conflict(Box::new(
                    remote_inconsistent_preview(
                        remote.provider(),
                        &local_snapshot.meta.payload_hash,
                        &pointer,
                        &recovery_candidate,
                    ),
                )));
            }
        };

    if latest.payload_hash == local_snapshot.meta.payload_hash {
        next_state.last_synced_payload_hash = Some(latest.payload_hash.clone());
        next_state.last_applied_remote_revision = Some(latest.revision_id.clone());
        next_state.last_checked_at_ms = Some(current_time_ms());
        let result = result(
            next_state,
            remote.provider(),
            "idle",
            "Cloud sync is already up to date",
            Some(latest),
            None,
            None,
        );
        local_store.persist_cloud_sync_state(&result.state)?;
        return Ok(result);
    }

    let local_changed = next_state
        .last_synced_payload_hash
        .as_deref()
        .is_none_or(|hash| hash != local_snapshot.meta.payload_hash);
    let remote_changed = next_state
        .last_applied_remote_revision
        .as_deref()
        .is_none_or(|revision| revision != latest.revision_id);

    if remote_changed && local_changed && !force {
        let conflict = conflict_preview(
            options,
            remote.provider(),
            &local_snapshot.meta.payload_hash,
            &latest,
        );
        return Err(CloudSyncError::Conflict(Box::new(conflict)));
    }
    if !remote_changed && !force {
        return Err(CloudSyncError::NoNewerRemoteSnapshot);
    }

    let snapshot = remote_snapshot;
    let backup = local_store.apply_sync_snapshot(options, &snapshot)?;
    let _ = protocol::write_current_sync_snapshot_compat(local_store, remote, options, &snapshot);
    next_state.last_synced_payload_hash = Some(snapshot.meta.payload_hash.clone());
    next_state.last_applied_remote_revision = Some(snapshot.meta.revision_id.clone());
    next_state.last_synced_at_ms = Some(current_time_ms());
    next_state.last_checked_at_ms = Some(current_time_ms());
    let result = result(
        next_state,
        remote.provider(),
        "idle",
        "Cloud sync snapshot downloaded",
        Some(latest),
        None,
        Some(backup),
    );
    local_store.persist_cloud_sync_state(&result.state)?;
    Ok(result)
}

pub fn recover_local_current_snapshot(
    local_store: &dyn CloudLocalStore,
    options: &LocalCloudSyncOptions,
) -> Result<CloudSyncResult, CloudSyncError> {
    let remote = LocalDirectoryRemote::new(options.remote_dir.clone());
    recover_current_snapshot_with_remote(local_store, options, &remote)
}

pub fn recover_current_snapshot_with_remote(
    local_store: &dyn CloudLocalStore,
    options: &LocalCloudSyncOptions,
    remote: &dyn CloudSyncRemote,
) -> Result<CloudSyncResult, CloudSyncError> {
    ensure_enabled(options)?;
    ensure_remote_layout(remote, &options.remote_root)?;
    let snapshot = protocol::recover_current_remote_snapshot(local_store, remote, options)?;
    let backup = local_store.apply_sync_snapshot(options, &snapshot)?;
    let pointer = protocol::pointer_from_snapshot(&snapshot);
    let now = current_time_ms();
    let state = CloudSyncState {
        device_id: options.device_id.clone(),
        last_synced_payload_hash: Some(pointer.payload_hash.clone()),
        last_applied_remote_revision: Some(pointer.revision_id.clone()),
        last_checked_at_ms: Some(now),
        last_synced_at_ms: Some(now),
    };
    let result = result(
        state,
        remote.provider(),
        "idle",
        "Cloud sync metadata recovered",
        Some(pointer),
        None,
        Some(backup),
    );
    local_store.persist_cloud_sync_state(&result.state)?;
    Ok(result)
}

pub fn load_sync_pointer(
    local_store: &dyn CloudLocalStore,
    options: &LocalCloudSyncOptions,
) -> Result<Option<RemoteSyncPointer>, CloudSyncError> {
    let remote = LocalDirectoryRemote::new(options.remote_dir.clone());
    load_sync_pointer_from_remote(local_store, &remote, &options.remote_root)
}

pub fn load_sync_pointer_from_remote(
    local_store: &dyn CloudLocalStore,
    remote: &dyn CloudSyncRemote,
    remote_root: &str,
) -> Result<Option<RemoteSyncPointer>, CloudSyncError> {
    let path = remote_path(remote_root, SYNC_LATEST_FILE);
    let Some(bytes) = remote.read_if_exists(&path)? else {
        return Ok(None);
    };
    local_store.decode_sync_pointer(bytes.as_slice()).map(Some)
}

fn ensure_enabled(options: &LocalCloudSyncOptions) -> Result<(), CloudSyncError> {
    if options.enabled {
        Ok(())
    } else {
        Err(CloudSyncError::Disabled)
    }
}

fn ensure_remote_layout(
    remote: &dyn CloudSyncRemote,
    remote_root: &str,
) -> Result<(), CloudSyncError> {
    for child in ["sync", SYNC_SNAPSHOTS_DIR] {
        remote.create_dir(&remote_path(remote_root, child))?;
    }
    Ok(())
}

fn write_sync_pointer(
    local_store: &dyn CloudLocalStore,
    remote: &dyn CloudSyncRemote,
    remote_root: &str,
    pointer: &RemoteSyncPointer,
) -> Result<(), CloudSyncError> {
    let bytes = local_store.encode_sync_pointer(pointer)?;
    remote.write(&remote_path(remote_root, SYNC_LATEST_FILE), &bytes)
}

fn conflict_preview(
    options: &LocalCloudSyncOptions,
    provider: &str,
    local_hash: &str,
    remote: &RemoteSyncPointer,
) -> CloudConflictPreview {
    CloudConflictPreview {
        detected_at_ms: current_time_ms(),
        provider: provider.to_string(),
        kind: CloudConflictKind::ContentConflict,
        local_payload_hash: local_hash.to_string(),
        remote_payload_hash: remote.payload_hash.clone(),
        remote_revision: remote.revision_id.clone(),
        remote_created_at_ms: remote.created_at_ms,
        remote_device_id: remote.device_id.clone(),
        recovery_revision: None,
        recovery_payload_hash: None,
        recovery_created_at_ms: None,
        message: format!(
            "Both local and cloud state changed since last sync ({})",
            options.remote_dir.display()
        ),
    }
}

fn remote_inconsistent_preview(
    provider: &str,
    local_hash: &str,
    pointer: &RemoteSyncPointer,
    recovery_candidate: &RawPortableSnapshot,
) -> CloudConflictPreview {
    CloudConflictPreview {
        detected_at_ms: current_time_ms(),
        provider: provider.to_string(),
        kind: CloudConflictKind::RemoteInconsistent,
        local_payload_hash: local_hash.to_string(),
        remote_payload_hash: pointer.payload_hash.clone(),
        remote_revision: pointer.revision_id.clone(),
        remote_created_at_ms: pointer.created_at_ms,
        remote_device_id: pointer.device_id.clone(),
        recovery_revision: Some(recovery_candidate.meta.revision_id.clone()),
        recovery_payload_hash: Some(recovery_candidate.meta.payload_hash.clone()),
        recovery_created_at_ms: Some(recovery_candidate.meta.created_at_ms),
        message: "Remote cloud sync metadata is incomplete. The latest pointer references a missing snapshot, but current.redb.enc contains a recoverable snapshot."
            .to_string(),
    }
}

fn result(
    state: CloudSyncState,
    provider: &str,
    status_state: &str,
    message: &str,
    pointer: Option<RemoteSyncPointer>,
    conflict: Option<CloudConflictPreview>,
    backup: Option<CloudSyncBackupInfo>,
) -> CloudSyncResult {
    CloudSyncResult {
        status: CloudSyncStatus {
            enabled: true,
            provider: provider.to_string(),
            state: status_state.to_string(),
            message: message.to_string(),
            current_operation: None,
            last_checked_at_ms: state.last_checked_at_ms,
            last_synced_at_ms: state.last_synced_at_ms,
            conflict,
        },
        state,
        pointer,
        backup,
    }
}

fn normalized_state(state: &CloudSyncState, device_id: &str) -> CloudSyncState {
    let mut state = state.clone();
    if state.device_id.trim().is_empty() {
        state.device_id = device_id.to_string();
    }
    state
}

fn cloud_sync_history_log_file_name() -> String {
    format!(
        "{}-cloud-sync.{}",
        crate::diagnostics::LOG_FILE_PREFIX,
        crate::diagnostics::LOG_FILE_SUFFIX
    )
}

fn cloud_sync_history_log_value(entry: &CloudSyncHistoryEntry) -> serde_json::Value {
    serde_json::json!({
        "level": history_log_level(&entry.status),
        "domain": CLOUD_SYNC_HISTORY_DOMAIN,
        "event": CLOUD_SYNC_HISTORY_EVENT,
        "message": entry.message,
        "ids": {
            "history_id": entry.id,
        },
        "data": {
            "id": entry.id,
            "timestamp_ms": entry.timestamp_ms,
            "kind": entry.kind,
            "status": entry.status,
            "trigger": entry.trigger,
            "provider": entry.provider,
            "revision": entry.revision,
            "duration_ms": entry.duration_ms,
        },
        "error": null,
        "client_timestamp": null,
    })
}

fn history_log_level(status: &str) -> &'static str {
    match status {
        "failed" => "error",
        "conflict" => "warn",
        _ => "info",
    }
}

fn collect_cloud_sync_history_log_files(
    log_dir: &Path,
    retention_days: u32,
) -> Result<Vec<PathBuf>, CloudSyncError> {
    let min_modified = SystemTime::now()
        .checked_sub(Duration::from_secs(
            u64::from(retention_days.max(1)) * 24 * 60 * 60,
        ))
        .unwrap_or(SystemTime::UNIX_EPOCH);
    let entries = match std::fs::read_dir(log_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(CloudSyncError::ReadFile {
                path: log_dir.to_path_buf(),
                source,
            });
        }
    };
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| CloudSyncError::ReadFile {
            path: log_dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if !is_cloud_sync_history_log_file(&path) {
            continue;
        }
        let modified = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(SystemTime::UNIX_EPOCH);
        if modified >= min_modified {
            files.push((path, modified));
        }
    }
    files.sort_by(|(left_path, left_modified), (right_path, right_modified)| {
        right_modified
            .cmp(left_modified)
            .then_with(|| right_path.cmp(left_path))
    });
    Ok(files.into_iter().map(|(path, _)| path).collect())
}

fn is_cloud_sync_history_log_file(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| {
                value.starts_with(crate::diagnostics::LOG_FILE_PREFIX)
                    && value.ends_with(crate::diagnostics::LOG_FILE_SUFFIX)
            })
}

fn parse_cloud_sync_history_entry(value: &serde_json::Value) -> Option<CloudSyncHistoryEntry> {
    let root = value.as_object()?;
    if root.get("domain")?.as_str()? != CLOUD_SYNC_HISTORY_DOMAIN {
        return None;
    }
    if root.get("event")?.as_str()? != CLOUD_SYNC_HISTORY_EVENT {
        return None;
    }
    let data = root.get("data")?.as_object()?;
    Some(CloudSyncHistoryEntry {
        id: data.get("id")?.as_str()?.to_string(),
        timestamp_ms: data.get("timestamp_ms")?.as_u64()?,
        kind: data.get("kind")?.as_str()?.to_string(),
        status: data.get("status")?.as_str()?.to_string(),
        trigger: data.get("trigger")?.as_str()?.to_string(),
        provider: data
            .get("provider")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        revision: data
            .get("revision")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned),
        duration_ms: data.get("duration_ms").and_then(serde_json::Value::as_u64),
        message: root.get("message")?.as_str()?.to_string(),
    })
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn uuid_v4() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests;

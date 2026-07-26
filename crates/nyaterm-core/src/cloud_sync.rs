use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::{
    Engine,
    engine::general_purpose::{STANDARD as BASE64_STANDARD, URL_SAFE_NO_PAD},
};
use hmac::{Hmac, Mac, digest::KeyInit as HmacKeyInit};
use redb::{Database, ReadableDatabase, TableDefinition};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use time::{OffsetDateTime, macros::format_description};

use crate::{
    ConfigBackupInfo, ConnectionStore, PortableSnapshotError, PortableSnapshotKind,
    RawPortableSnapshot, StorageError, decode_encrypted_raw_portable_snapshot,
    encode_encrypted_raw_portable_snapshot,
};

type HmacSha256 = Hmac<Sha256>;

pub const SYNC_CURRENT_FILE: &str = "sync/current.redb.enc";
pub const SYNC_LATEST_FILE: &str = "sync/latest.redb";
pub const SYNC_SNAPSHOTS_DIR: &str = "sync/snapshots/";
pub const MASKED_SECRET_VALUE: &str = "__SET__";
pub const CLOUD_SYNC_HISTORY_DOMAIN: &str = "cloud_sync.history";
pub const CLOUD_SYNC_HISTORY_EVENT: &str = "entry";
pub const CLOUD_SYNC_HISTORY_LIMIT: usize = 100;

const REMOTE_SYNC_POINTER_TABLE: TableDefinition<&str, &str> = TableDefinition::new("sync_pointer");
const REMOTE_SYNC_POINTER_KEY: &str = "latest";

#[derive(Debug, Error)]
pub enum CloudSyncError {
    #[error("cloud sync is disabled")]
    Disabled,
    #[error("cloud sync conflict detected: {0}")]
    Conflict(String),
    #[error("remote snapshot is newer than local state; pull first")]
    RemoteNewer,
    #[error("no remote sync snapshot found")]
    NoRemoteSnapshot,
    #[error("no newer remote sync snapshot is available")]
    NoNewerRemoteSnapshot,
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
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("portable snapshot error: {0}")]
    PortableSnapshot(#[from] PortableSnapshotError),
    #[error("redb database error: {0}")]
    RedbDatabase(#[from] redb::DatabaseError),
    #[error("redb transaction error: {0}")]
    RedbTransaction(#[from] redb::TransactionError),
    #[error("redb table error: {0}")]
    RedbTable(#[from] redb::TableError),
    #[error("redb storage error: {0}")]
    RedbStorage(#[from] redb::StorageError),
    #[error("redb commit error: {0}")]
    RedbCommit(#[from] redb::CommitError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("base64 error: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteSyncPointer {
    pub revision_id: String,
    pub created_at_ms: u64,
    pub payload_hash: String,
    pub device_id: String,
    pub app_version: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CloudConflictPreview {
    pub detected_at_ms: u64,
    pub provider: String,
    pub local_payload_hash: String,
    pub remote_payload_hash: String,
    pub remote_revision: String,
    pub remote_created_at_ms: u64,
    pub remote_device_id: String,
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
    pub backup: Option<ConfigBackupInfo>,
}

pub trait CloudSyncRemote {
    fn provider(&self) -> &'static str;
    fn create_dir(&self, path: &str) -> Result<(), CloudSyncError>;
    fn read(&self, path: &str) -> Result<Vec<u8>, CloudSyncError>;
    fn read_if_exists(&self, path: &str) -> Result<Option<Vec<u8>>, CloudSyncError>;
    fn write(&self, path: &str, bytes: &[u8]) -> Result<(), CloudSyncError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum S3HttpMethod {
    Get,
    Head,
    Put,
}

impl S3HttpMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Head => "HEAD",
            Self::Put => "PUT",
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
        "{}\n{}\n\n{}\n{}\n{}",
        method.as_str(),
        target.canonical_uri,
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
        url: target.url,
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
    settings
        .region
        .trim()
        .is_empty()
        .then(|| "us-east-1".to_string())
        .unwrap_or_else(|| settings.region.trim().to_string())
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
        if !file.truncated {
            if let Some(content) = non_empty_optional(&file.content) {
                return Ok(Some(content.to_string()));
            }
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

    fn read(&self, path: &str) -> Result<Vec<u8>, CloudSyncError> {
        self.read_if_exists(path)?
            .ok_or_else(|| CloudSyncError::ReadFile {
                path: PathBuf::from(path),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "snippet blob missing"),
            })
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

    fn read(&self, path: &str) -> Result<Vec<u8>, CloudSyncError> {
        let path = self.path_for(path)?;
        std::fs::read(&path).map_err(|source| CloudSyncError::ReadFile { path, source })
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
                    entries.sort_by(|left, right| right.timestamp_ms.cmp(&left.timestamp_ms));
                    return Ok(entries);
                }
            }
        }
    }
    entries.sort_by(|left, right| right.timestamp_ms.cmp(&left.timestamp_ms));
    Ok(entries)
}

pub fn push_local_snapshot(
    options: &LocalCloudSyncOptions,
    state: &CloudSyncState,
    force: bool,
) -> Result<CloudSyncResult, CloudSyncError> {
    let remote = LocalDirectoryRemote::new(options.remote_dir.clone());
    push_snapshot_with_remote(options, &remote, state, force)
}

pub fn push_snapshot_with_remote(
    options: &LocalCloudSyncOptions,
    remote: &dyn CloudSyncRemote,
    state: &CloudSyncState,
    force: bool,
) -> Result<CloudSyncResult, CloudSyncError> {
    ensure_enabled(options)?;
    ensure_remote_layout(remote, &options.remote_root)?;
    let mut next_state = normalized_state(state, &options.device_id);
    let mut snapshot = build_sync_snapshot(options)?;
    snapshot.recalculate_hash()?;
    let local_hash = snapshot.meta.payload_hash.clone();
    let latest = load_sync_pointer_from_remote(remote, &options.remote_root)?;

    if let Some(remote_pointer) = &latest
        && remote_pointer.payload_hash == local_hash
    {
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
        persist_cloud_sync_state(options, &result.state)?;
        return Ok(result);
    }

    let remote_changed = latest.as_ref().is_some_and(|remote| {
        next_state
            .last_applied_remote_revision
            .as_deref()
            .map_or(true, |revision| revision != remote.revision_id)
    });
    let local_changed = next_state
        .last_synced_payload_hash
        .as_deref()
        .map_or(true, |hash| hash != local_hash);

    if remote_changed && !force {
        let remote_pointer = latest.expect("remote changed requires remote pointer");
        if local_changed {
            let conflict =
                conflict_preview(options, remote.provider(), &local_hash, &remote_pointer);
            return Err(CloudSyncError::Conflict(conflict.message));
        }
        return Err(CloudSyncError::RemoteNewer);
    }

    write_current_sync_snapshot(remote, &options.remote_root, options, &snapshot)?;
    let pointer = RemoteSyncPointer {
        revision_id: snapshot.meta.revision_id.clone(),
        created_at_ms: snapshot.meta.created_at_ms,
        payload_hash: snapshot.meta.payload_hash.clone(),
        device_id: snapshot.meta.device_id.clone(),
        app_version: snapshot.meta.app_version.clone(),
    };
    write_sync_pointer(remote, &options.remote_root, &pointer)?;

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
    persist_cloud_sync_state(options, &result.state)?;
    Ok(result)
}

pub fn pull_local_snapshot(
    options: &LocalCloudSyncOptions,
    state: &CloudSyncState,
    force: bool,
) -> Result<CloudSyncResult, CloudSyncError> {
    let remote = LocalDirectoryRemote::new(options.remote_dir.clone());
    pull_snapshot_with_remote(options, &remote, state, force)
}

pub fn pull_snapshot_with_remote(
    options: &LocalCloudSyncOptions,
    remote: &dyn CloudSyncRemote,
    state: &CloudSyncState,
    force: bool,
) -> Result<CloudSyncResult, CloudSyncError> {
    ensure_enabled(options)?;
    ensure_remote_layout(remote, &options.remote_root)?;
    let latest = load_sync_pointer_from_remote(remote, &options.remote_root)?
        .ok_or(CloudSyncError::NoRemoteSnapshot)?;
    let mut next_state = normalized_state(state, &options.device_id);
    let mut local_snapshot = build_sync_snapshot(options)?;
    local_snapshot.recalculate_hash()?;

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
        persist_cloud_sync_state(options, &result.state)?;
        return Ok(result);
    }

    let local_changed = next_state
        .last_synced_payload_hash
        .as_deref()
        .map_or(true, |hash| hash != local_snapshot.meta.payload_hash);
    let remote_changed = next_state
        .last_applied_remote_revision
        .as_deref()
        .map_or(true, |revision| revision != latest.revision_id);

    if remote_changed && local_changed && !force {
        let conflict = conflict_preview(
            options,
            remote.provider(),
            &local_snapshot.meta.payload_hash,
            &latest,
        );
        return Err(CloudSyncError::Conflict(conflict.message));
    }
    if !remote_changed && !force {
        return Err(CloudSyncError::NoNewerRemoteSnapshot);
    }

    let snapshot = read_sync_snapshot(remote, &options.remote_root, options, &latest)?;
    let backup = apply_sync_snapshot(options, &snapshot)?;
    write_current_sync_snapshot(remote, &options.remote_root, options, &snapshot)?;

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
    persist_cloud_sync_state(options, &result.state)?;
    Ok(result)
}

pub fn load_sync_pointer(
    options: &LocalCloudSyncOptions,
) -> Result<Option<RemoteSyncPointer>, CloudSyncError> {
    let remote = LocalDirectoryRemote::new(options.remote_dir.clone());
    load_sync_pointer_from_remote(&remote, &options.remote_root)
}

pub fn load_sync_pointer_from_remote(
    remote: &dyn CloudSyncRemote,
    remote_root: &str,
) -> Result<Option<RemoteSyncPointer>, CloudSyncError> {
    let path = remote_path(remote_root, SYNC_LATEST_FILE);
    let Some(bytes) = remote.read_if_exists(&path)? else {
        return Ok(None);
    };
    decode_redb_json_doc(bytes.as_slice()).map(Some)
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

fn build_sync_snapshot(
    options: &LocalCloudSyncOptions,
) -> Result<RawPortableSnapshot, CloudSyncError> {
    let store = ConnectionStore::open_with_portable_key_path(
        &options.config_dir,
        options.portable_key_path.clone(),
    )?;
    let snapshot = store.build_raw_portable_snapshot(
        PortableSnapshotKind::Sync,
        options.device_id.clone(),
        options.app_version.clone(),
    )?;
    Ok(snapshot)
}

fn write_current_sync_snapshot(
    remote: &dyn CloudSyncRemote,
    remote_root: &str,
    options: &LocalCloudSyncOptions,
    snapshot: &RawPortableSnapshot,
) -> Result<(), CloudSyncError> {
    let bytes = encode_encrypted_raw_portable_snapshot(snapshot, &options.master_password)?;
    remote.write(&remote_path(remote_root, SYNC_CURRENT_FILE), &bytes)?;
    remote.write(
        &remote_path(
            remote_root,
            &legacy_sync_snapshot_file(&snapshot.meta.revision_id),
        ),
        &bytes,
    )
}

fn read_sync_snapshot(
    remote: &dyn CloudSyncRemote,
    remote_root: &str,
    options: &LocalCloudSyncOptions,
    pointer: &RemoteSyncPointer,
) -> Result<RawPortableSnapshot, CloudSyncError> {
    if let Some(bytes) = remote.read_if_exists(&remote_path(remote_root, SYNC_CURRENT_FILE))? {
        let snapshot = decode_encrypted_raw_portable_snapshot(&bytes, &options.master_password)?;
        if snapshot.meta.revision_id == pointer.revision_id {
            return Ok(snapshot);
        }
    }
    let legacy = legacy_sync_snapshot_file(&pointer.revision_id);
    let bytes = remote.read(&remote_path(remote_root, &legacy))?;
    Ok(decode_encrypted_raw_portable_snapshot(
        &bytes,
        &options.master_password,
    )?)
}

fn apply_sync_snapshot(
    options: &LocalCloudSyncOptions,
    snapshot: &RawPortableSnapshot,
) -> Result<ConfigBackupInfo, CloudSyncError> {
    std::fs::create_dir_all(&options.config_dir).map_err(|source| CloudSyncError::CreateDir {
        path: options.config_dir.clone(),
        source,
    })?;
    let store = ConnectionStore::open_with_portable_key_path(
        &options.config_dir,
        options.portable_key_path.clone(),
    )?;
    let database_path = store.db_path().to_path_buf();
    // On Windows, redb keeps file ranges locked while the database handle is
    // alive. Release it before copying the current DB for the safety backup.
    drop(store);
    let safety_backup_path = if database_path.exists() {
        let path = options.config_dir.join(format!(
            "nyaterm.redb.cloud-sync-backup-{}.redb",
            current_time_ms()
        ));
        std::fs::copy(&database_path, &path).map_err(|source| CloudSyncError::WriteFile {
            path: path.clone(),
            source,
        })?;
        Some(path)
    } else {
        None
    };
    let store = ConnectionStore::open_with_portable_key_path(
        &options.config_dir,
        options.portable_key_path.clone(),
    )?;
    if let Err(error) = store.apply_raw_portable_snapshot(snapshot) {
        if let Some(backup) = &safety_backup_path {
            let _ = std::fs::copy(backup, &database_path);
        }
        return Err(error.into());
    }
    Ok(ConfigBackupInfo {
        database_path,
        backup_path: PathBuf::from("cloud-sync"),
        bytes: 0,
        safety_backup_path,
    })
}

fn persist_cloud_sync_state(
    options: &LocalCloudSyncOptions,
    state: &CloudSyncState,
) -> Result<(), CloudSyncError> {
    let store = ConnectionStore::open_with_portable_key_path(
        &options.config_dir,
        options.portable_key_path.clone(),
    )?;
    store.save_cloud_sync_state(state)?;
    Ok(())
}

fn write_sync_pointer(
    remote: &dyn CloudSyncRemote,
    remote_root: &str,
    pointer: &RemoteSyncPointer,
) -> Result<(), CloudSyncError> {
    let bytes = encode_redb_json_doc(pointer)?;
    remote.write(&remote_path(remote_root, SYNC_LATEST_FILE), &bytes)
}

fn encode_redb_json_doc(pointer: &RemoteSyncPointer) -> Result<Vec<u8>, CloudSyncError> {
    let temp = TempRedbFile::new("cloud-meta-encode");
    {
        let db = Database::create(temp.path())?;
        let txn = db.begin_write()?;
        {
            let mut docs = txn.open_table(REMOTE_SYNC_POINTER_TABLE)?;
            let content = serde_json::to_string(pointer)?;
            docs.insert(REMOTE_SYNC_POINTER_KEY, content.as_str())?;
        }
        txn.commit()?;
    }
    Ok(std::fs::read(temp.path())?)
}

fn decode_redb_json_doc(bytes: &[u8]) -> Result<RemoteSyncPointer, CloudSyncError> {
    let temp = TempRedbFile::new("cloud-meta-decode");
    std::fs::write(temp.path(), bytes)?;
    let db = Database::open(temp.path())?;
    let read = db.begin_read()?;
    let docs = read.open_table(REMOTE_SYNC_POINTER_TABLE)?;
    let content = docs
        .get(REMOTE_SYNC_POINTER_KEY)?
        .ok_or(CloudSyncError::NoRemoteSnapshot)?
        .value()
        .to_string();
    Ok(serde_json::from_str(&content)?)
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
        local_payload_hash: local_hash.to_string(),
        remote_payload_hash: remote.payload_hash.clone(),
        remote_revision: remote.revision_id.clone(),
        remote_created_at_ms: remote.created_at_ms,
        remote_device_id: remote.device_id.clone(),
        message: format!(
            "Both local and cloud state changed since last sync ({})",
            options.remote_dir.display()
        ),
    }
}

fn result(
    state: CloudSyncState,
    provider: &str,
    status_state: &str,
    message: &str,
    pointer: Option<RemoteSyncPointer>,
    conflict: Option<CloudConflictPreview>,
    backup: Option<ConfigBackupInfo>,
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

struct TempRedbFile {
    path: PathBuf,
}

impl TempRedbFile {
    fn new(prefix: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "nyaterm-{prefix}-{}-{}.redb",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let _ = std::fs::remove_file(&path);
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempRedbFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AiExecutionProfile, ConnectionType, SavedConnection, SessionsConfig};
    use std::collections::HashMap;
    use std::collections::VecDeque;
    use std::sync::Arc;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryRemote {
        files: Mutex<HashMap<String, Vec<u8>>>,
    }

    #[derive(Default)]
    struct MemorySnippetBackend {
        blobs: Mutex<std::collections::BTreeMap<String, String>>,
    }

    #[derive(Clone)]
    struct RecordingSnippetHttpClient {
        inner: Arc<RecordingSnippetHttpClientInner>,
    }

    struct RecordingSnippetHttpClientInner {
        requests: Mutex<Vec<SnippetHttpRequest>>,
        responses: Mutex<VecDeque<Result<SnippetHttpResponse, CloudSyncError>>>,
    }

    impl RecordingSnippetHttpClient {
        fn new(responses: Vec<SnippetHttpResponse>) -> Self {
            Self {
                inner: Arc::new(RecordingSnippetHttpClientInner {
                    requests: Mutex::new(Vec::new()),
                    responses: Mutex::new(responses.into_iter().map(Ok).collect()),
                }),
            }
        }

        fn requests(&self) -> Vec<SnippetHttpRequest> {
            self.inner
                .requests
                .lock()
                .expect("http requests lock")
                .clone()
        }
    }

    impl SnippetHttpClient for RecordingSnippetHttpClient {
        fn send(&self, request: SnippetHttpRequest) -> Result<SnippetHttpResponse, CloudSyncError> {
            self.inner
                .requests
                .lock()
                .expect("http requests lock")
                .push(request);
            self.inner
                .responses
                .lock()
                .expect("http responses lock")
                .pop_front()
                .expect("queued response")
        }
    }

    impl SnippetBlobBackend for MemorySnippetBackend {
        fn fetch_blob(&self, filename: &str) -> Result<Option<String>, CloudSyncError> {
            Ok(self
                .blobs
                .lock()
                .expect("snippet lock")
                .get(filename)
                .cloned())
        }

        fn patch_blobs(
            &self,
            files: std::collections::BTreeMap<String, Option<String>>,
        ) -> Result<(), CloudSyncError> {
            let mut blobs = self.blobs.lock().expect("snippet lock");
            for (filename, content) in files {
                match content {
                    Some(content) => {
                        blobs.insert(filename, content);
                    }
                    None => {
                        blobs.remove(&filename);
                    }
                }
            }
            Ok(())
        }
    }

    impl CloudSyncRemote for MemoryRemote {
        fn provider(&self) -> &'static str {
            "memory"
        }

        fn create_dir(&self, _path: &str) -> Result<(), CloudSyncError> {
            Ok(())
        }

        fn read(&self, path: &str) -> Result<Vec<u8>, CloudSyncError> {
            self.files
                .lock()
                .expect("memory lock")
                .get(path)
                .cloned()
                .ok_or_else(|| CloudSyncError::ReadFile {
                    path: PathBuf::from(path),
                    source: std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
                })
        }

        fn read_if_exists(&self, path: &str) -> Result<Option<Vec<u8>>, CloudSyncError> {
            Ok(self.files.lock().expect("memory lock").get(path).cloned())
        }

        fn write(&self, path: &str, bytes: &[u8]) -> Result<(), CloudSyncError> {
            self.files
                .lock()
                .expect("memory lock")
                .insert(path.to_string(), bytes.to_vec());
            Ok(())
        }
    }

    #[test]
    fn remote_path_joins_without_duplicate_slashes() {
        assert_eq!(
            remote_path("nyaterm", "sync/latest.redb"),
            "nyaterm/sync/latest.redb"
        );
        assert_eq!(
            remote_path("/nyaterm/", "/sync/latest.redb"),
            "nyaterm/sync/latest.redb"
        );
        assert_eq!(remote_path("", "sync/latest.redb"), "sync/latest.redb");
    }

    #[test]
    fn drive_remote_segments_trim_root_and_child_paths() {
        assert_eq!(
            drive_remote_segments("/root/", "/sync/latest.redb"),
            vec!["root", "sync", "latest.redb"]
        );
        assert_eq!(
            drive_remote_segments("", "nyaterm//sync/latest.redb"),
            vec!["nyaterm", "sync", "latest.redb"]
        );
    }

    #[test]
    fn google_drive_query_literal_escapes_quotes_and_backslashes() {
        assert_eq!(google_drive_query_literal("a'b\\c"), "'a\\'b\\\\c'");
    }

    #[test]
    fn s3_signed_request_uses_path_style_url_and_headers() {
        let settings = S3SyncSettings {
            endpoint: "https://s3.example.com/".to_string(),
            bucket: "nyaterm-sync".to_string(),
            region: "ap-east-1".to_string(),
            root: "/profiles/default/".to_string(),
            access_key_id: Some("AKIDEXAMPLE".to_string()),
            secret_access_key: Some("SECRET".to_string()),
            session_token: Some("SESSION".to_string()),
            virtual_host_style: false,
        };
        let request = build_s3_signed_request(
            &settings,
            S3HttpMethod::Put,
            "/nyaterm/sync/latest redb",
            &s3_payload_sha256(b"payload"),
            UNIX_EPOCH + Duration::from_secs(1_704_067_200),
        )
        .expect("signed request");

        assert_eq!(
            request.url,
            "https://s3.example.com/nyaterm-sync/profiles/default/nyaterm/sync/latest%20redb"
        );
        assert_eq!(
            request.headers.get("x-amz-date").map(String::as_str),
            Some("20240101T000000Z")
        );
        assert_eq!(
            request
                .headers
                .get("x-amz-security-token")
                .map(String::as_str),
            Some("SESSION")
        );
        let authorization = request.headers.get("authorization").expect("authorization");
        assert!(
            authorization.contains("Credential=AKIDEXAMPLE/20240101/ap-east-1/s3/aws4_request")
        );
        assert!(
            authorization.contains(
                "SignedHeaders=host;x-amz-content-sha256;x-amz-date;x-amz-security-token"
            )
        );
    }

    #[test]
    fn s3_signed_request_supports_virtual_host_style() {
        let settings = S3SyncSettings {
            endpoint: "https://objects.example.com/base".to_string(),
            bucket: "nyaterm".to_string(),
            region: String::new(),
            root: String::new(),
            access_key_id: Some("key".to_string()),
            secret_access_key: Some("secret".to_string()),
            session_token: None,
            virtual_host_style: true,
        };
        let request = build_s3_signed_request(
            &settings,
            S3HttpMethod::Get,
            "sync/current.redb.enc",
            &s3_payload_sha256(&[]),
            UNIX_EPOCH,
        )
        .expect("signed request");

        assert_eq!(
            request.url,
            "https://nyaterm.objects.example.com/base/sync/current.redb.enc"
        );
        assert_eq!(
            request.headers.get("host").map(String::as_str),
            Some("nyaterm.objects.example.com")
        );
        assert!(request.headers["authorization"].contains("/19700101/us-east-1/s3/aws4_request"));
    }

    #[test]
    fn s3_signed_request_requires_bucket_and_credentials() {
        let settings = S3SyncSettings {
            endpoint: "https://s3.example.com".to_string(),
            access_key_id: Some("key".to_string()),
            secret_access_key: Some("secret".to_string()),
            ..S3SyncSettings::default()
        };
        let error = build_s3_signed_request(
            &settings,
            S3HttpMethod::Head,
            "sync/latest.redb",
            &s3_payload_sha256(&[]),
            UNIX_EPOCH,
        )
        .expect_err("missing bucket");
        assert!(error.to_string().contains("S3 bucket is required"));

        let settings = S3SyncSettings {
            endpoint: "https://s3.example.com".to_string(),
            bucket: "bucket".to_string(),
            ..S3SyncSettings::default()
        };
        let error = build_s3_signed_request(
            &settings,
            S3HttpMethod::Head,
            "sync/latest.redb",
            &s3_payload_sha256(&[]),
            UNIX_EPOCH,
        )
        .expect_err("missing access key");
        assert!(error.to_string().contains("S3 access key ID is required"));
    }

    #[test]
    fn cloud_sync_history_append_and_read_matches_legacy_log_shape() {
        let dir = unique_temp_dir("cloud-history-append");
        let entry = CloudSyncHistoryEntry {
            id: "history-1".to_string(),
            timestamp_ms: 300,
            kind: "sync".to_string(),
            status: "success".to_string(),
            trigger: "manual_push".to_string(),
            provider: Some("local_directory".to_string()),
            revision: Some("rev-1".to_string()),
            duration_ms: Some(42),
            message: "uploaded".to_string(),
        };

        append_cloud_sync_history(&dir, &entry).expect("append history");
        let entries = read_cloud_sync_history(&dir, 7, CLOUD_SYNC_HISTORY_LIMIT)
            .expect("read appended history");

        assert_eq!(entries, vec![entry]);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn cloud_sync_history_reads_only_recent_cloud_entries_with_limit() {
        let dir = unique_temp_dir("cloud-history-limit");
        let path = dir.join(format!(
            "{}-legacy.{}",
            crate::diagnostics::LOG_FILE_PREFIX,
            crate::diagnostics::LOG_FILE_SUFFIX
        ));
        let lines = [
            serde_json::json!({
                "domain": "session.lifecycle",
                "event": "entry",
                "message": "ignored",
                "data": {
                    "id": "ignored",
                    "timestamp_ms": 999,
                    "kind": "sync",
                    "status": "success",
                    "trigger": "manual_push"
                }
            })
            .to_string(),
            history_line("old", 100),
            history_line("new", 200),
        ];
        std::fs::write(&path, lines.join("\n")).expect("write legacy history log");

        let entries = read_cloud_sync_history(&dir, 7, 1).expect("read history");

        assert_eq!(
            entries
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["new"]
        );
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn local_cloud_sync_push_and_forced_pull_round_trip() {
        let source_dir = unique_temp_dir("cloud-source");
        let target_dir = unique_temp_dir("cloud-target");
        let remote_dir = unique_temp_dir("cloud-remote");
        let source_options = options(&source_dir, &remote_dir, "source-device");
        let target_options = options(&target_dir, &remote_dir, "target-device");
        let source_store = ConnectionStore::open(&source_dir).expect("source store");
        source_store
            .replace_sessions(&SessionsConfig {
                groups: Vec::new(),
                connections: vec![local_connection("conn-1", "Synced Shell", "bash")],
            })
            .expect("seed source");
        drop(source_store);

        let push = push_local_snapshot(&source_options, &CloudSyncState::default(), false)
            .expect("push snapshot");
        assert_eq!(push.status.message, "Cloud sync snapshot uploaded");
        assert!(remote_dir.join("nyaterm/sync/current.redb.enc").exists());
        assert!(remote_dir.join("nyaterm/sync/latest.redb").exists());
        let saved_source_state = ConnectionStore::open(&source_dir)
            .expect("source reopen")
            .load_cloud_sync_state()
            .expect("source cloud state");
        assert_eq!(
            saved_source_state.last_synced_payload_hash,
            push.state.last_synced_payload_hash
        );

        let pull = pull_local_snapshot(&target_options, &CloudSyncState::default(), true)
            .expect("pull snapshot");
        assert_eq!(pull.status.message, "Cloud sync snapshot downloaded");
        assert!(pull.backup.is_some());
        let saved_target_state = ConnectionStore::open(&target_dir)
            .expect("target reopen")
            .load_cloud_sync_state()
            .expect("target cloud state");
        assert_eq!(
            saved_target_state.last_applied_remote_revision,
            pull.state.last_applied_remote_revision
        );

        let loaded = ConnectionStore::open(&target_dir)
            .expect("target store")
            .load_sessions()
            .expect("load target");
        assert_eq!(loaded.connections[0].name, "Synced Shell");
        assert_eq!(
            pull.state.last_synced_payload_hash,
            push.state.last_synced_payload_hash
        );

        std::fs::remove_dir_all(source_dir).ok();
        std::fs::remove_dir_all(target_dir).ok();
        std::fs::remove_dir_all(remote_dir).ok();
    }

    #[test]
    fn cloud_sync_algorithm_uses_remote_backend_abstraction() {
        let source_dir = unique_temp_dir("cloud-remote-source");
        let target_dir = unique_temp_dir("cloud-remote-target");
        let remote_dir = unique_temp_dir("cloud-remote-unused");
        let source_options = options(&source_dir, &remote_dir, "source-device");
        let target_options = options(&target_dir, &remote_dir, "target-device");
        let remote = MemoryRemote::default();

        ConnectionStore::open(&source_dir)
            .expect("source store")
            .replace_sessions(&SessionsConfig {
                groups: Vec::new(),
                connections: vec![local_connection("conn-1", "Remote Trait Shell", "bash")],
            })
            .expect("seed source");

        let push =
            push_snapshot_with_remote(&source_options, &remote, &CloudSyncState::default(), false)
                .expect("push through memory remote");
        assert_eq!(push.status.provider, "memory");
        assert!(
            remote
                .read_if_exists("nyaterm/sync/latest.redb")
                .expect("read pointer")
                .is_some()
        );

        let pull =
            pull_snapshot_with_remote(&target_options, &remote, &CloudSyncState::default(), true)
                .expect("pull through memory remote");
        assert_eq!(pull.status.provider, "memory");

        let loaded = ConnectionStore::open(&target_dir)
            .expect("target store")
            .load_sessions()
            .expect("load target");
        assert_eq!(loaded.connections[0].name, "Remote Trait Shell");

        std::fs::remove_dir_all(source_dir).ok();
        std::fs::remove_dir_all(target_dir).ok();
        std::fs::remove_dir_all(remote_dir).ok();
    }

    #[test]
    fn snippet_remote_codec_matches_legacy_blob_layout_and_syncs() {
        let source_dir = unique_temp_dir("cloud-snippet-source");
        let target_dir = unique_temp_dir("cloud-snippet-target");
        let remote_dir = unique_temp_dir("cloud-snippet-unused");
        let source_options = options(&source_dir, &remote_dir, "source-device");
        let target_options = options(&target_dir, &remote_dir, "target-device");
        let backend = MemorySnippetBackend::default();
        let remote = SnippetRemote::new("gitee_snippet", backend);

        assert_eq!(
            snippet_remote_path(&snippet_remote_filename("nyaterm/sync/latest.redb")).as_deref(),
            Some("nyaterm/sync/latest.redb")
        );
        assert_eq!(
            decode_snippet_blob(&encode_snippet_blob(b"hello")).expect("decode"),
            b"hello"
        );

        ConnectionStore::open(&source_dir)
            .expect("source store")
            .replace_sessions(&SessionsConfig {
                groups: Vec::new(),
                connections: vec![local_connection("conn-1", "Snippet Shell", "bash")],
            })
            .expect("seed source");

        let push =
            push_snapshot_with_remote(&source_options, &remote, &CloudSyncState::default(), false)
                .expect("push snippet");
        assert_eq!(push.status.provider, "gitee_snippet");
        assert!(
            remote
                .read_if_exists("nyaterm/sync/latest.redb")
                .expect("snippet pointer")
                .is_some()
        );

        let pull =
            pull_snapshot_with_remote(&target_options, &remote, &CloudSyncState::default(), true)
                .expect("pull snippet");
        assert_eq!(pull.status.provider, "gitee_snippet");

        let loaded = ConnectionStore::open(&target_dir)
            .expect("target store")
            .load_sessions()
            .expect("load target");
        assert_eq!(loaded.connections[0].name, "Snippet Shell");

        std::fs::remove_dir_all(source_dir).ok();
        std::fs::remove_dir_all(target_dir).ok();
        std::fs::remove_dir_all(remote_dir).ok();
    }

    #[test]
    fn gitee_http_backend_fetches_raw_filename_with_access_token() {
        let settings = GiteeSnippetSyncSettings {
            api_endpoint: "https://gitee.example/api/v5/".to_string(),
            gist_id: "gist-1".to_string(),
            access_token: Some("token-1".to_string()),
        };
        let client = RecordingSnippetHttpClient::new(vec![SnippetHttpResponse {
            status: 200,
            body: encode_snippet_blob(b"hello"),
        }]);
        let backend = GiteeSnippetHttpBackend::new(&settings, client.clone()).expect("backend");

        let content = backend
            .fetch_blob(&snippet_remote_filename("nyaterm/sync/latest.redb"))
            .expect("fetch blob")
            .expect("blob");

        assert_eq!(decode_snippet_blob(&content).expect("decode"), b"hello");
        let requests = client.requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, SnippetHttpMethod::Get);
        assert_eq!(
            requests[0].query.get("access_token").map(String::as_str),
            Some("token-1")
        );
        assert!(requests[0].url.contains("/gists/gist-1/raw/nyaterm-"));
    }

    #[test]
    fn github_gist_http_backend_fetches_raw_url_for_truncated_file() {
        let filename = snippet_remote_filename("nyaterm/sync/current.redb.enc");
        let settings = GithubGistSyncSettings {
            gist_id: "gist-2".to_string(),
            access_token: Some("gh-token".to_string()),
        };
        let document = serde_json::json!({
            "files": {
                filename.clone(): {
                    "content": "partial",
                    "raw_url": "https://gist.example/raw/file",
                    "truncated": true
                }
            }
        });
        let client = RecordingSnippetHttpClient::new(vec![
            SnippetHttpResponse {
                status: 200,
                body: document.to_string(),
            },
            SnippetHttpResponse {
                status: 200,
                body: encode_snippet_blob(b"full"),
            },
        ]);
        let backend = GithubGistHttpBackend::new(&settings, client.clone()).expect("backend");

        let content = backend
            .fetch_blob(&filename)
            .expect("fetch blob")
            .expect("blob");

        assert_eq!(decode_snippet_blob(&content).expect("decode"), b"full");
        let requests = client.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].url, "https://api.github.com/gists/gist-2");
        assert_eq!(requests[1].url, "https://gist.example/raw/file");
        assert_eq!(
            requests[0].headers.get("Authorization").map(String::as_str),
            Some("Bearer gh-token")
        );
    }

    #[test]
    fn github_gist_http_backend_retries_retryable_update_conflict() {
        let settings = GithubGistSyncSettings {
            gist_id: "gist-3".to_string(),
            access_token: Some("gh-token".to_string()),
        };
        let client = RecordingSnippetHttpClient::new(vec![
            SnippetHttpResponse {
                status: 409,
                body: r#"{"message":"Gist cannot be updated."}"#.to_string(),
            },
            SnippetHttpResponse {
                status: 200,
                body: "{}".to_string(),
            },
        ]);
        let backend = GithubGistHttpBackend::new(&settings, client.clone()).expect("backend");
        let mut files = BTreeMap::new();
        files.insert("nyaterm-rev.blob".to_string(), Some("payload".to_string()));

        backend.patch_blobs(files).expect("patch retry");

        let requests = client.requests();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0], requests[1]);
        assert_eq!(requests[0].method, SnippetHttpMethod::Patch);
    }

    #[test]
    fn snippet_patch_bodies_match_gitee_and_github_shapes() {
        let mut files = BTreeMap::new();
        files.insert("nyaterm-a.blob".to_string(), Some("payload".to_string()));
        files.insert("nyaterm-b.blob".to_string(), None);

        let gitee = gitee_snippet_patch_body("token", files.clone());
        assert_eq!(gitee["access_token"], "token");
        assert_eq!(gitee["files"]["nyaterm-a.blob"]["content"], "payload");
        assert!(gitee["files"]["nyaterm-b.blob"].is_null());

        let github = github_gist_patch_body(files);
        assert!(github.get("access_token").is_none());
        assert_eq!(github["files"]["nyaterm-a.blob"]["content"], "payload");
        assert!(github["files"]["nyaterm-b.blob"].is_null());
    }

    #[test]
    fn local_cloud_sync_detects_push_conflict() {
        let source_dir = unique_temp_dir("cloud-conflict-source");
        let other_dir = unique_temp_dir("cloud-conflict-other");
        let remote_dir = unique_temp_dir("cloud-conflict-remote");
        let source_options = options(&source_dir, &remote_dir, "source-device");
        let other_options = options(&other_dir, &remote_dir, "other-device");

        ConnectionStore::open(&source_dir)
            .expect("source")
            .replace_sessions(&SessionsConfig {
                groups: Vec::new(),
                connections: vec![local_connection("conn-1", "Local A", "bash")],
            })
            .expect("seed source");
        let source_state = push_local_snapshot(&source_options, &CloudSyncState::default(), false)
            .expect("initial push")
            .state;

        ConnectionStore::open(&other_dir)
            .expect("other")
            .replace_sessions(&SessionsConfig {
                groups: Vec::new(),
                connections: vec![local_connection("conn-2", "Remote B", "zsh")],
            })
            .expect("seed other");
        push_local_snapshot(&other_options, &CloudSyncState::default(), true)
            .expect("remote force push");

        ConnectionStore::open(&source_dir)
            .expect("source reopen")
            .replace_sessions(&SessionsConfig {
                groups: Vec::new(),
                connections: vec![local_connection("conn-1", "Local Changed", "fish")],
            })
            .expect("change source");
        let error = push_local_snapshot(&source_options, &source_state, false)
            .expect_err("conflict expected");
        assert!(matches!(error, CloudSyncError::Conflict(_)));

        std::fs::remove_dir_all(source_dir).ok();
        std::fs::remove_dir_all(other_dir).ok();
        std::fs::remove_dir_all(remote_dir).ok();
    }

    #[test]
    fn local_cloud_sync_detects_pull_conflict_until_forced() {
        let source_dir = unique_temp_dir("cloud-pull-conflict-source");
        let target_dir = unique_temp_dir("cloud-pull-conflict-target");
        let other_dir = unique_temp_dir("cloud-pull-conflict-other");
        let remote_dir = unique_temp_dir("cloud-pull-conflict-remote");
        let source_options = options(&source_dir, &remote_dir, "source-device");
        let target_options = options(&target_dir, &remote_dir, "target-device");
        let other_options = options(&other_dir, &remote_dir, "other-device");

        ConnectionStore::open(&source_dir)
            .expect("source")
            .replace_sessions(&SessionsConfig {
                groups: Vec::new(),
                connections: vec![local_connection("conn-1", "Initial", "bash")],
            })
            .expect("seed source");
        push_local_snapshot(&source_options, &CloudSyncState::default(), false)
            .expect("initial push");
        let target_state = pull_local_snapshot(&target_options, &CloudSyncState::default(), true)
            .expect("initial pull")
            .state;

        ConnectionStore::open(&other_dir)
            .expect("other")
            .replace_sessions(&SessionsConfig {
                groups: Vec::new(),
                connections: vec![local_connection("conn-2", "Remote Changed", "zsh")],
            })
            .expect("seed other");
        push_local_snapshot(&other_options, &CloudSyncState::default(), true)
            .expect("remote force push");

        ConnectionStore::open(&target_dir)
            .expect("target reopen")
            .replace_sessions(&SessionsConfig {
                groups: Vec::new(),
                connections: vec![local_connection("conn-1", "Local Changed", "fish")],
            })
            .expect("change target");

        let error = pull_local_snapshot(&target_options, &target_state, false)
            .expect_err("pull conflict expected");
        assert!(matches!(error, CloudSyncError::Conflict(_)));

        pull_local_snapshot(&target_options, &target_state, true).expect("forced pull");
        let loaded = ConnectionStore::open(&target_dir)
            .expect("target final")
            .load_sessions()
            .expect("load target");
        assert_eq!(loaded.connections[0].name, "Remote Changed");

        std::fs::remove_dir_all(source_dir).ok();
        std::fs::remove_dir_all(target_dir).ok();
        std::fs::remove_dir_all(other_dir).ok();
        std::fs::remove_dir_all(remote_dir).ok();
    }

    #[test]
    fn local_cloud_sync_wrong_password_does_not_replace_target() {
        let source_dir = unique_temp_dir("cloud-password-source");
        let target_dir = unique_temp_dir("cloud-password-target");
        let remote_dir = unique_temp_dir("cloud-password-remote");
        let source_options = options(&source_dir, &remote_dir, "source-device");
        let mut wrong_options = options(&target_dir, &remote_dir, "target-device");
        wrong_options.master_password = "wrong".to_string();

        ConnectionStore::open(&source_dir)
            .expect("source")
            .replace_sessions(&SessionsConfig {
                groups: Vec::new(),
                connections: vec![local_connection("conn-1", "Remote State", "bash")],
            })
            .expect("seed source");
        push_local_snapshot(&source_options, &CloudSyncState::default(), false).expect("push");

        ConnectionStore::open(&target_dir)
            .expect("target")
            .replace_sessions(&SessionsConfig {
                groups: Vec::new(),
                connections: vec![local_connection("keep", "Keep Local", "zsh")],
            })
            .expect("seed target");

        let error = pull_local_snapshot(&wrong_options, &CloudSyncState::default(), true)
            .expect_err("wrong password");
        assert!(
            error
                .to_string()
                .contains("cloud snapshot decryption failed")
        );
        let loaded = ConnectionStore::open(&target_dir)
            .expect("target reopen")
            .load_sessions()
            .expect("load target");
        assert_eq!(loaded.connections[0].name, "Keep Local");

        std::fs::remove_dir_all(source_dir).ok();
        std::fs::remove_dir_all(target_dir).ok();
        std::fs::remove_dir_all(remote_dir).ok();
    }

    #[test]
    fn masked_cloud_sync_merge_preserves_provider_secrets() {
        let mut current = CloudSyncSettings::default();
        current.webdav.password = Some("webdav-password".to_string());
        current.s3.secret_access_key = Some("s3-secret".to_string());
        current.google_drive.access_token = Some("google-access".to_string());
        current.google_drive.refresh_token = Some("google-refresh".to_string());
        current.google_drive.client_secret = Some("google-secret".to_string());
        current.onedrive.access_token = Some("onedrive-access".to_string());
        current.aliyun_drive.refresh_token = Some("aliyun-refresh".to_string());
        current.github_gist.access_token = Some("github-token".to_string());

        let mut next = CloudSyncSettings::default();
        next.webdav.password = Some(MASKED_SECRET_VALUE.to_string());
        next.s3.secret_access_key = Some(String::new());
        next.google_drive.access_token = Some(MASKED_SECRET_VALUE.to_string());
        next.google_drive.refresh_token = Some(MASKED_SECRET_VALUE.to_string());
        next.google_drive.client_secret = Some(MASKED_SECRET_VALUE.to_string());
        next.onedrive.access_token = Some(MASKED_SECRET_VALUE.to_string());
        next.aliyun_drive.refresh_token = Some(MASKED_SECRET_VALUE.to_string());
        next.github_gist.access_token = Some("replacement".to_string());

        let merged = merge_masked_cloud_sync_settings(&current, next);

        assert_eq!(merged.webdav.password.as_deref(), Some("webdav-password"));
        assert_eq!(merged.s3.secret_access_key, None);
        assert_eq!(
            merged.google_drive.access_token.as_deref(),
            Some("google-access")
        );
        assert_eq!(
            merged.google_drive.refresh_token.as_deref(),
            Some("google-refresh")
        );
        assert_eq!(
            merged.google_drive.client_secret.as_deref(),
            Some("google-secret")
        );
        assert_eq!(
            merged.onedrive.access_token.as_deref(),
            Some("onedrive-access")
        );
        assert_eq!(
            merged.aliyun_drive.refresh_token.as_deref(),
            Some("aliyun-refresh")
        );
        assert_eq!(
            merged.github_gist.access_token.as_deref(),
            Some("replacement")
        );
    }

    fn options(config_dir: &Path, remote_dir: &Path, device_id: &str) -> LocalCloudSyncOptions {
        LocalCloudSyncOptions {
            config_dir: config_dir.to_path_buf(),
            portable_key_path: None,
            remote_dir: remote_dir.to_path_buf(),
            remote_root: "nyaterm".to_string(),
            device_id: device_id.to_string(),
            app_version: "test".to_string(),
            master_password: "secret".to_string(),
            enabled: true,
        }
    }

    fn local_connection(id: &str, name: &str, shell: &str) -> SavedConnection {
        SavedConnection {
            id: id.to_string(),
            name: name.to_string(),
            config: ConnectionType::LocalTerminal {
                shell_path: shell.to_string(),
                shell_args: String::new(),
                working_dir: None,
                ai_execution_profile: AiExecutionProfile::Auto,
            },
            group_id: None,
            description: None,
            sort_order: 0,
            icon: None,
            icon_auto_detect: None,
            auth: None,
            network: None,
            post_login: None,
            created_at_ms: None,
            updated_at_ms: None,
            last_used_at_ms: None,
        }
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nyaterm-cloud-sync-{name}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    fn history_line(id: &str, timestamp_ms: u64) -> String {
        serde_json::json!({
            "domain": CLOUD_SYNC_HISTORY_DOMAIN,
            "event": CLOUD_SYNC_HISTORY_EVENT,
            "message": format!("history {id}"),
            "data": {
                "id": id,
                "timestamp_ms": timestamp_ms,
                "kind": "sync",
                "status": "success",
                "trigger": "manual_pull",
                "provider": "webdav",
                "revision": null,
                "duration_ms": 1,
            }
        })
        .to_string()
    }
}

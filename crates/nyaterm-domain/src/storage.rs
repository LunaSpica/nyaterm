use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{Engine, engine::general_purpose::STANDARD as B64};
use hmac::{Hmac, Mac, digest::KeyInit as HmacKeyInit};
use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    AiAuditFile, AiAuditLog, AiHistoryFile, AiMessage, AiMessageRole, AiSession, AiSettings,
    AliyunDriveSyncSettings, AppSettingsSummary, AppendAiAuditRequest, CloudSyncSettings,
    CloudSyncState, CommandHistoryEntry, CredentialCrypto, CredentialCryptoError,
    DecryptedOtpEntry, DecryptedSavedCredential, DecryptedSavedPassword, DecryptedSshKey, Group,
    KeywordHighlightConfig, KeywordHighlightImportResult, KeywordHighlightRule,
    OAuthDriveSyncSettings, OtpEntry, PortableSnapshotError, PortableSnapshotKind, ProxyConfig,
    ProxyGroup, ProxyGroupsConfig, QuickCommand, QuickCommandCategory, QuickCommandsConfig,
    SearchEngineConfig, default_search_engines,
    RawPortableSnapshot, SavedConnection, SavedCredential, SavedPassword, SessionsConfig, SshKey,
    TranslationSettings, TunnelConfig, TunnelGroup, TunnelGroupsConfig,
    ai_settings_has_secret, merge_masked_ai_settings, merge_masked_cloud_sync_settings,
    merge_masked_translation_settings, normalize_ai_settings, now_rfc3339,
    translation_settings_has_secret, trim_ai_audit, trim_ai_history, uuid,
};

const DATABASE_FILE: &str = "nyaterm.redb";
const GROUP_PREFIX: &str = "groups/";
const CONNECTION_PREFIX: &str = "connections/";
const TUNNEL_PREFIX: &str = "tunnels/";
const SSH_KEY_PREFIX: &str = "credentials/key/";
const CREDENTIAL_PREFIX: &str = "credentials/credential/";
const PASSWORD_PREFIX: &str = "credentials/password/";
const CONNECTION_PASSWORD_PREFIX: &str = "credentials/connection-password/";
const OTP_PREFIX: &str = "otp_accounts/";
const PROXY_PREFIX: &str = "proxies/";
const KNOWN_HOST_PREFIX: &str = "known_hosts/";
const KNOWN_HOST_RAW_PREFIX: &str = "known_hosts/raw/";
const COMMAND_HISTORY_PREFIX: &str = "command_history/";
const META_MASTER_KEY: &str = "security/master_key";
const LEGACY_TEXT_MASTER_KEY: &str = "master.key";
const LEGACY_TEXT_KNOWN_HOSTS: &str = "known_hosts";
const SETTINGS_DEFAULT: &str = "settings/default";
const SETTINGS_AI_FIELD: &str = "ai";
const SETTINGS_TRANSLATION_FIELD: &str = "translation";
const SETTINGS_AI_HISTORY: &str = "settings/doc/ai-history";
const SETTINGS_AI_AUDIT: &str = "settings/doc/ai-audit";
const SETTINGS_TUNNEL_GROUPS: &str = "settings/doc/tunnel-groups";
const SETTINGS_PROXY_GROUPS: &str = "settings/doc/proxy-groups";
const SETTINGS_CLOUD_SYNC: &str = "settings/doc/cloud-sync";
const SETTINGS_QUICK_COMMANDS: &str = "settings/doc/quick-command";
const SETTINGS_CLOUD_SYNC_STATE: &str = "settings/doc/cloud-sync-state";
const LEGACY_TEXT_CLOUD_SYNC_STATE: &str = "cloud-sync-state";

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum KeywordHighlightImportFile {
    Config {
        keyword_highlights: Vec<KeywordHighlightRule>,
    },
    Rules(Vec<KeywordHighlightRule>),
}

const META_TABLE: TableDefinition<&str, &str> = TableDefinition::new("meta");
const TEXT_DOCS_TABLE: TableDefinition<&str, &str> = TableDefinition::new("text_docs");
const GROUPS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("groups");
const CONNECTIONS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("connections");
const TUNNELS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("tunnels");
const PROXIES_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("proxies");
const CREDENTIALS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("credentials");
const OTP_ACCOUNTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("otp_accounts");
const KNOWN_HOSTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("known_hosts");
const COMMAND_HISTORY_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("command_history");
const SETTINGS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("settings");
const IDX_CONNECTIONS_BY_GROUP_TABLE: TableDefinition<&str, &str> =
    TableDefinition::new("idx_connections_by_group");
const IDX_CONNECTIONS_BY_LAST_USED_TABLE: TableDefinition<&str, &str> =
    TableDefinition::new("idx_connections_by_last_used");
const IDX_CONNECTIONS_BY_PROTOCOL_TABLE: TableDefinition<&str, &str> =
    TableDefinition::new("idx_connections_by_protocol");

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("failed to create storage directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to open redb database {path}: {source}")]
    Open {
        path: PathBuf,
        source: redb::DatabaseError,
    },
    #[error("redb table error: {0}")]
    Table(#[from] redb::TableError),
    #[error("redb storage error: {0}")]
    Storage(#[from] redb::StorageError),
    #[error("redb transaction error: {0}")]
    Transaction(#[from] redb::TransactionError),
    #[error("redb commit error: {0}")]
    Commit(#[from] redb::CommitError),
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("portable snapshot error: {0}")]
    PortableSnapshot(#[from] PortableSnapshotError),
    #[error("portable snapshot entity '{entity}' is invalid: {message}")]
    PortableSnapshotEntity { entity: String, message: String },
    #[error("credential crypto error: {0}")]
    Crypto(#[from] CredentialCryptoError),
    #[error("encrypted credential material exists but master key is missing")]
    MissingMasterKey,
    #[error("configuration backup does not exist: {path}")]
    ConfigBackupMissing { path: PathBuf },
    #[error("configuration backup path is not a file: {path}")]
    ConfigBackupNotFile { path: PathBuf },
    #[error("configuration backup source and destination are the same path: {path}")]
    ConfigBackupSamePath { path: PathBuf },
    #[error("failed to copy configuration database from {from} to {to}: {source}")]
    ConfigBackupCopy {
        from: PathBuf,
        to: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to remove configuration database {path}: {source}")]
    ConfigBackupRemove {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to rename configuration database from {from} to {to}: {source}")]
    ConfigBackupRename {
        from: PathBuf,
        to: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid data: {0}")]
    InvalidData(String),
}

#[derive(Debug)]
pub struct ConnectionStore {
    db: Database,
    db_path: PathBuf,
    portable_key_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigBackupInfo {
    pub database_path: PathBuf,
    pub backup_path: PathBuf,
    pub bytes: u64,
    pub safety_backup_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConnectionPasswordRecord {
    id: String,
    connection_id: String,
    password: String,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownHostCheck {
    Match,
    HostSeen,
    UnknownHost,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct KnownHostRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    marker: Option<String>,
    host_identifier: String,
    #[serde(default)]
    host_patterns: Vec<String>,
    key_type: String,
    key_base64: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    raw_line: Option<String>,
    created_at_ms: u64,
    updated_at_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct KnownHostRawRecord {
    line: String,
    created_at_ms: u64,
    updated_at_ms: u64,
}

type HmacSha1 = Hmac<Sha1>;

impl ConnectionStore {
    pub fn open(config_dir: impl AsRef<Path>) -> Result<Self, StorageError> {
        Self::open_with_portable_key_path(config_dir, None)
    }

    pub fn open_with_portable_key_path(
        config_dir: impl AsRef<Path>,
        portable_key_path: Option<PathBuf>,
    ) -> Result<Self, StorageError> {
        let config_dir = config_dir.as_ref();
        std::fs::create_dir_all(config_dir).map_err(|source| StorageError::CreateDir {
            path: config_dir.to_path_buf(),
            source,
        })?;
        let db_path = config_dir.join(DATABASE_FILE);
        let db = Database::create(&db_path).map_err(|source| StorageError::Open {
            path: db_path.clone(),
            source,
        })?;
        let store = Self {
            db,
            db_path,
            portable_key_path,
        };
        store.ensure_tables()?;
        store.import_legacy_known_hosts_if_needed()?;
        Ok(store)
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn export_config_database(
        config_dir: impl AsRef<Path>,
        portable_key_path: Option<PathBuf>,
        output_path: impl AsRef<Path>,
    ) -> Result<ConfigBackupInfo, StorageError> {
        let store = Self::open_with_portable_key_path(config_dir, portable_key_path)?;
        let database_path = store.db_path().to_path_buf();
        drop(store);

        let backup_path = output_path.as_ref().to_path_buf();
        ensure_not_same_existing_file(&database_path, &backup_path)?;
        ensure_parent_dir(&backup_path)?;
        let bytes = copy_config_database(&database_path, &backup_path)?;

        Ok(ConfigBackupInfo {
            database_path,
            backup_path,
            bytes,
            safety_backup_path: None,
        })
    }

    pub fn import_config_database(
        config_dir: impl AsRef<Path>,
        portable_key_path: Option<PathBuf>,
        input_path: impl AsRef<Path>,
    ) -> Result<ConfigBackupInfo, StorageError> {
        let config_dir = config_dir.as_ref();
        let input_path = input_path.as_ref().to_path_buf();
        validate_config_backup_source(&input_path)?;

        let database_path = config_dir.join(DATABASE_FILE);
        ensure_not_same_existing_file(&database_path, &input_path)?;
        validate_config_backup_file(&input_path, portable_key_path.clone())?;
        std::fs::create_dir_all(config_dir).map_err(|source| StorageError::CreateDir {
            path: config_dir.to_path_buf(),
            source,
        })?;

        let safety_backup_path = if database_path.exists() {
            let backup_path = config_dir.join(format!(
                "{DATABASE_FILE}.import-backup-{}.redb",
                current_time_ms()
            ));
            copy_config_database(&database_path, &backup_path)?;
            Some(backup_path)
        } else {
            None
        };

        let temp_path =
            config_dir.join(format!("{DATABASE_FILE}.import-{}.tmp", current_time_ms()));
        let bytes = copy_config_database(&input_path, &temp_path)?;
        if database_path.exists() {
            std::fs::remove_file(&database_path).map_err(|source| {
                StorageError::ConfigBackupRemove {
                    path: database_path.clone(),
                    source,
                }
            })?;
        }
        std::fs::rename(&temp_path, &database_path).map_err(|source| {
            if let Some(safety_backup_path) = &safety_backup_path {
                let _ = std::fs::copy(safety_backup_path, &database_path);
            }
            StorageError::ConfigBackupRename {
                from: temp_path,
                to: database_path.clone(),
                source,
            }
        })?;

        let store =
            Self::open_with_portable_key_path(config_dir, portable_key_path).inspect_err(|_| {
                if let Some(safety_backup_path) = &safety_backup_path {
                    let _ = std::fs::copy(safety_backup_path, &database_path);
                }
            })?;
        store.load_sessions()?;
        store.load_app_settings_summary()?;
        store.list_tunnels()?;
        drop(store);

        Ok(ConfigBackupInfo {
            database_path,
            backup_path: input_path,
            bytes,
            safety_backup_path,
        })
    }

    pub fn export_portable_snapshot(
        config_dir: impl AsRef<Path>,
        portable_key_path: Option<PathBuf>,
        output_path: impl AsRef<Path>,
        device_id: impl Into<String>,
        app_version: impl Into<String>,
    ) -> Result<ConfigBackupInfo, StorageError> {
        let store = Self::open_with_portable_key_path(config_dir, portable_key_path)?;
        let database_path = store.db_path().to_path_buf();
        let mut snapshot = store.build_raw_portable_snapshot(
            PortableSnapshotKind::Backup,
            device_id,
            app_version,
        )?;
        snapshot.recalculate_hash()?;
        let encoded = crate::portable_snapshot::encode_raw_portable_snapshot(&snapshot)?;

        let backup_path = output_path.as_ref().to_path_buf();
        ensure_parent_dir(&backup_path)?;
        std::fs::write(&backup_path, &encoded).map_err(|source| {
            StorageError::ConfigBackupCopy {
                from: database_path.clone(),
                to: backup_path.clone(),
                source,
            }
        })?;

        Ok(ConfigBackupInfo {
            database_path,
            backup_path,
            bytes: encoded.len().try_into().unwrap_or(u64::MAX),
            safety_backup_path: None,
        })
    }

    pub fn export_encrypted_portable_snapshot(
        config_dir: impl AsRef<Path>,
        portable_key_path: Option<PathBuf>,
        output_path: impl AsRef<Path>,
        device_id: impl Into<String>,
        app_version: impl Into<String>,
        master_password: &str,
    ) -> Result<ConfigBackupInfo, StorageError> {
        let store = Self::open_with_portable_key_path(config_dir, portable_key_path)?;
        let database_path = store.db_path().to_path_buf();
        let mut snapshot = store.build_raw_portable_snapshot(
            PortableSnapshotKind::Backup,
            device_id,
            app_version,
        )?;
        snapshot.recalculate_hash()?;
        let encoded = crate::portable_snapshot::encode_encrypted_raw_portable_snapshot(
            &snapshot,
            master_password,
        )?;
        write_portable_snapshot_file(database_path, output_path, &encoded)
    }

    pub fn import_portable_snapshot(
        config_dir: impl AsRef<Path>,
        portable_key_path: Option<PathBuf>,
        input_path: impl AsRef<Path>,
    ) -> Result<ConfigBackupInfo, StorageError> {
        let config_dir = config_dir.as_ref();
        let input_path = input_path.as_ref().to_path_buf();
        validate_config_backup_source(&input_path)?;
        std::fs::create_dir_all(config_dir).map_err(|source| StorageError::CreateDir {
            path: config_dir.to_path_buf(),
            source,
        })?;
        let bytes =
            std::fs::read(&input_path).map_err(|source| StorageError::ConfigBackupCopy {
                from: input_path.clone(),
                to: config_dir.join(DATABASE_FILE),
                source,
            })?;
        let snapshot = crate::portable_snapshot::decode_raw_portable_snapshot(&bytes)?;

        let database_path = config_dir.join(DATABASE_FILE);
        let safety_backup_path = if database_path.exists() {
            let backup_path = config_dir.join(format!(
                "{DATABASE_FILE}.portable-import-backup-{}.redb",
                current_time_ms()
            ));
            copy_config_database(&database_path, &backup_path)?;
            Some(backup_path)
        } else {
            None
        };

        let store = Self::open_with_portable_key_path(config_dir, portable_key_path)?;
        if let Err(error) = store.apply_raw_portable_snapshot(&snapshot) {
            if let Some(safety_backup_path) = &safety_backup_path {
                let _ = std::fs::copy(safety_backup_path, &database_path);
            }
            return Err(error);
        }
        store.load_sessions()?;
        store.load_app_settings_summary()?;
        store.list_tunnels()?;

        Ok(ConfigBackupInfo {
            database_path,
            backup_path: input_path,
            bytes: bytes.len().try_into().unwrap_or(u64::MAX),
            safety_backup_path,
        })
    }

    pub fn import_encrypted_portable_snapshot(
        config_dir: impl AsRef<Path>,
        portable_key_path: Option<PathBuf>,
        input_path: impl AsRef<Path>,
        master_password: &str,
    ) -> Result<ConfigBackupInfo, StorageError> {
        let config_dir = config_dir.as_ref();
        let input_path = input_path.as_ref().to_path_buf();
        validate_config_backup_source(&input_path)?;
        std::fs::create_dir_all(config_dir).map_err(|source| StorageError::CreateDir {
            path: config_dir.to_path_buf(),
            source,
        })?;
        let bytes =
            std::fs::read(&input_path).map_err(|source| StorageError::ConfigBackupCopy {
                from: input_path.clone(),
                to: config_dir.join(DATABASE_FILE),
                source,
            })?;
        let snapshot = crate::portable_snapshot::decode_encrypted_raw_portable_snapshot(
            &bytes,
            master_password,
        )?;
        Self::apply_portable_snapshot_to_config_dir(
            config_dir,
            portable_key_path,
            input_path,
            bytes.len().try_into().unwrap_or(u64::MAX),
            snapshot,
        )
    }

    fn apply_portable_snapshot_to_config_dir(
        config_dir: &Path,
        portable_key_path: Option<PathBuf>,
        input_path: PathBuf,
        bytes: u64,
        snapshot: RawPortableSnapshot,
    ) -> Result<ConfigBackupInfo, StorageError> {
        let database_path = config_dir.join(DATABASE_FILE);
        let safety_backup_path = if database_path.exists() {
            let backup_path = config_dir.join(format!(
                "{DATABASE_FILE}.portable-import-backup-{}.redb",
                current_time_ms()
            ));
            copy_config_database(&database_path, &backup_path)?;
            Some(backup_path)
        } else {
            None
        };

        let store = Self::open_with_portable_key_path(config_dir, portable_key_path)?;
        if let Err(error) = store.apply_raw_portable_snapshot(&snapshot) {
            if let Some(safety_backup_path) = &safety_backup_path {
                let _ = std::fs::copy(safety_backup_path, &database_path);
            }
            return Err(error);
        }
        store.load_sessions()?;
        store.load_app_settings_summary()?;
        store.list_tunnels()?;

        Ok(ConfigBackupInfo {
            database_path,
            backup_path: input_path,
            bytes,
            safety_backup_path,
        })
    }

    pub fn load_sessions(&self) -> Result<SessionsConfig, StorageError> {
        let groups = self.list_groups()?;
        let mut connections = self.list_connections()?;
        self.hydrate_connection_passwords(&mut connections)?;
        Ok(SessionsConfig {
            groups,
            connections,
        })
    }

    pub fn list_ssh_keys(&self) -> Result<Vec<SshKey>, StorageError> {
        let mut keys = self.list_json_by_prefix(CREDENTIALS_TABLE, SSH_KEY_PREFIX)?;
        for key in &mut keys {
            apply_ssh_key_status_flags(key);
        }
        keys.sort_by(|left: &SshKey, right| {
            left.name.cmp(&right.name).then(left.id.cmp(&right.id))
        });
        Ok(keys)
    }

    pub fn load_ssh_key_by_id(&self, key_id: &str) -> Result<Option<SshKey>, StorageError> {
        let key = entity_key(SSH_KEY_PREFIX, key_id);
        let Some(mut key) = self.read_json_table(CREDENTIALS_TABLE, &key)? else {
            return Ok(None);
        };
        apply_ssh_key_status_flags(&mut key);
        Ok(Some(key))
    }

    pub fn load_decrypted_ssh_key_by_id(
        &self,
        key_id: &str,
    ) -> Result<Option<DecryptedSshKey>, StorageError> {
        let Some(key) = self.load_ssh_key_by_id(key_id)? else {
            return Ok(None);
        };
        let master_key_token = self.load_master_key_token()?;
        let crypto = self.credential_crypto()?;
        let decrypted = DecryptedSshKey {
            id: key.id,
            name: key.name,
            key_data: decrypt_optional_secret(&crypto, master_key_token.as_deref(), &key.key)?,
            cert_data: decrypt_optional_secret(&crypto, master_key_token.as_deref(), &key.cert)?,
            passphrase: decrypt_optional_secret(
                &crypto,
                master_key_token.as_deref(),
                &key.passphrase,
            )?,
        };
        Ok(Some(decrypted))
    }

    pub fn list_otp_entries(&self) -> Result<Vec<OtpEntry>, StorageError> {
        let mut entries: Vec<OtpEntry> =
            self.list_json_by_prefix(OTP_ACCOUNTS_TABLE, OTP_PREFIX)?;
        for entry in &mut entries {
            entry.has_secret = entry.secret.is_some();
        }
        entries.sort_by(|left, right| {
            left.issuer
                .cmp(&right.issuer)
                .then(left.username.cmp(&right.username))
                .then(left.id.cmp(&right.id))
        });
        Ok(entries)
    }

    pub fn load_otp_entry_by_id(&self, otp_id: &str) -> Result<Option<OtpEntry>, StorageError> {
        let key = entity_key(OTP_PREFIX, otp_id);
        let Some(mut entry) = self.read_json_table::<OtpEntry>(OTP_ACCOUNTS_TABLE, &key)? else {
            return Ok(None);
        };
        entry.has_secret = entry.secret.is_some();
        Ok(Some(entry))
    }

    pub fn load_decrypted_otp_entry_by_id(
        &self,
        otp_id: &str,
    ) -> Result<Option<DecryptedOtpEntry>, StorageError> {
        let Some(entry) = self.load_otp_entry_by_id(otp_id)? else {
            return Ok(None);
        };
        let master_key_token = self.load_master_key_token()?;
        let crypto = self.credential_crypto()?;
        Ok(Some(DecryptedOtpEntry {
            id: entry.id,
            otp_type: entry.otp_type,
            issuer: entry.issuer,
            username: entry.username,
            secret: decrypt_optional_secret(&crypto, master_key_token.as_deref(), &entry.secret)?,
            algorithm: entry.algorithm,
            digits: entry.digits,
            period: entry.period,
            counter: entry.counter,
        }))
    }

    pub fn increment_otp_counter(&self, otp_id: &str) -> Result<(), StorageError> {
        let key = entity_key(OTP_PREFIX, otp_id);
        let txn = self.db.begin_write()?;
        {
            let table = txn.open_table(OTP_ACCOUNTS_TABLE)?;
            let Some(raw) = table.get(key.as_str())? else {
                drop(table);
                txn.commit()?;
                return Ok(());
            };
            let mut entry: OtpEntry = deserialize_json(raw.value())?;
            entry.counter = entry.counter.saturating_add(1);
            drop(raw);
            drop(table);
            write_json_in_txn(&txn, OTP_ACCOUNTS_TABLE, &key, &entry)?;
        }
        txn.commit()?;
        Ok(())
    }

    pub fn save_ssh_key(&self, mut key: SshKey) -> Result<String, StorageError> {
        if key.id.trim().is_empty() {
            key.id = uuid::Uuid::new_v4().to_string();
        }
        let target_id = key.id.clone();
        let existing = self.load_ssh_key_by_id(&target_id)?;
        let crypto = self.credential_crypto()?;

        key.key = if let Some(path) = key
            .key_file_path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
        {
            let content = std::fs::read_to_string(path).map_err(|source| {
                StorageError::InvalidData(format!("failed to read key material from {path}: {source}"))
            })?;
            let token = self.get_or_create_master_key_token(&crypto)?;
            Some(crypto.encrypt_secret(&token, &content)?)
        } else if let Some(plain) = key
            .key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            // Treat non-empty draft key material as plaintext replacement.
            let token = self.get_or_create_master_key_token(&crypto)?;
            Some(crypto.encrypt_secret(&token, plain)?)
        } else {
            existing.as_ref().and_then(|entry| entry.key.clone())
        };

        key.cert = if let Some(path) = key
            .cert_file_path
            .as_deref()
            .map(str::trim)
            .filter(|path| !path.is_empty())
        {
            let content = std::fs::read_to_string(path).map_err(|source| {
                StorageError::InvalidData(format!("failed to read certificate from {path}: {source}"))
            })?;
            let token = self.get_or_create_master_key_token(&crypto)?;
            Some(crypto.encrypt_secret(&token, &content)?)
        } else if let Some(plain) = key
            .cert
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            let token = self.get_or_create_master_key_token(&crypto)?;
            Some(crypto.encrypt_secret(&token, plain)?)
        } else {
            existing.as_ref().and_then(|entry| entry.cert.clone())
        };

        key.passphrase = match key.passphrase.as_deref().map(str::trim) {
            Some("") => None,
            Some(plain) if !plain.is_empty() => {
                let token = self.get_or_create_master_key_token(&crypto)?;
                Some(crypto.encrypt_secret(&token, plain)?)
            }
            _ => existing.as_ref().and_then(|entry| entry.passphrase.clone()),
        };

        key.key_file_path = None;
        key.cert_file_path = None;
        apply_ssh_key_status_flags(&mut key);

        let txn = self.db.begin_write()?;
        write_json_in_txn(
            &txn,
            CREDENTIALS_TABLE,
            &entity_key(SSH_KEY_PREFIX, &target_id),
            &key,
        )?;
        txn.commit()?;
        Ok(target_id)
    }

    pub fn delete_ssh_key(&self, key_id: &str) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        txn.open_table(CREDENTIALS_TABLE)?
            .remove(entity_key(SSH_KEY_PREFIX, key_id).as_str())?;
        txn.commit()?;
        Ok(())
    }

    pub fn save_otp_entry(&self, mut entry: OtpEntry) -> Result<String, StorageError> {
        if entry.id.trim().is_empty() {
            entry.id = uuid::Uuid::new_v4().to_string();
        }
        let target_id = entry.id.clone();
        let existing = self.load_otp_entry_by_id(&target_id)?;
        let crypto = self.credential_crypto()?;

        entry.secret = match entry.secret.as_deref().map(str::trim) {
            Some(plain) if !plain.is_empty() => {
                let token = self.get_or_create_master_key_token(&crypto)?;
                Some(crypto.encrypt_secret(&token, plain)?)
            }
            _ => existing.as_ref().and_then(|entry| entry.secret.clone()),
        };
        if entry.otp_type.trim().is_empty() {
            entry.otp_type = "totp".to_string();
        }
        if entry.algorithm.trim().is_empty() {
            entry.algorithm = "SHA1".to_string();
        }
        if entry.digits == 0 {
            entry.digits = 6;
        }
        if entry.period == 0 {
            entry.period = 30;
        }
        entry.has_secret = entry.secret.is_some();

        let txn = self.db.begin_write()?;
        write_json_in_txn(
            &txn,
            OTP_ACCOUNTS_TABLE,
            &entity_key(OTP_PREFIX, &target_id),
            &entry,
        )?;
        txn.commit()?;
        Ok(target_id)
    }

    pub fn delete_otp_entry(&self, otp_id: &str) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        txn.open_table(OTP_ACCOUNTS_TABLE)?
            .remove(entity_key(OTP_PREFIX, otp_id).as_str())?;
        txn.commit()?;
        Ok(())
    }

    pub fn list_passwords(&self) -> Result<Vec<SavedPassword>, StorageError> {
        let mut passwords: Vec<SavedPassword> =
            self.list_json_by_prefix(CREDENTIALS_TABLE, PASSWORD_PREFIX)?;
        for password in &mut passwords {
            password.has_password = password.password.is_some();
            password.password = None;
        }
        passwords.sort_by(|left, right| {
            left.name.cmp(&right.name).then(left.id.cmp(&right.id))
        });
        Ok(passwords)
    }

    pub fn load_password_by_id(&self, password_id: &str) -> Result<Option<SavedPassword>, StorageError> {
        let key = entity_key(PASSWORD_PREFIX, password_id);
        let Some(mut entry) = self.read_json_table::<SavedPassword>(CREDENTIALS_TABLE, &key)? else {
            return Ok(None);
        };
        entry.has_password = entry.password.is_some();
        Ok(Some(entry))
    }

    pub fn load_decrypted_password_by_id(
        &self,
        password_id: &str,
    ) -> Result<Option<DecryptedSavedPassword>, StorageError> {
        let Some(entry) = self.load_password_by_id(password_id)? else {
            return Ok(None);
        };
        let master_key_token = self.load_master_key_token()?;
        let crypto = self.credential_crypto()?;
        Ok(Some(DecryptedSavedPassword {
            id: entry.id,
            name: entry.name,
            password: decrypt_optional_secret(&crypto, master_key_token.as_deref(), &entry.password)?,
        }))
    }

    pub fn save_password(&self, mut entry: SavedPassword) -> Result<String, StorageError> {
        if entry.id.trim().is_empty() {
            entry.id = uuid::Uuid::new_v4().to_string();
        }
        let target_id = entry.id.clone();
        let existing = self.load_password_by_id(&target_id)?;
        let crypto = self.credential_crypto()?;
        entry.password = match entry.password.as_deref().map(str::trim) {
            Some(plain) if !plain.is_empty() => {
                let token = self.get_or_create_master_key_token(&crypto)?;
                Some(crypto.encrypt_secret(&token, plain)?)
            }
            _ => existing.as_ref().and_then(|entry| entry.password.clone()),
        };
        entry.has_password = entry.password.is_some();
        let txn = self.db.begin_write()?;
        write_json_in_txn(
            &txn,
            CREDENTIALS_TABLE,
            &entity_key(PASSWORD_PREFIX, &target_id),
            &entry,
        )?;
        txn.commit()?;
        Ok(target_id)
    }

    pub fn delete_password(&self, password_id: &str) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        txn.open_table(CREDENTIALS_TABLE)?
            .remove(entity_key(PASSWORD_PREFIX, password_id).as_str())?;
        txn.commit()?;
        Ok(())
    }

    pub fn list_credentials(&self) -> Result<Vec<SavedCredential>, StorageError> {
        let mut credentials: Vec<SavedCredential> =
            self.list_json_by_prefix(CREDENTIALS_TABLE, CREDENTIAL_PREFIX)?;
        for credential in &mut credentials {
            credential.has_password = credential.password.is_some();
            credential.password = None;
        }
        credentials.sort_by(|left, right| {
            left.name.cmp(&right.name).then(left.id.cmp(&right.id))
        });
        Ok(credentials)
    }

    pub fn load_credential_by_id(
        &self,
        credential_id: &str,
    ) -> Result<Option<SavedCredential>, StorageError> {
        let key = entity_key(CREDENTIAL_PREFIX, credential_id);
        let Some(mut entry) = self.read_json_table::<SavedCredential>(CREDENTIALS_TABLE, &key)? else {
            return Ok(None);
        };
        entry.has_password = entry.password.is_some();
        Ok(Some(entry))
    }

    pub fn load_decrypted_credential_by_id(
        &self,
        credential_id: &str,
    ) -> Result<Option<DecryptedSavedCredential>, StorageError> {
        let Some(entry) = self.load_credential_by_id(credential_id)? else {
            return Ok(None);
        };
        let master_key_token = self.load_master_key_token()?;
        let crypto = self.credential_crypto()?;
        Ok(Some(DecryptedSavedCredential {
            id: entry.id,
            name: entry.name,
            username: entry.username,
            password: decrypt_optional_secret(&crypto, master_key_token.as_deref(), &entry.password)?,
            username_prompt_regex: entry.username_prompt_regex,
            password_prompt_regex: entry.password_prompt_regex,
            enabled: entry.enabled,
        }))
    }

    pub fn save_credential(&self, mut entry: SavedCredential) -> Result<String, StorageError> {
        if entry.id.trim().is_empty() {
            entry.id = uuid::Uuid::new_v4().to_string();
        }
        let target_id = entry.id.clone();
        let existing = self.load_credential_by_id(&target_id)?;
        let crypto = self.credential_crypto()?;
        entry.password = match entry.password.as_deref().map(str::trim) {
            Some(plain) if !plain.is_empty() => {
                let token = self.get_or_create_master_key_token(&crypto)?;
                Some(crypto.encrypt_secret(&token, plain)?)
            }
            _ => existing.as_ref().and_then(|entry| entry.password.clone()),
        };
        entry.has_password = entry.password.is_some();
        let txn = self.db.begin_write()?;
        write_json_in_txn(
            &txn,
            CREDENTIALS_TABLE,
            &entity_key(CREDENTIAL_PREFIX, &target_id),
            &entry,
        )?;
        txn.commit()?;
        Ok(target_id)
    }

    pub fn delete_credential(&self, credential_id: &str) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        txn.open_table(CREDENTIALS_TABLE)?
            .remove(entity_key(CREDENTIAL_PREFIX, credential_id).as_str())?;
        txn.commit()?;
        Ok(())
    }

    pub fn list_tunnels(&self) -> Result<Vec<TunnelConfig>, StorageError> {
        let mut tunnels: Vec<TunnelConfig> =
            self.list_json_by_prefix(TUNNELS_TABLE, TUNNEL_PREFIX)?;
        tunnels.sort_by(|left, right| {
            left.group_id
                .cmp(&right.group_id)
                .then(left.name.cmp(&right.name))
                .then(left.id.cmp(&right.id))
        });
        Ok(tunnels)
    }

    pub fn list_tunnel_groups(&self) -> Result<Vec<TunnelGroup>, StorageError> {
        let value = self.load_settings_doc_value(SETTINGS_TUNNEL_GROUPS, serde_json::json!({}))?;
        let mut groups: Vec<TunnelGroup> =
            serde_json::from_value::<TunnelGroupsConfig>(value)?.groups;
        groups.sort_by(|left, right| {
            left.sort_order
                .cmp(&right.sort_order)
                .then(left.name.cmp(&right.name))
                .then(left.id.cmp(&right.id))
        });
        Ok(groups)
    }

    pub fn replace_tunnels(&self, tunnels: &[TunnelConfig]) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        clear_prefix_in_txn(&txn, TUNNELS_TABLE, TUNNEL_PREFIX)?;
        for tunnel in tunnels {
            write_json_in_txn(
                &txn,
                TUNNELS_TABLE,
                &entity_key(TUNNEL_PREFIX, &tunnel.id),
                tunnel,
            )?;
        }
        txn.commit()?;
        Ok(())
    }

    pub fn replace_tunnel_groups(&self, groups: &[TunnelGroup]) -> Result<(), StorageError> {
        self.save_settings_doc_value(
            SETTINGS_TUNNEL_GROUPS,
            &serde_json::json!({ "groups": groups }),
        )
    }

    pub fn list_proxies(&self) -> Result<Vec<ProxyConfig>, StorageError> {
        let mut proxies: Vec<ProxyConfig> =
            self.list_json_by_prefix(PROXIES_TABLE, PROXY_PREFIX)?;
        proxies.sort_by(|left, right| {
            left.group_id
                .cmp(&right.group_id)
                .then(left.name.cmp(&right.name))
                .then(left.id.cmp(&right.id))
        });
        Ok(proxies)
    }

    pub fn list_proxy_groups(&self) -> Result<Vec<ProxyGroup>, StorageError> {
        let value = self.load_settings_doc_value(SETTINGS_PROXY_GROUPS, serde_json::json!({}))?;
        let mut groups: Vec<ProxyGroup> =
            serde_json::from_value::<ProxyGroupsConfig>(value)?.groups;
        groups.sort_by(|left, right| {
            left.sort_order
                .cmp(&right.sort_order)
                .then(left.name.cmp(&right.name))
                .then(left.id.cmp(&right.id))
        });
        Ok(groups)
    }

    pub fn replace_proxies(&self, proxies: &[ProxyConfig]) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        clear_prefix_in_txn(&txn, PROXIES_TABLE, PROXY_PREFIX)?;
        for proxy in proxies {
            write_json_in_txn(
                &txn,
                PROXIES_TABLE,
                &entity_key(PROXY_PREFIX, &proxy.id),
                proxy,
            )?;
        }
        txn.commit()?;
        Ok(())
    }

    pub fn replace_proxy_groups(&self, groups: &[ProxyGroup]) -> Result<(), StorageError> {
        self.save_settings_doc_value(
            SETTINGS_PROXY_GROUPS,
            &serde_json::json!({ "groups": groups }),
        )
    }

    pub fn replace_sessions(&self, config: &SessionsConfig) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        replace_sessions_in_txn(&txn, config)?;
        txn.commit()?;
        Ok(())
    }

    pub fn list_groups(&self) -> Result<Vec<Group>, StorageError> {
        let mut groups = self.list_json_by_prefix(GROUPS_TABLE, GROUP_PREFIX)?;
        groups.sort_by(|left: &Group, right| {
            left.sort_order
                .cmp(&right.sort_order)
                .then(left.name.cmp(&right.name))
                .then(left.id.cmp(&right.id))
        });
        Ok(groups)
    }

    pub fn save_group(&self, group: &Group) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        save_group_in_txn(&txn, group)?;
        txn.commit()?;
        Ok(())
    }

    pub fn delete_group(&self, group_id: &str) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        txn.open_table(GROUPS_TABLE)?
            .remove(entity_key(GROUP_PREFIX, group_id).as_str())?;
        txn.commit()?;
        Ok(())
    }

    pub fn list_connections(&self) -> Result<Vec<SavedConnection>, StorageError> {
        let mut connections = self.list_json_by_prefix(CONNECTIONS_TABLE, CONNECTION_PREFIX)?;
        sort_connections(&mut connections);
        Ok(connections)
    }

    pub fn get_connection(
        &self,
        connection_id: &str,
    ) -> Result<Option<SavedConnection>, StorageError> {
        let txn = self.db.begin_read()?;
        let table = match txn.open_table(CONNECTIONS_TABLE) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let key = entity_key(CONNECTION_PREFIX, connection_id);
        let Some(raw) = table.get(key.as_str())? else {
            return Ok(None);
        };
        let mut connection: SavedConnection = deserialize_json(raw.value())?;
        drop(table);
        drop(txn);
        self.hydrate_connection_passwords(std::slice::from_mut(&mut connection))?;
        Ok(Some(connection))
    }

    pub fn save_connection(&self, connection: &SavedConnection) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        save_connection_in_txn(&txn, connection)?;
        txn.commit()?;
        Ok(())
    }

    pub fn delete_connection(&self, connection_id: &str) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        delete_connection_in_txn(&txn, connection_id)?;
        txn.commit()?;
        Ok(())
    }

    pub fn mark_connection_used(&self, connection_id: &str) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        let key = entity_key(CONNECTION_PREFIX, connection_id);
        let mut connection = {
            let table = txn.open_table(CONNECTIONS_TABLE)?;
            let Some(raw) = table.get(key.as_str())? else {
                return Ok(());
            };
            deserialize_json::<SavedConnection>(raw.value())?
        };
        connection.last_used_at_ms = Some(current_time_ms());
        connection.updated_at_ms = Some(current_time_ms());
        write_json_in_txn(&txn, CONNECTIONS_TABLE, &key, &connection)?;
        remove_connection_index_entries(&txn, connection_id)?;
        insert_connection_indexes(&txn, &connection)?;
        txn.commit()?;
        Ok(())
    }

    pub fn check_known_host(
        &self,
        host_identifier: &str,
        key_type: &str,
        key_base64: &str,
    ) -> Result<KnownHostCheck, StorageError> {
        let mut host_seen = false;
        let records = self.list_raw_by_prefix(KNOWN_HOSTS_TABLE, KNOWN_HOST_PREFIX)?;
        for (key, value) in records {
            if key.starts_with(KNOWN_HOST_RAW_PREFIX) {
                continue;
            }
            let record: KnownHostRecord = deserialize_json(&value)?;
            if known_host_record_matches(&record, host_identifier) {
                host_seen = true;
                if record.key_type == key_type && record.key_base64 == key_base64 {
                    return Ok(KnownHostCheck::Match);
                }
            }
        }
        if host_seen {
            Ok(KnownHostCheck::HostSeen)
        } else {
            Ok(KnownHostCheck::UnknownHost)
        }
    }

    pub fn upsert_known_host(&self, line: &str) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        save_known_hosts_line_in_txn(&txn, line)?;
        txn.commit()?;
        Ok(())
    }

    pub fn replace_known_host_for_host(
        &self,
        host_identifier: &str,
        line: &str,
    ) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        remove_known_hosts_for_host_in_txn(&txn, host_identifier)?;
        save_known_hosts_line_in_txn(&txn, line)?;
        txn.commit()?;
        Ok(())
    }

    pub fn render_known_hosts_export(&self) -> Result<String, StorageError> {
        let mut records = self.list_raw_by_prefix(KNOWN_HOSTS_TABLE, KNOWN_HOST_PREFIX)?;
        records.sort_by(|left, right| left.0.cmp(&right.0));
        let mut lines = Vec::new();
        for (key, value) in records {
            if key.starts_with(KNOWN_HOST_RAW_PREFIX) {
                let raw: KnownHostRawRecord = deserialize_json(&value)?;
                lines.push(raw.line);
            } else {
                let host: KnownHostRecord = deserialize_json(&value)?;
                lines.push(
                    host.raw_line
                        .clone()
                        .unwrap_or_else(|| render_known_host_record(&host)),
                );
            }
        }
        if lines.is_empty() {
            Ok(String::new())
        } else {
            Ok(format!("{}\n", lines.join("\n")))
        }
    }

    pub fn replace_known_hosts_export(&self, content: &str) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        replace_known_hosts_text_in_txn(&txn, content)?;
        txn.commit()?;
        Ok(())
    }

    pub fn load_app_settings_summary(&self) -> Result<AppSettingsSummary, StorageError> {
        let value = self.load_settings_value()?;
        Ok(AppSettingsSummary {
            theme: json_string(&value, &["appearance", "theme"], "github-dark"),
            background_image_path: json_path(&value, &["appearance", "background_image_path"])
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string()),
            background_image_fit: json_string(
                &value,
                &["appearance", "background_image_fit"],
                "cover",
            ),
            background_image_opacity: {
                let raw = json_path(&value, &["appearance", "background_image_opacity"]);
                let pct = raw
                    .and_then(|v| v.as_f64())
                    .map(|v| {
                        // Accept both 0..1 float (Tauri) and 0..100 percent.
                        if v <= 1.0 {
                            (v * 100.0).round() as u8
                        } else {
                            v.round() as u8
                        }
                    })
                    .unwrap_or(45)
                    .clamp(5, 100);
                pct
            },
            background_content_opacity: {
                let raw = json_path(&value, &["appearance", "background_opacity"]);
                let pct = raw
                    .and_then(|v| v.as_f64())
                    .map(|v| {
                        if v <= 1.0 {
                            (v * 100.0).round() as u8
                        } else {
                            v.round() as u8
                        }
                    })
                    .unwrap_or(82)
                    .clamp(20, 100);
                pct
            },
            language: json_string(&value, &["translation", "target_language"], "zh-CN"),
            terminal_font_family: json_string(
                &value,
                &["appearance", "font_family"],
                "JetBrains Mono",
            ),
            terminal_font_size: json_u16(&value, &["appearance", "font_size"], 16),
            cursor_style: {
                let raw = json_string(&value, &["appearance", "cursor_style"], "block");
                match raw.as_str() {
                    "underline" | "bar" | "block" => raw,
                    _ => "block".to_string(),
                }
            },
            cursor_blink: json_bool(&value, &["appearance", "cursor_blink"], true),
            x11_display: json_string(&value, &["terminal", "x11_display"], ""),
            terminal_scrollback_lines: json_u32(&value, &["terminal", "scrollback_lines"], 5000)
                .clamp(100, 100_000),
            terminal_keep_alive_interval: json_u32(
                &value,
                &["terminal", "keep_alive_interval"],
                30,
            )
            .min(600),
            terminal_hardware_acceleration: json_bool(
                &value,
                &["terminal", "hardware_acceleration"],
                true,
            ),
            terminal_show_workspace_padding: json_bool(
                &value,
                &["terminal", "show_workspace_padding"],
                false,
            ),
            terminal_show_line_numbers: json_bool(
                &value,
                &["terminal", "show_line_numbers"],
                false,
            ),
            terminal_show_timestamps: json_bool(&value, &["terminal", "show_timestamps"], false),
            terminal_show_timestamp_milliseconds: json_bool(
                &value,
                &["terminal", "show_timestamp_milliseconds"],
                false,
            ),
            terminal_show_multi_line_paste_dialog: json_bool(
                &value,
                &["terminal", "show_multi_line_paste_dialog"],
                true,
            ),
            terminal_paste_image_as_path: json_bool(
                &value,
                &["terminal", "paste_image_as_path"],
                true,
            ),
            terminal_action_links_enabled: json_bool(
                &value,
                &["terminal", "action_links_enabled"],
                false,
            ),
            terminal_action_links_matchers: load_action_links_matchers(&value),
            search_custom_engines: load_search_engines(&value),
            ui_show_remote_stats: json_bool(&value, &["ui", "show_remote_stats"], true),
            ui_remote_stats_interval: json_u32(&value, &["ui", "remote_stats_interval"], 3)
                .clamp(1, 60),
            ui_show_process_manager: json_bool(&value, &["ui", "show_process_manager"], true),
            ui_process_manager_interval: json_u32(&value, &["ui", "process_manager_interval"], 5)
                .clamp(3, 120),
            ui_show_docker_manager: json_bool(&value, &["ui", "show_docker_manager"], true),
            ui_docker_manager_interval: json_u32(&value, &["ui", "docker_manager_interval"], 10)
                .clamp(3, 120),
            ui_quick_cmd_view_mode: normalize_quick_cmd_view_mode(&json_string(
                &value,
                &["ui", "quick_cmd_view_mode"],
                "tile",
            )),
            ui_quick_cmd_sort_mode: normalize_quick_cmd_sort_mode(&json_string(
                &value,
                &["ui", "quick_cmd_sort_mode"],
                "created",
            )),
            ui_file_explorer_auto_sync_cwd_connection_ids: json_string_vec(
                &value,
                &["ui", "file_explorer_auto_sync_cwd_connection_ids"],
                256,
            ),
            ui_file_explorer_favorite_dirs_by_connection_id: json_string_vec_map(
                &value,
                &["ui", "file_explorer_favorite_dirs_by_connection_id"],
                12,
            ),
            ui_left_panel_width: json_u32(&value, &["ui", "left_width"], 256).clamp(160, 720),
            ui_right_panel_width: json_u32(&value, &["ui", "right_width"], 288).clamp(200, 720),
            ui_transfer_height: json_u32(&value, &["ui", "transfer_height"], 180).clamp(60, 600),
            ui_active_left_panel: json_optional_string(&value, &["ui", "active_left_panel"]),
            ui_active_right_panel: json_optional_string(&value, &["ui", "active_right_panel"]),
            ui_left_panel_collapsed: json_bool(&value, &["ui", "left_panel_collapsed"], false),
            ui_right_panel_collapsed: json_bool(&value, &["ui", "right_panel_collapsed"], false),
            ui_activity_bar_left_top: {
                let values = json_string_vec(
                    &value,
                    &["ui", "activity_bar_layout", "left_top"],
                    32,
                );
                if values.is_empty() {
                    default_activity_left_top()
                } else {
                    values
                }
            },
            ui_activity_bar_left_bottom: {
                let values = json_string_vec(
                    &value,
                    &["ui", "activity_bar_layout", "left_bottom"],
                    32,
                );
                if values.is_empty() {
                    default_activity_left_bottom()
                } else {
                    values
                }
            },
            ui_activity_bar_right_top: {
                let values = json_string_vec(
                    &value,
                    &["ui", "activity_bar_layout", "right_top"],
                    32,
                );
                if values.is_empty() {
                    default_activity_right_top()
                } else {
                    values
                }
            },
            ui_activity_bar_right_bottom: {
                let values = json_string_vec(
                    &value,
                    &["ui", "activity_bar_layout", "right_bottom"],
                    32,
                );
                if values.is_empty() {
                    default_activity_right_bottom()
                } else {
                    values
                }
            },
            ui_activity_bar_show_labels: json_bool(
                &value,
                &["ui", "activity_bar_layout", "show_labels"],
                false,
            ),
            ui_panel_multi_open: json_bool(&value, &["appearance", "panel_multi_open"], false)
                || json_bool(&value, &["ui", "panel_multi_open"], false),
            ui_left_open_panels: json_string_vec(&value, &["ui", "left_open_panels"], 32),
            ui_right_open_panels: json_string_vec(&value, &["ui", "right_open_panels"], 32),
            ui_panel_stack_sizes: json_u32_map(&value, &["ui", "panel_stack_sizes"]),
            interaction_copy_on_select: json_bool(
                &value,
                &["interaction", "copy_on_select"],
                false,
            ),
            interaction_right_click_paste: json_bool(
                &value,
                &["interaction", "right_click_paste"],
                false,
            ),
            interaction_command_suggestions_enabled: json_bool(
                &value,
                &["interaction", "command_suggestions_enabled"],
                true,
            ),
            interaction_command_suggestion_min_chars: json_u32(
                &value,
                &["interaction", "command_suggestion_min_chars"],
                2,
            )
            .clamp(1, 500),
            interaction_command_suggestion_max_chars: json_u32(
                &value,
                &["interaction", "command_suggestion_max_chars"],
                64,
            )
            .clamp(1, 500),
            interaction_word_separators: json_string(
                &value,
                &["interaction", "word_separators"],
                " \t\r\n\"'`~!@#$%^&*()-=+[{]}\\|;:,<.>/?",
            ),
            interaction_duplicate_session_command_delay_ms: json_u32(
                &value,
                &["interaction", "duplicate_session_command_delay_ms"],
                1000,
            )
            .min(60_000),
            interaction_alt_as_meta: json_bool(&value, &["interaction", "alt_as_meta"], false),
            interaction_mac_ime_compatibility: json_bool(
                &value,
                &["interaction", "mac_ime_compatibility"],
                true,
            ),
            interaction_tab_double_click_action: normalize_tab_mouse_action(&json_string(
                &value,
                &["interaction", "tab_double_click_action"],
                "disconnect_session",
            )),
            interaction_tab_middle_click_action: normalize_tab_mouse_action(&json_string(
                &value,
                &["interaction", "tab_middle_click_action"],
                "rename_tab",
            )),
            interaction_tab_right_click_action: normalize_tab_mouse_action(&json_string(
                &value,
                &["interaction", "tab_right_click_action"],
                "none",
            )),
            interaction_default_encoding: normalize_interaction_encoding(&json_string(
                &value,
                &["interaction", "default_encoding"],
                "UTF-8",
            )),
            host_key_policy: normalize_host_key_policy(&json_string(
                &value,
                &["security", "host_key_policy"],
                "prompt",
            )),
            transfer_download_path: json_string(&value, &["transfer", "download_path"], ""),
            transfer_ask_save_location: json_bool(
                &value,
                &["transfer", "ask_save_location"],
                false,
            ),
            transfer_duplicate_strategy: normalize_transfer_duplicate_strategy(&json_string(
                &value,
                &["transfer", "duplicate_strategy"],
                "ask",
            )),
            transfer_editor_type: normalize_transfer_editor_type(&json_string(
                &value,
                &["transfer", "editor_type"],
                "external",
            )),
            transfer_default_editor: json_string(&value, &["transfer", "default_editor"], ""),
            transfer_download_threads: json_u32(&value, &["transfer", "download_threads"], 3)
                .clamp(1, 10),
            transfer_upload_threads: json_u32(&value, &["transfer", "upload_threads"], 3)
                .clamp(1, 10),
            transfer_max_retries: json_u32(&value, &["transfer", "max_transfer_retries"], 2)
                .min(10),
            transfer_buffer_size: json_u32(&value, &["transfer", "transfer_buffer_size"], 32)
                .clamp(8, 256),
            transfer_default_file_permissions: normalize_transfer_file_permissions(&json_string(
                &value,
                &["transfer", "default_file_permissions"],
                "644",
            )),
            transfer_preserve_timestamps: json_bool(
                &value,
                &["transfer", "preserve_timestamps"],
                true,
            ),
            transfer_resume_broken_transfer: json_bool(
                &value,
                &["transfer", "resume_broken_transfer"],
                true,
            ),
            recording_path: json_string(&value, &["transfer", "recording_path"], ""),
            recording_auto_start: json_bool(&value, &["transfer", "recording_auto_start"], false),
            recording_include_io_labels: json_bool(
                &value,
                &["transfer", "recording_include_io_labels"],
                true,
            ),
            recording_include_timestamps: json_bool(
                &value,
                &["transfer", "recording_include_timestamps"],
                true,
            ),
            recording_memory_limit_bytes: json_u64(
                &value,
                &["transfer", "recording_memory_limit_bytes"],
                5 * 1024 * 1024,
            ),
            diagnostics_level: json_string(&value, &["diagnostics", "level"], "info"),
            diagnostics_retention_days: u32::from(json_u16(
                &value,
                &["diagnostics", "retention_days"],
                7,
            )),
            startup_restore: json_bool(&value, &["general", "startup_restore"], false),
            confirm_on_close: json_bool(&value, &["general", "confirm_on_close"], true),
            enable_screen_lock: json_bool(&value, &["security", "enable_screen_lock"], false),
            idle_lock_minutes: u32::from(json_u16(&value, &["security", "idle_lock_minutes"], 0)),
            has_master_password: value
                .get("security")
                .and_then(|security| security.get("master_password"))
                .and_then(|master_password| master_password.as_str())
                .is_some_and(|master_password| !master_password.is_empty()),
            keybindings: json_string_map(&value, &["keybindings"]),
        })
    }

    pub fn verify_master_password(&self, password: &str) -> Result<bool, StorageError> {
        let Some(token) = self.load_encrypted_master_password()? else {
            return Ok(true);
        };
        let bootstrap = CredentialCrypto::new(self.portable_key_path.clone(), None);
        let stored = bootstrap.decrypt_settings_secret(&token)?;
        Ok(stored == password)
    }

    pub fn save_host_key_policy(&self, policy: &str) -> Result<AppSettingsSummary, StorageError> {
        let policy = normalize_host_key_policy(policy);
        let mut value = self.load_settings_value()?;
        set_nested_json_string(&mut value, &["security", "host_key_policy"], policy);
        self.save_settings_value(&value)?;
        self.load_app_settings_summary()
    }

    pub fn save_recording_settings(
        &self,
        settings: &AppSettingsSummary,
    ) -> Result<AppSettingsSummary, StorageError> {
        let mut value = self.load_settings_value()?;
        set_nested_json_string(
            &mut value,
            &["transfer", "recording_path"],
            settings.recording_path.clone(),
        );
        set_nested_json_value(
            &mut value,
            &["transfer", "recording_auto_start"],
            serde_json::Value::Bool(settings.recording_auto_start),
        );
        set_nested_json_value(
            &mut value,
            &["transfer", "recording_include_io_labels"],
            serde_json::Value::Bool(settings.recording_include_io_labels),
        );
        set_nested_json_value(
            &mut value,
            &["transfer", "recording_include_timestamps"],
            serde_json::Value::Bool(settings.recording_include_timestamps),
        );
        set_nested_json_value(
            &mut value,
            &["transfer", "recording_memory_limit_bytes"],
            serde_json::Value::from(settings.recording_memory_limit_bytes),
        );
        self.save_settings_value(&value)?;
        self.load_app_settings_summary()
    }

    pub fn save_transfer_settings(
        &self,
        settings: &AppSettingsSummary,
    ) -> Result<AppSettingsSummary, StorageError> {
        let mut value = self.load_settings_value()?;
        set_nested_json_string(
            &mut value,
            &["transfer", "download_path"],
            settings.transfer_download_path.clone(),
        );
        set_nested_json_value(
            &mut value,
            &["transfer", "ask_save_location"],
            serde_json::Value::Bool(settings.transfer_ask_save_location),
        );
        set_nested_json_string(
            &mut value,
            &["transfer", "duplicate_strategy"],
            normalize_transfer_duplicate_strategy(&settings.transfer_duplicate_strategy),
        );
        set_nested_json_string(
            &mut value,
            &["transfer", "editor_type"],
            normalize_transfer_editor_type(&settings.transfer_editor_type),
        );
        set_nested_json_string(
            &mut value,
            &["transfer", "default_editor"],
            settings.transfer_default_editor.clone(),
        );
        set_nested_json_value(
            &mut value,
            &["transfer", "download_threads"],
            serde_json::Value::from(settings.transfer_download_threads.clamp(1, 10)),
        );
        set_nested_json_value(
            &mut value,
            &["transfer", "upload_threads"],
            serde_json::Value::from(settings.transfer_upload_threads.clamp(1, 10)),
        );
        set_nested_json_value(
            &mut value,
            &["transfer", "max_transfer_retries"],
            serde_json::Value::from(settings.transfer_max_retries.min(10)),
        );
        set_nested_json_value(
            &mut value,
            &["transfer", "transfer_buffer_size"],
            serde_json::Value::from(settings.transfer_buffer_size.clamp(8, 256)),
        );
        set_nested_json_string(
            &mut value,
            &["transfer", "default_file_permissions"],
            normalize_transfer_file_permissions(&settings.transfer_default_file_permissions),
        );
        set_nested_json_value(
            &mut value,
            &["transfer", "preserve_timestamps"],
            serde_json::Value::Bool(settings.transfer_preserve_timestamps),
        );
        set_nested_json_value(
            &mut value,
            &["transfer", "resume_broken_transfer"],
            serde_json::Value::Bool(settings.transfer_resume_broken_transfer),
        );
        self.save_settings_value(&value)?;
        self.load_app_settings_summary()
    }

    pub fn save_file_explorer_favorite_dirs(
        &self,
        settings: &AppSettingsSummary,
    ) -> Result<AppSettingsSummary, StorageError> {
        let mut value = self.load_settings_value()?;
        set_nested_json_value(
            &mut value,
            &["ui", "file_explorer_auto_sync_cwd_connection_ids"],
            string_vec_json_value(&settings.ui_file_explorer_auto_sync_cwd_connection_ids, 256),
        );
        set_nested_json_value(
            &mut value,
            &["ui", "file_explorer_favorite_dirs_by_connection_id"],
            string_vec_map_json_value(
                &settings.ui_file_explorer_favorite_dirs_by_connection_id,
                12,
            ),
        );
        self.save_settings_value(&value)?;
        self.load_app_settings_summary()
    }

    pub fn save_quick_command_ui_settings(
        &self,
        settings: &AppSettingsSummary,
    ) -> Result<AppSettingsSummary, StorageError> {
        let mut value = self.load_settings_value()?;
        set_nested_json_string(
            &mut value,
            &["ui", "quick_cmd_view_mode"],
            normalize_quick_cmd_view_mode(&settings.ui_quick_cmd_view_mode),
        );
        set_nested_json_string(
            &mut value,
            &["ui", "quick_cmd_sort_mode"],
            normalize_quick_cmd_sort_mode(&settings.ui_quick_cmd_sort_mode),
        );
        self.save_settings_value(&value)?;
        self.load_app_settings_summary()
    }

    pub fn save_ui_layout_settings(
        &self,
        settings: &AppSettingsSummary,
    ) -> Result<AppSettingsSummary, StorageError> {
        let mut value = self.load_settings_value()?;
        set_nested_json_value(
            &mut value,
            &["ui", "left_width"],
            serde_json::Value::from(settings.ui_left_panel_width.clamp(160, 720)),
        );
        set_nested_json_value(
            &mut value,
            &["ui", "right_width"],
            serde_json::Value::from(settings.ui_right_panel_width.clamp(200, 720)),
        );
        set_nested_json_value(
            &mut value,
            &["ui", "transfer_height"],
            serde_json::Value::from(settings.ui_transfer_height.clamp(60, 600)),
        );
        match &settings.ui_active_left_panel {
            Some(panel) if !panel.trim().is_empty() => set_nested_json_string(
                &mut value,
                &["ui", "active_left_panel"],
                panel.clone(),
            ),
            _ => set_nested_json_value(
                &mut value,
                &["ui", "active_left_panel"],
                serde_json::Value::Null,
            ),
        }
        match &settings.ui_active_right_panel {
            Some(panel) if !panel.trim().is_empty() => set_nested_json_string(
                &mut value,
                &["ui", "active_right_panel"],
                panel.clone(),
            ),
            _ => set_nested_json_value(
                &mut value,
                &["ui", "active_right_panel"],
                serde_json::Value::Null,
            ),
        }
        set_nested_json_value(
            &mut value,
            &["ui", "left_panel_collapsed"],
            serde_json::Value::Bool(settings.ui_left_panel_collapsed),
        );
        set_nested_json_value(
            &mut value,
            &["ui", "right_panel_collapsed"],
            serde_json::Value::Bool(settings.ui_right_panel_collapsed),
        );
        set_nested_json_value(
            &mut value,
            &["ui", "activity_bar_layout", "left_top"],
            string_vec_json_value(&settings.ui_activity_bar_left_top, 32),
        );
        set_nested_json_value(
            &mut value,
            &["ui", "activity_bar_layout", "left_bottom"],
            string_vec_json_value(&settings.ui_activity_bar_left_bottom, 32),
        );
        set_nested_json_value(
            &mut value,
            &["ui", "activity_bar_layout", "right_top"],
            string_vec_json_value(&settings.ui_activity_bar_right_top, 32),
        );
        set_nested_json_value(
            &mut value,
            &["ui", "activity_bar_layout", "right_bottom"],
            string_vec_json_value(&settings.ui_activity_bar_right_bottom, 32),
        );
        set_nested_json_value(
            &mut value,
            &["ui", "activity_bar_layout", "show_labels"],
            serde_json::Value::Bool(settings.ui_activity_bar_show_labels),
        );
        set_nested_json_value(
            &mut value,
            &["ui", "panel_multi_open"],
            serde_json::Value::Bool(settings.ui_panel_multi_open),
        );
        // Keep appearance key for Tauri compatibility.
        set_nested_json_value(
            &mut value,
            &["appearance", "panel_multi_open"],
            serde_json::Value::Bool(settings.ui_panel_multi_open),
        );
        set_nested_json_value(
            &mut value,
            &["ui", "left_open_panels"],
            string_vec_json_value(&settings.ui_left_open_panels, 32),
        );
        set_nested_json_value(
            &mut value,
            &["ui", "right_open_panels"],
            string_vec_json_value(&settings.ui_right_open_panels, 32),
        );
        set_nested_json_value(
            &mut value,
            &["ui", "panel_stack_sizes"],
            u32_map_json_value(&settings.ui_panel_stack_sizes),
        );
        self.save_settings_value(&value)?;
        self.load_app_settings_summary()
    }

    pub fn save_appearance_settings(
        &self,
        settings: &AppSettingsSummary,
    ) -> Result<AppSettingsSummary, StorageError> {
        let mut value = self.load_settings_value()?;
        set_nested_json_string(&mut value, &["appearance", "theme"], settings.theme.clone());
        match settings.background_image_path.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(path) => set_nested_json_string(
                &mut value,
                &["appearance", "background_image_path"],
                path.to_string(),
            ),
            None => set_nested_json_value(
                &mut value,
                &["appearance", "background_image_path"],
                serde_json::Value::Null,
            ),
        }
        set_nested_json_string(
            &mut value,
            &["appearance", "background_image_fit"],
            settings.background_image_fit.clone(),
        );
        set_nested_json_value(
            &mut value,
            &["appearance", "background_image_opacity"],
            serde_json::Value::from(settings.background_image_opacity as f64 / 100.0),
        );
        set_nested_json_value(
            &mut value,
            &["appearance", "background_opacity"],
            serde_json::Value::from(settings.background_content_opacity as f64 / 100.0),
        );
        set_nested_json_string(
            &mut value,
            &["appearance", "font_family"],
            settings.terminal_font_family.clone(),
        );
        set_nested_json_value(
            &mut value,
            &["appearance", "font_size"],
            serde_json::Value::from(settings.terminal_font_size),
        );
        set_nested_json_string(
            &mut value,
            &["appearance", "cursor_style"],
            match settings.cursor_style.as_str() {
                "underline" | "bar" => settings.cursor_style.clone(),
                _ => "block".to_string(),
            },
        );
        set_nested_json_value(
            &mut value,
            &["appearance", "cursor_blink"],
            serde_json::Value::from(settings.cursor_blink),
        );
        set_nested_json_string(
            &mut value,
            &["terminal", "x11_display"],
            settings.x11_display.clone(),
        );
        self.save_settings_value(&value)?;
        self.load_app_settings_summary()
    }

    pub fn save_terminal_settings(
        &self,
        settings: &AppSettingsSummary,
    ) -> Result<AppSettingsSummary, StorageError> {
        let mut value = self.load_settings_value()?;
        set_nested_json_string(
            &mut value,
            &["terminal", "x11_display"],
            settings.x11_display.clone(),
        );
        set_nested_json_value(
            &mut value,
            &["terminal", "scrollback_lines"],
            serde_json::Value::from(settings.terminal_scrollback_lines.clamp(100, 100_000)),
        );
        set_nested_json_value(
            &mut value,
            &["terminal", "keep_alive_interval"],
            serde_json::Value::from(settings.terminal_keep_alive_interval.min(600)),
        );
        set_nested_json_value(
            &mut value,
            &["terminal", "hardware_acceleration"],
            serde_json::Value::Bool(settings.terminal_hardware_acceleration),
        );
        set_nested_json_value(
            &mut value,
            &["terminal", "show_workspace_padding"],
            serde_json::Value::Bool(settings.terminal_show_workspace_padding),
        );
        set_nested_json_value(
            &mut value,
            &["terminal", "show_line_numbers"],
            serde_json::Value::Bool(settings.terminal_show_line_numbers),
        );
        set_nested_json_value(
            &mut value,
            &["terminal", "show_timestamps"],
            serde_json::Value::Bool(settings.terminal_show_timestamps),
        );
        set_nested_json_value(
            &mut value,
            &["terminal", "show_timestamp_milliseconds"],
            serde_json::Value::Bool(settings.terminal_show_timestamp_milliseconds),
        );
        set_nested_json_value(
            &mut value,
            &["terminal", "show_multi_line_paste_dialog"],
            serde_json::Value::Bool(settings.terminal_show_multi_line_paste_dialog),
        );
        set_nested_json_value(
            &mut value,
            &["terminal", "paste_image_as_path"],
            serde_json::Value::Bool(settings.terminal_paste_image_as_path),
        );
        set_nested_json_value(
            &mut value,
            &["terminal", "action_links_enabled"],
            serde_json::Value::Bool(settings.terminal_action_links_enabled),
        );
        set_nested_json_value(
            &mut value,
            &["terminal", "action_links_matchers"],
            serde_json::json!({
                "ipv4": settings.terminal_action_links_matchers.ipv4,
                "archive": settings.terminal_action_links_matchers.archive,
                "host_port": settings.terminal_action_links_matchers.host_port,
            }),
        );
        set_nested_json_value(
            &mut value,
            &["search", "custom_engines"],
            search_engines_to_json(&settings.search_custom_engines),
        );
        set_nested_json_value(
            &mut value,
            &["ui", "show_remote_stats"],
            serde_json::Value::Bool(settings.ui_show_remote_stats),
        );
        set_nested_json_value(
            &mut value,
            &["ui", "remote_stats_interval"],
            serde_json::Value::from(settings.ui_remote_stats_interval.clamp(1, 60)),
        );
        set_nested_json_value(
            &mut value,
            &["ui", "show_process_manager"],
            serde_json::Value::Bool(settings.ui_show_process_manager),
        );
        set_nested_json_value(
            &mut value,
            &["ui", "process_manager_interval"],
            serde_json::Value::from(settings.ui_process_manager_interval.clamp(3, 120)),
        );
        set_nested_json_value(
            &mut value,
            &["ui", "show_docker_manager"],
            serde_json::Value::Bool(settings.ui_show_docker_manager),
        );
        set_nested_json_value(
            &mut value,
            &["ui", "docker_manager_interval"],
            serde_json::Value::from(settings.ui_docker_manager_interval.clamp(3, 120)),
        );
        self.save_settings_value(&value)?;
        self.load_app_settings_summary()
    }

    pub fn save_interaction_settings(
        &self,
        settings: &AppSettingsSummary,
    ) -> Result<AppSettingsSummary, StorageError> {
        let mut value = self.load_settings_value()?;
        let min_chars = settings
            .interaction_command_suggestion_min_chars
            .clamp(1, 500);
        let max_chars = settings
            .interaction_command_suggestion_max_chars
            .clamp(min_chars, 500);
        set_nested_json_value(
            &mut value,
            &["interaction", "copy_on_select"],
            serde_json::Value::Bool(settings.interaction_copy_on_select),
        );
        set_nested_json_value(
            &mut value,
            &["interaction", "right_click_paste"],
            serde_json::Value::Bool(settings.interaction_right_click_paste),
        );
        set_nested_json_value(
            &mut value,
            &["interaction", "command_suggestions_enabled"],
            serde_json::Value::Bool(settings.interaction_command_suggestions_enabled),
        );
        set_nested_json_value(
            &mut value,
            &["interaction", "command_suggestion_min_chars"],
            serde_json::Value::from(min_chars),
        );
        set_nested_json_value(
            &mut value,
            &["interaction", "command_suggestion_max_chars"],
            serde_json::Value::from(max_chars),
        );
        set_nested_json_string(
            &mut value,
            &["interaction", "word_separators"],
            settings.interaction_word_separators.clone(),
        );
        set_nested_json_value(
            &mut value,
            &["interaction", "duplicate_session_command_delay_ms"],
            serde_json::Value::from(
                settings
                    .interaction_duplicate_session_command_delay_ms
                    .min(60_000),
            ),
        );
        set_nested_json_value(
            &mut value,
            &["interaction", "alt_as_meta"],
            serde_json::Value::Bool(settings.interaction_alt_as_meta),
        );
        set_nested_json_value(
            &mut value,
            &["interaction", "mac_ime_compatibility"],
            serde_json::Value::Bool(settings.interaction_mac_ime_compatibility),
        );
        set_nested_json_string(
            &mut value,
            &["interaction", "tab_double_click_action"],
            normalize_tab_mouse_action(&settings.interaction_tab_double_click_action),
        );
        set_nested_json_string(
            &mut value,
            &["interaction", "tab_middle_click_action"],
            normalize_tab_mouse_action(&settings.interaction_tab_middle_click_action),
        );
        set_nested_json_string(
            &mut value,
            &["interaction", "tab_right_click_action"],
            normalize_tab_mouse_action(&settings.interaction_tab_right_click_action),
        );
        set_nested_json_string(
            &mut value,
            &["interaction", "default_encoding"],
            normalize_interaction_encoding(&settings.interaction_default_encoding),
        );
        self.save_settings_value(&value)?;
        self.load_app_settings_summary()
    }

    pub fn save_general_settings(
        &self,
        settings: &AppSettingsSummary,
    ) -> Result<AppSettingsSummary, StorageError> {
        let mut value = self.load_settings_value()?;
        set_nested_json_value(
            &mut value,
            &["general", "startup_restore"],
            serde_json::Value::Bool(settings.startup_restore),
        );
        set_nested_json_value(
            &mut value,
            &["general", "confirm_on_close"],
            serde_json::Value::Bool(settings.confirm_on_close),
        );
        self.save_settings_value(&value)?;
        self.load_app_settings_summary()
    }

    pub fn save_screen_lock_settings(
        &self,
        settings: &AppSettingsSummary,
    ) -> Result<AppSettingsSummary, StorageError> {
        let mut value = self.load_settings_value()?;
        set_nested_json_value(
            &mut value,
            &["security", "enable_screen_lock"],
            serde_json::Value::Bool(settings.enable_screen_lock),
        );
        set_nested_json_value(
            &mut value,
            &["security", "idle_lock_minutes"],
            serde_json::Value::from(settings.idle_lock_minutes),
        );
        self.save_settings_value(&value)?;
        self.load_app_settings_summary()
    }

    pub fn save_keybindings(
        &self,
        keybindings: &HashMap<String, String>,
    ) -> Result<AppSettingsSummary, StorageError> {
        let mut value = self.load_settings_value()?;
        let mut object = serde_json::Map::new();
        for (id, keys) in keybindings {
            let id = id.trim();
            let keys = keys.trim();
            if !id.is_empty() && !keys.is_empty() {
                object.insert(id.to_string(), serde_json::Value::String(keys.to_string()));
            }
        }
        set_nested_json_value(
            &mut value,
            &["keybindings"],
            serde_json::Value::Object(object),
        );
        self.save_settings_value(&value)?;
        self.load_app_settings_summary()
    }

    pub fn load_translation_settings(&self) -> Result<TranslationSettings, StorageError> {
        let value = self.load_settings_value()?;
        let mut settings = value
            .get(SETTINGS_TRANSLATION_FIELD)
            .cloned()
            .map(serde_json::from_value::<TranslationSettings>)
            .transpose()?
            .unwrap_or_default();
        self.decrypt_translation_settings(&mut settings)?;
        if settings.target_language.trim().is_empty() {
            settings.target_language = TranslationSettings::default().target_language;
        }
        Ok(settings)
    }

    pub fn save_translation_settings(
        &self,
        next: TranslationSettings,
    ) -> Result<TranslationSettings, StorageError> {
        let current = self.load_translation_settings()?;
        let mut merged = merge_masked_translation_settings(&current, next);
        if merged.target_language.trim().is_empty() {
            merged.target_language = TranslationSettings::default().target_language;
        }
        let encrypted = self.encrypt_translation_settings(merged.clone())?;
        let mut value = self.load_settings_value()?;
        set_nested_json_value(
            &mut value,
            &[SETTINGS_TRANSLATION_FIELD],
            serde_json::to_value(encrypted)?,
        );
        self.save_settings_value(&value)?;
        Ok(merged)
    }

    pub fn load_keyword_highlights(&self) -> Result<KeywordHighlightConfig, StorageError> {
        let value = self.load_settings_value()?;
        let rules = json_path(&value, &["terminal", "keyword_highlights"])
            .cloned()
            .map(serde_json::from_value::<Vec<KeywordHighlightRule>>)
            .transpose()?
            .unwrap_or_default()
            .into_iter()
            .filter_map(normalize_keyword_highlight_rule)
            .collect();
        Ok(KeywordHighlightConfig {
            enabled: json_bool(&value, &["terminal", "keyword_highlights_enabled"], false),
            across_wrapped_lines: json_bool(
                &value,
                &["terminal", "keyword_highlights_across_wrapped_lines"],
                false,
            ),
            rules,
        })
    }

    pub fn save_keyword_highlights(
        &self,
        config: &KeywordHighlightConfig,
    ) -> Result<KeywordHighlightConfig, StorageError> {
        let mut value = self.load_settings_value()?;
        set_nested_json_value(
            &mut value,
            &["terminal", "keyword_highlights_enabled"],
            serde_json::Value::Bool(config.enabled),
        );
        set_nested_json_value(
            &mut value,
            &["terminal", "keyword_highlights_across_wrapped_lines"],
            serde_json::Value::Bool(config.across_wrapped_lines),
        );
        let rules = config
            .rules
            .iter()
            .cloned()
            .filter_map(normalize_keyword_highlight_rule)
            .collect::<Vec<_>>();
        set_nested_json_value(
            &mut value,
            &["terminal", "keyword_highlights"],
            serde_json::to_value(rules)?,
        );
        self.save_settings_value(&value)?;
        self.load_keyword_highlights()
    }

    pub fn import_keyword_highlights_json(
        &self,
        raw: &str,
    ) -> Result<(KeywordHighlightConfig, KeywordHighlightImportResult), StorageError> {
        let import_file: KeywordHighlightImportFile = serde_json::from_str(raw)?;
        let imported = match import_file {
            KeywordHighlightImportFile::Config { keyword_highlights } => keyword_highlights,
            KeywordHighlightImportFile::Rules(rules) => rules,
        };
        let mut config = self.load_keyword_highlights()?;
        let result = merge_keyword_highlight_rules(&mut config.rules, imported);
        if result.imported_rules == 0 && result.updated_rules == 0 {
            return Err(StorageError::InvalidData(
                "No valid highlight rules found in import file".to_string(),
            ));
        }
        let saved = self.save_keyword_highlights(&config)?;
        Ok((saved, result))
    }

    pub fn load_cloud_sync_state(&self) -> Result<CloudSyncState, StorageError> {
        let mut state = if let Some(state) =
            self.read_json_table::<CloudSyncState>(SETTINGS_TABLE, SETTINGS_CLOUD_SYNC_STATE)?
        {
            state
        } else if let Some(raw) =
            self.read_string_table(TEXT_DOCS_TABLE, LEGACY_TEXT_CLOUD_SYNC_STATE)?
        {
            serde_json::from_str(&raw)?
        } else {
            CloudSyncState::default()
        };
        if state.device_id.trim().is_empty() {
            state.device_id = CloudSyncState::default().device_id;
        }
        Ok(state)
    }

    pub fn save_cloud_sync_state(&self, state: &CloudSyncState) -> Result<(), StorageError> {
        let mut state = state.clone();
        if state.device_id.trim().is_empty() {
            state.device_id = CloudSyncState::default().device_id;
        }
        self.save_settings_doc_value(SETTINGS_CLOUD_SYNC_STATE, &serde_json::to_value(state)?)?;
        Ok(())
    }

    pub fn load_cloud_sync_settings(&self) -> Result<CloudSyncSettings, StorageError> {
        let mut settings = self
            .read_json_table::<CloudSyncSettings>(SETTINGS_TABLE, SETTINGS_CLOUD_SYNC)?
            .unwrap_or_default();
        self.decrypt_cloud_sync_settings(&mut settings)?;
        Ok(settings)
    }

    pub fn save_cloud_sync_settings(
        &self,
        next: CloudSyncSettings,
    ) -> Result<CloudSyncSettings, StorageError> {
        let current = self.load_cloud_sync_settings()?;
        let merged = merge_masked_cloud_sync_settings(&current, next);
        let encrypted = self.encrypt_cloud_sync_settings(merged.clone())?;
        self.save_settings_doc_value(SETTINGS_CLOUD_SYNC, &serde_json::to_value(encrypted)?)?;
        Ok(merged)
    }

    pub fn load_quick_commands(&self) -> Result<QuickCommandsConfig, StorageError> {
        self.read_json_table::<QuickCommandsConfig>(SETTINGS_TABLE, SETTINGS_QUICK_COMMANDS)
            .map(|config| config.unwrap_or_default())
    }

    pub fn save_quick_commands(&self, config: QuickCommandsConfig) -> Result<(), StorageError> {
        self.save_settings_doc_value(SETTINGS_QUICK_COMMANDS, &serde_json::to_value(config)?)?;
        Ok(())
    }

    pub fn upsert_quick_command(
        &self,
        mut command: QuickCommand,
        new_category: Option<QuickCommandCategory>,
    ) -> Result<QuickCommandsConfig, StorageError> {
        let mut config = self.load_quick_commands()?;
        let now = current_time_ms();

        if let Some(category) = new_category
            && !config.categories.iter().any(|item| item.id == category.id)
        {
            config.categories.push(category);
        }

        command.updated_at = Some(now);
        if let Some(existing) = config
            .commands
            .iter_mut()
            .find(|item| item.id == command.id)
        {
            let original_created_at = existing.created_at;
            let original_use_count = existing.use_count;
            *existing = command;
            if original_created_at.is_some() {
                existing.created_at = original_created_at;
            }
            if original_use_count.is_some() {
                existing.use_count = original_use_count;
            }
        } else {
            command.created_at = command.created_at.or(Some(now));
            config.commands.push(command);
        }

        self.save_quick_commands(config.clone())?;
        Ok(config)
    }

    pub fn increment_quick_command_use_count(&self, id: &str) -> Result<(), StorageError> {
        let mut config = self.load_quick_commands()?;
        if let Some(command) = config.commands.iter_mut().find(|command| command.id == id) {
            command.use_count = Some(command.use_count.unwrap_or_default().saturating_add(1));
            command.updated_at = Some(current_time_ms());
            self.save_quick_commands(config)?;
        }
        Ok(())
    }

    pub fn append_command_history(&self, command: &str) -> Result<(), StorageError> {
        let Some(command) = sanitize_history_command(command) else {
            return Ok(());
        };
        let mut entry = self
            .list_command_history(usize::MAX)?
            .into_iter()
            .find(|entry| entry.command == command)
            .unwrap_or(CommandHistoryEntry {
                command,
                last_used_at_ms: 0,
                use_count: 0,
            });
        entry.last_used_at_ms = current_time_ms();
        entry.use_count = if entry.use_count == 0 {
            1
        } else {
            entry.use_count.saturating_add(1)
        };
        self.save_command_history_entry(&entry)
    }

    pub fn list_command_history(
        &self,
        limit: usize,
    ) -> Result<Vec<CommandHistoryEntry>, StorageError> {
        let mut entries: Vec<(String, CommandHistoryEntry)> =
            self.list_keyed_json_by_prefix(COMMAND_HISTORY_TABLE, COMMAND_HISTORY_PREFIX)?;
        entries.sort_by(|left, right| right.0.cmp(&left.0));
        let mut history = Vec::new();
        for (_, entry) in entries.into_iter().take(limit) {
            if let Some(command) = sanitize_history_command(&entry.command) {
                history.push(CommandHistoryEntry {
                    command,
                    last_used_at_ms: entry.last_used_at_ms,
                    use_count: entry.use_count.max(1),
                });
            }
        }
        Ok(history)
    }

    pub fn delete_command_history(&self, command: &str) -> Result<(), StorageError> {
        let Some(command) = sanitize_history_command(command) else {
            return Ok(());
        };
        let txn = self.db.begin_write()?;
        remove_command_history_id_in_txn(&txn, &command_history_id(&command))?;
        txn.commit()?;
        Ok(())
    }

    pub fn replace_command_history(
        &self,
        entries: &[CommandHistoryEntry],
    ) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        replace_command_history_in_txn(&txn, entries)?;
        txn.commit()?;
        Ok(())
    }

    fn save_command_history_entry(&self, entry: &CommandHistoryEntry) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        save_command_history_entry_in_txn(&txn, entry)?;
        txn.commit()?;
        Ok(())
    }

    pub fn load_ai_settings(&self) -> Result<AiSettings, StorageError> {
        let value = self.load_settings_value()?;
        let mut settings = value
            .get(SETTINGS_AI_FIELD)
            .cloned()
            .map(serde_json::from_value::<AiSettings>)
            .transpose()?
            .unwrap_or_default();
        self.decrypt_ai_settings(&mut settings)?;
        normalize_ai_settings(&mut settings);
        Ok(settings)
    }

    pub fn save_ai_settings(&self, next: AiSettings) -> Result<AiSettings, StorageError> {
        let current = self.load_ai_settings()?;
        let merged = merge_masked_ai_settings(&current, next);
        let encrypted = self.encrypt_ai_settings(merged.clone())?;
        let mut value = self.load_settings_value()?;
        set_nested_json_value(
            &mut value,
            &[SETTINGS_AI_FIELD],
            serde_json::to_value(encrypted)?,
        );
        self.save_settings_value(&value)?;
        Ok(merged)
    }

    pub fn load_ai_history(&self) -> Result<AiHistoryFile, StorageError> {
        self.read_json_table::<AiHistoryFile>(SETTINGS_TABLE, SETTINGS_AI_HISTORY)
            .map(|history| history.unwrap_or_default())
    }

    pub fn save_ai_history(&self, mut history: AiHistoryFile) -> Result<(), StorageError> {
        trim_ai_history(&mut history);
        self.save_settings_doc_value(SETTINGS_AI_HISTORY, &serde_json::to_value(history)?)?;
        Ok(())
    }

    pub fn append_ai_user_message(
        &self,
        session_id: &str,
        connection_id: Option<String>,
        user_input: String,
    ) -> Result<(), StorageError> {
        let now = now_rfc3339();
        let title = ai_session_title(&user_input);
        let session_id = session_id.to_string();
        let mut history = self.load_ai_history()?;
        if let Some(session) = history
            .sessions
            .iter_mut()
            .find(|item| item.id == session_id)
        {
            session.updated_at = now.clone();
        } else {
            history.sessions.push(AiSession {
                id: session_id.clone(),
                connection_id,
                title,
                created_at: now.clone(),
                updated_at: now.clone(),
            });
        }
        history.messages.push(AiMessage {
            id: format!("msg-{}", uuid()),
            session_id,
            role: AiMessageRole::User,
            content: user_input,
            created_at: now,
            reasoning_content: None,
            command_cards: Vec::new(),
        });
        self.save_ai_history(history)
    }

    pub fn append_ai_message(&self, message: AiMessage) -> Result<(), StorageError> {
        let mut history = self.load_ai_history()?;
        if let Some(session) = history
            .sessions
            .iter_mut()
            .find(|item| item.id == message.session_id)
        {
            session.updated_at = message.created_at.clone();
        }
        history.messages.push(message);
        self.save_ai_history(history)
    }

    pub fn list_ai_sessions(&self) -> Result<Vec<AiSession>, StorageError> {
        let mut sessions = self.load_ai_history()?.sessions;
        sessions.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        Ok(sessions)
    }

    pub fn list_ai_messages(&self, session_id: &str) -> Result<Vec<AiMessage>, StorageError> {
        Ok(self
            .load_ai_history()?
            .messages
            .into_iter()
            .filter(|message| message.session_id == session_id)
            .collect())
    }

    pub fn clear_ai_history(&self) -> Result<(), StorageError> {
        self.save_settings_doc_value(
            SETTINGS_AI_HISTORY,
            &serde_json::to_value(AiHistoryFile::default())?,
        )?;
        Ok(())
    }

    pub fn delete_ai_session(&self, session_id: &str) -> Result<(), StorageError> {
        let mut history = self.load_ai_history()?;
        history.sessions.retain(|session| session.id != session_id);
        history
            .messages
            .retain(|message| message.session_id != session_id);
        self.save_ai_history(history)
    }

    pub fn append_ai_audit(
        &self,
        request: AppendAiAuditRequest,
    ) -> Result<AiAuditLog, StorageError> {
        let log = AiAuditLog {
            id: format!("audit-{}", uuid()),
            connection_id: request.connection_id,
            action: request.action,
            user_input: request.user_input,
            generated_command: request.generated_command,
            risk_level: request.risk_level,
            inserted_to_terminal: request.inserted_to_terminal,
            executed: request.executed,
            blocked: request.blocked,
            created_at: now_rfc3339(),
        };
        let mut file = self.load_ai_audit_file()?;
        file.logs.push(log.clone());
        trim_ai_audit(&mut file);
        self.save_settings_doc_value(SETTINGS_AI_AUDIT, &serde_json::to_value(file)?)?;
        Ok(log)
    }

    pub fn list_ai_audit_logs(
        &self,
        limit: Option<usize>,
    ) -> Result<Vec<AiAuditLog>, StorageError> {
        let mut logs = self.load_ai_audit_file()?.logs;
        logs.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        if let Some(limit) = limit {
            logs.truncate(limit);
        }
        Ok(logs)
    }

    pub(crate) fn build_raw_portable_snapshot(
        &self,
        snapshot_kind: PortableSnapshotKind,
        device_id: impl Into<String>,
        app_version: impl Into<String>,
    ) -> Result<RawPortableSnapshot, StorageError> {
        let mut snapshot = match snapshot_kind {
            PortableSnapshotKind::Sync => RawPortableSnapshot::sync(device_id, app_version),
            PortableSnapshotKind::Backup => RawPortableSnapshot::backup(device_id, app_version),
        };
        let mut settings = self.load_settings_value()?;
        set_nested_json_value(
            &mut settings,
            &["security", "master_password"],
            serde_json::Value::Null,
        );

        snapshot
            .entities
            .insert("settings".to_string(), serde_json::to_string(&settings)?);
        snapshot.entities.insert(
            "sessions".to_string(),
            serde_json::to_string(&self.load_sessions()?)?,
        );
        snapshot.entities.insert(
            "keys".to_string(),
            wrapped_raw_array_json(
                "keys",
                self.list_raw_json_values_by_prefix(CREDENTIALS_TABLE, SSH_KEY_PREFIX)?,
            )?,
        );
        snapshot.entities.insert(
            "passwords".to_string(),
            wrapped_raw_array_json(
                "passwords",
                self.list_raw_json_values_by_prefix(CREDENTIALS_TABLE, PASSWORD_PREFIX)?,
            )?,
        );
        snapshot.entities.insert(
            "credentials".to_string(),
            wrapped_raw_array_json(
                "credentials",
                self.list_raw_json_values_by_prefix(CREDENTIALS_TABLE, CREDENTIAL_PREFIX)?,
            )?,
        );
        snapshot.entities.insert(
            "otp".to_string(),
            wrapped_raw_array_json(
                "entries",
                self.list_raw_json_values_by_prefix(OTP_ACCOUNTS_TABLE, OTP_PREFIX)?,
            )?,
        );
        snapshot.entities.insert(
            "proxies".to_string(),
            serde_json::to_string(
                &self.list_raw_json_values_by_prefix(PROXIES_TABLE, PROXY_PREFIX)?,
            )?,
        );
        snapshot.entities.insert(
            "proxy_groups".to_string(),
            portable_settings_doc_array(
                self.load_settings_doc_value(SETTINGS_PROXY_GROUPS, serde_json::json!({}))?,
                "groups",
            )?,
        );
        snapshot.entities.insert(
            "tunnels".to_string(),
            serde_json::to_string(
                &self.list_raw_json_values_by_prefix(TUNNELS_TABLE, TUNNEL_PREFIX)?,
            )?,
        );
        snapshot.entities.insert(
            "tunnel_groups".to_string(),
            serde_json::to_string(&self.list_tunnel_groups()?)?,
        );
        snapshot.entities.insert(
            "quick_commands".to_string(),
            serde_json::to_string(&self.load_settings_doc_value(
                SETTINGS_QUICK_COMMANDS,
                serde_json::json!({"commands":[],"categories":[]}),
            )?)?,
        );
        snapshot.entities.insert(
            "history".to_string(),
            serde_json::to_string(&self.list_command_history(usize::MAX)?)?,
        );
        snapshot.entities.insert(
            "master_key_token".to_string(),
            serde_json::to_string(&self.load_master_key_token()?)?,
        );
        snapshot.entities.insert(
            "known_hosts".to_string(),
            serde_json::to_string(&self.render_known_hosts_export()?)?,
        );
        Ok(snapshot)
    }

    pub(crate) fn apply_raw_portable_snapshot(
        &self,
        snapshot: &RawPortableSnapshot,
    ) -> Result<(), StorageError> {
        let sessions: SessionsConfig = read_snapshot_entity(snapshot, "sessions")?;
        let settings: serde_json::Value = read_snapshot_entity(snapshot, "settings")?;
        let known_hosts: String = read_snapshot_entity(snapshot, "known_hosts")?;
        let master_key_token: Option<String> = read_snapshot_entity(snapshot, "master_key_token")?;
        let tunnel_groups: Vec<TunnelGroup> = read_snapshot_entity(snapshot, "tunnel_groups")?;
        let current_settings = self.load_settings_value()?;

        let txn = self.db.begin_write()?;
        replace_sessions_in_txn(&txn, &sessions)?;
        replace_raw_wrapped_array_in_txn(
            &txn,
            CREDENTIALS_TABLE,
            SSH_KEY_PREFIX,
            snapshot,
            "keys",
            "keys",
        )?;
        replace_raw_wrapped_array_in_txn(
            &txn,
            CREDENTIALS_TABLE,
            PASSWORD_PREFIX,
            snapshot,
            "passwords",
            "passwords",
        )?;
        replace_raw_wrapped_array_in_txn(
            &txn,
            CREDENTIALS_TABLE,
            CREDENTIAL_PREFIX,
            snapshot,
            "credentials",
            "credentials",
        )?;
        replace_raw_wrapped_array_in_txn(
            &txn,
            OTP_ACCOUNTS_TABLE,
            OTP_PREFIX,
            snapshot,
            "otp",
            "entries",
        )?;
        replace_raw_array_in_txn(&txn, PROXIES_TABLE, PROXY_PREFIX, snapshot, "proxies")?;
        replace_raw_array_in_txn(&txn, TUNNELS_TABLE, TUNNEL_PREFIX, snapshot, "tunnels")?;
        write_json_in_txn(
            &txn,
            SETTINGS_TABLE,
            SETTINGS_TUNNEL_GROUPS,
            &serde_json::json!({ "groups": tunnel_groups }),
        )?;
        write_settings_doc_from_entity_in_txn(
            &txn,
            SETTINGS_PROXY_GROUPS,
            snapshot,
            "proxy_groups",
            |value| serde_json::json!({ "groups": value }),
        )?;
        write_settings_doc_from_entity_in_txn(
            &txn,
            SETTINGS_QUICK_COMMANDS,
            snapshot,
            "quick_commands",
            std::convert::identity,
        )?;
        let history: Vec<CommandHistoryEntry> = read_snapshot_entity(snapshot, "history")?;
        replace_command_history_in_txn(&txn, &history)?;

        let merged_settings = merge_imported_settings(settings, current_settings);
        write_json_in_txn(&txn, SETTINGS_TABLE, SETTINGS_DEFAULT, &merged_settings)?;
        match master_key_token {
            Some(token) if !token.trim().is_empty() => {
                txn.open_table(META_TABLE)?
                    .insert(META_MASTER_KEY, token.as_str())?;
                txn.open_table(TEXT_DOCS_TABLE)?
                    .insert(LEGACY_TEXT_MASTER_KEY, token.as_str())?;
            }
            _ => {}
        }
        replace_known_hosts_text_in_txn(&txn, &known_hosts)?;
        txn.commit()?;
        Ok(())
    }

    fn ensure_tables(&self) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        txn.open_table(META_TABLE)?;
        txn.open_table(TEXT_DOCS_TABLE)?;
        txn.open_table(GROUPS_TABLE)?;
        txn.open_table(CONNECTIONS_TABLE)?;
        txn.open_table(TUNNELS_TABLE)?;
        txn.open_table(PROXIES_TABLE)?;
        txn.open_table(CREDENTIALS_TABLE)?;
        txn.open_table(OTP_ACCOUNTS_TABLE)?;
        txn.open_table(KNOWN_HOSTS_TABLE)?;
        txn.open_table(COMMAND_HISTORY_TABLE)?;
        txn.open_table(SETTINGS_TABLE)?;
        txn.open_table(IDX_CONNECTIONS_BY_GROUP_TABLE)?;
        txn.open_table(IDX_CONNECTIONS_BY_LAST_USED_TABLE)?;
        txn.open_table(IDX_CONNECTIONS_BY_PROTOCOL_TABLE)?;
        txn.commit()?;
        Ok(())
    }

    fn list_json_by_prefix<T>(
        &self,
        definition: TableDefinition<&str, &[u8]>,
        prefix: &str,
    ) -> Result<Vec<T>, StorageError>
    where
        T: DeserializeOwned,
    {
        let txn = self.db.begin_read()?;
        let table = match txn.open_table(definition) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let mut values = Vec::new();
        for entry in table.iter()? {
            let (key, value) = entry?;
            if key.value().starts_with(prefix) {
                values.push(deserialize_json(value.value())?);
            }
        }
        Ok(values)
    }

    fn read_json_table<T>(
        &self,
        definition: TableDefinition<&str, &[u8]>,
        key: &str,
    ) -> Result<Option<T>, StorageError>
    where
        T: DeserializeOwned,
    {
        let txn = self.db.begin_read()?;
        let table = match txn.open_table(definition) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let Some(raw) = table.get(key)? else {
            return Ok(None);
        };
        Ok(Some(deserialize_json(raw.value())?))
    }

    fn hydrate_connection_passwords(
        &self,
        connections: &mut [SavedConnection],
    ) -> Result<(), StorageError> {
        let master_key_token = self.load_master_key_token()?;
        let crypto = self.credential_crypto()?;
        let txn = self.db.begin_read()?;
        let table = match txn.open_table(CREDENTIALS_TABLE) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        for connection in connections {
            let Some(auth) = connection.auth.as_mut() else {
                continue;
            };
            if let (Some(master_key_token), Some(password)) =
                (master_key_token.as_deref(), auth.password.as_deref())
            {
                if let Ok(plaintext) = crypto.decrypt_secret(master_key_token, password) {
                    auth.password = Some(plaintext);
                    auth.has_password = false;
                    continue;
                }
            }
            let key = entity_key(CONNECTION_PASSWORD_PREFIX, &connection.id);
            if let Some(raw) = table.get(key.as_str())? {
                let record: ConnectionPasswordRecord = deserialize_json(raw.value())?;
                if let Some(master_key_token) = master_key_token.as_deref() {
                    match crypto.decrypt_secret(master_key_token, &record.password) {
                        Ok(plaintext) => {
                            auth.password = Some(plaintext);
                            auth.has_password = false;
                        }
                        Err(_) => {
                            auth.password = Some(record.password);
                            auth.has_password = true;
                        }
                    }
                } else {
                    auth.password = Some(record.password);
                    auth.has_password = true;
                }
            }
        }
        Ok(())
    }

    fn credential_crypto(&self) -> Result<CredentialCrypto, StorageError> {
        let bootstrap = CredentialCrypto::new(self.portable_key_path.clone(), None);
        let master_password = self
            .load_encrypted_master_password()?
            .and_then(|token| bootstrap.decrypt_settings_secret(&token).ok());
        Ok(CredentialCrypto::new(
            self.portable_key_path.clone(),
            master_password,
        ))
    }

    fn load_encrypted_master_password(&self) -> Result<Option<String>, StorageError> {
        let value = self.load_settings_value()?;
        Ok(value
            .get("security")
            .and_then(|security| security.get("master_password"))
            .and_then(|master_password| master_password.as_str())
            .filter(|master_password| !master_password.is_empty())
            .map(ToOwned::to_owned))
    }

    fn load_settings_value(&self) -> Result<serde_json::Value, StorageError> {
        let txn = self.db.begin_read()?;
        let table = match txn.open_table(SETTINGS_TABLE) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(default_settings_value()),
            Err(error) => return Err(error.into()),
        };
        let Some(raw) = table.get(SETTINGS_DEFAULT)? else {
            return Ok(default_settings_value());
        };
        Ok(deserialize_json(raw.value())?)
    }

    fn load_settings_doc_value(
        &self,
        key: &str,
        fallback: serde_json::Value,
    ) -> Result<serde_json::Value, StorageError> {
        let txn = self.db.begin_read()?;
        let table = match txn.open_table(SETTINGS_TABLE) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(fallback),
            Err(error) => return Err(error.into()),
        };
        let Some(raw) = table.get(key)? else {
            return Ok(fallback);
        };
        Ok(deserialize_json(raw.value())?)
    }

    fn load_ai_audit_file(&self) -> Result<AiAuditFile, StorageError> {
        self.read_json_table::<AiAuditFile>(SETTINGS_TABLE, SETTINGS_AI_AUDIT)
            .map(|file| file.unwrap_or_default())
    }

    fn save_settings_value(&self, value: &serde_json::Value) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        write_json_in_txn(&txn, SETTINGS_TABLE, SETTINGS_DEFAULT, value)?;
        txn.commit()?;
        Ok(())
    }

    fn save_settings_doc_value(
        &self,
        key: &str,
        value: &serde_json::Value,
    ) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        write_json_in_txn(&txn, SETTINGS_TABLE, key, value)?;
        txn.commit()?;
        Ok(())
    }

    fn decrypt_cloud_sync_settings(
        &self,
        settings: &mut CloudSyncSettings,
    ) -> Result<(), StorageError> {
        let crypto = self.credential_crypto()?;
        let master_key_token = self.load_master_key_token()?;
        let master_key_token = master_key_token.as_deref();
        settings.webdav.password =
            decrypt_optional_secret(&crypto, master_key_token, &settings.webdav.password)?;
        settings.s3.access_key_id =
            decrypt_optional_secret(&crypto, master_key_token, &settings.s3.access_key_id)?;
        settings.s3.secret_access_key =
            decrypt_optional_secret(&crypto, master_key_token, &settings.s3.secret_access_key)?;
        settings.s3.session_token =
            decrypt_optional_secret(&crypto, master_key_token, &settings.s3.session_token)?;
        settings.gitee_snippet.access_token = decrypt_optional_secret(
            &crypto,
            master_key_token,
            &settings.gitee_snippet.access_token,
        )?;
        decrypt_oauth_drive_settings(&crypto, master_key_token, &mut settings.google_drive)?;
        decrypt_oauth_drive_settings(&crypto, master_key_token, &mut settings.onedrive)?;
        decrypt_aliyun_drive_settings(&crypto, master_key_token, &mut settings.aliyun_drive)?;
        settings.github_gist.access_token = decrypt_optional_secret(
            &crypto,
            master_key_token,
            &settings.github_gist.access_token,
        )?;
        Ok(())
    }

    fn encrypt_cloud_sync_settings(
        &self,
        mut settings: CloudSyncSettings,
    ) -> Result<CloudSyncSettings, StorageError> {
        let crypto = self.credential_crypto()?;
        let master_key_token = if cloud_sync_settings_has_secret(&settings) {
            Some(self.get_or_create_master_key_token(&crypto)?)
        } else {
            None
        };
        let master_key_token = master_key_token.as_deref();
        settings.webdav.password =
            encrypt_optional_secret(&crypto, master_key_token, &settings.webdav.password)?;
        settings.s3.access_key_id =
            encrypt_optional_secret(&crypto, master_key_token, &settings.s3.access_key_id)?;
        settings.s3.secret_access_key =
            encrypt_optional_secret(&crypto, master_key_token, &settings.s3.secret_access_key)?;
        settings.s3.session_token =
            encrypt_optional_secret(&crypto, master_key_token, &settings.s3.session_token)?;
        settings.gitee_snippet.access_token = encrypt_optional_secret(
            &crypto,
            master_key_token,
            &settings.gitee_snippet.access_token,
        )?;
        encrypt_oauth_drive_settings(&crypto, master_key_token, &mut settings.google_drive)?;
        encrypt_oauth_drive_settings(&crypto, master_key_token, &mut settings.onedrive)?;
        encrypt_aliyun_drive_settings(&crypto, master_key_token, &mut settings.aliyun_drive)?;
        settings.github_gist.access_token = encrypt_optional_secret(
            &crypto,
            master_key_token,
            &settings.github_gist.access_token,
        )?;
        Ok(settings)
    }

    fn decrypt_ai_settings(&self, settings: &mut AiSettings) -> Result<(), StorageError> {
        let crypto = self.credential_crypto()?;
        let master_key_token = self.load_master_key_token()?;
        let master_key_token = master_key_token.as_deref();
        for profile in &mut settings.provider_profiles {
            profile.api_key = decrypt_optional_secret(&crypto, master_key_token, &profile.api_key)?;
        }
        for credential in &mut settings.provider_credentials {
            credential.api_key =
                decrypt_optional_secret(&crypto, master_key_token, &credential.api_key)?;
        }
        Ok(())
    }

    fn encrypt_ai_settings(&self, mut settings: AiSettings) -> Result<AiSettings, StorageError> {
        let crypto = self.credential_crypto()?;
        let master_key_token = if ai_settings_has_secret(&settings) {
            Some(self.get_or_create_master_key_token(&crypto)?)
        } else {
            None
        };
        let master_key_token = master_key_token.as_deref();
        for profile in &mut settings.provider_profiles {
            profile.api_key = encrypt_optional_secret(&crypto, master_key_token, &profile.api_key)?;
        }
        for credential in &mut settings.provider_credentials {
            credential.api_key =
                encrypt_optional_secret(&crypto, master_key_token, &credential.api_key)?;
        }
        Ok(settings)
    }

    fn decrypt_translation_settings(
        &self,
        settings: &mut TranslationSettings,
    ) -> Result<(), StorageError> {
        let crypto = self.credential_crypto()?;
        let master_key_token = self.load_master_key_token()?;
        let master_key_token = master_key_token.as_deref();
        settings.deepl_api_key =
            decrypt_legacy_plaintext_secret(&crypto, master_key_token, &settings.deepl_api_key)?;
        settings.baidu_app_key =
            decrypt_legacy_plaintext_secret(&crypto, master_key_token, &settings.baidu_app_key)?;
        settings.ali_app_key =
            decrypt_legacy_plaintext_secret(&crypto, master_key_token, &settings.ali_app_key)?;
        settings.youdao_app_key =
            decrypt_legacy_plaintext_secret(&crypto, master_key_token, &settings.youdao_app_key)?;
        Ok(())
    }

    fn encrypt_translation_settings(
        &self,
        mut settings: TranslationSettings,
    ) -> Result<TranslationSettings, StorageError> {
        let crypto = self.credential_crypto()?;
        let master_key_token = if translation_settings_has_secret(&settings) {
            Some(self.get_or_create_master_key_token(&crypto)?)
        } else {
            None
        };
        let master_key_token = master_key_token.as_deref();
        settings.deepl_api_key =
            encrypt_string_secret(&crypto, master_key_token, &settings.deepl_api_key)?;
        settings.baidu_app_key =
            encrypt_string_secret(&crypto, master_key_token, &settings.baidu_app_key)?;
        settings.ali_app_key =
            encrypt_string_secret(&crypto, master_key_token, &settings.ali_app_key)?;
        settings.youdao_app_key =
            encrypt_string_secret(&crypto, master_key_token, &settings.youdao_app_key)?;
        Ok(settings)
    }

    fn get_or_create_master_key_token(
        &self,
        crypto: &CredentialCrypto,
    ) -> Result<String, StorageError> {
        if let Some(token) = self.load_master_key_token()? {
            return Ok(token);
        }
        let token = crypto.generate_master_key_token()?;
        self.save_master_key_token(&token)?;
        Ok(token)
    }

    fn save_master_key_token(&self, token: &str) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        txn.open_table(META_TABLE)?.insert(META_MASTER_KEY, token)?;
        txn.open_table(TEXT_DOCS_TABLE)?
            .insert(LEGACY_TEXT_MASTER_KEY, token)?;
        txn.commit()?;
        Ok(())
    }

    fn load_master_key_token(&self) -> Result<Option<String>, StorageError> {
        if let Some(token) = self.read_string_table(META_TABLE, META_MASTER_KEY)? {
            return Ok(Some(token));
        }
        self.read_string_table(TEXT_DOCS_TABLE, LEGACY_TEXT_MASTER_KEY)
    }

    fn import_legacy_known_hosts_if_needed(&self) -> Result<(), StorageError> {
        let has_native = !self
            .list_raw_by_prefix(KNOWN_HOSTS_TABLE, KNOWN_HOST_PREFIX)?
            .is_empty();
        if has_native {
            return Ok(());
        }
        let Some(content) = self.read_string_table(TEXT_DOCS_TABLE, LEGACY_TEXT_KNOWN_HOSTS)?
        else {
            return Ok(());
        };
        let txn = self.db.begin_write()?;
        replace_known_hosts_text_in_txn(&txn, &content)?;
        txn.commit()?;
        Ok(())
    }

    fn read_string_table(
        &self,
        definition: TableDefinition<&str, &str>,
        key: &str,
    ) -> Result<Option<String>, StorageError> {
        let txn = self.db.begin_read()?;
        let table = match txn.open_table(definition) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        Ok(table.get(key)?.map(|raw| raw.value().to_string()))
    }

    fn list_raw_by_prefix(
        &self,
        definition: TableDefinition<&str, &[u8]>,
        prefix: &str,
    ) -> Result<Vec<(String, Vec<u8>)>, StorageError> {
        let txn = self.db.begin_read()?;
        let table = match txn.open_table(definition) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let mut values = Vec::new();
        for entry in table.iter()? {
            let (key, value) = entry?;
            if key.value().starts_with(prefix) {
                values.push((key.value().to_string(), value.value().to_vec()));
            }
        }
        Ok(values)
    }

    fn list_raw_json_values_by_prefix(
        &self,
        definition: TableDefinition<&str, &[u8]>,
        prefix: &str,
    ) -> Result<Vec<serde_json::Value>, StorageError> {
        self.list_raw_by_prefix(definition, prefix)?
            .into_iter()
            .map(|(_, value)| serde_json::from_slice(&value).map_err(StorageError::from))
            .collect()
    }

    fn list_keyed_json_by_prefix<T>(
        &self,
        definition: TableDefinition<&str, &[u8]>,
        prefix: &str,
    ) -> Result<Vec<(String, T)>, StorageError>
    where
        T: DeserializeOwned,
    {
        self.list_raw_by_prefix(definition, prefix)?
            .into_iter()
            .map(|(key, value)| {
                serde_json::from_slice(&value)
                    .map(|value| (key, value))
                    .map_err(StorageError::from)
            })
            .collect()
    }
}

fn wrapped_raw_array_json(
    field: &str,
    values: Vec<serde_json::Value>,
) -> Result<String, StorageError> {
    serde_json::to_string(&serde_json::json!({ field: values })).map_err(StorageError::from)
}

fn portable_settings_doc_array(
    value: serde_json::Value,
    field: &str,
) -> Result<String, StorageError> {
    if value.is_array() {
        return serde_json::to_string(&value).map_err(StorageError::from);
    }
    let values = value
        .get(field)
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Array(Vec::new()));
    serde_json::to_string(&values).map_err(StorageError::from)
}

fn replace_command_history_in_txn(
    txn: &redb::WriteTransaction,
    entries: &[CommandHistoryEntry],
) -> Result<(), StorageError> {
    clear_prefix_in_txn(txn, COMMAND_HISTORY_TABLE, COMMAND_HISTORY_PREFIX)?;
    let mut normalized = Vec::new();
    for entry in entries {
        let Some(command) = sanitize_history_command(&entry.command) else {
            continue;
        };
        merge_command_history_entry(
            &mut normalized,
            CommandHistoryEntry {
                command,
                last_used_at_ms: entry.last_used_at_ms,
                use_count: entry.use_count.max(1),
            },
        );
    }
    normalized.sort_by_key(|entry| entry.last_used_at_ms);
    trim_command_history(&mut normalized);
    for entry in normalized {
        save_command_history_entry_in_txn(txn, &entry)?;
    }
    Ok(())
}

fn save_command_history_entry_in_txn(
    txn: &redb::WriteTransaction,
    entry: &CommandHistoryEntry,
) -> Result<(), StorageError> {
    let Some(command) = sanitize_history_command(&entry.command) else {
        return Ok(());
    };
    let normalized = CommandHistoryEntry {
        command,
        last_used_at_ms: entry.last_used_at_ms,
        use_count: entry.use_count.max(1),
    };
    let id = command_history_id(&normalized.command);
    remove_command_history_id_in_txn(txn, &id)?;
    write_json_in_txn(
        txn,
        COMMAND_HISTORY_TABLE,
        &command_history_key(&normalized, &id),
        &normalized,
    )
}

fn remove_command_history_id_in_txn(
    txn: &redb::WriteTransaction,
    id: &str,
) -> Result<(), StorageError> {
    let table = txn.open_table(COMMAND_HISTORY_TABLE)?;
    let suffix = format!("|{id}");
    let mut keys = Vec::new();
    for entry in table.iter()? {
        let (key, _) = entry?;
        if key.value().ends_with(&suffix) {
            keys.push(key.value().to_string());
        }
    }
    drop(table);
    let mut table = txn.open_table(COMMAND_HISTORY_TABLE)?;
    for key in keys {
        table.remove(key.as_str())?;
    }
    Ok(())
}

fn merge_command_history_entry(
    entries: &mut Vec<CommandHistoryEntry>,
    incoming: CommandHistoryEntry,
) {
    if let Some(index) = entries
        .iter()
        .position(|entry| entry.command == incoming.command)
    {
        let mut existing = entries.remove(index);
        existing.last_used_at_ms = existing.last_used_at_ms.max(incoming.last_used_at_ms);
        existing.use_count = existing.use_count.saturating_add(incoming.use_count.max(1));
        entries.push(existing);
    } else {
        entries.push(incoming);
    }
}

fn trim_command_history(entries: &mut Vec<CommandHistoryEntry>) {
    const MAX_HISTORY: usize = 5000;
    if entries.len() > MAX_HISTORY {
        let overflow = entries.len() - MAX_HISTORY;
        entries.drain(..overflow);
    }
}

fn command_history_key(entry: &CommandHistoryEntry, id: &str) -> String {
    format!(
        "{}{:020}|{}",
        COMMAND_HISTORY_PREFIX, entry.last_used_at_ms, id
    )
}

fn command_history_id(command: &str) -> String {
    let digest = Sha256::digest(command.as_bytes());
    lower_hex(&digest[..16])
}

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

fn sanitize_history_command(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let without_prompt = strip_known_prompt_prefix(strip_leading_env_prefixes(trimmed))
        .unwrap_or(trimmed)
        .trim();
    if without_prompt.is_empty() {
        None
    } else {
        Some(without_prompt.to_string())
    }
}

fn strip_leading_env_prefixes(mut input: &str) -> &str {
    loop {
        let Some(rest) = input.strip_prefix('(') else {
            return input;
        };
        let Some(close_idx) = rest.find(')') else {
            return input;
        };
        let after_close = &rest[close_idx + 1..];
        input = after_close.trim_start_matches([' ', '\t']);
    }
}

fn strip_known_prompt_prefix(input: &str) -> Option<&str> {
    strip_bracket_prompt(input)
        .or_else(|| strip_posix_prompt(input))
        .or_else(|| strip_powershell_prompt(input))
        .or_else(|| strip_windows_prompt(input))
}

fn strip_bracket_prompt(input: &str) -> Option<&str> {
    let rest = input.strip_prefix('[')?;
    let close_idx = rest.find(']')?;
    let after_bracket = rest[close_idx + 1..].trim_start_matches([' ', '\t']);
    let after_marker = after_bracket
        .strip_prefix('#')
        .or_else(|| after_bracket.strip_prefix('$'))?;
    Some(after_marker.trim_start_matches([' ', '\t']))
}

fn strip_posix_prompt(input: &str) -> Option<&str> {
    let prompt_end = input.find(['#', '$'])?;
    let prompt = &input[..prompt_end];
    let after_marker = &input[prompt_end + 1..];
    let at_idx = prompt.find('@')?;
    let colon_rel = prompt[at_idx + 1..].find(':')?;
    let colon_idx = at_idx + 1 + colon_rel;
    let user = &prompt[..at_idx];
    let host = &prompt[at_idx + 1..colon_idx];
    if user.is_empty()
        || host.is_empty()
        || user.chars().any(char::is_whitespace)
        || host.chars().any(char::is_whitespace)
    {
        return None;
    }
    Some(after_marker.trim_start_matches([' ', '\t']))
}

fn strip_powershell_prompt(input: &str) -> Option<&str> {
    let rest = input
        .strip_prefix("PS ")
        .or_else(|| input.strip_prefix("PS\t"))?;
    let marker_idx = rest.find('>')?;
    if rest[..marker_idx].trim().is_empty() {
        return None;
    }
    Some(rest[marker_idx + 1..].trim_start_matches([' ', '\t']))
}

fn strip_windows_prompt(input: &str) -> Option<&str> {
    let bytes = input.as_bytes();
    if bytes.len() < 3 || !bytes[0].is_ascii_alphabetic() || bytes[1] != b':' {
        return None;
    }
    let marker_idx = input.find('>')?;
    if input[..marker_idx].contains(['\r', '\n']) {
        return None;
    }
    Some(input[marker_idx + 1..].trim_start_matches([' ', '\t']))
}

fn read_snapshot_entity<T>(
    snapshot: &RawPortableSnapshot,
    entity: &'static str,
) -> Result<T, StorageError>
where
    T: DeserializeOwned,
{
    let raw = snapshot
        .entities
        .get(entity)
        .ok_or(StorageError::PortableSnapshotEntity {
            entity: entity.to_string(),
            message: "missing entity".to_string(),
        })?;
    serde_json::from_str(raw).map_err(StorageError::from)
}

fn replace_raw_wrapped_array_in_txn(
    txn: &redb::WriteTransaction,
    definition: TableDefinition<&str, &[u8]>,
    prefix: &str,
    snapshot: &RawPortableSnapshot,
    entity: &'static str,
    field: &'static str,
) -> Result<(), StorageError> {
    let value: serde_json::Value = read_snapshot_entity(snapshot, entity)?;
    let values = value
        .get(field)
        .and_then(serde_json::Value::as_array)
        .cloned()
        .ok_or(StorageError::PortableSnapshotEntity {
            entity: entity.to_string(),
            message: format!("expected object field '{field}' to be an array"),
        })?;
    replace_raw_json_values_by_id_in_txn(txn, definition, prefix, entity, values)
}

fn replace_raw_array_in_txn(
    txn: &redb::WriteTransaction,
    definition: TableDefinition<&str, &[u8]>,
    prefix: &str,
    snapshot: &RawPortableSnapshot,
    entity: &'static str,
) -> Result<(), StorageError> {
    let values: Vec<serde_json::Value> = read_snapshot_entity(snapshot, entity)?;
    replace_raw_json_values_by_id_in_txn(txn, definition, prefix, entity, values)
}

fn replace_raw_json_values_by_id_in_txn(
    txn: &redb::WriteTransaction,
    definition: TableDefinition<&str, &[u8]>,
    prefix: &str,
    entity: &'static str,
    values: Vec<serde_json::Value>,
) -> Result<(), StorageError> {
    clear_prefix_in_txn(txn, definition, prefix)?;
    for (index, value) in values.iter().enumerate() {
        let id = value
            .get("id")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or(StorageError::PortableSnapshotEntity {
                entity: entity.to_string(),
                message: format!("entry {index} is missing string id"),
            })?;
        write_json_in_txn(txn, definition, &entity_key(prefix, id), value)?;
    }
    Ok(())
}

fn write_settings_doc_from_entity_in_txn(
    txn: &redb::WriteTransaction,
    key: &str,
    snapshot: &RawPortableSnapshot,
    entity: &'static str,
    wrap: impl FnOnce(serde_json::Value) -> serde_json::Value,
) -> Result<(), StorageError> {
    let value: serde_json::Value = read_snapshot_entity(snapshot, entity)?;
    write_json_in_txn(txn, SETTINGS_TABLE, key, &wrap(value))
}

fn merge_imported_settings(
    mut imported: serde_json::Value,
    current: serde_json::Value,
) -> serde_json::Value {
    if let Some(master_password) = current
        .get("security")
        .and_then(|security| security.get("master_password"))
        .cloned()
    {
        set_nested_json_value(
            &mut imported,
            &["security", "master_password"],
            master_password,
        );
    }
    imported
}

fn replace_sessions_in_txn(
    txn: &redb::WriteTransaction,
    config: &SessionsConfig,
) -> Result<(), StorageError> {
    clear_prefix_in_txn(txn, GROUPS_TABLE, GROUP_PREFIX)?;
    clear_prefix_in_txn(txn, CONNECTIONS_TABLE, CONNECTION_PREFIX)?;
    clear_prefix_in_txn(txn, CREDENTIALS_TABLE, CONNECTION_PASSWORD_PREFIX)?;
    clear_string_table(txn, IDX_CONNECTIONS_BY_GROUP_TABLE)?;
    clear_string_table(txn, IDX_CONNECTIONS_BY_LAST_USED_TABLE)?;
    clear_string_table(txn, IDX_CONNECTIONS_BY_PROTOCOL_TABLE)?;
    for group in &config.groups {
        save_group_in_txn(txn, group)?;
    }
    for connection in &config.connections {
        save_connection_in_txn(txn, connection)?;
    }
    Ok(())
}

fn save_group_in_txn(txn: &redb::WriteTransaction, group: &Group) -> Result<(), StorageError> {
    let mut group = group.clone();
    let now = current_time_ms();
    let key = entity_key(GROUP_PREFIX, &group.id);
    if group.created_at_ms.is_none() {
        group.created_at_ms = existing_group_created_at(txn, &key)?.or(Some(now));
    }
    group.updated_at_ms = Some(now);
    write_json_in_txn(txn, GROUPS_TABLE, &key, &group)
}

fn existing_group_created_at(
    txn: &redb::WriteTransaction,
    key: &str,
) -> Result<Option<u64>, StorageError> {
    let table = txn.open_table(GROUPS_TABLE)?;
    let Some(raw) = table.get(key)? else {
        return Ok(None);
    };
    let group: Group = deserialize_json(raw.value())?;
    Ok(group.created_at_ms)
}

fn save_connection_in_txn(
    txn: &redb::WriteTransaction,
    connection: &SavedConnection,
) -> Result<(), StorageError> {
    let mut connection = connection.clone();
    let now = current_time_ms();
    let connection_key = entity_key(CONNECTION_PREFIX, &connection.id);
    if connection.created_at_ms.is_none() {
        connection.created_at_ms =
            existing_connection_created_at(txn, &connection_key)?.or(Some(now));
    }
    connection.updated_at_ms = Some(now);

    remove_connection_index_entries(txn, &connection.id)?;
    delete_connection_password_in_txn(txn, &connection.id)?;
    if let Some(auth) = connection.auth.as_mut() {
        if let Some(password) = auth.password.take().filter(|value| !value.is_empty()) {
            let record = ConnectionPasswordRecord {
                id: connection.id.clone(),
                connection_id: connection.id.clone(),
                password,
                created_at_ms: now,
                updated_at_ms: now,
            };
            write_json_in_txn(
                txn,
                CREDENTIALS_TABLE,
                &entity_key(CONNECTION_PASSWORD_PREFIX, &connection.id),
                &record,
            )?;
        }
        auth.has_password = false;
    }

    write_json_in_txn(txn, CONNECTIONS_TABLE, &connection_key, &connection)?;
    insert_connection_indexes(txn, &connection)?;
    Ok(())
}

fn existing_connection_created_at(
    txn: &redb::WriteTransaction,
    key: &str,
) -> Result<Option<u64>, StorageError> {
    let table = txn.open_table(CONNECTIONS_TABLE)?;
    let Some(raw) = table.get(key)? else {
        return Ok(None);
    };
    let connection: SavedConnection = deserialize_json(raw.value())?;
    Ok(connection.created_at_ms)
}

fn delete_connection_in_txn(
    txn: &redb::WriteTransaction,
    connection_id: &str,
) -> Result<(), StorageError> {
    txn.open_table(CONNECTIONS_TABLE)?
        .remove(entity_key(CONNECTION_PREFIX, connection_id).as_str())?;
    delete_connection_password_in_txn(txn, connection_id)?;
    remove_connection_index_entries(txn, connection_id)
}

fn delete_connection_password_in_txn(
    txn: &redb::WriteTransaction,
    connection_id: &str,
) -> Result<(), StorageError> {
    txn.open_table(CREDENTIALS_TABLE)?
        .remove(entity_key(CONNECTION_PASSWORD_PREFIX, connection_id).as_str())?;
    Ok(())
}

fn insert_connection_indexes(
    txn: &redb::WriteTransaction,
    connection: &SavedConnection,
) -> Result<(), StorageError> {
    let group_id = connection.group_id.as_deref().unwrap_or_default();
    let group_key = format!(
        "{}|{}|{}",
        group_id,
        padded_i64(i64::from(connection.sort_order)),
        connection.id
    );
    txn.open_table(IDX_CONNECTIONS_BY_GROUP_TABLE)?
        .insert(group_key.as_str(), connection.id.as_str())?;

    let last_used = connection.last_used_at_ms.unwrap_or_default();
    let reverse = u64::MAX.saturating_sub(last_used);
    let last_used_key = format!("{reverse:020}|{}", connection.id);
    txn.open_table(IDX_CONNECTIONS_BY_LAST_USED_TABLE)?
        .insert(last_used_key.as_str(), connection.id.as_str())?;

    let protocol_key = format!(
        "{}|{}",
        connection.kind_label().to_lowercase(),
        connection.id
    );
    txn.open_table(IDX_CONNECTIONS_BY_PROTOCOL_TABLE)?
        .insert(protocol_key.as_str(), connection.id.as_str())?;

    Ok(())
}

fn remove_connection_index_entries(
    txn: &redb::WriteTransaction,
    connection_id: &str,
) -> Result<(), StorageError> {
    remove_connection_index_entries_from_table(txn, IDX_CONNECTIONS_BY_GROUP_TABLE, connection_id)?;
    remove_connection_index_entries_from_table(
        txn,
        IDX_CONNECTIONS_BY_LAST_USED_TABLE,
        connection_id,
    )?;
    remove_connection_index_entries_from_table(
        txn,
        IDX_CONNECTIONS_BY_PROTOCOL_TABLE,
        connection_id,
    )
}

fn remove_connection_index_entries_from_table(
    txn: &redb::WriteTransaction,
    definition: TableDefinition<&str, &str>,
    connection_id: &str,
) -> Result<(), StorageError> {
    let table = txn.open_table(definition)?;
    let mut keys = Vec::new();
    for entry in table.iter()? {
        let (key, value) = entry?;
        if value.value() == connection_id || key.value().ends_with(&format!("|{connection_id}")) {
            keys.push(key.value().to_string());
        }
    }
    drop(table);

    let mut table = txn.open_table(definition)?;
    for key in keys {
        table.remove(key.as_str())?;
    }
    Ok(())
}

fn clear_prefix_in_txn(
    txn: &redb::WriteTransaction,
    definition: TableDefinition<&str, &[u8]>,
    prefix: &str,
) -> Result<(), StorageError> {
    let table = txn.open_table(definition)?;
    let mut keys = Vec::new();
    for entry in table.iter()? {
        let (key, _) = entry?;
        if key.value().starts_with(prefix) {
            keys.push(key.value().to_string());
        }
    }
    drop(table);

    let mut table = txn.open_table(definition)?;
    for key in keys {
        table.remove(key.as_str())?;
    }
    Ok(())
}

fn clear_string_table(
    txn: &redb::WriteTransaction,
    definition: TableDefinition<&str, &str>,
) -> Result<(), StorageError> {
    let table = txn.open_table(definition)?;
    let keys = table
        .iter()?
        .map(|entry| entry.map(|(key, _)| key.value().to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    drop(table);

    let mut table = txn.open_table(definition)?;
    for key in keys {
        table.remove(key.as_str())?;
    }
    Ok(())
}

fn replace_known_hosts_text_in_txn(
    txn: &redb::WriteTransaction,
    content: &str,
) -> Result<(), StorageError> {
    clear_prefix_in_txn(txn, KNOWN_HOSTS_TABLE, KNOWN_HOST_PREFIX)?;
    for line in content.lines() {
        save_known_hosts_line_in_txn(txn, line)?;
    }
    Ok(())
}

fn save_known_hosts_line_in_txn(
    txn: &redb::WriteTransaction,
    line: &str,
) -> Result<(), StorageError> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    let now = current_time_ms();
    if let Some(record) = parse_known_host_line(trimmed, now) {
        write_json_in_txn(txn, KNOWN_HOSTS_TABLE, &known_host_key(&record), &record)?;
    } else {
        let record = KnownHostRawRecord {
            line: trimmed.to_string(),
            created_at_ms: now,
            updated_at_ms: now,
        };
        write_json_in_txn(
            txn,
            KNOWN_HOSTS_TABLE,
            &format!("{}{}", KNOWN_HOST_RAW_PREFIX, stable_id(trimmed)),
            &record,
        )?;
    }
    Ok(())
}

fn remove_known_hosts_for_host_in_txn(
    txn: &redb::WriteTransaction,
    host_identifier: &str,
) -> Result<(), StorageError> {
    let table = txn.open_table(KNOWN_HOSTS_TABLE)?;
    let mut keys = Vec::new();
    for entry in table.iter()? {
        let (key, value) = entry?;
        if key.value().starts_with(KNOWN_HOST_RAW_PREFIX) {
            continue;
        }
        let record: KnownHostRecord = deserialize_json(value.value())?;
        if known_host_record_matches(&record, host_identifier) {
            keys.push(key.value().to_string());
        }
    }
    drop(table);

    let mut table = txn.open_table(KNOWN_HOSTS_TABLE)?;
    for key in keys {
        table.remove(key.as_str())?;
    }
    Ok(())
}

fn parse_known_host_line(line: &str, now: u64) -> Option<KnownHostRecord> {
    if line.starts_with('#') {
        return None;
    }
    let mut parts = line.split_whitespace();
    let first = parts.next()?;
    let (marker, host_list) = if first.starts_with('@') {
        (Some(first.to_string()), parts.next()?)
    } else {
        (None, first)
    };
    let key_type = parts.next()?;
    let key_base64 = parts.next()?;
    let comment = {
        let rest = parts.collect::<Vec<_>>().join(" ");
        if rest.is_empty() { None } else { Some(rest) }
    };
    let host_patterns = host_list
        .split(',')
        .filter(|pattern| !pattern.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if host_patterns.is_empty() {
        return None;
    }

    Some(KnownHostRecord {
        marker,
        host_identifier: host_patterns[0].clone(),
        host_patterns,
        key_type: key_type.to_string(),
        key_base64: key_base64.to_string(),
        comment,
        raw_line: Some(line.to_string()),
        created_at_ms: now,
        updated_at_ms: now,
    })
}

fn apply_ssh_key_status_flags(key: &mut SshKey) {
    key.has_key_data = key.key.is_some();
    key.has_cert_data = key.cert.is_some();
}

fn decrypt_optional_secret(
    crypto: &CredentialCrypto,
    master_key_token: Option<&str>,
    value: &Option<String>,
) -> Result<Option<String>, StorageError> {
    let Some(value) = value.as_deref().filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let Some(master_key_token) = master_key_token else {
        return Err(StorageError::MissingMasterKey);
    };
    crypto
        .decrypt_secret(master_key_token, value)
        .map(Some)
        .map_err(StorageError::from)
}

fn encrypt_optional_secret(
    crypto: &CredentialCrypto,
    master_key_token: Option<&str>,
    value: &Option<String>,
) -> Result<Option<String>, StorageError> {
    let Some(value) = value.as_deref().filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let Some(master_key_token) = master_key_token else {
        return Err(StorageError::MissingMasterKey);
    };
    crypto
        .encrypt_secret(master_key_token, value)
        .map(Some)
        .map_err(StorageError::from)
}

fn decrypt_legacy_plaintext_secret(
    crypto: &CredentialCrypto,
    master_key_token: Option<&str>,
    value: &str,
) -> Result<String, StorageError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(String::new());
    }
    let Some(master_key_token) = master_key_token else {
        return Ok(value.to_string());
    };
    Ok(crypto
        .decrypt_secret(master_key_token, value)
        .unwrap_or_else(|_| value.to_string()))
}

fn encrypt_string_secret(
    crypto: &CredentialCrypto,
    master_key_token: Option<&str>,
    value: &str,
) -> Result<String, StorageError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(String::new());
    }
    let Some(master_key_token) = master_key_token else {
        return Err(StorageError::MissingMasterKey);
    };
    crypto
        .encrypt_secret(master_key_token, value)
        .map_err(StorageError::from)
}

fn decrypt_oauth_drive_settings(
    crypto: &CredentialCrypto,
    master_key_token: Option<&str>,
    settings: &mut OAuthDriveSyncSettings,
) -> Result<(), StorageError> {
    settings.access_token =
        decrypt_optional_secret(crypto, master_key_token, &settings.access_token)?;
    settings.refresh_token =
        decrypt_optional_secret(crypto, master_key_token, &settings.refresh_token)?;
    settings.client_secret =
        decrypt_optional_secret(crypto, master_key_token, &settings.client_secret)?;
    Ok(())
}

fn encrypt_oauth_drive_settings(
    crypto: &CredentialCrypto,
    master_key_token: Option<&str>,
    settings: &mut OAuthDriveSyncSettings,
) -> Result<(), StorageError> {
    settings.access_token =
        encrypt_optional_secret(crypto, master_key_token, &settings.access_token)?;
    settings.refresh_token =
        encrypt_optional_secret(crypto, master_key_token, &settings.refresh_token)?;
    settings.client_secret =
        encrypt_optional_secret(crypto, master_key_token, &settings.client_secret)?;
    Ok(())
}

fn decrypt_aliyun_drive_settings(
    crypto: &CredentialCrypto,
    master_key_token: Option<&str>,
    settings: &mut AliyunDriveSyncSettings,
) -> Result<(), StorageError> {
    settings.access_token =
        decrypt_optional_secret(crypto, master_key_token, &settings.access_token)?;
    settings.refresh_token =
        decrypt_optional_secret(crypto, master_key_token, &settings.refresh_token)?;
    settings.client_secret =
        decrypt_optional_secret(crypto, master_key_token, &settings.client_secret)?;
    Ok(())
}

fn encrypt_aliyun_drive_settings(
    crypto: &CredentialCrypto,
    master_key_token: Option<&str>,
    settings: &mut AliyunDriveSyncSettings,
) -> Result<(), StorageError> {
    settings.access_token =
        encrypt_optional_secret(crypto, master_key_token, &settings.access_token)?;
    settings.refresh_token =
        encrypt_optional_secret(crypto, master_key_token, &settings.refresh_token)?;
    settings.client_secret =
        encrypt_optional_secret(crypto, master_key_token, &settings.client_secret)?;
    Ok(())
}

fn cloud_sync_settings_has_secret(settings: &CloudSyncSettings) -> bool {
    optional_secret_present(&settings.webdav.password)
        || optional_secret_present(&settings.s3.access_key_id)
        || optional_secret_present(&settings.s3.secret_access_key)
        || optional_secret_present(&settings.s3.session_token)
        || optional_secret_present(&settings.gitee_snippet.access_token)
        || oauth_drive_settings_has_secret(&settings.google_drive)
        || oauth_drive_settings_has_secret(&settings.onedrive)
        || aliyun_drive_settings_has_secret(&settings.aliyun_drive)
        || optional_secret_present(&settings.github_gist.access_token)
}

fn oauth_drive_settings_has_secret(settings: &OAuthDriveSyncSettings) -> bool {
    optional_secret_present(&settings.access_token)
        || optional_secret_present(&settings.refresh_token)
        || optional_secret_present(&settings.client_secret)
}

fn aliyun_drive_settings_has_secret(settings: &AliyunDriveSyncSettings) -> bool {
    optional_secret_present(&settings.access_token)
        || optional_secret_present(&settings.refresh_token)
        || optional_secret_present(&settings.client_secret)
}

fn optional_secret_present(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|value| !value.is_empty())
}

fn ai_session_title(user_input: &str) -> String {
    let title = user_input.chars().take(42).collect::<String>();
    let title = title.trim();
    if title.is_empty() {
        "AI Session".to_string()
    } else {
        title.to_string()
    }
}

fn known_host_record_matches(record: &KnownHostRecord, host_identifier: &str) -> bool {
    let patterns = if record.host_patterns.is_empty() {
        std::slice::from_ref(&record.host_identifier)
    } else {
        record.host_patterns.as_slice()
    };
    let mut matched = false;
    for pattern in patterns {
        let (negated, pattern) = pattern
            .strip_prefix('!')
            .map_or((false, pattern.as_str()), |pattern| (true, pattern));
        if known_host_pattern_matches(pattern, host_identifier) {
            if negated {
                return false;
            }
            matched = true;
        }
    }
    matched
}

fn known_host_pattern_matches(pattern: &str, host_identifier: &str) -> bool {
    if pattern == host_identifier {
        return true;
    }
    if pattern.starts_with("|1|") {
        return hashed_known_host_matches(pattern, host_identifier);
    }
    false
}

fn hashed_known_host_matches(pattern: &str, host_identifier: &str) -> bool {
    let mut parts = pattern.split('|');
    if parts.next() != Some("") || parts.next() != Some("1") {
        return false;
    }
    let Some(salt_b64) = parts.next() else {
        return false;
    };
    let Some(hash_b64) = parts.next() else {
        return false;
    };
    if parts.next().is_some() {
        return false;
    }
    let Ok(salt) = B64.decode(salt_b64) else {
        return false;
    };
    let Ok(expected) = B64.decode(hash_b64) else {
        return false;
    };
    let Ok(mut mac) = HmacSha1::new_from_slice(&salt) else {
        return false;
    };
    mac.update(host_identifier.as_bytes());
    let actual = mac.finalize().into_bytes();
    expected.as_slice() == actual.as_slice()
}

fn known_host_key(record: &KnownHostRecord) -> String {
    let digest_input = format!(
        "{}|{}|{}",
        record.marker.as_deref().unwrap_or_default(),
        record.host_patterns.join(","),
        record.key_type
    );
    format!("{KNOWN_HOST_PREFIX}{}", stable_id(&digest_input))
}

fn render_known_host_record(record: &KnownHostRecord) -> String {
    let host_list = if record.host_patterns.is_empty() {
        record.host_identifier.clone()
    } else {
        record.host_patterns.join(",")
    };
    let mut line = String::new();
    if let Some(marker) = &record.marker {
        line.push_str(marker);
        line.push(' ');
    }
    line.push_str(&host_list);
    line.push(' ');
    line.push_str(&record.key_type);
    line.push(' ');
    line.push_str(&record.key_base64);
    if let Some(comment) = &record.comment {
        if !comment.is_empty() {
            line.push(' ');
            line.push_str(comment);
        }
    }
    line
}

fn write_json_in_txn<T>(
    txn: &redb::WriteTransaction,
    definition: TableDefinition<&str, &[u8]>,
    key: &str,
    value: &T,
) -> Result<(), StorageError>
where
    T: Serialize,
{
    let bytes = serde_json::to_vec(value)?;
    txn.open_table(definition)?.insert(key, bytes.as_slice())?;
    Ok(())
}

fn deserialize_json<T>(value: &[u8]) -> Result<T, StorageError>
where
    T: DeserializeOwned,
{
    Ok(serde_json::from_slice(value)?)
}

fn entity_key(prefix: &str, id: &str) -> String {
    format!("{prefix}{id}")
}

fn sort_connections(connections: &mut [SavedConnection]) {
    connections.sort_by(|left, right| {
        left.group_id
            .cmp(&right.group_id)
            .then(left.sort_order.cmp(&right.sort_order))
            .then(left.name.cmp(&right.name))
            .then(left.id.cmp(&right.id))
    });
}

fn default_settings_value() -> serde_json::Value {
    serde_json::json!({
        "general": {
            "startup_restore": false,
            "startup_restore_window_layout": true,
            "confirm_on_close": true
        },
        "appearance": {
            "theme": "github-dark",
            "font_family": "JetBrains Mono",
            "font_size": 16.0
        },
        "translation": {
            "target_language": "zh-CN",
            "deepl_api_key": "",
            "baidu_app_id": "",
            "baidu_app_key": "",
            "ali_app_id": "",
            "ali_app_key": "",
            "youdao_app_id": "",
            "youdao_app_key": ""
        },
        "security": {
            "use_os_keyring": true,
            "enable_screen_lock": false,
            "idle_lock_minutes": 0,
            "master_password": null,
            "host_key_policy": "prompt"
        },
        "transfer": {
            "duplicate_strategy": "ask"
        }
    })
}

fn json_u32_map(value: &serde_json::Value, path: &[&str]) -> HashMap<String, u32> {
    let mut map = HashMap::new();
    let Some(object) = json_path(value, path).and_then(|value| value.as_object()) else {
        return map;
    };
    for (key, raw) in object {
        let number = raw
            .as_u64()
            .or_else(|| raw.as_f64().map(|value| value.round() as u64));
        if let Some(number) = number {
            if number > 0 {
                map.insert(key.clone(), number.min(u32::MAX as u64) as u32);
            }
        }
    }
    map
}

fn u32_map_json_value(map: &HashMap<String, u32>) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    for (key, value) in map {
        if *value > 0 {
            object.insert(key.clone(), serde_json::json!(*value));
        }
    }
    serde_json::Value::Object(object)
}

fn json_optional_string(value: &serde_json::Value, path: &[&str]) -> Option<String> {
    json_path(value, path)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn json_string(value: &serde_json::Value, path: &[&str], fallback: &str) -> String {
    json_path(value, path)
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn json_bool(value: &serde_json::Value, path: &[&str], fallback: bool) -> bool {
    json_path(value, path)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(fallback)
}

fn json_string_map(value: &serde_json::Value, path: &[&str]) -> HashMap<String, String> {
    json_path(value, path)
        .and_then(serde_json::Value::as_object)
        .map(|object| {
            object
                .iter()
                .filter_map(|(key, value)| {
                    value
                        .as_str()
                        .filter(|value| !value.trim().is_empty())
                        .map(|value| (key.clone(), value.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn json_string_vec(value: &serde_json::Value, path: &[&str], limit: usize) -> Vec<String> {
    json_path(value, path)
        .and_then(serde_json::Value::as_array)
        .map(|array| {
            array
                .iter()
                .filter_map(|entry| {
                    entry
                        .as_str()
                        .map(str::trim)
                        .filter(|entry| !entry.is_empty())
                        .map(ToOwned::to_owned)
                })
                .fold(Vec::<String>::new(), |mut values, entry| {
                    if !values.iter().any(|existing| existing == &entry) {
                        values.push(entry);
                    }
                    values
                })
                .into_iter()
                .take(limit)
                .collect()
        })
        .unwrap_or_default()
}

fn json_string_vec_map(
    value: &serde_json::Value,
    path: &[&str],
    limit_per_key: usize,
) -> HashMap<String, Vec<String>> {
    json_path(value, path)
        .and_then(serde_json::Value::as_object)
        .map(|object| {
            object
                .iter()
                .filter_map(|(key, value)| {
                    let values = value
                        .as_array()?
                        .iter()
                        .filter_map(|entry| {
                            entry
                                .as_str()
                                .map(str::trim)
                                .filter(|entry| !entry.is_empty())
                                .map(ToOwned::to_owned)
                        })
                        .fold(Vec::<String>::new(), |mut values, entry| {
                            if !values.iter().any(|existing| existing == &entry) {
                                values.push(entry);
                            }
                            values
                        })
                        .into_iter()
                        .take(limit_per_key)
                        .collect::<Vec<_>>();
                    (!values.is_empty()).then(|| (key.clone(), values))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn string_vec_json_value(values: &[String], limit: usize) -> serde_json::Value {
    serde_json::Value::Array(
        values
            .iter()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .fold(Vec::<String>::new(), |mut values, value| {
                if !values.iter().any(|existing| existing == value) {
                    values.push(value.to_string());
                }
                values
            })
            .into_iter()
            .take(limit)
            .map(serde_json::Value::String)
            .collect(),
    )
}

fn string_vec_map_json_value(
    map: &HashMap<String, Vec<String>>,
    limit_per_key: usize,
) -> serde_json::Value {
    let object = map
        .iter()
        .filter_map(|(key, values)| {
            let key = key.trim();
            if key.is_empty() {
                return None;
            }
            let values = values
                .iter()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .fold(Vec::<String>::new(), |mut values, value| {
                    if !values.iter().any(|existing| existing == value) {
                        values.push(value.to_string());
                    }
                    values
                })
                .into_iter()
                .take(limit_per_key)
                .map(serde_json::Value::String)
                .collect::<Vec<_>>();
            (!values.is_empty()).then(|| (key.to_string(), serde_json::Value::Array(values)))
        })
        .collect();
    serde_json::Value::Object(object)
}

fn json_u16(value: &serde_json::Value, path: &[&str], fallback: u16) -> u16 {
    let Some(value) = json_path(value, path) else {
        return fallback;
    };
    if let Some(number) = value.as_u64() {
        return number.try_into().unwrap_or(fallback);
    }
    if let Some(number) = value.as_f64() {
        if number.is_finite() && number >= 0.0 && number <= f64::from(u16::MAX) {
            return number.round() as u16;
        }
    }
    fallback
}

fn json_u32(value: &serde_json::Value, path: &[&str], fallback: u32) -> u32 {
    let Some(value) = json_path(value, path) else {
        return fallback;
    };
    if let Some(number) = value.as_u64() {
        return number.try_into().unwrap_or(fallback);
    }
    if let Some(number) = value.as_f64() {
        if number.is_finite() && number >= 0.0 && number <= f64::from(u32::MAX) {
            return number.round() as u32;
        }
    }
    fallback
}

fn json_u64(value: &serde_json::Value, path: &[&str], fallback: u64) -> u64 {
    let Some(value) = json_path(value, path) else {
        return fallback;
    };
    if let Some(number) = value.as_u64() {
        return number;
    }
    if let Some(number) = value.as_f64() {
        if number.is_finite() && number >= 0.0 && number <= u64::MAX as f64 {
            return number.round() as u64;
        }
    }
    fallback
}

fn normalize_keyword_highlight_rule(
    mut rule: KeywordHighlightRule,
) -> Option<KeywordHighlightRule> {
    rule.id = rule.id.trim().to_string();
    rule.name = rule.name.trim().to_string();
    rule.patterns = rule
        .patterns
        .into_iter()
        .map(|pattern| pattern.trim().to_string())
        .filter(|pattern| !pattern.is_empty())
        .collect();
    if rule.name.is_empty() || rule.patterns.is_empty() {
        return None;
    }
    if rule.color_dark.trim().is_empty() {
        rule.color_dark = "#79c0ff".to_string();
    }
    if rule.color_light.trim().is_empty() {
        rule.color_light = "#0969da".to_string();
    }
    Some(rule)
}

fn merge_keyword_highlight_rules(
    existing: &mut Vec<KeywordHighlightRule>,
    imported: Vec<KeywordHighlightRule>,
) -> KeywordHighlightImportResult {
    let mut imported_rules = 0;
    let mut updated_rules = 0;
    let mut indexes = existing
        .iter()
        .enumerate()
        .filter_map(|(index, rule)| (!rule.id.trim().is_empty()).then(|| (rule.id.clone(), index)))
        .collect::<HashMap<_, _>>();

    for mut rule in imported
        .into_iter()
        .filter_map(normalize_keyword_highlight_rule)
    {
        if rule.id.is_empty() {
            rule.id = format!("highlight-{}", uuid());
        }
        if let Some(index) = indexes.get(&rule.id).copied() {
            existing[index] = rule;
            updated_rules += 1;
        } else {
            let id = rule.id.clone();
            existing.push(rule);
            indexes.insert(id, existing.len() - 1);
            imported_rules += 1;
        }
    }

    KeywordHighlightImportResult {
        imported_rules,
        updated_rules,
        total_rules: existing.len(),
    }
}

fn json_path<'a>(value: &'a serde_json::Value, path: &[&str]) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn set_nested_json_string(value: &mut serde_json::Value, path: &[&str], new_value: String) {
    set_nested_json_value(value, path, serde_json::Value::String(new_value));
}



fn load_action_links_matchers(value: &serde_json::Value) -> crate::ActionLinksMatcherSettings {
    let defaults = crate::ActionLinksMatcherSettings::default();
    let Some(obj) = json_path(value, &["terminal", "action_links_matchers"]) else {
        return defaults;
    };
    crate::ActionLinksMatcherSettings {
        ipv4: obj
            .get("ipv4")
            .and_then(|v| v.as_bool())
            .unwrap_or(defaults.ipv4),
        archive: obj
            .get("archive")
            .and_then(|v| v.as_bool())
            .unwrap_or(defaults.archive),
        host_port: obj
            .get("host_port")
            .and_then(|v| v.as_bool())
            .unwrap_or(defaults.host_port),
    }
}

fn load_search_engines(value: &serde_json::Value) -> Vec<SearchEngineConfig> {
    let Some(arr) = json_path(value, &["search", "custom_engines"]).and_then(|v| v.as_array()) else {
        return default_search_engines();
    };
    let mut engines = Vec::new();
    for item in arr {
        let name = item
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let url_template = item
            .get("url_template")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if name.is_empty() || url_template.is_empty() {
            continue;
        }
        let show_in_menu = item
            .get("show_in_menu")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let icon = item
            .get("icon")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string);
        engines.push(SearchEngineConfig {
            name,
            url_template,
            icon,
            show_in_menu,
        });
    }
    if engines.is_empty() {
        default_search_engines()
    } else {
        engines
    }
}

fn search_engines_to_json(engines: &[SearchEngineConfig]) -> serde_json::Value {
    serde_json::Value::Array(
        engines
            .iter()
            .map(|engine| {
                serde_json::json!({
                    "name": engine.name,
                    "url_template": engine.url_template,
                    "icon": engine.icon,
                    "show_in_menu": engine.show_in_menu,
                })
            })
            .collect(),
    )
}

fn set_nested_json_value(
    value: &mut serde_json::Value,
    path: &[&str],
    new_value: serde_json::Value,
) {
    if !value.is_object() {
        *value = serde_json::Value::Object(Default::default());
    }
    let mut current = value;
    for key in &path[..path.len().saturating_sub(1)] {
        if !current.get(*key).is_some_and(serde_json::Value::is_object) {
            current[*key] = serde_json::Value::Object(Default::default());
        }
        current = current.get_mut(*key).expect("object child exists");
    }
    if let Some(key) = path.last() {
        current[*key] = new_value;
    }
}

fn normalize_host_key_policy(policy: &str) -> String {
    match policy {
        "strict" | "accept" | "prompt" => policy.to_string(),
        _ => "prompt".to_string(),
    }
}

fn normalize_transfer_duplicate_strategy(strategy: &str) -> String {
    match strategy {
        "ask" | "overwrite" | "skip" | "rename" => strategy.to_string(),
        _ => "ask".to_string(),
    }
}

fn normalize_transfer_editor_type(editor_type: &str) -> String {
    match editor_type {
        "external" | "internal" => editor_type.to_string(),
        _ => "external".to_string(),
    }
}

fn normalize_transfer_file_permissions(value: &str) -> String {
    let trimmed = value
        .trim()
        .trim_start_matches("0o")
        .trim_start_matches('0');
    let normalized = if trimmed.is_empty() { "0" } else { trimmed };
    if (3..=4).contains(&normalized.len()) && normalized.chars().all(|ch| matches!(ch, '0'..='7')) {
        normalized.to_string()
    } else {
        "644".to_string()
    }
}

fn default_activity_left_top() -> Vec<String> {
    vec![
        "fileExplorer".to_string(),
        "network".to_string(),
        "securityAuth".to_string(),
    ]
}

fn default_activity_left_bottom() -> Vec<String> {
    vec!["syncBackupHistory".to_string(), "settings".to_string()]
}

fn default_activity_right_top() -> Vec<String> {
    vec![
        "savedConnections".to_string(),
        "aiAssistant".to_string(),
        "activeSessions".to_string(),
        "commandHistory".to_string(),
        "resourceMonitor".to_string(),
        "processManager".to_string(),
        "dockerManager".to_string(),
    ]
}

fn default_activity_right_bottom() -> Vec<String> {
    vec![
        "quickCmdBar".to_string(),
        "serialSend".to_string(),
        "recording".to_string(),
        "lock".to_string(),
    ]
}

fn normalize_quick_cmd_view_mode(value: &str) -> String {
    match value.trim() {
        "list" | "compact" | "tile" => value.trim().to_string(),
        _ => "tile".to_string(),
    }
}

fn normalize_quick_cmd_sort_mode(value: &str) -> String {
    match value.trim() {
        "created" | "name" | "useCount" => value.trim().to_string(),
        _ => "created".to_string(),
    }
}

fn normalize_tab_mouse_action(action: &str) -> String {
    match action {
        "none" | "rename_tab" | "copy_tab_name" | "copy_server_ip" | "duplicate_session"
        | "multiplex_ssh" | "reconnect_session" | "disconnect_session" | "close_tab" => {
            action.to_string()
        }
        _ => "none".to_string(),
    }
}

fn normalize_interaction_encoding(encoding: &str) -> String {
    if encoding.eq_ignore_ascii_case("gbk") {
        "GBK".to_string()
    } else {
        "UTF-8".to_string()
    }
}

fn padded_i64(value: i64) -> String {
    let shifted = i128::from(value) - i128::from(i64::MIN);
    format!("{shifted:020}")
}

fn stable_id(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let mut output = String::with_capacity(32);
    for byte in &digest[..16] {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn ensure_parent_dir(path: &Path) -> Result<(), StorageError> {
    let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    else {
        return Ok(());
    };
    std::fs::create_dir_all(parent).map_err(|source| StorageError::CreateDir {
        path: parent.to_path_buf(),
        source,
    })
}

fn write_portable_snapshot_file(
    database_path: PathBuf,
    output_path: impl AsRef<Path>,
    bytes: &[u8],
) -> Result<ConfigBackupInfo, StorageError> {
    let backup_path = output_path.as_ref().to_path_buf();
    ensure_parent_dir(&backup_path)?;
    std::fs::write(&backup_path, bytes).map_err(|source| StorageError::ConfigBackupCopy {
        from: database_path.clone(),
        to: backup_path.clone(),
        source,
    })?;

    Ok(ConfigBackupInfo {
        database_path,
        backup_path,
        bytes: bytes.len().try_into().unwrap_or(u64::MAX),
        safety_backup_path: None,
    })
}

fn validate_config_backup_source(path: &Path) -> Result<(), StorageError> {
    if !path.exists() {
        return Err(StorageError::ConfigBackupMissing {
            path: path.to_path_buf(),
        });
    }
    if !path.is_file() {
        return Err(StorageError::ConfigBackupNotFile {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

fn ensure_not_same_existing_file(left: &Path, right: &Path) -> Result<(), StorageError> {
    if !left.exists() || !right.exists() {
        return Ok(());
    }
    let left = left.canonicalize().ok();
    let right = right.canonicalize().ok();
    if let (Some(left), Some(right)) = (left, right)
        && left == right
    {
        return Err(StorageError::ConfigBackupSamePath { path: left });
    }
    Ok(())
}

fn copy_config_database(from: &Path, to: &Path) -> Result<u64, StorageError> {
    validate_config_backup_source(from)?;
    ensure_parent_dir(to)?;
    std::fs::copy(from, to).map_err(|source| StorageError::ConfigBackupCopy {
        from: from.to_path_buf(),
        to: to.to_path_buf(),
        source,
    })
}

fn validate_config_backup_file(
    source: &Path,
    portable_key_path: Option<PathBuf>,
) -> Result<(), StorageError> {
    let validation_dir = std::env::temp_dir().join(format!(
        "nyaterm-config-backup-validate-{}-{}-{}",
        std::process::id(),
        current_time_ms(),
        uuid()
    ));
    std::fs::create_dir_all(&validation_dir).map_err(|source| StorageError::CreateDir {
        path: validation_dir.clone(),
        source,
    })?;
    let validation_db = validation_dir.join(DATABASE_FILE);
    copy_config_database(source, &validation_db)?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        || -> Result<(), StorageError> {
            let store =
                ConnectionStore::open_with_portable_key_path(&validation_dir, portable_key_path)?;
            store.load_sessions()?;
            store.load_app_settings_summary()?;
            store.list_tunnels()?;
            drop(store);
            Ok(())
        },
    ))
    .map_err(|_| {
        StorageError::InvalidData(format!(
            "configuration backup is not a valid redb database: {}",
            source.display()
        ))
    })?;
    std::fs::remove_dir_all(validation_dir).ok();
    result
}

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AiExecutionProfile, ConnectionAuth, ConnectionType};
    use aes_gcm::{Aes256Gcm, Key, KeyInit, aead::Aead};
    use base64::{Engine, engine::general_purpose::STANDARD as B64};
    use sha2::{Digest, Sha256};

    #[test]
    fn round_trips_sessions_in_redb_compatible_tables() {
        let dir = unique_temp_dir("round-trip");
        let store = ConnectionStore::open(&dir).expect("store");
        let config = SessionsConfig {
            groups: vec![Group {
                id: "group-1".to_string(),
                name: "Servers".to_string(),
                parent_id: None,
                sort_order: 0,
                created_at_ms: None,
                updated_at_ms: None,
            }],
            connections: vec![SavedConnection {
                id: "conn-1".to_string(),
                name: "Production".to_string(),
                config: ConnectionType::Ssh {
                    host: "10.0.0.8".to_string(),
                    port: 22,
                    username: "root".to_string(),
                    backspace_mode: "del".to_string(),
                    ai_execution_profile: AiExecutionProfile::Auto,
                    x11_forwarding: false,
                },
                group_id: Some("group-1".to_string()),
                description: Some("Primary".to_string()),
                sort_order: 0,
                icon: None,
                auth: Some(ConnectionAuth {
                    mode: "password".to_string(),
                    password: Some("secret".to_string()),
                    ..Default::default()
                }),
                network: None,
                post_login: None,
                created_at_ms: None,
                updated_at_ms: None,
                last_used_at_ms: None,
            }],
        };

        store.replace_sessions(&config).expect("replace");
        let loaded = store.load_sessions().expect("load");

        assert_eq!(loaded.groups.len(), 1);
        assert_eq!(loaded.connections.len(), 1);
        assert_eq!(loaded.connections[0].endpoint(), "root@10.0.0.8:22");
        assert_eq!(
            loaded.connections[0]
                .auth
                .as_ref()
                .and_then(|auth| auth.password.as_deref()),
            Some("secret")
        );
        assert!(
            loaded.connections[0]
                .auth
                .as_ref()
                .is_some_and(|auth| auth.has_password)
        );

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn exports_and_imports_native_redb_backup() {
        let source_dir = unique_temp_dir("backup-source");
        let target_dir = unique_temp_dir("backup-target");
        let backup_path = unique_temp_dir("backup-output").join("nyaterm.redb");
        let source_store = ConnectionStore::open(&source_dir).expect("source store");
        let config = SessionsConfig {
            groups: vec![Group {
                id: "ops".to_string(),
                name: "Ops".to_string(),
                parent_id: None,
                sort_order: 0,
                created_at_ms: None,
                updated_at_ms: None,
            }],
            connections: vec![SavedConnection {
                id: "local-1".to_string(),
                name: "Shell".to_string(),
                config: ConnectionType::LocalTerminal {
                    shell_path: "/bin/sh".to_string(),
                    shell_args: String::new(),
                    working_dir: Some("/tmp".to_string()),
                    ai_execution_profile: AiExecutionProfile::Auto,
                },
                group_id: Some("ops".to_string()),
                description: None,
                sort_order: 0,
                icon: None,
                auth: None,
                network: None,
                post_login: None,
                created_at_ms: None,
                updated_at_ms: None,
                last_used_at_ms: None,
            }],
        };
        source_store.replace_sessions(&config).expect("seed source");
        drop(source_store);

        let export = ConnectionStore::export_config_database(&source_dir, None, &backup_path)
            .expect("export backup");
        assert_eq!(export.backup_path, backup_path);
        assert!(export.bytes > 0);

        let target_store = ConnectionStore::open(&target_dir).expect("target store");
        target_store
            .replace_sessions(&SessionsConfig {
                groups: Vec::new(),
                connections: Vec::new(),
            })
            .expect("seed target");
        drop(target_store);

        let import = ConnectionStore::import_config_database(&target_dir, None, &backup_path)
            .expect("import backup");
        assert!(import.safety_backup_path.is_some());
        let loaded = ConnectionStore::open(&target_dir)
            .expect("open imported")
            .load_sessions()
            .expect("load imported");
        assert_eq!(loaded.groups.len(), 1);
        assert_eq!(loaded.connections.len(), 1);
        assert_eq!(loaded.connections[0].name, "Shell");

        std::fs::remove_dir_all(source_dir).ok();
        std::fs::remove_dir_all(target_dir).ok();
        if let Some(parent) = backup_path.parent() {
            std::fs::remove_dir_all(parent).ok();
        }
    }

    #[test]
    fn exports_and_imports_portable_snapshot() {
        let source_dir = unique_temp_dir("portable-source");
        let target_dir = unique_temp_dir("portable-target");
        let snapshot_path = unique_temp_dir("portable-output").join("nyaterm.nya");

        let source_store = ConnectionStore::open(&source_dir).expect("source store");
        source_store
            .replace_sessions(&SessionsConfig {
                groups: vec![Group {
                    id: "group-1".to_string(),
                    name: "Servers".to_string(),
                    parent_id: None,
                    sort_order: 0,
                    created_at_ms: None,
                    updated_at_ms: None,
                }],
                connections: vec![SavedConnection {
                    id: "conn-1".to_string(),
                    name: "Production".to_string(),
                    config: ConnectionType::Ssh {
                        host: "10.0.0.8".to_string(),
                        port: 22,
                        username: "deploy".to_string(),
                        backspace_mode: "del".to_string(),
                        ai_execution_profile: AiExecutionProfile::Auto,
                        x11_forwarding: true,
                    },
                    group_id: Some("group-1".to_string()),
                    description: Some("Primary".to_string()),
                    sort_order: 0,
                    icon: None,
                    auth: Some(ConnectionAuth {
                        mode: "password".to_string(),
                        password: Some("session-secret".to_string()),
                        ..Default::default()
                    }),
                    network: None,
                    post_login: None,
                    created_at_ms: None,
                    updated_at_ms: None,
                    last_used_at_ms: None,
                }],
            })
            .expect("seed sessions");
        source_store
            .replace_tunnels(&[TunnelConfig {
                id: "tun-1".to_string(),
                name: "DB".to_string(),
                tunnel_type: "local".to_string(),
                connection_id: Some("conn-1".to_string()),
                listen_port: 15432,
                target_host: "127.0.0.1".to_string(),
                target_port: 5432,
                is_open: false,
                auto_open: true,
                bind_localhost: true,
                group_id: Some("tg-1".to_string()),
            }])
            .expect("seed tunnels");
        source_store
            .replace_tunnel_groups(&[TunnelGroup {
                id: "tg-1".to_string(),
                name: "Databases".to_string(),
                sort_order: 2,
            }])
            .expect("seed tunnel groups");
        source_store
            .replace_known_hosts_export("example.com ssh-ed25519 AAAA\n")
            .expect("seed known hosts");
        source_store
            .save_settings_value(&serde_json::json!({
                "appearance": {
                    "theme": "github-light",
                    "font_family": "Berkeley Mono",
                    "font_size": 14
                },
                "security": {
                    "master_password": "source-local-secret",
                    "host_key_policy": "strict"
                },
                "transfer": {
                    "duplicate_strategy": "rename"
                }
            }))
            .expect("seed settings");
        {
            let txn = source_store.db.begin_write().expect("txn");
            write_json_in_txn(
                &txn,
                CREDENTIALS_TABLE,
                &entity_key(SSH_KEY_PREFIX, "key-1"),
                &SshKey {
                    id: "key-1".to_string(),
                    name: "Deploy".to_string(),
                    key: Some("encrypted-key".to_string()),
                    cert: None,
                    passphrase: None,
                    key_file_path: None,
                    cert_file_path: None,
                    has_key_data: false,
                    has_cert_data: false,
                },
            )
            .expect("write key");
            txn.commit().expect("commit");
        }
        drop(source_store);

        let target_store = ConnectionStore::open(&target_dir).expect("target store");
        target_store
            .save_settings_value(&serde_json::json!({
                "security": {
                    "master_password": "target-local-secret",
                    "host_key_policy": "prompt"
                }
            }))
            .expect("seed target settings");
        drop(target_store);

        let export = ConnectionStore::export_portable_snapshot(
            &source_dir,
            None,
            &snapshot_path,
            "dev",
            "1",
        )
        .expect("export snapshot");
        assert_eq!(export.backup_path, snapshot_path);
        assert!(snapshot_path.exists());

        let import = ConnectionStore::import_portable_snapshot(&target_dir, None, &snapshot_path)
            .expect("import snapshot");
        assert!(import.safety_backup_path.is_some());

        let imported = ConnectionStore::open(&target_dir).expect("imported store");
        let sessions = imported.load_sessions().expect("load sessions");
        assert_eq!(sessions.connections.len(), 1);
        assert_eq!(sessions.connections[0].endpoint(), "deploy@10.0.0.8:22");
        assert_eq!(
            sessions.connections[0]
                .auth
                .as_ref()
                .and_then(|auth| auth.password.as_deref()),
            Some("session-secret")
        );
        assert_eq!(imported.list_ssh_keys().expect("keys")[0].name, "Deploy");
        assert_eq!(imported.list_tunnels().expect("tunnels")[0].name, "DB");
        assert_eq!(
            imported.list_tunnel_groups().expect("tunnel groups")[0].name,
            "Databases"
        );
        assert_eq!(
            imported.render_known_hosts_export().expect("known hosts"),
            "example.com ssh-ed25519 AAAA\n"
        );
        let settings = imported.load_settings_value().expect("settings");
        assert_eq!(
            json_path(&settings, &["appearance", "theme"]).and_then(serde_json::Value::as_str),
            Some("github-light")
        );
        assert_eq!(
            json_path(&settings, &["security", "master_password"])
                .and_then(serde_json::Value::as_str),
            Some("target-local-secret")
        );

        std::fs::remove_dir_all(source_dir).ok();
        std::fs::remove_dir_all(target_dir).ok();
        if let Some(parent) = snapshot_path.parent() {
            std::fs::remove_dir_all(parent).ok();
        }
    }

    #[test]
    fn encrypted_portable_snapshot_requires_master_password() {
        let source_dir = unique_temp_dir("portable-encrypted-source");
        let target_dir = unique_temp_dir("portable-encrypted-target");
        let wrong_target_dir = unique_temp_dir("portable-encrypted-wrong-target");
        let snapshot_path = unique_temp_dir("portable-encrypted-output").join("nyaterm.nya");

        let source_store = ConnectionStore::open(&source_dir).expect("source store");
        source_store
            .replace_sessions(&SessionsConfig {
                groups: Vec::new(),
                connections: vec![SavedConnection {
                    id: "conn-1".to_string(),
                    name: "Encrypted Snapshot".to_string(),
                    config: ConnectionType::LocalTerminal {
                        shell_path: "bash".to_string(),
                        shell_args: String::new(),
                        working_dir: None,
                        ai_execution_profile: AiExecutionProfile::Auto,
                    },
                    group_id: None,
                    description: None,
                    sort_order: 0,
                    icon: None,
                    auth: None,
                    network: None,
                    post_login: None,
                    created_at_ms: None,
                    updated_at_ms: None,
                    last_used_at_ms: None,
                }],
            })
            .expect("seed source");
        drop(source_store);

        assert!(
            ConnectionStore::export_encrypted_portable_snapshot(
                &source_dir,
                None,
                &snapshot_path,
                "dev",
                "1",
                "",
            )
            .is_err()
        );

        ConnectionStore::export_encrypted_portable_snapshot(
            &source_dir,
            None,
            &snapshot_path,
            "dev",
            "1",
            "secret",
        )
        .expect("export encrypted snapshot");
        assert!(
            crate::portable_snapshot::decode_raw_portable_snapshot(
                &std::fs::read(&snapshot_path).expect("read encrypted snapshot")
            )
            .is_err()
        );

        let wrong_target = ConnectionStore::open(&wrong_target_dir).expect("wrong target");
        wrong_target
            .replace_sessions(&SessionsConfig {
                groups: Vec::new(),
                connections: vec![SavedConnection {
                    id: "keep".to_string(),
                    name: "Keep".to_string(),
                    config: ConnectionType::LocalTerminal {
                        shell_path: "zsh".to_string(),
                        shell_args: String::new(),
                        working_dir: None,
                        ai_execution_profile: AiExecutionProfile::Auto,
                    },
                    group_id: None,
                    description: None,
                    sort_order: 0,
                    icon: None,
                    auth: None,
                    network: None,
                    post_login: None,
                    created_at_ms: None,
                    updated_at_ms: None,
                    last_used_at_ms: None,
                }],
            })
            .expect("seed wrong target");
        drop(wrong_target);
        assert!(
            ConnectionStore::import_encrypted_portable_snapshot(
                &wrong_target_dir,
                None,
                &snapshot_path,
                "wrong",
            )
            .is_err()
        );
        let preserved = ConnectionStore::open(&wrong_target_dir)
            .expect("open wrong target")
            .load_sessions()
            .expect("load preserved");
        assert_eq!(preserved.connections[0].name, "Keep");

        ConnectionStore::import_encrypted_portable_snapshot(
            &target_dir,
            None,
            &snapshot_path,
            "secret",
        )
        .expect("import encrypted snapshot");
        let imported = ConnectionStore::open(&target_dir)
            .expect("open imported")
            .load_sessions()
            .expect("load imported");
        assert_eq!(imported.connections[0].name, "Encrypted Snapshot");

        std::fs::remove_dir_all(source_dir).ok();
        std::fs::remove_dir_all(target_dir).ok();
        std::fs::remove_dir_all(wrong_target_dir).ok();
        if let Some(parent) = snapshot_path.parent() {
            std::fs::remove_dir_all(parent).ok();
        }
    }

    #[test]
    fn rejects_invalid_backup_without_replacing_current_database() {
        let target_dir = unique_temp_dir("backup-reject-target");
        let invalid_dir = unique_temp_dir("backup-reject-invalid");
        std::fs::create_dir_all(&invalid_dir).expect("invalid dir");
        let invalid_path = invalid_dir.join("not-redb.redb");
        std::fs::write(&invalid_path, b"not a redb database").expect("write invalid");
        let store = ConnectionStore::open(&target_dir).expect("target store");
        store
            .replace_sessions(&SessionsConfig {
                groups: Vec::new(),
                connections: vec![SavedConnection {
                    id: "keep".to_string(),
                    name: "Keep".to_string(),
                    config: ConnectionType::LocalTerminal {
                        shell_path: String::new(),
                        shell_args: String::new(),
                        working_dir: None,
                        ai_execution_profile: AiExecutionProfile::Auto,
                    },
                    group_id: None,
                    description: None,
                    sort_order: 0,
                    icon: None,
                    auth: None,
                    network: None,
                    post_login: None,
                    created_at_ms: None,
                    updated_at_ms: None,
                    last_used_at_ms: None,
                }],
            })
            .expect("seed target");
        drop(store);

        assert!(ConnectionStore::import_config_database(&target_dir, None, &invalid_path).is_err());
        let loaded = ConnectionStore::open(&target_dir)
            .expect("open target")
            .load_sessions()
            .expect("load target");
        assert_eq!(loaded.connections.len(), 1);
        assert_eq!(loaded.connections[0].name, "Keep");

        std::fs::remove_dir_all(target_dir).ok();
        std::fs::remove_dir_all(invalid_dir).ok();
    }

    #[test]
    fn load_tunnels_reads_legacy_tunnel_table() {
        let dir = unique_temp_dir("tunnels");
        let store = ConnectionStore::open(&dir).expect("store");
        let tunnel = TunnelConfig {
            id: "tunnel-1".to_string(),
            name: "Local Web".to_string(),
            tunnel_type: "local".to_string(),
            connection_id: Some("conn-1".to_string()),
            listen_port: 8080,
            target_host: "127.0.0.1".to_string(),
            target_port: 80,
            is_open: true,
            auto_open: true,
            bind_localhost: true,
            group_id: None,
        };
        let txn = store.db.begin_write().expect("txn");
        write_json_in_txn(&txn, TUNNELS_TABLE, "tunnels/tunnel-1", &tunnel).expect("write tunnel");
        txn.commit().expect("commit");

        let loaded = store.list_tunnels().expect("tunnels");
        assert_eq!(loaded, vec![tunnel]);

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn save_and_delete_connection_updates_store() {
        let dir = unique_temp_dir("delete");
        let store = ConnectionStore::open(&dir).expect("store");
        let connection = SavedConnection {
            id: "local-1".to_string(),
            name: "Local".to_string(),
            config: ConnectionType::LocalTerminal {
                shell_path: "bash".to_string(),
                shell_args: String::new(),
                working_dir: None,
                ai_execution_profile: Default::default(),
            },
            group_id: None,
            description: None,
            sort_order: 0,
            icon: None,
            auth: None,
            network: None,
            post_login: None,
            created_at_ms: None,
            updated_at_ms: None,
            last_used_at_ms: None,
        };

        store.save_connection(&connection).expect("save");
        assert!(store.get_connection("local-1").expect("get").is_some());
        store.delete_connection("local-1").expect("delete");
        assert!(store.get_connection("local-1").expect("missing").is_none());

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn load_sessions_decrypts_legacy_connection_password_record() {
        let dir = unique_temp_dir("decrypt-password");
        let store = ConnectionStore::open(&dir).expect("store");
        let connection = SavedConnection {
            id: "ssh-1".to_string(),
            name: "SSH".to_string(),
            config: ConnectionType::Ssh {
                host: "127.0.0.1".to_string(),
                port: 22,
                username: "root".to_string(),
                backspace_mode: "del".to_string(),
                ai_execution_profile: AiExecutionProfile::Auto,
                x11_forwarding: false,
            },
            group_id: None,
            description: None,
            sort_order: 0,
            icon: None,
            auth: Some(ConnectionAuth {
                mode: "password".to_string(),
                ..Default::default()
            }),
            network: None,
            post_login: None,
            created_at_ms: None,
            updated_at_ms: None,
            last_used_at_ms: None,
        };
        store.save_connection(&connection).expect("save connection");

        let master_key = test_key(3);
        let master_key_token = encrypt_for_test(master_key.as_slice(), &home_wrapping_key());
        let encrypted_password = encrypt_for_test(b"legacy-secret", &master_key);
        let now = current_time_ms();
        let record = ConnectionPasswordRecord {
            id: "ssh-1".to_string(),
            connection_id: "ssh-1".to_string(),
            password: encrypted_password.clone(),
            created_at_ms: now,
            updated_at_ms: now,
        };
        {
            let txn = store.db.begin_write().expect("txn");
            txn.open_table(META_TABLE)
                .expect("meta")
                .insert(META_MASTER_KEY, master_key_token.as_str())
                .expect("insert master");
            write_json_in_txn(
                &txn,
                CREDENTIALS_TABLE,
                &entity_key(CONNECTION_PASSWORD_PREFIX, "ssh-1"),
                &record,
            )
            .expect("write credential");
            txn.commit().expect("commit");
        }

        let loaded = store.load_sessions().expect("load sessions");
        let auth = loaded.connections[0].auth.as_ref().expect("auth");
        assert_eq!(auth.password.as_deref(), Some("legacy-secret"));
        assert!(!auth.has_password);

        store.mark_connection_used("ssh-1").expect("mark used");
        let stored_record: ConnectionPasswordRecord = {
            let txn = store.db.begin_read().expect("txn");
            let table = txn.open_table(CREDENTIALS_TABLE).expect("credentials");
            let raw = table
                .get(entity_key(CONNECTION_PASSWORD_PREFIX, "ssh-1").as_str())
                .expect("get")
                .expect("record");
            deserialize_json(raw.value()).expect("deserialize")
        };
        assert_eq!(stored_record.password, encrypted_password);

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn load_decrypted_ssh_key_reads_legacy_key_store() {
        let dir = unique_temp_dir("decrypt-ssh-key");
        let store = ConnectionStore::open(&dir).expect("store");
        let master_key = test_key(7);
        let master_key_token = encrypt_for_test(master_key.as_slice(), &home_wrapping_key());
        let key = SshKey {
            id: "key-1".to_string(),
            name: "Deploy Key".to_string(),
            key: Some(encrypt_for_test(
                b"-----BEGIN PRIVATE KEY-----",
                &master_key,
            )),
            cert: Some(encrypt_for_test(
                b"ssh-ed25519-cert-v01@openssh.com AAAA",
                &master_key,
            )),
            passphrase: Some(encrypt_for_test(b"passphrase", &master_key)),
            key_file_path: None,
            cert_file_path: None,
            has_key_data: false,
            has_cert_data: false,
        };
        {
            let txn = store.db.begin_write().expect("txn");
            txn.open_table(META_TABLE)
                .expect("meta")
                .insert(META_MASTER_KEY, master_key_token.as_str())
                .expect("insert master");
            write_json_in_txn(
                &txn,
                CREDENTIALS_TABLE,
                &entity_key(SSH_KEY_PREFIX, "key-1"),
                &key,
            )
            .expect("write key");
            txn.commit().expect("commit");
        }

        let listed = store.list_ssh_keys().expect("list keys");
        assert_eq!(listed.len(), 1);
        assert!(listed[0].has_key_data);
        assert!(listed[0].has_cert_data);

        let decrypted = store
            .load_decrypted_ssh_key_by_id("key-1")
            .expect("decrypt key")
            .expect("key");
        assert_eq!(decrypted.name, "Deploy Key");
        assert_eq!(
            decrypted.key_data.as_deref(),
            Some("-----BEGIN PRIVATE KEY-----")
        );
        assert_eq!(
            decrypted.cert_data.as_deref(),
            Some("ssh-ed25519-cert-v01@openssh.com AAAA")
        );
        assert_eq!(decrypted.passphrase.as_deref(), Some("passphrase"));

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn load_decrypted_otp_entry_reads_legacy_otp_store() {
        let dir = unique_temp_dir("decrypt-otp");
        let store = ConnectionStore::open(&dir).expect("store");
        let master_key = test_key(8);
        let master_key_token = encrypt_for_test(master_key.as_slice(), &home_wrapping_key());
        let entry = OtpEntry {
            id: "otp-1".to_string(),
            otp_type: "totp".to_string(),
            issuer: "Example".to_string(),
            username: "deploy".to_string(),
            secret: Some(encrypt_for_test(b"JBSWY3DPEHPK3PXP", &master_key)),
            algorithm: "SHA1".to_string(),
            digits: 6,
            period: 30,
            counter: 0,
            has_secret: false,
        };
        {
            let txn = store.db.begin_write().expect("txn");
            txn.open_table(META_TABLE)
                .expect("meta")
                .insert(META_MASTER_KEY, master_key_token.as_str())
                .expect("insert master");
            write_json_in_txn(
                &txn,
                OTP_ACCOUNTS_TABLE,
                &entity_key(OTP_PREFIX, "otp-1"),
                &entry,
            )
            .expect("write otp");
            txn.commit().expect("commit");
        }

        let listed = store.list_otp_entries().expect("list");
        assert_eq!(listed.len(), 1);
        assert!(listed[0].has_secret);
        let decrypted = store
            .load_decrypted_otp_entry_by_id("otp-1")
            .expect("decrypt")
            .expect("entry");
        assert_eq!(decrypted.secret.as_deref(), Some("JBSWY3DPEHPK3PXP"));
        assert_eq!(decrypted.period, 30);
        store.increment_otp_counter("otp-1").expect("increment");
        let incremented = store
            .load_otp_entry_by_id("otp-1")
            .expect("load incremented")
            .expect("entry");
        assert_eq!(incremented.counter, 1);

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn known_hosts_repository_preserves_structured_hashed_and_raw_lines() {
        let dir = unique_temp_dir("known-hosts");
        let store = ConnectionStore::open(&dir).expect("store");
        store
            .replace_known_hosts_export(
                "# comment\n@cert-authority *.example.com ssh-ed25519 AAAA ca\n|1|nNMSH1CuL4w6FneDFn3ONf5paeg=|q8MlMsHsBk6GOpNwYqhnCeXKlRk= ssh-rsa BBBB\n",
            )
            .expect("save known hosts");

        let rendered = store.render_known_hosts_export().expect("render");
        assert!(rendered.contains("# comment"));
        assert!(rendered.contains("@cert-authority *.example.com ssh-ed25519 AAAA ca"));
        assert!(
            rendered.contains(
                "|1|nNMSH1CuL4w6FneDFn3ONf5paeg=|q8MlMsHsBk6GOpNwYqhnCeXKlRk= ssh-rsa BBBB"
            )
        );

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn known_hosts_check_distinguishes_match_changed_and_unknown() {
        let dir = unique_temp_dir("known-hosts-check");
        let store = ConnectionStore::open(&dir).expect("store");
        store
            .upsert_known_host("example.com ssh-ed25519 AAAA")
            .expect("known host");

        assert_eq!(
            store
                .check_known_host("example.com", "ssh-ed25519", "AAAA")
                .expect("match"),
            KnownHostCheck::Match
        );
        assert_eq!(
            store
                .check_known_host("example.com", "ssh-ed25519", "BBBB")
                .expect("changed"),
            KnownHostCheck::HostSeen
        );
        assert_eq!(
            store
                .check_known_host("other.example.com", "ssh-ed25519", "AAAA")
                .expect("unknown"),
            KnownHostCheck::UnknownHost
        );

        store
            .replace_known_host_for_host("example.com", "example.com ssh-ed25519 CCCC")
            .expect("replace");
        assert_eq!(
            store
                .check_known_host("example.com", "ssh-ed25519", "CCCC")
                .expect("replaced"),
            KnownHostCheck::Match
        );

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn imports_legacy_text_doc_known_hosts() {
        let dir = unique_temp_dir("legacy-known-hosts");
        std::fs::create_dir_all(&dir).expect("temp dir");
        {
            let db = Database::create(dir.join(DATABASE_FILE)).expect("db");
            let txn = db.begin_write().expect("txn");
            txn.open_table(TEXT_DOCS_TABLE)
                .expect("text docs")
                .insert(LEGACY_TEXT_KNOWN_HOSTS, "legacy.example.com ssh-rsa AAAA")
                .expect("insert");
            txn.commit().expect("commit");
        }

        let store = ConnectionStore::open(&dir).expect("store");
        assert_eq!(
            store
                .check_known_host("legacy.example.com", "ssh-rsa", "AAAA")
                .expect("legacy"),
            KnownHostCheck::Match
        );

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn app_settings_summary_reads_and_updates_host_key_policy() {
        let dir = unique_temp_dir("settings-summary");
        let store = ConnectionStore::open(&dir).expect("store");
        let initial = serde_json::json!({
            "general": {
                "startup_restore": true,
                "confirm_on_close": false,
                "custom_general": "keep"
            },
            "appearance": {
                "theme": "catppuccin",
                "font_family": "Iosevka",
                "font_size": 14.0
            },
            "translation": {
                "target_language": "ja"
            },
            "security": {
                "host_key_policy": "strict",
                "enable_screen_lock": true,
                "idle_lock_minutes": 12,
                "master_password": "encrypted"
            },
            "transfer": {
                "download_path": "/tmp/downloads",
                "ask_save_location": true,
                "duplicate_strategy": "rename",
                "editor_type": "internal",
                "default_editor": "code",
                "download_threads": 5,
                "upload_threads": 4,
                "max_transfer_retries": 6,
                "transfer_buffer_size": 64,
                "default_file_permissions": "664",
                "preserve_timestamps": false,
                "resume_broken_transfer": false,
                "recording_path": "/tmp/nyaterm-recordings",
                "recording_auto_start": true,
                "recording_include_io_labels": false,
                "recording_include_timestamps": false,
                "recording_memory_limit_bytes": 1048576
            },
            "terminal": {
                "x11_display": "localhost:1",
                "scrollback_lines": 8000,
                "keep_alive_interval": 45,
                "hardware_acceleration": false,
                "show_workspace_padding": true,
                "show_line_numbers": true,
                "show_timestamps": true,
                "show_timestamp_milliseconds": true,
                "show_multi_line_paste_dialog": false,
                "paste_image_as_path": false
            },
            "ui": {
                "show_remote_stats": false,
                "remote_stats_interval": 9,
                "show_process_manager": false,
                "process_manager_interval": 11,
                "show_docker_manager": false,
                "docker_manager_interval": 13,
                "quick_cmd_view_mode": "compact",
                "quick_cmd_sort_mode": "useCount",
                "file_explorer_auto_sync_cwd_connection_ids": ["conn-1", "conn-1", " ", "conn-2"],
                "file_explorer_favorite_dirs_by_connection_id": {
                    "conn-1": ["/var", "/var", " ", "/opt", "/srv", "/tmp", "/home", "/etc", "/usr", "/bin", "/sbin", "/lib", "/mnt"],
                    "conn-empty": [],
                    "conn-invalid": false
                }
            },
            "interaction": {
                "copy_on_select": true,
                "right_click_paste": true,
                "command_suggestions_enabled": false,
                "command_suggestion_min_chars": 3,
                "command_suggestion_max_chars": 80,
                "word_separators": " .,:",
                "duplicate_session_command_delay_ms": 1500,
                "alt_as_meta": true,
                "mac_ime_compatibility": false,
                "tab_double_click_action": "duplicate_session",
                "tab_middle_click_action": "close_tab",
                "tab_right_click_action": "copy_tab_name",
                "default_encoding": "GBK"
            },
            "diagnostics": {
                "level": "debug",
                "retention_days": 3
            },
            "keybindings": {
                "terminal.find": "ctrl+f",
                "ignored_non_string": false
            },
            "unrelated": {
                "preserve": true
            }
        });
        store.save_settings_value(&initial).expect("seed settings");

        let summary = store.load_app_settings_summary().expect("summary");
        assert_eq!(summary.theme, "catppuccin");
        assert_eq!(summary.language, "ja");
        assert_eq!(summary.terminal_font_family, "Iosevka");
        assert_eq!(summary.terminal_font_size, 14);
        assert_eq!(summary.x11_display, "localhost:1");
        assert_eq!(summary.terminal_scrollback_lines, 8000);
        assert_eq!(summary.terminal_keep_alive_interval, 45);
        assert!(!summary.terminal_hardware_acceleration);
        assert!(summary.terminal_show_workspace_padding);
        assert!(summary.terminal_show_line_numbers);
        assert!(summary.terminal_show_timestamps);
        assert!(summary.terminal_show_timestamp_milliseconds);
        assert!(!summary.terminal_show_multi_line_paste_dialog);
        assert!(!summary.terminal_paste_image_as_path);
        assert!(!summary.ui_show_remote_stats);
        assert_eq!(summary.ui_remote_stats_interval, 9);
        assert!(!summary.ui_show_process_manager);
        assert_eq!(summary.ui_process_manager_interval, 11);
        assert!(!summary.ui_show_docker_manager);
        assert_eq!(summary.ui_docker_manager_interval, 13);
        assert_eq!(summary.ui_quick_cmd_view_mode, "compact");
        assert_eq!(summary.ui_quick_cmd_sort_mode, "useCount");
        assert_eq!(
            summary.ui_file_explorer_auto_sync_cwd_connection_ids,
            vec!["conn-1".to_string(), "conn-2".to_string()]
        );
        assert_eq!(
            summary
                .ui_file_explorer_favorite_dirs_by_connection_id
                .get("conn-1"),
            Some(&vec![
                "/var".to_string(),
                "/opt".to_string(),
                "/srv".to_string(),
                "/tmp".to_string(),
                "/home".to_string(),
                "/etc".to_string(),
                "/usr".to_string(),
                "/bin".to_string(),
                "/sbin".to_string(),
                "/lib".to_string(),
                "/mnt".to_string(),
            ])
        );
        assert!(
            !summary
                .ui_file_explorer_favorite_dirs_by_connection_id
                .contains_key("conn-empty")
        );
        assert!(summary.interaction_copy_on_select);
        assert!(summary.interaction_right_click_paste);
        assert!(!summary.interaction_command_suggestions_enabled);
        assert_eq!(summary.interaction_command_suggestion_min_chars, 3);
        assert_eq!(summary.interaction_command_suggestion_max_chars, 80);
        assert_eq!(summary.interaction_word_separators, " .,:");
        assert_eq!(summary.interaction_duplicate_session_command_delay_ms, 1500);
        assert!(summary.interaction_alt_as_meta);
        assert!(!summary.interaction_mac_ime_compatibility);
        assert_eq!(
            summary.interaction_tab_double_click_action,
            "duplicate_session"
        );
        assert_eq!(summary.interaction_tab_middle_click_action, "close_tab");
        assert_eq!(summary.interaction_tab_right_click_action, "copy_tab_name");
        assert_eq!(summary.interaction_default_encoding, "GBK");
        assert_eq!(summary.host_key_policy, "strict");
        assert_eq!(summary.transfer_download_path, "/tmp/downloads");
        assert!(summary.transfer_ask_save_location);
        assert_eq!(summary.transfer_duplicate_strategy, "rename");
        assert_eq!(summary.transfer_editor_type, "internal");
        assert_eq!(summary.transfer_default_editor, "code");
        assert_eq!(summary.transfer_download_threads, 5);
        assert_eq!(summary.transfer_upload_threads, 4);
        assert_eq!(summary.transfer_max_retries, 6);
        assert_eq!(summary.transfer_buffer_size, 64);
        assert_eq!(summary.transfer_default_file_permissions, "664");
        assert!(!summary.transfer_preserve_timestamps);
        assert!(!summary.transfer_resume_broken_transfer);
        assert_eq!(summary.recording_path, "/tmp/nyaterm-recordings");
        assert!(summary.recording_auto_start);
        assert!(!summary.recording_include_io_labels);
        assert!(!summary.recording_include_timestamps);
        assert_eq!(summary.recording_memory_limit_bytes, 1048576);
        assert_eq!(summary.diagnostics_level, "debug");
        assert_eq!(summary.diagnostics_retention_days, 3);
        assert!(summary.startup_restore);
        assert!(!summary.confirm_on_close);
        assert!(summary.enable_screen_lock);
        assert_eq!(summary.idle_lock_minutes, 12);
        assert!(summary.has_master_password);
        assert_eq!(
            summary.keybindings.get("terminal.find").map(String::as_str),
            Some("ctrl+f")
        );
        assert!(!summary.keybindings.contains_key("ignored_non_string"));

        let updated = store.save_host_key_policy("accept").expect("save policy");
        assert_eq!(updated.host_key_policy, "accept");
        let stored = store.load_settings_value().expect("stored settings");
        assert_eq!(
            json_path(&stored, &["general", "custom_general"]).and_then(|value| value.as_str()),
            Some("keep")
        );
        assert_eq!(
            json_path(&stored, &["unrelated", "preserve"]).and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            json_path(&stored, &["security", "host_key_policy"]).and_then(|value| value.as_str()),
            Some("accept")
        );
        assert_eq!(
            json_path(&stored, &["keybindings", "terminal.find"]).and_then(|value| value.as_str()),
            Some("ctrl+f")
        );

        let mut transfer_update = summary.clone();
        transfer_update.transfer_download_path = "/var/tmp/downloads".to_string();
        transfer_update.transfer_ask_save_location = false;
        transfer_update.transfer_duplicate_strategy = "overwrite".to_string();
        transfer_update.transfer_editor_type = "external".to_string();
        transfer_update.transfer_default_editor = "gedit".to_string();
        transfer_update.transfer_download_threads = 2;
        transfer_update.transfer_upload_threads = 6;
        transfer_update.transfer_max_retries = 3;
        transfer_update.transfer_buffer_size = 128;
        transfer_update.transfer_default_file_permissions = "755".to_string();
        transfer_update.transfer_preserve_timestamps = true;
        transfer_update.transfer_resume_broken_transfer = true;
        let updated = store
            .save_transfer_settings(&transfer_update)
            .expect("save transfer settings");
        assert_eq!(updated.transfer_download_path, "/var/tmp/downloads");
        assert!(!updated.transfer_ask_save_location);
        assert_eq!(updated.transfer_duplicate_strategy, "overwrite");
        assert_eq!(updated.transfer_editor_type, "external");
        assert_eq!(updated.transfer_default_editor, "gedit");
        assert_eq!(updated.transfer_download_threads, 2);
        assert_eq!(updated.transfer_upload_threads, 6);
        assert_eq!(updated.transfer_max_retries, 3);
        assert_eq!(updated.transfer_buffer_size, 128);
        assert_eq!(updated.transfer_default_file_permissions, "755");
        assert!(updated.transfer_preserve_timestamps);
        assert!(updated.transfer_resume_broken_transfer);
        let stored = store
            .load_settings_value()
            .expect("stored transfer settings");
        assert_eq!(
            json_path(&stored, &["transfer", "download_path"]).and_then(|value| value.as_str()),
            Some("/var/tmp/downloads")
        );
        assert_eq!(
            json_path(&stored, &["transfer", "ask_save_location"])
                .and_then(|value| value.as_bool()),
            Some(false)
        );
        assert_eq!(
            json_path(&stored, &["transfer", "duplicate_strategy"])
                .and_then(|value| value.as_str()),
            Some("overwrite")
        );
        assert_eq!(
            json_path(&stored, &["transfer", "editor_type"]).and_then(|value| value.as_str()),
            Some("external")
        );
        assert_eq!(
            json_path(&stored, &["transfer", "default_editor"]).and_then(|value| value.as_str()),
            Some("gedit")
        );
        assert_eq!(
            json_path(&stored, &["transfer", "download_threads"]).and_then(|value| value.as_u64()),
            Some(2)
        );
        assert_eq!(
            json_path(&stored, &["transfer", "upload_threads"]).and_then(|value| value.as_u64()),
            Some(6)
        );
        assert_eq!(
            json_path(&stored, &["transfer", "max_transfer_retries"])
                .and_then(|value| value.as_u64()),
            Some(3)
        );
        assert_eq!(
            json_path(&stored, &["transfer", "transfer_buffer_size"])
                .and_then(|value| value.as_u64()),
            Some(128)
        );
        assert_eq!(
            json_path(&stored, &["transfer", "default_file_permissions"])
                .and_then(|value| value.as_str()),
            Some("755")
        );
        assert_eq!(
            json_path(&stored, &["transfer", "preserve_timestamps"])
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            json_path(&stored, &["transfer", "resume_broken_transfer"])
                .and_then(|value| value.as_bool()),
            Some(true)
        );
        assert_eq!(
            json_path(&stored, &["unrelated", "preserve"]).and_then(|value| value.as_bool()),
            Some(true)
        );

        let mut favorite_update = summary.clone();
        favorite_update.ui_file_explorer_auto_sync_cwd_connection_ids = vec![
            "conn-3".to_string(),
            "conn-3".to_string(),
            " ".to_string(),
            "conn-1".to_string(),
        ];
        favorite_update
            .ui_file_explorer_favorite_dirs_by_connection_id
            .insert(
                "conn-2".to_string(),
                vec![
                    "/data".to_string(),
                    "/data".to_string(),
                    " ".to_string(),
                    "/logs".to_string(),
                ],
            );
        let updated = store
            .save_file_explorer_favorite_dirs(&favorite_update)
            .expect("save favorites");
        assert_eq!(
            updated.ui_file_explorer_auto_sync_cwd_connection_ids,
            vec!["conn-3".to_string(), "conn-1".to_string()]
        );
        assert_eq!(
            updated
                .ui_file_explorer_favorite_dirs_by_connection_id
                .get("conn-2"),
            Some(&vec!["/data".to_string(), "/logs".to_string()])
        );
        let stored = store.load_settings_value().expect("stored favorites");
        assert_eq!(
            json_path(
                &stored,
                &["ui", "file_explorer_auto_sync_cwd_connection_ids"]
            )
            .and_then(|value| value.as_array())
            .map(|values| values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()),
            Some(vec!["conn-3", "conn-1"])
        );
        assert_eq!(
            json_path(
                &stored,
                &[
                    "ui",
                    "file_explorer_favorite_dirs_by_connection_id",
                    "conn-2"
                ]
            )
            .and_then(|value| value.as_array())
            .map(|values| values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()),
            Some(vec!["/data", "/logs"])
        );

        let mut next_keybindings = summary.keybindings.clone();
        next_keybindings.insert("view.openSettings".to_string(), "ctrl+.".to_string());
        next_keybindings.insert("blank".to_string(), " ".to_string());
        let updated = store
            .save_keybindings(&next_keybindings)
            .expect("save keybindings");
        assert_eq!(
            updated
                .keybindings
                .get("view.openSettings")
                .map(String::as_str),
            Some("ctrl+.")
        );
        assert!(!updated.keybindings.contains_key("blank"));

        let mut terminal_update = updated.clone();
        terminal_update.terminal_scrollback_lines = 12_000;
        terminal_update.terminal_keep_alive_interval = 20;
        terminal_update.terminal_show_multi_line_paste_dialog = true;
        terminal_update.ui_show_remote_stats = true;
        terminal_update.ui_remote_stats_interval = 4;
        let saved_terminal = store
            .save_terminal_settings(&terminal_update)
            .expect("save terminal settings");
        assert_eq!(saved_terminal.terminal_scrollback_lines, 12_000);
        assert_eq!(saved_terminal.terminal_keep_alive_interval, 20);
        assert!(saved_terminal.terminal_show_multi_line_paste_dialog);
        assert!(saved_terminal.ui_show_remote_stats);
        assert_eq!(saved_terminal.ui_remote_stats_interval, 4);
        let stored = store
            .load_settings_value()
            .expect("stored terminal settings");
        assert_eq!(
            json_path(&stored, &["terminal", "scrollback_lines"]).and_then(|value| value.as_u64()),
            Some(12_000)
        );
        assert_eq!(
            json_path(&stored, &["ui", "remote_stats_interval"]).and_then(|value| value.as_u64()),
            Some(4)
        );

        let mut quick_command_update = saved_terminal.clone();
        quick_command_update.ui_quick_cmd_view_mode = "list".to_string();
        quick_command_update.ui_quick_cmd_sort_mode = "name".to_string();
        let saved_quick_command_ui = store
            .save_quick_command_ui_settings(&quick_command_update)
            .expect("save quick command ui settings");
        assert_eq!(saved_quick_command_ui.ui_quick_cmd_view_mode, "list");
        assert_eq!(saved_quick_command_ui.ui_quick_cmd_sort_mode, "name");
        let stored = store
            .load_settings_value()
            .expect("stored quick command ui settings");
        assert_eq!(
            json_path(&stored, &["ui", "quick_cmd_view_mode"]).and_then(|value| value.as_str()),
            Some("list")
        );
        assert_eq!(
            json_path(&stored, &["ui", "quick_cmd_sort_mode"]).and_then(|value| value.as_str()),
            Some("name")
        );

        let mut interaction_update = saved_quick_command_ui.clone();
        interaction_update.interaction_right_click_paste = false;
        interaction_update.interaction_command_suggestion_min_chars = 4;
        interaction_update.interaction_command_suggestion_max_chars = 120;
        interaction_update.interaction_duplicate_session_command_delay_ms = 2_500;
        interaction_update.interaction_alt_as_meta = false;
        interaction_update.interaction_tab_double_click_action = "reconnect_session".to_string();
        interaction_update.interaction_default_encoding = "utf-8".to_string();
        let saved_interaction = store
            .save_interaction_settings(&interaction_update)
            .expect("save interaction settings");
        assert!(!saved_interaction.interaction_right_click_paste);
        assert_eq!(
            saved_interaction.interaction_command_suggestion_min_chars,
            4
        );
        assert_eq!(
            saved_interaction.interaction_command_suggestion_max_chars,
            120
        );
        assert_eq!(
            saved_interaction.interaction_duplicate_session_command_delay_ms,
            2_500
        );
        assert!(!saved_interaction.interaction_alt_as_meta);
        assert_eq!(
            saved_interaction.interaction_tab_double_click_action,
            "reconnect_session"
        );
        assert_eq!(saved_interaction.interaction_default_encoding, "UTF-8");

        let normalized = store.save_host_key_policy("wild").expect("normalize");
        assert_eq!(normalized.host_key_policy, "prompt");

        let mut recording_update = normalized.clone();
        recording_update.recording_path = "/var/log/nyaterm".to_string();
        recording_update.recording_auto_start = false;
        recording_update.recording_include_io_labels = true;
        recording_update.recording_include_timestamps = true;
        recording_update.recording_memory_limit_bytes = 2 * 1024 * 1024;
        let saved_recording = store
            .save_recording_settings(&recording_update)
            .expect("save recording settings");
        assert_eq!(saved_recording.recording_path, "/var/log/nyaterm");
        assert!(!saved_recording.recording_auto_start);
        assert!(saved_recording.recording_include_io_labels);
        assert!(saved_recording.recording_include_timestamps);
        assert_eq!(
            saved_recording.recording_memory_limit_bytes,
            2 * 1024 * 1024
        );

        let mut lock_update = saved_recording.clone();
        lock_update.enable_screen_lock = false;
        lock_update.idle_lock_minutes = 30;
        let saved_lock = store
            .save_screen_lock_settings(&lock_update)
            .expect("save screen lock settings");
        assert!(!saved_lock.enable_screen_lock);
        assert_eq!(saved_lock.idle_lock_minutes, 30);
        assert!(saved_lock.has_master_password);
        let stored = store.load_settings_value().expect("stored settings");
        assert_eq!(
            json_path(&stored, &["security", "master_password"]).and_then(|value| value.as_str()),
            Some("encrypted")
        );

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn verifies_encrypted_master_password_from_settings() {
        let dir = unique_temp_dir("verify-master-password");
        let store = ConnectionStore::open(&dir).expect("store");
        let token = encrypt_for_test(b"swordfish", &home_wrapping_key());
        store
            .save_settings_value(&serde_json::json!({
                "security": {
                    "master_password": token
                }
            }))
            .expect("seed settings");

        assert!(
            store
                .verify_master_password("swordfish")
                .expect("verify correct")
        );
        assert!(
            !store
                .verify_master_password("wrong")
                .expect("verify incorrect")
        );

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn missing_master_password_verifies_without_prompt_secret() {
        let dir = unique_temp_dir("verify-empty-master-password");
        let store = ConnectionStore::open(&dir).expect("store");
        assert!(
            store
                .verify_master_password("")
                .expect("verify without master password")
        );
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn keyword_highlights_round_trip_and_import_merge() {
        let dir = unique_temp_dir("keyword-highlights");
        let store = ConnectionStore::open(&dir).expect("store");
        let initial = serde_json::json!({
            "general": {
                "custom_general": "keep"
            },
            "terminal": {
                "keyword_highlights_enabled": true,
                "keyword_highlights_across_wrapped_lines": true,
                "keyword_highlights": [
                    {
                        "id": "panic",
                        "name": "Panic",
                        "patterns": ["panic", "ERROR"],
                        "color_dark": "#ff6b6b",
                        "color_light": "#b00020",
                        "enabled": true
                    },
                    {
                        "id": "invalid-empty-pattern",
                        "name": "Ignored",
                        "patterns": [""]
                    },
                    {
                        "id": "invalid-name",
                        "name": "   ",
                        "patterns": ["warn"]
                    }
                ]
            }
        });
        store.save_settings_value(&initial).expect("seed settings");

        let loaded = store.load_keyword_highlights().expect("load highlights");
        assert!(loaded.enabled);
        assert!(loaded.across_wrapped_lines);
        assert_eq!(loaded.rules.len(), 1);
        assert_eq!(loaded.rules[0].id, "panic");
        assert_eq!(loaded.rules[0].patterns, vec!["panic", "ERROR"]);

        let object_import = r##"{
            "keyword_highlights": [
                {
                    "id": "panic",
                    "name": "Panic Updated",
                    "patterns": ["panic:"],
                    "color_dark": "#ffd166",
                    "color_light": "#8a5a00",
                    "enabled": false
                },
                {
                    "name": "Deploy",
                    "patterns": ["deploy"]
                }
            ]
        }"##;
        let (saved, result) = store
            .import_keyword_highlights_json(object_import)
            .expect("object import");
        assert_eq!(result.imported_rules, 1);
        assert_eq!(result.updated_rules, 1);
        assert_eq!(result.total_rules, 2);
        assert!(saved.enabled);
        assert!(saved.across_wrapped_lines);

        let panic_rule = saved
            .rules
            .iter()
            .find(|rule| rule.id == "panic")
            .expect("panic rule");
        assert_eq!(panic_rule.name, "Panic Updated");
        assert!(!panic_rule.enabled);

        let deploy_rule = saved
            .rules
            .iter()
            .find(|rule| rule.name == "Deploy")
            .expect("deploy rule");
        assert!(deploy_rule.id.starts_with("highlight-"));
        assert_eq!(deploy_rule.color_dark, "#79c0ff");
        assert_eq!(deploy_rule.color_light, "#0969da");

        let array_import = r##"[
            {
                "id": "panic",
                "name": "Panic Final",
                "patterns": ["fatal"],
                "color_dark": "#fca5a5",
                "color_light": "#991b1b",
                "enabled": true
            }
        ]"##;
        let (saved, result) = store
            .import_keyword_highlights_json(array_import)
            .expect("array import");
        assert_eq!(result.imported_rules, 0);
        assert_eq!(result.updated_rules, 1);
        assert_eq!(result.total_rules, 2);
        assert_eq!(
            saved
                .rules
                .iter()
                .find(|rule| rule.id == "panic")
                .map(|rule| rule.patterns.as_slice()),
            Some(&["fatal".to_string()][..])
        );

        let stored = store.load_settings_value().expect("stored settings");
        assert_eq!(
            json_path(&stored, &["general", "custom_general"]).and_then(|value| value.as_str()),
            Some("keep")
        );

        let invalid = store.import_keyword_highlights_json(r#"[{"name":"","patterns":[""]}]"#);
        assert!(matches!(invalid, Err(StorageError::InvalidData(_))));

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn cloud_sync_state_round_trips_and_reads_legacy_doc() {
        let dir = unique_temp_dir("cloud-sync-state");
        let store = ConnectionStore::open(&dir).expect("store");
        let state = CloudSyncState {
            device_id: "device-a".to_string(),
            last_synced_payload_hash: Some("hash-a".to_string()),
            last_applied_remote_revision: Some("rev-a".to_string()),
            last_checked_at_ms: Some(10),
            last_synced_at_ms: Some(20),
        };
        store
            .save_cloud_sync_state(&state)
            .expect("save cloud state");
        let loaded = store.load_cloud_sync_state().expect("load cloud state");
        assert_eq!(loaded, state);

        let legacy_dir = unique_temp_dir("cloud-sync-state-legacy");
        let legacy_store = ConnectionStore::open(&legacy_dir).expect("legacy store");
        let legacy = CloudSyncState {
            device_id: "legacy-device".to_string(),
            last_synced_payload_hash: Some("legacy-hash".to_string()),
            last_applied_remote_revision: Some("legacy-rev".to_string()),
            last_checked_at_ms: Some(30),
            last_synced_at_ms: Some(40),
        };
        let legacy_content = serde_json::to_string(&legacy).expect("legacy json");
        let txn = legacy_store.db.begin_write().expect("legacy txn");
        txn.open_table(TEXT_DOCS_TABLE)
            .expect("text docs")
            .insert(LEGACY_TEXT_CLOUD_SYNC_STATE, legacy_content.as_str())
            .expect("insert legacy state");
        txn.commit().expect("commit legacy state");
        let loaded_legacy = legacy_store
            .load_cloud_sync_state()
            .expect("load legacy cloud state");
        assert_eq!(loaded_legacy, legacy);

        std::fs::remove_dir_all(dir).ok();
        std::fs::remove_dir_all(legacy_dir).ok();
    }

    #[test]
    fn translation_settings_read_legacy_plaintext_and_encrypt_on_save() {
        let dir = unique_temp_dir("translation-settings");
        let store = ConnectionStore::open(&dir).expect("store");
        let initial = serde_json::json!({
            "translation": {
                "target_language": "ja",
                "deepl_api_key": "deepl-secret",
                "baidu_app_id": "baidu-id",
                "baidu_app_key": "baidu-secret",
                "ali_app_id": "ali-id",
                "ali_app_key": "ali-secret",
                "youdao_app_id": "youdao-id",
                "youdao_app_key": "youdao-secret"
            },
            "general": {
                "custom_general": "keep"
            }
        });
        store.save_settings_value(&initial).expect("seed settings");

        let loaded = store
            .load_translation_settings()
            .expect("load translation settings");
        assert_eq!(loaded.target_language, "ja");
        assert_eq!(loaded.deepl_api_key, "deepl-secret");
        assert_eq!(loaded.baidu_app_key, "baidu-secret");
        assert_eq!(loaded.ali_app_key, "ali-secret");
        assert_eq!(loaded.youdao_app_key, "youdao-secret");

        let mut update = loaded.clone();
        update.deepl_api_key = crate::MASKED_SECRET_VALUE.to_string();
        update.baidu_app_key.clear();
        update.ali_app_key = "ali-replacement".to_string();
        let saved = store
            .save_translation_settings(update)
            .expect("save translation settings");
        assert_eq!(saved.deepl_api_key, "deepl-secret");
        assert_eq!(saved.baidu_app_key, "");
        assert_eq!(saved.ali_app_key, "ali-replacement");
        assert_eq!(saved.youdao_app_key, "youdao-secret");
        assert!(store.load_master_key_token().expect("master key").is_some());

        let raw = store
            .read_json_table::<serde_json::Value>(SETTINGS_TABLE, SETTINGS_DEFAULT)
            .expect("read raw settings")
            .expect("raw settings");
        let raw_translation = raw.get("translation").expect("translation");
        assert_ne!(
            raw_translation["deepl_api_key"].as_str(),
            Some("deepl-secret")
        );
        assert_eq!(raw_translation["baidu_app_key"].as_str(), Some(""));
        assert_ne!(
            raw_translation["ali_app_key"].as_str(),
            Some("ali-replacement")
        );
        assert_ne!(
            raw_translation["youdao_app_key"].as_str(),
            Some("youdao-secret")
        );
        assert_eq!(
            json_path(&raw, &["general", "custom_general"]).and_then(|value| value.as_str()),
            Some("keep")
        );
        assert_eq!(
            store
                .load_translation_settings()
                .expect("reload translation settings"),
            saved
        );

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn cloud_sync_settings_encrypt_and_merge_masked_provider_secrets() {
        let dir = unique_temp_dir("cloud-sync-settings");
        let store = ConnectionStore::open(&dir).expect("store");
        let mut settings = CloudSyncSettings {
            enabled: true,
            provider: "github_gist".to_string(),
            ..CloudSyncSettings::default()
        };
        settings.webdav.password = Some("webdav-secret".to_string());
        settings.s3.secret_access_key = Some("s3-secret".to_string());
        settings.google_drive.access_token = Some("google-access".to_string());
        settings.github_gist.access_token = Some("github-token".to_string());

        let saved = store
            .save_cloud_sync_settings(settings.clone())
            .expect("save cloud settings");
        assert_eq!(saved, settings);
        assert!(store.load_master_key_token().expect("master key").is_some());

        let raw = store
            .read_json_table::<CloudSyncSettings>(SETTINGS_TABLE, SETTINGS_CLOUD_SYNC)
            .expect("read raw")
            .expect("raw cloud settings");
        assert_ne!(raw.webdav.password.as_deref(), Some("webdav-secret"));
        assert_ne!(raw.s3.secret_access_key.as_deref(), Some("s3-secret"));
        assert_ne!(
            raw.google_drive.access_token.as_deref(),
            Some("google-access")
        );
        assert_ne!(
            raw.github_gist.access_token.as_deref(),
            Some("github-token")
        );

        let loaded = store
            .load_cloud_sync_settings()
            .expect("load cloud settings");
        assert_eq!(loaded, settings);

        let mut masked_update = loaded.clone();
        masked_update.webdav.password = Some(crate::MASKED_SECRET_VALUE.to_string());
        masked_update.s3.secret_access_key = Some(String::new());
        masked_update.github_gist.access_token = Some("replacement-token".to_string());
        let merged = store
            .save_cloud_sync_settings(masked_update)
            .expect("save masked cloud settings");
        assert_eq!(merged.webdav.password.as_deref(), Some("webdav-secret"));
        assert_eq!(merged.s3.secret_access_key, None);
        assert_eq!(
            merged.github_gist.access_token.as_deref(),
            Some("replacement-token")
        );

        let reloaded = store
            .load_cloud_sync_settings()
            .expect("reload cloud settings");
        assert_eq!(reloaded, merged);

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn ai_settings_encrypt_and_merge_masked_provider_secrets() {
        let dir = unique_temp_dir("ai-settings");
        let store = ConnectionStore::open(&dir).expect("store");
        let mut settings = AiSettings {
            enabled: true,
            default_mode: crate::AiMode::Agent,
            ..AiSettings::default()
        };
        settings.provider_profiles[0].enabled = true;
        settings.provider_profiles[0].api_key = Some("profile-key".to_string());
        settings.provider_credentials[0].enabled = true;
        settings.provider_credentials[0].api_key = Some("credential-key".to_string());

        let saved = store.save_ai_settings(settings.clone()).expect("save ai");
        assert_eq!(saved.default_mode, crate::AiMode::Agent);
        assert_eq!(
            saved.provider_profiles[0].api_key.as_deref(),
            Some("profile-key")
        );
        assert_eq!(
            saved.provider_credentials[0].api_key.as_deref(),
            Some("credential-key")
        );
        assert_eq!(
            saved.default_model_id.as_deref(),
            Some("openai:gpt-4o-mini")
        );
        assert!(!saved.models.is_empty());
        assert!(store.load_master_key_token().expect("master key").is_some());

        let raw = store
            .read_json_table::<serde_json::Value>(SETTINGS_TABLE, SETTINGS_DEFAULT)
            .expect("read raw settings")
            .expect("raw settings");
        let raw_ai = raw.get("ai").expect("ai field");
        assert_ne!(
            raw_ai["provider_profiles"][0]["api_key"].as_str(),
            Some("profile-key")
        );
        assert_ne!(
            raw_ai["provider_credentials"][0]["api_key"].as_str(),
            Some("credential-key")
        );

        let loaded = store.load_ai_settings().expect("load ai");
        assert_eq!(loaded, saved);

        let mut masked_update = loaded.clone();
        masked_update.provider_profiles[0].api_key = Some(crate::MASKED_SECRET_VALUE.to_string());
        masked_update.provider_credentials[0].api_key = Some("replacement-key".to_string());
        let merged = store
            .save_ai_settings(masked_update)
            .expect("save masked ai");
        assert_eq!(
            merged.provider_profiles[0].api_key.as_deref(),
            Some("profile-key")
        );
        assert_eq!(
            merged.provider_credentials[0].api_key.as_deref(),
            Some("replacement-key")
        );
        assert_eq!(store.load_ai_settings().expect("reload ai"), merged);

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn ai_settings_loads_and_normalizes_legacy_embedded_settings() {
        let dir = unique_temp_dir("ai-settings-legacy");
        let store = ConnectionStore::open(&dir).expect("store");
        let mut raw = default_settings_value();
        set_nested_json_value(
            &mut raw,
            &["ai"],
            serde_json::json!({
                "schema_version": 2,
                "enabled": true,
                "active_profile_id": "ollama",
                "provider_profiles": [{
                    "id": "ollama",
                    "name": "Ollama",
                    "provider_kind": "ollama",
                    "model": "llama3",
                    "base_url": "http://localhost:11434/v1/",
                    "enabled": true
                }],
                "models": [],
                "provider_credentials": []
            }),
        );
        store.save_settings_value(&raw).expect("save raw settings");

        let loaded = store.load_ai_settings().expect("load ai");
        assert_eq!(loaded.schema_version, 3);
        assert_eq!(loaded.active_profile_id, "ollama");
        assert_eq!(loaded.default_model_id.as_deref(), Some("ollama:llama3"));
        assert!(!loaded.provider_credentials.is_empty());
        assert!(!loaded.terminal_ai_actions.is_empty());

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn quick_commands_round_trip_and_upsert_preserves_created_use_count() {
        let dir = unique_temp_dir("quick-commands");
        let store = ConnectionStore::open(&dir).expect("store");

        assert_eq!(
            store.load_quick_commands().expect("empty quick commands"),
            QuickCommandsConfig::default()
        );

        let inserted = store
            .upsert_quick_command(
                QuickCommand {
                    id: "cmd-1".to_string(),
                    label: "List".to_string(),
                    command: "ls -la".to_string(),
                    category_id: Some("cat-shell".to_string()),
                    description: Some("List files".to_string()),
                    color_tag: Some("blue".to_string()),
                    icon_tag: Some("terminal".to_string()),
                    pinned: Some(false),
                    execution_mode: Some("append".to_string()),
                    source: Some("ai".to_string()),
                    risk_level: Some(crate::RiskLevel::Low),
                    updated_at: None,
                    created_at: Some(111),
                    use_count: Some(7),
                },
                Some(QuickCommandCategory {
                    id: "cat-shell".to_string(),
                    name: "Shell".to_string(),
                }),
            )
            .expect("insert quick command");
        assert_eq!(inserted.categories.len(), 1);
        assert_eq!(inserted.commands[0].created_at, Some(111));
        assert_eq!(inserted.commands[0].use_count, Some(7));
        assert!(inserted.commands[0].updated_at.is_some());

        let updated = store
            .upsert_quick_command(
                QuickCommand {
                    id: "cmd-1".to_string(),
                    label: "List all".to_string(),
                    command: "ls -lah".to_string(),
                    category_id: Some("cat-shell".to_string()),
                    description: None,
                    color_tag: Some("green".to_string()),
                    icon_tag: Some("terminal".to_string()),
                    pinned: Some(true),
                    execution_mode: Some("append".to_string()),
                    source: Some("manual".to_string()),
                    risk_level: Some(crate::RiskLevel::Medium),
                    updated_at: None,
                    created_at: Some(999),
                    use_count: Some(99),
                },
                Some(QuickCommandCategory {
                    id: "cat-shell".to_string(),
                    name: "Duplicate Shell".to_string(),
                }),
            )
            .expect("update quick command");
        assert_eq!(updated.categories.len(), 1);
        assert_eq!(updated.commands.len(), 1);
        assert_eq!(updated.commands[0].label, "List all");
        assert_eq!(updated.commands[0].command, "ls -lah");
        assert_eq!(updated.commands[0].created_at, Some(111));
        assert_eq!(updated.commands[0].use_count, Some(7));

        store
            .increment_quick_command_use_count("cmd-1")
            .expect("increment quick command");
        let loaded = store.load_quick_commands().expect("load quick commands");
        assert_eq!(loaded.commands[0].use_count, Some(8));

        let raw = store
            .read_json_table::<serde_json::Value>(SETTINGS_TABLE, SETTINGS_QUICK_COMMANDS)
            .expect("read raw quick commands")
            .expect("quick command doc");
        assert_eq!(raw["categories"][0]["id"], "cat-shell");
        assert_eq!(raw["commands"][0]["category_id"], "cat-shell");
        assert_eq!(raw["commands"][0]["execution_mode"], "append");
        assert_eq!(raw["commands"][0]["source"], "manual");
        assert!(raw["commands"][0].get("created_at").is_some());
        assert!(raw["commands"][0].get("use_count").is_some());

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn command_history_uses_legacy_table_and_normalizes_entries() {
        let dir = unique_temp_dir("command-history");
        let store = ConnectionStore::open(&dir).expect("store");

        store
            .append_command_history(" user@host:~$ ls -la ")
            .expect("append ls");
        store
            .append_command_history("ls -la")
            .expect("append ls again");
        store
            .append_command_history("PS C:\\Users\\me> git status")
            .expect("append powershell");
        store.append_command_history("   ").expect("ignore blank");

        let history = store.list_command_history(10).expect("list history");
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].command, "git status");
        assert_eq!(history[1].command, "ls -la");
        assert_eq!(history[1].use_count, 2);

        let raw = store
            .list_raw_by_prefix(COMMAND_HISTORY_TABLE, COMMAND_HISTORY_PREFIX)
            .expect("raw history");
        assert_eq!(raw.len(), 2);
        assert!(
            raw.iter()
                .all(|(key, _)| key.starts_with(COMMAND_HISTORY_PREFIX))
        );
        assert!(raw.iter().all(|(key, _)| key.contains('|')));

        store
            .delete_command_history("root@server:/tmp# ls -la")
            .expect("delete normalized");
        let remaining = store.list_command_history(10).expect("remaining history");
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].command, "git status");

        store
            .replace_command_history(&[
                CommandHistoryEntry {
                    command: "[prod] $ uptime".to_string(),
                    last_used_at_ms: 10,
                    use_count: 1,
                },
                CommandHistoryEntry {
                    command: "uptime".to_string(),
                    last_used_at_ms: 20,
                    use_count: 3,
                },
                CommandHistoryEntry {
                    command: "pwd".to_string(),
                    last_used_at_ms: 30,
                    use_count: 1,
                },
            ])
            .expect("replace history");
        let replaced = store.list_command_history(10).expect("replaced history");
        assert_eq!(
            replaced
                .iter()
                .map(|entry| entry.command.as_str())
                .collect::<Vec<_>>(),
            ["pwd", "uptime"]
        );
        assert_eq!(replaced[1].use_count, 4);

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn ai_history_round_trips_messages_and_deletes_session() {
        let dir = unique_temp_dir("ai-history");
        let store = ConnectionStore::open(&dir).expect("store");

        store
            .append_ai_user_message("session-1", Some("conn-1".to_string()), "  ".to_string())
            .expect("append user");
        let sessions = store.list_ai_sessions().expect("sessions");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].title, "AI Session");
        assert_eq!(sessions[0].connection_id.as_deref(), Some("conn-1"));

        store
            .append_ai_message(AiMessage {
                id: "assistant-1".to_string(),
                session_id: "session-1".to_string(),
                role: AiMessageRole::Assistant,
                content: "hello".to_string(),
                created_at: "2026-04-28T00:00:01Z".to_string(),
                reasoning_content: Some("reasoning".to_string()),
                command_cards: Vec::new(),
            })
            .expect("append assistant");

        let messages = store.list_ai_messages("session-1").expect("messages");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, AiMessageRole::User);
        assert_eq!(messages[1].reasoning_content.as_deref(), Some("reasoning"));

        store
            .delete_ai_session("session-1")
            .expect("delete session");
        assert!(store.list_ai_sessions().expect("sessions").is_empty());
        assert!(
            store
                .list_ai_messages("session-1")
                .expect("messages")
                .is_empty()
        );

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn ai_audit_round_trips_sorts_and_limits_logs() {
        let dir = unique_temp_dir("ai-audit");
        let store = ConnectionStore::open(&dir).expect("store");

        let first = store
            .append_ai_audit(AppendAiAuditRequest {
                connection_id: Some("conn-1".to_string()),
                action: "generate_command".to_string(),
                user_input: Some("list files".to_string()),
                generated_command: Some("ls".to_string()),
                risk_level: Some(crate::RiskLevel::Low),
                inserted_to_terminal: true,
                executed: false,
                blocked: false,
            })
            .expect("append audit");
        assert!(first.id.starts_with("audit-"));

        let mut file = AiAuditFile::default();
        file.logs.push(first.clone());
        file.logs.push(AiAuditLog {
            id: "audit-later".to_string(),
            connection_id: None,
            action: "execute".to_string(),
            user_input: None,
            generated_command: None,
            risk_level: Some(crate::RiskLevel::Medium),
            inserted_to_terminal: false,
            executed: true,
            blocked: false,
            created_at: "2999-01-01T00:00:00Z".to_string(),
        });
        store
            .save_settings_doc_value(SETTINGS_AI_AUDIT, &serde_json::to_value(file).unwrap())
            .expect("save audit");

        let limited = store.list_ai_audit_logs(Some(1)).expect("audit logs");
        assert_eq!(limited.len(), 1);
        assert_eq!(limited[0].id, "audit-later");

        std::fs::remove_dir_all(dir).ok();
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("nyaterm-domain-{name}-{}", std::process::id()));
        std::fs::remove_dir_all(&dir).ok();
        dir
    }

    fn encrypt_for_test(plaintext: &[u8], key: &Key<Aes256Gcm>) -> String {
        let cipher = Aes256Gcm::new(key);
        let nonce = aes_gcm::Nonce::from([9_u8; 12]);
        let ciphertext = cipher.encrypt(&nonce, plaintext).expect("encrypt");
        let mut combined = nonce.to_vec();
        combined.extend_from_slice(&ciphertext);
        B64.encode(combined)
    }

    #[allow(deprecated)]
    fn test_key(seed: u8) -> Key<Aes256Gcm> {
        *Key::<Aes256Gcm>::from_slice(&[seed; 32])
    }

    #[allow(deprecated)]
    fn home_wrapping_key() -> Key<Aes256Gcm> {
        let mut hasher = Sha256::new();
        hasher.update(b"nyaterm-key-wrap-v1:");
        hasher.update(dirs::home_dir().expect("home").to_string_lossy().as_bytes());
        *Key::<Aes256Gcm>::from_slice(&hasher.finalize())
    }
}

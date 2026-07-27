use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use redb::{Database, ReadableDatabase, ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    AiSettings, CredentialCrypto, CredentialCryptoError, Group, KeywordHighlightConfig,
    KeywordHighlightImportResult, KeywordHighlightRule, PortableSnapshotError, ProxyConfig,
    ProxyGroup, ProxyGroupsConfig, QuickCommand, QuickCommandCategory, QuickCommandsConfig,
    SavedConnection, SessionsConfig, TranslationSettings, TunnelConfig, TunnelGroup,
    TunnelGroupsConfig, ai_settings_has_secret, merge_masked_ai_settings,
    merge_masked_translation_settings, normalize_ai_settings, translation_settings_has_secret,
    uuid,
};

mod ai_history;
mod app_settings;
mod cloud_sync;
mod command_history;
mod config_backup;
mod keyword_highlights;
mod known_hosts;
mod portable;
mod vault;

use self::command_history::replace_command_history_in_txn;
pub use self::config_backup::ConfigBackupInfo;
use self::config_backup::{
    copy_config_database, ensure_not_same_existing_file, ensure_parent_dir,
    validate_config_backup_source, write_portable_snapshot_file,
};
use self::keyword_highlights::{
    merge_keyword_highlight_rules, normalize_keyword_highlight_rule, parse_keyword_highlight_import,
};
pub use self::known_hosts::KnownHostCheck;
use self::known_hosts::replace_known_hosts_text_in_txn;

const DATABASE_FILE: &str = "nyaterm.redb";
const GROUP_PREFIX: &str = "groups/";
const CONNECTION_PREFIX: &str = "connections/";
const TUNNEL_PREFIX: &str = "tunnels/";
const SSH_KEY_PREFIX: &str = "credentials/key/";
const CREDENTIAL_PREFIX: &str = "credentials/credential/";
const PASSWORD_PREFIX: &str = "credentials/password/";
const CONNECTION_PASSWORD_PREFIX: &str = "credentials/connection-password/";
const SSH_KEY_FILE_IMPORT_MAX_BYTES: u64 = 1024 * 1024;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ConnectionPasswordRecord {
    id: String,
    connection_id: String,
    password: String,
    created_at_ms: u64,
    updated_at_ms: u64,
}

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

    pub fn load_sessions(&self) -> Result<SessionsConfig, StorageError> {
        let groups = self.list_groups()?;
        let mut connections = self.list_connections()?;
        self.hydrate_connection_passwords(&mut connections)?;
        Ok(SessionsConfig {
            groups,
            connections,
        })
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
        let groups = self.list_groups()?;
        let connections = self.list_connections()?;
        let mut group_ids = std::collections::HashSet::from([group_id.to_string()]);
        let mut changed = true;
        while changed {
            changed = false;
            for group in &groups {
                if group
                    .parent_id
                    .as_ref()
                    .is_some_and(|parent| group_ids.contains(parent))
                    && group_ids.insert(group.id.clone())
                {
                    changed = true;
                }
            }
        }

        let txn = self.db.begin_write()?;
        for connection in connections.iter().filter(|connection| {
            connection
                .group_id
                .as_ref()
                .is_some_and(|id| group_ids.contains(id))
        }) {
            delete_connection_in_txn(&txn, &connection.id)?;
        }
        for id in group_ids {
            txn.open_table(GROUPS_TABLE)?
                .remove(entity_key(GROUP_PREFIX, &id).as_str())?;
        }
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

    pub fn save_group_and_connection(
        &self,
        group: &Group,
        connection: &SavedConnection,
    ) -> Result<(), StorageError> {
        let txn = self.db.begin_write()?;
        save_group_in_txn(&txn, group)?;
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
        let builtin_rules = json_path(&value, &["terminal", "keyword_highlight_builtin_rules"])
            .cloned()
            .map(serde_json::from_value::<std::collections::HashMap<String, bool>>)
            .transpose()?
            .unwrap_or_default();
        Ok(KeywordHighlightConfig {
            enabled: json_bool(&value, &["terminal", "keyword_highlights_enabled"], false),
            across_wrapped_lines: json_bool(
                &value,
                &["terminal", "keyword_highlights_across_wrapped_lines"],
                false,
            ),
            builtin_rules,
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
        set_nested_json_value(
            &mut value,
            &["terminal", "keyword_highlight_builtin_rules"],
            serde_json::to_value(&config.builtin_rules)?,
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
        let imported = parse_keyword_highlight_import(raw)?;
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

fn lower_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
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

fn optional_secret_present(value: &Option<String>) -> bool {
    value.as_deref().is_some_and(|value| !value.is_empty())
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
            "minimize_to_tray": false,
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

fn json_bool(value: &serde_json::Value, path: &[&str], fallback: bool) -> bool {
    json_path(value, path)
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(fallback)
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

fn json_path<'a>(value: &'a serde_json::Value, path: &[&str]) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
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
    use crate::{
        AiExecutionProfile, CloudSyncSettings, CloudSyncState, CommandHistoryEntry, ConnectionAuth,
        ConnectionType, OtpEntry, SearchEngineConfig, SshKey,
    };
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
                icon_auto_detect: None,
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
                icon_auto_detect: None,
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
                    icon_auto_detect: None,
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
                    icon_auto_detect: None,
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
                    icon_auto_detect: None,
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
                    icon_auto_detect: None,
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
            icon_auto_detect: None,
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
    fn save_group_and_connection_persists_both_records() {
        let dir = unique_temp_dir("group-and-connection");
        let store = ConnectionStore::open(&dir).expect("store");
        let group = Group {
            id: "group-1".to_string(),
            name: "Servers".to_string(),
            parent_id: None,
            sort_order: 0,
            created_at_ms: None,
            updated_at_ms: None,
        };
        let connection = SavedConnection {
            id: "local-grouped".to_string(),
            name: "Local".to_string(),
            config: ConnectionType::LocalTerminal {
                shell_path: "bash".to_string(),
                shell_args: String::new(),
                working_dir: None,
                ai_execution_profile: Default::default(),
            },
            group_id: Some(group.id.clone()),
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
        };

        store
            .save_group_and_connection(&group, &connection)
            .expect("save group and connection");

        assert_eq!(store.list_groups().expect("groups")[0].id, group.id);
        assert_eq!(
            store
                .get_connection(&connection.id)
                .expect("connection")
                .expect("saved connection")
                .group_id
                .as_deref(),
            Some("group-1")
        );

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn deleting_group_removes_descendants_and_grouped_connections() {
        let dir = unique_temp_dir("delete-group-tree");
        let store = ConnectionStore::open(&dir).expect("store");
        let root = Group {
            id: "root".to_string(),
            name: "Root".to_string(),
            parent_id: None,
            sort_order: 0,
            created_at_ms: None,
            updated_at_ms: None,
        };
        let child = Group {
            id: "child".to_string(),
            name: "Child".to_string(),
            parent_id: Some(root.id.clone()),
            sort_order: 0,
            created_at_ms: None,
            updated_at_ms: None,
        };
        for group in [&root, &child] {
            store.save_group(group).expect("save group");
        }
        for (id, group_id) in [
            ("root-connection", root.id.clone()),
            ("child-connection", child.id.clone()),
        ] {
            store
                .save_connection(&SavedConnection {
                    id: id.to_string(),
                    name: id.to_string(),
                    config: ConnectionType::LocalTerminal {
                        shell_path: "bash".to_string(),
                        shell_args: String::new(),
                        working_dir: None,
                        ai_execution_profile: Default::default(),
                    },
                    group_id: Some(group_id),
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
                })
                .expect("save connection");
        }

        store.delete_group(&root.id).expect("delete group tree");

        assert!(store.list_groups().expect("groups").is_empty());
        assert!(store.list_connections().expect("connections").is_empty());
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
            icon_auto_detect: None,
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
    fn save_ssh_key_rejects_oversized_key_file_import() {
        let dir = unique_temp_dir("ssh-key-large-key-file");
        std::fs::create_dir_all(&dir).expect("dir");
        let key_path = dir.join("too-large.key");
        let file = std::fs::File::create(&key_path).expect("create key file");
        file.set_len(SSH_KEY_FILE_IMPORT_MAX_BYTES + 1)
            .expect("grow key file");

        let store = ConnectionStore::open(&dir).expect("store");
        let error = store
            .save_ssh_key(SshKey {
                id: "key-1".to_string(),
                name: "Large Key".to_string(),
                key: None,
                cert: None,
                passphrase: None,
                key_file_path: Some(key_path.display().to_string()),
                cert_file_path: None,
                has_key_data: false,
                has_cert_data: false,
            })
            .expect_err("large key import should fail");

        assert!(error.to_string().contains("key material file is too large"));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn save_ssh_key_rejects_oversized_cert_file_import() {
        let dir = unique_temp_dir("ssh-key-large-cert-file");
        std::fs::create_dir_all(&dir).expect("dir");
        let cert_path = dir.join("too-large-cert.pub");
        let file = std::fs::File::create(&cert_path).expect("create cert file");
        file.set_len(SSH_KEY_FILE_IMPORT_MAX_BYTES + 1)
            .expect("grow cert file");

        let store = ConnectionStore::open(&dir).expect("store");
        let error = store
            .save_ssh_key(SshKey {
                id: "key-1".to_string(),
                name: "Large Cert".to_string(),
                key: Some("-----BEGIN PRIVATE KEY-----\nsmall\n".to_string()),
                cert: None,
                passphrase: None,
                key_file_path: None,
                cert_file_path: Some(cert_path.display().to_string()),
                has_key_data: false,
                has_cert_data: false,
            })
            .expect_err("large cert import should fail");

        assert!(error.to_string().contains("certificate file is too large"));
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
    fn save_appearance_theme_and_contrast_roundtrip() {
        let dir = unique_temp_dir("settings-appearance-extra");
        let store = ConnectionStore::open(&dir).expect("store");
        let mut summary = store.load_app_settings_summary().expect("load");
        summary.theme = "dracula".to_string();
        summary.terminal_theme = Some("nord".to_string());
        summary.minimum_contrast_ratio = "4.5".to_string();
        summary.background_image_opacity = 0;
        summary.background_content_opacity = 0;
        summary.ui_font_family = "Segoe UI, Inter".to_string();
        summary.terminal_font_family = "JetBrains Mono, monospace".to_string();
        summary.ui_font_size = 18;
        summary.terminal_font_weight = 500;
        summary.terminal_font_weight_bold = 800;
        let saved = store.save_appearance_settings(&summary).expect("save");
        assert_eq!(saved.theme, "dracula");
        assert_eq!(saved.terminal_theme.as_deref(), Some("nord"));
        assert_eq!(saved.minimum_contrast_ratio, "4.5");
        assert_eq!(saved.background_image_opacity, 0);
        assert_eq!(saved.background_content_opacity, 0);
        assert_eq!(saved.ui_font_family, "Segoe UI, Inter");
        assert_eq!(saved.terminal_font_family, "JetBrains Mono, monospace");
        assert_eq!(saved.ui_font_size, 18);
        assert_eq!(saved.terminal_font_weight, 500);
        assert_eq!(saved.terminal_font_weight_bold, 800);
        let raw = store.load_settings_value().expect("raw");
        assert_eq!(
            raw["appearance"]["terminal_theme"],
            serde_json::Value::String("nord".into())
        );
        assert_eq!(
            raw["appearance"]["minimum_contrast_ratio"],
            serde_json::json!(4.5)
        );
        assert_eq!(
            raw["appearance"]["ui_font_family"],
            serde_json::Value::String("Segoe UI, Inter".into())
        );
        assert_eq!(
            raw["appearance"]["font_family"],
            serde_json::Value::String("JetBrains Mono, monospace".into())
        );
        assert_eq!(
            raw["appearance"]["background_image_opacity"],
            serde_json::json!(0.0)
        );
        assert_eq!(
            raw["appearance"]["background_opacity"],
            serde_json::json!(0.0)
        );
        assert_eq!(raw["appearance"]["ui_font_size"], serde_json::json!(18));
        assert_eq!(raw["appearance"]["font_weight"], serde_json::json!(500));
        assert_eq!(
            raw["appearance"]["font_weight_bold"],
            serde_json::json!(800)
        );
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn save_empty_search_engine_list_roundtrip() {
        let dir = unique_temp_dir("settings-empty-search-engines");
        let store = ConnectionStore::open(&dir).expect("store");
        let mut summary = store.load_app_settings_summary().expect("load");
        assert!(!summary.search_custom_engines.is_empty());

        summary.search_custom_engines.clear();
        let saved = store.save_terminal_settings(&summary).expect("save");
        assert!(saved.search_custom_engines.is_empty());

        let raw = store.load_settings_value().expect("raw");
        assert_eq!(raw["search"]["custom_engines"], serde_json::json!([]));

        summary.search_custom_engines = vec![SearchEngineConfig {
            name: String::new(),
            url_template: String::new(),
            icon: None,
            show_in_menu: true,
        }];
        let saved = store
            .save_terminal_settings(&summary)
            .expect("save blank engine");
        assert_eq!(saved.search_custom_engines.len(), 1);
        assert!(saved.search_custom_engines[0].name.is_empty());
        assert!(saved.search_custom_engines[0].url_template.is_empty());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn save_general_and_diagnostics_settings_roundtrip() {
        let dir = unique_temp_dir("settings-general-diag");
        let store = ConnectionStore::open(&dir).expect("store");
        let mut summary = store.load_app_settings_summary().expect("load");
        summary.startup_restore = true;
        summary.startup_restore_window_layout = false;
        summary.minimize_to_tray = true;
        summary.confirm_on_close = false;
        summary.language = "zh-CN".to_string();
        let saved = store.save_general_settings(&summary).expect("save general");
        assert!(saved.startup_restore);
        assert!(!saved.startup_restore_window_layout);
        assert!(saved.minimize_to_tray);
        assert!(!saved.confirm_on_close);
        assert_eq!(saved.language, "zh-CN");

        summary = saved;
        summary.diagnostics_level = "debug".to_string();
        summary.diagnostics_retention_days = 14;
        let saved = store
            .save_diagnostics_settings(&summary)
            .expect("save diagnostics");
        assert_eq!(saved.diagnostics_level, "debug");
        assert_eq!(saved.diagnostics_retention_days, 14);

        let raw = store.load_settings_value().expect("raw");
        assert_eq!(
            raw["general"]["minimize_to_tray"],
            serde_json::Value::Bool(true)
        );
        assert_eq!(
            raw["ui"]["language"],
            serde_json::Value::String("zh-CN".into())
        );
        assert_eq!(
            raw["diagnostics"]["level"],
            serde_json::Value::String("debug".into())
        );
        assert_eq!(
            raw["diagnostics"]["retention_days"],
            serde_json::Value::from(14)
        );

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn save_ui_layout_bottom_panel_state_roundtrip_and_clamp() {
        let dir = unique_temp_dir("settings-bottom-panel-heights");
        let store = ConnectionStore::open(&dir).expect("store");
        let mut summary = store.load_app_settings_summary().expect("load");
        summary.ui_quick_cmd_height = 312;
        summary.ui_quick_cmd_visible = false;
        summary.ui_serial_send_height = 284;
        summary.ui_serial_send_visible = true;

        let saved = store.save_ui_layout_settings(&summary).expect("save");
        assert_eq!(saved.ui_quick_cmd_height, 312);
        assert!(!saved.ui_quick_cmd_visible);
        assert_eq!(saved.ui_serial_send_height, 284);
        assert!(saved.ui_serial_send_visible);
        let raw = store.load_settings_value().expect("raw");
        assert_eq!(raw["ui"]["quick_cmd_height"], serde_json::json!(312));
        assert_eq!(raw["ui"]["show_quick_cmd_bar"], serde_json::json!(false));
        assert_eq!(raw["ui"]["serial_send_height"], serde_json::json!(284));
        assert_eq!(raw["ui"]["show_serial_send_panel"], serde_json::json!(true));

        summary.ui_quick_cmd_height = 0;
        summary.ui_serial_send_height = 999;
        let clamped = store
            .save_ui_layout_settings(&summary)
            .expect("save clamped");
        assert_eq!(clamped.ui_quick_cmd_height, 36);
        assert_eq!(clamped.ui_serial_send_height, 520);

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn save_ui_layout_preserves_explicit_empty_activity_zone() {
        let dir = unique_temp_dir("settings-empty-activity-zone");
        let store = ConnectionStore::open(&dir).expect("store");
        let mut summary = store.load_app_settings_summary().expect("load");
        let moved = std::mem::take(&mut summary.ui_activity_bar_left_top);
        summary.ui_activity_bar_right_top.extend(moved);
        summary.ui_saved_connections_sort_mode = "name-desc".to_string();

        let saved = store.save_ui_layout_settings(&summary).expect("save");
        assert!(saved.ui_activity_bar_left_top.is_empty());
        assert_eq!(saved.ui_saved_connections_sort_mode, "name-desc");
        assert!(
            saved
                .ui_activity_bar_right_top
                .iter()
                .any(|id| id == "fileExplorer")
        );
        let raw = store.load_settings_value().expect("raw");
        assert_eq!(
            raw["ui"]["activity_bar_layout"]["left_top"],
            serde_json::json!([])
        );

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn legacy_settings_default_header_status_to_visible_session() {
        let dir = unique_temp_dir("settings-header-status-legacy-defaults");
        let store = ConnectionStore::open(&dir).expect("store");

        let summary = store.load_app_settings_summary().expect("load");
        assert_eq!(summary.ui_header_status_mode, "session");
        assert!(summary.ui_header_status_visible);

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn header_status_settings_roundtrip_and_preserve_unknown_ui_fields() {
        let dir = unique_temp_dir("settings-header-status-roundtrip");
        let store = ConnectionStore::open(&dir).expect("store");
        store
            .save_settings_value(&serde_json::json!({
                "ui": {
                    "header_status_mode": "host",
                    "header_status_visible": false,
                    "future_header_option": { "keep": true }
                }
            }))
            .expect("seed settings");

        let mut summary = store.load_app_settings_summary().expect("load");
        assert_eq!(summary.ui_header_status_mode, "host");
        assert!(!summary.ui_header_status_visible);

        summary.ui_header_status_mode = "resources".to_string();
        summary.ui_header_status_visible = true;
        let saved = store.save_ui_layout_settings(&summary).expect("save");
        assert_eq!(saved.ui_header_status_mode, "resources");
        assert!(saved.ui_header_status_visible);

        let raw = store.load_settings_value().expect("raw");
        assert_eq!(raw["ui"]["header_status_mode"], "resources");
        assert_eq!(raw["ui"]["header_status_visible"], true);
        assert_eq!(raw["ui"]["future_header_option"]["keep"], true);

        summary.ui_header_status_mode = "unsupported".to_string();
        let normalized = store
            .save_ui_layout_settings(&summary)
            .expect("save normalized");
        assert_eq!(normalized.ui_header_status_mode, "session");

        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn app_settings_summary_reads_and_updates_host_key_policy() {
        let dir = unique_temp_dir("settings-summary");
        let store = ConnectionStore::open(&dir).expect("store");
        let initial = serde_json::json!({
            "general": {
                "startup_restore": true,
                "minimize_to_tray": true,
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
                "paste_image_as_path": false,
                "low_latency_mode": true
            },
            "ui": {
                "language": "zh-CN",
                "show_remote_stats": false,
                "remote_stats_interval": 9,
                "show_process_manager": false,
                "process_manager_interval": 11,
                "show_docker_manager": false,
                "docker_manager_interval": 13,
                "quick_cmd_view_mode": "compact",
                "quick_cmd_sort_mode": "useCount",
                "file_explorer_show_hidden_files": false,
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
        assert_eq!(summary.language, "zh-CN");
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
        assert!(summary.terminal_low_latency_mode);
        assert!(!summary.ui_show_remote_stats);
        assert_eq!(summary.ui_remote_stats_interval, 9);
        assert!(!summary.ui_show_process_manager);
        assert_eq!(summary.ui_process_manager_interval, 11);
        assert!(!summary.ui_show_docker_manager);
        assert_eq!(summary.ui_docker_manager_interval, 13);
        assert_eq!(summary.ui_quick_cmd_view_mode, "compact");
        assert_eq!(summary.ui_quick_cmd_sort_mode, "useCount");
        assert!(!summary.ui_file_explorer_show_hidden_files);
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
        assert!(summary.minimize_to_tray);
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
        favorite_update.ui_file_explorer_show_hidden_files = true;
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
        assert!(updated.ui_file_explorer_show_hidden_files);
        assert_eq!(
            updated
                .ui_file_explorer_favorite_dirs_by_connection_id
                .get("conn-2"),
            Some(&vec!["/data".to_string(), "/logs".to_string()])
        );
        let stored = store.load_settings_value().expect("stored favorites");
        assert_eq!(
            json_path(&stored, &["ui", "file_explorer_show_hidden_files"])
                .and_then(|value| value.as_bool()),
            Some(true)
        );
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
        terminal_update.terminal_low_latency_mode = true;
        terminal_update.ui_show_remote_stats = true;
        terminal_update.ui_remote_stats_interval = 4;
        let saved_terminal = store
            .save_terminal_settings(&terminal_update)
            .expect("save terminal settings");
        assert_eq!(saved_terminal.terminal_scrollback_lines, 12_000);
        assert_eq!(saved_terminal.terminal_keep_alive_interval, 20);
        assert!(saved_terminal.terminal_show_multi_line_paste_dialog);
        assert!(saved_terminal.terminal_low_latency_mode);
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
        assert_eq!(
            json_path(&stored, &["terminal", "low_latency_mode"]).and_then(|value| value.as_bool()),
            Some(true)
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
    fn workspace_pane_layout_roundtrip() {
        let dir = unique_temp_dir("workspace-pane-layout");
        let store = ConnectionStore::open(&dir).expect("store");
        let layout = crate::models::RestorableWorkspacePaneNode::Split {
            id: "split-1".to_string(),
            direction: "vertical".to_string(),
            ratio: 0.4,
            first: Box::new(crate::models::RestorableWorkspacePaneNode::Leaf { tab_index: 0 }),
            second: Box::new(crate::models::RestorableWorkspacePaneNode::Leaf { tab_index: 1 }),
        };
        store
            .save_workspace_pane_layout(Some(&layout))
            .expect("save");
        let loaded = store
            .load_workspace_pane_layout()
            .expect("load")
            .expect("some");
        assert_eq!(loaded, layout);
        store.save_workspace_pane_layout(None).expect("clear");
        assert!(store.load_workspace_pane_layout().expect("load").is_none());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn open_tab_pane_root_expands_and_maps_layout() {
        let tab = crate::models::RestorableOpenTab {
            title: "split".to_string(),
            session_type: "SSH".to_string(),
            connection_id: Some("c1".to_string()),
            custom_name: Some("g".to_string()),
            tab_color: Some("#ff0000".to_string()),
            active_pane_id: None,
            root: Some(crate::models::RestorablePaneNode::Split {
                id: "s".to_string(),
                direction: "horizontal".to_string(),
                ratio: 0.4,
                first: Box::new(crate::models::RestorablePaneNode::leaf_session(
                    "a",
                    "SSH",
                    Some("c1".to_string()),
                )),
                second: Box::new(crate::models::RestorablePaneNode::leaf_session(
                    "b", "Local", None,
                )),
            }),
        };
        let sessions = tab.expanded_sessions();
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].title, "a");
        assert_eq!(sessions[1].session_type, "Local");
        let layout = tab.workspace_pane_layout_from_root(3).expect("layout");
        match layout {
            crate::models::RestorableWorkspacePaneNode::Split {
                direction,
                ratio,
                first,
                second,
                ..
            } => {
                assert_eq!(direction, "horizontal");
                assert!((ratio - 0.4).abs() < f64::EPSILON);
                assert_eq!(
                    *first,
                    crate::models::RestorableWorkspacePaneNode::Leaf { tab_index: 3 }
                );
                assert_eq!(
                    *second,
                    crate::models::RestorableWorkspacePaneNode::Leaf { tab_index: 4 }
                );
            }
            _ => panic!("expected split"),
        }
    }

    #[test]
    fn open_tabs_roundtrip() {
        let dir = unique_temp_dir("open-tabs");
        let store = ConnectionStore::open(&dir).expect("store");
        let tabs = vec![
            crate::models::RestorableOpenTab::with_leaf_root(
                "Local",
                "Local",
                None,
                Some("dev".to_string()),
                Some("#22c55e".to_string()),
            ),
            crate::models::RestorableOpenTab {
                title: "prod".to_string(),
                session_type: "SSH".to_string(),
                connection_id: Some("conn-1".to_string()),
                custom_name: None,
                tab_color: None,
                active_pane_id: None,
                root: Some(crate::models::RestorablePaneNode::Split {
                    id: "split-1".to_string(),
                    direction: "vertical".to_string(),
                    ratio: 0.5,
                    first: Box::new(crate::models::RestorablePaneNode::leaf_session(
                        "prod-a",
                        "SSH",
                        Some("conn-1".to_string()),
                    )),
                    second: Box::new(crate::models::RestorablePaneNode::leaf_session(
                        "prod-b",
                        "SSH",
                        Some("conn-1".to_string()),
                    )),
                }),
            },
        ];
        store.save_open_tabs(&tabs).expect("save");
        let loaded = store.load_open_tabs().expect("load");
        assert_eq!(loaded, tabs);
        store.save_open_tabs(&[]).expect("clear");
        assert!(store.load_open_tabs().expect("load empty").is_empty());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn terminal_window_layout_roundtrip() {
        let dir = unique_temp_dir("terminal-window-layout");
        let store = ConnectionStore::open(&dir).expect("store");
        let layout = crate::models::RestorableTerminalWindowNode::Split {
            direction: "vertical".to_string(),
            ratio: 0.45,
            first: Box::new(crate::models::RestorableTerminalWindowNode::Leaf {
                tab_indexes: vec![0, 1],
                active_tab_index: Some(0),
            }),
            second: Box::new(crate::models::RestorableTerminalWindowNode::Leaf {
                tab_indexes: vec![2],
                active_tab_index: Some(2),
            }),
        };
        store
            .save_terminal_window_layout(Some(&layout))
            .expect("save layout");
        let loaded = store
            .load_terminal_window_layout()
            .expect("load layout")
            .expect("some layout");
        assert_eq!(loaded, layout);
        store.save_terminal_window_layout(None).expect("clear");
        assert!(store.load_terminal_window_layout().expect("load").is_none());
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
    fn changing_master_password_preserves_encrypted_cloud_secrets() {
        let dir = unique_temp_dir("change-master-password");
        let store = ConnectionStore::open(&dir).expect("store");
        let mut cloud = CloudSyncSettings::default();
        cloud.webdav.password = Some("cloud-secret".to_string());
        store
            .save_cloud_sync_settings(cloud)
            .expect("save cloud secret");

        let summary = store
            .save_master_password(Some("first-password"))
            .expect("set password");
        assert!(summary.has_master_password);
        assert!(
            store
                .verify_master_password("first-password")
                .expect("verify")
        );
        assert_eq!(
            store
                .load_cloud_sync_settings()
                .expect("load after setting password")
                .webdav
                .password
                .as_deref(),
            Some("cloud-secret")
        );

        store
            .save_master_password(Some("second-password"))
            .expect("change password");
        assert!(
            store
                .verify_master_password("second-password")
                .expect("verify")
        );
        assert_eq!(
            store
                .load_cloud_sync_settings()
                .expect("load after changing password")
                .webdav
                .password
                .as_deref(),
            Some("cloud-secret")
        );

        let summary = store.save_master_password(None).expect("remove password");
        assert!(!summary.has_master_password);
        assert_eq!(
            store
                .load_cloud_sync_settings()
                .expect("load after removing password")
                .webdav
                .password
                .as_deref(),
            Some("cloud-secret")
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
        // Blank-name + patterns becomes "Untitled rule"; blank-pattern named draft is kept.
        assert_eq!(loaded.rules.len(), 3);
        assert_eq!(loaded.rules[0].id, "panic");
        assert_eq!(loaded.rules[0].patterns, vec!["panic", "ERROR"]);
        assert_eq!(loaded.rules[1].id, "invalid-empty-pattern");
        assert_eq!(loaded.rules[2].name, "Untitled rule");

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
        assert_eq!(result.total_rules, 4);
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
        assert_eq!(result.total_rules, 4);
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

    pub(super) fn unique_temp_dir(name: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("nyaterm-core-{name}-{}-{n}", std::process::id()));
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

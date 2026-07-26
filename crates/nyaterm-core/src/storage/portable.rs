//! Portable snapshot and config-database import/export.
//!
//! Split out of `storage.rs` by domain. The on-disk snapshot layout, the
//! encrypted envelope and the merge rules for imported settings are
//! unchanged; this only moves the code.

use std::path::{Path, PathBuf};

use redb::TableDefinition;
use serde::de::DeserializeOwned;

use super::{
    CREDENTIAL_PREFIX, CREDENTIALS_TABLE, ConnectionStore, DATABASE_FILE, LEGACY_TEXT_MASTER_KEY,
    META_MASTER_KEY, META_TABLE, OTP_ACCOUNTS_TABLE, OTP_PREFIX, PASSWORD_PREFIX, PROXIES_TABLE,
    PROXY_PREFIX, SETTINGS_DEFAULT, SETTINGS_PROXY_GROUPS, SETTINGS_QUICK_COMMANDS, SETTINGS_TABLE,
    SETTINGS_TUNNEL_GROUPS, SSH_KEY_PREFIX, StorageError, TEXT_DOCS_TABLE, TUNNEL_PREFIX,
    TUNNELS_TABLE, clear_prefix_in_txn, copy_config_database, current_time_ms,
    ensure_not_same_existing_file, ensure_parent_dir, entity_key, replace_command_history_in_txn,
    replace_known_hosts_text_in_txn, replace_sessions_in_txn, set_nested_json_value,
    validate_config_backup_file, validate_config_backup_source, write_json_in_txn,
    write_portable_snapshot_file,
};
use crate::{
    CommandHistoryEntry, ConfigBackupInfo, PortableSnapshotKind, RawPortableSnapshot,
    SessionsConfig, TunnelGroup,
};

impl ConnectionStore {
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

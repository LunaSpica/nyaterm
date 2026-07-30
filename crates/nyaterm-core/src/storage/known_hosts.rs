//! `known_hosts` persistence and host-key verification.
//!
//! Split out of `storage.rs` by domain. Table name, key layout, record shape
//! and the hashed-host matching rules are unchanged; this only moves the code.

use hmac::{Hmac, Mac, digest::KeyInit as HmacKeyInit};
use redb::ReadableTable;
use serde::{Deserialize, Serialize};
use sha1::Sha1;

use super::{
    ConnectionStore, KNOWN_HOST_PREFIX, KNOWN_HOST_RAW_PREFIX, KNOWN_HOSTS_TABLE,
    LEGACY_TEXT_KNOWN_HOSTS, StorageError, TEXT_DOCS_TABLE, clear_prefix_in_txn, current_time_ms,
    deserialize_json, stable_id, write_json_in_txn,
};
use base64::{Engine, engine::general_purpose::STANDARD as B64};

type HmacSha1 = Hmac<Sha1>;

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

impl ConnectionStore {
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
    pub(super) fn import_legacy_known_hosts_if_needed(&self) -> Result<(), StorageError> {
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
}

pub(super) fn replace_known_hosts_text_in_txn(
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
    if let Some(comment) = &record.comment
        && !comment.is_empty()
    {
        line.push(' ');
        line.push_str(comment);
    }
    line
}

//! Import sessions from Xshell (.xts), MobaXterm (.mxtsessions), WindTerm (.sessions),
//! SecureCRT (.xml), FinalShell conn directories, NyaTerm JSON files, Electerm
//! bookmarks, and Termius IndexedDB data.

use crate::{AiExecutionProfile, ConnectionAuth, ConnectionType, SavedPassword, SshKey};
use serde::Deserialize;
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use thiserror::Error;

mod electerm;
mod finalshell;
mod securecrt;
mod termius;

const SESSION_IMPORT_MAX_BYTES: u64 = 16 * 1024 * 1024;
const XSHELL_ARCHIVE_MAX_BYTES: u64 = 64 * 1024 * 1024;
const XSHELL_ENTRY_MAX_BYTES: u64 = 1024 * 1024;
const XSHELL_ENTRY_LIMIT: usize = 10_000;

#[derive(Debug, Error)]
pub enum SessionImportError {
    #[error("{0}")]
    Config(String),
    #[error("{0}")]
    Crypto(String),
}

type AppError = SessionImportError;
type AppResult<T> = Result<T, SessionImportError>;

struct ImportedSession {
    name: String,
    host: String,
    port: u16,
    username: String,
    auth_type: String,
    /// Hierarchical group path segments, e.g. ["Production", "Web"].
    group_path: Option<Vec<String>>,
    description: Option<String>,
}

pub struct PreparedSessionConnection {
    pub name: String,
    pub config: ConnectionType,
    pub group_path: Option<Vec<String>>,
    pub description: Option<String>,
    pub sort_order: i32,
    pub icon: Option<String>,
    pub auth: Option<ConnectionAuth>,
}

pub struct PreparedSessionImport {
    pub groups: Vec<Vec<String>>,
    pub passwords: Vec<SavedPassword>,
    pub ssh_keys: Vec<SshKey>,
    pub connections: Vec<PreparedSessionConnection>,
}

type PreparedJsonConnection = PreparedSessionConnection;
type PreparedJsonImport = PreparedSessionImport;

impl std::fmt::Debug for PreparedSessionImport {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PreparedSessionImport")
            .field("group_count", &self.groups.len())
            .field("password_count", &self.passwords.len())
            .field("ssh_key_count", &self.ssh_keys.len())
            .field("connection_count", &self.connections.len())
            .finish()
    }
}

#[derive(Deserialize)]
struct NyatermJsonImportFile {
    #[serde(default = "default_import_version")]
    version: u32,
    #[serde(default)]
    passwords: Vec<NyatermJsonPassword>,
    #[serde(default)]
    ssh_keys: Vec<NyatermJsonSshKey>,
    #[serde(default)]
    groups: Vec<NyatermJsonGroup>,
    #[serde(default)]
    sessions: Vec<NyatermJsonSession>,
}

fn default_import_version() -> u32 {
    1
}

#[derive(Deserialize)]
struct NyatermJsonPassword {
    #[serde(rename = "ref")]
    ref_name: String,
    name: String,
    password: String,
}

#[derive(Deserialize)]
struct NyatermJsonSshKey {
    #[serde(rename = "ref")]
    ref_name: String,
    name: String,
    private_key: String,
    #[serde(default)]
    certificate: Option<String>,
    #[serde(default)]
    passphrase: Option<String>,
}

#[derive(Deserialize)]
struct NyatermJsonGroup {
    path: Vec<String>,
}

#[derive(Deserialize)]
struct NyatermJsonSshAuth {
    mode: String,
    #[serde(default)]
    password: Option<String>,
    #[serde(default)]
    password_ref: Option<String>,
    #[serde(default)]
    key_ref: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum NyatermJsonSession {
    Ssh {
        name: String,
        #[serde(default)]
        group_path: Vec<String>,
        host: String,
        #[serde(default = "default_ssh_port")]
        port: u16,
        #[serde(default = "default_ssh_user")]
        username: String,
        #[serde(default)]
        auth: Option<NyatermJsonSshAuth>,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        sort_order: i32,
        #[serde(default)]
        icon: Option<String>,
    },
    LocalTerminal {
        name: String,
        #[serde(default)]
        group_path: Vec<String>,
        #[serde(default)]
        shell_path: String,
        #[serde(default)]
        shell_args: String,
        #[serde(default)]
        working_dir: Option<String>,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        sort_order: i32,
        #[serde(default)]
        icon: Option<String>,
    },
    Telnet {
        name: String,
        #[serde(default)]
        group_path: Vec<String>,
        host: String,
        #[serde(default = "default_telnet_port")]
        port: u16,
        #[serde(default = "default_telnet_backspace_mode")]
        backspace_mode: String,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        sort_order: i32,
        #[serde(default)]
        icon: Option<String>,
    },
    Serial {
        name: String,
        #[serde(default)]
        group_path: Vec<String>,
        port_name: String,
        #[serde(default = "default_serial_baud_rate")]
        baud_rate: u32,
        #[serde(default = "default_serial_data_bits")]
        data_bits: u8,
        #[serde(default = "default_serial_parity")]
        parity: String,
        #[serde(default = "default_serial_stop_bits")]
        stop_bits: String,
        #[serde(default = "default_serial_backspace_mode")]
        backspace_mode: String,
        #[serde(default)]
        description: Option<String>,
        #[serde(default)]
        sort_order: i32,
        #[serde(default)]
        icon: Option<String>,
    },
}

fn default_ssh_port() -> u16 {
    22
}

fn default_ssh_user() -> String {
    "root".to_string()
}

fn default_telnet_port() -> u16 {
    23
}

fn default_telnet_backspace_mode() -> String {
    "del".to_string()
}

fn default_serial_baud_rate() -> u32 {
    115_200
}

fn default_serial_data_bits() -> u8 {
    8
}

fn default_serial_parity() -> String {
    "none".to_string()
}

fn default_serial_stop_bits() -> String {
    "1".to_string()
}

fn default_serial_backspace_mode() -> String {
    "ctrl_h".to_string()
}

/// Detect BOM (UTF-8/UTF-16) and decode accordingly; fall back to GBK.
fn decode_bytes(raw: &[u8]) -> String {
    if let Some((enc, bom_len)) = encoding_rs::Encoding::for_bom(raw) {
        let (decoded, _, _) = enc.decode(&raw[bom_len..]);
        return decoded.into_owned();
    }
    match std::str::from_utf8(raw) {
        Ok(s) => s.to_string(),
        Err(_) => {
            let (decoded, _, _) = encoding_rs::GBK.decode(raw);
            decoded.into_owned()
        }
    }
}

fn read_file_limited(path: impl AsRef<Path>, label: &str, max_bytes: u64) -> AppResult<Vec<u8>> {
    let file = std::fs::File::open(path)
        .map_err(|error| AppError::Config(format!("Cannot open {label}: {error}")))?;
    let size = file
        .metadata()
        .map_err(|error| AppError::Config(format!("Cannot inspect {label}: {error}")))?
        .len();
    if size > max_bytes {
        return Err(AppError::Config(format!(
            "{label} exceeds the {} MiB import limit",
            max_bytes / (1024 * 1024)
        )));
    }

    let mut raw = Vec::with_capacity(size as usize);
    file.take(max_bytes + 1)
        .read_to_end(&mut raw)
        .map_err(|error| AppError::Config(format!("Cannot read {label}: {error}")))?;
    if raw.len() as u64 > max_bytes {
        return Err(AppError::Config(format!(
            "{label} exceeds the {} MiB import limit",
            max_bytes / (1024 * 1024)
        )));
    }
    Ok(raw)
}

fn read_text_file_limited(path: impl AsRef<Path>, label: &str) -> AppResult<String> {
    let raw = read_file_limited(path, label, SESSION_IMPORT_MAX_BYTES)?;
    String::from_utf8(raw)
        .map_err(|error| AppError::Config(format!("{label} is not valid UTF-8: {error}")))
}

// Xshell (.xts)

fn parse_xshell(path: &str) -> AppResult<Vec<ImportedSession>> {
    let file = std::fs::File::open(path)
        .map_err(|e| AppError::Config(format!("Cannot open file: {e}")))?;
    let archive_size = file
        .metadata()
        .map_err(|error| AppError::Config(format!("Cannot inspect Xshell archive: {error}")))?
        .len();
    if archive_size > XSHELL_ARCHIVE_MAX_BYTES {
        return Err(AppError::Config(format!(
            "Xshell archive exceeds the {} MiB import limit",
            XSHELL_ARCHIVE_MAX_BYTES / (1024 * 1024)
        )));
    }
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| AppError::Config(format!("Invalid ZIP/XTS file: {e}")))?;
    if archive.len() > XSHELL_ENTRY_LIMIT {
        return Err(AppError::Config(format!(
            "Xshell archive contains more than {XSHELL_ENTRY_LIMIT} entries"
        )));
    }

    let mut sessions = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| AppError::Config(format!("ZIP entry error: {e}")))?;

        // ZIP filenames on Chinese Windows are typically GBK-encoded
        let entry_path_raw = entry.name_raw().to_vec();
        let entry_path = decode_bytes(&entry_path_raw);
        if !entry_path.ends_with(".xsh") {
            continue;
        }
        if entry.size() > XSHELL_ENTRY_MAX_BYTES {
            return Err(AppError::Config(format!(
                "Xshell session entry '{entry_path}' exceeds the 1 MiB import limit"
            )));
        }

        let mut raw = Vec::new();
        (&mut entry)
            .take(XSHELL_ENTRY_MAX_BYTES + 1)
            .read_to_end(&mut raw)
            .map_err(|e| AppError::Config(format!("Failed to read {entry_path}: {e}")))?;
        if raw.len() as u64 > XSHELL_ENTRY_MAX_BYTES {
            return Err(AppError::Config(format!(
                "Xshell session entry '{entry_path}' exceeds the 1 MiB import limit"
            )));
        }

        let content = decode_bytes(&raw);

        if let Some(sess) = parse_xsh_content(&content, &entry_path) {
            sessions.push(sess);
        }
    }

    Ok(sessions)
}

fn parse_xsh_content(content: &str, entry_path: &str) -> Option<ImportedSession> {
    let sections = parse_ini_sections(content);

    let conn = sections.get("CONNECTION")?;
    let protocol = conn.get("Protocol").map(String::as_str).unwrap_or("");
    if !protocol.eq_ignore_ascii_case("SSH") {
        return None;
    }

    let host = conn.get("Host")?.clone();
    if host.is_empty() {
        return None;
    }

    let port: u16 = conn.get("Port").and_then(|p| p.parse().ok()).unwrap_or(22);

    let auth = sections.get("CONNECTION:AUTHENTICATION");
    let username = auth
        .and_then(|a| a.get("UserName"))
        .cloned()
        .unwrap_or_else(|| "root".to_string());

    let has_user_key = auth
        .and_then(|a| a.get("UserKey"))
        .is_some_and(|k| !k.is_empty());
    let auth_type = if has_user_key { "key" } else { "password" }.to_string();

    let path_obj = Path::new(entry_path);
    let name = path_obj
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Unnamed")
        .to_string();

    let group_path = path_obj.parent().and_then(|p| {
        let p_str = p.to_str().unwrap_or("");
        let stripped = p_str
            .strip_prefix("Xshell/Sessions/")
            .or_else(|| p_str.strip_prefix("Xshell/"))
            .unwrap_or(p_str);
        if stripped.is_empty() {
            None
        } else {
            let segments: Vec<String> = stripped
                .split('/')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
            if segments.is_empty() {
                None
            } else {
                Some(segments)
            }
        }
    });

    Some(ImportedSession {
        name,
        host,
        port,
        username,
        auth_type,
        group_path,
        description: None,
    })
}

fn parse_ini_sections(content: &str) -> HashMap<String, HashMap<String, String>> {
    let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current_section = String::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].to_string();
            sections.entry(current_section.clone()).or_default();
        } else if let Some((key, value)) = line.split_once('=')
            && let Some(section) = sections.get_mut(&current_section)
        {
            section.insert(key.trim().to_string(), value.trim().to_string());
        }
    }

    sections
}

// MobaXterm (.mxtsessions)

fn parse_mobaxterm(path: &str) -> AppResult<Vec<ImportedSession>> {
    let raw = read_file_limited(path, "MobaXterm session file", SESSION_IMPORT_MAX_BYTES)?;
    let content = decode_bytes(&raw);

    let sections = parse_ini_sections(&content);
    let mut sessions = Vec::new();

    for (section_name, entries) in &sections {
        if !section_name.starts_with("Bookmarks") {
            continue;
        }

        let group_path = entries.get("SubRep").and_then(|s| {
            let s = s.trim();
            if s.is_empty() {
                None
            } else {
                let segments: Vec<String> = s
                    .split('\\')
                    .filter(|seg| !seg.is_empty())
                    .map(|seg| seg.trim().to_string())
                    .collect();
                if segments.is_empty() {
                    None
                } else {
                    Some(segments)
                }
            }
        });

        for (entry_name, value) in entries {
            if entry_name == "SubRep" || entry_name == "ImgNum" {
                continue;
            }

            if let Some(sess) = parse_moba_entry(entry_name, value, &group_path) {
                sessions.push(sess);
            }
        }
    }

    Ok(sessions)
}

fn parse_moba_entry(
    name: &str,
    value: &str,
    group_path: &Option<Vec<String>>,
) -> Option<ImportedSession> {
    // Format: #<type>#<subtype>%host%port%username%...
    let hash_parts: Vec<&str> = value.splitn(2, '#').skip(1).collect::<Vec<_>>();
    if hash_parts.is_empty() {
        return None;
    }

    let after_hash = hash_parts.join("#");
    let type_and_rest: Vec<&str> = after_hash.splitn(2, '%').collect();
    if type_and_rest.len() < 2 {
        return None;
    }

    // Type 109 = SSH
    let type_marker = type_and_rest[0];
    if !type_marker.starts_with("109") {
        return None;
    }

    let fields: Vec<&str> = type_and_rest[1].split('%').collect();
    if fields.len() < 3 {
        return None;
    }

    let host = fields[0].to_string();
    if host.is_empty() {
        return None;
    }

    let port: u16 = fields[1].parse().unwrap_or(22);
    let username = if fields[2].is_empty() {
        "root".to_string()
    } else {
        fields[2].to_string()
    };

    Some(ImportedSession {
        name: name.to_string(),
        host,
        port,
        username,
        auth_type: "password".to_string(),
        group_path: group_path.clone(),
        description: None,
    })
}

// WindTerm (user.sessions)

fn parse_windterm(path: &str) -> AppResult<Vec<ImportedSession>> {
    let content = read_text_file_limited(path, "WindTerm session file")?;
    parse_windterm_content(&content)
}

fn parse_windterm_content(content: &str) -> AppResult<Vec<ImportedSession>> {
    let entries: Vec<serde_json::Value> = serde_json::from_str(content)
        .map_err(|e| AppError::Config(format!("Invalid WindTerm JSON: {e}")))?;

    let mut sessions = Vec::new();

    for entry in &entries {
        let protocol = entry
            .get("session.protocol")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !protocol.eq_ignore_ascii_case("SSH") {
            continue;
        }

        let target = entry
            .get("session.target")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let (host, username) = parse_windterm_target(target);
        if host.is_empty() {
            continue;
        }

        let name = entry
            .get("session.label")
            .and_then(|v| v.as_str())
            .unwrap_or(&host)
            .to_string();

        let port: u16 = entry
            .get("session.port")
            .and_then(|v| v.as_u64())
            .map_or(22, |p| p as u16);

        let group_path = entry
            .get("session.group")
            .and_then(|v| v.as_str())
            .and_then(|s| {
                let s = s.trim();
                if s.is_empty() {
                    None
                } else {
                    let segments: Vec<String> = s
                        .split('>')
                        .filter(|seg| !seg.is_empty())
                        .map(|seg| seg.trim().to_string())
                        .collect();
                    if segments.is_empty() {
                        None
                    } else {
                        Some(segments)
                    }
                }
            });

        sessions.push(ImportedSession {
            name,
            host,
            port,
            username,
            auth_type: "password".to_string(),
            group_path,
            description: None,
        });
    }

    Ok(sessions)
}

fn parse_windterm_target(target: &str) -> (String, String) {
    let target = target.trim();
    if let Some((username, host)) = target.rsplit_once('@')
        && !username.is_empty()
        && !host.is_empty()
    {
        return (host.to_string(), username.to_string());
    }
    (target.to_string(), "root".to_string())
}

// NyaTerm JSON (.json)

fn parse_nyaterm_json_content(content: &str) -> AppResult<PreparedSessionImport> {
    let file: NyatermJsonImportFile = serde_json::from_str(content)
        .map_err(|e| AppError::Config(format!("Invalid NyaTerm JSON: {e}")))?;
    prepare_nyaterm_json_import(file)
}

fn prepare_nyaterm_json_import(file: NyatermJsonImportFile) -> AppResult<PreparedSessionImport> {
    if file.version != 1 {
        return Err(AppError::Config(format!(
            "Unsupported NyaTerm JSON import version: {}",
            file.version
        )));
    }

    let mut password_ref_map: HashMap<String, String> = HashMap::new();
    let mut passwords = Vec::new();
    for entry in file.passwords {
        let ref_name = required_string(entry.ref_name, "password ref", "passwords")?;
        if password_ref_map.contains_key(&ref_name) {
            return Err(AppError::Config(format!(
                "Duplicate password ref in import file: {ref_name}"
            )));
        }
        if entry.password.is_empty() {
            return Err(AppError::Config(format!(
                "Password entry '{ref_name}' cannot have an empty password"
            )));
        }

        let id = uuid::Uuid::new_v4().to_string();
        password_ref_map.insert(ref_name, id.clone());
        passwords.push(SavedPassword {
            id,
            name: required_string(entry.name, "password name", "passwords")?,
            password: Some(entry.password),
            has_password: false,
        });
    }

    let mut key_ref_map: HashMap<String, String> = HashMap::new();
    let mut ssh_keys = Vec::new();
    for entry in file.ssh_keys {
        let ref_name = required_string(entry.ref_name, "ssh key ref", "ssh_keys")?;
        if key_ref_map.contains_key(&ref_name) {
            return Err(AppError::Config(format!(
                "Duplicate ssh key ref in import file: {ref_name}"
            )));
        }
        if entry.private_key.trim().is_empty() {
            return Err(AppError::Config(format!(
                "SSH key entry '{ref_name}' cannot have empty private_key"
            )));
        }

        let id = uuid::Uuid::new_v4().to_string();
        key_ref_map.insert(ref_name, id.clone());
        ssh_keys.push(SshKey {
            id,
            name: required_string(entry.name, "ssh key name", "ssh_keys")?,
            key: Some(entry.private_key),
            cert: normalize_optional_string(entry.certificate),
            passphrase: normalize_optional_string(entry.passphrase),
            key_file_path: None,
            cert_file_path: None,
            has_key_data: false,
            has_cert_data: false,
        });
    }

    let mut groups = Vec::new();
    for group in file.groups {
        let path = normalize_required_group_path(group.path, "groups.path")?;
        if !groups.contains(&path) {
            groups.push(path);
        }
    }

    let mut connections = Vec::new();
    for session in file.sessions {
        connections.push(prepare_nyaterm_json_session(
            session,
            &password_ref_map,
            &key_ref_map,
        )?);
    }

    Ok(PreparedSessionImport {
        groups,
        passwords,
        ssh_keys,
        connections,
    })
}

fn prepare_nyaterm_json_session(
    session: NyatermJsonSession,
    password_ref_map: &HashMap<String, String>,
    key_ref_map: &HashMap<String, String>,
) -> AppResult<PreparedSessionConnection> {
    match session {
        NyatermJsonSession::Ssh {
            name,
            group_path,
            host,
            port,
            username,
            auth,
            description,
            sort_order,
            icon,
        } => {
            validate_port(port, "ssh session")?;
            let context = format!("ssh session '{name}'");
            Ok(PreparedSessionConnection {
                name: required_string(name, "name", "ssh session")?,
                config: ConnectionType::Ssh {
                    host: required_string(host, "host", &context)?,
                    port,
                    username: required_string(username, "username", &context)?,
                    backspace_mode: "del".to_string(),
                    ai_execution_profile: AiExecutionProfile::Auto,
                    x11_forwarding: false,
                    agent_endpoint: Default::default(),
                    agent_forwarding: false,
                    encoding: String::new(),
                },
                group_path: normalize_optional_group_path(group_path, &context)?,
                description: normalize_optional_string(description),
                sort_order,
                icon: normalize_optional_string(icon),
                auth: Some(prepare_json_ssh_auth(
                    auth,
                    password_ref_map,
                    key_ref_map,
                    &context,
                )?),
            })
        }
        NyatermJsonSession::LocalTerminal {
            name,
            group_path,
            shell_path,
            shell_args,
            working_dir,
            description,
            sort_order,
            icon,
        } => {
            let context = format!("local_terminal session '{name}'");
            Ok(PreparedSessionConnection {
                name: required_string(name, "name", "local_terminal session")?,
                config: ConnectionType::LocalTerminal {
                    shell_path: required_string(shell_path, "shell_path", &context)?,
                    shell_args,
                    working_dir: normalize_optional_string(working_dir),
                    ai_execution_profile: AiExecutionProfile::Auto,
                    encoding: String::new(),
                },
                group_path: normalize_optional_group_path(group_path, &context)?,
                description: normalize_optional_string(description),
                sort_order,
                icon: normalize_optional_string(icon),
                auth: None,
            })
        }
        NyatermJsonSession::Telnet {
            name,
            group_path,
            host,
            port,
            backspace_mode,
            description,
            sort_order,
            icon,
        } => {
            validate_port(port, "telnet session")?;
            validate_backspace_mode(&backspace_mode, "telnet session")?;
            let context = format!("telnet session '{name}'");
            Ok(PreparedSessionConnection {
                name: required_string(name, "name", "telnet session")?,
                config: ConnectionType::Telnet {
                    host: required_string(host, "host", &context)?,
                    port,
                    username: String::new(),
                    ai_execution_profile: AiExecutionProfile::Auto,
                    backspace_mode,
                    raw_tcp_cli: false,
                    enter_mode: "cr".to_string(),
                    local_echo: false,
                    local_line_edit: false,
                    force_character_at_a_time: false,
                    send_naws: true,
                    send_sga: true,
                    auto_login: Default::default(),
                    encoding: String::new(),
                },
                group_path: normalize_optional_group_path(group_path, &context)?,
                description: normalize_optional_string(description),
                sort_order,
                icon: normalize_optional_string(icon),
                auth: None,
            })
        }
        NyatermJsonSession::Serial {
            name,
            group_path,
            port_name,
            baud_rate,
            data_bits,
            parity,
            stop_bits,
            backspace_mode,
            description,
            sort_order,
            icon,
        } => {
            validate_serial_config(baud_rate, data_bits, &parity, &stop_bits, &backspace_mode)?;
            let context = format!("serial session '{name}'");
            Ok(PreparedSessionConnection {
                name: required_string(name, "name", "serial session")?,
                config: ConnectionType::Serial {
                    port_name: required_string(port_name, "port_name", &context)?,
                    baud_rate,
                    data_bits,
                    parity,
                    stop_bits,
                    ai_execution_profile: AiExecutionProfile::Auto,
                    backspace_mode,
                    encoding: String::new(),
                },
                group_path: normalize_optional_group_path(group_path, &context)?,
                description: normalize_optional_string(description),
                sort_order,
                icon: normalize_optional_string(icon),
                auth: None,
            })
        }
    }
}

fn prepare_json_ssh_auth(
    auth: Option<NyatermJsonSshAuth>,
    password_ref_map: &HashMap<String, String>,
    key_ref_map: &HashMap<String, String>,
    context: &str,
) -> AppResult<ConnectionAuth> {
    let Some(auth) = auth else {
        return Ok(ConnectionAuth {
            mode: "none".to_string(),
            password_id: None,
            password: None,
            key_id: None,
            otp_id: None,
            auto_fill_otp: false,
            has_password: false,
        });
    };

    match auth.mode.trim() {
        "none" => {
            if auth.password.is_some() || auth.password_ref.is_some() || auth.key_ref.is_some() {
                return Err(AppError::Config(format!(
                    "{context}: auth.mode 'none' cannot include password, password_ref, or key_ref"
                )));
            }
            Ok(ConnectionAuth {
                mode: "none".to_string(),
                password_id: None,
                password: None,
                key_id: None,
                otp_id: None,
                auto_fill_otp: false,
                has_password: false,
            })
        }
        "password" => {
            let has_password = auth
                .password
                .as_ref()
                .is_some_and(|value| !value.is_empty());
            let password_ref = normalize_optional_string(auth.password_ref);
            if has_password == password_ref.is_some() {
                return Err(AppError::Config(format!(
                    "{context}: password auth must include exactly one of password or password_ref"
                )));
            }
            let password_id = if let Some(ref_name) = password_ref {
                Some(password_ref_map.get(&ref_name).cloned().ok_or_else(|| {
                    AppError::Config(format!(
                        "{context}: password_ref '{ref_name}' was not found"
                    ))
                })?)
            } else {
                None
            };
            let password = auth.password;

            Ok(ConnectionAuth {
                mode: "password".to_string(),
                password_id,
                password,
                key_id: None,
                otp_id: None,
                auto_fill_otp: false,
                has_password: false,
            })
        }
        "key" => {
            if auth.password.is_some() || auth.password_ref.is_some() {
                return Err(AppError::Config(format!(
                    "{context}: key auth cannot include password or password_ref"
                )));
            }
            let key_ref = normalize_optional_string(auth.key_ref)
                .ok_or_else(|| AppError::Config(format!("{context}: key auth requires key_ref")))?;
            let key_id = key_ref_map.get(&key_ref).cloned().ok_or_else(|| {
                AppError::Config(format!("{context}: key_ref '{key_ref}' was not found"))
            })?;

            Ok(ConnectionAuth {
                mode: "key".to_string(),
                password_id: None,
                password: None,
                key_id: Some(key_id),
                otp_id: None,
                auto_fill_otp: false,
                has_password: false,
            })
        }
        mode => Err(AppError::Config(format!(
            "{context}: unsupported SSH auth mode '{mode}'"
        ))),
    }
}

fn required_string(value: String, field: &str, context: &str) -> AppResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::Config(format!("{context}: {field} is required")));
    }
    Ok(trimmed.to_string())
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn normalize_required_group_path(path: Vec<String>, context: &str) -> AppResult<Vec<String>> {
    normalize_optional_group_path(path, context)?.ok_or_else(|| {
        AppError::Config(format!(
            "{context}: group path must contain at least one segment"
        ))
    })
}

fn normalize_optional_group_path(
    path: Vec<String>,
    context: &str,
) -> AppResult<Option<Vec<String>>> {
    if path.is_empty() {
        return Ok(None);
    }

    let mut segments = Vec::new();
    for segment in path {
        let trimmed = segment.trim();
        if trimmed.is_empty() {
            return Err(AppError::Config(format!(
                "{context}: group path segments cannot be empty"
            )));
        }
        segments.push(trimmed.to_string());
    }
    Ok(Some(segments))
}

fn validate_port(port: u16, context: &str) -> AppResult<()> {
    if port == 0 {
        return Err(AppError::Config(format!(
            "{context}: port must be between 1 and 65535"
        )));
    }
    Ok(())
}

fn validate_backspace_mode(value: &str, context: &str) -> AppResult<()> {
    match value {
        "ctrl_h" | "del" => Ok(()),
        _ => Err(AppError::Config(format!(
            "{context}: backspace_mode must be 'ctrl_h' or 'del'"
        ))),
    }
}

fn validate_serial_config(
    baud_rate: u32,
    data_bits: u8,
    parity: &str,
    stop_bits: &str,
    backspace_mode: &str,
) -> AppResult<()> {
    if baud_rate == 0 {
        return Err(AppError::Config(
            "serial session: baud_rate must be greater than 0".to_string(),
        ));
    }
    if !(5..=8).contains(&data_bits) {
        return Err(AppError::Config(
            "serial session: data_bits must be between 5 and 8".to_string(),
        ));
    }
    match parity {
        "none" | "even" | "odd" => {}
        _ => {
            return Err(AppError::Config(
                "serial session: parity must be 'none', 'even', or 'odd'".to_string(),
            ));
        }
    }
    match stop_bits {
        "1" | "1.5" | "2" => {}
        _ => {
            return Err(AppError::Config(
                "serial session: stop_bits must be '1', '1.5', or '2'".to_string(),
            ));
        }
    }
    validate_backspace_mode(backspace_mode, "serial session")
}

// Shared import persistence helpers

fn prepare_legacy_sessions(imported: Vec<ImportedSession>) -> PreparedSessionImport {
    let connections = imported
        .into_iter()
        .map(|session| PreparedSessionConnection {
            name: session.name,
            config: ConnectionType::Ssh {
                host: session.host,
                port: session.port,
                username: session.username,
                backspace_mode: "del".to_string(),
                ai_execution_profile: AiExecutionProfile::Auto,
                x11_forwarding: false,
                agent_endpoint: Default::default(),
                agent_forwarding: false,
                encoding: String::new(),
            },
            group_path: session.group_path,
            description: session.description,
            sort_order: 0,
            icon: None,
            auth: Some(ConnectionAuth {
                mode: session.auth_type,
                password_id: None,
                password: None,
                key_id: None,
                otp_id: None,
                auto_fill_otp: false,
                has_password: false,
            }),
        })
        .collect();

    PreparedSessionImport {
        groups: Vec::new(),
        passwords: Vec::new(),
        ssh_keys: Vec::new(),
        connections,
    }
}

pub fn prepare_session_import(
    file_path: &Path,
) -> Result<PreparedSessionImport, SessionImportError> {
    if file_path.is_dir() {
        return Ok(prepare_legacy_sessions(finalshell::parse_finalshell(
            file_path,
        )?));
    }

    let path = file_path.to_string_lossy();
    let lower = path.to_ascii_lowercase();
    let prepared = if lower.ends_with(".xts") {
        prepare_legacy_sessions(parse_xshell(&path)?)
    } else if lower.ends_with(".mxtsessions") {
        prepare_legacy_sessions(parse_mobaxterm(&path)?)
    } else if lower.ends_with(".sessions") {
        prepare_legacy_sessions(parse_windterm(&path)?)
    } else if lower.ends_with(".xml") {
        prepare_legacy_sessions(securecrt::parse_securecrt(file_path)?)
    } else if lower.ends_with(".json") {
        electerm::parse_json_import(file_path)?
    } else {
        return Err(AppError::Config(
            "Unsupported file format. Please use .xts (Xshell), .mxtsessions (MobaXterm), .sessions (WindTerm), .xml (SecureCRT), .json (NyaTerm JSON or Electerm bookmarks), or a FinalShell conn directory."
                .to_string(),
        ));
    };

    Ok(prepared)
}

pub fn prepare_termius_session_import(
    indexed_db_path: Option<&Path>,
    local_key: &[u8],
) -> Result<PreparedSessionImport, SessionImportError> {
    termius::parse_termius_indexed_db(indexed_db_path, local_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_JSON: &str = r#"
{
  "version": 1,
  "passwords": [
    { "ref": "prod-root-password", "name": "Prod root password", "password": "replace-me" }
  ],
  "ssh_keys": [
    {
      "ref": "ops-ed25519",
      "name": "Ops ED25519",
      "private_key": "-----BEGIN OPENSSH PRIVATE KEY-----\n...\n-----END OPENSSH PRIVATE KEY-----",
      "passphrase": "optional-passphrase"
    }
  ],
  "groups": [
    { "path": ["Production"] },
    { "path": ["Production", "Web"] },
    { "path": ["Lab"] }
  ],
  "sessions": [
    {
      "name": "Prod web direct password",
      "type": "ssh",
      "group_path": ["Production", "Web"],
      "host": "web-01.example.com",
      "port": 22,
      "username": "deploy",
      "auth": { "mode": "password", "password": "replace-me" }
    },
    {
      "name": "Prod db saved password",
      "type": "ssh",
      "group_path": ["Production", "Database"],
      "host": "db-01.example.com",
      "username": "root",
      "auth": { "mode": "password", "password_ref": "prod-root-password" }
    },
    {
      "name": "Bastion saved key",
      "type": "ssh",
      "group_path": ["Production"],
      "host": "bastion.example.com",
      "username": "ops",
      "auth": { "mode": "key", "key_ref": "ops-ed25519" }
    },
    {
      "name": "Lab router",
      "type": "telnet",
      "group_path": ["Lab"],
      "host": "192.168.10.1",
      "port": 23,
      "backspace_mode": "del"
    },
    {
      "name": "USB console",
      "type": "serial",
      "group_path": ["Lab"],
      "port_name": "COM3",
      "baud_rate": 115200,
      "data_bits": 8,
      "parity": "none",
      "stop_bits": "1",
      "backspace_mode": "ctrl_h"
    },
    {
      "name": "Local PowerShell",
      "type": "local_terminal",
      "shell_path": "pwsh.exe",
      "shell_args": "-NoLogo",
      "working_dir": "C:\\Users\\me"
    }
  ]
}
"#;

    #[test]
    fn xshell_session_parser_preserves_group_and_key_auth() {
        let session = parse_xsh_content(
            r#"
[CONNECTION]
Protocol=SSH
Host=web.example.com
Port=2222

[CONNECTION:AUTHENTICATION]
UserName=deploy
UserKey=C:\keys\deploy.key
"#,
            "Xshell/Sessions/Production/Web/prod.xsh",
        )
        .expect("parse Xshell session");

        assert_eq!(session.name, "prod");
        assert_eq!(session.host, "web.example.com");
        assert_eq!(session.port, 2222);
        assert_eq!(session.username, "deploy");
        assert_eq!(session.auth_type, "key");
        assert_eq!(
            session.group_path,
            Some(vec!["Production".to_string(), "Web".to_string()])
        );
    }

    #[test]
    fn mobaxterm_session_parser_reads_ssh_bookmark_fields() {
        let group = Some(vec!["Production".to_string()]);
        let session = parse_moba_entry("Prod web", "#109#0%web.example.com%2200%deploy%", &group)
            .expect("parse MobaXterm session");

        assert_eq!(session.name, "Prod web");
        assert_eq!(session.host, "web.example.com");
        assert_eq!(session.port, 2200);
        assert_eq!(session.username, "deploy");
        assert_eq!(session.auth_type, "password");
        assert_eq!(session.group_path, group);
    }

    #[test]
    fn windterm_import_splits_user_at_host_targets() {
        let sessions = parse_windterm_content(
            r#"
[
  {
    "session.protocol": "SSH",
    "session.target": "deploy@192.168.1.10",
    "session.label": "Prod web",
    "session.port": 2222
  }
]
"#,
        )
        .expect("parse windterm sessions");

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].name, "Prod web");
        assert_eq!(sessions[0].host, "192.168.1.10");
        assert_eq!(sessions[0].username, "deploy");
        assert_eq!(sessions[0].port, 2222);
    }

    #[test]
    fn windterm_import_defaults_username_when_target_has_no_user() {
        let sessions = parse_windterm_content(
            r#"
[
  {
    "session.protocol": "SSH",
    "session.target": "192.168.1.10"
  }
]
"#,
        )
        .expect("parse windterm sessions");

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].host, "192.168.1.10");
        assert_eq!(sessions[0].username, "root");
    }

    #[test]
    fn windterm_target_rejects_empty_user_or_host_splits() {
        assert_eq!(
            parse_windterm_target("@192.168.1.10"),
            ("@192.168.1.10".to_string(), "root".to_string())
        );
        assert_eq!(
            parse_windterm_target("deploy@"),
            ("deploy@".to_string(), "root".to_string())
        );
    }

    #[test]
    fn windterm_target_splits_on_last_at_symbol() {
        assert_eq!(
            parse_windterm_target("ops@team@example.com"),
            ("example.com".to_string(), "ops@team".to_string())
        );
    }

    #[test]
    fn limited_reader_rejects_oversized_files() {
        let dir = std::env::temp_dir().join(format!(
            "nyaterm-session-import-limit-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("create limit directory");
        let path = dir.join("large.json");
        std::fs::write(&path, b"12345").expect("write oversized file");

        let error = read_file_limited(path.to_str().expect("utf8 path"), "test import file", 4)
            .expect_err("oversized file should fail");

        assert!(error.to_string().contains("exceeds"));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn nyaterm_json_sample_import_prepares_supported_shapes() {
        let prepared = parse_nyaterm_json_content(SAMPLE_JSON).expect("parse sample");

        assert_eq!(prepared.groups.len(), 3);
        assert_eq!(prepared.passwords.len(), 1);
        assert_eq!(prepared.ssh_keys.len(), 1);
        assert_eq!(prepared.connections.len(), 6);
        assert_eq!(
            prepared.passwords[0].password.as_deref(),
            Some("replace-me")
        );
        assert_eq!(
            prepared.ssh_keys[0].key.as_deref(),
            Some("-----BEGIN OPENSSH PRIVATE KEY-----\n...\n-----END OPENSSH PRIVATE KEY-----")
        );

        let direct_auth = prepared.connections[0].auth.as_ref().expect("direct auth");
        assert_eq!(direct_auth.mode, "password");
        assert!(direct_auth.password_id.is_none());
        assert_eq!(direct_auth.password.as_deref(), Some("replace-me"));

        let saved_password_auth = prepared.connections[1]
            .auth
            .as_ref()
            .expect("saved password auth");
        assert_eq!(saved_password_auth.mode, "password");
        assert!(saved_password_auth.password_id.is_some());
        assert!(saved_password_auth.password.is_none());

        let key_auth = prepared.connections[2].auth.as_ref().expect("key auth");
        assert_eq!(key_auth.mode, "key");
        assert!(key_auth.key_id.is_some());

        let local_config = &prepared.connections[5].config;
        assert!(matches!(
            local_config,
            ConnectionType::LocalTerminal {
                shell_path,
                shell_args,
                ..
            } if shell_path == "pwsh.exe" && shell_args == "-NoLogo"
        ));
    }

    #[test]
    fn nyaterm_json_rejects_duplicate_password_refs() {
        let json = r#"
{
  "version": 1,
  "passwords": [
    { "ref": "dup", "name": "One", "password": "a" },
    { "ref": "dup", "name": "Two", "password": "b" }
  ],
  "sessions": []
}
"#;

        let error = parse_nyaterm_json_content(json).unwrap_err();
        assert!(error.to_string().contains("Duplicate password ref"));
    }

    #[test]
    fn nyaterm_json_rejects_missing_password_refs() {
        let json = r#"
{
  "version": 1,
  "sessions": [
    {
      "name": "Missing password",
      "type": "ssh",
      "host": "example.com",
      "username": "root",
      "auth": { "mode": "password", "password_ref": "missing" }
    }
  ]
}
"#;

        let error = parse_nyaterm_json_content(json).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("password_ref 'missing' was not found")
        );
    }

    #[test]
    fn nyaterm_json_rejects_invalid_ports() {
        let json = r#"
{
  "version": 1,
  "sessions": [
    {
      "name": "Bad port",
      "type": "ssh",
      "host": "example.com",
      "port": 0,
      "username": "root",
      "auth": { "mode": "none" }
    }
  ]
}
"#;

        let error = parse_nyaterm_json_content(json).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("port must be between 1 and 65535")
        );
    }

    #[test]
    fn securecrt_import_prepares_xml_sessions_with_groups() {
        let dir = temp_import_dir("securecrt");
        let import_path = dir.join("sessions.xml");
        std::fs::write(
            &import_path,
            r#"
<?xml version="1.0" encoding="UTF-8"?>
<key name="Sessions">
  <key name="Production">
    <key name="Web">
      <key name="Prod web">
        <string name="Hostname">web.example.com</string>
        <dword name="Port">2200</dword>
        <string name="Username">deploy</string>
        <string name="Protocol Name">SSH2</string>
      </key>
    </key>
  </key>
</key>
"#,
        )
        .expect("write SecureCRT XML");
        let prepared = prepare_session_import(&import_path).expect("prepare SecureCRT");

        assert_eq!(prepared.connections.len(), 1);
        let connection = prepared
            .connections
            .iter()
            .find(|connection| connection.name == "Prod web")
            .expect("SecureCRT connection");
        assert!(matches!(
            &connection.config,
            ConnectionType::Ssh { host, port, username, .. }
                if host == "web.example.com" && *port == 2200 && username == "deploy"
        ));
        assert_eq!(
            connection.group_path.as_deref(),
            Some(["Production".to_string(), "Web".to_string()].as_slice())
        );
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn finalshell_import_accepts_conn_directory() {
        let dir = temp_import_dir("finalshell");
        let conn_dir = dir.join("conn");
        let nested = conn_dir.join("prod");
        std::fs::create_dir_all(&nested).expect("create FinalShell conn dir");
        std::fs::write(
            nested.join("folder.json"),
            r#"{"id":"folder-prod","name":"Production","parent_id":"root","delete_time":0}"#,
        )
        .expect("write FinalShell folder");
        std::fs::write(
            nested.join("prod_connect_config.json"),
            r#"{"name":"Prod shell","host":"prod.example.com","port":2222,"user_name":"ops","parent_id":"folder-prod","conection_type":100,"description":"primary","delete_time":0}"#,
        )
        .expect("write FinalShell connection");
        let prepared = prepare_session_import(&conn_dir).expect("prepare FinalShell");

        assert_eq!(prepared.connections.len(), 1);
        let connection = prepared
            .connections
            .iter()
            .find(|connection| connection.name == "Prod shell")
            .expect("FinalShell connection");
        assert_eq!(connection.description.as_deref(), Some("primary"));
        assert_eq!(
            connection.group_path.as_deref(),
            Some(["Production".to_string()].as_slice())
        );
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn electerm_json_imports_bookmarks_with_groups() {
        let dir = temp_import_dir("electerm");
        let import_path = dir.join("bookmarks.json");
        std::fs::write(
            &import_path,
            r#"
{
  "bookmarkGroups": [
    { "id": "root", "title": "Production", "bookmarkIds": ["web"], "bookmarkGroupIds": [] }
  ],
  "bookmarks": [
    {
      "id": "web",
      "title": "Web",
      "host": "web.example.com",
      "username": "deploy",
      "authType": "password",
      "port": 2200,
      "type": "ssh"
    }
  ]
}
"#,
        )
        .expect("write Electerm bookmarks");
        let prepared = prepare_session_import(&import_path).expect("prepare Electerm");

        assert_eq!(prepared.connections.len(), 1);
        let connection = prepared
            .connections
            .iter()
            .find(|connection| connection.name == "Web")
            .expect("Electerm connection");
        assert!(matches!(
            &connection.config,
            ConnectionType::Ssh { host, port, username, .. }
                if host == "web.example.com" && *port == 2200 && username == "deploy"
        ));
        assert_eq!(
            connection.auth.as_ref().map(|auth| auth.mode.as_str()),
            Some("password")
        );
        assert_eq!(
            connection.group_path.as_deref(),
            Some(["Production".to_string()].as_slice())
        );
        std::fs::remove_dir_all(dir).ok();
    }

    fn temp_import_dir(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nyaterm-session-import-{label}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).expect("create import directory");
        dir
    }
}

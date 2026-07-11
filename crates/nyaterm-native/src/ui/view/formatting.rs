use gpui::rgb;
use nyaterm_domain::{
    AppSettingsSummary, CloudSyncError, CloudSyncHistoryEntry, CloudSyncSettings, RiskLevel,
    TunnelConfig,
};
use nyaterm_session::{
    SessionKind, SshSessionConfig, SshTunnelMode, TelnetEnterMode, safe_recording_name,
};

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::AiAgentStepStatus;

pub(in crate::ui::view) fn ai_agent_step_status_style(
    status: AiAgentStepStatus,
) -> (&'static str, u32, u32) {
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

pub(in crate::ui::view) fn format_rate(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1024. * 1024. {
        format!("{:.1} MiB/s", bytes_per_sec / 1024. / 1024.)
    } else if bytes_per_sec >= 1024. {
        format!("{:.1} KiB/s", bytes_per_sec / 1024.)
    } else {
        format!("{bytes_per_sec:.0} B/s")
    }
}

pub(in crate::ui::view) fn format_uptime(seconds: u64) -> String {
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

pub(in crate::ui::view) fn risk_label(risk: Option<&RiskLevel>) -> &'static str {
    match risk {
        Some(RiskLevel::Low) => "Low",
        Some(RiskLevel::Medium) => "Medium",
        Some(RiskLevel::High) => "High",
        Some(RiskLevel::Critical) => "Critical",
        None => "Unrated",
    }
}

pub(in crate::ui::view) fn command_source_label(source: &str) -> &'static str {
    match source {
        "quickCommand" => "quick",
        "history" => "history",
        _ => "command",
    }
}

pub(in crate::ui::view) fn recording_file_path(
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

pub(in crate::ui::view) fn docker_state_rank(state: &str) -> u8 {
    match state.trim().to_ascii_lowercase().as_str() {
        "running" => 0,
        "restarting" | "paused" => 1,
        "created" => 2,
        "exited" | "dead" => 3,
        _ => 4,
    }
}

pub(in crate::ui::view) fn docker_state_label(state: &str) -> &'static str {
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

pub(in crate::ui::view) fn docker_state_color(state: &str) -> gpui::Hsla {
    match state.trim().to_ascii_lowercase().as_str() {
        "running" => rgb(0x6ee7b7).into(),
        "restarting" | "paused" => rgb(0xfbbf24).into(),
        "created" => rgb(0x93c5fd).into(),
        "exited" | "dead" => rgb(0xfca5a5).into(),
        _ => rgb(0x98a3b8).into(),
    }
}

pub(in crate::ui::view) fn docker_compose_project_key(
    project_name: &str,
    config_files: Option<&str>,
) -> String {
    format!(
        "{}\n{}",
        project_name.trim(),
        config_files.unwrap_or_default().trim()
    )
}

pub(in crate::ui::view) fn session_kind_label(kind: SessionKind) -> &'static str {
    match kind {
        SessionKind::LocalPty => "local",
        SessionKind::Ssh => "ssh",
        SessionKind::Telnet => "telnet",
        SessionKind::RawTcp => "raw tcp",
        SessionKind::Serial => "serial",
    }
}

pub(in crate::ui::view) fn cloud_sync_history_status(error: &CloudSyncError) -> &'static str {
    match error {
        CloudSyncError::Conflict(_) => "conflict",
        _ => "failed",
    }
}

pub(in crate::ui::view) fn configured_cloud_sync_provider(settings: &CloudSyncSettings) -> String {
    let provider = settings.provider.trim();
    if provider.is_empty() {
        "local_directory".to_string()
    } else {
        provider.to_string()
    }
}

pub(in crate::ui::view) fn none_if_blank(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

pub(in crate::ui::view) fn recent_terminal_output(output: &str, max_lines: usize) -> String {
    let lines = output.lines().collect::<Vec<_>>();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}

pub(in crate::ui::view) fn cloud_secret_display(draft: &str, current: &Option<String>) -> String {
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

pub(in crate::ui::view) fn compact_id(value: &str) -> String {
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


pub(in crate::ui::view) fn format_cloud_provider(provider: &str) -> String {
    match provider.trim() {
        "" => "Unknown".to_string(),
        "local_directory" => "Local directory".to_string(),
        "webdav" => "WebDAV".to_string(),
        "s3" => "S3".to_string(),
        "gitee_snippet" => "Gitee snippet".to_string(),
        "github_gist" => "GitHub gist".to_string(),
        "aliyun_drive" => "Aliyun Drive".to_string(),
        "google_drive" => "Google Drive".to_string(),
        "onedrive" => "OneDrive".to_string(),
        other => other.to_string(),
    }
}

pub(in crate::ui::view) fn format_history_timestamp_ms(timestamp_ms: u64) -> String {
    if timestamp_ms == 0 {
        return "never".to_string();
    }
    let secs = (timestamp_ms / 1000) as i64;
    let hours = ((secs % 86_400) / 3_600).rem_euclid(24);
    let minutes = ((secs % 3_600) / 60).rem_euclid(60);
    let seconds = (secs % 60).rem_euclid(60);
    // Compact wall-clock style without pulling chrono; good enough for panel density.
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

pub(in crate::ui::view) fn format_duration_ms(duration_ms: Option<u64>) -> Option<String> {
    let value = duration_ms?;
    if value < 1000 {
        Some(format!("{value} ms"))
    } else if value < 60_000 {
        Some(format!("{:.1} s", value as f64 / 1000.0))
    } else {
        let minutes = value / 60_000;
        let seconds = (value % 60_000) as f64 / 1000.0;
        Some(format!("{minutes}m {seconds:.0}s"))
    }
}

pub(in crate::ui::view) fn cloud_sync_status_dot_color(status: &str) -> gpui::Rgba {
    match status {
        "running" => rgb(0x3b82f6),
        "success" => rgb(0x22c55e),
        "failed" => rgb(0xef4444),
        "conflict" => rgb(0xf59e0b),
        "disabled" => rgb(0x6e7681),
        _ => rgb(0x8b949e),
    }
}

pub(in crate::ui::view) fn cloud_sync_status_text_color(status: &str) -> gpui::Rgba {
    match status {
        "running" => rgb(0x58a6ff),
        "success" => rgb(0x3fb950),
        "failed" => rgb(0xff7b72),
        "conflict" => rgb(0xd29922),
        "disabled" => rgb(0x6e7681),
        _ => rgb(0x8b949e),
    }
}

pub(in crate::ui::view) fn cloud_sync_kind_text_color(kind: &str) -> gpui::Rgba {
    match kind {
        "sync" => rgb(0x58a6ff),
        "backup" => rgb(0xa371f7),
        _ => rgb(0x8b949e),
    }
}

pub(in crate::ui::view) fn cloud_sync_history_summary(entry: &CloudSyncHistoryEntry) -> String {
    let normalized = entry.message.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return format!("{} · {}", entry.kind, entry.status);
    }
    if !normalized.contains('\n') && normalized.chars().count() <= 110 {
        return normalized;
    }
    // Prefer first sentence when short enough.
    let first = normalized
        .split(|ch| ch == '.' || ch == '!' || ch == '?')
        .next()
        .unwrap_or("")
        .trim();
    if !first.is_empty() && first.chars().count() <= 110 {
        let end = first.chars().count();
        let punct = normalized.chars().nth(end).unwrap_or('.');
        if matches!(punct, '.' | '!' | '?') {
            return format!("{first}{punct}");
        }
        return first.to_string();
    }
    format!("{} · {}", entry.kind, entry.status)
}

pub(in crate::ui::view) fn normalize_startup_command(value: &str) -> String {
    let mut command = value.trim().replace("\r\n", "\n").replace('\r', "\n");
    if !command.ends_with('\n') {
        command.push('\n');
    }
    command
}

pub(in crate::ui::view) fn short_id(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

pub(in crate::ui::view) fn status_label(status: &str) -> &'static str {
    if status.starts_with("running") {
        "session running"
    } else if status.contains("failed") || status.contains("error") {
        "session attention"
    } else {
        "session ready"
    }
}

pub(in crate::ui::view) fn non_empty_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub(in crate::ui::view) fn split_shell_args(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(in crate::ui::view) fn parse_telnet_enter_mode(value: &str) -> TelnetEnterMode {
    match value {
        "crlf" => TelnetEnterMode::Crlf,
        "lf" => TelnetEnterMode::Lf,
        _ => TelnetEnterMode::Cr,
    }
}

pub(in crate::ui::view) fn download_file_name_from_remote_path(remote_path: &str) -> String {
    remote_path
        .trim()
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty() && *name != ".")
        .unwrap_or("nyaterm-download.bin")
        .to_string()
}

pub(in crate::ui::view) fn tunnel_mode(tunnel: &TunnelConfig) -> Option<SshTunnelMode> {
    match tunnel.tunnel_type.as_str() {
        "local" => Some(SshTunnelMode::Local),
        "remote" => Some(SshTunnelMode::Remote),
        "dynamic" => Some(SshTunnelMode::Dynamic),
        _ => None,
    }
}

pub(in crate::ui::view) fn tunnel_mode_label(tunnel: &TunnelConfig) -> &'static str {
    match tunnel.tunnel_type.as_str() {
        "local" => "Local",
        "remote" => "Remote",
        "dynamic" => "SOCKS5",
        _ => "Tunnel",
    }
}

pub(in crate::ui::view) fn tunnel_name(tunnel: &TunnelConfig) -> String {
    if tunnel.name.trim().is_empty() {
        tunnel.id.clone()
    } else {
        tunnel.name.clone()
    }
}

pub(in crate::ui::view) fn tunnel_endpoint(tunnel: &TunnelConfig, listen: &str) -> String {
    match tunnel.tunnel_type.as_str() {
        "dynamic" => format!("{listen} SOCKS5"),
        "remote" => format!(
            "remote {} -> {}:{}",
            tunnel.listen_port, tunnel.target_host, tunnel.target_port
        ),
        _ => format!("{listen} -> {}:{}", tunnel.target_host, tunnel.target_port),
    }
}

pub(in crate::ui::view) fn format_permissions_octal(mode: u32) -> String {
    format!("{:04o}", mode & 0o7777)
}

pub(in crate::ui::view) fn configured_status(secret: &str) -> String {
    if secret.trim().is_empty() {
        "missing".to_string()
    } else {
        "configured".to_string()
    }
}

pub(in crate::ui::view) fn configured_pair_status(id: &str, secret: &str) -> String {
    if id.trim().is_empty() || secret.trim().is_empty() {
        "missing".to_string()
    } else {
        "configured".to_string()
    }
}

pub(in crate::ui::view) fn trim_terminal_output_to(output: &mut String, max_bytes: usize) {
    if max_bytes == 0 || output.len() <= max_bytes {
        return;
    }
    let drain_to = output
        .char_indices()
        .find_map(|(index, _)| (index >= output.len() - max_bytes).then_some(index))
        .unwrap_or(0);
    output.drain(..drain_to);
}

pub(in crate::ui::view) fn ssh_multiplex_key(config: &SshSessionConfig) -> String {
    format!(
        "{}@{}:{}",
        config.username.trim(),
        config.host.trim().to_ascii_lowercase(),
        config.port
    )
}


pub(in crate::ui::view) fn format_last_used_ms(last_used_at_ms: Option<u64>) -> String {
    let Some(ms) = last_used_at_ms.filter(|value| *value > 0) else {
        return "never".to_string();
    };
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(ms);
    if now_ms < ms {
        return "just now".to_string();
    }
    let secs = (now_ms - ms) / 1000;
    if secs < 60 {
        "just now".to_string()
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86_400 {
        format!("{}h ago", secs / 3600)
    } else if secs < 86_400 * 30 {
        format!("{}d ago", secs / 86_400)
    } else {
        format!("{}mo ago", secs / (86_400 * 30))
    }
}



/// Tauri AI history date buckets (`groupSessionsByDate`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ui::view) enum AiHistoryDateGroup {
    Today,
    Yesterday,
    Last7Days,
    Earlier,
}

impl AiHistoryDateGroup {
    pub(in crate::ui::view) fn label(self) -> &'static str {
        match self {
            Self::Today => "Today",
            Self::Yesterday => "Yesterday",
            Self::Last7Days => "Last 7 Days",
            Self::Earlier => "Earlier",
        }
    }
}

fn civil_day_number(year: i32, month: u32, day: u32) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = (y - era * 400) as i64;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = ((153 * mp + 2) / 5 + day - 1) as i64;
    era as i64 * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy
}

fn utc_today_day_number() -> i64 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    // Unix epoch day 0 is 1970-01-01.
    719_468 + secs.div_euclid(86_400)
}

fn parse_rfc3339_day_number(value: &str) -> Option<i64> {
    let date = value.trim().get(..10)?;
    let mut parts = date.split('-');
    let year: i32 = parts.next()?.parse().ok()?;
    let month: u32 = parts.next()?.parse().ok()?;
    let day: u32 = parts.next()?.parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(civil_day_number(year, month, day))
}

pub(in crate::ui::view) fn ai_history_date_group(updated_at: &str) -> AiHistoryDateGroup {
    let today = utc_today_day_number();
    let Some(day) = parse_rfc3339_day_number(updated_at) else {
        return AiHistoryDateGroup::Earlier;
    };
    if day >= today {
        AiHistoryDateGroup::Today
    } else if day == today - 1 {
        AiHistoryDateGroup::Yesterday
    } else if day >= today - 6 {
        AiHistoryDateGroup::Last7Days
    } else {
        AiHistoryDateGroup::Earlier
    }
}

pub(in crate::ui::view) fn group_ai_sessions_by_date(
    sessions: &[nyaterm_domain::AiSession],
) -> [(AiHistoryDateGroup, Vec<nyaterm_domain::AiSession>); 4] {
    let mut groups: [(AiHistoryDateGroup, Vec<nyaterm_domain::AiSession>); 4] = [
        (AiHistoryDateGroup::Today, Vec::new()),
        (AiHistoryDateGroup::Yesterday, Vec::new()),
        (AiHistoryDateGroup::Last7Days, Vec::new()),
        (AiHistoryDateGroup::Earlier, Vec::new()),
    ];
    for session in sessions {
        let group = ai_history_date_group(&session.updated_at);
        let index = match group {
            AiHistoryDateGroup::Today => 0,
            AiHistoryDateGroup::Yesterday => 1,
            AiHistoryDateGroup::Last7Days => 2,
            AiHistoryDateGroup::Earlier => 3,
        };
        groups[index].1.push(session.clone());
    }
    groups
}


/// Tauri `resolveConnectionIcon`: map stored icon key / connection kind to SVG path + color.
#[derive(Debug, Clone, Copy)]
pub(in crate::ui::view) struct ConnectionIconDef {
    pub path: &'static str,
    pub color: u32,
    pub glyph: &'static str,
}

pub(in crate::ui::view) fn resolve_connection_icon(
    icon_key: Option<&str>,
    kind: &str,
) -> ConnectionIconDef {
    if let Some(key) = icon_key.map(str::trim).filter(|value| !value.is_empty()) {
        if let Some(def) = connection_icon_by_key(key) {
            return def;
        }
    }
    default_connection_icon_for_kind(kind)
}

fn connection_icon_by_key(key: &str) -> Option<ConnectionIconDef> {
    let key = key.to_ascii_lowercase();
    // Server palette (Tauri SERVER_ICONS colors).
    let server = match key.as_str() {
        "server" => Some(0x60a5fa),
        "server-emerald" => Some(0x34d399),
        "server-amber" => Some(0xfbbf24),
        "server-rose" => Some(0xfb7185),
        "server-violet" => Some(0xa78bfa),
        "server-cyan" => Some(0x22d3ee),
        "server-slate" => Some(0x94a3b8),
        _ => None,
    };
    if let Some(color) = server {
        return Some(ConnectionIconDef {
            path: "icons/conn/server.svg",
            color,
            glyph: "☰",
        });
    }
    Some(match key.as_str() {
        "linux" => ConnectionIconDef {
            path: "icons/conn/linux.svg",
            color: 0xfcc624,
            glyph: "🐧",
        },
        "ubuntu" => ConnectionIconDef {
            path: "icons/conn/ubuntu.svg",
            color: 0xe95420,
            glyph: "U",
        },
        "debian" => ConnectionIconDef {
            path: "icons/conn/debian.svg",
            color: 0xa81d33,
            glyph: "D",
        },
        "centos" | "fedora" | "arch" | "manjaro" | "opensuse" | "rocky" | "alma" | "alpine"
        | "kali" | "mint" | "nixos" | "gentoo" | "freebsd" | "raspberrypi" => ConnectionIconDef {
            path: "icons/conn/linux.svg",
            color: match key.as_str() {
                "centos" => 0xa14f8c,
                "fedora" => 0x3c4fb1,
                "arch" => 0x1793d1,
                "manjaro" => 0x35bf5c,
                "opensuse" => 0x73ba25,
                "rocky" => 0x10b981,
                "alma" => 0xff4649,
                "alpine" => 0x0d597f,
                "kali" => 0x268bee,
                "mint" => 0x87cf3e,
                "nixos" => 0x5277c3,
                "gentoo" => 0x54487a,
                "freebsd" => 0xab2b28,
                "raspberrypi" => 0xa22846,
                _ => 0xfcc624,
            },
            glyph: "🐧",
        },
        "apple" => ConnectionIconDef {
            path: "icons/conn/apple.svg",
            color: 0xa2aaad,
            glyph: "",
        },
        "windows" => ConnectionIconDef {
            path: "icons/conn/windows.svg",
            color: 0x0078d4,
            glyph: "▣",
        },
        "android" => ConnectionIconDef {
            path: "icons/conn/linux.svg",
            color: 0x3ddc84,
            glyph: "A",
        },
        "docker" => ConnectionIconDef {
            path: "icons/conn/docker.svg",
            color: 0x2496ed,
            glyph: "🐋",
        },
        "python" => ConnectionIconDef {
            path: "icons/conn/python.svg",
            color: 0x3776ab,
            glyph: "Py",
        },
        "github" => ConnectionIconDef {
            path: "icons/conn/github.svg",
            color: 0xc9d1d9,
            glyph: "GH",
        },
        "k8s" | "kubernetes" => ConnectionIconDef {
            path: "icons/conn/docker.svg",
            color: 0x326ce5,
            glyph: "K",
        },
        "local" | "terminal" => ConnectionIconDef {
            path: "icons/conn/terminal.svg",
            color: 0x4ade80,
            glyph: ">_",
        },
        "telnet" => ConnectionIconDef {
            path: "icons/conn/telnet.svg",
            color: 0xd29922,
            glyph: "⇄",
        },
        "serial" => ConnectionIconDef {
            path: "icons/conn/serial.svg",
            color: 0xbc8cff,
            glyph: "⌁",
        },
        "folder" | "group" => ConnectionIconDef {
            path: "icons/conn/folder.svg",
            color: 0xfbbf24,
            glyph: "📁",
        },
        // Other QUICK_ICONS keys fall back to colored server glyph.
        "nginx" | "redis" | "postgres" | "mysql" | "mongodb" | "js" | "ts" | "rust" | "go"
        | "node" | "php" | "aws" | "gcp" | "gitlab" => ConnectionIconDef {
            path: "icons/conn/server.svg",
            color: match key.as_str() {
                "nginx" => 0x009639,
                "redis" => 0xdc382d,
                "postgres" => 0x4169e1,
                "mysql" => 0x4479a1,
                "mongodb" => 0x47a248,
                "js" => 0xf7df1e,
                "ts" => 0x3178c6,
                "rust" => 0xdea584,
                "go" => 0x00add8,
                "node" => 0x339933,
                "php" => 0x777bb4,
                "aws" => 0xff9900,
                "gcp" => 0x4285f4,
                "gitlab" => 0xfc6d26,
                _ => 0x60a5fa,
            },
            glyph: "☰",
        },
        _ => return None,
    })
}

fn default_connection_icon_for_kind(kind: &str) -> ConnectionIconDef {
    match kind {
        "Local" => ConnectionIconDef {
            path: "icons/conn/terminal.svg",
            color: 0x4ade80,
            glyph: ">_",
        },
        "Telnet" => ConnectionIconDef {
            path: "icons/conn/telnet.svg",
            color: 0xd29922,
            glyph: "⇄",
        },
        "Serial" => ConnectionIconDef {
            path: "icons/conn/serial.svg",
            color: 0xbc8cff,
            glyph: "⌁",
        },
        _ => ConnectionIconDef {
            path: "icons/conn/server.svg",
            color: 0x60a5fa,
            glyph: "☰",
        },
    }
}


/// Lightweight GFM-ish blocks for AI transcript (closer to Tauri MarkdownContent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ui::view) enum MarkdownBlock {
    Paragraph(String),
    Bullet(String),
    Numbered { index: u32, text: String },
    Code { language: String, code: String },
    Quote(String),
    Heading { level: u8, text: String },
    Table {
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
    },
    ThematicBreak,
}

/// Inline style span after markdown markers are stripped (byte ranges into `text`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::ui::view) enum InlineMdStyle {
    Bold,
    Italic,
    BoldItalic,
    Code,
    Link,
    Strike,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::ui::view) struct InlineMarkdown {
    pub text: String,
    pub highlights: Vec<(std::ops::Range<usize>, InlineMdStyle)>,
}

/// Strip `<think>…</think>` segments (Tauri `extractThinkContent`).
pub(in crate::ui::view) fn extract_think_content(content: &str) -> (String, Option<String>) {
    let mut reasoning_parts: Vec<String> = Vec::new();
    let mut visible = String::new();
    let mut rest = content;
    while let Some(start) = rest.find("<think>") {
        visible.push_str(&rest[..start]);
        let after = &rest[start + 7..];
        if let Some(end) = after.find("</think>") {
            let part = after[..end].trim();
            if !part.is_empty() {
                reasoning_parts.push(part.to_string());
            }
            rest = &after[end + 8..];
        } else {
            let trailing = after.trim();
            if !trailing.is_empty() {
                reasoning_parts.push(trailing.to_string());
            }
            rest = "";
            break;
        }
    }
    visible.push_str(rest);
    // Drop incomplete trailing open-tag prefix fragments.
    if let Some(idx) = visible.rfind('<') {
        let tail = &visible[idx..];
        if "<think>".starts_with(tail) || tail == "<" || tail.starts_with("<t") {
            visible.truncate(idx);
        }
    }
    let visible = visible.trim().to_string();
    let reasoning = if reasoning_parts.is_empty() {
        None
    } else {
        Some(reasoning_parts.join("\n\n"))
    };
    (visible, reasoning)
}

fn is_table_separator_row(line: &str) -> bool {
    let cells = split_table_row(line);
    !cells.is_empty()
        && cells.iter().all(|cell| {
            let t = cell.trim();
            !t.is_empty()
                && t.chars().all(|ch| matches!(ch, '-' | ':' | ' '))
                && t.contains('-')
        })
}

fn split_table_row(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    if !trimmed.contains('|') {
        return Vec::new();
    }
    let body = trimmed
        .strip_prefix('|')
        .unwrap_or(trimmed)
        .strip_suffix('|')
        .unwrap_or(trimmed);
    body.split('|')
        .map(|cell| cell.trim().to_string())
        .collect()
}

fn looks_like_table_header(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.contains('|') && split_table_row(trimmed).len() >= 2
}

fn is_thematic_break(line: &str) -> bool {
    let t = line.trim();
    if t.len() < 3 {
        return false;
    }
    let chars: Vec<char> = t.chars().collect();
    let first = chars[0];
    if !matches!(first, '-' | '*' | '_') {
        return false;
    }
    chars.iter().all(|ch| *ch == first || ch.is_whitespace())
        && chars.iter().filter(|ch| **ch == first).count() >= 3
}

/// Parse common GFM-ish inline markers into plain text + highlight ranges.
pub(in crate::ui::view) fn parse_inline_markdown(input: &str) -> InlineMarkdown {
    let bytes = input.as_bytes();
    let mut text = String::new();
    let mut highlights = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        // Fenced-style inline code: `code`
        if bytes[i] == b'`' {
            if let Some(end) = input[i + 1..].find('`') {
                let inner = &input[i + 1..i + 1 + end];
                if !inner.is_empty() && !inner.contains('\n') {
                    let start = text.len();
                    text.push_str(inner);
                    highlights.push((start..text.len(), InlineMdStyle::Code));
                    i = i + 1 + end + 1;
                    continue;
                }
            }
        }

        // Links: [label](url)
        if bytes[i] == b'[' {
            if let Some(label_end) = input[i + 1..].find(']') {
                let after_label = i + 1 + label_end + 1;
                if input[after_label..].starts_with('(') {
                    if let Some(url_end) = input[after_label + 1..].find(')') {
                        let label = &input[i + 1..i + 1 + label_end];
                        let start = text.len();
                        text.push_str(label);
                        highlights.push((start..text.len(), InlineMdStyle::Link));
                        i = after_label + 1 + url_end + 1;
                        continue;
                    }
                }
            }
        }

        // Bold / bold-italic / italic with * or _
        if bytes[i] == b'*' || bytes[i] == b'_' {
            let marker = bytes[i] as char;
            let rest = &input[i..];
            if rest.starts_with("***") || rest.starts_with("___") {
                let close = format!("{marker}{marker}{marker}");
                if let Some(end) = input[i + 3..].find(&close) {
                    let inner = &input[i + 3..i + 3 + end];
                    if !inner.is_empty() && !inner.contains('\n') {
                        let start = text.len();
                        text.push_str(inner);
                        highlights.push((start..text.len(), InlineMdStyle::BoldItalic));
                        i = i + 3 + end + 3;
                        continue;
                    }
                }
            }
            if rest.starts_with("**") || rest.starts_with("__") {
                let close = format!("{marker}{marker}");
                if let Some(end) = input[i + 2..].find(&close) {
                    let inner = &input[i + 2..i + 2 + end];
                    if !inner.is_empty() && !inner.contains('\n') {
                        let start = text.len();
                        text.push_str(inner);
                        highlights.push((start..text.len(), InlineMdStyle::Bold));
                        i = i + 2 + end + 2;
                        continue;
                    }
                }
            }
            // Single marker italic; avoid matching mid-word underscores when possible.
            if let Some(end) = input[i + 1..].find(marker) {
                let inner = &input[i + 1..i + 1 + end];
                let ok = !inner.is_empty()
                    && !inner.contains('\n')
                    && (marker != '_'
                        || (!inner.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
                            || i == 0
                            || !input[..i]
                                .chars()
                                .next_back()
                                .is_some_and(|c| c.is_ascii_alphanumeric())));
                // Simpler italic rule: single * always; single _ when not mid-word.
                let mid_word_underscore = marker == '_'
                    && i > 0
                    && input[..i]
                        .chars()
                        .next_back()
                        .is_some_and(|c| c.is_ascii_alphanumeric())
                    && inner
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphanumeric());
                if ok && !mid_word_underscore {
                    let start = text.len();
                    text.push_str(inner);
                    highlights.push((start..text.len(), InlineMdStyle::Italic));
                    i = i + 1 + end + 1;
                    continue;
                }
            }
        }

        // Strikethrough: ~~text~~
        if input[i..].starts_with("~~") {
            if let Some(end) = input[i + 2..].find("~~") {
                let inner = &input[i + 2..i + 2 + end];
                if !inner.is_empty() && !inner.contains('\n') {
                    let start = text.len();
                    text.push_str(inner);
                    highlights.push((start..text.len(), InlineMdStyle::Strike));
                    i = i + 2 + end + 2;
                    continue;
                }
            }
        }

        let ch = input[i..].chars().next().unwrap();
        text.push(ch);
        i += ch.len_utf8();
    }

    InlineMarkdown { text, highlights }
}

pub(in crate::ui::view) fn parse_markdown_blocks(content: &str) -> Vec<MarkdownBlock> {
    let mut blocks = Vec::new();
    let mut lines = content.lines().peekable();
    let mut paragraph: Vec<String> = Vec::new();

    let flush_paragraph = |paragraph: &mut Vec<String>, blocks: &mut Vec<MarkdownBlock>| {
        if paragraph.is_empty() {
            return;
        }
        let text = paragraph.join(" ").trim().to_string();
        paragraph.clear();
        if !text.is_empty() {
            blocks.push(MarkdownBlock::Paragraph(text));
        }
    };

    while let Some(line) = lines.next() {
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() {
            flush_paragraph(&mut paragraph, &mut blocks);
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("```") {
            flush_paragraph(&mut paragraph, &mut blocks);
            let language = rest.trim().to_string();
            let mut code_lines = Vec::new();
            while let Some(code_line) = lines.next() {
                if code_line.trim_start().starts_with("```") {
                    break;
                }
                code_lines.push(code_line.to_string());
            }
            blocks.push(MarkdownBlock::Code {
                language,
                code: code_lines.join("\n"),
            });
            continue;
        }

        // GFM pipe table: header + separator + body rows.
        if looks_like_table_header(trimmed) {
            if let Some(next) = lines.peek().copied() {
                if is_table_separator_row(next) {
                    flush_paragraph(&mut paragraph, &mut blocks);
                    let headers = split_table_row(trimmed);
                    lines.next(); // consume separator
                    let mut rows = Vec::new();
                    while let Some(body) = lines.peek().copied() {
                        if body.trim().is_empty() || !body.trim().contains('|') {
                            break;
                        }
                        if body.trim_start().starts_with("```")
                            || body.trim_start().starts_with('#')
                            || body.trim_start().starts_with('>')
                        {
                            break;
                        }
                        rows.push(split_table_row(body));
                        lines.next();
                    }
                    blocks.push(MarkdownBlock::Table { headers, rows });
                    continue;
                }
            }
        }

        if is_thematic_break(trimmed) {
            flush_paragraph(&mut paragraph, &mut blocks);
            blocks.push(MarkdownBlock::ThematicBreak);
            continue;
        }

        if trimmed.trim_start().starts_with('>') {
            flush_paragraph(&mut paragraph, &mut blocks);
            let mut quote_lines = Vec::new();
            let first = trimmed
                .trim_start()
                .strip_prefix('>')
                .map(|s| s.strip_prefix(' ').unwrap_or(s))
                .unwrap_or("")
                .to_string();
            quote_lines.push(first);
            while let Some(next) = lines.peek().copied() {
                let nt = next.trim_end();
                if nt.trim_start().starts_with('>') {
                    let part = nt
                        .trim_start()
                        .strip_prefix('>')
                        .map(|s| s.strip_prefix(' ').unwrap_or(s))
                        .unwrap_or("")
                        .to_string();
                    quote_lines.push(part);
                    lines.next();
                } else {
                    break;
                }
            }
            blocks.push(MarkdownBlock::Quote(quote_lines.join("\n")));
            continue;
        }

        let heading_level = trimmed
            .chars()
            .take_while(|ch| *ch == '#')
            .count()
            .min(6) as u8;
        if heading_level > 0 && trimmed.as_bytes().get(heading_level as usize) == Some(&b' ') {
            flush_paragraph(&mut paragraph, &mut blocks);
            blocks.push(MarkdownBlock::Heading {
                level: heading_level,
                text: trimmed[heading_level as usize + 1..].trim().to_string(),
            });
            continue;
        }
        let bullet = trimmed.trim_start();
        if let Some(rest) = bullet
            .strip_prefix("- ")
            .or_else(|| bullet.strip_prefix("* "))
            .or_else(|| bullet.strip_prefix("+ "))
        {
            flush_paragraph(&mut paragraph, &mut blocks);
            blocks.push(MarkdownBlock::Bullet(rest.to_string()));
            continue;
        }
        if let Some((num, rest)) = bullet.split_once(". ") {
            if !num.is_empty() && num.chars().all(|ch| ch.is_ascii_digit()) {
                flush_paragraph(&mut paragraph, &mut blocks);
                blocks.push(MarkdownBlock::Numbered {
                    index: num.parse().unwrap_or(1),
                    text: rest.to_string(),
                });
                continue;
            }
        }
        paragraph.push(trimmed.trim().to_string());
    }
    flush_paragraph(&mut paragraph, &mut blocks);
    if blocks.is_empty() && !content.trim().is_empty() {
        blocks.push(MarkdownBlock::Paragraph(content.trim().to_string()));
    }
    blocks
}

#[cfg(test)]
mod markdown_tests {
    use super::*;

    #[test]
    fn parse_table_and_inline() {
        let md = "\
# Title

Hello **bold** and `code` and [link](https://example.com).

| A | B |
| --- | --- |
| 1 | 2 |
| 3 | 4 |

> quote line 1
> quote line 2

---
";
        let blocks = parse_markdown_blocks(md);
        assert!(matches!(blocks[0], MarkdownBlock::Heading { level: 1, .. }));
        assert!(matches!(blocks[1], MarkdownBlock::Paragraph(_)));
        match &blocks[2] {
            MarkdownBlock::Table { headers, rows } => {
                assert_eq!(headers, &["A".to_string(), "B".to_string()]);
                assert_eq!(rows.len(), 2);
            }
            other => panic!("expected table, got {other:?}"),
        }
        match &blocks[3] {
            MarkdownBlock::Quote(q) => assert_eq!(q, "quote line 1\nquote line 2"),
            other => panic!("expected quote, got {other:?}"),
        }
        assert!(matches!(blocks[4], MarkdownBlock::ThematicBreak));

        let inline = parse_inline_markdown("Hello **bold** and `code` and [link](https://x)");
        assert_eq!(inline.text, "Hello bold and code and link");
        assert!(
            inline
                .highlights
                .iter()
                .any(|(_, s)| *s == InlineMdStyle::Bold)
        );
        assert!(
            inline
                .highlights
                .iter()
                .any(|(_, s)| *s == InlineMdStyle::Code)
        );
        assert!(
            inline
                .highlights
                .iter()
                .any(|(_, s)| *s == InlineMdStyle::Link)
        );
    }

    #[test]
    fn extract_think_keeps_visible() {
        let (visible, think) =
            extract_think_content("hi <think>secret</think> there");
        assert_eq!(visible, "hi  there");
        assert_eq!(think.as_deref(), Some("secret"));
    }
}

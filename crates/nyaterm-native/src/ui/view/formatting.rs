use gpui::rgb;
use nyaterm_domain::{
    AppSettingsSummary, CloudSyncError, CloudSyncSettings, RiskLevel, TunnelConfig,
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

use super::*;

pub(in crate::features) fn ai_agent_step_status_style(
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

pub(in crate::features) fn format_rate(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1024. * 1024. {
        format!("{:.1} MiB/s", bytes_per_sec / 1024. / 1024.)
    } else if bytes_per_sec >= 1024. {
        format!("{:.1} KiB/s", bytes_per_sec / 1024.)
    } else {
        format!("{bytes_per_sec:.0} B/s")
    }
}

pub(in crate::features) fn format_uptime(seconds: u64) -> String {
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

pub(in crate::features) fn risk_label(risk: Option<&RiskLevel>) -> &'static str {
    match risk {
        Some(RiskLevel::Low) => "Low",
        Some(RiskLevel::Medium) => "Medium",
        Some(RiskLevel::High) => "High",
        Some(RiskLevel::Critical) => "Critical",
        None => "Unrated",
    }
}

pub(in crate::features) fn recording_file_path(
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

pub(in crate::features) fn docker_state_rank(state: &str) -> u8 {
    match state.trim().to_ascii_lowercase().as_str() {
        "running" => 0,
        "restarting" | "paused" => 1,
        "created" => 2,
        "exited" | "dead" => 3,
        _ => 4,
    }
}

pub(in crate::features) fn docker_state_label(state: &str) -> &'static str {
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

pub(in crate::features) fn docker_state_color(palette: ThemePalette, state: &str) -> gpui::Hsla {
    match state.trim().to_ascii_lowercase().as_str() {
        "running" => rgb(palette.success).into(),
        "restarting" | "paused" => rgb(palette.warning).into(),
        "created" => rgb(palette.link).into(),
        "exited" | "dead" => rgb(palette.danger).into(),
        _ => rgb(palette.text_muted).into(),
    }
}

pub(in crate::features) fn docker_compose_project_key(
    project_name: &str,
    config_files: Option<&str>,
) -> String {
    format!(
        "{}\n{}",
        project_name.trim(),
        config_files.unwrap_or_default().trim()
    )
}

pub(in crate::features) fn session_kind_label(kind: SessionKind) -> &'static str {
    match kind {
        SessionKind::LocalPty => "local",
        SessionKind::Ssh => "ssh",
        SessionKind::Telnet => "telnet",
        SessionKind::RawTcp => "raw tcp",
        SessionKind::Serial => "serial",
    }
}

pub(in crate::features) fn cloud_sync_history_status(error: &CloudSyncError) -> &'static str {
    match error {
        CloudSyncError::Conflict(_) => "conflict",
        _ => "failed",
    }
}

pub(in crate::features) fn configured_cloud_sync_provider(settings: &CloudSyncSettings) -> String {
    let provider = settings.provider.trim();
    if provider.is_empty() {
        "local_directory".to_string()
    } else {
        provider.to_string()
    }
}

pub(in crate::features) fn none_if_blank(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

pub(in crate::features) fn recent_terminal_output(output: &str, max_lines: usize) -> String {
    let lines = output.lines().collect::<Vec<_>>();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join("\n")
}

pub(in crate::features) fn cloud_secret_display(draft: &str, current: &Option<String>) -> String {
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

pub(in crate::features) fn compact_id(value: &str) -> String {
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

pub(in crate::features) fn format_cloud_provider(provider: &str) -> String {
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

pub(in crate::features) fn format_terminal_line_timestamp_ms(
    timestamp_ms: u64,
    include_milliseconds: bool,
) -> String {
    let secs = (timestamp_ms / 1000) as i64;
    let millis = timestamp_ms % 1000;
    let hours = ((secs % 86_400) / 3_600).rem_euclid(24);
    let minutes = ((secs % 3_600) / 60).rem_euclid(60);
    let seconds = (secs % 60).rem_euclid(60);
    if include_milliseconds {
        format!("[{hours:02}:{minutes:02}:{seconds:02}.{millis:03}]")
    } else {
        format!("[{hours:02}:{minutes:02}:{seconds:02}]")
    }
}

pub(in crate::features) fn format_history_timestamp_ms(timestamp_ms: u64) -> String {
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

pub(in crate::features) fn format_duration_ms(duration_ms: Option<u64>) -> Option<String> {
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

pub(in crate::features) fn cloud_sync_status_dot_color(
    palette: ThemePalette,
    status: &str,
) -> gpui::Rgba {
    match status {
        "running" => rgb(palette.link),
        "success" => rgb(palette.success),
        "failed" => rgb(palette.danger),
        "conflict" => rgb(palette.warning),
        "disabled" => rgb(palette.text_dimmed),
        _ => rgb(palette.text_muted),
    }
}

pub(in crate::features) fn cloud_sync_status_text_color(
    palette: ThemePalette,
    status: &str,
) -> gpui::Rgba {
    match status {
        "running" => rgb(palette.link),
        "success" => rgb(palette.success),
        "failed" => rgb(palette.danger),
        "conflict" => rgb(palette.warning),
        "disabled" => rgb(palette.text_dimmed),
        _ => rgb(palette.text_muted),
    }
}

pub(in crate::features) fn cloud_sync_kind_text_color(
    palette: ThemePalette,
    kind: &str,
) -> gpui::Rgba {
    match kind {
        "sync" => rgb(palette.link),
        "backup" => rgb(palette.link),
        _ => rgb(palette.text_muted),
    }
}

pub(in crate::features) fn cloud_sync_history_summary(entry: &CloudSyncHistoryEntry) -> String {
    let normalized = entry
        .message
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
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

pub(in crate::features) fn normalize_startup_command(value: &str) -> String {
    let mut command = value.trim().replace("\r\n", "\n").replace('\r', "\n");
    if !command.ends_with('\n') {
        command.push('\n');
    }
    command
}

pub(in crate::features) fn short_id(id: &str) -> &str {
    id.get(..8).unwrap_or(id)
}

pub(in crate::features) fn status_label(status: &str) -> &'static str {
    if status.starts_with("running") {
        "session running"
    } else if status.contains("failed") || status.contains("error") {
        "session attention"
    } else {
        "session ready"
    }
}

pub(in crate::features) fn non_empty_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub(in crate::features) fn split_shell_args(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(in crate::features) fn parse_telnet_enter_mode(value: &str) -> TelnetEnterMode {
    match value {
        "crlf" => TelnetEnterMode::Crlf,
        "lf" => TelnetEnterMode::Lf,
        _ => TelnetEnterMode::Cr,
    }
}

pub(in crate::features) fn download_file_name_from_remote_path(remote_path: &str) -> String {
    remote_path
        .trim()
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty() && *name != ".")
        .unwrap_or("nyaterm-download.bin")
        .to_string()
}

pub(in crate::features) fn tunnel_mode(tunnel: &TunnelConfig) -> Option<SshTunnelMode> {
    match tunnel.tunnel_type.as_str() {
        "local" => Some(SshTunnelMode::Local),
        "remote" => Some(SshTunnelMode::Remote),
        "dynamic" => Some(SshTunnelMode::Dynamic),
        _ => None,
    }
}

pub(in crate::features) fn tunnel_mode_label(tunnel: &TunnelConfig) -> &'static str {
    match tunnel.tunnel_type.as_str() {
        "local" => "Local",
        "remote" => "Remote",
        "dynamic" => "SOCKS5",
        _ => "Tunnel",
    }
}

pub(in crate::features) fn tunnel_name(tunnel: &TunnelConfig) -> String {
    if tunnel.name.trim().is_empty() {
        tunnel.id.clone()
    } else {
        tunnel.name.clone()
    }
}

pub(in crate::features) fn tunnel_endpoint(tunnel: &TunnelConfig, listen: &str) -> String {
    match tunnel.tunnel_type.as_str() {
        "dynamic" => format!("{listen} SOCKS5"),
        "remote" => format!(
            "remote {} -> {}:{}",
            tunnel.listen_port, tunnel.target_host, tunnel.target_port
        ),
        _ => format!("{listen} -> {}:{}", tunnel.target_host, tunnel.target_port),
    }
}

pub(in crate::features) fn format_permissions_octal(mode: u32) -> String {
    format!("{:04o}", mode & 0o7777)
}

pub(in crate::features) fn trim_terminal_output_to(output: &mut String, max_bytes: usize) {
    if max_bytes == 0 || output.len() <= max_bytes {
        return;
    }
    let drain_to = output
        .char_indices()
        .find_map(|(index, _)| (index >= output.len() - max_bytes).then_some(index))
        .unwrap_or(0);
    output.drain(..drain_to);
}

pub(in crate::features) fn ssh_multiplex_key(config: &SshSessionConfig) -> String {
    format!(
        "{}@{}:{}",
        config.username.trim(),
        config.host.trim().to_ascii_lowercase(),
        config.port
    )
}

pub(in crate::features) fn format_last_used_ms(last_used_at_ms: Option<u64>) -> String {
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

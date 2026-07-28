use super::NyaTermApp;

mod labels;
pub(in crate::features) use labels::{
    ai_agent_step_status_style, cloud_sync_history_status, cloud_sync_history_summary,
    cloud_sync_kind_text_color, cloud_sync_status_dot_color, cloud_sync_status_text_color,
    compact_id, configured_cloud_sync_provider, docker_compose_project_key, docker_state_color,
    docker_state_label, docker_state_rank, download_file_name_from_remote_path,
    format_cloud_provider, format_duration_ms, format_history_timestamp_ms, format_last_used_ms,
    format_permissions_octal, format_rate, format_terminal_line_timestamp_ms, format_uptime,
    non_empty_string, none_if_blank, normalize_startup_command, parse_telnet_enter_mode,
    recent_terminal_output, recording_file_path, risk_label, session_kind_label, short_id,
    split_shell_args, ssh_multiplex_key, status_label, trim_terminal_output_to, tunnel_endpoint,
    tunnel_mode, tunnel_name,
};

mod ai_history;
pub(in crate::features) use ai_history::group_ai_sessions_by_date;

mod markdown;
pub(in crate::features) use markdown::{
    InlineMdStyle, MarkdownBlock, extract_think_content, parse_inline_markdown,
    parse_markdown_blocks,
};

impl NyaTermApp {
    pub(in crate::features) fn tr(&self, key: &'static str) -> &'static str {
        crate::i18n::text(&self.settings.language, key)
    }
}

#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

failures=0

fail() {
  printf 'architecture-boundary: %s\n' "$*" >&2
  failures=$((failures + 1))
}

require_rg() {
  if ! command -v rg >/dev/null 2>&1; then
    fail "ripgrep (rg) is required"
    return 1
  fi
}

count_matches() {
  local pattern="$1"
  local file="$2"
  if [[ ! -f "$file" ]]; then
    printf '0'
    return
  fi
  (rg -n "$pattern" "$file" || true) | wc -l | tr -d ' '
}

check_max_count() {
  local label="$1"
  local pattern="$2"
  local file="$3"
  local max_count="$4"
  local count
  count="$(count_matches "$pattern" "$file")"
  if (( count > max_count )); then
    fail "$label exceeded baseline in $file ($count > $max_count)"
  fi
}

check_no_matches() {
  local label="$1"
  local pattern="$2"
  local path="$3"
  if rg -n "$pattern" "$path" >/tmp/nyaterm-architecture-boundary.$$ 2>/dev/null; then
    fail "$label"
    sed 's/^/  /' /tmp/nyaterm-architecture-boundary.$$ >&2
  fi
  rm -f /tmp/nyaterm-architecture-boundary.$$
}

check_no_matches_in_rust_fn() {
  local label="$1"
  local file="$2"
  local fn_name="$3"
  local pattern="$4"
  local tmp="/tmp/nyaterm-architecture-boundary-fn.$$"
  awk -v fn_name="$fn_name" -v pattern="$pattern" '
    $0 ~ "fn " fn_name "\\(" {
      in_fn = 1
      depth = 0
    }
    in_fn {
      if ($0 ~ pattern) {
        printf "%s:%d:%s\n", FILENAME, FNR, $0
      }
      for (i = 1; i <= length($0); i++) {
        ch = substr($0, i, 1)
        if (ch == "{") {
          depth++
        } else if (ch == "}") {
          depth--
        }
      }
      if (depth == 0) {
        in_fn = 0
      }
    }
  ' "$file" >"$tmp"
  if [[ -s "$tmp" ]]; then
    fail "$label"
    sed 's/^/  /' "$tmp" >&2
  fi
  rm -f "$tmp"
}

require_rg || exit 1

# Low-level crates must stay independent of GPUI and desktop presentation code.
check_no_matches \
  "nyaterm-terminal must not depend on GPUI" \
  '^gpui(\.workspace)?\s*=' \
  crates/nyaterm-terminal/Cargo.toml

check_no_matches \
  "nyaterm-transport must not depend on nyaterm-desktop" \
  '^nyaterm-desktop(\.workspace)?\s*=' \
  crates/nyaterm-transport/Cargo.toml

check_no_matches \
  "quick switch app mirror fields must not return" \
  'quick_switch_(open|query|marked_text|selected_index)' \
  crates/nyaterm-desktop/src/features/app_state
check_no_matches \
  "QuickSwitchState fields must remain private; use OverlayStore mutations and read-only getters" \
  '^[[:space:]]*pub[[:space:]]+(open|query|marked_text|selected_index)[[:space:]]*:' \
  crates/nyaterm-desktop/src/entities/overlay.rs

check_no_matches \
  "terminal assist fields must stay grouped under TerminalFeatureState" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+(command_suggestions|command_input_tracker|command_suggestions_suppressed|pending_command_history_entry|command_suggestion_search_gen|command_suggestion_refresh_task|credential_suggestions|credential_autofill_buffer|credential_autofill_recent|credential_autofill_pending|credential_autofill_detection_pending|credential_autofill_next_request_id|credential_autofill_pending_request|credential_autofill_match_pipeline|credential_autofill_sending|credential_prompt_input_until_ms)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs

check_no_matches \
  "terminal presentation runtime fields must stay grouped under TerminalFeatureState" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+(action_link_menu|action_link_tooltip|action_link_hover_pending|multi_line_paste|multi_line_paste_marked_text|multi_line_paste_marked_range|multi_line_paste_cursor|multi_line_paste_anchor|multi_line_paste_focus|pending_terminal_frame_events|cached_terminal_theme_palette|cached_keyword_highlight_rules)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs

check_no_matches \
  "sync input fields must stay grouped under SyncInputFeatureState" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+(sync_groups|sync_groups_open|sync_groups_focus|sync_groups_search_draft|sync_groups_selected_id|sync_groups_delete_pending|broadcast_to_all)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs

check_no_matches \
  "bottom panel fields must stay grouped under ShellFeatureState" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+(bottom_panel|quick_cmd_height|serial_send_height|bottom_panel_resize)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs

check_no_matches \
  "window shell state must stay grouped under ShellFeatureState" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+(last_viewport_size|wallpaper_tile_dimensions|last_viewport_change_at|title_drag_active_until|selected_nav|main_mode|settings_active_tab|settings_expanded_groups|settings_draft_snapshot|settings_window|settings_window_open_pending|settings_previous_left_collapsed|settings_previous_right_collapsed|active_left_panel|active_right_panel|left_open_panels|right_open_panels|panel_stack_sizes|panel_multi_open|right_focus|left_sidebar_collapsed|right_inspector_collapsed|mobile_left_open|mobile_right_open|left_panel_width|right_panel_width|panel_resize|panel_stack_resize|activity_bar_layout|activity_bar_context_menu|title_menu_open|title_menu_submenu|header_status|open_tabs_menu_open|new_session_menu_open|new_session_all_sessions_open|new_session_group_menu_path|session_tab_strip_scroll|session_tab_scroll_into_view_pending|last_connect_failure_name|last_connect_failure_error|workspace_split|workspace_split_resize|session_pane_roots|session_tab_owner|focused_terminal_window_leaf_id|workspace_pane_layout_restored)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs

check_no_matches \
  "panel resize transitions must stay on ShellPanelState" \
  'PanelResizeState' \
  crates/nyaterm-desktop/src/features/shell/panel_resize_runtime.rs
check_no_matches \
  "panel stack resize transitions must stay on ShellPanelState" \
  'PanelStackResizeState' \
  crates/nyaterm-desktop/src/features/shell/panel_stack_runtime.rs

check_no_matches \
  "screen lock fields must stay grouped under SecurityFeatureState" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+(lock_focus|lock_password_draft|lock_status|is_locked|last_user_activity_at)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs

check_no_matches \
  "sync input pure mutations must stay on SyncInputFeatureState" \
  'fn[[:space:]]+(delete_selected_sync_group|selected_sync_group_mut|next_sync_group_color)[[:space:]]*\(' \
  crates/nyaterm-desktop/src/features

check_no_matches \
  "session lifecycle must update sync membership through SyncInputFeatureState" \
  '&mut[[:space:]]+self\.sync_input\.groups' \
  crates/nyaterm-desktop/src/features/session/session_lifecycle.rs

check_no_matches \
  "remote job lifecycle must stay behind the pane state API" \
  'remote_ops\.(docker|process|stats)\.(tx|rx|pending|job_id|job_session_id)' \
  crates/nyaterm-desktop/src/features
check_no_matches \
  "remote event identity matching must stay on RemoteJobState" \
  'fn[[:space:]]+remote_job_event_matches[[:space:]]*\(' \
  crates/nyaterm-desktop/src/features

check_no_matches \
  "settings transient UI fields must stay grouped under SettingsFeatureState" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+(search_engine_edit_index|search_engine_expanded_index|search_engine_icon_picker_index|search_engine_actions_index|search_engine_edit_field|search_engine_focus|keyword_highlight_expanded_id|keyword_highlight_edit_id|keyword_highlight_edit_field|keyword_highlight_focus|appearance_menu_open|appearance_ui_font_options|appearance_terminal_font_options|keybinding_recording_id|keybinding_pending_keys|keybinding_search_draft|keybindings_focus)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs

check_no_matches \
  "compatibility-sensitive settings must stay grouped under SettingsFeatureState" \
  '(AppSettingsSummary|KeywordHighlightConfig|SettingsMasterPasswordState|StoreStatus|settings_master_password_(enabled|draft))' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs

check_no_matches \
  "translation state must stay grouped under TranslationFeatureState" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+(translation_dialog|translate_tx|translate_rx|translate_provider|translation_settings|translation_secret_draft|translate_target_language|translate_input|translate_result|translate_status|translate_pending|translate_focused_field)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs

check_no_matches \
  "update state must stay grouped under UpdateFeatureState" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+(update_tx|update_rx|update_status|update_info|update_pending|update_dialog_open)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs

check_no_matches \
  "translation and update job events must stay in their owning features" \
  'struct[[:space:]]+(TranslateJobResult|UpdateJobResult)' \
  crates/nyaterm-desktop/src/features/runtime_jobs.rs

check_no_matches \
  "translation job lifecycle must mutate through TranslationFeatureState" \
  '(self|this)\.translation\.(tx|rx|pending|result|dialog|focused_field)[[:space:]]*=' \
  crates/nyaterm-desktop/src/features
check_no_matches \
  "translation settings replacement must preserve owner invariants" \
  '(self|this)\.translation\.(settings|secret_draft|target_language)[[:space:]]*=' \
  crates/nyaterm-desktop/src/features
check_no_matches \
  "update job lifecycle must mutate through UpdateFeatureState" \
  '(self|this)\.update\.(tx|rx|status|info|pending|dialog_open)[[:space:]]*=' \
  crates/nyaterm-desktop/src/features

check_no_matches \
  "cloud sync state must stay grouped under CloudSyncFeatureState" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+(cloud_sync_settings|cloud_sync_state|cloud_sync_history|cloud_sync_history_expanded|cloud_sync_conflict|cloud_sync_secret_draft|cloud_sync_status|cloud_sync_job_running|cloud_sync_focused_field|cloud_sync_provider_menu_open|github_gist_auth|github_gist_auth_tx|github_gist_auth_rx|github_gist_auth_job_id|github_gist_auth_cancel)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs

check_no_matches \
  "cloud sync input mutation must stay on CloudSyncFeatureState" \
  'fn[[:space:]]+cloud_sync_input_value_mut[[:space:]]*\(' \
  crates/nyaterm-desktop/src/features

check_no_matches \
  "recording runtime state must stay grouped under RecordingFeatureState" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+(recording_manager|recording_active_count|pending_auto_recording_session|recording_write_pipeline|recording_search_draft|recording_busy_actions|recording_path_prompt)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs

check_no_matches \
  "tunnel runtime state must stay grouped under TunnelFeatureState" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+(tunnel_manager|tunnel_tx|tunnel_rx|pending_tunnels)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs

check_no_matches \
  "connection catalogs must stay in their authoritative feature owners" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+(connections|connection_groups|connection_ssh_keys|connection_otp_entries|connection_saved_passwords|connection_saved_credentials|connection_serial_ports|tunnels|tunnel_groups|proxies|proxy_groups|tunnel_runtime)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs

check_no_matches \
  "command catalog, UI, history and runtime fields must stay grouped under CommandFeatureState" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+(quick_commands|quick_command_categories|quick_command_state|command_history|command_runtime)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs

check_no_matches \
  "live session runtime state must stay grouped under SessionFeatureState" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+(session_manager|session_event_bridge|session_start|session_command_history|active_sessions_search_draft|active_session_menu|active_session_busy_actions|active_session_id|active_ssh_config|active_ai_execution_profile|session_order|session_metadata|session_custom_names|session_dynamic_titles|session_cwds|zmodem_sessions|trzsz_sessions|session_tab_colors|ssh_multiplex_handles)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs

check_no_matches \
  "session prompt runtime must stay grouped under SessionPromptState" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+(duplicate_prompts|active_duplicate_prompt|host_key_prompts|active_host_key_prompt|credential_prompts|active_credential_prompt|active_keyboard_interactive_prompt|credential_prompt_focus_pending|credential_focus|otp_provider)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs
check_no_matches \
  "SessionPromptState fields must remain private; use owner transitions and read-only getters" \
  '^[[:space:]]*pub([[:space:]]|\([^)]*\))[[:space:]]+(duplicate_prompts|active_duplicate_prompt|host_key_prompts|active_host_key_prompt|credential_prompts|active_credential_prompt|active_keyboard_interactive_prompt|credential_prompt_focus_pending|credential_focus|otp_provider)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/session/state.rs

check_no_matches \
  "session dialogs must stay grouped under SessionDialogState" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+(tab_actions_session_id|tab_actions_anchor|tab_actions_submenu|tab_actions_focus|close_all_sessions_confirm_open|pending_quit_after_close_all|pending_window_quit|close_all_sessions_confirm_focus|rename_session_id|rename_draft|rename_focus|color_picker_open|color_picker_focus|session_info_open|session_info_focus|startup_command_open|startup_command_action|startup_command_draft|startup_command_delay_ms|startup_command_focus|temporary_ssh_link_open|temporary_ssh_link_draft|temporary_ssh_link_error|temporary_ssh_link_focus)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs
check_no_matches \
  "SessionDialogState fields must remain private; use owner transitions and read-only getters" \
  '^[[:space:]]*pub([[:space:]]|\([^)]*\))[[:space:]]+(tab_actions_session_id|tab_actions_anchor|tab_actions_submenu|tab_actions_focus|close_all_sessions_confirm_open|pending_quit_after_close_all|pending_window_quit|close_all_sessions_confirm_focus|rename_session_id|rename_draft|rename_focus|color_picker_open|color_picker_focus|session_info_open|session_info_focus|startup_command_open|startup_command_action|startup_command_draft|startup_command_delay_ms|startup_command_focus|temporary_ssh_link_open|temporary_ssh_link_draft|temporary_ssh_link_error|temporary_ssh_link_focus)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/session/state.rs

check_no_matches \
  "session start state must stay grouped under SessionStartFeatureState" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+(session_start_tx|session_start_rx|pending_session_starts|active_pending_session_start|failed_session_starts|active_failed_session_start|cancelled_session_start_requests|session_pane_states|pending_reconnect_replace_id|reconnect_session_failures|pending_workspace_split)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs
check_no_matches \
  "SessionStartFeatureState lifecycle fields must remain private" \
  '^[[:space:]]*pub([[:space:]]|\([^)]*\))[[:space:]]+(active_pending|active_failed|cancelled|panes|reconnect_replace_id|reconnect_failures|pending_workspace_split|saved_connection_queue)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/session/state.rs
check_no_matches \
  "SessionStartFeatureState maps and result channels must remain private" \
  '^[[:space:]]*pub([[:space:]]|\([^)]*\))[[:space:]]+((pending|failed)[[:space:]]*:[[:space:]]*HashMap<String,[[:space:]]*(PendingSessionStart|FailedSessionStart)>|(tx|rx)[[:space:]]*:[[:space:]]*mpsc::(Sender|Receiver)<SessionStartResult>)' \
  crates/nyaterm-desktop/src/features/session/state.rs
check_no_matches \
  "retired SessionPaneState write-only projection must not return" \
  'SessionPaneState' \
  crates/nyaterm-desktop/src

check_no_matches \
  "session start models must stay with SessionStartFeatureState" \
  '(struct[[:space:]]+(PendingSessionStart|FailedSessionStart|PendingSavedConnectionStart|SavedConnectionStartOptions)|enum[[:space:]]+SessionPaneState)' \
  crates/nyaterm-desktop/src/features/app_state/types.rs

check_no_matches \
  "saved connection start queue must stay under SessionStartFeatureState" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+pending_saved_connection_queue[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/app_state/mod.rs

# These low-frequency transport helpers have explicit imports at their call
# sites. Keep them out of the shared feature prelude so new modules do not
# acquire unrelated transport dependencies implicitly.
check_no_matches \
  "low-frequency transport helpers must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(SFTP_TRANSFER_CANCELLED|SftpTransferDirection|RemoteStatsService|TerminalHistorySearchRequest|open_ssh_multiplex_handle|run_local_command)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency core helpers must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(AgentApprovalDecision|AgentCapturedOutput|AgentCommandExecutionMode|AiChatStreamDelta|AiContext|AiMessage|AiMessageRole|AiModelDiscovery|AiSession|AppendAiAuditRequest|CLOUD_SYNC_HISTORY_LIMIT|CloudSyncResult|DiagnosticsExportOptions|DiagnosticsRuntimeSnapshot|GiteeSnippetHttpBackend|GithubGistHttpBackend|KeywordHighlightRule|LocalCloudSyncOptions|SearchEngineConfig|SnippetRemote|TerminalMouseReportEligibility|TerminalResizeGeometry|TerminalViewportInsets|TranslateResult|TranslationSettings|agent_response_action|ai_model_id_for_credential|ai_model_id_for_provider|append_cloud_sync_history|assess_agent_command_risk|build_agent_capture_command|build_observation_message|decide_agent_command_execution|export_diagnostics_archive|merge_model_discoveries|parse_agent_model_output|parse_agent_tool_call|parse_model_output|pull_local_snapshot|pull_snapshot_with_remote|push_local_snapshot|push_snapshot_with_remote|read_cloud_sync_history|redact_context|redact_sensitive_text|terminal_mouse_report_should_send|terminal_resize_geometry_for_size_with_insets|terminal_resize_geometry_for_size_with_insets_and_scale|terminal_snapped_cell_height)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "session-specific core types must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(ConnectionAuth|CredentialPromptKind|DecryptedOtpEntry|KnownHostCheck)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "session-specific transport types must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(LocalSessionConfig|SerialSessionConfig|SftpDuplicateRequest|SshCredentialPrompt|SshCredentialProvider|SshHostKey|SshHostKeyDecision|SshHostKeyVerifier|SshKeyAuthConfig|SshKeyboardInteractiveRequest|SshOtpProvider|SshProxyConfig|TelnetSessionConfig)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "session-specific standard-library types must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(Mutex|SystemTime|UNIX_EPOCH)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "session-specific UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(CredentialAutofillMatchEvent|CredentialAutofillMatchOutcome|CredentialAutofillMatchRequest)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency widgets must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(mode_button|session_info_row|svg_icon_button)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency GPUI types must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])WindowControlArea([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "send-command helpers must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(build_send_command_units_for|format_send_command_hex_display|parse_send_command_hex)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "shortcut formatting helper must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])event_to_hotkey_string([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "panels module entry points must use named helper imports" \
  '^[[:space:]]*(pub\([^)]*\)[[:space:]]+)?use[[:space:]]+(helpers|send_command_helpers)::\*;' \
  crates/nyaterm-desktop/src/features/panels/mod.rs \
  crates/nyaterm-desktop/src/features/panels/tab_actions_overlay/mod.rs

check_no_matches \
  "presentation support module entry points must use named re-exports" \
  '^[[:space:]]*pub\(in crate::features\)[[:space:]]+use[[:space:]]+(formatting|view_widgets|labels|ai_history|markdown|chrome|inspector_widgets|stats|rows|icons)::\*;' \
  crates/nyaterm-desktop/src/features

check_no_matches \
  "cloud sync HTTP backend helpers must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(NativeAliyunDriveRemote|NativeGoogleDriveRemote|NativeOneDriveRemote|NativeS3Remote|NativeSnippetHttpClient|NativeWebdavRemote|run_github_gist_device_flow)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "AI HTTP facade helpers must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(complete_native_chat|discover_openai_compatible_models|stream_native_chat)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency sync UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(GithubGistAuthEvent|GithubGistAuthJobEvent|GithubGistAuthState)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency cloud sync prompt UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(CloudSyncConflictState|CloudSyncSecretDraft)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "service runtime dependencies must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(CloudSyncError|DockerService)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "transfer runtime dependencies must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(AiAction|PathPromptOptions|SftpDuplicateResolver|SftpFileEntry|SftpService|SftpTransferControl|SftpTransferOptions|SftpTransferProgress|SshProcessService|TransferJobEvent|TransferJobKind|TransferJobOutput|TransferJobState|TransferJobStatus)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "service runtime internals must stay unflattened from the features facade" \
  '(^|[,{[:space:]])(cloud_sync_history_status|DockerJobOutput|ProcessJobOutput|remote_job_event_matches)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/mod.rs

check_no_matches \
  "remote runtime module entry point must use named helper imports" \
  '^[[:space:]]*use[[:space:]]+helpers::\*;' \
  crates/nyaterm-desktop/src/features/remote/remote_runtime/mod.rs

check_no_matches \
  "transfer runtime internals must stay unflattened" \
  '(^|[,{[:space:]])format_permissions_octal([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/mod.rs

check_no_matches \
  "transfer module entry points must use named internal imports" \
  '^[[:space:]]*(pub\([^)]*\)[[:space:]]+)?use[[:space:]]+(helpers::\*|transfer_widgets::\*);|(^|[,{[:space:]])format_transfer_progress([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/transfers/mod.rs \
  crates/nyaterm-desktop/src/features/transfers/transfer_jobs/mod.rs

check_no_matches \
  "cloud sync HTTP module entry point must use named imports and re-exports" \
  '^[[:space:]]*(pub(\([^)]*\))?[[:space:]]+)?use[[:space:]]+(helpers|aliyun|github_gist_auth|google_drive|onedrive|s3|snippet|webdav)::\*;' \
  crates/nyaterm-desktop/src/http/cloud_sync/mod.rs

check_no_matches \
  "low-frequency translation UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TranslateInputField|TranslationDialogState|TranslationSecretDraft)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency settings UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(SearchEngineEditorField|SettingsTab)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency chrome UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(ActiveSessionMenuState|ActivityBarContextMenuState|ActivityBarEntry|ActivityBarLayoutState|ActivityBarZone|BottomPanelMode|MainMode|RightFocus)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal action-link UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(ActionLinkMenuAction|ActionLinkMenuState|ActionLinkTooltipState)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency diagnostics prompt UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(DiagnosticsPathPromptKind|DiagnosticsPathPromptResult)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency remote Docker UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(DockerConfirmAction|DockerConfirmState|DockerTab)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency remote process UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(RemoteProcessSignalConfirmState|RemoteProcessSortDirection|RemoteProcessSortKey)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency layout resize UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(BottomPanelResizeState|PanelResizeSide|PanelResizeState|PanelStackResizeState|TransferHeightResizeState|WorkspaceSplitResizeState)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency chrome menu UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TabActionsSubmenu|TitleMenu|TitleMenuSubmenu)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency quick switch UI model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(QuickSwitchItem)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency keyword-highlight UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(KeywordHighlightEditorField|KeywordHighlightPathPromptKind|KeywordHighlightPathPromptResult)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency recording path prompt UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(RecordingPathPromptKind|RecordingPathPromptResult)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency quick-command import prompt UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(QuickCommandImportPathPromptKind|QuickCommandImportPathPromptResult)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency recording pipeline UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(RecordingHistorySearchEvent|RecordingHistorySearchKey|RecordingWriteEvent|RecordingWritePipeline)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency quick-command menu dialog UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(QuickCommandCategoryDeleteState|QuickCommandCategoryMenuState|QuickCommandCategoryRenameState|QuickCommandDeleteState|QuickCommandDetailsState|QuickCommandRowMenuState)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency quick-command view/sort UI modes must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(QuickCommandSortMode|QuickCommandViewMode)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency transfer path prompt UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TransferPathPromptKind|TransferPathPromptResult)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency transfer browser UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TransferBrowserColumnResizeState|TransferBrowserColumnWidths|TransferBrowserContextMenuState|TransferBrowserDragSelectionState|TransferBrowserFavoritesMenuState|TransferBrowserPendingRenameState|TransferBrowserSessionCacheState|TransferBrowserUploadMenuState)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency transfer file-operation UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TransferNewFileState|TransferNewFolderState|TransferNewSymlinkState|TransferPropertiesState|TransferRenameState|TransferUnknownFileState)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency transfer operation UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TransferDeleteState|TransferExternalSyncPromptState|TransferJobDeleteState|TransferJobMenuState|TransferMoveState)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency transfer editor workspace UI model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TransferEditorWorkspaceState)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency transfer input focus UI model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TransferInputField)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency transfer browser sort UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TransferBrowserSortColumn|TransferBrowserSortDirection)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency snapshot password/store status UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(SnapshotPasswordPromptKind|SnapshotPasswordPromptState|StoreStatus)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency sync input and paste draft UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(MultiLinePasteDraft|SyncInputGroup)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency config path prompt UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(ConfigPathPromptKind|ConfigPathPromptResult)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency AI prepared request/menu UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(AiMessageMenuState|AiPreparedRequest)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency AI settings UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(AiActionEditorField|AiActionListKind|AiCredentialEditorField|AiInputField)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency command suggestion UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(CommandSuggestionItem|CommandSuggestionState)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency AI detected-error UI model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(AiDetectedErrorState)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency quick-command editor UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(QuickCommandEditorField|QuickCommandEditorState|QuickCommandVariableDef|QuickCommandVariablePromptState)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency security UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(SecurityAuthTab|SecurityCredentialEditorField|SecurityCredentialEditorState|SecurityDeleteConfirmState|SecurityKeyEditorField|SecurityKeyEditorState|SecurityOtpEditorField|SecurityOtpEditorState|SecurityPasswordEditorField|SecurityPasswordEditorState|SecurityUnlockAction)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency credential autofill UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(CredentialSuggestionState|PendingCredentialAutofill)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal context menu UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TerminalContextMenuState|TerminalContextSubmenu)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal paint policy UI model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(EffectiveTerminalPaintPolicy)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal performance overlay UI model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TerminalPerformanceMode|TerminalPerformanceOverlay)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal protocol state model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TerminalProtocolState)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal cell position model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TerminalCellPos)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal selection model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TerminalSelection)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal frame buffer-text event model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TerminalFrameBufferTextEvent)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal frame output event model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TerminalFrameOutputEvent)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal frame output submission model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TerminalFrameOutputSubmission)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal frame search event model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TerminalFrameSearchEvent)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal frame snapshot event model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TerminalFrameSnapshotEvent)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal frame event model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TerminalFrameEvent)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal frame search key model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TerminalFrameSearchKey)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal frame pipeline model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TerminalFramePipeline)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal UI output tail cap must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TERMINAL_UI_OUTPUT_TAIL_CAP)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal search result helper must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(terminal_frame_search_result_is_current)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal snapshot geometry helper must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(terminal_snapshot_matches_grid_geometry)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal expensive interactions helper must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(terminal_expensive_interactions_enabled)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal action-link matcher helper must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(terminal_action_link_matcher_key)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency terminal search UI model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(TerminalSearchMode)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency session runtime models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(SessionEventBridge|SessionRuntimeMetadata)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency startup command request model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(StartupCommandAction|StartupCommandRequest)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency workspace smart split UI model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(SmartSplitMode)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency workspace tab split UI model must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(SplitEdge|TabDockZone|TerminalWindowNode|WorkspacePaneNode|WorkspaceSplitDirection)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency workspace split compatibility alias must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(WorkspaceSplitState)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency send-command UI models must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(SendCommandControlFocus|SendCommandDataType|SendCommandLineEnding|SendCommandMode|SendCommandTarget)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "shortcut matching helper must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(shortcut_matches)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

# Keep the terminal GPUI crate root's public surface explicit. These modules
# are implementation details; callers use the named facade exports below.
check_no_matches \
  "terminal-gpui must not re-export implementation modules with glob imports" \
  '^pub[[:space:]]+use[[:space:]]+(images|keywords|paint)::\*;' \
  crates/nyaterm-terminal-gpui/src/lib.rs

# Migration inventory/dashboard code and the local legacy source checkout have
# reached their exit conditions. Keep the product, docs and asset tooling free
# of those retired boundaries.
check_no_matches 'retired migration dashboard feature must not return' \
  'migration-dashboard' Cargo.toml
check_no_matches 'retired migration dashboard feature must not return' \
  'migration-dashboard' crates
check_no_matches 'retired legacy inventory crate must not return' \
  'nyaterm[-_]legacy' Cargo.toml
check_no_matches 'retired legacy inventory crate must not return' \
  'nyaterm[-_]legacy' crates
check_no_matches 'retired local source checkout must not return in crates' \
  'nyaterm-tauri' crates
check_no_matches 'retired local source checkout must not return in docs' \
  'nyaterm-tauri' docs
check_no_matches 'retired local source checkout must not return in repository guidance' \
  'nyaterm-tauri' AGENTS.md
check_no_matches 'retired local source checkout must not return in icon manifest' \
  'nyaterm-tauri' scripts/icons.manifest
check_no_matches 'retired local source checkout must not return in icon sync tooling' \
  'nyaterm-tauri' scripts/sync-icons.sh

# The desktop crate no longer uses `#[path]` anywhere: every directory is a
# real module with its own `mod.rs`. Keep it that way, so module paths keep
# matching the directory layout and `pub(in ...)` keeps meaning something.
check_no_matches 'desktop #[path] debt' '#\[path\s*=' \
  crates/nyaterm-desktop/src
check_no_matches 'terminal-gpui #[path] debt' '#\[path\s*=' \
  crates/nyaterm-terminal-gpui/src

# Parent-module wildcard imports and the shared feature prelude are fully
# cleared. Keep both debts at zero across the complete desktop crate.
check_no_matches 'desktop use super::* debt' '^[[:space:]]*use super::\*;' \
  crates/nyaterm-desktop/src
if [[ -e crates/nyaterm-desktop/src/features/prelude.rs ]]; then
  fail 'features/prelude.rs must not be reintroduced'
fi

# Baseline-friendly checks for areas under active governance. Existing debt is
# allowed, but new files or additional occurrences fail. As files are cleaned,
# lower these counts so the debt cannot return.

declare -A SUPER_BASELINE=(
  [crates/nyaterm-desktop/src/features/terminal/input_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/mod.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/send_command_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/state.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_context_menu_runtime/action_links.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_context_menu_runtime/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_context_menu_runtime/menu.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_context_menu_runtime/mod.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_runtime/buffer.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_runtime/mod.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_runtime/paste.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_runtime/scroll.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_runtime/sessions.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_runtime/view_io.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_search_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_selection_runtime/action_links.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_selection_runtime/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_selection_runtime/metrics.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_selection_runtime/mod.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_selection_runtime/selection.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_selection_runtime/smart_input.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_surface/canvas.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_surface/chrome.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_surface/decorations.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_surface/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_surface/mod.rs]=0
  [crates/nyaterm-desktop/src/features/terminal/terminal_surface_entity.rs]=0
  [crates/nyaterm-desktop/src/features/transfers/mod.rs]=0
  [crates/nyaterm-desktop/src/features/transfers/state.rs]=0
  [crates/nyaterm-desktop/src/features/transfers/transfer_events.rs]=0
  [crates/nyaterm-desktop/src/features/transfers/transfer_options.rs]=0
  [crates/nyaterm-desktop/src/features/transfers/transfer_paths.rs]=0
  [crates/nyaterm-desktop/src/features/transfers/transfer_widgets.rs]=0
  [crates/nyaterm-desktop/src/features/transfers/transfer_jobs/mod.rs]=0
  [crates/nyaterm-desktop/src/features/transfers/transfer_jobs/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/transfers/transfer_jobs/list_cwd.rs]=0
  [crates/nyaterm-desktop/src/features/transfers/transfer_jobs/selection.rs]=0
  [crates/nyaterm-desktop/src/features/transfers/transfer_jobs/transfer.rs]=0
  [crates/nyaterm-desktop/src/http/cloud_sync/mod.rs]=0
  [crates/nyaterm-desktop/src/http/cloud_sync/aliyun.rs]=0
  [crates/nyaterm-desktop/src/http/cloud_sync/github_gist_auth.rs]=0
  [crates/nyaterm-desktop/src/http/cloud_sync/google_drive.rs]=0
  [crates/nyaterm-desktop/src/http/cloud_sync/helpers.rs]=0
  [crates/nyaterm-desktop/src/http/cloud_sync/onedrive.rs]=0
  [crates/nyaterm-desktop/src/http/cloud_sync/s3.rs]=0
  [crates/nyaterm-desktop/src/http/cloud_sync/snippet.rs]=0
  [crates/nyaterm-desktop/src/http/cloud_sync/tests.rs]=0
  [crates/nyaterm-desktop/src/http/cloud_sync/webdav.rs]=0
  [crates/nyaterm-desktop/src/features/remote/mod.rs]=0
  [crates/nyaterm-desktop/src/features/remote/state.rs]=0
  [crates/nyaterm-desktop/src/features/remote/remote_runtime/mod.rs]=0
  [crates/nyaterm-desktop/src/features/remote/remote_runtime/docker.rs]=0
  [crates/nyaterm-desktop/src/features/remote/remote_runtime/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/remote/remote_runtime/process.rs]=0
  [crates/nyaterm-desktop/src/features/remote/remote_runtime/stats.rs]=0
  [crates/nyaterm-desktop/src/features/sync/mod.rs]=0
  [crates/nyaterm-desktop/src/features/sync/cloud_sync_provider.rs]=0
  [crates/nyaterm-desktop/src/features/sync/cloud_sync_runtime/mod.rs]=0
  [crates/nyaterm-desktop/src/features/sync/cloud_sync_runtime/github_gist_auth.rs]=0
  [crates/nyaterm-desktop/src/features/sync/cloud_sync_runtime/jobs.rs]=0
  [crates/nyaterm-desktop/src/features/sync/cloud_sync_runtime/prompts.rs]=0
  [crates/nyaterm-desktop/src/features/sync/cloud_sync_runtime/settings.rs]=0
  [crates/nyaterm-desktop/src/features/translation/mod.rs]=0
  [crates/nyaterm-desktop/src/features/translation/translation_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/connections/connections/mod.rs]=0
  [crates/nyaterm-desktop/src/features/connections/connections/dnd.rs]=0
  [crates/nyaterm-desktop/src/features/connections/connections/menus.rs]=0
  [crates/nyaterm-desktop/src/features/connections/connections/selection.rs]=0
  [crates/nyaterm-desktop/src/features/connections/connection_runtime/mod.rs]=0
  [crates/nyaterm-desktop/src/features/connections/connection_runtime/actions.rs]=0
  [crates/nyaterm-desktop/src/features/connections/connection_runtime/editor.rs]=0
  [crates/nyaterm-desktop/src/features/connections/connection_runtime/groups.rs]=0
  [crates/nyaterm-desktop/src/features/connections/connection_runtime/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/pages/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/connections/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/connections/editor/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/connections/editor/connection/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/connections/editor/connection/local.rs]=0
  [crates/nyaterm-desktop/src/features/pages/connections/editor/connection/serial.rs]=0
  [crates/nyaterm-desktop/src/features/pages/connections/editor/connection/ssh.rs]=0
  [crates/nyaterm-desktop/src/features/pages/connections/editor/connection/telnet.rs]=0
  [crates/nyaterm-desktop/src/features/pages/connections/editor/group_delete.rs]=0
  [crates/nyaterm-desktop/src/features/pages/connections/list.rs]=0
  [crates/nyaterm-desktop/src/features/pages/connections/menus.rs]=0
  [crates/nyaterm-desktop/src/features/pages/connections/view/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/connections/view/page.rs]=0
  [crates/nyaterm-desktop/src/features/pages/connections/view/rows.rs]=0
  [crates/nyaterm-desktop/src/features/pages/tunnels/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/tunnels/common.rs]=0
  [crates/nyaterm-desktop/src/features/pages/tunnels/proxy/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/tunnels/proxy/editor.rs]=0
  [crates/nyaterm-desktop/src/features/pages/tunnels/proxy/rows.rs]=0
  [crates/nyaterm-desktop/src/features/pages/tunnels/proxy/sections.rs]=0
  [crates/nyaterm-desktop/src/features/pages/tunnels/tunnel/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/tunnels/tunnel/editor.rs]=0
  [crates/nyaterm-desktop/src/features/pages/tunnels/tunnel/row.rs]=0
  [crates/nyaterm-desktop/src/features/pages/tunnels/tunnel/sections.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/browser_columns.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/browser_filter.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/browser_keys.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/browser_navigation.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/browser_selection.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/browser/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/browser/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/browser/view.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/entry_row.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/editor/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/editor/input_sync.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/editor/lifecycle.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/editor/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/editor/open.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/file_ops/mkdir_file.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/file_ops/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/file_ops/move_delete.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/file_ops/symlink_rename.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/overlays.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/overlays_context.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/overlays_create.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/overlays_delete_move.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/overlays_editor.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/overlays_favorites.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/overlays_properties.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/overlays_unknown.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/overlays_upload.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/path_bar.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/properties.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/queue.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/process/data.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/process/details.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/process/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/process/resources.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/process/table.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/process_view.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/docker/compose/menus.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/docker/compose/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/docker/compose/project.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/docker/compose/service.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/docker/compose/status.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/docker/containers.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/docker/controls.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/docker/details.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/docker/matchers.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/docker/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/docker/resources.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/docker_view.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/remote/stats_view.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/ai/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/ai/models/credential_rows.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/ai/models/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/ai/models/model_groups.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/ai/rules.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/ai/section.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/security.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/sync_backup/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/sync_backup/cloud_sync/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/sync_backup/cloud_sync/providers.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/translation.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/transfer/advanced.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/transfer/editor.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/transfer/files.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/transfer/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/transfer/recording.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/terminal/general.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/terminal/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/terminal/keywords.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/terminal/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/terminal/search.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/workspace/appearance.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/workspace/general.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/workspace/interaction.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/workspace/keybindings.rs]=0
  [crates/nyaterm-desktop/src/features/pages/settings/workspace/mod.rs]=0
  [crates/nyaterm-desktop/src/features/ai/mod.rs]=0
  [crates/nyaterm-desktop/src/features/ai/ai_agent_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/ai/ai_jobs.rs]=0
  [crates/nyaterm-desktop/src/features/ai/state.rs]=0
  [crates/nyaterm-desktop/src/features/ai/ai_runtime/mod.rs]=0
  [crates/nyaterm-desktop/src/features/ai/ai_runtime/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/ai/ai_runtime/chat/mod.rs]=0
  [crates/nyaterm-desktop/src/features/ai/ai_runtime/chat/discovery.rs]=0
  [crates/nyaterm-desktop/src/features/ai/ai_runtime/chat/history.rs]=0
  [crates/nyaterm-desktop/src/features/ai/ai_runtime/chat/jobs.rs]=0
  [crates/nyaterm-desktop/src/features/ai/ai_runtime/chat/settings_actions.rs]=0
  [crates/nyaterm-desktop/src/features/ai/ai_runtime/settings/mod.rs]=0
  [crates/nyaterm-desktop/src/features/ai/ai_runtime/settings/credentials.rs]=0
  [crates/nyaterm-desktop/src/features/ai/ai_runtime/settings/models.rs]=0
  [crates/nyaterm-desktop/src/features/ai/ai_runtime/settings/profile.rs]=0
  [crates/nyaterm-desktop/src/features/settings/mod.rs]=0
  [crates/nyaterm-desktop/src/features/settings/config_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/settings/lock_diagnostics_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/settings/security_state.rs]=0
  [crates/nyaterm-desktop/src/features/settings/update_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/settings/security_runtime/mod.rs]=0
  [crates/nyaterm-desktop/src/features/settings/security_runtime/credentials.rs]=0
  [crates/nyaterm-desktop/src/features/settings/security_runtime/delete.rs]=0
  [crates/nyaterm-desktop/src/features/settings/security_runtime/keys.rs]=0
  [crates/nyaterm-desktop/src/features/settings/security_runtime/otp.rs]=0
  [crates/nyaterm-desktop/src/features/settings/security_runtime/passwords.rs]=0
  [crates/nyaterm-desktop/src/features/settings/security_runtime/unlock.rs]=0
  [crates/nyaterm-desktop/src/features/settings/settings_runtime/mod.rs]=0
  [crates/nyaterm-desktop/src/features/settings/settings_runtime/draft.rs]=0
  [crates/nyaterm-desktop/src/features/settings/settings_runtime/general_interaction.rs]=0
  [crates/nyaterm-desktop/src/features/settings/settings_runtime/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/settings/settings_runtime/recording_transfer.rs]=0
  [crates/nyaterm-desktop/src/features/settings/settings_runtime/search_engines.rs]=0
  [crates/nyaterm-desktop/src/features/settings/settings_runtime/terminal_remote.rs]=0
  [crates/nyaterm-desktop/src/features/formatting/ai_history.rs]=0
  [crates/nyaterm-desktop/src/features/formatting/labels.rs]=0
  [crates/nyaterm-desktop/src/features/formatting/markdown.rs]=0
  [crates/nyaterm-desktop/src/features/formatting/mod.rs]=0
  [crates/nyaterm-desktop/src/features/icons/aliases.rs]=0
  [crates/nyaterm-desktop/src/features/icons/connection.rs]=0
  [crates/nyaterm-desktop/src/features/icons/file_kind.rs]=0
  [crates/nyaterm-desktop/src/features/icons/mod.rs]=0
  [crates/nyaterm-desktop/src/features/icons/quick.rs]=0
  [crates/nyaterm-desktop/src/features/icons/remote_system.rs]=0
  [crates/nyaterm-desktop/src/features/icons/search.rs]=0
  [crates/nyaterm-desktop/src/features/inspector/mod.rs]=0
  [crates/nyaterm-desktop/src/features/inspector/ai_ask.rs]=0
  [crates/nyaterm-desktop/src/features/inspector/commands.rs]=0
  [crates/nyaterm-desktop/src/features/inspector/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/inspector/right_domain.rs]=0
  [crates/nyaterm-desktop/src/features/inspector/right_shell.rs]=0
  [crates/nyaterm-desktop/src/features/inspector/ai_widgets/mod.rs]=0
  [crates/nyaterm-desktop/src/features/inspector/ai_widgets/agent.rs]=0
  [crates/nyaterm-desktop/src/features/inspector/ai_widgets/cards.rs]=0
  [crates/nyaterm-desktop/src/features/inspector/ai_widgets/history.rs]=0
  [crates/nyaterm-desktop/src/features/inspector/ai_widgets/messages.rs]=0
  [crates/nyaterm-desktop/src/features/inspector/ai_widgets/transcript.rs]=0
  [crates/nyaterm-desktop/src/features/layout/mod.rs]=0
  [crates/nyaterm-desktop/src/features/layout/activity_bar.rs]=0
  [crates/nyaterm-desktop/src/features/layout/prompts.rs]=0
  [crates/nyaterm-desktop/src/features/layout/sidebar/mod.rs]=0
  [crates/nyaterm-desktop/src/features/layout/sidebar/sessions.rs]=0
  [crates/nyaterm-desktop/src/features/layout/sidebar/shell.rs]=0
  [crates/nyaterm-desktop/src/features/layout/security_editors/mod.rs]=0
  [crates/nyaterm-desktop/src/features/layout/security_editors/credential.rs]=0
  [crates/nyaterm-desktop/src/features/layout/security_editors/key.rs]=0
  [crates/nyaterm-desktop/src/features/layout/security_editors/otp.rs]=0
  [crates/nyaterm-desktop/src/features/layout/security_editors/password.rs]=0
  [crates/nyaterm-desktop/src/features/layout/security_panel/mod.rs]=0
  [crates/nyaterm-desktop/src/features/layout/security_panel/chrome.rs]=0
  [crates/nyaterm-desktop/src/features/layout/security_panel/panel/mod.rs]=0
  [crates/nyaterm-desktop/src/features/layout/security_panel/panel/credentials.rs]=0
  [crates/nyaterm-desktop/src/features/layout/security_panel/panel/keys.rs]=0
  [crates/nyaterm-desktop/src/features/layout/security_panel/panel/otp.rs]=0
  [crates/nyaterm-desktop/src/features/layout/security_panel/panel/passwords.rs]=0
  [crates/nyaterm-desktop/src/features/layout/sync_history_panel.rs]=0
  [crates/nyaterm-desktop/src/features/layout/title_bar/mod.rs]=0
  [crates/nyaterm-desktop/src/features/layout/title_bar/bar.rs]=0
  [crates/nyaterm-desktop/src/features/layout/title_bar/menu.rs]=0
  [crates/nyaterm-desktop/src/features/layout/title_menu_helpers.rs]=0
  [crates/nyaterm-desktop/src/features/layout/view_helpers.rs]=0
  [crates/nyaterm-desktop/src/features/layout/workspace/mod.rs]=0
  [crates/nyaterm-desktop/src/features/layout/workspace/bottom.rs]=0
  [crates/nyaterm-desktop/src/features/layout/workspace/surface/mod.rs]=0
  [crates/nyaterm-desktop/src/features/layout/workspace/surface/empty.rs]=0
  [crates/nyaterm-desktop/src/features/layout/workspace/surface/menus.rs]=0
  [crates/nyaterm-desktop/src/features/layout/workspace/surface/tabs.rs]=0
  [crates/nyaterm-desktop/src/features/panels/about_overlay.rs]=0
  [crates/nyaterm-desktop/src/features/panels/active_session_menu_overlay.rs]=0
  [crates/nyaterm-desktop/src/features/panels/connection_import_overlay.rs]=0
  [crates/nyaterm-desktop/src/features/panels/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/panels/lock_screen_overlay.rs]=0
  [crates/nyaterm-desktop/src/features/panels/mod.rs]=0
  [crates/nyaterm-desktop/src/features/panels/multi_line_paste_overlay.rs]=0
  [crates/nyaterm-desktop/src/features/panels/quick_command_category_menu_overlay.rs]=0
  [crates/nyaterm-desktop/src/features/panels/quick_command_category_overlays.rs]=0
  [crates/nyaterm-desktop/src/features/panels/quick_command_delete_overlay.rs]=0
  [crates/nyaterm-desktop/src/features/panels/quick_command_details_overlay.rs]=0
  [crates/nyaterm-desktop/src/features/panels/quick_command_editor_overlay.rs]=0
  [crates/nyaterm-desktop/src/features/panels/quick_command_import_overlay.rs]=0
  [crates/nyaterm-desktop/src/features/panels/quick_command_row_menu_overlay.rs]=0
  [crates/nyaterm-desktop/src/features/panels/quick_command_variable_overlay.rs]=0
  [crates/nyaterm-desktop/src/features/panels/quick_commands_panel/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/panels/quick_commands_panel/mod.rs]=0
  [crates/nyaterm-desktop/src/features/panels/quick_commands_panel/panel/mod.rs]=0
  [crates/nyaterm-desktop/src/features/panels/quick_commands_panel/panel/rows.rs]=0
  [crates/nyaterm-desktop/src/features/panels/quick_commands_panel/panel/sidebar.rs]=0
  [crates/nyaterm-desktop/src/features/panels/quick_switch_overlay.rs]=0
  [crates/nyaterm-desktop/src/features/panels/recording_panel.rs]=0
  [crates/nyaterm-desktop/src/features/panels/send_command_bar/controls.rs]=0
  [crates/nyaterm-desktop/src/features/panels/send_command_bar/editor.rs]=0
  [crates/nyaterm-desktop/src/features/panels/send_command_bar/header.rs]=0
  [crates/nyaterm-desktop/src/features/panels/send_command_bar/mod.rs]=0
  [crates/nyaterm-desktop/src/features/panels/send_command_bar/state.rs]=0
  [crates/nyaterm-desktop/src/features/panels/send_command_helpers.rs]=0
  [crates/nyaterm-desktop/src/features/panels/send_command_state.rs]=0
  [crates/nyaterm-desktop/src/features/panels/session_confirm_overlays.rs]=0
  [crates/nyaterm-desktop/src/features/panels/session_overlays.rs]=0
  [crates/nyaterm-desktop/src/features/panels/sync_groups_overlay.rs]=0
  [crates/nyaterm-desktop/src/features/panels/tab_actions_overlay/compact.rs]=0
  [crates/nyaterm-desktop/src/features/panels/tab_actions_overlay/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/panels/tab_actions_overlay/mod.rs]=0
  [crates/nyaterm-desktop/src/features/panels/tab_actions_overlay/overlay.rs]=0
  [crates/nyaterm-desktop/src/features/panels/temporary_ssh_link_overlay.rs]=0
  [crates/nyaterm-desktop/src/features/panels/terminal_actions_overlay.rs]=0
  [crates/nyaterm-desktop/src/features/panels/update_overlay.rs]=0
  [crates/nyaterm-desktop/src/features/session/auth_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/session/credential_autofill_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/session/mod.rs]=0
  [crates/nyaterm-desktop/src/features/session/prompt_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/session/recording_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/session/session_dialog_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/session/session_lifecycle.rs]=0
  [crates/nyaterm-desktop/src/features/session/session_order.rs]=0
  [crates/nyaterm-desktop/src/features/session/session_runtime/background.rs]=0
  [crates/nyaterm-desktop/src/features/session/session_runtime/mod.rs]=0
  [crates/nyaterm-desktop/src/features/session/session_runtime/start.rs]=0
  [crates/nyaterm-desktop/src/features/session/session_state.rs]=0
  [crates/nyaterm-desktop/src/features/session/startup_restore_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/session/temporary_ssh_link.rs]=0
  [crates/nyaterm-desktop/src/features/session/trzsz_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/session/zmodem_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/commands/mod.rs]=0
  [crates/nyaterm-desktop/src/features/commands/state.rs]=0
  [crates/nyaterm-desktop/src/features/commands/command_runtime/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/commands/command_runtime/history.rs]=0
  [crates/nyaterm-desktop/src/features/commands/command_runtime/mod.rs]=0
  [crates/nyaterm-desktop/src/features/commands/command_runtime/suggestions.rs]=0
  [crates/nyaterm-desktop/src/features/commands/quick_command_runtime/catalog.rs]=0
  [crates/nyaterm-desktop/src/features/commands/quick_command_runtime/dialogs.rs]=0
  [crates/nyaterm-desktop/src/features/commands/quick_command_runtime/editor.rs]=0
  [crates/nyaterm-desktop/src/features/commands/quick_command_runtime/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/commands/quick_command_runtime/mod.rs]=0
  [crates/nyaterm-desktop/src/features/commands/quick_command_runtime/run.rs]=0
  [crates/nyaterm-desktop/src/features/commands/quick_command_runtime/variables.rs]=0
  [crates/nyaterm-desktop/src/features/commands/quick_command_runtime/import/dialog.rs]=0
  [crates/nyaterm-desktop/src/features/commands/quick_command_runtime/import/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/commands/quick_command_runtime/import/json.rs]=0
  [crates/nyaterm-desktop/src/features/commands/quick_command_runtime/import/merge.rs]=0
  [crates/nyaterm-desktop/src/features/commands/quick_command_runtime/import/mod.rs]=0
  [crates/nyaterm-desktop/src/features/commands/quick_command_runtime/import/sources.rs]=0
  [crates/nyaterm-desktop/src/features/commands/quick_command_runtime/import/tests.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/helpers/browser.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/helpers/editor.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/helpers/job_row.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/helpers/mod.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/helpers/paths.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/helpers/properties.rs]=0
  [crates/nyaterm-desktop/src/features/pages/transfers/helpers/queue.rs]=0
  [crates/nyaterm-desktop/src/features/shell/mod.rs]=0
  [crates/nyaterm-desktop/src/features/shell/activity_bar_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/shell/appearance.rs]=0
  [crates/nyaterm-desktop/src/features/shell/global_shortcut_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/shell/navigation_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/shell/panel_stack_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/shell/quick_switch_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/shell/tab_mouse.rs]=0
  [crates/nyaterm-desktop/src/features/shell/tab_windows_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/shell/workspace_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/shell/keybinding_runtime/mod.rs]=0
  [crates/nyaterm-desktop/src/features/shell/keybinding_runtime/keybindings.rs]=0
  [crates/nyaterm-desktop/src/features/shell/keybinding_runtime/keyword_highlights.rs]=0
  [crates/nyaterm-desktop/src/features/shell/panel_resize_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/shell/event_pump/mod.rs]=0
  [crates/nyaterm-desktop/src/features/shell/event_pump/bridge.rs]=0
  [crates/nyaterm-desktop/src/features/shell/event_pump/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/shell/event_pump/planes.rs]=0
  [crates/nyaterm-desktop/src/features/shell/event_pump/publish.rs]=0
  [crates/nyaterm-desktop/src/features/shell/event_pump/session_events.rs]=0
  [crates/nyaterm-desktop/src/features/tunnels/tunnel_runtime.rs]=0
  [crates/nyaterm-desktop/src/features/tunnels/tunnel_runtime/actions.rs]=0
  [crates/nyaterm-desktop/src/features/tunnels/tunnel_runtime/groups.rs]=0
  [crates/nyaterm-desktop/src/features/tunnels/tunnel_runtime/helpers.rs]=0
  [crates/nyaterm-desktop/src/features/tunnels/tunnel_runtime/proxy_editor.rs]=0
  [crates/nyaterm-desktop/src/features/tunnels/tunnel_runtime/tunnel_editor.rs]=0
  [crates/nyaterm-desktop/src/features/view_widgets/chrome.rs]=0
  [crates/nyaterm-desktop/src/features/view_widgets/icons.rs]=0
  [crates/nyaterm-desktop/src/features/view_widgets/inspector_widgets.rs]=0
  [crates/nyaterm-desktop/src/features/view_widgets/markdown.rs]=0
  [crates/nyaterm-desktop/src/features/view_widgets/mod.rs]=0
  [crates/nyaterm-desktop/src/features/view_widgets/rows.rs]=0
  [crates/nyaterm-desktop/src/features/view_widgets/stats.rs]=0
)

for file in "${!SUPER_BASELINE[@]}"; do
  check_max_count 'use super::* debt' '^[[:space:]]*use super::\*;' "$file" "${SUPER_BASELINE[$file]}"
done

while IFS=: read -r file _line _text; do
  if [[ -z "${SUPER_BASELINE[$file]+set}" ]]; then
    fail "new use super::* in governed scope: $file"
  fi
done < <(rg -n --path-separator / '^[[:space:]]*use super::\*;' \
  crates/nyaterm-desktop/src/http/cloud_sync \
  crates/nyaterm-desktop/src/features/ai \
  crates/nyaterm-desktop/src/features/connections \
  crates/nyaterm-desktop/src/features/commands \
  crates/nyaterm-desktop/src/features/formatting \
  crates/nyaterm-desktop/src/features/icons \
  crates/nyaterm-desktop/src/features/inspector \
  crates/nyaterm-desktop/src/features/layout \
  crates/nyaterm-desktop/src/features/panels \
  crates/nyaterm-desktop/src/features/remote \
  crates/nyaterm-desktop/src/features/session \
  crates/nyaterm-desktop/src/features/settings \
  crates/nyaterm-desktop/src/features/sync \
  crates/nyaterm-desktop/src/features/terminal \
  crates/nyaterm-desktop/src/features/translation \
  crates/nyaterm-desktop/src/features/transfers \
  crates/nyaterm-desktop/src/features/view_widgets \
  crates/nyaterm-desktop/src/features/pages/mod.rs \
  crates/nyaterm-desktop/src/features/pages/connections \
  crates/nyaterm-desktop/src/features/pages/tunnels \
  crates/nyaterm-desktop/src/features/pages/transfers \
  crates/nyaterm-desktop/src/features/pages/remote \
  crates/nyaterm-desktop/src/features/pages/settings \
  crates/nyaterm-desktop/src/features/shell/mod.rs \
  crates/nyaterm-desktop/src/features/shell/activity_bar_runtime.rs \
  crates/nyaterm-desktop/src/features/shell/appearance.rs \
  crates/nyaterm-desktop/src/features/shell/panel_stack_runtime.rs \
  crates/nyaterm-desktop/src/features/shell/quick_switch_runtime.rs \
  crates/nyaterm-desktop/src/features/shell/tab_mouse.rs \
  crates/nyaterm-desktop/src/features/shell/tab_windows_runtime.rs \
  crates/nyaterm-desktop/src/features/shell/workspace_runtime.rs \
  crates/nyaterm-desktop/src/features/shell/event_pump \
  crates/nyaterm-desktop/src/features/shell/keybinding_runtime \
  crates/nyaterm-desktop/src/features/tunnels 2>/dev/null || true)

# Layout module entries stay declarative. Leaf modules import their own GPUI,
# model and helper dependencies instead of rebuilding a shared parent scope.
check_no_matches 'layout module entries must not become shared import buckets' \
  '^[[:space:]]*use[[:space:]]' \
  crates/nyaterm-desktop/src/features/layout/mod.rs \
  crates/nyaterm-desktop/src/features/layout/security_editors/mod.rs \
  crates/nyaterm-desktop/src/features/layout/security_panel/mod.rs \
  crates/nyaterm-desktop/src/features/layout/sidebar/mod.rs \
  crates/nyaterm-desktop/src/features/layout/title_bar/mod.rs \
  crates/nyaterm-desktop/src/features/layout/workspace/mod.rs \
  crates/nyaterm-desktop/src/features/layout/workspace/surface/mod.rs

# The terminal context-menu entry is declaration-only. Leaf modules own their
# GPUI and action-link dependencies directly.
check_no_matches 'terminal context-menu entry must not become a shared import bucket' \
  '^[[:space:]]*use[[:space:]]' \
  crates/nyaterm-desktop/src/features/terminal/terminal_context_menu_runtime/mod.rs

# Connection list selection invariants are owned by ConnectionFeatureState's
# private list child. Keep
# governed production paths on semantic methods so clearing selection also
# clears the range anchor and future stale-reference cleanup stays centralized.
check_no_matches \
  "connection list child state must stay behind ConnectionFeatureState methods" \
  'connection_state\.list\.' \
  crates/nyaterm-desktop/src/features
check_no_matches \
  "connection list selection must be mutated through ConnectionFeatureState methods" \
  'connection_state\.list\.(selected_ids|last_selected_id)\s*(=|\.clear\(|\.insert\(|\.remove\()' \
  crates/nyaterm-desktop/src/features/connections
check_no_matches \
  "connection page selection must be mutated through ConnectionFeatureState methods" \
  'connection_state\.list\.(selected_ids|last_selected_id)\s*(=|\.clear\(|\.insert\(|\.remove\()' \
  crates/nyaterm-desktop/src/features/pages/connections

check_no_matches \
  "connection list search/sort/menu state must be mutated through ConnectionFeatureState methods" \
  'connection_state\.list\.(search_draft|sort_mode|more_menu_open)\s*(=|\.clear\(|\.push_str\(|\.pop\()' \
  crates/nyaterm-desktop/src/features/connections
check_no_matches \
  "connection page search/sort/menu state must be mutated through ConnectionFeatureState methods" \
  'connection_state\.list\.(search_draft|sort_mode|more_menu_open)\s*(=|\.clear\(|\.push_str\(|\.pop\()' \
  crates/nyaterm-desktop/src/features/pages/connections
check_no_matches \
  "root connection more menu state must be mutated through ConnectionFeatureState methods" \
  'connection_state\.list\.more_menu_open\s*=' \
  crates/nyaterm-desktop/src/features/root.rs

check_no_matches \
  "connection drag target must be mutated through ConnectionFeatureState methods" \
  'connection_state\.list\.drop_target\s*=' \
  crates/nyaterm-desktop/src/features/connections
check_no_matches \
  "connection page drag target must be mutated through ConnectionFeatureState methods" \
  'connection_state\.list\.drop_target\s*=' \
  crates/nyaterm-desktop/src/features/pages/connections
check_no_matches \
  "connection feature child state fields must remain private" \
  '^[[:space:]]*pub(\([^)]*\))?[[:space:]]+(search_draft|search_focus|sort_mode|more_menu_open|context_menu|group_context_menu|hovered_connection_id|hover_pending|drop_target|hovered_group_id|expanded_group_ids|selected_ids|last_selected_id|import_dialog_open|import_path_prompt|import_focus|draft|window|window_open_pending|focus|icon_picker_open|menu|clear_all_open|delete|group_delete|group_open|group_open_focus|tab|delete_confirm|group_delete_confirm|item_menu|move_picker|expanded_sections|tunnel_editor|proxy_editor|group_editor_focus|tunnel_editor_focus|proxy_editor_focus)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/connections/state.rs
check_no_matches \
  "connection confirmation child state must stay behind ConnectionFeatureState methods" \
  'connection_state\.confirmations\.' \
  crates/nyaterm-desktop/src/features
check_no_matches \
  "connection confirmations must be mutated through ConnectionFeatureState methods" \
  'connection_state\.confirmations\.(clear_all_open|delete|group_delete|group_open)\s*=' \
  crates/nyaterm-desktop/src/features/connections
check_no_matches \
  "connection page confirmations must be mutated through ConnectionFeatureState methods" \
  'connection_state\.confirmations\.(clear_all_open|delete|group_delete|group_open)\s*=' \
  crates/nyaterm-desktop/src/features/pages/connections
check_no_matches \
  "connection menu confirmation reads must use ConnectionFeatureState methods" \
  'connection_state\.confirmations\.(clear_all_open|delete|group_delete|group_open|group_open_focus)(\.|[[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/connections/connections/menus.rs
check_no_matches \
  "connection confirmation panel reads must use ConnectionFeatureState methods" \
  'connection_state\.confirmations\.(clear_all_open|delete|group_delete|group_open|group_open_focus)(\.|[[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/pages/connections/editor/group_delete.rs
check_no_matches \
  "connections page confirmation reads must use ConnectionFeatureState methods" \
  'connection_state\.confirmations\.(clear_all_open|delete|group_delete|group_open|group_open_focus)(\.|[[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/pages/connections/view/page.rs
check_no_matches \
  "event pump confirmation projection must use ConnectionFeatureState methods" \
  'connection_state\.confirmations\.(clear_all_open|delete|group_delete|group_open|group_open_focus)(\.|[[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/shell/event_pump/publish.rs

check_no_matches \
  "connection editor child state must stay behind ConnectionFeatureState methods" \
  'connection_state\.editor\.' \
  crates/nyaterm-desktop/src/features
check_no_matches \
  "connection editor draft mutations must go through ConnectionEditorFeatureState methods" \
  'connection_state\.editor\.draft\.as_mut\(' \
  crates/nyaterm-desktop/src/features/connections/connection_runtime/editor.rs
check_no_matches \
  "connection editor runtime reads must use ConnectionFeatureState methods" \
  'connection_state\.editor\.(draft|menu|icon_picker_open|focus)(\.|[[:space:]]|==|$)' \
  crates/nyaterm-desktop/src/features/connections/connection_runtime/editor.rs
check_no_matches \
  "connection editor view reads must use ConnectionFeatureState methods" \
  'connection_state\.editor\.(draft|menu|icon_picker_open|focus)(\.|[[:space:]]|==|$)' \
  crates/nyaterm-desktop/src/features/pages/connections/editor/connection/mod.rs
check_no_matches \
  "event pump editor projection must use ConnectionFeatureState methods" \
  'connection_state\.editor\.(draft|menu|icon_picker_open|focus)(\.|[[:space:]]|==|$)' \
  crates/nyaterm-desktop/src/features/shell/event_pump/publish.rs

check_no_matches_in_rust_fn \
  "connection editor save success cleanup must stay centralized in ConnectionFeatureState::finish_editor_save" \
  crates/nyaterm-desktop/src/features/connections/connection_runtime/editor.rs \
  save_connection_editor \
  'connection_state[.](editor[.](close[(]|window|window_open_pending|icon_picker_open|menu)|list[.](select_only[(]|expand_group[(]|selected_ids|last_selected_id|expanded_group_ids))'

check_no_matches \
  "connection editor window lifecycle reads must use ConnectionFeatureState methods" \
  'connection_state\.editor\.(draft|window)(\.|[[:space:]]|$)|connection_state\.editor\.window_open_pending([[:space:]]|[=!&|),;}]|$)' \
  crates/nyaterm-desktop/src/features/connection_editor_window.rs
check_no_matches \
  "root connection editor window lifecycle reads must use ConnectionFeatureState methods" \
  'connection_state\.editor\.(draft|window)(\.|[[:space:]]|$)|connection_state\.editor\.window_open_pending([[:space:]]|[=!&|),;}]|$)' \
  crates/nyaterm-desktop/src/features/root.rs

check_no_matches \
  "connection import child state must stay behind ConnectionFeatureState methods" \
  'connection_state\.import\.' \
  crates/nyaterm-desktop/src/features
check_no_matches \
  "connection import runtime must use ConnectionFeatureState methods" \
  'connection_state\.import\.(import_dialog_open|import_path_prompt|import_focus)' \
  crates/nyaterm-desktop/src/features/connections/connection_import_runtime.rs
check_no_matches \
  "connection import overlay must use ConnectionFeatureState methods" \
  'connection_state\.import\.(import_dialog_open|import_path_prompt|import_focus)' \
  crates/nyaterm-desktop/src/features/panels/connection_import_overlay.rs
check_no_matches \
  "root connection import overlay state must use ConnectionFeatureState methods" \
  'connection_state\.import\.(import_dialog_open|import_path_prompt|import_focus)' \
  crates/nyaterm-desktop/src/features/root.rs

check_no_matches \
  "connection group editor draft mutations must go through ConnectionGroupEditorFeatureState methods" \
  'connection_state\.group_editor\.draft\.as_mut\(' \
  crates/nyaterm-desktop/src/features/connections
check_no_matches \
  "connection group editor child state must stay behind ConnectionFeatureState methods" \
  'connection_state\.group_editor\.' \
  crates/nyaterm-desktop/src/features

check_no_matches \
  "connection group runtime must use ConnectionFeatureState methods" \
  'connection_state\.group_editor\.(draft|focus)(\.|[[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/connections/connection_runtime/groups.rs
check_no_matches \
  "connection group editor panel must use ConnectionFeatureState methods" \
  'connection_state\.group_editor\.(draft|focus)(\.|[[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/pages/connections/editor/group_delete.rs
check_no_matches \
  "connections page group editor state must use ConnectionFeatureState methods" \
  'connection_state\.group_editor\.(draft|focus)(\.|[[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/pages/connections/view/page.rs
check_no_matches \
  "event pump group editor state must use ConnectionFeatureState methods" \
  'connection_state\.group_editor\.(draft|focus)(\.|[[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/shell/event_pump/publish.rs

check_no_matches \
  "connection expanded groups must be mutated through ConnectionFeatureState methods" \
  'connection_state\.list\.expanded_group_ids\.insert\(' \
  crates/nyaterm-desktop/src/features/connections

check_no_matches \
  "connection hover state must be mutated through ConnectionFeatureState methods" \
  'connection_state\.list\.(hovered_connection_id|hover_pending|hovered_group_id)\s*(=|\.take\()' \
  crates/nyaterm-desktop/src/features/pages/connections

check_no_matches \
  "connection runtime list reads must use ConnectionFeatureState methods" \
  'connection_state\.list\.(search_draft|sort_mode|selected_ids|last_selected_id|context_menu|group_context_menu|hover_pending|hovered_connection_id|hovered_group_id|drop_target|expanded_group_ids|more_menu_open)(\.|[[:space:]]|==|,|\)|$)' \
  crates/nyaterm-desktop/src/features/connections/connections/selection.rs
check_no_matches \
  "connection selection runtime derived queries must stay on ConnectionFeatureState" \
  '(natural_compare|append_visible_connection_ids|children_by_parent|by_group)' \
  crates/nyaterm-desktop/src/features/connections/connections/selection.rs
check_no_matches \
  "connection selection runtime must not reintroduce NyaTermApp query wrappers" \
  'fn (selected_connections|visible_connection_ids)\(' \
  crates/nyaterm-desktop/src/features/connections/connections/selection.rs
check_no_matches \
  "connection group tree queries must stay on ConnectionFeatureState" \
  'fn saved_connections_in_group_tree|group_ids = std::collections::HashSet::from' \
  crates/nyaterm-desktop/src/features/connections/connections/menus.rs
check_no_matches \
  "connection page group tree queries must stay on ConnectionFeatureState" \
  'group_ids = std::collections::HashSet::from' \
  crates/nyaterm-desktop/src/features/pages/connections/menus.rs
check_no_matches \
  "connections page list reads must use ConnectionFeatureState methods" \
  'connection_state\.list\.(search_draft|sort_mode|selected_ids|last_selected_id|context_menu|group_context_menu|search_focus|hover_pending|hovered_connection_id|hovered_group_id|drop_target|expanded_group_ids|more_menu_open)(\.|[[:space:]]|==|,|\)|$)' \
  crates/nyaterm-desktop/src/features/pages/connections
check_no_matches \
  "root connection menu reads must use ConnectionFeatureState methods" \
  'connection_state\.list\.more_menu_open([[:space:]]|[=!&|),;}]|$)' \
  crates/nyaterm-desktop/src/features/root.rs
check_no_matches \
  "event pump quiet-tick list reads must use ConnectionFeatureState methods" \
  'connection_state\.list\.(search_draft|sort_mode|hover_pending)(\.|[[:space:]]|==|,|\)|$)' \
  crates/nyaterm-desktop/src/features/shell/event_pump/mod.rs
check_no_matches \
  "event pump list projection reads must use ConnectionFeatureState methods" \
  'connection_state\.list\.(search_draft|sort_mode|hover_pending)(\.|[[:space:]]|==|,|\)|$)' \
  crates/nyaterm-desktop/src/features/shell/event_pump/publish.rs
check_no_matches \
  "connection import list reads must use ConnectionFeatureState methods" \
  'connection_state\.list\.(expanded_group_ids|sort_mode)(\.|[[:space:]]|==|,|\)|$)' \
  crates/nyaterm-desktop/src/features/connections/connection_import_runtime.rs
check_no_matches \
  "panel resize list projection reads must use ConnectionFeatureState methods" \
  'connection_state\.list\.(expanded_group_ids|sort_mode)(\.|[[:space:]]|==|,|\)|$)' \
  crates/nyaterm-desktop/src/features/shell/panel_resize_runtime.rs

check_no_matches \
  "network child state must stay behind ConnectionFeatureState methods" \
  'connection_state\.network\.' \
  crates/nyaterm-desktop/src/features
check_no_matches \
  "network editor drafts must be mutated through ConnectionFeatureState methods" \
  'connection_state\.network\.(group_editor|tunnel_editor|proxy_editor)\.as_mut\(' \
  crates/nyaterm-desktop/src/features/tunnels
check_no_matches \
  "network page state reads must use ConnectionFeatureState methods" \
  'connection_state\.network\.(tab|delete_confirm|group_editor|group_delete_confirm|item_menu|move_picker|expanded_sections|tunnel_editor|proxy_editor|group_editor_focus|tunnel_editor_focus|proxy_editor_focus)(\.|[[:space:]]|==|,|\)|$)' \
  crates/nyaterm-desktop/src/features/pages/tunnels
check_no_matches \
  "network runtime state reads must use ConnectionFeatureState methods" \
  'connection_state\.network\.(tab|delete_confirm|group_editor|group_delete_confirm|item_menu|move_picker|expanded_sections|tunnel_editor|proxy_editor|group_editor_focus|tunnel_editor_focus|proxy_editor_focus)(\.|[[:space:]]|==|,|\)|$)' \
  crates/nyaterm-desktop/src/features/tunnels
check_no_matches \
  "panel stack network projection must use ConnectionFeatureState methods" \
  'connection_state\.network\.(tab|delete_confirm|group_editor|group_delete_confirm|item_menu|move_picker|expanded_sections|tunnel_editor|proxy_editor|group_editor_focus|tunnel_editor_focus|proxy_editor_focus)(\.|[[:space:]]|==|,|\)|$)' \
  crates/nyaterm-desktop/src/features/shell/panel_stack_runtime.rs

# Obvious secret-bearing Debug derives are forbidden. This is intentionally
# conservative; if a secret-bearing type really needs Debug, implement a custom
# redacted formatter and add a narrow exception here with a comment.
# NOTE: the name match below is deliberately case-sensitive. `IGNORECASE` is a
# gawk extension, so relying on it made this check behave differently depending
# on which awk was installed. Keeping it case-sensitive keeps the result
# identical everywhere. It also means the heuristic is currently very weak and
# still needs a real triage pass over secret-bearing types.
if awk '
  /#\[derive\(/ {
    derive = $0
    file = FILENAME
    line = FNR
    next
  }
  derive && /struct .*(secret|password|credential|otp|token|key)/ {
    if (derive ~ /Debug/) {
      printf "%s:%d: %s -> %s\n", file, line, derive, $0
    }
  }
  $0 !~ /^#\[derive\(/ && $0 !~ /^[[:space:]]*$/ {
    derive = ""
  }
' crates/nyaterm-core/src/*.rs crates/nyaterm-desktop/src/models/*.rs crates/nyaterm-transport/src/*.rs \
  >/tmp/nyaterm-secret-debug.$$; then
  if [[ -s /tmp/nyaterm-secret-debug.$$ ]]; then
    fail "secret-bearing structs must not derive Debug"
    sed 's/^/  /' /tmp/nyaterm-secret-debug.$$ >&2
  fi
fi
rm -f /tmp/nyaterm-secret-debug.$$

if (( failures > 0 )); then
  printf 'architecture-boundary: %d violation(s)\n' "$failures" >&2
  exit 1
fi

printf 'architecture-boundary: ok\n'

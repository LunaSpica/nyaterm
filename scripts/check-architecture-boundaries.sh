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

# These low-frequency transport helpers have explicit imports at their call
# sites. Keep them out of the shared feature prelude so new modules do not
# acquire unrelated transport dependencies implicitly.
check_no_matches \
  "low-frequency transport helpers must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(SFTP_TRANSFER_CANCELLED|SftpTransferDirection|RemoteStatsService|open_ssh_multiplex_handle|run_local_command)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

check_no_matches \
  "low-frequency core helpers must stay out of features/prelude.rs" \
  '(^|[,{[:space:]])(AgentApprovalDecision|AgentCapturedOutput|AiChatStreamDelta|AiContext|AiModelDiscovery|AiSession|AppendAiAuditRequest|CLOUD_SYNC_HISTORY_LIMIT|CloudSyncResult|DiagnosticsExportOptions|DiagnosticsRuntimeSnapshot|GiteeSnippetHttpBackend|GithubGistHttpBackend|KeywordHighlightRule|LocalCloudSyncOptions|SearchEngineConfig|SnippetRemote|TerminalMouseReportEligibility|TerminalResizeGeometry|TerminalViewportInsets|TranslateResult|TranslationSettings|agent_response_action|ai_model_id_for_credential|ai_model_id_for_provider|append_cloud_sync_history|assess_agent_command_risk|build_agent_capture_command|build_observation_message|decide_agent_command_execution|merge_model_discoveries|parse_agent_model_output|parse_agent_tool_call|parse_model_output|pull_local_snapshot|pull_snapshot_with_remote|push_local_snapshot|push_snapshot_with_remote|read_cloud_sync_history|redact_context|redact_sensitive_text|terminal_mouse_report_should_send|terminal_resize_geometry_for_size_with_insets|terminal_resize_geometry_for_size_with_insets_and_scale|terminal_snapped_cell_height)([},[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/prelude.rs

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

# Keep migration-only local source paths contained. Default builds may carry an
# inert inventory path, but new runtime dependencies on the local legacy tree
# should not spread beyond the existing gated migration code.
while IFS=: read -r file _line text; do
  if [[ "$text" =~ ^[[:space:]]*// ]]; then
    continue
  fi
  case "$file" in
    crates/nyaterm-desktop/src/features/mod.rs) ;;
    crates/nyaterm-desktop/src/features/app_state/construct.rs) ;;
    scripts/sync-icons.sh) ;;
    scripts/icons.manifest) ;;
    docs/architecture/gpui-migration-status.md) ;;
    scripts/check-architecture-boundaries.sh) ;;
    *)
      fail "local legacy source path appears outside the migration allowlist: $file"
      ;;
  esac
done < <(rg -n --path-separator / './temp/nyaterm-tauri|nyaterm-tauri' crates docs scripts 2>/dev/null || true)

# The desktop crate no longer uses `#[path]` anywhere: every directory is a
# real module with its own `mod.rs`. Keep it that way, so module paths keep
# matching the directory layout and `pub(in ...)` keeps meaning something.
check_no_matches 'desktop #[path] debt' '#\[path\s*=' \
  crates/nyaterm-desktop/src
check_no_matches 'terminal-gpui #[path] debt' '#\[path\s*=' \
  crates/nyaterm-terminal-gpui/src

# Baseline-friendly checks for areas under active governance. Existing debt is
# allowed, but new files or additional occurrences fail. As files are cleaned,
# lower these counts so the debt cannot return.

declare -A SUPER_BASELINE=(
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
  [crates/nyaterm-desktop/src/features/pages/transfers/helpers/editor.rs]=0
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
)

for file in "${!SUPER_BASELINE[@]}"; do
  check_max_count 'use super::* debt' '^[[:space:]]*use super::\*;' "$file" "${SUPER_BASELINE[$file]}"
done

while IFS=: read -r file _line _text; do
  if [[ -z "${SUPER_BASELINE[$file]+set}" ]]; then
    fail "new use super::* in governed scope: $file"
  fi
done < <(rg -n --path-separator / '^[[:space:]]*use super::\*;' \
  crates/nyaterm-desktop/src/features/connections \
  crates/nyaterm-desktop/src/features/pages/mod.rs \
  crates/nyaterm-desktop/src/features/pages/connections \
  crates/nyaterm-desktop/src/features/pages/tunnels \
  crates/nyaterm-desktop/src/features/pages/transfers/helpers/editor.rs \
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

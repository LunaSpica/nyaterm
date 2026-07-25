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
  '(^|[,{[:space:]])(AgentApprovalDecision|AgentCapturedOutput|AiChatStreamDelta|CloudSyncResult|DiagnosticsExportOptions|DiagnosticsRuntimeSnapshot|KeywordHighlightRule|SearchEngineConfig|TerminalMouseReportEligibility|TerminalResizeGeometry|TerminalViewportInsets|terminal_mouse_report_should_send|terminal_resize_geometry_for_size_with_insets|terminal_resize_geometry_for_size_with_insets_and_scale|terminal_snapped_cell_height)([},[:space:]]|$)' \
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
    docs/architecture/gpui-migration-status.md) ;;
    scripts/check-architecture-boundaries.sh) ;;
    *)
      fail "local legacy source path appears outside the migration allowlist: $file"
      ;;
  esac
done < <(rg -n './temp/nyaterm-tauri|nyaterm-tauri' crates docs scripts 2>/dev/null || true)

# Baseline-friendly checks for areas under active governance. Existing debt is
# allowed, but new files or additional occurrences fail. As files are cleaned,
# lower these counts so the debt cannot return.
check_no_matches 'connections feature #[path] debt' '#\[path\s*=' \
  crates/nyaterm-desktop/src/features/connections
check_no_matches 'connections page #[path] debt' '#\[path\s*=' \
  crates/nyaterm-desktop/src/features/pages/connections
check_max_count 'network page #[path] debt' '#\[path\s*=' \
  crates/nyaterm-desktop/src/features/pages/tunnels/mod.rs 0
check_max_count 'network proxy page #[path] debt' '#\[path\s*=' \
  crates/nyaterm-desktop/src/features/pages/tunnels/proxy/mod.rs 0
check_max_count 'network tunnel page #[path] debt' '#\[path\s*=' \
  crates/nyaterm-desktop/src/features/pages/tunnels/tunnel/mod.rs 0
check_max_count 'event pump #[path] debt' '#\[path\s*=' \
  crates/nyaterm-desktop/src/features/shell/event_pump.rs 5
check_max_count 'tunnel runtime #[path] debt' '#\[path\s*=' \
  crates/nyaterm-desktop/src/features/tunnels/tunnel_runtime.rs 0

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
  [crates/nyaterm-desktop/src/features/shell/event_pump.rs]=1
  [crates/nyaterm-desktop/src/features/shell/event_pump/bridge.rs]=1
  [crates/nyaterm-desktop/src/features/shell/event_pump/helpers.rs]=2
  [crates/nyaterm-desktop/src/features/shell/event_pump/planes.rs]=1
  [crates/nyaterm-desktop/src/features/shell/event_pump/publish.rs]=1
  [crates/nyaterm-desktop/src/features/shell/event_pump/session_events.rs]=1
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
done < <(rg -n '^[[:space:]]*use super::\*;' \
  crates/nyaterm-desktop/src/features/connections \
  crates/nyaterm-desktop/src/features/pages/mod.rs \
  crates/nyaterm-desktop/src/features/pages/connections.rs \
  crates/nyaterm-desktop/src/features/pages/connections \
  crates/nyaterm-desktop/src/features/pages/tunnels \
  crates/nyaterm-desktop/src/features/shell/event_pump.rs \
  crates/nyaterm-desktop/src/features/shell/event_pump \
  crates/nyaterm-desktop/src/features/tunnels 2>/dev/null || true)

# Connection list selection invariants are owned by ConnectionListState. Keep
# governed production paths on semantic methods so clearing selection also
# clears the range anchor and future stale-reference cleanup stays centralized.
check_no_matches \
  "connection list selection must be mutated through ConnectionListState methods" \
  'connection_state\.list\.(selected_ids|last_selected_id)\s*(=|\.clear\(|\.insert\(|\.remove\()' \
  crates/nyaterm-desktop/src/features/connections
check_no_matches \
  "connection page selection must be mutated through ConnectionListState methods" \
  'connection_state\.list\.(selected_ids|last_selected_id)\s*(=|\.clear\(|\.insert\(|\.remove\()' \
  crates/nyaterm-desktop/src/features/pages/connections

check_no_matches \
  "connection list search/sort/menu state must be mutated through ConnectionListState methods" \
  'connection_state\.list\.(search_draft|sort_mode|more_menu_open)\s*(=|\.clear\(|\.push_str\(|\.pop\()' \
  crates/nyaterm-desktop/src/features/connections
check_no_matches \
  "connection page search/sort/menu state must be mutated through ConnectionListState methods" \
  'connection_state\.list\.(search_draft|sort_mode|more_menu_open)\s*(=|\.clear\(|\.push_str\(|\.pop\()' \
  crates/nyaterm-desktop/src/features/pages/connections
check_no_matches \
  "root connection more menu state must be mutated through ConnectionListState methods" \
  'connection_state\.list\.more_menu_open\s*=' \
  crates/nyaterm-desktop/src/features/root.rs

check_no_matches \
  "connection drag target must be mutated through ConnectionListState methods" \
  'connection_state\.list\.drop_target\s*=' \
  crates/nyaterm-desktop/src/features/connections
check_no_matches \
  "connection page drag target must be mutated through ConnectionListState methods" \
  'connection_state\.list\.drop_target\s*=' \
  crates/nyaterm-desktop/src/features/pages/connections
check_no_matches \
  "connection feature child state fields must remain private" \
  '^[[:space:]]*pub(\([^)]*\))?[[:space:]]+(search_draft|search_focus|sort_mode|more_menu_open|context_menu|group_context_menu|hovered_connection_id|hover_pending|drop_target|hovered_group_id|expanded_group_ids|selected_ids|last_selected_id|import_dialog_open|import_path_prompt|import_focus|draft|window|window_open_pending|focus|icon_picker_open|menu|clear_all_open|delete|group_delete|group_open|group_open_focus|tab|delete_confirm|group_delete_confirm|item_menu|move_picker|expanded_sections|tunnel_editor|proxy_editor|group_editor_focus|tunnel_editor_focus|proxy_editor_focus)[[:space:]]*:' \
  crates/nyaterm-desktop/src/features/connections/state.rs
check_no_matches \
  "connection confirmations must be mutated through ConnectionConfirmationState methods" \
  'connection_state\.confirmations\.(clear_all_open|delete|group_delete|group_open)\s*=' \
  crates/nyaterm-desktop/src/features/connections
check_no_matches \
  "connection page confirmations must be mutated through ConnectionConfirmationState methods" \
  'connection_state\.confirmations\.(clear_all_open|delete|group_delete|group_open)\s*=' \
  crates/nyaterm-desktop/src/features/pages/connections
check_no_matches \
  "connection menu confirmation reads must use ConnectionConfirmationState methods" \
  'connection_state\.confirmations\.(clear_all_open|delete|group_delete|group_open|group_open_focus)(\.|[[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/connections/connections/menus.rs
check_no_matches \
  "connection confirmation panel reads must use ConnectionConfirmationState methods" \
  'connection_state\.confirmations\.(clear_all_open|delete|group_delete|group_open|group_open_focus)(\.|[[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/pages/connections/editor/group_delete.rs
check_no_matches \
  "connections page confirmation reads must use ConnectionConfirmationState methods" \
  'connection_state\.confirmations\.(clear_all_open|delete|group_delete|group_open|group_open_focus)(\.|[[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/pages/connections/view/page.rs
check_no_matches \
  "event pump confirmation projection must use ConnectionConfirmationState methods" \
  'connection_state\.confirmations\.(clear_all_open|delete|group_delete|group_open|group_open_focus)(\.|[[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/shell/event_pump/publish.rs

check_no_matches \
  "connection editor draft mutations must go through ConnectionEditorFeatureState methods" \
  'connection_state\.editor\.draft\.as_mut\(' \
  crates/nyaterm-desktop/src/features/connections/connection_runtime/editor.rs
check_no_matches \
  "connection editor runtime reads must use ConnectionEditorFeatureState methods" \
  'connection_state\.editor\.(draft|menu|icon_picker_open|focus)(\.|[[:space:]]|==|$)' \
  crates/nyaterm-desktop/src/features/connections/connection_runtime/editor.rs
check_no_matches \
  "connection editor view reads must use ConnectionEditorFeatureState methods" \
  'connection_state\.editor\.(draft|menu|icon_picker_open|focus)(\.|[[:space:]]|==|$)' \
  crates/nyaterm-desktop/src/features/pages/connections/editor/connection/mod.rs
check_no_matches \
  "event pump editor projection must use ConnectionEditorFeatureState methods" \
  'connection_state\.editor\.(draft|menu|icon_picker_open|focus)(\.|[[:space:]]|==|$)' \
  crates/nyaterm-desktop/src/features/shell/event_pump/publish.rs

check_no_matches_in_rust_fn \
  "connection editor save success cleanup must stay centralized in ConnectionFeatureState::finish_editor_save" \
  crates/nyaterm-desktop/src/features/connections/connection_runtime/editor.rs \
  save_connection_editor \
  'connection_state\.(editor\.(close\(|window|window_open_pending|icon_picker_open|menu)|list\.(select_only\(|expand_group\(|selected_ids|last_selected_id|expanded_group_ids))'

check_no_matches \
  "connection editor window lifecycle reads must use ConnectionEditorFeatureState methods" \
  'connection_state\.editor\.(draft|window)(\.|[[:space:]]|$)|connection_state\.editor\.window_open_pending([[:space:]]|[=!&|),;}]|$)' \
  crates/nyaterm-desktop/src/features/connection_editor_window.rs
check_no_matches \
  "root connection editor window lifecycle reads must use ConnectionEditorFeatureState methods" \
  'connection_state\.editor\.(draft|window)(\.|[[:space:]]|$)|connection_state\.editor\.window_open_pending([[:space:]]|[=!&|),;}]|$)' \
  crates/nyaterm-desktop/src/features/root.rs

check_no_matches \
  "connection import runtime must use ConnectionImportState methods" \
  'connection_state\.import\.(import_dialog_open|import_path_prompt|import_focus)' \
  crates/nyaterm-desktop/src/features/connections/connection_import_runtime.rs
check_no_matches \
  "connection import overlay must use ConnectionImportState methods" \
  'connection_state\.import\.(import_dialog_open|import_path_prompt|import_focus)' \
  crates/nyaterm-desktop/src/features/panels/connection_import_overlay.rs
check_no_matches \
  "root connection import overlay state must use ConnectionImportState methods" \
  'connection_state\.import\.(import_dialog_open|import_path_prompt|import_focus)' \
  crates/nyaterm-desktop/src/features/root.rs

check_no_matches \
  "connection group editor draft mutations must go through ConnectionGroupEditorFeatureState methods" \
  'connection_state\.group_editor\.draft\.as_mut\(' \
  crates/nyaterm-desktop/src/features/connections

check_no_matches \
  "connection group runtime must use ConnectionGroupEditorFeatureState methods" \
  'connection_state\.group_editor\.(draft|focus)(\.|[[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/connections/connection_runtime/groups.rs
check_no_matches \
  "connection group editor panel must use ConnectionGroupEditorFeatureState methods" \
  'connection_state\.group_editor\.(draft|focus)(\.|[[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/pages/connections/editor/group_delete.rs
check_no_matches \
  "connections page group editor state must use ConnectionGroupEditorFeatureState methods" \
  'connection_state\.group_editor\.(draft|focus)(\.|[[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/pages/connections/view/page.rs
check_no_matches \
  "event pump group editor state must use ConnectionGroupEditorFeatureState methods" \
  'connection_state\.group_editor\.(draft|focus)(\.|[[:space:]]|$)' \
  crates/nyaterm-desktop/src/features/shell/event_pump/publish.rs

check_no_matches \
  "connection expanded groups must be mutated through ConnectionListState methods" \
  'connection_state\.list\.expanded_group_ids\.insert\(' \
  crates/nyaterm-desktop/src/features/connections

check_no_matches \
  "connection hover state must be mutated through ConnectionListState methods" \
  'connection_state\.list\.(hovered_connection_id|hover_pending|hovered_group_id)\s*(=|\.take\()' \
  crates/nyaterm-desktop/src/features/pages/connections

check_no_matches \
  "connection runtime list reads must use ConnectionListState methods" \
  'connection_state\.list\.(search_draft|sort_mode|selected_ids|last_selected_id|context_menu|group_context_menu|hover_pending|hovered_connection_id|hovered_group_id|drop_target|expanded_group_ids|more_menu_open)(\.|[[:space:]]|==|,|\)|$)' \
  crates/nyaterm-desktop/src/features/connections/connections/selection.rs
check_no_matches \
  "connections page list reads must use ConnectionListState methods" \
  'connection_state\.list\.(search_draft|sort_mode|selected_ids|last_selected_id|context_menu|group_context_menu|search_focus|hover_pending|hovered_connection_id|hovered_group_id|drop_target|expanded_group_ids|more_menu_open)(\.|[[:space:]]|==|,|\)|$)' \
  crates/nyaterm-desktop/src/features/pages/connections
check_no_matches \
  "root connection menu reads must use ConnectionListState methods" \
  'connection_state\.list\.more_menu_open([[:space:]]|[=!&|),;}]|$)' \
  crates/nyaterm-desktop/src/features/root.rs
check_no_matches \
  "event pump quiet-tick list reads must use ConnectionListState methods" \
  'connection_state\.list\.(search_draft|sort_mode|hover_pending)(\.|[[:space:]]|==|,|\)|$)' \
  crates/nyaterm-desktop/src/features/shell/event_pump.rs
check_no_matches \
  "event pump list projection reads must use ConnectionListState methods" \
  'connection_state\.list\.(search_draft|sort_mode|hover_pending)(\.|[[:space:]]|==|,|\)|$)' \
  crates/nyaterm-desktop/src/features/shell/event_pump/publish.rs
check_no_matches \
  "connection import list reads must use ConnectionListState methods" \
  'connection_state\.list\.(expanded_group_ids|sort_mode)(\.|[[:space:]]|==|,|\)|$)' \
  crates/nyaterm-desktop/src/features/connections/connection_import_runtime.rs
check_no_matches \
  "panel resize list projection reads must use ConnectionListState methods" \
  'connection_state\.list\.(expanded_group_ids|sort_mode)(\.|[[:space:]]|==|,|\)|$)' \
  crates/nyaterm-desktop/src/features/shell/panel_resize_runtime.rs

check_no_matches \
  "network editor drafts must be mutated through NetworkFeatureState methods" \
  'connection_state\.network\.(group_editor|tunnel_editor|proxy_editor)\.as_mut\(' \
  crates/nyaterm-desktop/src/features/tunnels
check_no_matches \
  "network page state reads must use NetworkFeatureState methods" \
  'connection_state\.network\.(tab|delete_confirm|group_editor|group_delete_confirm|item_menu|move_picker|expanded_sections|tunnel_editor|proxy_editor|group_editor_focus|tunnel_editor_focus|proxy_editor_focus)(\.|[[:space:]]|==|,|\)|$)' \
  crates/nyaterm-desktop/src/features/pages/tunnels
check_no_matches \
  "network runtime state reads must use NetworkFeatureState methods" \
  'connection_state\.network\.(tab|delete_confirm|group_editor|group_delete_confirm|item_menu|move_picker|expanded_sections|tunnel_editor|proxy_editor|group_editor_focus|tunnel_editor_focus|proxy_editor_focus)(\.|[[:space:]]|==|,|\)|$)' \
  crates/nyaterm-desktop/src/features/tunnels
check_no_matches \
  "panel stack network projection must use NetworkFeatureState methods" \
  'connection_state\.network\.(tab|delete_confirm|group_editor|group_delete_confirm|item_menu|move_picker|expanded_sections|tunnel_editor|proxy_editor|group_editor_focus|tunnel_editor_focus|proxy_editor_focus)(\.|[[:space:]]|==|,|\)|$)' \
  crates/nyaterm-desktop/src/features/shell/panel_stack_runtime.rs

# Obvious secret-bearing Debug derives are forbidden. This is intentionally
# conservative; if a secret-bearing type really needs Debug, implement a custom
# redacted formatter and add a narrow exception here with a comment.
if awk '
  BEGIN { IGNORECASE = 1 }
  /#\[derive\(/ {
    derive = $0
    file = FILENAME
    line = FNR
    next
  }
  derive && /struct .*?(secret|password|credential|otp|token|key)/ {
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

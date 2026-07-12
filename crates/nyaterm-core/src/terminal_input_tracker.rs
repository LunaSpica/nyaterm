//! Local terminal line input tracker (Tauri `terminalInputTracker.ts` parity).

use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TerminalInputState {
    pub value: String,
    /// Byte cursor into `value`.
    pub cursor: usize,
    pub desynced: bool,
    pub desync_reason: Option<&'static str>,
    pub line_rewrite_required: bool,
    pub multiline: bool,
    pub paste_mode: bool,
}

impl TerminalInputState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(multiline: bool) -> Self {
        Self {
            multiline,
            ..Self::default()
        }
    }
}

fn leading_env_prefix() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\(([^()\r\n]+)\)\s*").expect("env prefix"))
}

fn bracket_prompt_prefix() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\[[^\]\r\n]+\]\s*[#$]\s*").expect("bracket prompt"))
}

fn posix_prompt_prefix() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[^\s@]+@[^:\s]+:[^#$\r\n]*[#$]\s*").expect("posix prompt"))
}

fn powershell_prompt_prefix() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^PS\s+[^>\r\n]+>\s*").expect("powershell prompt"))
}

fn windows_prompt_prefix() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[A-Za-z]:(?:[\\/][^>\r\n]*)?>\s*").expect("windows prompt"))
}

fn strip_leading_env_prefixes(input: &str) -> String {
    let mut remaining = input.to_string();
    loop {
        if let Some(m) = leading_env_prefix().find(&remaining) {
            remaining = remaining[m.end()..].to_string();
        } else {
            return remaining;
        }
    }
}

fn strip_known_prompt_prefix(input: &str) -> String {
    for matcher in [
        bracket_prompt_prefix(),
        posix_prompt_prefix(),
        powershell_prompt_prefix(),
        windows_prompt_prefix(),
    ] {
        if let Some(m) = matcher.find(input) {
            return input[m.end()..].to_string();
        }
    }
    input.to_string()
}

/// Remove known shell prompt prefixes while preserving command spacing.
pub fn strip_terminal_command_prompt(input: &str) -> String {
    let without_leading = input.trim_start();
    if without_leading.trim().is_empty() {
        return String::new();
    }
    strip_known_prompt_prefix(&strip_leading_env_prefixes(without_leading))
}

/// Remove known shell prompt prefixes so command parsing stays stable across shells.
pub fn sanitize_terminal_command(input: &str) -> String {
    strip_terminal_command_prompt(input).trim().to_string()
}

fn clamp_cursor(value: &str, cursor: usize) -> usize {
    cursor.min(value.len())
}

fn insert_text(state: &TerminalInputState, text: &str) -> TerminalInputState {
    if text.is_empty() {
        return state.clone();
    }
    let cursor = clamp_cursor(&state.value, state.cursor);
    let mut value = String::with_capacity(state.value.len() + text.len());
    value.push_str(&state.value[..cursor]);
    value.push_str(text);
    value.push_str(&state.value[cursor..]);
    let next_cursor = cursor + text.len();
    TerminalInputState {
        value: value.clone(),
        cursor: next_cursor,
        desynced: state.desynced,
        desync_reason: state.desync_reason,
        line_rewrite_required: state.line_rewrite_required,
        multiline: value.contains('\n') || value.contains('\r'),
        paste_mode: state.paste_mode,
    }
}

fn delete_left(state: &TerminalInputState) -> TerminalInputState {
    let cursor = clamp_cursor(&state.value, state.cursor);
    if cursor == 0 {
        return state.clone();
    }
    // Delete one char before cursor.
    let prev = state.value[..cursor]
        .char_indices()
        .last()
        .map(|(i, _)| i)
        .unwrap_or(0);
    let mut value = String::with_capacity(state.value.len());
    value.push_str(&state.value[..prev]);
    value.push_str(&state.value[cursor..]);
    TerminalInputState {
        value: value.clone(),
        cursor: prev,
        desynced: state.desynced,
        desync_reason: state.desync_reason,
        line_rewrite_required: state.line_rewrite_required,
        multiline: value.contains('\n'),
        paste_mode: state.paste_mode,
    }
}

fn delete_right(state: &TerminalInputState) -> TerminalInputState {
    let cursor = clamp_cursor(&state.value, state.cursor);
    if cursor >= state.value.len() {
        return state.clone();
    }
    let next = state.value[cursor..]
        .char_indices()
        .nth(1)
        .map(|(i, _)| cursor + i)
        .unwrap_or(state.value.len());
    let mut value = String::with_capacity(state.value.len());
    value.push_str(&state.value[..cursor]);
    value.push_str(&state.value[next..]);
    TerminalInputState {
        value: value.clone(),
        cursor,
        desynced: state.desynced,
        desync_reason: state.desync_reason,
        line_rewrite_required: state.line_rewrite_required,
        multiline: value.contains('\n'),
        paste_mode: state.paste_mode,
    }
}

fn delete_previous_word(state: &TerminalInputState) -> TerminalInputState {
    let cursor = clamp_cursor(&state.value, state.cursor);
    if cursor == 0 {
        return state.clone();
    }
    let chars: Vec<(usize, char)> = state.value[..cursor].char_indices().collect();
    let mut start_idx = chars.len();
    while start_idx > 0 && chars[start_idx - 1].1.is_whitespace() {
        start_idx -= 1;
    }
    while start_idx > 0 && !chars[start_idx - 1].1.is_whitespace() {
        start_idx -= 1;
    }
    let start = if start_idx == 0 {
        0
    } else {
        chars[start_idx].0
    };
    let mut value = String::with_capacity(state.value.len());
    value.push_str(&state.value[..start]);
    value.push_str(&state.value[cursor..]);
    TerminalInputState {
        value: value.clone(),
        cursor: start,
        desynced: state.desynced,
        desync_reason: state.desync_reason,
        line_rewrite_required: state.line_rewrite_required,
        multiline: value.contains('\n'),
        paste_mode: state.paste_mode,
    }
}

fn mark_desynced(
    state: &TerminalInputState,
    reason: &'static str,
    multiline: bool,
) -> TerminalInputState {
    TerminalInputState {
        value: state.value.clone(),
        cursor: state.cursor,
        desynced: true,
        desync_reason: Some(reason),
        line_rewrite_required: state.line_rewrite_required || reason == "tab",
        multiline,
        paste_mode: state.paste_mode,
    }
}

fn apply_pasted_input_data(state: &TerminalInputState, data: &str) -> TerminalInputState {
    let mut text = data.to_string();
    let mut paste_mode = state.paste_mode;
    if text.contains("\u{1b}[200~") {
        paste_mode = true;
        text = text.replace("\u{1b}[200~", "");
    }
    if text.contains("\u{1b}[201~") {
        paste_mode = false;
        text = text.replace("\u{1b}[201~", "");
    }
    text = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut next = state.clone();
    next.paste_mode = paste_mode;
    if text.is_empty() {
        return next;
    }
    if next.desynced && next.desync_reason == Some("tab") {
        next.desynced = false;
        next.desync_reason = None;
        next.line_rewrite_required = true;
    }
    insert_text(&next, &text)
}

/// Apply a raw terminal input chunk to the local tracker.
pub fn apply_terminal_input_data(state: &TerminalInputState, data: &str) -> TerminalInputState {
    if data.is_empty() {
        return state.clone();
    }

    match data {
        "\r" | "\u{0003}" => return TerminalInputState::reset(false),
        "\u{0001}" => {
            return TerminalInputState {
                cursor: 0,
                ..state.clone()
            };
        }
        "\u{0005}" => {
            return TerminalInputState {
                cursor: state.value.len(),
                ..state.clone()
            };
        }
        "\u{0015}" => {
            let cursor = clamp_cursor(&state.value, state.cursor);
            let value = state.value[cursor..].to_string();
            return TerminalInputState {
                value: value.clone(),
                cursor: 0,
                multiline: value.contains('\n'),
                desynced: state.desynced,
                desync_reason: state.desync_reason,
                line_rewrite_required: state.line_rewrite_required,
                paste_mode: state.paste_mode,
            };
        }
        "\u{0017}" => return delete_previous_word(state),
        "\u{000b}" => {
            let cursor = clamp_cursor(&state.value, state.cursor);
            let value = state.value[..cursor].to_string();
            return TerminalInputState {
                value: value.clone(),
                cursor,
                multiline: value.contains('\n'),
                desynced: state.desynced,
                desync_reason: state.desync_reason,
                line_rewrite_required: state.line_rewrite_required,
                paste_mode: state.paste_mode,
            };
        }
        "\u{000c}" => return state.clone(),
        "\u{007f}" | "\u{0008}" => return delete_left(state),
        "\u{1b}[D" | "\u{1b}OD" => {
            let cursor = clamp_cursor(&state.value, state.cursor);
            let prev = if cursor == 0 {
                0
            } else {
                state.value[..cursor]
                    .char_indices()
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            };
            return TerminalInputState {
                cursor: prev,
                ..state.clone()
            };
        }
        "\u{1b}[C" | "\u{1b}OC" => {
            let cursor = clamp_cursor(&state.value, state.cursor);
            let next = if cursor >= state.value.len() {
                state.value.len()
            } else {
                state.value[cursor..]
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| cursor + i)
                    .unwrap_or(state.value.len())
            };
            return TerminalInputState {
                cursor: next,
                ..state.clone()
            };
        }
        "\u{1b}[H" | "\u{1b}OH" => {
            return TerminalInputState {
                cursor: 0,
                ..state.clone()
            };
        }
        "\u{1b}[F" | "\u{1b}OF" => {
            return TerminalInputState {
                cursor: state.value.len(),
                ..state.clone()
            };
        }
        "\u{1b}[3~" => return delete_right(state),
        "\t" => return mark_desynced(state, "tab", state.multiline),
        _ => {}
    }

    if state.paste_mode || data.contains("\u{1b}[200~") || data.contains("\u{1b}[201~") {
        return apply_pasted_input_data(state, data);
    }

    if (data.contains('\n') || data.contains('\r')) && data != "\r" {
        return apply_pasted_input_data(state, data);
    }

    if data.starts_with('\u{1b}') {
        return mark_desynced(state, "terminal", state.multiline);
    }

    if data.chars().any(|ch| ch.is_control()) {
        return mark_desynced(state, "terminal", state.multiline);
    }

    if state.desynced && state.desync_reason == Some("tab") {
        let mut cleared = state.clone();
        cleared.desynced = false;
        cleared.desync_reason = None;
        cleared.line_rewrite_required = true;
        return insert_text(&cleared, data);
    }

    insert_text(state, data)
}

pub fn get_tracked_command(state: &TerminalInputState) -> String {
    if state.desynced || state.multiline {
        return String::new();
    }
    sanitize_terminal_command(&state.value)
}

pub fn can_suggest_from_tracker(state: &TerminalInputState) -> bool {
    !state.desynced
        && !state.multiline
        && state.cursor == state.value.len()
        && !get_tracked_command(state).is_empty()
}

pub fn can_register_command_from_tracker(state: &TerminalInputState) -> bool {
    !state.desynced && !state.multiline && !state.line_rewrite_required
}

pub fn get_tracked_submission_command(state: &TerminalInputState) -> String {
    if !can_register_command_from_tracker(state) {
        return String::new();
    }
    sanitize_terminal_command(&state.value)
}

fn normalize_line_content(value: &str) -> String {
    value
        .replace("\r\n", "")
        .replace('\n', "")
        .replace('\r', "")
}

fn choose_terminal_line_command(previous_value: &str, line_content: &str) -> Option<String> {
    let previous_command = sanitize_terminal_command(previous_value);
    let sanitized_line = sanitize_terminal_command(line_content);
    let mut candidates: Vec<(String, u32)> = Vec::new();

    let mut push_candidate = |raw: &str| {
        let normalized = strip_terminal_command_prompt(&normalize_line_content(raw));
        let command = sanitize_terminal_command(&normalized);
        if command.is_empty() {
            return;
        }
        let score = if !previous_command.is_empty() && command.starts_with(&previous_command) {
            command.len() as u32
        } else if previous_command.is_empty() {
            command.len() as u32
        } else {
            0
        };
        if !previous_command.is_empty() && score == 0 {
            return;
        }
        candidates.push((command, score));
    };

    push_candidate(&sanitized_line);
    push_candidate(line_content);

    // Suffix candidates: line after previous prefix
    for prefix in [
        &previous_value.to_string(),
        &previous_command,
        &sanitized_line,
    ] {
        let source = normalize_line_content(line_content);
        let source_cmd = sanitize_terminal_command(&source);
        let prefix_cmd = sanitize_terminal_command(prefix);
        if !prefix_cmd.is_empty() {
            if let Some(pos) = source_cmd.find(&prefix_cmd) {
                let suffix = &source_cmd[pos..];
                push_candidate(suffix);
            }
        }
    }

    candidates.sort_by(|a, b| b.1.cmp(&a.1).then(b.0.len().cmp(&a.0.len())));
    candidates.into_iter().map(|(cmd, _)| cmd).next()
}

/// Inclusive-exclusive byte range within `TerminalInputState::value`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputSelectionRange {
    pub start: usize,
    pub end: usize,
}

impl InputSelectionRange {
    pub fn new(start: usize, end: usize) -> Option<Self> {
        if end > start {
            Some(Self { start, end })
        } else {
            None
        }
    }

    pub fn len_chars(&self, value: &str) -> usize {
        let end = self.end.min(value.len());
        let start = self.start.min(end);
        value[start..end].chars().count()
    }
}

/// Delete a byte range from the tracked input line (Tauri `deleteTerminalInputRange`).
pub fn delete_terminal_input_range(
    state: &TerminalInputState,
    start: usize,
    end: usize,
) -> TerminalInputState {
    let length = state.value.len();
    let from = start.min(length);
    let to = end.min(length).max(from);
    if to <= from {
        return state.clone();
    }
    let mut value = String::with_capacity(length - (to - from));
    value.push_str(&state.value[..from]);
    value.push_str(&state.value[to..]);
    TerminalInputState {
        value: value.clone(),
        cursor: from,
        desynced: state.desynced,
        desync_reason: state.desync_reason,
        line_rewrite_required: state.line_rewrite_required,
        multiline: value.contains('\n') || value.contains('\r'),
        paste_mode: state.paste_mode,
    }
}

/// Build CSI left/right moves between two byte cursors (character steps).
pub fn build_move_input_cursor_data(
    value: &str,
    current_cursor: usize,
    target_cursor: usize,
) -> String {
    let current = current_cursor.min(value.len());
    let target = target_cursor.min(value.len());
    let current_chars = value[..current].chars().count();
    let target_chars = value[..target].chars().count();
    if target_chars > current_chars {
        "\u{1b}[C".repeat(target_chars - current_chars)
    } else if current_chars > target_chars {
        "\u{1b}[D".repeat(current_chars - target_chars)
    } else {
        String::new()
    }
}

/// Map a character index into `value` to a byte offset.
pub fn char_index_to_byte(value: &str, char_index: usize) -> usize {
    value
        .char_indices()
        .nth(char_index)
        .map(|(i, _)| i)
        .unwrap_or(value.len())
}

/// Map a byte offset into `value` to a character index.
pub fn byte_index_to_char(value: &str, byte_index: usize) -> usize {
    value[..byte_index.min(value.len())].chars().count()
}

/// Recover tracker value from the terminal buffer line after tab completion desync.
pub fn resync_from_terminal_line(
    current: &TerminalInputState,
    line_content: &str,
) -> Option<TerminalInputState> {
    let value = choose_terminal_line_command(&current.value, line_content)?;
    Some(TerminalInputState {
        value: value.clone(),
        cursor: value.len(),
        desynced: false,
        desync_reason: None,
        line_rewrite_required: false,
        multiline: false,
        paste_mode: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_insert_backspace_and_submit() {
        let mut state = TerminalInputState::new();
        state = apply_terminal_input_data(&state, "gi");
        state = apply_terminal_input_data(&state, "t");
        assert_eq!(get_tracked_command(&state), "git");
        state = apply_terminal_input_data(&state, "\u{007f}");
        assert_eq!(get_tracked_command(&state), "gi");
        state = apply_terminal_input_data(&state, "\r");
        assert_eq!(get_tracked_command(&state), "");
        assert!(!state.desynced);
    }

    #[test]
    fn tab_desync_hides_suggestions() {
        let mut state = TerminalInputState::new();
        state = apply_terminal_input_data(&state, "doc");
        state = apply_terminal_input_data(&state, "\t");
        assert!(state.desynced);
        assert!(!can_suggest_from_tracker(&state));
    }

    #[test]
    fn strips_shell_prompt_prefixes() {
        assert_eq!(sanitize_terminal_command("user@host:~$ ls -la"), "ls -la");
        assert_eq!(sanitize_terminal_command("PS C:\\Users> dir"), "dir");
    }

    #[test]
    fn resyncs_after_tab_desync_from_terminal_line() {
        let mut state = TerminalInputState::new();
        state = apply_terminal_input_data(&state, "doc");
        state = apply_terminal_input_data(&state, "\t");
        assert!(state.desynced);
        let recovered =
            resync_from_terminal_line(&state, "user@host:~$ docker compose ps").expect("recover");
        assert_eq!(get_tracked_command(&recovered), "docker compose ps");
        assert!(!recovered.desynced);
    }

    #[test]
    fn submission_requires_synced_state() {
        let mut state = TerminalInputState::new();
        state = apply_terminal_input_data(&state, "ls");
        assert_eq!(get_tracked_submission_command(&state), "ls");
        state = apply_terminal_input_data(&state, "\t");
        assert!(get_tracked_submission_command(&state).is_empty());
    }

    #[test]
    fn deletes_input_range() {
        let mut state = TerminalInputState::new();
        state = apply_terminal_input_data(&state, "abcdef");
        state.cursor = 4;
        let next = delete_terminal_input_range(&state, 1, 4);
        assert_eq!(next.value, "aef");
        assert_eq!(next.cursor, 1);
    }

    #[test]
    fn builds_move_cursor_sequences_by_char() {
        let value = "abc";
        assert_eq!(
            build_move_input_cursor_data(value, 0, 3),
            "\u{1b}[C\u{1b}[C\u{1b}[C"
        );
        assert_eq!(
            build_move_input_cursor_data(value, 3, 1),
            "\u{1b}[D\u{1b}[D"
        );
        assert_eq!(build_move_input_cursor_data(value, 2, 2), "");
    }
}

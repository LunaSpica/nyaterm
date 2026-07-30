use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fmt::Write as FmtWrite;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::mem;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};
use std::time::Instant;
use thiserror::Error;
use time::OffsetDateTime;

pub const DEFAULT_MEMORY_LIMIT_BYTES: usize = 5 * 1024 * 1024;
pub const DEFAULT_HISTORY_SEARCH_LINES: usize = 30_000;
pub const MAX_HISTORY_SEARCH_LINES: usize = 100_000;
pub const DEFAULT_HISTORY_SEARCH_LIMIT: usize = 100;

#[derive(Debug, Error)]
pub enum RecordingError {
    #[error("{0}")]
    Config(String),
    #[error("invalid regular expression: {0}")]
    Regex(#[from] regex::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug)]
struct TranscriptRecord {
    line_id: u64,
    timestamp: String,
    label: &'static str,
    data: String,
    size_bytes: usize,
}

impl TranscriptRecord {
    fn new(line_id: u64, label: &'static str, data: String) -> Self {
        let timestamp = chrono_timestamp();
        let size_bytes = format_record_parts(&timestamp, label, &data, true, true).len();
        Self {
            line_id,
            timestamp,
            label,
            data,
            size_bytes,
        }
    }

    fn format(&self, include_io_labels: bool, include_timestamps: bool) -> String {
        format_record_parts(
            &self.timestamp,
            self.label,
            &self.data,
            include_io_labels,
            include_timestamps,
        )
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalHistorySearchRequest {
    pub session_id: String,
    pub query: String,
    #[serde(default)]
    pub case_sensitive: bool,
    #[serde(default)]
    pub regex: bool,
    #[serde(default)]
    pub whole_word: bool,
    pub limit: Option<usize>,
    pub context_before: Option<usize>,
    pub context_after: Option<usize>,
    pub max_lines: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalHistorySearchResponse {
    pub total: usize,
    pub elapsed_ms: u128,
    pub truncated: bool,
    pub results: Vec<TerminalHistorySearchResult>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalHistorySearchResult {
    pub line_id: u64,
    pub line_number: usize,
    pub column_start: usize,
    pub column_end: usize,
    pub preview: String,
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub source: String,
}

struct FileRecording {
    writer: BufWriter<File>,
    file_path: PathBuf,
    include_io_labels: bool,
    include_timestamps: bool,
}

impl FileRecording {
    fn new(
        file: File,
        file_path: PathBuf,
        include_io_labels: bool,
        include_timestamps: bool,
    ) -> Self {
        Self {
            writer: BufWriter::new(file),
            file_path,
            include_io_labels,
            include_timestamps,
        }
    }

    fn write_record(&mut self, record: &TranscriptRecord) {
        let _ = self.writer.write_all(
            record
                .format(self.include_io_labels, self.include_timestamps)
                .as_bytes(),
        );
    }

    fn finish(&mut self) {
        let _ = self.writer.flush();
    }
}

struct SessionCaptureState {
    recording: Option<FileRecording>,
    records: VecDeque<TranscriptRecord>,
    record_bytes: usize,
    memory_limit_bytes: usize,
    pending_input_escape: Option<TerminalInputEscapeState>,
    input_buffer: String,
    output_buffer: String,
    live_echo_buffer: String,
    submitted_line_echo: Option<String>,
    suppress_next_newline: bool,
    next_line_id: u64,
}

impl SessionCaptureState {
    fn new(memory_limit_bytes: usize) -> Self {
        Self {
            recording: None,
            records: VecDeque::new(),
            record_bytes: 0,
            memory_limit_bytes,
            pending_input_escape: None,
            input_buffer: String::new(),
            output_buffer: String::new(),
            live_echo_buffer: String::new(),
            submitted_line_echo: None,
            suppress_next_newline: false,
            next_line_id: 1,
        }
    }

    fn set_memory_limit(&mut self, memory_limit_bytes: usize) {
        self.memory_limit_bytes = memory_limit_bytes.max(1);
        self.trim_records();
    }

    fn start_recording(
        &mut self,
        file: File,
        file_path: PathBuf,
        include_io_labels: bool,
        include_timestamps: bool,
    ) -> Result<(), RecordingError> {
        if self.recording.is_some() {
            return Err(RecordingError::Config(
                "recording is already active".to_string(),
            ));
        }
        self.flush_output_lines(true);
        self.recording = Some(FileRecording::new(
            file,
            file_path,
            include_io_labels,
            include_timestamps,
        ));
        Ok(())
    }

    fn stop_recording(&mut self) -> Result<String, RecordingError> {
        if self.recording.is_none() {
            return Err(RecordingError::Config("no active recording".to_string()));
        }
        self.commit_partial_input();
        self.flush_output_lines(true);
        let mut recording = self
            .recording
            .take()
            .ok_or_else(|| RecordingError::Config("no active recording".to_string()))?;
        recording.finish();
        Ok(recording.file_path.to_string_lossy().to_string())
    }

    fn write_input(&mut self, data: &[u8]) {
        let mut index = 0;

        while index < data.len() {
            if let Some(state) = self.pending_input_escape.take() {
                let (next_index, next_state) =
                    consume_terminal_input_escape_state(data, index, state);
                self.pending_input_escape = next_state;
                index = next_index;
                continue;
            }

            match data[index] {
                b'\r' | b'\n' => {
                    self.commit_input_line();
                    index += 1;
                }
                b'\x08' | b'\x7f' => {
                    self.handle_backspace();
                    index += 1;
                }
                b'\t' => {
                    self.input_buffer.push('\t');
                    self.live_echo_buffer.push('\t');
                    index += 1;
                }
                b'\x1b' => {
                    let (next_index, next_state) = consume_terminal_input_escape_state(
                        data,
                        index + 1,
                        TerminalInputEscapeState::Esc,
                    );
                    self.pending_input_escape = next_state;
                    index = next_index;
                }
                byte if byte.is_ascii_control() => {
                    index += 1;
                }
                byte if byte.is_ascii() => {
                    let ch = byte as char;
                    self.input_buffer.push(ch);
                    self.live_echo_buffer.push(ch);
                    index += 1;
                }
                _ => match next_utf8_char(data, index) {
                    Some((ch, next_index)) if !ch.is_control() => {
                        self.input_buffer.push(ch);
                        self.live_echo_buffer.push(ch);
                        index = next_index;
                    }
                    Some((_ch, next_index)) => {
                        index = next_index;
                    }
                    None => {
                        index += 1;
                    }
                },
            }
        }
    }

    fn write_raw_input(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }

        self.commit_partial_input();
        self.flush_output_lines(true);
        self.append_record("RAW_INPUT", format_raw_input_bytes(data));
    }

    fn write_output(&mut self, data: &str) {
        let mut sanitized = strip_terminal_control_sequences(data);
        if sanitized.is_empty() {
            return;
        }

        if self.suppress_next_newline {
            sanitized = strip_one_leading_newline(&sanitized).to_string();
            self.suppress_next_newline = false;
            if sanitized.is_empty() {
                return;
            }
        }

        sanitized = self.consume_live_echo(&sanitized);
        if sanitized.is_empty() {
            return;
        }

        let (mut sanitized, consumed_submitted_echo) = self.consume_submitted_echo(&sanitized);
        if sanitized.is_empty() {
            return;
        }

        if !consumed_submitted_echo && self.submitted_line_echo.is_some() {
            sanitized = strip_one_leading_newline(&sanitized).to_string();
            self.submitted_line_echo = None;
            if sanitized.is_empty() {
                return;
            }
        }

        self.output_buffer.push_str(&sanitized);
        self.flush_output_lines(false);
    }

    fn finish(&mut self) {
        self.commit_partial_input();
        self.flush_output_lines(true);
        if let Some(recording) = self.recording.as_mut() {
            recording.finish();
        }
        self.recording = None;
    }

    fn snapshot_records(&mut self) -> Vec<TranscriptRecord> {
        self.flush_output_lines(true);
        self.records.iter().cloned().collect()
    }

    fn append_record(&mut self, label: &'static str, data: String) {
        if data.is_empty() {
            return;
        }

        let line_id = self.next_line_id;
        self.next_line_id = self.next_line_id.saturating_add(1);
        let record = TranscriptRecord::new(line_id, label, data);
        if let Some(recording) = self.recording.as_mut() {
            recording.write_record(&record);
        }

        self.record_bytes += record.size_bytes;
        self.records.push_back(record);
        self.trim_records();
    }

    fn trim_records(&mut self) {
        while self.records.len() > 1 && self.record_bytes > self.memory_limit_bytes {
            if let Some(record) = self.records.pop_front() {
                self.record_bytes = self.record_bytes.saturating_sub(record.size_bytes);
            }
        }
    }

    fn handle_backspace(&mut self) {
        if let Some(removed) = self.input_buffer.pop()
            && self.live_echo_buffer.ends_with(removed)
        {
            self.live_echo_buffer.pop();
        }
    }

    fn commit_input_line(&mut self) {
        self.flush_output_lines(true);
        let line = mem::take(&mut self.input_buffer);
        self.live_echo_buffer.clear();

        if line.trim().is_empty() {
            self.submitted_line_echo = None;
            return;
        }

        self.append_record("INPUT", line.clone());
        self.submitted_line_echo = Some(line);
    }

    fn commit_partial_input(&mut self) {
        self.flush_output_lines(true);
        let line = mem::take(&mut self.input_buffer);
        self.live_echo_buffer.clear();
        self.submitted_line_echo = None;

        if line.trim().is_empty() {
            return;
        }

        self.append_record("INPUT", line);
    }

    fn consume_live_echo(&mut self, text: &str) -> String {
        let consumed = consume_matching_prefix(&mut self.live_echo_buffer, text);
        text[consumed..].to_string()
    }

    fn consume_submitted_echo(&mut self, text: &str) -> (String, bool) {
        let Some(line) = self.submitted_line_echo.as_ref() else {
            return (text.to_string(), false);
        };

        if !text.starts_with(line) {
            return (text.to_string(), false);
        }

        let mut remaining = text[line.len()..].to_string();
        self.submitted_line_echo = None;

        let stripped = strip_one_leading_newline(&remaining);
        if stripped.len() != remaining.len() {
            remaining = stripped.to_string();
        } else {
            self.suppress_next_newline = true;
        }

        (remaining, true)
    }

    fn flush_output_lines(&mut self, flush_partial: bool) {
        while let Some(pos) = self.output_buffer.find('\n') {
            let line = self.output_buffer[..pos].to_string();
            self.output_buffer.drain(..=pos);
            self.append_record("OUTPUT", line);
        }

        if flush_partial && !self.output_buffer.is_empty() {
            let tail = mem::take(&mut self.output_buffer);
            self.append_record("OUTPUT", tail);
        }
    }
}

pub struct RecordingManager {
    sessions: Mutex<HashMap<String, SessionCaptureState>>,
    memory_limit_bytes: Mutex<usize>,
}

impl RecordingManager {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            memory_limit_bytes: Mutex::new(DEFAULT_MEMORY_LIMIT_BYTES),
        }
    }

    pub fn start(
        &self,
        session_id: &str,
        file_path: &str,
        include_io_labels: bool,
        include_timestamps: bool,
    ) -> Result<(), RecordingError> {
        let path = prepare_output_file_path(file_path)?;
        let file = File::create(&path)?;
        let memory_limit_bytes = *lock_recover(&self.memory_limit_bytes);

        let mut sessions = lock_recover(&self.sessions);
        let state = sessions
            .entry(session_id.to_string())
            .or_insert_with(|| SessionCaptureState::new(memory_limit_bytes));
        state.set_memory_limit(memory_limit_bytes);
        state.start_recording(file, path, include_io_labels, include_timestamps)
    }

    pub fn stop(&self, session_id: &str) -> Result<String, RecordingError> {
        let mut sessions = lock_recover(&self.sessions);
        let state = sessions
            .get_mut(session_id)
            .ok_or_else(|| RecordingError::Config("no active recording".to_string()))?;
        state.stop_recording()
    }

    pub fn save_transcript(
        &self,
        session_id: &str,
        file_path: &str,
        include_io_labels: bool,
        include_timestamps: bool,
    ) -> Result<String, RecordingError> {
        let path = prepare_output_file_path(file_path)?;
        let records = {
            let mut sessions = lock_recover(&self.sessions);
            sessions
                .get_mut(session_id)
                .map(SessionCaptureState::snapshot_records)
                .unwrap_or_default()
        };

        let mut writer = BufWriter::new(File::create(&path)?);
        for record in &records {
            writer.write_all(
                record
                    .format(include_io_labels, include_timestamps)
                    .as_bytes(),
            )?;
        }
        writer.flush()?;
        Ok(path.to_string_lossy().to_string())
    }

    pub fn search_history(
        &self,
        request: TerminalHistorySearchRequest,
    ) -> Result<TerminalHistorySearchResponse, RecordingError> {
        let started = Instant::now();
        let query = request.query;
        if query.is_empty() {
            return Ok(TerminalHistorySearchResponse {
                total: 0,
                elapsed_ms: started.elapsed().as_millis(),
                truncated: false,
                results: Vec::new(),
            });
        }

        let limit = request.limit.unwrap_or(DEFAULT_HISTORY_SEARCH_LIMIT).max(1);
        let context_before = request.context_before.unwrap_or(0).min(20);
        let context_after = request.context_after.unwrap_or(0).min(20);
        let max_lines = request
            .max_lines
            .unwrap_or(DEFAULT_HISTORY_SEARCH_LINES)
            .clamp(1, MAX_HISTORY_SEARCH_LINES);
        let records = {
            let mut sessions = lock_recover(&self.sessions);
            sessions
                .get_mut(&request.session_id)
                .map(SessionCaptureState::snapshot_records)
                .unwrap_or_default()
        };
        let start_index = records.len().saturating_sub(max_lines);
        let searched_records = &records[start_index..];
        let matcher = HistoryMatcher::new(
            &query,
            request.case_sensitive,
            request.regex,
            request.whole_word,
        )?;
        let mut total = 0usize;
        let mut results = Vec::new();

        for (relative_index, record) in searched_records.iter().enumerate() {
            if let Some((column_start, column_end)) = matcher.find(&record.data) {
                total += 1;
                if results.len() < limit {
                    let absolute_index = start_index + relative_index;
                    results.push(TerminalHistorySearchResult {
                        line_id: record.line_id,
                        line_number: absolute_index + 1,
                        column_start,
                        column_end,
                        preview: record.data.clone(),
                        before: context_records(&records, absolute_index, context_before, true),
                        after: context_records(&records, absolute_index, context_after, false),
                        source: record.label.to_ascii_lowercase(),
                    });
                }
            }
        }

        Ok(TerminalHistorySearchResponse {
            total,
            elapsed_ms: started.elapsed().as_millis(),
            truncated: total > results.len() || records.len() > max_lines,
            results,
        })
    }

    pub fn set_memory_limit(&self, max_bytes: usize) {
        let bounded = max_bytes.max(1);
        *lock_recover(&self.memory_limit_bytes) = bounded;

        let mut sessions = lock_recover(&self.sessions);
        for state in sessions.values_mut() {
            state.set_memory_limit(bounded);
        }
    }

    pub fn is_recording(&self, session_id: &str) -> bool {
        self.sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(session_id)
            .is_some_and(|state| state.recording.is_some())
    }

    pub fn list_recording_sessions(&self) -> Vec<String> {
        self.sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .filter_map(|(id, state)| state.recording.as_ref().map(|_| id.clone()))
            .collect()
    }

    pub fn write_output(&self, session_id: &str, data: &str) {
        let memory_limit_bytes = *lock_recover(&self.memory_limit_bytes);
        let mut sessions = lock_recover(&self.sessions);
        let state = sessions
            .entry(session_id.to_string())
            .or_insert_with(|| SessionCaptureState::new(memory_limit_bytes));
        state.set_memory_limit(memory_limit_bytes);
        state.write_output(data);
    }

    pub fn write_input(&self, session_id: &str, data: &[u8]) {
        let memory_limit_bytes = *lock_recover(&self.memory_limit_bytes);
        let mut sessions = lock_recover(&self.sessions);
        let state = sessions
            .entry(session_id.to_string())
            .or_insert_with(|| SessionCaptureState::new(memory_limit_bytes));
        state.set_memory_limit(memory_limit_bytes);
        state.write_input(data);
    }

    pub fn write_raw_input(&self, session_id: &str, data: &[u8]) {
        let memory_limit_bytes = *lock_recover(&self.memory_limit_bytes);
        let mut sessions = lock_recover(&self.sessions);
        let state = sessions
            .entry(session_id.to_string())
            .or_insert_with(|| SessionCaptureState::new(memory_limit_bytes));
        state.set_memory_limit(memory_limit_bytes);
        state.write_raw_input(data);
    }

    pub fn cleanup_session(&self, session_id: &str) {
        let removed = {
            let mut sessions = lock_recover(&self.sessions);
            sessions.remove(session_id)
        };
        if let Some(mut state) = removed {
            state.finish();
        }
    }
}

impl Default for RecordingManager {
    fn default() -> Self {
        Self::new()
    }
}

pub fn safe_recording_name(name: &str) -> String {
    let mut safe = String::new();
    let mut last_was_separator = false;

    for ch in name.chars() {
        if ch.is_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            safe.push(ch);
            last_was_separator = false;
        } else if !last_was_separator {
            safe.push('_');
            last_was_separator = true;
        }
    }

    let safe = safe.trim_matches('_');
    if safe.is_empty() {
        "session".to_string()
    } else {
        safe.to_string()
    }
}

fn prepare_output_file_path(file_path: &str) -> Result<PathBuf, RecordingError> {
    let path = PathBuf::from(file_path);
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    Ok(path)
}

fn format_record_parts(
    timestamp: &str,
    label: &str,
    data: &str,
    include_io_labels: bool,
    include_timestamps: bool,
) -> String {
    match (include_timestamps, include_io_labels) {
        (true, true) => format!("[{timestamp}] [{label}] {data}\n"),
        (true, false) => format!("[{timestamp}] {data}\n"),
        (false, true) => format!("[{label}] {data}\n"),
        (false, false) => format!("{data}\n"),
    }
}

fn chrono_timestamp() -> String {
    let now = OffsetDateTime::now_utc();
    now.format(time::macros::format_description!(
        "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"
    ))
    .unwrap_or_else(|_| "1970-01-01 00:00:00.000".to_string())
}

fn format_raw_input_bytes(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().saturating_mul(3).saturating_sub(1));
    for (index, byte) in data.iter().enumerate() {
        if index > 0 {
            out.push(' ');
        }
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TerminalInputEscapeState {
    Esc,
    Csi,
    Ss3,
    String,
    StringEsc,
}

fn consume_terminal_input_escape_state(
    data: &[u8],
    mut index: usize,
    mut state: TerminalInputEscapeState,
) -> (usize, Option<TerminalInputEscapeState>) {
    loop {
        match state {
            TerminalInputEscapeState::Esc => {
                let Some(byte) = data.get(index).copied() else {
                    return (index, Some(TerminalInputEscapeState::Esc));
                };
                index += 1;
                match byte {
                    b'[' => state = TerminalInputEscapeState::Csi,
                    b'O' => state = TerminalInputEscapeState::Ss3,
                    b']' | b'P' | b'X' | b'^' | b'_' => state = TerminalInputEscapeState::String,
                    _ => return (index, None),
                }
            }
            TerminalInputEscapeState::Csi => {
                while let Some(byte) = data.get(index).copied() {
                    index += 1;
                    if (0x40..=0x7e).contains(&byte) {
                        return (index, None);
                    }
                }
                return (index, Some(TerminalInputEscapeState::Csi));
            }
            TerminalInputEscapeState::Ss3 => {
                if index < data.len() {
                    return (index + 1, None);
                }
                return (index, Some(TerminalInputEscapeState::Ss3));
            }
            TerminalInputEscapeState::String => {
                while let Some(byte) = data.get(index).copied() {
                    index += 1;
                    if byte == b'\x07' {
                        return (index, None);
                    }
                    if byte == b'\x1b' {
                        state = TerminalInputEscapeState::StringEsc;
                        break;
                    }
                }
                if index >= data.len() && state == TerminalInputEscapeState::String {
                    return (index, Some(TerminalInputEscapeState::String));
                }
            }
            TerminalInputEscapeState::StringEsc => {
                let Some(byte) = data.get(index).copied() else {
                    return (index, Some(TerminalInputEscapeState::StringEsc));
                };
                index += 1;
                if byte == b'\\' || byte == b'\x07' {
                    return (index, None);
                }
                state = TerminalInputEscapeState::String;
            }
        }
    }
}

fn next_utf8_char(data: &[u8], index: usize) -> Option<(char, usize)> {
    let suffix = &data[index..];
    let text = match std::str::from_utf8(suffix) {
        Ok(text) => text,
        Err(error) if error.valid_up_to() > 0 => {
            std::str::from_utf8(&suffix[..error.valid_up_to()]).ok()?
        }
        Err(_) => return None,
    };
    let ch = text.chars().next()?;
    Some((ch, index + ch.len_utf8()))
}

fn consume_matching_prefix(prefix_buffer: &mut String, text: &str) -> usize {
    let mut prefix_idx = 0;
    let mut text_idx = 0;

    while prefix_idx < prefix_buffer.len() && text_idx < text.len() {
        let prefix_char = prefix_buffer[prefix_idx..].chars().next();
        let text_char = text[text_idx..].chars().next();

        match (prefix_char, text_char) {
            (Some(left), Some(right)) if left == right => {
                prefix_idx += left.len_utf8();
                text_idx += right.len_utf8();
            }
            _ => break,
        }
    }

    if prefix_idx > 0 {
        prefix_buffer.drain(..prefix_idx);
    }

    text_idx
}

fn strip_one_leading_newline(text: &str) -> &str {
    text.strip_prefix('\n').unwrap_or(text)
}

fn lock_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

enum HistoryMatcher {
    Literal {
        needle: String,
        case_sensitive: bool,
        whole_word: bool,
    },
    Regex(regex::Regex),
}

impl HistoryMatcher {
    fn new(
        query: &str,
        case_sensitive: bool,
        regex: bool,
        whole_word: bool,
    ) -> Result<Self, RecordingError> {
        if regex {
            let pattern = if whole_word {
                format!(r"\b(?:{query})\b")
            } else {
                query.to_string()
            };
            let compiled = RegexBuilder::new(&pattern)
                .case_insensitive(!case_sensitive)
                .build()?;
            return Ok(Self::Regex(compiled));
        }

        let needle = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };

        Ok(Self::Literal {
            needle,
            case_sensitive,
            whole_word,
        })
    }

    fn find(&self, haystack: &str) -> Option<(usize, usize)> {
        match self {
            Self::Literal {
                needle,
                case_sensitive,
                whole_word,
            } => {
                let searchable = if *case_sensitive {
                    haystack.to_string()
                } else {
                    haystack.to_lowercase()
                };
                find_literal_match(&searchable, needle, *whole_word)
            }
            Self::Regex(regex) => regex
                .find(haystack)
                .map(|found| (found.start(), found.end())),
        }
    }
}

fn find_literal_match(haystack: &str, needle: &str, whole_word: bool) -> Option<(usize, usize)> {
    if needle.is_empty() {
        return None;
    }

    let mut offset = 0;
    while offset <= haystack.len() {
        let relative = haystack[offset..].find(needle)?;
        let start = offset + relative;
        let end = start + needle.len();

        if !whole_word || is_word_boundary_match(haystack, start, end) {
            return Some((start, end));
        }

        offset = end;
    }

    None
}

fn is_word_boundary_match(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();

    before.is_none_or(|ch| !is_word_char(ch)) && after.is_none_or(|ch| !is_word_char(ch))
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

fn context_records(
    records: &[TranscriptRecord],
    index: usize,
    count: usize,
    before: bool,
) -> Vec<String> {
    if count == 0 {
        return Vec::new();
    }

    if before {
        let start = index.saturating_sub(count);
        return records[start..index]
            .iter()
            .map(|record| record.data.clone())
            .collect();
    }

    let start = index.saturating_add(1);
    let end = start.saturating_add(count).min(records.len());
    records[start..end]
        .iter()
        .map(|record| record.data.clone())
        .collect()
}

fn strip_terminal_control_sequences(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'\x1b' => {
                i += 1;
                if i >= bytes.len() {
                    break;
                }
                match bytes[i] {
                    b'[' => {
                        i += 1;
                        while i < bytes.len() {
                            let b = bytes[i];
                            i += 1;
                            if (0x40..=0x7e).contains(&b) {
                                break;
                            }
                        }
                    }
                    b']' => {
                        i += 1;
                        while i < bytes.len() {
                            if bytes[i] == b'\x07' {
                                i += 1;
                                break;
                            }
                            if bytes[i] == b'\x1b' && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                                i += 2;
                                break;
                            }
                            i += 1;
                        }
                    }
                    b'P' | b'X' | b'^' | b'_' => {
                        i += 1;
                        while i < bytes.len() {
                            if bytes[i] == b'\x1b' && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                                i += 2;
                                break;
                            }
                            i += 1;
                        }
                    }
                    _ => {
                        advance_one_char(text, &mut i);
                    }
                }
            }
            b'\r' => {
                if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                    out.push('\n');
                    i += 2;
                } else {
                    i += 1;
                }
            }
            b'\n' | b'\t' => {
                out.push(bytes[i] as char);
                i += 1;
            }
            b if b.is_ascii_control() => {
                i += 1;
            }
            b if b.is_ascii() => {
                out.push(b as char);
                i += 1;
            }
            _ => {
                if !text.is_char_boundary(i) {
                    i += 1;
                    continue;
                }
                let Some(ch) = text[i..].chars().next() else {
                    break;
                };
                out.push(ch);
                i += ch.len_utf8();
            }
        }
    }

    out
}

fn advance_one_char(text: &str, index: &mut usize) {
    if *index >= text.len() {
        return;
    }

    if !text.is_char_boundary(*index) {
        *index += 1;
        return;
    }

    if let Some(ch) = text[*index..].chars().next() {
        *index += ch.len_utf8();
    } else {
        *index = text.len();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RecordingManager, consume_matching_prefix, safe_recording_name, strip_one_leading_newline,
        strip_terminal_control_sequences,
    };
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_path(name: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir()
            .join(format!("nyaterm-recording-{name}-{nanos}.log"))
            .to_string_lossy()
            .to_string()
    }

    #[test]
    fn safe_recording_name_preserves_readable_parts() {
        assert_eq!(safe_recording_name(""), "session");
        assert_eq!(safe_recording_name("my session!/prod"), "my_session_prod");
        assert_eq!(safe_recording_name("ssh.host_01-prod"), "ssh.host_01-prod");
    }

    #[test]
    fn strips_terminal_escape_sequences_from_output() {
        let raw = concat!(
            "\x1b[?2004l",
            "app.log  \x1b[0m\x1b[01;34mgo\x1b[0m\n",
            "\x1b]7;file://ubuntu/root\x07",
            "\x1b[?2004h\x1b[0m\x1b[1;33m[root\x1b[1;37m@\x1b[1;36mubuntu ",
            "\x1b[1;32m~\x1b[1;35m]\x1b[1;31m\n\n# \x1b[0m"
        );

        let cleaned = strip_terminal_control_sequences(raw);
        assert_eq!(cleaned, "app.log  go\n[root@ubuntu ~]\n\n# ");
    }

    #[test]
    fn strips_unknown_escape_with_multibyte_replacement_without_panicking() {
        let raw = format!("before\x1b{}after\n", char::REPLACEMENT_CHARACTER);
        let cleaned = strip_terminal_control_sequences(&raw);
        assert_eq!(cleaned, "beforeafter\n");
    }

    #[test]
    fn consumes_matching_echo_prefix() {
        let mut prefix = "ps -ef".to_string();
        let consumed = consume_matching_prefix(&mut prefix, "ps -ef\nUID");
        assert_eq!(consumed, "ps -ef".len());
        assert!(prefix.is_empty());
    }

    #[test]
    fn strips_only_one_leading_newline() {
        assert_eq!(strip_one_leading_newline("\nhello"), "hello");
        assert_eq!(strip_one_leading_newline("hello"), "hello");
        assert_eq!(strip_one_leading_newline("\n\nhello"), "\nhello");
    }

    #[test]
    fn writes_recording_with_and_without_io_labels() {
        let manager = RecordingManager::new();
        let labeled_path = unique_path("labels");
        manager.start("s1", &labeled_path, true, true).unwrap();
        manager.write_input("s1", b"echo hi\r");
        manager.write_output("s1", "echo hi\r\nhi\n");
        manager.stop("s1").unwrap();

        let labeled = fs::read_to_string(&labeled_path).unwrap();
        assert!(labeled.contains("[INPUT] echo hi"));
        assert!(labeled.contains("[OUTPUT] hi"));

        let plain_path = unique_path("plain");
        manager.start("s1", &plain_path, false, true).unwrap();
        manager.write_output("s1", "done\n");
        manager.stop("s1").unwrap();

        let plain = fs::read_to_string(&plain_path).unwrap();
        assert!(!plain.contains("[INPUT]"));
        assert!(!plain.contains("[OUTPUT]"));
        assert!(plain.contains("done"));

        let _ = fs::remove_file(labeled_path);
        let _ = fs::remove_file(plain_path);
    }

    #[test]
    fn writes_recording_without_timestamps() {
        let manager = RecordingManager::new();

        let labeled_path = unique_path("no-timestamp-labels");
        manager.start("s1", &labeled_path, true, false).unwrap();
        manager.write_output("s1", "done\n");
        manager.stop("s1").unwrap();

        let labeled = fs::read_to_string(&labeled_path).unwrap();
        assert_eq!(labeled, "[OUTPUT] done\n");

        let plain_path = unique_path("no-timestamp-plain");
        manager.start("s1", &plain_path, false, false).unwrap();
        manager.write_output("s1", "plain\n");
        manager.stop("s1").unwrap();

        let plain = fs::read_to_string(&plain_path).unwrap();
        assert_eq!(plain, "plain\n");

        let _ = fs::remove_file(labeled_path);
        let _ = fs::remove_file(plain_path);
    }

    #[test]
    fn text_input_records_logical_utf8_text() {
        let manager = RecordingManager::new();
        let path = unique_path("logical-input");

        manager.start("s1", &path, true, false).unwrap();
        manager.write_input("s1", "echo 测试\r".as_bytes());
        manager.stop("s1").unwrap();

        let recorded = fs::read_to_string(&path).unwrap();
        assert_eq!(recorded, "[INPUT] echo 测试\n");
        assert!(!recorded.contains(char::REPLACEMENT_CHARACTER));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn raw_input_records_exact_bytes_as_hex() {
        let manager = RecordingManager::new();
        let path = unique_path("raw-input");
        let bytes = [0x00, 0xff, 0x1b, b'[', b'A'];

        manager.start("s1", &path, true, false).unwrap();
        manager.write_raw_input("s1", &bytes);
        manager.stop("s1").unwrap();

        let recorded = fs::read_to_string(&path).unwrap();
        assert_eq!(recorded, "[RAW_INPUT] 00 ff 1b 5b 41\n");
        assert!(!recorded.contains(char::REPLACEMENT_CHARACTER));

        manager.write_raw_input("s2", &bytes);
        let transcript_path = unique_path("raw-input-transcript");
        manager
            .save_transcript("s2", &transcript_path, true, false)
            .unwrap();
        let transcript = fs::read_to_string(&transcript_path).unwrap();
        assert_eq!(transcript, "[RAW_INPUT] 00 ff 1b 5b 41\n");
        assert!(!transcript.contains(char::REPLACEMENT_CHARACTER));

        let _ = fs::remove_file(path);
        let _ = fs::remove_file(transcript_path);
    }

    #[test]
    fn terminal_protocol_input_sequences_do_not_pollute_recorded_text() {
        let manager = RecordingManager::new();
        let path = unique_path("protocol-input");

        manager.write_input("s1", b"\x1b[A");
        manager.write_input("s1", b"echo ");
        manager.write_input("s1", b"\x1b[<0;12;5M");
        manager.write_input("s1", "界".as_bytes());
        manager.write_input("s1", b"\r");
        manager.save_transcript("s1", &path, true, false).unwrap();

        let transcript = fs::read_to_string(&path).unwrap();
        assert_eq!(transcript, "[INPUT] echo 界\n");
        assert!(!transcript.contains("[A"));
        assert!(!transcript.contains("<0;12;5M"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn split_terminal_protocol_input_sequences_do_not_pollute_recorded_text() {
        let manager = RecordingManager::new();
        let path = unique_path("split-protocol-input");

        manager.write_input("s1", b"echo ");
        manager.write_input("s1", b"\x1b[<0;");
        manager.write_input("s1", b"12;5M");
        manager.write_input("s1", b"\x1b]");
        manager.write_input("s1", b"52;c;clipboard");
        manager.write_input("s1", b"\x1b");
        manager.write_input("s1", b"\\");
        manager.write_input("s1", b"\x1bO");
        manager.write_input("s1", b"A");
        manager.write_input("s1", b"done\r");
        manager.save_transcript("s1", &path, true, false).unwrap();

        let transcript = fs::read_to_string(&path).unwrap();
        assert_eq!(transcript, "[INPUT] echo done\n");
        assert!(!transcript.contains("<0;"));
        assert!(!transcript.contains("clipboard"));
        assert!(!transcript.contains("\x1b"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn saves_memory_transcript_and_trims_old_records() {
        let manager = RecordingManager::new();
        manager.set_memory_limit(90);
        manager.write_output("s1", "first line\n");
        manager.write_output("s1", "second line\n");
        manager.write_output("s1", "third line\n");

        let path = unique_path("memory");
        manager.save_transcript("s1", &path, true, true).unwrap();
        let saved = fs::read_to_string(&path).unwrap();

        assert!(!saved.contains("first line"));
        assert!(saved.contains("third line"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn terminal_history_search_finds_literal_matches() {
        let manager = RecordingManager::new();
        manager.write_output("s1", "alpha\nbeta install\nbeta done\n");

        let result = manager
            .search_history(super::TerminalHistorySearchRequest {
                session_id: "s1".to_string(),
                query: "beta".to_string(),
                case_sensitive: false,
                regex: false,
                whole_word: false,
                limit: Some(100),
                context_before: Some(1),
                context_after: Some(1),
                max_lines: None,
            })
            .unwrap();

        assert_eq!(result.total, 2);
        assert_eq!(result.results.len(), 2);
        assert_eq!(result.results[0].line_number, 2);
        assert_eq!(result.results[0].before, vec!["alpha"]);
        assert_eq!(result.results[0].after, vec!["beta done"]);
        assert_eq!(result.results[0].source, "output");
    }

    #[test]
    fn terminal_history_search_honors_case_and_whole_word() {
        let manager = RecordingManager::new();
        manager.write_output("s1", "install\nInstall\ninstaller\n");

        let case_sensitive = manager
            .search_history(super::TerminalHistorySearchRequest {
                session_id: "s1".to_string(),
                query: "Install".to_string(),
                case_sensitive: true,
                regex: false,
                whole_word: false,
                limit: Some(100),
                context_before: Some(0),
                context_after: Some(0),
                max_lines: None,
            })
            .unwrap();
        assert_eq!(case_sensitive.total, 1);
        assert_eq!(case_sensitive.results[0].preview, "Install");

        let whole_word = manager
            .search_history(super::TerminalHistorySearchRequest {
                session_id: "s1".to_string(),
                query: "install".to_string(),
                case_sensitive: false,
                regex: false,
                whole_word: true,
                limit: Some(100),
                context_before: Some(0),
                context_after: Some(0),
                max_lines: None,
            })
            .unwrap();
        assert_eq!(whole_word.total, 2);
    }

    #[test]
    fn terminal_history_search_supports_regex_limit_and_truncation() {
        let manager = RecordingManager::new();
        manager.write_output("s1", "error 100\nerror 200\nok\n");

        let result = manager
            .search_history(super::TerminalHistorySearchRequest {
                session_id: "s1".to_string(),
                query: r"error \d+".to_string(),
                case_sensitive: false,
                regex: true,
                whole_word: false,
                limit: Some(1),
                context_before: Some(0),
                context_after: Some(0),
                max_lines: None,
            })
            .unwrap();

        assert_eq!(result.total, 2);
        assert_eq!(result.results.len(), 1);
        assert!(result.truncated);
        assert_eq!(result.results[0].preview, "error 100");
    }

    #[test]
    fn recording_does_not_backfill_existing_memory() {
        let manager = RecordingManager::new();
        manager.write_output("s1", "before\n");

        let path = unique_path("no-backfill");
        manager.start("s1", &path, true, true).unwrap();
        manager.write_output("s1", "after\n");
        manager.stop("s1").unwrap();

        let recorded = fs::read_to_string(&path).unwrap();
        assert!(!recorded.contains("before"));
        assert!(recorded.contains("after"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn recording_does_not_backfill_partial_output_buffer() {
        let manager = RecordingManager::new();
        manager.write_output("s1", "prompt without newline");

        let path = unique_path("no-partial-backfill");
        manager.start("s1", &path, true, true).unwrap();
        manager.write_output("s1", "\nafter\n");
        manager.stop("s1").unwrap();

        let recorded = fs::read_to_string(&path).unwrap();
        assert!(!recorded.contains("prompt without newline"));
        assert!(recorded.contains("after"));

        let _ = fs::remove_file(path);
    }
}

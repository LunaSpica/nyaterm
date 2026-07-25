use super::*;
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::Arc;

pub(super) type CompiledKeywordRules = Vec<(regex::Regex, u32)>;

pub struct TerminalKeywordHighlighter {
    rules_key: u64,
    compiled: CompiledKeywordRules,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum TerminalKeywordRowReuseKey {
    Single { signature: u64 },
    Wrapped { group_key: u64, row_offset: usize },
}

/// Immutable keyword data prepared away from GPUI's paint path.
pub struct TerminalKeywordHighlightSnapshot {
    rules_key: u64,
    palette_key: u64,
    display_offset: usize,
    line_signatures: Vec<u64>,
    wrapped_flags: Vec<bool>,
    row_reuse_keys: Vec<Option<TerminalKeywordRowReuseKey>>,
    known_rows: Vec<bool>,
    rows: Vec<Option<Arc<Vec<TerminalKeywordRange>>>>,
    rows_by_reuse_key: HashMap<TerminalKeywordRowReuseKey, Option<Arc<Vec<TerminalKeywordRange>>>>,
}

pub(super) enum TerminalKeywordHighlightLookup<'a> {
    Current(Option<&'a Arc<Vec<TerminalKeywordRange>>>),
    Reused(Option<&'a Arc<Vec<TerminalKeywordRange>>>),
}

impl<'a> TerminalKeywordHighlightLookup<'a> {
    pub(super) fn ranges(&self) -> Option<&'a Arc<Vec<TerminalKeywordRange>>> {
        match self {
            Self::Current(ranges) | Self::Reused(ranges) => *ranges,
        }
    }

    pub(super) fn is_known_empty(&self) -> bool {
        self.ranges().is_none()
    }
}

impl TerminalKeywordHighlightSnapshot {
    pub fn rules_key(&self) -> u64 {
        self.rules_key
    }

    pub fn known_row_count(&self) -> usize {
        self.known_rows.iter().filter(|known| **known).count()
    }

    pub fn range_count(&self) -> usize {
        self.rows
            .iter()
            .filter_map(|ranges| ranges.as_ref())
            .map(|ranges| ranges.len())
            .sum()
    }

    pub fn matches_snapshot(
        &self,
        snapshot: &TerminalSnapshot,
        palette: nyaterm_ui::ThemePalette,
    ) -> bool {
        self.matches_snapshot_rows(snapshot, palette, 0..snapshot.row_count())
    }

    pub fn matches_snapshot_rows(
        &self,
        snapshot: &TerminalSnapshot,
        palette: nyaterm_ui::ThemePalette,
        rows: Range<usize>,
    ) -> bool {
        if self.display_offset != snapshot.display_offset {
            return false;
        }
        if self.palette_key != terminal_keyword_palette_key(palette) {
            return false;
        }
        let start = rows.start.min(snapshot.row_count());
        let end = rows.end.min(snapshot.row_count()).max(start);
        (start..end).all(|row| self.has_row_at(row, snapshot))
    }

    pub(super) fn lookup(
        &self,
        row: usize,
        snapshot: &TerminalSnapshot,
    ) -> Option<TerminalKeywordHighlightLookup<'_>> {
        snapshot.row(row)?;
        if self.has_row_at(row, snapshot) {
            return Some(TerminalKeywordHighlightLookup::Current(
                self.rows.get(row)?.as_ref(),
            ));
        }
        let reuse_key = terminal_keyword_row_reuse_key(snapshot, row)?;
        self.rows_by_reuse_key
            .get(&reuse_key)
            .map(|ranges| TerminalKeywordHighlightLookup::Reused(ranges.as_ref()))
    }

    pub(super) fn stale_lookup(
        &self,
        row: usize,
        snapshot: &TerminalSnapshot,
    ) -> Option<TerminalKeywordHighlightLookup<'_>> {
        if self.display_offset != snapshot.display_offset
            || self.line_signatures.len() != snapshot.row_count()
        {
            return None;
        }
        snapshot.row(row)?;
        if !self.has_row_at(row, snapshot) {
            return None;
        }
        Some(TerminalKeywordHighlightLookup::Current(
            self.rows.get(row)?.as_ref(),
        ))
    }

    fn has_row_at(&self, row: usize, snapshot: &TerminalSnapshot) -> bool {
        let Some(snapshot_row) = snapshot.row(row) else {
            return false;
        };
        self.known_rows.get(row).copied().unwrap_or(false)
            && self.line_signatures.get(row).copied() == Some(snapshot_row.signature)
            && self.wrapped_flags.get(row).copied() == Some(snapshot_row.wrapped)
            && self.row_reuse_keys.get(row).and_then(|key| *key)
                == terminal_keyword_row_reuse_key(snapshot, row)
    }
}

pub fn terminal_keyword_highlight_expanded_rows(
    snapshot: &TerminalSnapshot,
    rows: Range<usize>,
) -> Range<usize> {
    let start = rows.start.min(snapshot.row_count());
    let end = rows.end.min(snapshot.row_count()).max(start);
    if start == end {
        return start..end;
    }
    let mut expanded_start = start;
    while expanded_start > 0 && snapshot.row(expanded_start).is_some_and(|row| row.wrapped) {
        expanded_start -= 1;
    }
    let mut expanded_end = end;
    while expanded_end < snapshot.row_count()
        && snapshot.row(expanded_end).is_some_and(|row| row.wrapped)
    {
        expanded_end += 1;
    }
    expanded_start..expanded_end
}

pub fn precompute_terminal_keyword_highlights(
    snapshot: &TerminalSnapshot,
    highlighter: &TerminalKeywordHighlighter,
    palette: nyaterm_ui::ThemePalette,
    previous: Option<&TerminalKeywordHighlightSnapshot>,
) -> TerminalKeywordHighlightSnapshot {
    precompute_terminal_keyword_highlights_for_rows(
        snapshot,
        highlighter,
        palette,
        previous,
        0..snapshot.row_count(),
    )
}

pub fn precompute_terminal_keyword_highlights_for_rows(
    snapshot: &TerminalSnapshot,
    highlighter: &TerminalKeywordHighlighter,
    palette: nyaterm_ui::ThemePalette,
    previous: Option<&TerminalKeywordHighlightSnapshot>,
    requested_rows: Range<usize>,
) -> TerminalKeywordHighlightSnapshot {
    let palette_key = terminal_keyword_palette_key(palette);
    let previous = previous.filter(|previous| {
        previous.rules_key == highlighter.rules_key && previous.palette_key == palette_key
    });
    let requested_rows = terminal_keyword_highlight_expanded_rows(snapshot, requested_rows);
    let mut known_rows = vec![false; snapshot.row_count()];
    let mut rows: Vec<Option<Arc<Vec<TerminalKeywordRange>>>> = vec![None; snapshot.row_count()];
    let row_reuse_keys = (0..snapshot.row_count())
        .map(|row| terminal_keyword_row_reuse_key(snapshot, row))
        .collect::<Vec<_>>();

    let mut row = 0;
    while row < snapshot.row_count() {
        let group = terminal_keyword_wrapped_group_bounds(snapshot, row);
        let group_range = group.start..group.end;
        let requested = group.start < requested_rows.end && group.end > requested_rows.start;
        let group_keys = (group.start..group.end)
            .map(|idx| terminal_keyword_row_reuse_key(snapshot, idx))
            .collect::<Option<Vec<_>>>();
        if let Some(keys) = group_keys.as_ref()
            && let Some(previous) = previous
            && keys
                .iter()
                .all(|key| previous.rows_by_reuse_key.contains_key(key))
        {
            for (idx, key) in group_range.clone().zip(keys.iter()) {
                known_rows[idx] = true;
                rows[idx] = previous.rows_by_reuse_key.get(key).cloned().flatten();
            }
            row = group.end;
            continue;
        }
        if !requested {
            row = group.end;
            continue;
        }

        let group_rows =
            terminal_keyword_ranges_for_wrapped_group(snapshot, group_range.clone(), highlighter);
        for (idx, ranges) in group_range.zip(group_rows) {
            known_rows[idx] = true;
            rows[idx] = (!ranges.is_empty()).then(|| Arc::new(ranges));
        }
        row = group.end;
    }

    let rows_by_reuse_key = (0..snapshot.row_count())
        .filter(|row| known_rows[*row])
        .filter_map(|row| {
            terminal_keyword_row_reuse_key(snapshot, row).map(|key| (key, rows[row].clone()))
        })
        .collect();
    TerminalKeywordHighlightSnapshot {
        rules_key: highlighter.rules_key,
        palette_key,
        display_offset: snapshot.display_offset,
        line_signatures: snapshot.rows().iter().map(|row| row.signature).collect(),
        wrapped_flags: snapshot.rows().iter().map(|row| row.wrapped).collect(),
        row_reuse_keys,
        known_rows,
        rows,
        rows_by_reuse_key,
    }
}

#[derive(Clone, Copy)]
struct TerminalKeywordWrappedGroup {
    start: usize,
    end: usize,
}

#[derive(Clone, Copy)]
struct TerminalKeywordByteCell {
    row: usize,
    start_col: usize,
    end_col: usize,
}

fn terminal_keyword_wrapped_group_bounds(
    snapshot: &TerminalSnapshot,
    row: usize,
) -> TerminalKeywordWrappedGroup {
    let mut start = row.min(snapshot.row_count());
    while start > 0 && snapshot.row(start).is_some_and(|row| row.wrapped) {
        start -= 1;
    }
    let mut end = start.saturating_add(1).min(snapshot.row_count());
    while end < snapshot.row_count() && snapshot.row(end).is_some_and(|row| row.wrapped) {
        end += 1;
    }
    TerminalKeywordWrappedGroup { start, end }
}

fn terminal_keyword_row_reuse_key(
    snapshot: &TerminalSnapshot,
    row: usize,
) -> Option<TerminalKeywordRowReuseKey> {
    let group = terminal_keyword_wrapped_group_bounds(snapshot, row);
    if group.end == group.start.saturating_add(1) {
        return snapshot
            .row(row)
            .map(|row| TerminalKeywordRowReuseKey::Single {
                signature: row.signature,
            });
    }

    let mut hasher = DefaultHasher::new();
    (group.start..group.end).for_each(|idx| {
        if let Some(row) = snapshot.row(idx) {
            row.signature.hash(&mut hasher);
            row.wrapped.hash(&mut hasher);
        }
    });
    Some(TerminalKeywordRowReuseKey::Wrapped {
        group_key: hasher.finish(),
        row_offset: row.saturating_sub(group.start),
    })
}

fn terminal_keyword_ranges_for_wrapped_group(
    snapshot: &TerminalSnapshot,
    rows: Range<usize>,
    highlighter: &TerminalKeywordHighlighter,
) -> Vec<Vec<TerminalKeywordRange>> {
    let row_count = rows.end.saturating_sub(rows.start);
    let mut row_ranges = vec![Vec::new(); row_count];
    if highlighter.compiled.is_empty() || row_count == 0 {
        return row_ranges;
    }

    let mut line = String::new();
    let mut byte_cells = Vec::new();
    for row in rows.clone() {
        let Some(snapshot_row) = snapshot.row(row) else {
            continue;
        };
        for (col, cell) in snapshot_row.cells.iter().enumerate() {
            if cell.width == 0 {
                continue;
            }
            let text = if cell.text.is_empty() {
                " "
            } else {
                cell.text.as_str()
            };
            let width = usize::from(cell.width).max(1);
            let start_col = col;
            let end_col = col.saturating_add(width);
            line.push_str(text);
            byte_cells.extend((0..text.len()).map(|_| TerminalKeywordByteCell {
                row,
                start_col,
                end_col,
            }));
        }
    }
    if line.is_empty() || byte_cells.is_empty() {
        return row_ranges;
    }

    for (start, end, color) in keyword_matches_compiled(&line, &highlighter.compiled) {
        if start >= end || end > byte_cells.len() {
            continue;
        }
        for cell in &byte_cells[start..end] {
            let row_idx = cell.row.saturating_sub(rows.start);
            if let Some(ranges) = row_ranges.get_mut(row_idx) {
                if let Some(previous) = ranges.last_mut()
                    && previous.color == color
                    && previous.end_col >= cell.start_col
                {
                    previous.end_col = previous.end_col.max(cell.end_col);
                    continue;
                }
                ranges.push(TerminalKeywordRange {
                    start_col: cell.start_col,
                    end_col: cell.end_col,
                    color,
                });
            }
        }
    }

    row_ranges
}

fn keyword_matches_compiled(
    line: &str,
    compiled: &[(regex::Regex, u32)],
) -> Vec<(usize, usize, u32)> {
    let mut matches = Vec::new();
    let mut cursor = 0;
    while cursor < line.len() {
        let mut best: Option<(usize, usize, u32)> = None;
        for (regex, color) in compiled {
            if let Some(found) = regex.find_at(line, cursor) {
                let start = found.start();
                let end = found.end();
                if end <= start {
                    continue;
                }
                let replace = best
                    .map(|(best_start, best_end, _)| {
                        start < best_start || (start == best_start && end > best_end)
                    })
                    .unwrap_or(true);
                if replace {
                    best = Some((start, end, *color));
                }
            }
        }

        let Some((start, end, color)) = best else {
            break;
        };
        matches.push((start, end, color));
        cursor = end;
    }
    matches
}

pub fn compile_terminal_keyword_highlighter(
    rules: &[ResolvedKeywordHighlightRule],
) -> TerminalKeywordHighlighter {
    TerminalKeywordHighlighter {
        rules_key: terminal_keyword_rules_key(rules),
        compiled: compile_keyword_rules(rules),
    }
}

pub fn terminal_keyword_rules_key(rules: &[ResolvedKeywordHighlightRule]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for rule in rules {
        rule.id.hash(&mut hasher);
        rule.name.hash(&mut hasher);
        rule.patterns.hash(&mut hasher);
        rule.color.hash(&mut hasher);
        rule.enabled.hash(&mut hasher);
    }
    hasher.finish()
}

pub(super) fn terminal_keyword_palette_key(palette: nyaterm_ui::ThemePalette) -> u64 {
    let mut hasher = DefaultHasher::new();
    palette.bg.hash(&mut hasher);
    palette.surface.hash(&mut hasher);
    palette.accent.hash(&mut hasher);
    palette.warning.hash(&mut hasher);
    palette.terminal_fg.hash(&mut hasher);
    palette.terminal_bg.hash(&mut hasher);
    palette.terminal_ansi.hash(&mut hasher);
    hasher.finish()
}

pub(super) fn keyword_highlight_spans_compiled(
    line: &str,
    compiled: &[(regex::Regex, u32)],
) -> Vec<TerminalHighlightSpan> {
    if compiled.is_empty() || line.is_empty() {
        return vec![TerminalHighlightSpan {
            text: line.to_string(),
            color: None,
            bg: None,
            keyword: false,
            underline: false,
            strikeout: false,
            bold: false,
            italic: false,
        }];
    }

    let mut spans = Vec::new();
    let mut cursor = 0;
    while cursor < line.len() {
        let mut best: Option<(usize, usize, u32)> = None;
        for (regex, color) in compiled {
            if let Some(found) = regex.find_at(line, cursor) {
                let start = found.start();
                let end = found.end();
                if end <= start {
                    continue;
                }
                let replace = best
                    .map(|(best_start, best_end, _)| {
                        start < best_start || (start == best_start && end > best_end)
                    })
                    .unwrap_or(true);
                if replace {
                    best = Some((start, end, *color));
                }
            }
        }

        let Some((start, end, color)) = best else {
            spans.push(TerminalHighlightSpan {
                text: line[cursor..].to_string(),
                color: None,
                bg: None,
                keyword: false,
                underline: false,
                strikeout: false,
                bold: false,
                italic: false,
            });
            break;
        };
        if start > cursor {
            spans.push(TerminalHighlightSpan {
                text: line[cursor..start].to_string(),
                color: None,
                bg: None,
                keyword: false,
                underline: false,
                strikeout: false,
                bold: false,
                italic: false,
            });
        }
        spans.push(TerminalHighlightSpan {
            text: line[start..end].to_string(),
            color: Some(color),
            bg: None,
            keyword: true,
            underline: false,
            strikeout: false,
            bold: false,
            italic: false,
        });
        cursor = end;
    }

    if spans.is_empty() {
        spans.push(TerminalHighlightSpan {
            text: " ".to_string(),
            color: None,
            bg: None,
            keyword: false,
            underline: false,
            strikeout: false,
            bold: false,
            italic: false,
        });
    }
    spans
}
pub(super) fn compile_keyword_rules(
    rules: &[ResolvedKeywordHighlightRule],
) -> CompiledKeywordRules {
    let mut compiled = Vec::new();
    for rule in rules.iter().filter(|rule| rule.enabled) {
        let color = parse_hex_rgb(&rule.color).unwrap_or(0x79c0ff);
        let mut alts = Vec::new();
        for pattern in rule
            .patterns
            .iter()
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
        {
            // Validate each alternative; skip invalid regex like Tauri.
            if regex::Regex::new(&format!("(?i)(?:{pattern})")).is_ok()
                || regex::Regex::new(pattern).is_ok()
            {
                alts.push(pattern.to_string());
            }
        }
        if alts.is_empty() {
            continue;
        }
        let combined = if alts.len() == 1 {
            alts[0].clone()
        } else {
            alts.iter()
                .map(|p| format!("(?:{p})"))
                .collect::<Vec<_>>()
                .join("|")
        };
        let pattern = if combined.contains("(?i)") || combined.contains("(?-i)") {
            combined
        } else {
            format!("(?i){combined}")
        };
        match regex::Regex::new(&pattern) {
            Ok(regex) => compiled.push((regex, color)),
            Err(_) => {
                for alt in alts {
                    let pat = if alt.contains("(?i)") {
                        alt
                    } else {
                        format!("(?i){alt}")
                    };
                    if let Ok(regex) = regex::Regex::new(&pat) {
                        compiled.push((regex, color));
                    }
                }
            }
        }
    }
    compiled
}
pub fn terminal_buffer_matches(
    output: &str,
    query: &str,
    flags: &TerminalSearchFlags,
    limit: usize,
) -> Result<Vec<TerminalBufferMatch>, String> {
    if query.trim().is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    let mut matches = Vec::new();
    if flags.regex {
        let pattern = if flags.case_sensitive {
            query.to_string()
        } else {
            format!("(?i){query}")
        };
        let regex = regex::Regex::new(&pattern).map_err(|error| error.to_string())?;
        for (line_index, line) in output.lines().enumerate() {
            for found in regex.find_iter(line) {
                if flags.whole_word && !is_whole_word_match(line, found.start(), found.end()) {
                    continue;
                }
                let start_col = terminal_cell_col_for_byte_index(line, found.start());
                let end_col = terminal_cell_col_for_byte_index(line, found.end());
                matches.push(TerminalBufferMatch {
                    line_index,
                    start_col,
                    end_col,
                });
                if matches.len() >= limit {
                    return Ok(matches);
                }
            }
        }
        return Ok(matches);
    }

    let needle = if flags.case_sensitive {
        query.to_string()
    } else {
        query.to_ascii_lowercase()
    };
    for (line_index, line) in output.lines().enumerate() {
        let haystack = if flags.case_sensitive {
            line.to_string()
        } else {
            line.to_ascii_lowercase()
        };
        let mut cursor = 0;
        while cursor <= haystack.len() {
            let Some(relative_start) = haystack[cursor..].find(&needle) else {
                break;
            };
            let start = cursor + relative_start;
            let end = start + needle.len();
            if !flags.whole_word || is_whole_word_match(line, start, end) {
                let start_col = terminal_cell_col_for_byte_index(line, start);
                let end_col = terminal_cell_col_for_byte_index(line, end);
                matches.push(TerminalBufferMatch {
                    line_index,
                    start_col,
                    end_col,
                });
                if matches.len() >= limit {
                    return Ok(matches);
                }
            }
            cursor = end.max(cursor + 1);
        }
    }
    Ok(matches)
}
pub(super) fn is_whole_word_match(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    !before.is_some_and(is_word_char) && !after.is_some_and(is_word_char)
}
pub(super) fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}
pub(super) fn parse_hex_rgb(value: &str) -> Option<u32> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if hex.len() != 6 {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_snapshot_row(
        snapshot: &mut TerminalSnapshot,
        row: usize,
        text: impl Into<String>,
        signature: u64,
    ) {
        let text = text.into();
        let cells = test_render_cells(&text, snapshot.cols);
        let rows = Arc::make_mut(&mut snapshot.row_data);
        let row = Arc::make_mut(&mut rows[row]);
        row.text = text.clone();
        row.styled_spans = vec![nyaterm_terminal::StyledSpan {
            text,
            style: nyaterm_terminal::CellStyle::default(),
        }]
        .into_boxed_slice();
        row.cells = cells.into_boxed_slice();
        row.signature = signature;
    }

    fn set_snapshot_row_wrapped(
        snapshot: &mut TerminalSnapshot,
        row: usize,
        text: impl Into<String>,
        signature: u64,
        wrapped: bool,
    ) {
        set_snapshot_row(snapshot, row, text, signature);
        let rows = Arc::make_mut(&mut snapshot.row_data);
        Arc::make_mut(&mut rows[row]).wrapped = wrapped;
    }

    fn test_render_cells(text: &str, cols: usize) -> Vec<nyaterm_terminal::RenderCell> {
        let mut cells: Vec<nyaterm_terminal::RenderCell> = Vec::new();
        for ch in text.chars() {
            if terminal_is_zero_width_mark(ch)
                && let Some(previous) = cells.last_mut()
            {
                previous.text.push(ch);
                continue;
            }
            let width = terminal_char_cell_width(ch);
            cells.push(nyaterm_terminal::RenderCell {
                text: ch.to_string(),
                style: nyaterm_terminal::CellStyle::default(),
                width: width as u8,
                hyperlink: None,
            });
            for _ in 1..width {
                cells.push(nyaterm_terminal::RenderCell {
                    text: String::new(),
                    style: nyaterm_terminal::CellStyle::default(),
                    width: 0,
                    hyperlink: None,
                });
            }
        }
        while cells.len() < cols {
            cells.push(nyaterm_terminal::RenderCell {
                text: String::new(),
                style: nyaterm_terminal::CellStyle::default(),
                width: 1,
                hyperlink: None,
            });
        }
        cells.truncate(cols);
        cells
    }

    fn shifted_display_offset(snapshot: &TerminalSnapshot, amount: usize) -> TerminalSnapshot {
        let mut shifted = snapshot.clone();
        shifted.display_offset = shifted.display_offset.saturating_add(amount);
        shifted
    }

    #[test]
    fn keyword_highlights_keep_earliest_longest_and_rule_priority() {
        let compiled = vec![
            (regex::Regex::new("ERR").unwrap(), 1),
            (regex::Regex::new("ERROR").unwrap(), 2),
            (regex::Regex::new("ERROR").unwrap(), 3),
        ];

        let spans = keyword_highlight_spans_compiled("x ERROR ERR", &compiled);

        assert_eq!(
            spans
                .iter()
                .map(|span| span.text.as_str())
                .collect::<Vec<_>>(),
            vec!["x ", "ERROR", " ", "ERR"]
        );
        assert_eq!(spans[1].color, Some(2));
        assert_eq!(spans[3].color, Some(1));
    }

    #[test]
    fn precomputed_keyword_snapshot_checks_line_signatures() {
        let mut snapshot = TerminalScreen::default().snapshot();
        set_snapshot_row(&mut snapshot, 0, "prefix ERROR suffix", 41);
        let rules = vec![ResolvedKeywordHighlightRule {
            id: "error".to_string(),
            name: "Error".to_string(),
            patterns: vec!["ERROR".to_string()],
            color: "#ff2244".to_string(),
            enabled: true,
        }];

        let highlighter = compile_terminal_keyword_highlighter(&rules);
        let palette = nyaterm_ui::theme_palette("github-dark");
        let highlights =
            precompute_terminal_keyword_highlights(&snapshot, &highlighter, palette, None);

        assert!(
            highlights
                .lookup(0, &snapshot)
                .and_then(|row| row.ranges())
                .is_some()
        );
        let mut changed_signature = snapshot.clone();
        {
            let rows = Arc::make_mut(&mut changed_signature.row_data);
            Arc::make_mut(&mut rows[0]).signature = 42;
        }
        assert!(highlights.lookup(0, &changed_signature).is_none());
        assert!(matches!(
            highlights.lookup(0, &snapshot),
            Some(TerminalKeywordHighlightLookup::Current(_))
        ));
        assert!(
            highlights
                .stale_lookup(0, &snapshot)
                .and_then(|row| row.ranges())
                .is_some()
        );
        assert!(highlights.stale_lookup(0, &changed_signature).is_none());
        assert!(highlights.stale_lookup(usize::MAX, &snapshot).is_none());
        assert!(
            highlights
                .stale_lookup(0, &shifted_display_offset(&snapshot, 1))
                .is_none()
        );
        assert!(highlights.matches_snapshot(&snapshot, palette));
        let mut shifted_snapshot = snapshot.clone();
        shifted_snapshot.display_offset = shifted_snapshot.display_offset.saturating_add(1);
        assert!(!highlights.matches_snapshot(&shifted_snapshot, palette));
        assert!(
            !highlights.matches_snapshot(&snapshot, nyaterm_ui::theme_palette("github-light"),)
        );
        assert!(highlights.rows.iter().skip(1).all(Option::is_none));
    }

    #[test]
    fn precomputed_keyword_snapshot_marks_empty_rows_as_known() {
        let mut snapshot = TerminalScreen::default().snapshot();
        set_snapshot_row(&mut snapshot, 0, "plain text", 41);
        let rules = vec![ResolvedKeywordHighlightRule {
            id: "error".to_string(),
            name: "Error".to_string(),
            patterns: vec!["ERROR".to_string()],
            color: "#ff2244".to_string(),
            enabled: true,
        }];

        let highlighter = compile_terminal_keyword_highlighter(&rules);
        let highlights = precompute_terminal_keyword_highlights(
            &snapshot,
            &highlighter,
            nyaterm_ui::theme_palette("github-dark"),
            None,
        );

        let lookup = highlights.lookup(0, &snapshot).expect("row lookup");
        assert!(lookup.is_known_empty());
        assert!(lookup.ranges().is_none());
    }

    #[test]
    fn partial_keyword_snapshot_keeps_unparsed_rows_unknown_and_accumulates() {
        let mut snapshot = TerminalScreen::default().snapshot();
        for (row, signature) in [(0, 41), (1, 42)] {
            set_snapshot_row(&mut snapshot, row, format!("row {row} ERROR"), signature);
        }
        let rules = vec![ResolvedKeywordHighlightRule {
            id: "error".to_string(),
            name: "Error".to_string(),
            patterns: vec!["ERROR".to_string()],
            color: "#ff2244".to_string(),
            enabled: true,
        }];
        let highlighter = compile_terminal_keyword_highlighter(&rules);
        let palette = nyaterm_ui::theme_palette("github-dark");

        let first = precompute_terminal_keyword_highlights_for_rows(
            &snapshot,
            &highlighter,
            palette,
            None,
            0..1,
        );
        assert!(first.lookup(0, &snapshot).is_some());
        assert!(first.lookup(1, &snapshot).is_none());
        assert!(first.matches_snapshot_rows(&snapshot, palette, 0..1));
        assert!(!first.matches_snapshot(&snapshot, palette));

        let second = precompute_terminal_keyword_highlights_for_rows(
            &snapshot,
            &highlighter,
            palette,
            Some(&first),
            1..2,
        );
        assert!(second.lookup(0, &snapshot).is_some());
        assert!(second.lookup(1, &snapshot).is_some());
        assert!(second.matches_snapshot_rows(&snapshot, palette, 0..2));
    }

    #[test]
    fn precomputed_keyword_snapshot_reuses_matching_rows_after_scroll() {
        let mut snapshot = TerminalScreen::default().snapshot();
        set_snapshot_row(&mut snapshot, 0, "prefix ERROR suffix", 41);
        let rules = vec![ResolvedKeywordHighlightRule {
            id: "error".to_string(),
            name: "Error".to_string(),
            patterns: vec!["ERROR".to_string()],
            color: "#ff2244".to_string(),
            enabled: true,
        }];

        let highlighter = compile_terminal_keyword_highlighter(&rules);
        let highlights = precompute_terminal_keyword_highlights(
            &snapshot,
            &highlighter,
            nyaterm_ui::theme_palette("github-dark"),
            None,
        );

        assert!(
            highlights
                .lookup(0, &snapshot)
                .and_then(|row| row.ranges())
                .is_some()
        );
        let mut scrolled_snapshot = TerminalScreen::default().snapshot();
        set_snapshot_row(&mut scrolled_snapshot, 3, "prefix ERROR suffix", 41);
        assert!(
            highlights
                .lookup(3, &scrolled_snapshot)
                .and_then(|row| row.ranges())
                .is_some()
        );
    }

    #[test]
    fn precomputed_keyword_snapshot_reuses_previous_rows_by_signature() {
        let mut first_snapshot = TerminalScreen::default().snapshot();
        set_snapshot_row(&mut first_snapshot, 0, "prefix ERROR suffix", 41);
        let mut second_snapshot = TerminalScreen::default().snapshot();
        set_snapshot_row(&mut second_snapshot, 5, "prefix ERROR suffix", 41);
        let rules = vec![ResolvedKeywordHighlightRule {
            id: "error".to_string(),
            name: "Error".to_string(),
            patterns: vec!["ERROR".to_string()],
            color: "#ff2244".to_string(),
            enabled: true,
        }];

        let highlighter = compile_terminal_keyword_highlighter(&rules);
        let palette = nyaterm_ui::theme_palette("github-dark");
        let first_highlights =
            precompute_terminal_keyword_highlights(&first_snapshot, &highlighter, palette, None);
        let first_ranges = first_highlights
            .lookup(0, &first_snapshot)
            .and_then(|row| row.ranges())
            .expect("first ranges")
            .clone();

        let second_highlights = precompute_terminal_keyword_highlights(
            &second_snapshot,
            &highlighter,
            palette,
            Some(&first_highlights),
        );
        let second_ranges = second_highlights
            .lookup(5, &second_snapshot)
            .and_then(|row| row.ranges())
            .expect("second ranges");

        assert!(Arc::ptr_eq(&first_ranges, second_ranges));
    }

    #[test]
    fn precomputed_keyword_snapshot_matches_across_wrapped_rows() {
        let screen = TerminalScreen::new(3, 4);
        let mut snapshot = screen.snapshot();
        set_snapshot_row_wrapped(&mut snapshot, 0, "ERR", 41, false);
        set_snapshot_row_wrapped(&mut snapshot, 1, "OR", 42, true);
        let rules = vec![ResolvedKeywordHighlightRule {
            id: "error".to_string(),
            name: "Error".to_string(),
            patterns: vec!["ERROR".to_string()],
            color: "#ff2244".to_string(),
            enabled: true,
        }];

        let highlighter = compile_terminal_keyword_highlighter(&rules);
        let highlights = precompute_terminal_keyword_highlights(
            &snapshot,
            &highlighter,
            nyaterm_ui::theme_palette("github-dark"),
            None,
        );

        let first = highlights
            .lookup(0, &snapshot)
            .and_then(|row| row.ranges())
            .expect("first wrapped row ranges");
        let second = highlights
            .lookup(1, &snapshot)
            .and_then(|row| row.ranges())
            .expect("second wrapped row ranges");

        assert_eq!(
            first.as_ref(),
            &[TerminalKeywordRange {
                start_col: 0,
                end_col: 3,
                color: 0xff2244,
            }]
        );
        assert_eq!(
            second.as_ref(),
            &[TerminalKeywordRange {
                start_col: 0,
                end_col: 2,
                color: 0xff2244,
            }]
        );
    }

    #[test]
    fn precomputed_keyword_snapshot_maps_wide_and_attached_marks_to_cells() {
        let mut snapshot = TerminalScreen::default().snapshot();
        set_snapshot_row(&mut snapshot, 0, "界ERROR", 41);
        set_snapshot_row(&mut snapshot, 1, "e\u{301}ERROR", 42);
        set_snapshot_row(&mut snapshot, 2, "a\u{fe0f}ERROR", 43);
        let rules = vec![ResolvedKeywordHighlightRule {
            id: "error".to_string(),
            name: "Error".to_string(),
            patterns: vec!["ERROR".to_string()],
            color: "#ff2244".to_string(),
            enabled: true,
        }];

        let highlighter = compile_terminal_keyword_highlighter(&rules);
        let highlights = precompute_terminal_keyword_highlights(
            &snapshot,
            &highlighter,
            nyaterm_ui::theme_palette("github-dark"),
            None,
        );

        let wide = highlights
            .lookup(0, &snapshot)
            .and_then(|row| row.ranges())
            .expect("wide row ranges");
        let combining = highlights
            .lookup(1, &snapshot)
            .and_then(|row| row.ranges())
            .expect("combining row ranges");
        let variation = highlights
            .lookup(2, &snapshot)
            .and_then(|row| row.ranges())
            .expect("variation row ranges");

        assert_eq!(wide[0].start_col, 2);
        assert_eq!(wide[0].end_col, 7);
        assert_eq!(combining[0].start_col, 1);
        assert_eq!(combining[0].end_col, 6);
        assert_eq!(variation[0].start_col, 1);
        assert_eq!(variation[0].end_col, 6);
    }

    #[test]
    fn wrapped_keyword_snapshot_does_not_reuse_plain_row_context() {
        let mut wrapped = TerminalScreen::new(3, 4).snapshot();
        set_snapshot_row_wrapped(&mut wrapped, 0, "ERR", 41, false);
        set_snapshot_row_wrapped(&mut wrapped, 1, "OR", 42, true);
        let mut plain = TerminalScreen::new(3, 4).snapshot();
        set_snapshot_row_wrapped(&mut plain, 0, "ERR", 41, false);
        set_snapshot_row_wrapped(&mut plain, 1, "OR", 42, false);
        let rules = vec![ResolvedKeywordHighlightRule {
            id: "error".to_string(),
            name: "Error".to_string(),
            patterns: vec!["ERROR".to_string()],
            color: "#ff2244".to_string(),
            enabled: true,
        }];

        let highlighter = compile_terminal_keyword_highlighter(&rules);
        let palette = nyaterm_ui::theme_palette("github-dark");
        let wrapped_highlights =
            precompute_terminal_keyword_highlights(&wrapped, &highlighter, palette, None);

        assert!(wrapped_highlights.lookup(0, &plain).is_none());

        let plain_highlights = precompute_terminal_keyword_highlights(
            &plain,
            &highlighter,
            palette,
            Some(&wrapped_highlights),
        );
        assert!(
            plain_highlights
                .lookup(0, &plain)
                .expect("plain row known")
                .ranges()
                .is_none()
        );
    }

    #[test]
    fn terminal_buffer_matches_count_combining_marks_with_previous_cell() {
        let flags = TerminalSearchFlags {
            case_sensitive: true,
            regex: false,
            whole_word: false,
        };

        let matches = terminal_buffer_matches("e\u{301}x", "x", &flags, 10).unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].start_col, 1);
        assert_eq!(matches[0].end_col, 2);
    }

    #[test]
    fn terminal_buffer_matches_count_wide_chars_as_two_cells() {
        let flags = TerminalSearchFlags {
            case_sensitive: true,
            regex: false,
            whole_word: false,
        };

        let matches = terminal_buffer_matches("界x", "x", &flags, 10).unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].start_col, 2);
        assert_eq!(matches[0].end_col, 3);
    }

    #[test]
    fn regex_terminal_buffer_matches_count_combining_marks_with_previous_cell() {
        let flags = TerminalSearchFlags {
            case_sensitive: true,
            regex: true,
            whole_word: false,
        };

        let matches = terminal_buffer_matches("e\u{301}x", "x", &flags, 10).unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].start_col, 1);
        assert_eq!(matches[0].end_col, 2);
    }

    #[test]
    fn regex_terminal_buffer_matches_count_wide_chars_as_two_cells() {
        let flags = TerminalSearchFlags {
            case_sensitive: true,
            regex: true,
            whole_word: false,
        };

        let matches = terminal_buffer_matches("界x", "x", &flags, 10).unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].start_col, 2);
        assert_eq!(matches[0].end_col, 3);
    }
}

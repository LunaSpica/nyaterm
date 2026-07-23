use super::*;
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

pub(super) type CompiledKeywordRules = Vec<(regex::Regex, u32)>;

pub struct TerminalKeywordHighlighter {
    rules_key: u64,
    compiled: CompiledKeywordRules,
}

/// Immutable keyword data prepared away from GPUI's paint path.
pub struct TerminalKeywordHighlightSnapshot {
    rules_key: u64,
    display_offset: usize,
    line_signatures: Vec<u64>,
    rows: Vec<Option<Arc<Vec<TerminalHighlightSpan>>>>,
    rows_by_signature: HashMap<u64, Arc<Vec<TerminalHighlightSpan>>>,
}

impl TerminalKeywordHighlightSnapshot {
    pub(super) fn rules_key(&self) -> u64 {
        self.rules_key
    }

    pub(super) fn row(
        &self,
        row: usize,
        line_signature: Option<u64>,
    ) -> Option<&Arc<Vec<TerminalHighlightSpan>>> {
        let signature = line_signature?;
        if self.line_signatures.get(row).copied() == Some(signature) {
            return self.rows.get(row)?.as_ref();
        }
        self.rows_by_signature.get(&signature)
    }

    pub(super) fn stale_row(
        &self,
        row: usize,
        display_offset: usize,
        current_row_count: usize,
    ) -> Option<&Arc<Vec<TerminalHighlightSpan>>> {
        if self.display_offset != display_offset || self.line_signatures.len() != current_row_count
        {
            return None;
        }
        self.rows.get(row)?.as_ref()
    }
}

pub fn precompute_terminal_keyword_highlights(
    snapshot: &TerminalSnapshot,
    highlighter: &TerminalKeywordHighlighter,
    palette: nyaterm_ui::ThemePalette,
) -> TerminalKeywordHighlightSnapshot {
    let rows: Vec<Option<Arc<Vec<TerminalHighlightSpan>>>> = snapshot
        .lines
        .iter()
        .enumerate()
        .map(|(row, line)| {
            let display_line = if line.is_empty() { " " } else { line };
            let ansi = snapshot.styled_lines.get(row).map(Vec::as_slice);
            let spans = terminal_highlight_spans_compiled(
                display_line,
                ansi,
                &highlighter.compiled,
                &[],
                &[],
                None,
                &[],
                palette,
            );
            spans
                .iter()
                .any(|span| span.keyword)
                .then(|| Arc::new(spans))
        })
        .collect();
    let rows_by_signature = snapshot
        .line_signatures
        .iter()
        .copied()
        .zip(rows.iter())
        .filter_map(|(signature, spans)| spans.as_ref().map(|spans| (signature, spans.clone())))
        .collect();
    TerminalKeywordHighlightSnapshot {
        rules_key: highlighter.rules_key,
        display_offset: snapshot.display_offset,
        line_signatures: snapshot.line_signatures.clone(),
        rows,
        rows_by_signature,
    }
}

pub fn compile_terminal_keyword_highlighter(
    rules: &[ResolvedKeywordHighlightRule],
) -> TerminalKeywordHighlighter {
    TerminalKeywordHighlighter {
        rules_key: terminal_keyword_rules_key(rules),
        compiled: compile_keyword_rules(rules),
    }
}

pub(super) fn terminal_keyword_rules_key(rules: &[ResolvedKeywordHighlightRule]) -> u64 {
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
        snapshot.lines[0] = "prefix ERROR suffix".to_string();
        snapshot.styled_lines[0] = vec![nyaterm_terminal::StyledSpan {
            text: snapshot.lines[0].clone(),
            style: nyaterm_terminal::CellStyle::default(),
        }];
        snapshot.line_signatures[0] = 41;
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
        );

        assert!(highlights.row(0, Some(41)).is_some());
        assert!(highlights.row(0, Some(42)).is_none());
        assert!(highlights.stale_row(0, 0, snapshot.rows).is_some());
        assert!(highlights.stale_row(0, 1, snapshot.rows).is_none());
        assert!(highlights.rows.iter().skip(1).all(Option::is_none));
    }

    #[test]
    fn precomputed_keyword_snapshot_reuses_matching_rows_after_scroll() {
        let mut snapshot = TerminalScreen::default().snapshot();
        snapshot.lines[0] = "prefix ERROR suffix".to_string();
        snapshot.styled_lines[0] = vec![nyaterm_terminal::StyledSpan {
            text: snapshot.lines[0].clone(),
            style: nyaterm_terminal::CellStyle::default(),
        }];
        snapshot.line_signatures[0] = 41;
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
        );

        assert!(highlights.row(0, Some(41)).is_some());
        assert!(highlights.row(3, Some(41)).is_some());
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

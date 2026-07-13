use super::*;

pub(super) fn keyword_highlight_spans(
    line: &str,
    rules: &[ResolvedKeywordHighlightRule],
) -> Vec<TerminalHighlightSpan> {
    if rules.is_empty() || line.is_empty() {
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

    let compiled = compile_keyword_rules(rules);
    if compiled.is_empty() {
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
        for (regex, color) in &compiled {
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
) -> Vec<(regex::Regex, u32)> {
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
                let start_col = line[..found.start()].chars().count();
                let end_col = line[..found.end()].chars().count();
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
                let start_col = line[..start.min(line.len())].chars().count();
                let end_col = line[..end.min(line.len())].chars().count();
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

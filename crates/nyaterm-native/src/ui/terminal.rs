use gpui::{FontWeight, IntoElement, KeyDownEvent, div, prelude::*, px, rgb};
use nyaterm_domain::ResolvedKeywordHighlightRule;
use nyaterm_terminal::TerminalScreen;

use super::view::INITIAL_TERMINAL_BANNER;

#[derive(Debug, Clone)]
pub(super) struct TerminalBufferMatch {
    pub(super) line_index: usize,
    /// Half-open character column range on the matched line.
    pub(super) start_col: usize,
    pub(super) end_col: usize,
}

#[derive(Debug, Clone)]
pub(super) struct TerminalSearchFlags {
    pub(super) case_sensitive: bool,
    pub(super) regex: bool,
    pub(super) whole_word: bool,
}

struct TerminalHighlightSpan {
    text: String,
    color: Option<u32>,
    bg: Option<u32>,
    keyword: bool,
    underline: bool,
    bold: bool,
}

pub(super) fn terminal_line_element(
    line: &str,
    ansi_spans: Option<&[nyaterm_terminal::StyledSpan]>,
    keyword_rules: &[ResolvedKeywordHighlightRule],
    search_ranges: &[(usize, usize)],
    active_search_ranges: &[(usize, usize)],
    cursor_col: Option<usize>,
    cursor_style: &str,
    // Half-open column range selected on this line, if any.
    selection_cols: Option<(usize, usize)>,
    // Half-open character ranges for action-link underlines.
    link_ranges: &[(usize, usize)],
    line_height: f32,
    palette: crate::ui::theme::ThemePalette,
    bold_weight: f32,
) -> impl IntoElement {
    let mut spans = if let Some(ansi) = ansi_spans {
        if ansi.is_empty() || (ansi.len() == 1 && ansi[0].text.is_empty()) {
            keyword_highlight_spans(line, keyword_rules)
        } else {
            ansi_to_highlight_spans(ansi, palette, keyword_rules)
        }
    } else {
        keyword_highlight_spans(line, keyword_rules)
    };
    if !link_ranges.is_empty() {
        spans = apply_action_link_ranges(spans, link_ranges, palette);
    }
    if let Some((start, end)) = selection_cols {
        spans = apply_selection_range(spans, start, end, palette);
    }
    if let Some(col) = cursor_col {
        spans = apply_cursor_style(spans, col, cursor_style, palette);
    }
    let line_h = px(line_height.max(12.));
    if !search_ranges.is_empty() {
        spans = apply_search_ranges(spans, search_ranges, false, palette);
    }
    if !active_search_ranges.is_empty() {
        spans = apply_search_ranges(spans, active_search_ranges, true, palette);
    }

    let mut row = div()
        .flex()
        .flex_row()
        .items_center()
        .min_h(line_h)
        .line_height(line_h)
        .whitespace_nowrap();
    // Active find row gets a subtle left marker (xterm active decoration cue).
    if !active_search_ranges.is_empty() {
        row = row.border_l_2().border_color(rgb(palette.warning));
    }

    for span in spans {
        let mut child = div()
            .line_height(line_h)
            .whitespace_nowrap()
            .child(if span.text.is_empty() {
                " ".to_string()
            } else {
                span.text
            });
        if let Some(color) = span.color {
            child = child.text_color(rgb(color));
        }
        if let Some(bg) = span.bg {
            child = child.bg(rgb(bg));
        } else if span.keyword {
            child = child.bg(rgb(palette.surface));
        }
        if span.underline {
            child = child.underline();
        }
        if span.bold {
            child = child.font_weight(FontWeight(bold_weight));
        }
        row = row.child(child);
    }

    row
}


/// Underline action-link ranges with the accent color (Tauri decoration look).
fn apply_action_link_ranges(
    spans: Vec<TerminalHighlightSpan>,
    ranges: &[(usize, usize)],
    palette: crate::ui::theme::ThemePalette,
) -> Vec<TerminalHighlightSpan> {
    if ranges.is_empty() {
        return spans;
    }
    let mut flat: Vec<(char, Option<u32>, Option<u32>, bool, bool, bool)> = Vec::new();
    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        for ch in span.text.chars() {
            flat.push((ch, span.color, span.bg, span.keyword, span.underline, span.bold));
        }
    }
    for &(start, end) in ranges {
        if start >= end {
            continue;
        }
        let end = end.min(flat.len());
        let start = start.min(end);
        for idx in start..end {
            if let Some(cell) = flat.get_mut(idx) {
                cell.4 = true; // underline
                if cell.1.is_none() {
                    cell.1 = Some(palette.accent);
                }
            }
        }
    }
    compress_flat_cells(flat)
}

/// Highlight a half-open [start, end) column range with the theme selection background.
fn apply_selection_range(
    spans: Vec<TerminalHighlightSpan>,
    start: usize,
    end: usize,
    palette: crate::ui::theme::ThemePalette,
) -> Vec<TerminalHighlightSpan> {
    if start >= end {
        return spans;
    }
    let mut flat: Vec<(char, Option<u32>, Option<u32>, bool, bool, bool)> = Vec::new();
    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        for ch in span.text.chars() {
            flat.push((ch, span.color, span.bg, span.keyword, span.underline, span.bold));
        }
    }
    while flat.len() < end {
        flat.push((' ', None, None, false, false, false));
    }
    let end = end.min(flat.len());
    for idx in start..end {
        if let Some(cell) = flat.get_mut(idx) {
            cell.2 = Some(palette.terminal_selection);
            cell.3 = false;
        }
    }
    compress_flat_cells(flat)
}

fn apply_search_ranges(
    spans: Vec<TerminalHighlightSpan>,
    ranges: &[(usize, usize)],
    active: bool,
    palette: crate::ui::theme::ThemePalette,
) -> Vec<TerminalHighlightSpan> {
    if ranges.is_empty() {
        return spans;
    }
    let mut flat: Vec<(char, Option<u32>, Option<u32>, bool, bool, bool)> = Vec::new();
    for span in spans {
        if span.text.is_empty() {
            continue;
        }
        for ch in span.text.chars() {
            flat.push((ch, span.color, span.bg, span.keyword, span.underline, span.bold));
        }
    }
    let max_end = ranges.iter().map(|(_, end)| *end).max().unwrap_or(0);
    while flat.len() < max_end {
        flat.push((' ', None, None, false, false, false));
    }
    // Tauri xterm find decorations: inactive selection-ish, active stronger accent.
    let bg = if active {
        // Mix selection with warning accent by using warning-ish selection.
        palette.warning
    } else {
        palette.terminal_selection
    };
    let fg = if active {
        Some(palette.terminal_bg)
    } else {
        None
    };
    for &(start, end) in ranges {
        if start >= end {
            continue;
        }
        let end = end.min(flat.len());
        for idx in start..end {
            if let Some(cell) = flat.get_mut(idx) {
                cell.2 = Some(bg);
                if let Some(fg) = fg {
                    cell.1 = Some(fg);
                }
                cell.3 = false;
            }
        }
    }
    compress_flat_cells(flat)
}

fn compress_flat_cells(
    flat: Vec<(char, Option<u32>, Option<u32>, bool, bool, bool)>,
) -> Vec<TerminalHighlightSpan> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < flat.len() {
        let (ch, color, bg, keyword, underline, bold) = flat[i];
        let mut text = String::new();
        text.push(ch);
        let mut j = i + 1;
        while j < flat.len() {
            let (ch2, c2, b2, k2, u2, bold2) = flat[j];
            if c2 == color && b2 == bg && k2 == keyword && u2 == underline && bold2 == bold {
                text.push(ch2);
                j += 1;
            } else {
                break;
            }
        }
        out.push(TerminalHighlightSpan {
            text,
            color,
            bg,
            keyword,
            underline,
            bold,
        });
        i = j;
    }
    out
}

/// Paint a caret at `cursor_col` (char index) using Tauri cursor styles.
fn apply_cursor_style(
    spans: Vec<TerminalHighlightSpan>,
    cursor_col: usize,
    cursor_style: &str,
    palette: crate::ui::theme::ThemePalette,
) -> Vec<TerminalHighlightSpan> {
    let mut flat: Vec<(char, Option<u32>, Option<u32>, bool, bool, bool)> = Vec::new();
    for span in spans {
        let color = span.color;
        let bg = span.bg;
        let keyword = span.keyword;
        let underline = span.underline;
        let bold = span.bold;
        if span.text.is_empty() {
            continue;
        }
        for ch in span.text.chars() {
            flat.push((ch, color, bg, keyword, underline, bold));
        }
    }
    // Ensure the cursor column exists even on a short/empty line.
    while flat.len() <= cursor_col {
        flat.push((' ', None, None, false, false, false));
    }
    if let Some(cell) = flat.get_mut(cursor_col) {
        match cursor_style {
            "underline" => {
                // Approximate underline caret: keep glyph, tint with cursor color and dim cell bg.
                if cell.1.is_none() {
                    cell.1 = Some(palette.terminal_cursor);
                }
                cell.2 = Some(palette.terminal_selection);
                cell.3 = false;
            }
            "bar" => {
                // Approximate bar caret: thin visual via inverted narrow space marker.
                cell.0 = '▌';
                cell.1 = Some(palette.terminal_cursor);
                cell.2 = None;
                cell.3 = false;
            }
            _ => {
                // Block cursor: invert with theme cursor color (Tauri xterm cursor).
                cell.1 = Some(palette.terminal_bg);
                cell.2 = Some(palette.terminal_cursor);
                cell.3 = false;
            }
        }
    }

    compress_flat_cells(flat)
}

fn ansi_to_highlight_spans(
    ansi: &[nyaterm_terminal::StyledSpan],
    palette: crate::ui::theme::ThemePalette,
    keyword_rules: &[ResolvedKeywordHighlightRule],
) -> Vec<TerminalHighlightSpan> {
    // Build plain line for keyword overlay, then prefer keyword fg over default ANSI fg.
    let line: String = ansi.iter().map(|s| s.text.as_str()).collect();
    let keyword = keyword_highlight_spans(&line, keyword_rules);
    if keyword.iter().all(|s| !s.keyword) {
        return ansi
            .iter()
            .filter(|s| !s.text.is_empty())
            .map(|s| TerminalHighlightSpan {
                text: s.text.clone(),
                color: Some(palette.resolve_cell_fg(s.style)),
                bg: palette.resolve_cell_bg(s.style),
                keyword: false,
                underline: s.style.underline,
                bold: s.style.bold,
            })
            .collect();
    }

    // Flatten keyword color map by byte offset, then re-slice per ANSI span.
    let mut keyword_color_at = vec![None; line.len()];
    let mut offset = 0usize;
    for span in &keyword {
        let end = offset + span.text.len();
        if span.keyword {
            for idx in offset..end.min(keyword_color_at.len()) {
                keyword_color_at[idx] = span.color;
            }
        }
        offset = end;
    }

    let mut out = Vec::new();
    let mut cursor = 0usize;
    for s in ansi {
        if s.text.is_empty() {
            continue;
        }
        let start = cursor;
        let end = cursor + s.text.len();
        cursor = end;
        let bg = palette.resolve_cell_bg(s.style);
        let mut color = palette.resolve_cell_fg(s.style);
        let mut keyword_hit = false;
        if s.style.fg.is_none() {
            if let Some(kc) = keyword_color_at.get(start).copied().flatten() {
                color = kc;
                keyword_hit = true;
            }
        }
        out.push(TerminalHighlightSpan {
            text: s.text.clone(),
            color: Some(color),
            bg,
            keyword: keyword_hit,
            underline: s.style.underline,
            bold: s.style.bold,
        });
    }
    if out.is_empty() {
        out.push(TerminalHighlightSpan {
            text: " ".to_string(),
            color: None,
            bg: None,
            keyword: false,
            underline: false,
            bold: false,
        });
    }
    out
}

fn keyword_highlight_spans(
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
            bold: false,
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
            bold: false,
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
                bold: false,
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
                bold: false,
            });
        }
        spans.push(TerminalHighlightSpan {
            text: line[start..end].to_string(),
            color: Some(color),
            bg: None,
            keyword: true,
            underline: false,
            bold: false,
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
            bold: false,
        });
    }
    spans
}

fn compile_keyword_rules(
    rules: &[ResolvedKeywordHighlightRule],
) -> Vec<(regex::Regex, u32)> {
    let mut compiled = Vec::new();
    for rule in rules.iter().filter(|rule| rule.enabled) {
        let color = parse_hex_rgb(&rule.color).unwrap_or(0x79c0ff);
        let mut alts = Vec::new();
        for pattern in rule.patterns.iter().map(|p| p.trim()).filter(|p| !p.is_empty()) {
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



pub(super) fn terminal_buffer_matches(
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

fn is_whole_word_match(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    !before.is_some_and(is_word_char) && !after.is_some_and(is_word_char)
}

fn is_word_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn parse_hex_rgb(value: &str) -> Option<u32> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if hex.len() != 6 {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

pub(super) fn terminal_key_bytes(event: &KeyDownEvent) -> Option<Vec<u8>> {
    let keystroke = &event.keystroke;
    if keystroke.modifiers.function {
        return None;
    }
    // Super/Win key combos are reserved for the shell/OS.
    if keystroke.modifiers.platform && !keystroke.modifiers.control && !keystroke.modifiers.alt {
        return None;
    }

    let key = keystroke.key.as_str();
    let ctrl = keystroke.modifiers.control;
    let alt = keystroke.modifiers.alt;
    let shift = keystroke.modifiers.shift;

    // Ctrl+Arrow / Alt+Arrow CSI sequences (Tauri XTerminal word-nav parity).
    if matches!(key, "up" | "down" | "left" | "right") {
        if ctrl && !alt && !shift {
            let suffix = match key {
                "up" => b"\x1b[1;5A",
                "down" => b"\x1b[1;5B",
                "right" => b"\x1b[1;5C",
                "left" => b"\x1b[1;5D",
                _ => unreachable!(),
            };
            return Some(suffix.to_vec());
        }
        if alt && !ctrl && !shift {
            let suffix = match key {
                "up" => b"\x1b[1;3A",
                "down" => b"\x1b[1;3B",
                "right" => b"\x1b[1;3C",
                "left" => b"\x1b[1;3D",
                _ => unreachable!(),
            };
            return Some(suffix.to_vec());
        }
    }

    if ctrl && !alt {
        return control_key_bytes(key);
    }

    // Plain navigation / editing keys (no modifiers other than shift where irrelevant).
    if !ctrl && !alt && !keystroke.modifiers.platform {
        match key {
            "enter" => return Some(b"\r".to_vec()),
            "backspace" => return Some(vec![0x7f]),
            "tab" => return Some(b"\t".to_vec()),
            "escape" => return Some(vec![0x1b]),
            "up" => return Some(b"\x1b[A".to_vec()),
            "down" => return Some(b"\x1b[B".to_vec()),
            "right" => return Some(b"\x1b[C".to_vec()),
            "left" => return Some(b"\x1b[D".to_vec()),
            "home" => return Some(b"\x1b[H".to_vec()),
            "end" => return Some(b"\x1b[F".to_vec()),
            "delete" => return Some(b"\x1b[3~".to_vec()),
            "pageup" => return Some(b"\x1b[5~".to_vec()),
            "pagedown" => return Some(b"\x1b[6~".to_vec()),
            _ => {}
        }

        return keystroke
            .key_char
            .as_deref()
            .filter(|value| !value.is_empty())
            .map(|value| value.as_bytes().to_vec());
    }

    None
}

fn control_key_bytes(key: &str) -> Option<Vec<u8>> {
    // Ctrl+Arrow handled above.
    if matches!(key, "up" | "down" | "left" | "right") {
        return None;
    }
    let byte = match key {
        "space" => 0x00,
        "left_bracket" | "[" => 0x1b,
        "backslash" | "\\" => 0x1c,
        "right_bracket" | "]" => 0x1d,
        "6" => 0x1e,
        "slash" | "/" => 0x1f,
        value if value.len() == 1 => {
            let byte = value.as_bytes()[0].to_ascii_lowercase();
            if byte.is_ascii_lowercase() {
                byte - b'a' + 1
            } else {
                return None;
            }
        }
        _ => return None,
    };
    Some(vec![byte])
}

pub(super) fn trim_terminal_output(output: &mut String) {
    const MAX_BYTES: usize = 64 * 1024;
    if output.len() <= MAX_BYTES {
        return;
    }
    let drain_to = output
        .char_indices()
        .find_map(|(index, _)| (index >= output.len() - MAX_BYTES).then_some(index))
        .unwrap_or(0);
    output.drain(..drain_to);
}

pub(super) fn initial_terminal_screen() -> TerminalScreen {
    terminal_screen_from_output(INITIAL_TERMINAL_BANNER)
}

pub(super) fn terminal_screen_from_output(output: &str) -> TerminalScreen {
    let mut screen = TerminalScreen::default();
    screen.advance(output.as_bytes());
    screen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buffer_matches_report_column_ranges() {
        let output = "hello world\nfoo hello bar";
        let matches = terminal_buffer_matches(
            output,
            "hello",
            &TerminalSearchFlags {
                case_sensitive: false,
                regex: false,
                whole_word: false,
            },
            10,
        )
        .expect("matches");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line_index, 0);
        assert_eq!(matches[0].start_col, 0);
        assert_eq!(matches[0].end_col, 5);
        assert_eq!(matches[1].line_index, 1);
        assert_eq!(matches[1].start_col, 4);
        assert_eq!(matches[1].end_col, 9);
    }
}

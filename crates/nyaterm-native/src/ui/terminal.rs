use gpui::{IntoElement, KeyDownEvent, div, prelude::*, px, rgb};
use nyaterm_domain::KeywordHighlightConfig;
use nyaterm_terminal::TerminalScreen;

use super::view::INITIAL_TERMINAL_BANNER;

#[derive(Debug, Clone)]
pub(super) struct TerminalBufferMatch {
    pub(super) line_index: usize,
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
}

pub(super) fn terminal_line_element(
    line: &str,
    config: &KeywordHighlightConfig,
    search_match: bool,
    active_search_match: bool,
    palette: crate::ui::theme::ThemePalette,
) -> impl IntoElement {
    let spans = keyword_highlight_spans(line, config);
    let mut row = div()
        .flex()
        .flex_row()
        .items_center()
        .min_h(px(18.))
        .line_height(px(18.))
        .whitespace_nowrap();
    if active_search_match {
        row = row
            .bg(rgb(palette.hover))
            .border_l_2()
            .border_color(rgb(palette.warning));
    } else if search_match {
        row = row.bg(rgb(palette.surface));
    }

    for span in spans {
        let mut child = div()
            .line_height(px(18.))
            .whitespace_nowrap()
            .child(span.text);
        if let Some(color) = span.color {
            child = child.text_color(rgb(color)).bg(rgb(palette.surface));
        }
        row = row.child(child);
    }

    row
}

fn keyword_highlight_spans(
    line: &str,
    config: &KeywordHighlightConfig,
) -> Vec<TerminalHighlightSpan> {
    if !config.enabled || config.rules.is_empty() || line.is_empty() {
        return vec![TerminalHighlightSpan {
            text: line.to_string(),
            color: None,
        }];
    }

    let lowered = line.to_ascii_lowercase();
    let mut spans = Vec::new();
    let mut cursor = 0;
    while cursor < line.len() {
        let mut best: Option<(usize, usize, u32)> = None;
        for rule in config.rules.iter().filter(|rule| rule.enabled) {
            let color = parse_hex_rgb(&rule.color_dark).unwrap_or(0x79c0ff);
            for pattern in rule.patterns.iter().map(|pattern| pattern.trim()) {
                if pattern.is_empty() {
                    continue;
                }
                let needle = pattern.to_ascii_lowercase();
                if let Some(relative_start) = lowered[cursor..].find(&needle) {
                    let start = cursor + relative_start;
                    let end = start + needle.len();
                    let replace = best
                        .map(|(best_start, best_end, _)| {
                            start < best_start || (start == best_start && end > best_end)
                        })
                        .unwrap_or(true);
                    if replace {
                        best = Some((start, end, color));
                    }
                }
            }
        }

        let Some((start, end, color)) = best else {
            spans.push(TerminalHighlightSpan {
                text: line[cursor..].to_string(),
                color: None,
            });
            break;
        };
        if start > cursor {
            spans.push(TerminalHighlightSpan {
                text: line[cursor..start].to_string(),
                color: None,
            });
        }
        spans.push(TerminalHighlightSpan {
            text: line[start..end].to_string(),
            color: Some(color),
        });
        cursor = end;
    }

    if spans.is_empty() {
        spans.push(TerminalHighlightSpan {
            text: " ".to_string(),
            color: None,
        });
    }
    spans
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
            if regex.find_iter(line).any(|found| {
                !flags.whole_word || is_whole_word_match(line, found.start(), found.end())
            }) {
                matches.push(TerminalBufferMatch { line_index });
                if matches.len() >= limit {
                    break;
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
        let mut matched = false;
        while cursor <= haystack.len() {
            let Some(relative_start) = haystack[cursor..].find(&needle) else {
                break;
            };
            let start = cursor + relative_start;
            let end = start + needle.len();
            if !flags.whole_word || is_whole_word_match(line, start, end) {
                matched = true;
                break;
            }
            cursor = end.max(cursor + 1);
        }
        if matched {
            matches.push(TerminalBufferMatch { line_index });
            if matches.len() >= limit {
                break;
            }
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
    if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.function {
        return None;
    }

    if keystroke.modifiers.control {
        return control_key_bytes(&keystroke.key);
    }

    match keystroke.key.as_str() {
        "enter" => return Some(b"\n".to_vec()),
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

    keystroke
        .key_char
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(|value| value.as_bytes().to_vec())
}

fn control_key_bytes(key: &str) -> Option<Vec<u8>> {
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

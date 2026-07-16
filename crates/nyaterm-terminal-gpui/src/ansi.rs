use super::*;

pub(super) fn ansi_to_highlight_spans_compiled(
    ansi: &[nyaterm_terminal::StyledSpan],
    palette: nyaterm_ui::ThemePalette,
    compiled_keyword_rules: &[(regex::Regex, u32)],
) -> Vec<TerminalHighlightSpan> {
    // Build plain line for keyword overlay, then prefer keyword fg over default ANSI fg.
    let line: String = ansi.iter().map(|s| s.text.as_str()).collect();
    let keyword = keyword_highlight_spans_compiled(&line, compiled_keyword_rules);
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
                strikeout: s.style.strikeout,
                bold: s.style.bold,
                italic: s.style.italic,
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
            strikeout: s.style.strikeout,
            bold: s.style.bold,
            italic: s.style.italic,
        });
    }
    if out.is_empty() {
        out.push(TerminalHighlightSpan {
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
    out
}

use super::*;
use crate::models::{TerminalFrameActionLinks, TerminalSelection};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub(in crate::features) fn terminal_snapshot_absolute_range(
    snapshot: &nyaterm_terminal::TerminalSnapshot,
) -> (usize, usize) {
    let end = snapshot.total_rows.saturating_sub(snapshot.display_offset);
    let start = end.saturating_sub(snapshot.rows);
    (start, end)
}

pub(in crate::features) fn terminal_line_decorations_cache_key(
    snapshot: &nyaterm_terminal::TerminalSnapshot,
    selection: Option<TerminalSelection>,
    selection_viewport_anchor_row: usize,
    search_ranges_by_line: &HashMap<usize, Vec<(usize, usize)>>,
    active_search_ranges_by_line: &HashMap<usize, Vec<(usize, usize)>>,
    frame_action_links: Option<&TerminalFrameActionLinks>,
    include_action_links: bool,
    include_hyperlinks: bool,
    include_command_marks: bool,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    snapshot.rows.hash(&mut hasher);
    snapshot.cols.hash(&mut hasher);
    snapshot.display_offset.hash(&mut hasher);
    snapshot.line_signatures.hash(&mut hasher);
    selection.hash(&mut hasher);
    selection_viewport_anchor_row.hash(&mut hasher);
    include_action_links.hash(&mut hasher);
    include_hyperlinks.hash(&mut hasher);
    include_command_marks.hash(&mut hasher);
    hash_ranges_by_line(search_ranges_by_line, &mut hasher);
    hash_ranges_by_line(active_search_ranges_by_line, &mut hasher);
    if include_action_links {
        if let Some(links) = frame_action_links {
            links.matcher_key.hash(&mut hasher);
            links.cell_ranges_by_line.hash(&mut hasher);
        } else {
            0u64.hash(&mut hasher);
        }
    }
    if include_hyperlinks {
        snapshot.hyperlink_lines.len().hash(&mut hasher);
        for spans in &snapshot.hyperlink_lines {
            spans.len().hash(&mut hasher);
            for span in spans {
                span.start_col.hash(&mut hasher);
                span.end_col.hash(&mut hasher);
                span.uri.hash(&mut hasher);
            }
        }
    }
    if include_command_marks {
        snapshot.command_marks.hash(&mut hasher);
    }
    hasher.finish()
}

fn hash_ranges_by_line<H: Hasher>(
    ranges_by_line: &HashMap<usize, Vec<(usize, usize)>>,
    hasher: &mut H,
) {
    let mut lines = ranges_by_line.keys().copied().collect::<Vec<_>>();
    lines.sort_unstable();
    lines.len().hash(hasher);
    for line in lines {
        line.hash(hasher);
        ranges_by_line
            .get(&line)
            .map(Vec::as_slice)
            .unwrap_or(&[])
            .hash(hasher);
    }
}

pub(in crate::features) fn build_terminal_line_decorations(
    snapshot: &nyaterm_terminal::TerminalSnapshot,
    selection: Option<TerminalSelection>,
    selection_viewport_anchor_row: usize,
    search_ranges_by_line: &HashMap<usize, Vec<(usize, usize)>>,
    active_search_ranges_by_line: &HashMap<usize, Vec<(usize, usize)>>,
    frame_action_links: Option<&TerminalFrameActionLinks>,
    include_action_links: bool,
    include_hyperlinks: bool,
    include_command_marks: bool,
) -> Vec<TerminalLineDecorations> {
    let line_count = snapshot.lines.len();
    let mut line_decorations = Vec::with_capacity(line_count);
    let empty_ranges: [(usize, usize); 0] = [];
    for line_index in 0..line_count {
        let selection_cols = line_index
            .checked_sub(selection_viewport_anchor_row)
            .and_then(|viewport_row| {
                selection.and_then(|selection| selection.cols_for_row(viewport_row))
            });
        let mut link_ranges: Vec<(usize, usize)> = if include_action_links {
            frame_action_links
                .and_then(|links| links.cell_ranges_by_line.get(line_index))
                .cloned()
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        if include_hyperlinks && let Some(spans) = snapshot.hyperlink_lines.get(line_index) {
            for span in spans {
                let start = span.start_col;
                let end = span.end_col.saturating_add(1);
                if end > start {
                    link_ranges.push((start, end));
                }
            }
        }
        let line_search_ranges = search_ranges_by_line
            .get(&line_index)
            .map(|ranges| ranges.as_slice())
            .unwrap_or(&empty_ranges);
        let line_active_search_ranges = active_search_ranges_by_line
            .get(&line_index)
            .map(|ranges| ranges.as_slice())
            .unwrap_or(&empty_ranges);
        let command_mark = include_command_marks
            .then(|| snapshot.command_marks.get(line_index).copied().flatten())
            .flatten();
        line_decorations.push(TerminalLineDecorations {
            search_ranges: line_search_ranges.to_vec(),
            active_search_ranges: line_active_search_ranges.to_vec(),
            selection_cols,
            link_ranges,
            command_mark,
        });
    }
    line_decorations
}

pub(in crate::features) fn terminal_line_decorations_needed(
    has_selection: bool,
    has_search_decorations: bool,
    has_frame_action_links: bool,
    has_hyperlinks: bool,
    has_command_marks: bool,
) -> bool {
    has_selection
        || has_search_decorations
        || has_frame_action_links
        || has_hyperlinks
        || has_command_marks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::TerminalCellPos;

    #[test]
    fn selection_decorations_map_viewport_rows_through_snapshot_anchor() {
        let snapshot = nyaterm_terminal::TerminalScreen::default().viewport_snapshot(0);
        let selection = TerminalSelection::from_range(
            TerminalCellPos::new(0, 2),
            TerminalCellPos::new(1, 4),
            0,
            2,
        );

        let decorations = build_terminal_line_decorations(
            &snapshot,
            Some(selection),
            2,
            &HashMap::new(),
            &HashMap::new(),
            None,
            false,
            false,
            false,
        );

        assert_eq!(decorations[0].selection_cols, None);
        assert_eq!(decorations[1].selection_cols, None);
        assert_eq!(decorations[2].selection_cols, Some((2, usize::MAX)));
        assert_eq!(decorations[3].selection_cols, Some((0, 5)));
        assert_eq!(decorations[4].selection_cols, None);
    }

    #[test]
    fn decoration_cache_key_tracks_selection_viewport_anchor() {
        let snapshot = nyaterm_terminal::TerminalScreen::default().viewport_snapshot(0);
        let selection = TerminalSelection::new(TerminalCellPos::new(0, 2));

        let first = terminal_line_decorations_cache_key(
            &snapshot,
            Some(selection),
            0,
            &HashMap::new(),
            &HashMap::new(),
            None,
            false,
            false,
            false,
        );
        let second = terminal_line_decorations_cache_key(
            &snapshot,
            Some(selection),
            1,
            &HashMap::new(),
            &HashMap::new(),
            None,
            false,
            false,
            false,
        );

        assert_ne!(first, second);
    }
}

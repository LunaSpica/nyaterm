use super::*;
use crate::models::{TerminalFrameActionLinks, TerminalSelection};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub(in crate::features) fn terminal_snapshot_absolute_range(
    snapshot: &nyaterm_terminal::TerminalSnapshot,
) -> (usize, usize) {
    let end = snapshot.total_rows.saturating_sub(snapshot.display_offset);
    let start = end.saturating_sub(snapshot.row_count());
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
    snapshot.row_count().hash(&mut hasher);
    snapshot.cols.hash(&mut hasher);
    snapshot.display_offset.hash(&mut hasher);
    for row in snapshot.rows() {
        row.signature.hash(&mut hasher);
    }
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
            links.absolute_start_row.hash(&mut hasher);
            links.absolute_end_row.hash(&mut hasher);
            for line_index in 0..snapshot.row_count() {
                terminal_action_link_ranges_for_snapshot_row(snapshot, line_index, links)
                    .hash(&mut hasher);
            }
        } else {
            0u64.hash(&mut hasher);
        }
    }
    if include_hyperlinks {
        snapshot.row_count().hash(&mut hasher);
        for row in snapshot.rows() {
            let spans = &row.hyperlinks;
            spans.len().hash(&mut hasher);
            for span in spans {
                span.start_col.hash(&mut hasher);
                span.end_col.hash(&mut hasher);
                span.uri.hash(&mut hasher);
            }
        }
    }
    if include_command_marks {
        for row in snapshot.rows() {
            row.command_mark.hash(&mut hasher);
        }
    }
    hasher.finish()
}

fn terminal_action_link_ranges_for_snapshot_row<'a>(
    snapshot: &nyaterm_terminal::TerminalSnapshot,
    line_index: usize,
    links: &'a TerminalFrameActionLinks,
) -> &'a [(usize, usize)] {
    let empty: &'a [(usize, usize)] = &[];
    if line_index >= snapshot.row_count() {
        return empty;
    }
    let (snapshot_start, snapshot_end) = terminal_snapshot_absolute_range(snapshot);
    let Some(absolute_row) = snapshot_start.checked_add(line_index) else {
        return empty;
    };
    if absolute_row >= snapshot_end
        || absolute_row < links.absolute_start_row
        || absolute_row >= links.absolute_end_row
    {
        return empty;
    }
    links
        .cell_ranges_by_line
        .get(absolute_row - links.absolute_start_row)
        .map(Vec::as_slice)
        .unwrap_or(empty)
}

pub(in crate::features) fn terminal_action_links_cover_snapshot(
    snapshot: &nyaterm_terminal::TerminalSnapshot,
    links: &TerminalFrameActionLinks,
) -> bool {
    let (snapshot_start, snapshot_end) = terminal_snapshot_absolute_range(snapshot);
    links.absolute_start_row <= snapshot_start && snapshot_end <= links.absolute_end_row
}

pub(in crate::features) fn terminal_action_links_have_ranges_for_snapshot(
    snapshot: &nyaterm_terminal::TerminalSnapshot,
    links: &TerminalFrameActionLinks,
) -> bool {
    (0..snapshot.row_count()).any(|line_index| {
        !terminal_action_link_ranges_for_snapshot_row(snapshot, line_index, links).is_empty()
    })
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
    let line_count = snapshot.row_count();
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
                .map(|links| {
                    terminal_action_link_ranges_for_snapshot_row(snapshot, line_index, links)
                        .to_vec()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        if include_hyperlinks
            && let Some(spans) = snapshot.row(line_index).map(|row| &row.hyperlinks)
        {
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
            .then(|| snapshot.row(line_index).and_then(|row| row.command_mark))
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
    fn link_decorations_merge_action_links_and_hyperlinks_when_included() {
        let mut screen = nyaterm_terminal::TerminalScreen::new(40, 3);
        screen.advance(b"\x1b]8;;https://example.com\x07click\x1b]8;;\x07 plain");
        let snapshot = screen.viewport_snapshot(0);
        let (absolute_start_row, absolute_end_row) = terminal_snapshot_absolute_range(&snapshot);
        let mut links = TerminalFrameActionLinks {
            matcher_key: 7,
            absolute_start_row,
            absolute_end_row,
            matches_by_line: vec![Vec::new(); snapshot.row_count()],
            cell_ranges_by_line: vec![Vec::new(); snapshot.row_count()],
        };
        links.cell_ranges_by_line[0].push((6, 11));

        let decorations = build_terminal_line_decorations(
            &snapshot,
            None,
            0,
            &HashMap::new(),
            &HashMap::new(),
            Some(&links),
            true,
            true,
            false,
        );

        assert_eq!(decorations[0].link_ranges, vec![(6, 11), (0, 5)]);
    }

    #[test]
    fn action_link_decorations_map_from_absolute_link_window() {
        let mut screen = nyaterm_terminal::TerminalScreen::new(40, 3);
        screen.advance(b"first\nsecond\nthird");
        let snapshot = screen.viewport_snapshot(0);
        let (snapshot_start, snapshot_end) = terminal_snapshot_absolute_range(&snapshot);
        let mut links = TerminalFrameActionLinks {
            matcher_key: 7,
            absolute_start_row: snapshot_start.saturating_sub(1),
            absolute_end_row: snapshot_end.saturating_add(1),
            matches_by_line: vec![Vec::new(); snapshot.row_count() + 2],
            cell_ranges_by_line: vec![Vec::new(); snapshot.row_count() + 2],
        };
        let relative_row = snapshot_start + 1 - links.absolute_start_row;
        links.cell_ranges_by_line[relative_row].push((2, 5));

        let decorations = build_terminal_line_decorations(
            &snapshot,
            None,
            0,
            &HashMap::new(),
            &HashMap::new(),
            Some(&links),
            true,
            false,
            false,
        );

        assert!(decorations[0].link_ranges.is_empty());
        assert_eq!(decorations[1].link_ranges, vec![(2, 5)]);
        assert!(decorations[2].link_ranges.is_empty());
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

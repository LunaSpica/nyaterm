use crate::{CommandHistoryEntry, FuzzyResult, QuickCommand};

pub fn search_command_sources(
    history: &[CommandHistoryEntry],
    quick_commands: &[QuickCommand],
    pattern: &str,
    limit: usize,
    min_history_command_length: Option<usize>,
    max_history_command_length: Option<usize>,
) -> Vec<FuzzyResult> {
    let mut results = Vec::new();
    let history_items: Vec<(&str, &str)> = history
        .iter()
        .map(|entry| (entry.command.as_str(), entry.command.as_str()))
        .collect();
    results.extend(fuzzy_search_items(
        &history_items,
        pattern,
        "history",
        limit,
        min_history_command_length,
        max_history_command_length,
    ));

    let quick_items: Vec<(&str, &str)> = quick_commands
        .iter()
        .map(|command| {
            let display = if command.label.trim().is_empty() {
                command.command.as_str()
            } else {
                command.label.as_str()
            };
            (display, command.command.as_str())
        })
        .collect();
    results.extend(fuzzy_search_items(
        &quick_items,
        pattern,
        "quickCommand",
        limit,
        None,
        None,
    ));

    results.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then(left.source.cmp(&right.source))
            .then(left.command.cmp(&right.command))
    });
    results.truncate(limit);
    results
}

pub fn fuzzy_search_items(
    items: &[(&str, &str)],
    pattern: &str,
    source: &str,
    limit: usize,
    min_command_length: Option<usize>,
    max_command_length: Option<usize>,
) -> Vec<FuzzyResult> {
    let pattern = pattern.trim();
    if pattern.is_empty() || limit == 0 {
        return Vec::new();
    }

    let mut scored = Vec::new();
    for (index, (display, command)) in items.iter().enumerate() {
        let command_len = command.chars().count();
        if min_command_length.is_some_and(|min| command_len < min) {
            continue;
        }
        if max_command_length.is_some_and(|max| command_len > max) {
            continue;
        }
        if let Some((score, indices)) = fuzzy_match(display, pattern) {
            scored.push((
                index,
                FuzzyResult {
                    command: (*command).to_string(),
                    score,
                    indices,
                    source: source.to_string(),
                    display: (*display).to_string(),
                },
            ));
        }
    }

    scored.sort_by(|left, right| right.1.score.cmp(&left.1.score).then(right.0.cmp(&left.0)));
    scored
        .into_iter()
        .take(limit)
        .map(|(_, result)| result)
        .collect()
}

fn fuzzy_match(display: &str, pattern: &str) -> Option<(u32, Vec<u32>)> {
    let haystack: Vec<char> = display.chars().collect();
    let needle: Vec<char> = pattern.chars().filter(|ch| !ch.is_whitespace()).collect();
    if haystack.is_empty() || needle.is_empty() {
        return None;
    }

    let haystack_lower: Vec<char> = haystack.iter().flat_map(|ch| ch.to_lowercase()).collect();
    let needle_lower: Vec<char> = needle.iter().flat_map(|ch| ch.to_lowercase()).collect();
    let mut search_from = 0;
    let mut indices = Vec::new();
    for needle_char in needle_lower {
        let relative_index = haystack_lower[search_from..]
            .iter()
            .position(|haystack_char| *haystack_char == needle_char)?;
        let index = search_from + relative_index;
        indices.push(index as u32);
        search_from = index.saturating_add(1);
    }

    let mut score = (indices.len() as u32).saturating_mul(100);
    for pair in indices.windows(2) {
        if pair[1] == pair[0].saturating_add(1) {
            score = score.saturating_add(40);
        } else {
            score = score.saturating_sub(pair[1].saturating_sub(pair[0]).min(30));
        }
    }
    if let Some(first) = indices.first().copied() {
        score = score.saturating_add(80_u32.saturating_sub(first.min(80)));
    }
    for index in &indices {
        if is_word_boundary(&haystack, *index as usize) {
            score = score.saturating_add(20);
        }
    }
    let display_lower = display.to_lowercase();
    let pattern_lower = pattern.to_lowercase();
    if display_lower.contains(&pattern_lower) {
        score = score.saturating_add(160);
    }

    Some((score, indices))
}

fn is_word_boundary(chars: &[char], index: usize) -> bool {
    if index == 0 {
        return true;
    }
    let previous = chars[index - 1];
    let current = chars[index];
    !previous.is_alphanumeric() || (previous.is_lowercase() && current.is_uppercase())
}

#[cfg(test)]
mod tests {
    use super::{fuzzy_search_items, search_command_sources};
    use crate::{CommandHistoryEntry, QuickCommand};

    #[test]
    fn fuzzy_search_items_scores_contiguous_and_filters_length() {
        let items = [
            ("docker compose ps", "docker compose ps"),
            ("git status", "git status"),
            ("long command", "0123456789"),
        ];
        let results = fuzzy_search_items(&items, "g s", "history", 5, Some(2), Some(10));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].command, "git status");
        assert_eq!(results[0].source, "history");
        assert!(!results[0].indices.is_empty());
    }

    #[test]
    fn command_search_merges_history_and_quick_commands() {
        let history = vec![CommandHistoryEntry {
            command: "git status".to_string(),
            last_used_at_ms: 10,
            use_count: 2,
        }];
        let quick_commands = vec![QuickCommand {
            id: "qc-1".to_string(),
            label: "Docker PS".to_string(),
            command: "docker ps".to_string(),
            category_id: None,
            description: None,
            color_tag: None,
            icon_tag: None,
            pinned: None,
            execution_mode: None,
            source: None,
            risk_level: None,
            updated_at: None,
            created_at: None,
            use_count: None,
        }];

        let results = search_command_sources(&history, &quick_commands, "ps", 10, None, None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].command, "docker ps");
        assert_eq!(results[0].display, "Docker PS");
        assert_eq!(results[0].source, "quickCommand");
    }
}

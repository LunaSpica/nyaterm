use super::*;

pub(in crate::features::pages::remote) fn process_matches(
    process: &RemoteProcess,
    normalized_query: &str,
) -> bool {
    if normalized_query.is_empty() {
        return true;
    }
    format!(
        "{} {} {} {} {} {}",
        process.pid,
        process.ppid,
        process.user,
        process.state,
        process.command,
        process.command_line
    )
    .to_ascii_lowercase()
    .contains(normalized_query)
}

pub(in crate::features::pages::remote) fn sort_processes(
    processes: &mut [RemoteProcess],
    key: RemoteProcessSortKey,
    direction: RemoteProcessSortDirection,
) {
    processes.sort_by(|left, right| {
        let ordering = match key {
            RemoteProcessSortKey::Command => left
                .command
                .cmp(&right.command)
                .then_with(|| left.pid.cmp(&right.pid)),
            RemoteProcessSortKey::Memory => left
                .memory_percent
                .partial_cmp(&right.memory_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    left.rss_kb
                        .partial_cmp(&right.rss_kb)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| left.pid.cmp(&right.pid)),
            RemoteProcessSortKey::Pid => left.pid.cmp(&right.pid),
            RemoteProcessSortKey::User => left
                .user
                .cmp(&right.user)
                .then_with(|| left.pid.cmp(&right.pid)),
            RemoteProcessSortKey::Cpu => left
                .cpu_percent
                .partial_cmp(&right.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    left.memory_percent
                        .partial_cmp(&right.memory_percent)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .then_with(|| left.pid.cmp(&right.pid)),
        };

        match direction {
            RemoteProcessSortDirection::Ascending => ordering,
            RemoteProcessSortDirection::Descending => ordering.reverse(),
        }
    });
}

pub(in crate::features::pages::remote) fn top_process_ratio(
    processes: &[RemoteProcess],
    cpu: bool,
) -> f64 {
    processes
        .iter()
        .map(|process| {
            if cpu {
                process.cpu_percent
            } else {
                process.memory_percent
            }
        })
        .fold(0.0, f64::max)
        / 100.
}

pub(in crate::features::pages::remote) fn process_summary_card(
    palette: ThemePalette,
    title: &'static str,
    value: String,
    ratio: f64,
) -> impl IntoElement {
    let ratio = ratio.clamp(0., 1.);
    // Compact metric chip for Process Manager summary strip.
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.bg))
        .px_2()
        .py_1()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_size(px(10.))
                .font_weight(FontWeight(600.))
                .text_color(rgb(palette.text_muted))
                .child(title),
        )
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight(700.))
                .text_color(usage_color(palette, ratio))
                .child(value),
        )
        .child(stats_progress_bar(palette, ratio))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::features::pages::remote) enum ProcessDisplayMode {
    Compact,
    Narrow,
    Medium,
    Wide,
}

pub(in crate::features::pages::remote) fn process_display_mode(
    panel_width: f32,
) -> ProcessDisplayMode {
    // Tauri getProcessDisplayMode thresholds.
    if panel_width > 0. && panel_width < 320. {
        ProcessDisplayMode::Compact
    } else if panel_width > 0. && panel_width < 430. {
        ProcessDisplayMode::Narrow
    } else if panel_width > 0. && panel_width < 540. {
        ProcessDisplayMode::Medium
    } else {
        ProcessDisplayMode::Wide
    }
}

pub(in crate::features::pages::remote) fn process_row_height_px(mode: ProcessDisplayMode) -> f32 {
    match mode {
        ProcessDisplayMode::Compact => 62.,
        _ => 38.,
    }
}

pub(in crate::features::pages::remote) fn process_details_height_px(
    mode: ProcessDisplayMode,
) -> f32 {
    // Native densified shells (Tauri uses 176/218/274).
    match mode {
        ProcessDisplayMode::Compact => 274.,
        ProcessDisplayMode::Narrow => 218.,
        _ => 176.,
    }
}

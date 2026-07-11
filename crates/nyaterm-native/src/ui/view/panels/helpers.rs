use super::*;

pub(in crate::ui::view::panels) struct QuickCommandCategoryOption {
    pub id: String,
    pub label: String,
    pub count: usize,
    pub manageable: bool,
}

pub(in crate::ui::view::panels) fn quick_command_category_options(
    commands: &[QuickCommand],
    categories: &[QuickCommandCategory],
) -> Vec<QuickCommandCategoryOption> {
    let mut options = vec![QuickCommandCategoryOption {
        id: "all".to_string(),
        label: "All".to_string(),
        count: commands.len(),
        manageable: false,
    }];

    for category in categories {
        let count = commands
            .iter()
            .filter(|command| command.category_id.as_deref() == Some(category.id.as_str()))
            .count();
        options.push(QuickCommandCategoryOption {
            id: category.id.clone(),
            label: category.name.clone(),
            count,
            manageable: true,
        });
    }

    let uncategorized = commands
        .iter()
        .filter(|command| {
            command
                .category_id
                .as_deref()
                .unwrap_or_default()
                .is_empty()
        })
        .count();
    options.push(QuickCommandCategoryOption {
        id: "uncategorized".to_string(),
        label: "Unsorted".to_string(),
        count: uncategorized,
        manageable: false,
    });
    options
}

pub(in crate::ui::view::panels) fn filtered_quick_commands(
    commands: &[QuickCommand],
    categories: &[QuickCommandCategory],
    query: &str,
    selected_category: &str,
    sort_mode: QuickCommandSortMode,
) -> Vec<QuickCommand> {
    let query = query.trim().to_lowercase();
    let mut filtered = commands
        .iter()
        .filter(|command| match selected_category {
            "all" => true,
            "uncategorized" => command
                .category_id
                .as_deref()
                .unwrap_or_default()
                .trim()
                .is_empty(),
            category_id => command.category_id.as_deref() == Some(category_id),
        })
        .filter(|command| {
            if query.is_empty() {
                return true;
            }
            let category = quick_command_category_label(categories, command);
            command.label.to_lowercase().contains(&query)
                || command.command.to_lowercase().contains(&query)
                || command
                    .description
                    .as_deref()
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains(&query)
                || category.to_lowercase().contains(&query)
        })
        .cloned()
        .collect::<Vec<_>>();

    filtered.sort_by(|left, right| {
        right
            .pinned
            .unwrap_or_default()
            .cmp(&left.pinned.unwrap_or_default())
            .then_with(|| match sort_mode {
                QuickCommandSortMode::Usage => right
                    .use_count
                    .unwrap_or_default()
                    .cmp(&left.use_count.unwrap_or_default())
                    .then_with(|| {
                        right
                            .updated_at
                            .unwrap_or_default()
                            .cmp(&left.updated_at.unwrap_or_default())
                    }),
                QuickCommandSortMode::Name => {
                    left.label.to_lowercase().cmp(&right.label.to_lowercase())
                }
                QuickCommandSortMode::Created => left
                    .created_at
                    .or(left.updated_at)
                    .unwrap_or(u64::MAX)
                    .cmp(&right.created_at.or(right.updated_at).unwrap_or(u64::MAX)),
            })
            .then_with(|| left.label.to_lowercase().cmp(&right.label.to_lowercase()))
    });
    filtered
}

pub(in crate::ui::view::panels) fn quick_command_icon_mark(
    icon_tag: Option<&str>,
    color_tag: Option<&str>,
) -> impl IntoElement {
    match icon_tag.and_then(quick_command_icon_def) {
        Some((label, color)) => div()
            .size(px(18.))
            .flex_none()
            .rounded_sm()
            .border_1()
            .border_color(color)
            .bg(rgb(0x101827))
            .flex()
            .items_center()
            .justify_center()
            .font_family("JetBrains Mono")
            .text_size(px(8.))
            .font_weight(FontWeight(800.))
            .text_color(color)
            .child(label)
            .into_any_element(),
        None => div()
            .size(px(9.))
            .flex_none()
            .rounded_full()
            .bg(quick_command_color(color_tag))
            .into_any_element(),
    }
}

pub(in crate::ui::view::panels) fn quick_command_color(color_tag: Option<&str>) -> gpui::Rgba {
    match color_tag.unwrap_or_default() {
        "red" => rgb(0xef4444),
        "green" => rgb(0x22c55e),
        "blue" => rgb(0x3b82f6),
        "yellow" => rgb(0xeab308),
        "purple" => rgb(0xa855f7),
        _ => rgb(0x94a3b8),
    }
}

pub(in crate::ui::view::panels) fn quick_command_icon_label(icon_tag: Option<&str>) -> String {
    icon_tag
        .and_then(quick_command_icon_def)
        .map(|(label, _)| label.to_string())
        .unwrap_or_else(|| "none".to_string())
}

fn quick_command_icon_def(icon_tag: &str) -> Option<(&'static str, gpui::Rgba)> {
    match icon_tag.trim().to_ascii_lowercase().as_str() {
        "docker" => Some(("DK", rgb(0x2496ed))),
        "k8s" => Some(("K8", rgb(0x326ce5))),
        "linux" => Some(("LX", rgb(0xfcc624))),
        "ubuntu" => Some(("UB", rgb(0xe95420))),
        "debian" => Some(("DB", rgb(0xa81d33))),
        "centos" => Some(("CE", rgb(0x7f3f98))),
        "fedora" => Some(("FE", rgb(0x3c6eb4))),
        "apple" => Some(("AP", rgb(0xa2aaad))),
        "github" => Some(("GH", rgb(0xe5e7eb))),
        "gitlab" => Some(("GL", rgb(0xfc6d26))),
        "nginx" => Some(("NX", rgb(0x009639))),
        "redis" => Some(("RD", rgb(0xdc382d))),
        "postgres" => Some(("PG", rgb(0x4169e1))),
        "mysql" => Some(("MY", rgb(0x4479a1))),
        "mongodb" => Some(("MO", rgb(0x47a248))),
        "python" => Some(("PY", rgb(0x3776ab))),
        "js" => Some(("JS", rgb(0xf7df1e))),
        "ts" => Some(("TS", rgb(0x3178c6))),
        "rust" => Some(("RS", rgb(0xce412b))),
        "go" => Some(("GO", rgb(0x00add8))),
        "node" => Some(("ND", rgb(0x339933))),
        "php" => Some(("PH", rgb(0x777bb4))),
        "aws" => Some(("AWS", rgb(0xff9900))),
        "gcp" => Some(("GC", rgb(0x4285f4))),
        "terminal" => Some((">$", rgb(0x6ee7b7))),
        "code" => Some(("</>", rgb(0x93c5fd))),
        "server" => Some(("SR", rgb(0xc4b5fd))),
        "folder" => Some(("FD", rgb(0xfacc15))),
        "sparkles" => Some(("AI", rgb(0xf0abfc))),
        "bolt" => Some(("BT", rgb(0xfbbf24))),
        _ => None,
    }
}

pub(in crate::ui::view::panels) fn quick_command_editor_field(
    id: &'static str,
    label: &'static str,
    placeholder: &'static str,
    value: String,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let is_placeholder = value.is_empty();
    let display = if is_placeholder {
        placeholder.to_string()
    } else {
        value
    };
    div()
        .id(SharedString::from(id))
        .rounded_sm()
        .border_1()
        .border_color(if active { rgb(0x4ade80) } else { rgb(0x263142) })
        .bg(if active { rgb(0x0f1f18) } else { rgb(0x0d1320) })
        .p_2()
        .cursor_pointer()
        .on_click(on_click)
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(0x64748b))
                .child(label),
        )
        .child(
            div()
                .mt_1()
                .min_h(px(20.))
                .font_family("JetBrains Mono")
                .text_xs()
                .line_height(px(18.))
                .text_color(if is_placeholder {
                    rgb(0x64748b)
                } else {
                    rgb(0xe5edf7)
                })
                .child(truncate_preview(&display, 160)),
        )
}

pub(in crate::ui::view::panels) fn quick_command_editor_script_field(
    id: &'static str,
    label: &'static str,
    placeholder: &'static str,
    value: String,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let is_placeholder = value.is_empty();
    let display = if is_placeholder {
        placeholder.to_string()
    } else {
        value
    };
    div()
        .id(SharedString::from(id))
        .rounded_sm()
        .border_1()
        .border_color(if active { rgb(0x4ade80) } else { rgb(0x263142) })
        .bg(if active { rgb(0x0f1f18) } else { rgb(0x101827) })
        .p_2()
        .cursor_pointer()
        .on_click(on_click)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(0x64748b))
                        .child(label),
                )
                .child(
                    div()
                        .text_size(px(10.))
                        .text_color(rgb(0x98a3b8))
                        .child("Enter inserts newline"),
                ),
        )
        .child(
            div()
                .mt_2()
                .min_h(px(112.))
                .font_family("JetBrains Mono")
                .text_xs()
                .line_height(px(18.))
                .text_color(if is_placeholder {
                    rgb(0x64748b)
                } else {
                    rgb(0xe5edf7)
                })
                .child(truncate_preview(&display, 360)),
        )
}

pub(in crate::ui::view::panels) fn send_command_hex_preview(draft: &str) -> String {
    match parse_send_command_hex(draft) {
        Ok(bytes) if bytes.is_empty() => String::new(),
        Ok(bytes) => bytes
            .iter()
            .take(96)
            .map(|byte| {
                if (0x20..=0x7e).contains(byte) {
                    char::from(*byte)
                } else {
                    '.'
                }
            })
            .collect(),
        Err(error) => error,
    }
}

pub(in crate::ui::view::panels) fn send_command_hex_byte_count(draft: &str) -> Option<usize> {
    parse_send_command_hex(draft).ok().map(|bytes| bytes.len())
}

/// Format hex draft like Tauri: uppercase pairs spaced, double-space every 4 bytes.
pub(in crate::ui::view::panels) fn format_send_command_hex_display(draft: &str) -> String {
    let normalized = draft.replace("\r\n", "\n").replace("\r", "\n");
    normalized
        .split('\n')
        .map(|line| {
            let cleaned: String = line
                .chars()
                .filter(|ch| ch.is_ascii_hexdigit())
                .map(|ch| ch.to_ascii_uppercase())
                .collect();
            let mut formatted = String::new();
            let mut byte_index = 0usize;
            let mut i = 0usize;
            while i < cleaned.len() {
                let end = (i + 2).min(cleaned.len());
                let byte = &cleaned[i..end];
                formatted.push_str(byte);
                if byte.len() == 2 {
                    byte_index += 1;
                    if i + 2 < cleaned.len() {
                        if byte_index % 4 == 0 {
                            formatted.push_str("  ");
                        } else {
                            formatted.push(' ');
                        }
                    }
                }
                i = end;
            }
            formatted
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(in crate::ui::view::panels) fn send_command_hex_guide_count(draft: &str) -> usize {
    let normalized = draft.replace("\r\n", "\n").replace("\r", "\n");
    normalized
        .split('\n')
        .map(|line| {
            let hex_chars = line.chars().filter(|ch| ch.is_ascii_hexdigit()).count();
            (hex_chars / 2) / 4
        })
        .sum()
}

/// Character offsets (approx mono columns) for 4-byte group boundaries on first line.
pub(in crate::ui::view::panels) fn send_command_hex_guide_marks(draft: &str) -> Vec<u32> {
    let display = format_send_command_hex_display(draft);
    let first_line = display.split('\n').next().unwrap_or("");
    // After each complete 4-byte group Tauri inserts an extra space: "AA BB CC DD  "
    // Count completed groups from cleaned hex.
    let cleaned: String = first_line
        .chars()
        .filter(|ch| ch.is_ascii_hexdigit())
        .collect();
    let bytes = cleaned.len() / 2;
    let groups = bytes / 4;
    // Each byte is 2 hex + 1 space, except after every 4th byte double space.
    // Approximate left position in ch units used by overlay (7.2px per ch-ish).
    // Position after group g (1-based) ~ g * 12 chars (4*(2+1) with trailing extra space).
    (1..=groups)
        .map(|group| (group as u32) * 12)
        .take(24)
        .collect()
}

pub(in crate::ui::view::panels) fn terminal_action_prompt_text(
    text: &str,
    max_chars: usize,
) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max_chars {
        return trimmed.to_string();
    }
    let tail = trimmed
        .chars()
        .rev()
        .take(max_chars)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("[truncated to last {max_chars} chars]\n{tail}")
}

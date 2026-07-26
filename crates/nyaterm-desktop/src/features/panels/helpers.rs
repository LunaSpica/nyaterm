use super::*;

use crate::models::QuickCommandSortMode;

pub(in crate::features::panels) struct QuickCommandCategoryOption {
    pub id: String,
    pub label: String,
    pub count: usize,
    pub manageable: bool,
}

pub(in crate::features::panels) fn quick_command_category_options(
    commands: &[QuickCommand],
    categories: &[QuickCommandCategory],
    all_label: &'static str,
    uncategorized_label: &'static str,
) -> Vec<QuickCommandCategoryOption> {
    let mut options = vec![QuickCommandCategoryOption {
        id: "all".to_string(),
        label: all_label.to_string(),
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
        label: uncategorized_label.to_string(),
        count: uncategorized,
        manageable: false,
    });
    options
}

pub(in crate::features::panels) fn filtered_quick_commands(
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

pub(in crate::features::panels) fn quick_command_icon_mark(
    palette: crate::theme::ThemePalette,
    icon_tag: Option<&str>,
    color_tag: Option<&str>,
) -> impl IntoElement {
    match icon_tag.and_then(quick_command_icon) {
        Some(def) => crate::features::mono_icon(
            def.path,
            rgb(def.tint(palette).unwrap_or(palette.text)).into(),
            16.,
        )
        .into_any_element(),
        // No icon chosen: the command still gets its color, as a plain dot.
        None => div()
            .size(px(9.))
            .flex_none()
            .rounded_full()
            .bg(quick_command_color(palette, color_tag))
            .into_any_element(),
    }
}

pub(in crate::features::panels) fn quick_command_pin_mark(
    palette: crate::theme::ThemePalette,
) -> impl IntoElement {
    svg()
        .size(px(12.))
        .flex_none()
        .text_color(rgb(palette.warning))
        .path("icons/pin.svg")
}

pub(in crate::features::panels) fn quick_command_color(
    palette: crate::theme::ThemePalette,
    color_tag: Option<&str>,
) -> gpui::Rgba {
    match color_tag.unwrap_or_default() {
        "red" => rgb(palette.danger),
        "green" => rgb(palette.success),
        "blue" => rgb(palette.link),
        "yellow" => rgb(palette.warning),
        "purple" => rgb(palette.link),
        _ => rgb(palette.text_muted),
    }
}

pub(in crate::features::panels) fn quick_command_editor_field(
    palette: crate::theme::ThemePalette,
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
        .min_w_0()
        .flex_1()
        .rounded_sm()
        .border_1()
        .border_color(if active {
            rgb(palette.success)
        } else {
            rgb(palette.border)
        })
        .bg(if active {
            rgb(palette.hover)
        } else {
            rgb(palette.input)
        })
        .p_2()
        .cursor_pointer()
        .on_click(on_click)
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(palette.text_muted))
                .child(label),
        )
        .child(
            div()
                .mt_1()
                .min_h(px(20.))
                .font_family(crate::features::gpui_code_font_family())
                .text_xs()
                .line_height(px(18.))
                .text_color(if is_placeholder {
                    rgb(palette.text_muted)
                } else {
                    rgb(palette.text)
                })
                .child(truncate_preview(&display, 160)),
        )
}

pub(in crate::features::panels) fn quick_command_editor_script_field(
    palette: crate::theme::ThemePalette,
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
        .border_color(if active {
            rgb(palette.success)
        } else {
            rgb(palette.border)
        })
        .bg(if active {
            rgb(palette.hover)
        } else {
            rgb(palette.input)
        })
        .p_2()
        .cursor_pointer()
        .on_click(on_click)
        .child(
            div().flex().items_center().child(
                div()
                    .text_size(px(10.))
                    .text_color(rgb(palette.text_muted))
                    .child(label),
            ),
        )
        .child(
            div()
                .mt_2()
                .min_h(px(112.))
                .font_family(crate::features::gpui_code_font_family())
                .text_xs()
                .line_height(px(18.))
                .text_color(if is_placeholder {
                    rgb(palette.text_muted)
                } else {
                    rgb(palette.text)
                })
                .child(truncate_preview(&display, 360)),
        )
}

pub(in crate::features::panels) fn send_command_hex_preview(draft: &str) -> String {
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

pub(in crate::features::panels) fn send_command_hex_byte_count(draft: &str) -> Option<usize> {
    parse_send_command_hex(draft).ok().map(|bytes| bytes.len())
}

/// Per-line character offsets for 4-byte group boundaries (Tauri `buildHexGuideRows`).
pub(in crate::features::panels) fn send_command_hex_guide_rows(draft: &str) -> Vec<Vec<u32>> {
    let display = crate::send_command::format_send_command_hex_display(draft);
    display
        .split('\n')
        .map(|line| {
            let cleaned: String = line.chars().filter(|ch| ch.is_ascii_hexdigit()).collect();
            let bytes = cleaned.len() / 2;
            let groups = bytes / 4;
            // Tauri: left = (groupNumber * 13 - 1) ch
            (1..=groups)
                .map(|group| (group as u32) * 13 - 1)
                .take(24)
                .collect()
        })
        .collect()
}

pub(in crate::features::panels) fn terminal_action_prompt_text(
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

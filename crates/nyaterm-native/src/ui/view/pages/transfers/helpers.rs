use super::*;

pub(super) fn remote_file_name(path: &str) -> String {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string()
}

pub(super) fn remote_parent_path(path: &str) -> String {
    let path = path.trim_end_matches('/');
    match path.rfind('/') {
        Some(0) => "/".to_string(),
        Some(index) => path[..index].to_string(),
        None => ".".to_string(),
    }
}

pub(super) fn remote_sibling_path(old_path: &str, new_name: &str) -> String {
    match remote_parent_path(old_path).as_str() {
        "/" => format!("/{new_name}"),
        "." => new_name.to_string(),
        parent => format!("{parent}/{new_name}"),
    }
}

pub(super) fn remote_child_path(parent: &str, child_name: &str) -> String {
    match parent.trim_end_matches('/') {
        "" | "." => child_name.to_string(),
        "/" => format!("/{child_name}"),
        parent => format!("{parent}/{child_name}"),
    }
}

pub(super) fn normalized_transfer_browser_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        ".".to_string()
    } else if trimmed == "/" {
        "/".to_string()
    } else {
        trimmed.trim_end_matches('/').to_string()
    }
}

pub(super) fn valid_remote_child_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains('/')
}

#[derive(Debug, Clone, Copy)]
pub(super) enum TransferPathPart {
    Full,
    Name,
    Directory,
}

impl TransferPathPart {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Full => "path",
            Self::Name => "name",
            Self::Directory => "directory",
        }
    }
}

pub(super) const TRANSFER_BROWSER_ACTIONS_WIDTH: gpui::Pixels = px(360.);
const TRANSFER_BROWSER_COLUMN_GAP_TOTAL: gpui::Pixels = px(48.);

pub(super) fn transfer_browser_table_width(widths: TransferBrowserColumnWidths) -> gpui::Pixels {
    widths.name
        + widths.modified
        + widths.size
        + widths.permissions
        + widths.owner
        + widths.group
        + TRANSFER_BROWSER_ACTIONS_WIDTH
        + TRANSFER_BROWSER_COLUMN_GAP_TOTAL
}

pub(super) fn transfer_path_part_value(path: &str, part: TransferPathPart) -> String {
    match part {
        TransferPathPart::Full => path.to_string(),
        TransferPathPart::Name => remote_file_name(path),
        TransferPathPart::Directory => remote_parent_path(path),
    }
}

pub(super) fn format_sftp_modified(value: Option<u32>) -> String {
    value
        .map(|seconds| format!("{seconds}s"))
        .unwrap_or_else(|| "-".to_string())
}

pub(super) fn symlink_input_row(
    id: &'static str,
    label: &'static str,
    value: &str,
    focused: bool,
    invalid: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .mt_3()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(72.))
                .text_xs()
                .text_color(rgb(0x98a3b8))
                .child(label),
        )
        .child(
            div()
                .id(SharedString::from(id))
                .h(px(36.))
                .flex_1()
                .min_w_0()
                .rounded_sm()
                .border_1()
                .border_color(if invalid {
                    rgb(0x7f1d1d)
                } else if focused {
                    rgb(0x256d3f)
                } else {
                    rgb(0x334155)
                })
                .bg(rgb(0x0d1320))
                .px_3()
                .flex()
                .items_center()
                .font_family("JetBrains Mono")
                .text_sm()
                .text_color(
                    if value.is_empty() || value == "Symlink name" || value == "/path/to/target" {
                        rgb(0x64748b)
                    } else {
                        rgb(0xe5edf7)
                    },
                )
                .cursor_pointer()
                .on_click(on_click)
                .child(truncate_preview(value, 88)),
        )
}

pub(super) fn property_row(
    label: &'static str,
    value: impl Into<SharedString>,
) -> impl IntoElement {
    div()
        .flex()
        .items_start()
        .gap_3()
        .text_xs()
        .child(
            div()
                .w(px(88.))
                .text_color(rgb(0x64748b))
                .child(format!("{label}:")),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .font_family("JetBrains Mono")
                .text_color(rgb(0xdbeafe))
                .child(value.into()),
        )
}

pub(super) fn property_input_row(
    id: &'static str,
    label: &'static str,
    value: &str,
    focused: bool,
    disabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(72.))
                .text_xs()
                .text_color(rgb(0x98a3b8))
                .child(label),
        )
        .child(
            div()
                .id(SharedString::from(id))
                .h(px(34.))
                .flex_1()
                .min_w_0()
                .rounded_sm()
                .border_1()
                .border_color(if focused {
                    rgb(0x256d3f)
                } else {
                    rgb(0x334155)
                })
                .bg(if disabled {
                    rgb(0x151923)
                } else {
                    rgb(0x0d1320)
                })
                .px_3()
                .flex()
                .items_center()
                .font_family("JetBrains Mono")
                .text_xs()
                .text_color(if value.is_empty() {
                    rgb(0x64748b)
                } else {
                    rgb(0xe5edf7)
                })
                .cursor_pointer()
                .on_click(on_click)
                .child(if value.is_empty() {
                    SharedString::from("-")
                } else {
                    SharedString::from(value.to_string())
                }),
        )
}

pub(super) fn transfer_properties_state_from_entry(
    entry: SftpFileEntry,
) -> TransferPropertiesState {
    let mode_value = entry
        .permissions
        .map(format_permissions_octal)
        .unwrap_or_else(|| "0644".to_string());
    TransferPropertiesState {
        owner_value: String::new(),
        group_value: String::new(),
        entry,
        properties: None,
        mode_value,
        recursive: false,
        saving: false,
        error: None,
        focused_field: TransferPropertiesField::Mode,
    }
}

pub(super) fn parse_transfer_mode(value: &str) -> Option<u32> {
    let value = value.trim();
    if !(3..=4).contains(&value.len()) || !value.chars().all(|ch| ('0'..='7').contains(&ch)) {
        return None;
    }
    u32::from_str_radix(value, 8).ok()
}

pub(super) fn format_owner_group(name: &str, id: Option<u32>) -> String {
    match (name.trim().is_empty(), id) {
        (true, Some(id)) => id.to_string(),
        (true, None) => "-".to_string(),
        (false, Some(id)) => format!("{} [{}]", name.trim(), id),
        (false, None) => name.trim().to_string(),
    }
}

pub(super) fn editor_content_preview(content: &str, query: &str, active_match: usize) -> String {
    const MAX_PREVIEW_CHARS: usize = 16_000;
    if query.trim().is_empty() {
        let mut output = content.chars().take(MAX_PREVIEW_CHARS).collect::<String>();
        if content.chars().count() > MAX_PREVIEW_CHARS {
            output.push_str("\n\n-- preview truncated; full content is kept for saving --");
        }
        return output;
    }

    let matches = editor_search_matches(content, query);
    let Some(&byte_index) = matches.get(active_match.min(matches.len().saturating_sub(1))) else {
        return content.chars().take(MAX_PREVIEW_CHARS).collect();
    };
    let start = content[..byte_index]
        .char_indices()
        .rev()
        .nth(320)
        .map(|(index, _)| index)
        .unwrap_or(0);
    let end = content[byte_index..]
        .char_indices()
        .nth(960)
        .map(|(index, _)| byte_index + index)
        .unwrap_or(content.len());
    let mut output = String::new();
    if start > 0 {
        output.push_str("-- search preview --\n");
    }
    output.push_str(&content[start..end]);
    if end < content.len() {
        output.push_str("\n\n-- preview truncated; full content is kept for saving --");
    }
    output
}

pub(super) fn editor_search_matches(content: &str, query: &str) -> Vec<usize> {
    let query = query.trim();
    if query.is_empty() {
        return Vec::new();
    }
    content
        .match_indices(query)
        .map(|(index, _)| index)
        .take(10_000)
        .collect()
}

pub(super) fn format_permissions_octal(mode: u32) -> String {
    format!("{:04o}", mode & 0o7777)
}

pub(super) fn format_permissions_symbolic(file_type: SftpFileType, mode: u32) -> String {
    let mut output = String::with_capacity(10);
    output.push(match file_type {
        SftpFileType::Directory => 'd',
        SftpFileType::Symlink => 'l',
        _ => '-',
    });
    for shift in [6, 3, 0] {
        let bits = (mode >> shift) & 0o7;
        output.push(if bits & 0o4 != 0 { 'r' } else { '-' });
        output.push(if bits & 0o2 != 0 { 'w' } else { '-' });
        output.push(if bits & 0o1 != 0 { 'x' } else { '-' });
    }
    output
}

pub(super) fn transfer_job_row(
    job: TransferJobState,
    selected_remote_path: Option<String>,
    selected_job_id: Option<String>,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let status_color = match job.status {
        TransferJobStatus::Running => rgb(0xfacc15),
        TransferJobStatus::Paused => rgb(0x93c5fd),
        TransferJobStatus::Cancelling => rgb(0xfbbf24),
        TransferJobStatus::Cancelled => rgb(0x94a3b8),
        TransferJobStatus::Completed => rgb(0x34d399),
        TransferJobStatus::Failed => rgb(0xfb7185),
    };
    let job_selected = selected_job_id.as_deref() == Some(job.id.as_str());
    let direction = transfer_direction_label(&job.kind);
    let can_reveal_local_target = transfer_job_has_local_target(&job);
    let can_retry = transfer_job_can_retry(&job);
    let mut status_action = div().flex().items_center().gap_1();
    if job.status == TransferJobStatus::Running && job.control.is_some() {
        let job_id = job.id.clone();
        status_action = status_action.child(small_button(crate::ui::theme::theme_palette("github-dark"), 
            format!("transfer-pause-{job_id}"),
            "Pause",
            cx.listener(move |this, _, _, cx| {
                this.pause_transfer_job(&job_id, cx);
            }),
        ));
    }
    if job.status == TransferJobStatus::Paused && job.control.is_some() {
        let job_id = job.id.clone();
        status_action = status_action.child(small_button(crate::ui::theme::theme_palette("github-dark"), 
            format!("transfer-resume-{job_id}"),
            "Resume",
            cx.listener(move |this, _, _, cx| {
                this.resume_transfer_job(&job_id, cx);
            }),
        ));
    }
    if matches!(
        job.status,
        TransferJobStatus::Running | TransferJobStatus::Paused
    ) && job.control.is_some()
    {
        let job_id = job.id.clone();
        status_action = status_action.child(small_button(crate::ui::theme::theme_palette("github-dark"), 
            format!("transfer-cancel-{job_id}"),
            "Cancel",
            cx.listener(move |this, _, _, cx| {
                this.cancel_transfer_job(&job_id, cx);
            }),
        ));
    }
    if !matches!(
        job.status,
        TransferJobStatus::Running | TransferJobStatus::Paused | TransferJobStatus::Cancelling
    ) {
        if can_retry {
            let job_id = job.id.clone();
            status_action = status_action.child(small_button(crate::ui::theme::theme_palette("github-dark"), 
                format!("transfer-retry-job-{job_id}"),
                "Retry",
                cx.listener(move |this, _, window, cx| {
                    this.retry_transfer_job(job_id.clone(), window, cx);
                }),
            ));
        }
        let job_id = job.id.clone();
        status_action = status_action.child(small_button(crate::ui::theme::theme_palette("github-dark"), 
            format!("transfer-delete-job-{job_id}"),
            "Delete",
            cx.listener(move |this, _, _, cx| {
                this.request_delete_transfer_job(job_id.clone(), cx);
            }),
        ));
    }
    if can_reveal_local_target {
        let job_id = job.id.clone();
        status_action = status_action.child(small_button(crate::ui::theme::theme_palette("github-dark"), 
            format!("transfer-open-target-dir-{job_id}"),
            "Open Dir",
            cx.listener(move |this, _, _, cx| {
                this.reveal_transfer_job_target_directory(job_id.clone(), cx);
            }),
        ));
    }

    let mut entries = div().mt_2().flex().flex_col().gap_1();
    for entry in job.entries.iter().take(6) {
        let entry_path = entry.path.clone();
        let entry_name = entry.name.clone();
        let is_selected = selected_remote_path.as_deref() == Some(entry.path.as_str());
        entries = entries.child(
            div()
                .id(SharedString::from(format!("transfer-entry-{entry_path}")))
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .rounded_sm()
                .px_2()
                .py_1()
                .cursor_pointer()
                .bg(if is_selected {
                    rgb(0x15351f)
                } else {
                    rgb(0x10151e)
                })
                .text_xs()
                .text_color(if is_selected {
                    rgb(0xdcfce7)
                } else {
                    rgb(0xaeb7c8)
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.transfer_selected_remote_path = Some(entry_path.clone());
                    this.transfer_remote_path = entry_path.clone();
                    this.transfer_focused_field = TransferInputField::Remote;
                    this.terminal_status = format!("selected remote {entry_path}");
                    cx.notify();
                }))
                .child(entry_kind_label(entry.file_type))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .child(truncate_preview(&entry_name, 54)),
                )
                .child(format_file_size(entry.size)),
        );
    }
    if let Some(summary) = job.summary.as_ref() {
        entries = entries.child(
            div()
                .mt_2()
                .text_xs()
                .text_color(rgb(0xaeb7c8))
                .child(format!(
                    "{} -> {}",
                    summary.remote_path,
                    summary.local_path.display()
                )),
        );
    }

    let progress = job
        .progress
        .as_ref()
        .map(transfer_progress_bar)
        .unwrap_or_else(|| div().into_any_element());
    let progress_detail = job
        .progress
        .as_ref()
        .map(transfer_progress_percent_label)
        .unwrap_or_else(|| "-".to_string());

    div()
        .id(SharedString::from(format!("transfer-job-row-{}", job.id)))
        .border_b_1()
        .border_color(if job_selected {
            rgb(0x256d3f)
        } else {
            rgb(0x21262d)
        })
        .bg(if job_selected {
            rgb(0x10251d)
        } else {
            rgb(0x161b22)
        })
        .px_2()
        .py_2()
        .cursor_pointer()
        .on_click({
            let job_id = job.id.clone();
            cx.listener(move |this, _, window, cx| {
                window.focus(&this.transfer_queue_focus);
                this.select_transfer_job(job_id.clone(), cx);
            })
        })
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(status_pill(direction, rgb(0x93c5fd), rgb(0x17253b)))
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(0xe5edf7))
                                        .child(transfer_job_title(&job.kind)),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .text_xs()
                                .text_color(rgb(0xaeb7c8))
                                .child(job.detail.clone())
                                .child("·")
                                .child(progress_detail),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight(700.))
                                .text_color(status_color)
                                .child(transfer_status_label(job.status)),
                        )
                        .child(status_action),
                ),
        )
        .child(progress)
        .child(entries)
}

pub(super) fn transfer_browser_search_status(query: &str, visible: usize, total: usize) -> String {
    if query.trim().is_empty() {
        format!("{total} item(s)")
    } else {
        format!("{visible} of {total} item(s) match search")
    }
}

pub(super) fn sort_header_cell(
    column: TransferBrowserSortColumn,
    width: gpui::Pixels,
    active_column: TransferBrowserSortColumn,
    direction: TransferBrowserSortDirection,
    resizing_column: Option<TransferBrowserSortColumn>,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let is_active = column == active_column;
    let is_resizing = resizing_column == Some(column);
    let label = if is_active {
        format!("{} {}", column.label().to_uppercase(), direction.marker())
    } else {
        column.label().to_uppercase()
    };

    div()
        .id(SharedString::from(format!(
            "transfer-browser-sort-{}",
            column.label().to_lowercase()
        )))
        .h(px(28.))
        .w(width)
        .flex_none()
        .relative()
        .flex()
        .items_center()
        .px_2()
        .border_r_1()
        .border_color(rgb(0x21262d))
        .cursor_pointer()
        .bg(if is_active {
            rgb(0x17253b)
        } else {
            rgb(0x12171f)
        })
        .text_size(px(10.))
        .font_weight(FontWeight(800.))
        .text_color(if is_active {
            rgb(0x93c5fd)
        } else {
            rgb(0x64748b)
        })
        .hover(|this| this.bg(rgb(0x18202b)).text_color(rgb(0xdbeafe)))
        .on_click(cx.listener(move |this, _, _, cx| {
            this.toggle_transfer_browser_sort(column, cx);
        }))
        .child(label)
        .child(
            div()
                .id(SharedString::from(format!(
                    "transfer-browser-resize-{}",
                    column.label().to_lowercase()
                )))
                .absolute()
                .right(px(-3.))
                .top(px(4.))
                .bottom(px(4.))
                .w(px(7.))
                .rounded_sm()
                .cursor_col_resize()
                .bg(if is_resizing {
                    rgb(0x256d3f)
                } else {
                    rgb(0x1b2433)
                })
                .hover(|this| this.bg(rgb(0x256d3f)))
                .on_click(cx.listener(|_, _: &ClickEvent, _, cx| {
                    cx.stop_propagation();
                }))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                        cx.stop_propagation();
                        this.start_transfer_browser_column_resize(column, event, cx);
                    }),
                ),
        )
}

pub(super) fn compare_transfer_browser_entries(
    left: &SftpFileEntry,
    right: &SftpFileEntry,
    column: TransferBrowserSortColumn,
    direction: TransferBrowserSortDirection,
) -> Ordering {
    if left.file_type != right.file_type {
        let left_dir = left.file_type == SftpFileType::Directory;
        let right_dir = right.file_type == SftpFileType::Directory;
        if left_dir != right_dir {
            return if left_dir {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
    }

    let result = match column {
        TransferBrowserSortColumn::Name => natural_compare_ascii(&left.name, &right.name),
        TransferBrowserSortColumn::Size => left.size.unwrap_or(0).cmp(&right.size.unwrap_or(0)),
        TransferBrowserSortColumn::Modified => left
            .modified_at
            .unwrap_or(0)
            .cmp(&right.modified_at.unwrap_or(0)),
        TransferBrowserSortColumn::Permissions => left
            .permissions
            .unwrap_or(0)
            .cmp(&right.permissions.unwrap_or(0)),
        TransferBrowserSortColumn::Owner => natural_compare_ascii(&left.owner, &right.owner),
        TransferBrowserSortColumn::Group => natural_compare_ascii(&left.group, &right.group),
    };

    let directed = match direction {
        TransferBrowserSortDirection::Ascending => result,
        TransferBrowserSortDirection::Descending => result.reverse(),
    };
    directed.then_with(|| natural_compare_ascii(&left.name, &right.name))
}

pub(super) fn natural_compare_ascii(left: &str, right: &str) -> Ordering {
    let left = left.to_lowercase();
    let right = right.to_lowercase();
    let mut left_chars = left.chars().peekable();
    let mut right_chars = right.chars().peekable();

    loop {
        match (left_chars.peek().copied(), right_chars.peek().copied()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(left_char), Some(right_char))
                if left_char.is_ascii_digit() && right_char.is_ascii_digit() =>
            {
                let mut left_number = String::new();
                while let Some(value) = left_chars.peek().copied() {
                    if value.is_ascii_digit() {
                        left_number.push(value);
                        left_chars.next();
                    } else {
                        break;
                    }
                }
                let mut right_number = String::new();
                while let Some(value) = right_chars.peek().copied() {
                    if value.is_ascii_digit() {
                        right_number.push(value);
                        right_chars.next();
                    } else {
                        break;
                    }
                }
                let left_trimmed = left_number.trim_start_matches('0');
                let right_trimmed = right_number.trim_start_matches('0');
                let left_key = if left_trimmed.is_empty() {
                    "0"
                } else {
                    left_trimmed
                };
                let right_key = if right_trimmed.is_empty() {
                    "0"
                } else {
                    right_trimmed
                };
                let ordering = left_key
                    .len()
                    .cmp(&right_key.len())
                    .then_with(|| left_key.cmp(right_key))
                    .then_with(|| left_number.len().cmp(&right_number.len()));
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            (Some(left_char), Some(right_char)) => {
                left_chars.next();
                right_chars.next();
                let ordering = left_char.cmp(&right_char);
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
        }
    }
}

pub(super) fn transfer_queue_counts(
    jobs: &[TransferJobState],
) -> (usize, usize, usize, usize, usize) {
    let total = jobs.len();
    let running = jobs
        .iter()
        .filter(|job| {
            matches!(
                job.status,
                TransferJobStatus::Running | TransferJobStatus::Cancelling
            )
        })
        .count();
    let paused = jobs
        .iter()
        .filter(|job| job.status == TransferJobStatus::Paused)
        .count();
    let completed = jobs
        .iter()
        .filter(|job| job.status == TransferJobStatus::Completed)
        .count();
    let failed = jobs
        .iter()
        .filter(|job| job.status == TransferJobStatus::Failed)
        .count();
    (total, running, paused, completed, failed)
}

pub(super) fn queue_metric(
    palette: crate::ui::theme::ThemePalette,
    label: &'static str,
    value: usize,
    color: impl Into<Hsla>,
) -> impl IntoElement {
    let color = color.into();
    div()
        .flex()
        .items_center()
        .gap_1()
        .rounded_sm()
        .bg(rgb(palette.input))
        .px_2()
        .py_1()
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(palette.text_muted))
                .child(label),
        )
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight(800.))
                .text_color(color)
                .child(value.to_string()),
        )
}

pub(super) fn queue_action_button(
    id: impl Into<String>,
    label: &'static str,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(22.))
        .min_w(px(22.))
        .px_1()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .text_size(px(10.))
        .text_color(if enabled {
            rgb(0x8b949e)
        } else {
            rgb(0x484f58)
        })
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x21262d)).text_color(rgb(0xc9d1d9)))
        .opacity(if enabled { 1. } else { 0.4 })
        .when(enabled, |this| this.on_click(on_click))
        .child(label)
}



pub(super) fn duplicate_policy_short_label(policy: SftpDuplicatePolicy) -> &'static str {
    match policy {
        SftpDuplicatePolicy::Ask => "ask",
        SftpDuplicatePolicy::Overwrite => "overwrite",
        SftpDuplicatePolicy::Skip => "skip",
        SftpDuplicatePolicy::Rename => "rename",
    }
}

pub(super) fn transfer_direction_label(kind: &TransferJobKind) -> &'static str {
    match kind {
        TransferJobKind::ListDir { .. } => "LIST",
        TransferJobKind::ResolveHome => "HOME",
        TransferJobKind::SyncCwd => "CWD",
        TransferJobKind::Download { .. } => "DOWN",
        TransferJobKind::Upload { .. } => "UP",
        TransferJobKind::Rename { .. } => "REN",
        TransferJobKind::Move { .. } => "MOV",
        TransferJobKind::Delete { .. } => "DEL",
        TransferJobKind::Mkdir { .. } => "MKD",
        TransferJobKind::CreateFile { .. } => "NEW",
        TransferJobKind::Symlink { .. } => "LNK",
        TransferJobKind::LoadProperties { .. } => "GET",
        TransferJobKind::UpdateProperties { .. } => "SET",
        TransferJobKind::LoadEditor { .. } => "EDIT",
        TransferJobKind::SaveEditor { .. } => "SAVE",
        TransferJobKind::OpenExternal { .. } => "OPEN",
        TransferJobKind::AiFileAction { .. } => "AI",
    }
}

pub(super) fn transfer_job_has_local_target(job: &TransferJobState) -> bool {
    job.summary.is_some()
        || job.progress.is_some()
        || matches!(
            job.kind,
            TransferJobKind::Download { .. } | TransferJobKind::OpenExternal { .. }
        )
}

pub(super) fn transfer_job_can_retry(job: &TransferJobState) -> bool {
    matches!(
        job.status,
        TransferJobStatus::Failed | TransferJobStatus::Cancelled
    ) && matches!(
        job.kind,
        TransferJobKind::Download { .. } | TransferJobKind::Upload { .. }
    )
}

pub(super) fn transfer_progress_percent_label(progress: &SftpTransferProgress) -> String {
    match progress.total_bytes.filter(|total| *total > 0) {
        Some(total) => {
            let percent = (progress.bytes_transferred as f64 / total as f64 * 100.).clamp(0., 100.);
            format!("{percent:.0}%")
        }
        None => "streaming".to_string(),
    }
}

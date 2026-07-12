use gpui::{
    App, ClickEvent, FontStyle, FontWeight, HighlightStyle, IntoElement, SharedString,
    StrikethroughStyle, StyledText, UnderlineStyle, Window, WindowControlArea, div, prelude::*,
    px, rgb, rgba, svg,
};
use nyaterm_domain::{
    CloudSyncHistoryEntry, ConnectionType, NativeServiceStatus, SavedConnection, TunnelConfig,
    truncate_preview,
};
use nyaterm_session::{DockerContainer, NetworkInfo, RemoteProcess};

use crate::ui::components::{mode_button, small_button, status_pill};
use crate::ui::models::WorkspaceSplitDirection;

use super::{
    InlineMdStyle, MarkdownBlock, ThemePalette, cloud_sync_history_summary, cloud_sync_kind_text_color,
    cloud_sync_status_dot_color, cloud_sync_status_text_color, compact_id, docker_state_color,
    docker_state_label, format_cloud_provider, format_duration_ms, format_history_timestamp_ms,
    format_rate, parse_inline_markdown, parse_markdown_blocks, tunnel_endpoint, tunnel_mode_label,
    tunnel_name,
};

pub(in crate::ui::view) fn logo_mark(palette: ThemePalette) -> impl IntoElement {
    div()
        .size(px(22.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(
            svg()
                .size(px(18.))
                .path("icons/logo.svg")
                .text_color(rgb(palette.accent)),
        )
}

pub(in crate::ui::view) fn menu_bar_button(palette: ThemePalette,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("menu-{label}")))
        .h(px(26.))
        .px_2()
        .flex()
        .items_center()
        .rounded_sm()
        .text_size(px(12.))
        .text_color(rgb(palette.text_muted))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
        .child(label)
        .on_click(on_click)
}

pub(in crate::ui::view) fn status_bar_label(
    palette: ThemePalette,
    label: &'static str,
    value: impl Into<String>,
    value_color: impl Into<gpui::Hsla>,
) -> impl IntoElement {
    let value = value.into();
    let value_color = value_color.into();
    div()
        .h(px(18.))
        .flex()
        .items_center()
        .gap_1()
        .px_1()
        .rounded_sm()
        .text_size(px(10.))
        .text_color(rgb(palette.text_muted))
        .child(label)
        .child(
            div()
                .font_weight(FontWeight(700.))
                .text_color(value_color)
                .child(value),
        )
}

pub(in crate::ui::view) fn status_bar_button(
    palette: ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    value: impl Into<String>,
    value_color: impl Into<gpui::Hsla>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let value = value.into();
    let value_color = value_color.into();
    let hover_bg = palette.hover;
    let hover_text = palette.text;
    div()
        .id(SharedString::from(id.into()))
        .h(px(18.))
        .flex()
        .items_center()
        .gap_1()
        .px_1()
        .rounded_sm()
        .text_size(px(10.))
        .text_color(rgb(palette.text_muted))
        .cursor_pointer()
        .hover(move |this| this.bg(rgb(hover_bg)).text_color(rgb(hover_text)))
        .child(label)
        .child(
            div()
                .font_weight(FontWeight(700.))
                .text_color(value_color)
                .child(value),
        )
        .on_click(on_click)
}

pub(in crate::ui::view) fn window_control_button(palette: ThemePalette,
    id: &'static str,
    label: &'static str,
    area: WindowControlArea,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,) -> impl IntoElement {
    div()
        .id(SharedString::from(id))
        .w(px(46.))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .text_xs()
        .text_color(rgb(palette.text_muted))
        .window_control_area(area)
        .cursor_pointer()
        .hover(|this| {
            if matches!(area, WindowControlArea::Close) {
                this.bg(rgb(0xe81123)).text_color(rgb(0xffffff))
            } else {
                this.bg(rgb(palette.hover)).text_color(rgb(0xffffff))
            }
        })
        .child(label)
        .on_click(on_click)
}

pub(in crate::ui::view) fn panel_header(
    title: impl Into<SharedString>,
    meta: impl Into<SharedString>,
    palette: ThemePalette,
) -> impl IntoElement {
    // Tauri PanelHeader: min-h-9, uppercase tracked title + dimmed meta/actions.
    let title = title.into();
    let meta = meta.into();
    let show_meta = !meta.is_empty();
    div()
        .h(px(36.))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .px_3()
        .border_b_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.section_header))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .items_baseline()
                .gap_2()
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(palette.text_muted))
                        .child(title.to_uppercase()),
                )
                .when(show_meta, |this| {
                    this.child(
                        div()
                            .min_w_0()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .opacity(0.85)
                            .overflow_hidden()
                            .child(meta),
                    )
                }),
        )
}


/// Dimmed full-area modal shell (Tauri Dialog backdrop + centered card).
pub(in crate::ui::view) fn modal_dialog_shell(
    palette: ThemePalette,
    id: impl Into<String>,
    width: f32,
    content: impl IntoElement,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .absolute()
        .top_0()
        .bottom_0()
        .left_0()
        .right_0()
        .bg(rgba(0x030508d8))
        .flex()
        .items_center()
        .justify_center()
        .p_3()
        .child(
            div()
                .w(px(width))
                .max_w_full()
                .max_h_full()
                .rounded_md()
                .border_1()
                .border_color(rgb(palette.border))
                .bg(rgb(palette.bg))
                .shadow_lg()
                .child(content),
        )
}

/// Tauri ActionFooter-like Cancel/Save row.
pub(in crate::ui::view) fn modal_dialog_footer(
    palette: ThemePalette,
    cancel_id: impl Into<String>,
    save_id: impl Into<String>,
    save_label: &'static str,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_save: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .mt_1()
        .pt_3()
        .border_t_1()
        .border_color(rgb(palette.border))
        .flex()
        .items_center()
        .justify_end()
        .gap_2()
        .child(small_button(palette, cancel_id, "Cancel", on_cancel))
        .child(small_button(palette, save_id, save_label, on_save))
}

pub(in crate::ui::view) fn inspector_card(palette: ThemePalette, title: &'static str) -> gpui::Div {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.surface))
        .p_4()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight(800.))
                .text_color(rgb(palette.text))
                .child(title),
        )
}

pub(in crate::ui::view) fn inspector_status_line(palette: ThemePalette, text: String) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_3()
        .text_xs()
        .line_height(px(18.))
        .text_color(rgb(palette.text_muted))
        .child(text)
}

pub(in crate::ui::view) fn compact_network_rows(palette: ThemePalette, networks: &[NetworkInfo]) -> impl IntoElement {
    let mut rows = div().mt_3().flex().flex_col().gap_2();
    if networks.is_empty() {
        rows = rows.child(
            div()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child("No network snapshot loaded."),
        );
    } else {
        for network in networks.iter().take(4) {
            rows = rows.child(
                div()
                    .border_t_1()
                    .border_color(rgb(palette.border))
                    .pt_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(palette.text))
                                    .child(network.nic.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(palette.text_muted))
                                    .child(network.state.clone()),
                            ),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(format!(
                                "rx {} / tx {}",
                                format_rate(network.rx_bytes_per_sec),
                                format_rate(network.tx_bytes_per_sec)
                            )),
                    ),
            );
        }
    }
    rows
}

pub(in crate::ui::view) fn compact_process_rows(palette: ThemePalette, processes: &[RemoteProcess]) -> impl IntoElement {
    let mut rows = div().mt_3().flex().flex_col().gap_2();
    if processes.is_empty() {
        rows = rows.child(
            div()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child("No process snapshot loaded."),
        );
    } else {
        let mut processes = processes.to_vec();
        processes.sort_by(|left, right| {
            right
                .cpu_percent
                .partial_cmp(&left.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(left.pid.cmp(&right.pid))
        });
        for process in processes.into_iter().take(5) {
            rows = rows.child(
                div()
                    .border_t_1()
                    .border_color(rgb(palette.border))
                    .pt_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .min_w_0()
                                    .text_xs()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(palette.text))
                                    .child(truncate_preview(&process.command, 24)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(palette.success))
                                    .child(format!("{:.1}%", process.cpu_percent)),
                            ),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(format!(
                                "pid {} / mem {:.1}% / {}",
                                process.pid, process.memory_percent, process.user
                            )),
                    ),
            );
        }
    }
    rows
}

pub(in crate::ui::view) fn compact_docker_container_rows(palette: ThemePalette, containers: &[DockerContainer],) -> impl IntoElement {
    let mut rows = div().mt_3().flex().flex_col().gap_2();
    if containers.is_empty() {
        rows = rows.child(
            div()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child("No containers loaded."),
        );
    } else {
        for container in containers.iter().take(5) {
            rows = rows.child(
                div()
                    .border_t_1()
                    .border_color(rgb(palette.border))
                    .pt_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .min_w_0()
                                    .text_xs()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(palette.text))
                                    .child(truncate_preview(&container.name, 24)),
                            )
                            .child(status_pill(
                                docker_state_label(&container.state),
                                docker_state_color(palette, &container.state),
                                rgb(palette.hover),
                            )),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(truncate_preview(&container.image, 42)),
                    ),
            );
        }
    }
    rows
}

/// Tauri EmptyWorkspaceState row: action label (primary) + shortcut key chips.
pub(in crate::ui::view) fn empty_workspace_action(palette: ThemePalette,
    label: &'static str,
    shortcut: impl Into<String>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,) -> impl IntoElement {    // Tauri EmptyWorkspaceState: primary label left, Kbd chips right with "+" separators.
    let shortcut = shortcut.into();
    let parts: Vec<String> = shortcut
        .split('+')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect();
    let mut keys = div().flex().items_center().gap_1();
    for (index, part) in parts.into_iter().enumerate() {
        if index > 0 {
            keys = keys.child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text_dimmed))
                    .child("+"),
            );
        }
        keys = keys.child(
            div()
                .h(px(24.))
                .min_w(px(28.))
                .px_1()
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .border_1()
                .border_color(rgb(palette.border))
                .bg(rgb(palette.surface_elevated))
                .text_size(px(12.))
                .font_weight(FontWeight(600.))
                .text_color(rgb(palette.text))
                .child(part),
        );
    }

    // Tauri: grid-cols-[max-content_auto] gap-x-4 gap-y-3; label primary, kbd chips right.
    div()
        .id(SharedString::from(format!("empty-action-{label}")))
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .w_full()
        .max_w(px(480.))
        .cursor_pointer()
        .child(
            div()
                .min_w(px(160.))
                .text_sm()
                .font_weight(FontWeight(500.))
                .text_color(rgb(palette.accent))
                .hover(|this| this.text_color(rgb(palette.text)))
                .child(label),
        )
        .child(keys)
        .on_click(on_click)
}


pub(in crate::ui::view) fn tab_menu_item(
    palette: ThemePalette,
    id: impl Into<String>,
    label: impl Into<String>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    tab_menu_item_enabled(palette, id, label, true, on_click)
}

pub(in crate::ui::view) fn tab_menu_item_enabled(
    palette: ThemePalette,
    id: impl Into<String>,
    label: impl Into<String>,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let label = label.into();
    let text_color = if enabled {
        rgb(palette.text)
    } else {
        rgb(palette.text_dimmed)
    };
    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .text_size(px(12.))
        .text_color(text_color)
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(|this| this.bg(rgb(palette.hover)))
                .on_click(on_click)
        })
        .child(div().min_w_0().flex_1().child(label))
}

pub(in crate::ui::view) fn tab_menu_separator(palette: ThemePalette) -> impl IntoElement {
    div()
        .h(px(1.))
        .my_1()
        .mx_2()
        .bg(rgb(palette.border))
}

pub(in crate::ui::view) fn tab_action_button(palette: ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    detail: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .min_h(px(46.))
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.surface))
        .px_3()
        .py_2()
        .flex()
        .flex_col()
        .justify_center()
        .gap_1()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.hover)).border_color(rgb(0x3b82f6)))
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight(800.))
                .text_color(rgb(palette.text))
                .child(label),
        )
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(palette.text_muted))
                .child(detail),
        )
        .on_click(on_click)
}

pub(in crate::ui::view) fn split_divider(palette: ThemePalette, direction: WorkspaceSplitDirection) -> gpui::AnyElement {
    match direction {
        WorkspaceSplitDirection::Horizontal => div()
            .h(px(6.))
            .flex_none()
            .rounded_sm()
            .bg(rgb(palette.border))
            .into_any_element(),
        WorkspaceSplitDirection::Vertical => div()
            .w(px(6.))
            .flex_none()
            .rounded_sm()
            .bg(rgb(palette.border))
            .into_any_element(),
    }
}

pub(in crate::ui::view) fn stats_resource_row(palette: ThemePalette,
    label: &str,
    detail: &str,
    ratio: f64,) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_3()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .min_w_0()
                        .text_sm()
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(palette.text))
                        .child(truncate_preview(label, 36)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette.text_muted))
                        .child(format!("{:.0}%", ratio.clamp(0., 1.) * 100.)),
                ),
        )
        .child(
            div()
                .mt_1()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child(truncate_preview(detail, 96)),
        )
        .child(stats_progress_bar(palette, ratio))
}

pub(in crate::ui::view) fn stats_progress_bar(palette: ThemePalette, ratio: f64) -> impl IntoElement {
    let ratio = ratio.clamp(0., 1.);
    div()
        .mt_3()
        .h(px(6.))
        .w_full()
        .overflow_hidden()
        .rounded_sm()
        .bg(rgb(palette.border))
        .child(
            div()
                .h(px(6.))
                .w(px(220. * ratio as f32))
                .rounded_sm()
                .bg(if ratio >= 0.9 {
                    rgb(0xfb7185)
                } else if ratio >= 0.75 {
                    rgb(palette.warning)
                } else {
                    rgb(0x38bdf8)
                }),
        )
}

pub(in crate::ui::view) fn service_status(palette: ThemePalette, status: NativeServiceStatus) -> impl IntoElement {
    match status {
        NativeServiceStatus::Ready => {
            status_pill("ready", rgb(palette.success), rgb(palette.hover)).into_any_element()
        }
        NativeServiceStatus::Porting => {
            status_pill("porting", rgb(palette.warning), rgb(palette.hover)).into_any_element()
        }
        NativeServiceStatus::Blocked => {
            status_pill("replace", rgb(palette.danger), rgb(0x3a1717)).into_any_element()
        }
    }
}

pub(in crate::ui::view) fn metric(
    palette: ThemePalette,
    label: &'static str,
    value: impl Into<SharedString>,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.surface))
        .p_4()
        .child(div().text_xs().text_color(rgb(palette.text_muted)).child(label))
        .child(
            div()
                .mt_2()
                .text_2xl()
                .font_weight(FontWeight(800.))
                .text_color(rgb(palette.text))
                .child(value.into()),
        )
}

pub(in crate::ui::view) fn setting_state(palette: ThemePalette,
    label: &'static str,
    value: &'static str,) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.surface))
        .p_4()
        .child(div().text_xs().text_color(rgb(palette.text_muted)).child(label))
        .child(
            div()
                .mt_2()
                .text_lg()
                .font_weight(FontWeight(700.))
                .child(value),
        )
}

pub(in crate::ui::view) fn compact_setting_state(palette: ThemePalette,
    label: &'static str,
    value: String,) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_3()
        .child(div().text_xs().text_color(rgb(palette.text_muted)).child(label))
        .child(
            div()
                .mt_1()
                .text_sm()
                .font_weight(FontWeight(700.))
                .child(value),
        )
}

pub(in crate::ui::view) fn cloud_sync_history_row(
    palette: ThemePalette,
    entry: CloudSyncHistoryEntry,
    expanded: bool,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_copy: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let summary = cloud_sync_history_summary(&entry);
    let normalized = entry.message.split_whitespace().collect::<Vec<_>>().join(" ");
    let is_problem = matches!(entry.status.as_str(), "failed" | "conflict");
    let has_message_details = !normalized.is_empty()
        && (is_problem || normalized != summary.split_whitespace().collect::<Vec<_>>().join(" "));
    let has_expandable = has_message_details || entry.revision.as_ref().is_some_and(|r| !r.trim().is_empty());
    let kind_color = cloud_sync_kind_text_color(palette, &entry.kind);
    let status_color = cloud_sync_status_text_color(palette, &entry.status);
    let dot_color = cloud_sync_status_dot_color(palette, &entry.status);
    let timestamp = format_history_timestamp_ms(entry.timestamp_ms);
    let provider = entry
        .provider
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(format_cloud_provider)
        .unwrap_or_else(|| "—".to_string());
    let duration = format_duration_ms(entry.duration_ms).unwrap_or_else(|| "—".to_string());
    let kind_label = match entry.kind.as_str() {
        "sync" => "Sync",
        "backup" => "Backup",
        other => other,
    };
    let status_label = match entry.status.as_str() {
        "success" => "Success",
        "failed" => "Failed",
        "conflict" => "Conflict",
        "running" => "Running",
        other => other,
    };
    let revision = entry
        .revision
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(compact_id);
    let message = entry.message.clone();

    // Tauri SyncBackupHistory list: dense row, compact meta chips, copy on expand.
    div()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(rgb(palette.surface_elevated))
        .child(
            div()
                .flex()
                .items_start()
                .gap_2()
                .child(
                    div()
                        .mt(px(5.))
                        .size(px(6.))
                        .rounded_full()
                        .flex_none()
                        .bg(dot_color),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .font_weight(FontWeight(700.))
                                        .text_color(kind_color)
                                        .child(kind_label.to_string()),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .text_color(status_color)
                                        .child(status_label.to_string()),
                                )
                                .child(
                                    div()
                                        .ml_auto()
                                        .flex_none()
                                        .text_size(px(10.))
                                        .text_color(rgb(palette.text_dimmed))
                                        .child(timestamp),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight(600.))
                                .text_color(rgb(palette.text))
                                .overflow_hidden()
                                .child(truncate_preview(&summary, 96)),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap_x_3()
                                .gap_y_1()
                                .text_size(px(10.))
                                .text_color(rgb(palette.text_dimmed))
                                .child(format!("Trigger {}", entry.trigger))
                                .child(format!("Provider {provider}"))
                                .child(format!("Duration {duration}")),
                        )
                        .when(has_expandable, |this| {
                            this.child(
                                div()
                                    .mt_1()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(
                                        div()
                                            .id(SharedString::from(format!(
                                                "sync-history-toggle-{}",
                                                entry.id
                                            )))
                                            .h(px(22.))
                                            .flex()
                                            .items_center()
                                            .text_size(px(10.))
                                            .text_color(rgb(palette.text_muted))
                                            .cursor_pointer()
                                            .hover(|style| style.text_color(rgb(palette.text)))
                                            .child(if expanded {
                                                "Hide details"
                                            } else {
                                                "View details"
                                            })
                                            .on_click(on_toggle),
                                    )
                                    .when(expanded && has_message_details, |this| {
                                        this.child(
                                            div()
                                                .id(SharedString::from(format!(
                                                    "sync-history-copy-{}",
                                                    entry.id
                                                )))
                                                .h(px(22.))
                                                .flex()
                                                .items_center()
                                                .text_size(px(10.))
                                                .text_color(rgb(palette.text_muted))
                                                .cursor_pointer()
                                                .hover(|style| style.text_color(rgb(palette.text)))
                                                .child("Copy message")
                                                .on_click(on_copy),
                                        )
                                    }),
                            )
                        })
                        .when(expanded && has_message_details, |this| {
                            this.child(
                                div()
                                    .mt_1()
                                    .rounded_md()
                                    .p_2()
                                    .bg(if is_problem {
                                        rgb(0x2a1215)
                                    } else {
                                        rgb(palette.surface)
                                    })
                                    .font_family("JetBrains Mono")
                                    .text_size(px(10.))
                                    .text_color(if is_problem {
                                        rgb(0xffa198)
                                    } else {
                                        rgb(palette.text_muted)
                                    })
                                    .child(message),
                            )
                        })
                        .when(expanded && revision.is_some(), |this| {
                            this.child(
                                div()
                                    .mt_1()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(palette.border))
                                    .bg(rgb(palette.bg))
                                    .px_2()
                                    .py_1()
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(rgb(palette.text_dimmed))
                                            .child("Revision"),
                                    )
                                    .child(
                                        div()
                                            .mt_0()
                                            .font_family("JetBrains Mono")
                                            .text_size(px(11.))
                                            .text_color(rgb(palette.text))
                                            .child(revision.unwrap_or_default()),
                                    ),
                            )
                        }),
                ),
        )
}

pub(in crate::ui::view) fn policy_button(palette: ThemePalette,
    id: &'static str,
    label: &'static str,
    selected: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,) -> impl IntoElement {
    div()
        .id(SharedString::from(id))
        .h(px(30.))
        .px_3()
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(if selected {
            rgb(0x4ade80)
        } else {
            rgb(palette.border)
        })
        .bg(if selected {
            rgb(0x173823)
        } else {
            rgb(palette.surface)
        })
        .text_color(if selected {
            rgb(palette.success)
        } else {
            rgb(palette.text)
        })
        .text_xs()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.hover)))
        .child(label)
        .on_click(on_click)
}

pub(in crate::ui::view) fn connection_row(palette: ThemePalette,
    connection: &SavedConnection,
    selected: bool,
    on_select: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_connect: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,) -> impl IntoElement {
    let can_connect = matches!(
        connection.config,
        ConnectionType::Ssh { .. }
            | ConnectionType::LocalTerminal { .. }
            | ConnectionType::Telnet { .. }
            | ConnectionType::Serial { .. }
    );
    let action = if can_connect {
        small_button(palette, format!("connect-{}", connection.id), "Connect", on_connect).into_any_element()
    } else {
        status_pill("porting", rgb(palette.warning), rgb(palette.hover)).into_any_element()
    };

    div()
        .flex()
        .items_center()
        .justify_between()
        .rounded_md()
        .border_1()
        .border_color(if selected {
            rgb(0x3b82f6)
        } else {
            rgb(palette.border)
        })
        .bg(if selected {
            rgb(palette.hover)
        } else {
            rgb(palette.surface)
        })
        .p_3()
        .hover(|this| this.bg(rgb(palette.hover)))
        .child(
            div()
                .min_w_0()
                .flex()
                .items_center()
                .gap_3()
                .child(mode_button(
                    format!("select-connection-{}", connection.id),
                    if selected { "Selected" } else { "Select" },
                    selected,
                    palette,
                    on_select,
                ))
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight(700.))
                                .child(connection.name.clone()),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(palette.text_muted))
                                .child(connection.endpoint()),
                        ),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(status_pill(
                    connection.kind_label(),
                    rgb(palette.accent),
                    rgb(palette.hover),
                ))
                .child(action),
        )
}

pub(in crate::ui::view) fn compact_connection_row(palette: ThemePalette,
    connection: &SavedConnection,
    on_connect: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_3()
        .hover(|this| this.bg(rgb(palette.hover)))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight(800.))
                                .child(truncate_preview(&connection.name, 28)),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(palette.text_muted))
                                .child(truncate_preview(&connection.endpoint(), 36)),
                        ),
                )
                .child(status_pill(
                    connection.kind_label(),
                    rgb(palette.accent),
                    rgb(palette.hover),
                )),
        )
        .child(
            div()
                .mt_2()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(div().text_xs().text_color(rgb(palette.text_muted)).child(
                    connection.description.clone().unwrap_or_else(|| {
                        match connection.group_id.as_deref() {
                            Some(group) if !group.trim().is_empty() => {
                                format!("group {group}")
                            }
                            _ => "ungrouped".to_string(),
                        }
                    }),
                ))
                .child(small_button(palette, 
                    format!("left-connect-{}", connection.id),
                    "Connect",
                    on_connect,
                )),
        )
}

pub(in crate::ui::view) fn compact_tunnel_row(palette: ThemePalette,
    tunnel: &TunnelConfig,
    is_open: bool,
    is_pending: bool,
    on_open: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,) -> impl IntoElement {
    let status = if is_pending {
        "pending"
    } else if is_open {
        "open"
    } else {
        "closed"
    };
    let (status_fg, status_bg) = if is_pending {
        (rgb(palette.warning), rgb(palette.hover))
    } else if is_open {
        (rgb(palette.success), rgb(palette.hover))
    } else {
        (rgb(palette.text_muted), rgb(palette.border))
    };

    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_3()
        .hover(|this| this.bg(rgb(palette.hover)))
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .text_sm()
                        .font_weight(FontWeight(800.))
                        .child(truncate_preview(&tunnel_name(tunnel), 30)),
                )
                .child(status_pill(status, status_fg, status_bg)),
        )
        .child(
            div()
                .mt_1()
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child(truncate_preview(
                    &tunnel_endpoint(tunnel, &tunnel.listen_port.to_string()),
                    42,
                )),
        )
        .child(
            div()
                .mt_2()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(status_pill(
                    tunnel_mode_label(tunnel),
                    rgb(palette.accent),
                    rgb(palette.hover),
                ))
                .child(if is_open {
                    small_button(palette, 
                        format!("left-tunnel-close-{}", tunnel.id),
                        "Close",
                        on_close,
                    )
                    .into_any_element()
                } else {
                    small_button(palette, format!("left-tunnel-open-{}", tunnel.id), "Open", on_open)
                        .into_any_element()
                }),
        )
}

/// Monochrome activity-bar SVG icon with glyph fallback.
pub(in crate::ui::view) fn activity_icon(
    path: Option<&'static str>,
    glyph: &'static str,
    color: gpui::Hsla,
    size_px: f32,
) -> gpui::AnyElement {
    let size = px(size_px);
    if let Some(path) = path {
        svg()
            .size(size)
            .flex_none()
            .path(path)
            .text_color(color)
            .into_any_element()
    } else {
        div()
            .size(size)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(size_px * 0.72))
            .font_weight(FontWeight(700.))
            .text_color(color)
            .child(glyph)
            .into_any_element()
    }
}


/// Compact ghost icon button with bundled SVG (Tauri ToolbarIconButton h-7).
pub(in crate::ui::view) fn toolbar_svg_button(palette: ThemePalette,
    id: impl Into<SharedString>,
    icon_path: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,) -> impl IntoElement {
    div()
        .id(id.into())
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(rgb(palette.text_muted))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface_elevated)).text_color(rgb(palette.text)))
        .child(
            svg()
                .size(px(16.))
                .flex_none()
                .path(icon_path)
                .text_color(rgb(palette.text_muted)),
        )
        .on_click(on_click)
}

/// Faded NyaTerm logo used by empty workspace (Tauri EmptyWorkspaceState).
pub(in crate::ui::view) fn nyaterm_logo_mark(palette: ThemePalette, size_px: f32, opacity: f32) -> impl IntoElement {
    let size = px(size_px);
    div()
        .size(size)
        .flex_none()
        .opacity(opacity)
        .flex()
        .items_center()
        .justify_center()
        .child(
            svg()
                .size(size)
                .path("icons/logo.svg")
                .text_color(rgb(palette.text_muted)),
        )
}


/// Colored connection/OS icon for saved connection rows (Tauri resolveConnectionIcon).
pub(in crate::ui::view) fn connection_type_icon(palette: ThemePalette,
    def: super::ConnectionIconDef,
    selected: bool,
    size_px: f32,) -> gpui::AnyElement {
    let size = px(size_px);
    let color = if selected {
        rgb(palette.accent)
    } else {
        rgb(def.color)
    };
    svg()
        .size(size)
        .flex_none()
        .path(def.path)
        .text_color(color)
        .into_any_element()
}


/// Lightweight GFM markdown renderer for AI transcript (Tauri MarkdownContent parity).
pub(in crate::ui::view) fn markdown_content_view(palette: ThemePalette, content: &str) -> impl IntoElement {
    let blocks = parse_markdown_blocks(content);
    let mut root = div()
        .flex()
        .flex_col()
        .gap_1()
        .text_size(px(12.))
        .line_height(px(18.));
    if blocks.is_empty() {
        return root;
    }
    for (index, block) in blocks.into_iter().enumerate() {
        root = root.child(markdown_block_view(palette, index, block));
    }
    root
}

fn markdown_inline_text(palette: ThemePalette, raw: &str) -> gpui::AnyElement {
    let parsed = parse_inline_markdown(raw);
    if parsed.highlights.is_empty() {
        return div().child(parsed.text).into_any_element();
    }
    let highlights = parsed.highlights.into_iter().map(|(range, style)| {
        let highlight = match style {
            InlineMdStyle::Bold => HighlightStyle {
                font_weight: Some(FontWeight(700.)),
                ..Default::default()
            },
            InlineMdStyle::Italic => HighlightStyle {
                font_style: Some(FontStyle::Italic),
                ..Default::default()
            },
            InlineMdStyle::BoldItalic => HighlightStyle {
                font_weight: Some(FontWeight(700.)),
                font_style: Some(FontStyle::Italic),
                ..Default::default()
            },
            InlineMdStyle::Code => HighlightStyle {
                color: Some(rgb(palette.text).into()),
                background_color: Some(rgb(palette.surface_elevated).into()),
                font_weight: Some(FontWeight(500.)),
                ..Default::default()
            },
            InlineMdStyle::Link => HighlightStyle {
                color: Some(rgb(palette.accent).into()),
                underline: Some(UnderlineStyle {
                    thickness: px(1.),
                    color: Some(rgb(palette.accent).into()),
                    wavy: false,
                }),
                ..Default::default()
            },
            InlineMdStyle::Strike => HighlightStyle {
                strikethrough: Some(StrikethroughStyle {
                    thickness: px(1.),
                    color: Some(rgb(palette.text_muted).into()),
                }),
                color: Some(rgb(palette.text_muted).into()),
                ..Default::default()
            },
        };
        (range, highlight)
    });
    StyledText::new(parsed.text)
        .with_highlights(highlights)
        .into_any_element()
}

fn markdown_block_view(palette: ThemePalette, index: usize, block: MarkdownBlock) -> gpui::AnyElement {
    match block {
        MarkdownBlock::Paragraph(text) => div()
            .id(SharedString::from(format!("md-p-{index}")))
            .text_size(px(12.))
            .text_color(rgb(palette.text))
            .line_height(px(18.))
            .child(markdown_inline_text(palette, &text))
            .into_any_element(),
        MarkdownBlock::Bullet(text) => div()
            .id(SharedString::from(format!("md-ul-{index}")))
            .flex()
            .items_start()
            .gap_2()
            .pl_1()
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(palette.text_muted))
                    .child("•"),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .text_size(px(12.))
                    .text_color(rgb(palette.text))
                    .line_height(px(18.))
                    .child(markdown_inline_text(palette, &text)),
            )
            .into_any_element(),
        MarkdownBlock::Numbered { index: n, text } => div()
            .id(SharedString::from(format!("md-ol-{index}")))
            .flex()
            .items_start()
            .gap_2()
            .pl_1()
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(palette.text_muted))
                    .child(format!("{n}.")),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .text_size(px(12.))
                    .text_color(rgb(palette.text))
                    .line_height(px(18.))
                    .child(markdown_inline_text(palette, &text)),
            )
            .into_any_element(),
        MarkdownBlock::Code { language, code } => div()
            .id(SharedString::from(format!("md-code-{index}")))
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.bg))
            .overflow_hidden()
            .max_h(px(256.))
            .child(
                div()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .text_size(px(10.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(palette.text_dimmed))
                    .child(if language.trim().is_empty() {
                        "code".to_string()
                    } else {
                        language
                    }),
            )
            .child(
                div()
                    .px_2()
                    .py_2()
                    .font_family("JetBrains Mono")
                    .text_size(px(11.))
                    .text_color(rgb(palette.text))
                    .line_height(px(16.))
                    .child(code),
            )
            .into_any_element(),
        MarkdownBlock::Quote(text) => {
            let mut body = div().flex().flex_col().gap_1();
            for (qi, line) in text.lines().enumerate() {
                body = body.child(
                    div()
                        .id(SharedString::from(format!("md-q-{index}-{qi}")))
                        .child(markdown_inline_text(palette, line)),
                );
            }
            div()
                .id(SharedString::from(format!("md-q-{index}")))
                .pl_3()
                .border_l_2()
                .border_color(rgb(palette.border))
                .text_size(px(12.))
                .text_color(rgb(palette.text_muted))
                .line_height(px(18.))
                .child(body)
                .into_any_element()
        }
        MarkdownBlock::Heading { level, text } => {
            let size = match level {
                1 => 16.,
                2 => 14.,
                _ => 13.,
            };
            div()
                .id(SharedString::from(format!("md-h-{index}")))
                .text_size(px(size))
                .font_weight(FontWeight(800.))
                .text_color(rgb(palette.text))
                .line_height(px(size + 4.))
                .child(markdown_inline_text(palette, &text))
                .into_any_element()
        }
        MarkdownBlock::Table { headers, rows } => {
            let col_count = headers
                .len()
                .max(rows.iter().map(|r| r.len()).max().unwrap_or(0))
                .max(1);
            let mut table = div()
                .id(SharedString::from(format!("md-table-{index}")))
                .flex()
                .flex_col()
                .border_1()
                .border_color(rgb(palette.border))
                .rounded_md()
                .overflow_hidden();

            let mut header_row = div()
                .flex()
                .bg(rgb(palette.surface))
                .border_b_1()
                .border_color(rgb(palette.border));
            for col in 0..col_count {
                let cell = headers.get(col).cloned().unwrap_or_default();
                header_row = header_row.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .px_2()
                        .py_1()
                        .border_r_1()
                        .border_color(rgb(palette.border))
                        .text_size(px(11.))
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(palette.text))
                        .child(markdown_inline_text(palette, &cell)),
                );
            }
            table = table.child(header_row);

            for (ri, row) in rows.into_iter().enumerate() {
                let mut body_row = div()
                    .flex()
                    .border_b_1()
                    .border_color(rgb(palette.surface_elevated))
                    .bg(if ri % 2 == 0 {
                        rgb(palette.bg)
                    } else {
                        rgb(palette.section_header)
                    });
                for col in 0..col_count {
                    let cell = row.get(col).cloned().unwrap_or_default();
                    body_row = body_row.child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .px_2()
                            .py_1()
                            .border_r_1()
                            .border_color(rgb(palette.surface_elevated))
                            .text_size(px(11.))
                            .text_color(rgb(palette.text))
                            .child(markdown_inline_text(palette, &cell)),
                    );
                }
                table = table.child(body_row);
            }
            table.into_any_element()
        }
        MarkdownBlock::ThematicBreak => div()
            .id(SharedString::from(format!("md-hr-{index}")))
            .my_1()
            .h(px(1.))
            .w_full()
            .bg(rgb(palette.border))
            .into_any_element(),
    }
}


/// Tauri file explorer entry icon (folder / symlink / file).
pub(in crate::ui::view) fn transfer_entry_icon(
    is_directory: bool,
    is_symlink: bool,
    selected: bool,
) -> gpui::AnyElement {
    let (path, color) = if is_symlink {
        ("icons/conn/symlink.svg", 0x67e8f9u32)
    } else if is_directory {
        ("icons/conn/folder.svg", 0xfbbf24u32)
    } else {
        ("icons/conn/file.svg", 0x94a3b8u32)
    };
    let color = if selected { 0x58a6ffu32 } else { color };
    svg()
        .size(px(14.))
        .flex_none()
        .path(path)
        .text_color(rgb(color))
        .into_any_element()
}

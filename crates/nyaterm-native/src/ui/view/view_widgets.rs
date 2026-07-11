use gpui::{
    App, ClickEvent, FontWeight, IntoElement, SharedString, Window, WindowControlArea, div,
    prelude::*, px, rgb, rgba, svg,
};
use nyaterm_domain::{
    CloudSyncHistoryEntry, ConnectionType, NativeServiceStatus, SavedConnection, TunnelConfig,
    truncate_preview,
};
use nyaterm_session::{DockerContainer, NetworkInfo, RemoteProcess};

use crate::ui::components::{mode_button, small_button, status_pill};
use crate::ui::models::WorkspaceSplitDirection;

use super::{
    MarkdownBlock, compact_id, docker_state_color, docker_state_label, format_rate,
    parse_markdown_blocks, tunnel_endpoint, tunnel_mode_label, tunnel_name,
};

pub(in crate::ui::view) fn logo_mark() -> impl IntoElement {
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
                .text_color(rgb(0x58a6ff)),
        )
}

pub(in crate::ui::view) fn menu_bar_button(
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("menu-{label}")))
        .h(px(26.))
        .px_2()
        .flex()
        .items_center()
        .rounded_sm()
        .text_size(px(12.))
        .text_color(rgb(0x8b949e))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x1c2128)).text_color(rgb(0xc9d1d9)))
        .child(label)
        .on_click(on_click)
}

pub(in crate::ui::view) fn status_bar_label(
    label: &'static str,
    value: impl Into<String>,
    value_color: impl Into<gpui::Hsla>,
) -> impl IntoElement {
    let value = value.into();
    let value_color = value_color.into();
    div()
        .h(px(20.))
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .rounded_sm()
        .text_xs()
        .text_color(rgb(0x8b949e))
        .child(label)
        .child(
            div()
                .font_weight(FontWeight(800.))
                .text_color(value_color)
                .child(value),
        )
}

pub(in crate::ui::view) fn status_bar_button(
    id: impl Into<String>,
    label: &'static str,
    value: impl Into<String>,
    value_color: impl Into<gpui::Hsla>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let value = value.into();
    let value_color = value_color.into();
    div()
        .id(SharedString::from(id.into()))
        .h(px(20.))
        .flex()
        .items_center()
        .gap_1()
        .px_2()
        .rounded_sm()
        .text_xs()
        .text_color(rgb(0x8b949e))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x1c2128)).text_color(rgb(0xc9d1d9)))
        .child(label)
        .child(
            div()
                .font_weight(FontWeight(800.))
                .text_color(value_color)
                .child(value),
        )
        .on_click(on_click)
}

pub(in crate::ui::view) fn window_control_button(
    id: &'static str,
    label: &'static str,
    area: WindowControlArea,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id))
        .w(px(46.))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .text_xs()
        .text_color(rgb(0x8b949e))
        .window_control_area(area)
        .cursor_pointer()
        .hover(|this| {
            if matches!(area, WindowControlArea::Close) {
                this.bg(rgb(0xe81123)).text_color(rgb(0xffffff))
            } else {
                this.bg(rgb(0x1c2128)).text_color(rgb(0xffffff))
            }
        })
        .child(label)
        .on_click(on_click)
}

pub(in crate::ui::view) fn panel_header(
    title: impl Into<SharedString>,
    meta: impl Into<SharedString>,
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
        .border_color(rgb(0x30363d))
        .bg(rgb(0x12171f))
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
                        .text_color(rgb(0x8b949e))
                        .child(title.to_uppercase()),
                )
                .when(show_meta, |this| {
                    this.child(
                        div()
                            .min_w_0()
                            .text_size(px(11.))
                            .text_color(rgb(0x6e7681))
                            .overflow_hidden()
                            .child(meta),
                    )
                }),
        )
}


/// Dimmed full-area modal shell (Tauri Dialog backdrop + centered card).
pub(in crate::ui::view) fn modal_dialog_shell(
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
                .border_color(rgb(0x30363d))
                .bg(rgb(0x0d1117))
                .shadow_lg()
                .child(content),
        )
}

/// Tauri ActionFooter-like Cancel/Save row.
pub(in crate::ui::view) fn modal_dialog_footer(
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
        .border_color(rgb(0x30363d))
        .flex()
        .items_center()
        .justify_end()
        .gap_2()
        .child(small_button(cancel_id, "Cancel", on_cancel))
        .child(small_button(save_id, save_label, on_save))
}

pub(in crate::ui::view) fn inspector_card(title: &'static str) -> gpui::Div {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x151923))
        .p_4()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight(800.))
                .text_color(rgb(0xc9d1d9))
                .child(title),
        )
}

pub(in crate::ui::view) fn inspector_status_line(text: String) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x273244))
        .bg(rgb(0x10151e))
        .p_3()
        .text_xs()
        .line_height(px(18.))
        .text_color(rgb(0x98a3b8))
        .child(text)
}

pub(in crate::ui::view) fn compact_network_rows(networks: &[NetworkInfo]) -> impl IntoElement {
    let mut rows = div().mt_3().flex().flex_col().gap_2();
    if networks.is_empty() {
        rows = rows.child(
            div()
                .text_xs()
                .text_color(rgb(0x98a3b8))
                .child("No network snapshot loaded."),
        );
    } else {
        for network in networks.iter().take(4) {
            rows = rows.child(
                div()
                    .border_t_1()
                    .border_color(rgb(0x2a3140))
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
                                    .text_color(rgb(0xc9d1d9))
                                    .child(network.nic.clone()),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x64748b))
                                    .child(network.state.clone()),
                            ),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_xs()
                            .text_color(rgb(0x98a3b8))
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

pub(in crate::ui::view) fn compact_process_rows(processes: &[RemoteProcess]) -> impl IntoElement {
    let mut rows = div().mt_3().flex().flex_col().gap_2();
    if processes.is_empty() {
        rows = rows.child(
            div()
                .text_xs()
                .text_color(rgb(0x98a3b8))
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
                    .border_color(rgb(0x2a3140))
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
                                    .text_color(rgb(0xc9d1d9))
                                    .child(truncate_preview(&process.command, 24)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x6ee7b7))
                                    .child(format!("{:.1}%", process.cpu_percent)),
                            ),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_xs()
                            .text_color(rgb(0x98a3b8))
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

pub(in crate::ui::view) fn compact_docker_container_rows(
    containers: &[DockerContainer],
) -> impl IntoElement {
    let mut rows = div().mt_3().flex().flex_col().gap_2();
    if containers.is_empty() {
        rows = rows.child(
            div()
                .text_xs()
                .text_color(rgb(0x98a3b8))
                .child("No containers loaded."),
        );
    } else {
        for container in containers.iter().take(5) {
            rows = rows.child(
                div()
                    .border_t_1()
                    .border_color(rgb(0x2a3140))
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
                                    .text_color(rgb(0xc9d1d9))
                                    .child(truncate_preview(&container.name, 24)),
                            )
                            .child(status_pill(
                                docker_state_label(&container.state),
                                docker_state_color(&container.state),
                                rgb(0x17253b),
                            )),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_xs()
                            .text_color(rgb(0x98a3b8))
                            .child(truncate_preview(&container.image, 42)),
                    ),
            );
        }
    }
    rows
}

/// Tauri EmptyWorkspaceState row: action label (primary) + shortcut key chips.
pub(in crate::ui::view) fn empty_workspace_action(
    label: &'static str,
    shortcut: impl Into<String>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    // Tauri EmptyWorkspaceState: primary label left, Kbd chips right with "+" separators.
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
                    .text_color(rgb(0x6e7681))
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
                .border_color(rgb(0x30363d))
                .bg(rgb(0x21262d))
                .text_size(px(12.))
                .font_weight(FontWeight(600.))
                .text_color(rgb(0xc9d1d9))
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
                .text_color(rgb(0xc9d1d9))
                .hover(|this| this.text_color(rgb(0x58a6ff)))
                .child(label),
        )
        .child(keys)
        .on_click(on_click)
}


pub(in crate::ui::view) fn tab_action_button(
    id: impl Into<String>,
    label: &'static str,
    detail: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .min_h(px(46.))
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x303848))
        .bg(rgb(0x151b27))
        .px_3()
        .py_2()
        .flex()
        .flex_col()
        .justify_center()
        .gap_1()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x223047)).border_color(rgb(0x3b82f6)))
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight(800.))
                .text_color(rgb(0xc9d1d9))
                .child(label),
        )
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(0x8b949e))
                .child(detail),
        )
        .on_click(on_click)
}

pub(in crate::ui::view) fn split_divider(direction: WorkspaceSplitDirection) -> gpui::AnyElement {
    match direction {
        WorkspaceSplitDirection::Horizontal => div()
            .h(px(6.))
            .flex_none()
            .rounded_sm()
            .bg(rgb(0x202633))
            .into_any_element(),
        WorkspaceSplitDirection::Vertical => div()
            .w(px(6.))
            .flex_none()
            .rounded_sm()
            .bg(rgb(0x202633))
            .into_any_element(),
    }
}

pub(in crate::ui::view) fn stats_resource_row(
    label: &str,
    detail: &str,
    ratio: f64,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x10151e))
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
                        .text_color(rgb(0xc9d1d9))
                        .child(truncate_preview(label, 36)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x98a3b8))
                        .child(format!("{:.0}%", ratio.clamp(0., 1.) * 100.)),
                ),
        )
        .child(
            div()
                .mt_1()
                .text_xs()
                .text_color(rgb(0x98a3b8))
                .child(truncate_preview(detail, 96)),
        )
        .child(stats_progress_bar(ratio))
}

pub(in crate::ui::view) fn stats_progress_bar(ratio: f64) -> impl IntoElement {
    let ratio = ratio.clamp(0., 1.);
    div()
        .mt_3()
        .h(px(6.))
        .w_full()
        .overflow_hidden()
        .rounded_sm()
        .bg(rgb(0x242b38))
        .child(
            div()
                .h(px(6.))
                .w(px(220. * ratio as f32))
                .rounded_sm()
                .bg(if ratio >= 0.9 {
                    rgb(0xfb7185)
                } else if ratio >= 0.75 {
                    rgb(0xfacc15)
                } else {
                    rgb(0x38bdf8)
                }),
        )
}

pub(in crate::ui::view) fn service_status(status: NativeServiceStatus) -> impl IntoElement {
    match status {
        NativeServiceStatus::Ready => {
            status_pill("ready", rgb(0x6ee7b7), rgb(0x12342a)).into_any_element()
        }
        NativeServiceStatus::Porting => {
            status_pill("porting", rgb(0xfbbf24), rgb(0x3a2f14)).into_any_element()
        }
        NativeServiceStatus::Blocked => {
            status_pill("replace", rgb(0xfca5a5), rgb(0x3a1717)).into_any_element()
        }
    }
}

pub(in crate::ui::view) fn metric(
    label: &'static str,
    value: impl Into<SharedString>,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x151923))
        .p_4()
        .child(div().text_xs().text_color(rgb(0x98a3b8)).child(label))
        .child(
            div()
                .mt_2()
                .text_2xl()
                .font_weight(FontWeight(800.))
                .child(value.into()),
        )
}

pub(in crate::ui::view) fn setting_state(
    label: &'static str,
    value: &'static str,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x151923))
        .p_4()
        .child(div().text_xs().text_color(rgb(0x98a3b8)).child(label))
        .child(
            div()
                .mt_2()
                .text_lg()
                .font_weight(FontWeight(700.))
                .child(value),
        )
}

pub(in crate::ui::view) fn compact_setting_state(
    label: &'static str,
    value: String,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x111722))
        .p_3()
        .child(div().text_xs().text_color(rgb(0x98a3b8)).child(label))
        .child(
            div()
                .mt_1()
                .text_sm()
                .font_weight(FontWeight(700.))
                .child(value),
        )
}

pub(in crate::ui::view) fn cloud_sync_history_row(
    entry: CloudSyncHistoryEntry,
) -> impl IntoElement {
    let status_color = match entry.status.as_str() {
        "success" => rgb(0x86efac),
        "conflict" => rgb(0xfacc15),
        "failed" => rgb(0xfca5a5),
        _ => rgb(0xcbd5e1),
    };
    let provider = entry
        .provider
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or("unknown");
    let revision = entry
        .revision
        .as_deref()
        .map(compact_id)
        .unwrap_or_else(|| "no revision".to_string());
    let duration = entry
        .duration_ms
        .map(|value| format!(" / {value} ms"))
        .unwrap_or_default();
    let meta = format!("{} / {provider} / {revision}{duration}", entry.trigger);

    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x273244))
        .bg(rgb(0x111722))
        .p_3()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .text_xs()
                        .font_weight(FontWeight(700.))
                        .text_color(status_color)
                        .child(entry.status),
                )
                .child(div().text_xs().text_color(rgb(0x98a3b8)).child(entry.kind)),
        )
        .child(
            div()
                .mt_1()
                .text_sm()
                .font_weight(FontWeight(700.))
                .child(entry.message),
        )
        .child(div().mt_1().text_xs().text_color(rgb(0x98a3b8)).child(meta))
}

pub(in crate::ui::view) fn policy_button(
    id: &'static str,
    label: &'static str,
    selected: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
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
            rgb(0x303848)
        })
        .bg(if selected {
            rgb(0x173823)
        } else {
            rgb(0x151b27)
        })
        .text_color(if selected {
            rgb(0xbbf7d0)
        } else {
            rgb(0xdbeafe)
        })
        .text_xs()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x223047)))
        .child(label)
        .on_click(on_click)
}

pub(in crate::ui::view) fn connection_row(
    connection: &SavedConnection,
    selected: bool,
    on_select: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_connect: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let can_connect = matches!(
        connection.config,
        ConnectionType::Ssh { .. }
            | ConnectionType::LocalTerminal { .. }
            | ConnectionType::Telnet { .. }
            | ConnectionType::Serial { .. }
    );
    let action = if can_connect {
        small_button(format!("connect-{}", connection.id), "Connect", on_connect).into_any_element()
    } else {
        status_pill("porting", rgb(0xfbbf24), rgb(0x3a2f14)).into_any_element()
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
            rgb(0x2a3140)
        })
        .bg(if selected {
            rgb(0x101b2d)
        } else {
            rgb(0x151923)
        })
        .p_3()
        .hover(|this| this.bg(rgb(0x1c2230)))
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
                                .text_color(rgb(0x98a3b8))
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
                    rgb(0x93c5fd),
                    rgb(0x17253b),
                ))
                .child(action),
        )
}

pub(in crate::ui::view) fn compact_connection_row(
    connection: &SavedConnection,
    on_connect: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x111722))
        .p_3()
        .hover(|this| this.bg(rgb(0x1a2230)))
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
                                .text_color(rgb(0x98a3b8))
                                .child(truncate_preview(&connection.endpoint(), 36)),
                        ),
                )
                .child(status_pill(
                    connection.kind_label(),
                    rgb(0x93c5fd),
                    rgb(0x17253b),
                )),
        )
        .child(
            div()
                .mt_2()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(div().text_xs().text_color(rgb(0x64748b)).child(
                    connection.description.clone().unwrap_or_else(|| {
                        match connection.group_id.as_deref() {
                            Some(group) if !group.trim().is_empty() => {
                                format!("group {group}")
                            }
                            _ => "ungrouped".to_string(),
                        }
                    }),
                ))
                .child(small_button(
                    format!("left-connect-{}", connection.id),
                    "Connect",
                    on_connect,
                )),
        )
}

pub(in crate::ui::view) fn compact_tunnel_row(
    tunnel: &TunnelConfig,
    is_open: bool,
    is_pending: bool,
    on_open: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let status = if is_pending {
        "pending"
    } else if is_open {
        "open"
    } else {
        "closed"
    };
    let (status_fg, status_bg) = if is_pending {
        (rgb(0xfacc15), rgb(0x3a2f14))
    } else if is_open {
        (rgb(0x6ee7b7), rgb(0x12342a))
    } else {
        (rgb(0x98a3b8), rgb(0x202633))
    };

    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x111722))
        .p_3()
        .hover(|this| this.bg(rgb(0x1a2230)))
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
                .text_color(rgb(0x98a3b8))
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
                    rgb(0x93c5fd),
                    rgb(0x17253b),
                ))
                .child(if is_open {
                    small_button(
                        format!("left-tunnel-close-{}", tunnel.id),
                        "Close",
                        on_close,
                    )
                    .into_any_element()
                } else {
                    small_button(format!("left-tunnel-open-{}", tunnel.id), "Open", on_open)
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
pub(in crate::ui::view) fn toolbar_svg_button(
    id: impl Into<SharedString>,
    icon_path: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id.into())
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(rgb(0x8b949e))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x21262d)).text_color(rgb(0xc9d1d9)))
        .child(
            svg()
                .size(px(16.))
                .flex_none()
                .path(icon_path)
                .text_color(rgb(0x8b949e)),
        )
        .on_click(on_click)
}

/// Faded NyaTerm logo used by empty workspace (Tauri EmptyWorkspaceState).
pub(in crate::ui::view) fn nyaterm_logo_mark(size_px: f32, opacity: f32) -> impl IntoElement {
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
                .text_color(rgb(0x8b949e)),
        )
}


/// Colored connection/OS icon for saved connection rows (Tauri resolveConnectionIcon).
pub(in crate::ui::view) fn connection_type_icon(
    def: super::ConnectionIconDef,
    selected: bool,
    size_px: f32,
) -> gpui::AnyElement {
    let size = px(size_px);
    let color = if selected {
        rgb(0x58a6ff)
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


/// Lightweight markdown renderer for AI transcript (paragraphs, lists, fenced code, quotes).
pub(in crate::ui::view) fn markdown_content_view(content: &str) -> impl IntoElement {
    let blocks = parse_markdown_blocks(content);
    let mut root = div().flex().flex_col().gap_1();
    if blocks.is_empty() {
        return root;
    }
    for (index, block) in blocks.into_iter().enumerate() {
        root = root.child(markdown_block_view(index, block));
    }
    root
}

fn markdown_block_view(index: usize, block: MarkdownBlock) -> gpui::AnyElement {
    match block {
        MarkdownBlock::Paragraph(text) => div()
            .id(SharedString::from(format!("md-p-{index}")))
            .text_size(px(12.))
            .text_color(rgb(0xc9d1d9))
            .line_height(px(18.))
            .child(text)
            .into_any_element(),
        MarkdownBlock::Bullet(text) => div()
            .id(SharedString::from(format!("md-ul-{index}")))
            .flex()
            .items_start()
            .gap_2()
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(0x8b949e))
                    .child("•"),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .text_size(px(12.))
                    .text_color(rgb(0xc9d1d9))
                    .line_height(px(18.))
                    .child(text),
            )
            .into_any_element(),
        MarkdownBlock::Numbered { index: n, text } => div()
            .id(SharedString::from(format!("md-ol-{index}")))
            .flex()
            .items_start()
            .gap_2()
            .child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(0x8b949e))
                    .child(format!("{n}.")),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .text_size(px(12.))
                    .text_color(rgb(0xc9d1d9))
                    .line_height(px(18.))
                    .child(text),
            )
            .into_any_element(),
        MarkdownBlock::Code { language, code } => div()
            .id(SharedString::from(format!("md-code-{index}")))
            .rounded_md()
            .border_1()
            .border_color(rgb(0x30363d))
            .bg(rgb(0x0d1117))
            .overflow_hidden()
            .child(
                div()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(rgb(0x30363d))
                    .text_size(px(10.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(0x6e7681))
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
                    .text_color(rgb(0xc9d1d9))
                    .line_height(px(16.))
                    .child(code),
            )
            .into_any_element(),
        MarkdownBlock::Quote(text) => div()
            .id(SharedString::from(format!("md-q-{index}")))
            .pl_3()
            .border_l_1()
            .border_color(rgb(0x30363d))
            .text_size(px(12.))
            .text_color(rgb(0x8b949e))
            .line_height(px(18.))
            .child(text)
            .into_any_element(),
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
                .text_color(rgb(0xe5edf7))
                .line_height(px(size + 4.))
                .child(text)
                .into_any_element()
        }
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

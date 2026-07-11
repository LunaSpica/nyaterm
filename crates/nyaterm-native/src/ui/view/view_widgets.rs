use gpui::{
    App, ClickEvent, FontWeight, IntoElement, SharedString, Window, WindowControlArea, div,
    prelude::*, px, rgb, svg,
};
use nyaterm_domain::{
    CloudSyncHistoryEntry, ConnectionType, NativeServiceStatus, SavedConnection, TunnelConfig,
    truncate_preview,
};
use nyaterm_session::{DockerContainer, NetworkInfo, RemoteProcess};

use crate::ui::components::{mode_button, small_button, status_pill};
use crate::ui::models::WorkspaceSplitDirection;

use super::{
    compact_id, docker_state_color, docker_state_label, format_rate, tunnel_endpoint,
    tunnel_mode_label, tunnel_name,
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
        .h(px(28.))
        .px_2()
        .flex()
        .items_center()
        .rounded_sm()
        .text_xs()
        .text_color(rgb(0x8b949e))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x1c2128)).text_color(rgb(0x58a6ff)))
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
    title: &'static str,
    meta: &'static str,
) -> impl IntoElement {
    // Tauri PanelHeader: min-h-9, uppercase tracking title + dimmed meta, optional actions slot.
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
                .child(
                    div()
                        .min_w_0()
                        .text_size(px(11.))
                        .text_color(rgb(0x6e7681))
                        .overflow_hidden()
                        .child(meta),
                ),
        )
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

    div()
        .id(SharedString::from(format!("empty-action-{label}")))
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .w_full()
        .cursor_pointer()
        .child(
            div()
                .text_sm()
                .font_weight(FontWeight(600.))
                .text_color(rgb(0x58a6ff))
                .hover(|this| this.text_color(rgb(0x79b8ff)))
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

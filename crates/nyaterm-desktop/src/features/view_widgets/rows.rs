use super::*;

pub(in crate::features) fn cloud_sync_history_row(
    palette: ThemePalette,
    entry: CloudSyncHistoryEntry,
    expanded: bool,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_copy: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let summary = cloud_sync_history_summary(&entry);
    let normalized = entry
        .message
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let is_problem = matches!(entry.status.as_str(), "failed" | "conflict");
    let has_message_details = !normalized.is_empty()
        && (is_problem || normalized != summary.split_whitespace().collect::<Vec<_>>().join(" "));
    let has_expandable = has_message_details
        || entry
            .revision
            .as_ref()
            .is_some_and(|r| !r.trim().is_empty());
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
                                    .font_family(crate::features::gpui_code_font_family())
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
                                            .font_family(crate::features::gpui_code_font_family())
                                            .text_size(px(11.))
                                            .text_color(rgb(palette.text))
                                            .child(revision.unwrap_or_default()),
                                    ),
                            )
                        }),
                ),
        )
}

pub(in crate::features) fn policy_button(
    palette: ThemePalette,
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

pub(in crate::features) fn connection_row(
    palette: ThemePalette,
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
        small_button(
            palette,
            format!("connect-{}", connection.id),
            "Connect",
            on_connect,
        )
        .into_any_element()
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
                    rgb(palette.link),
                    rgb(palette.hover),
                ))
                .child(action),
        )
}

pub(in crate::features) fn compact_connection_row(
    palette: ThemePalette,
    connection: &SavedConnection,
    on_connect: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
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
                    rgb(palette.link),
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
                .child(small_button(
                    palette,
                    format!("left-connect-{}", connection.id),
                    "Connect",
                    on_connect,
                )),
        )
}

pub(in crate::features) fn compact_tunnel_row(
    palette: ThemePalette,
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
                    rgb(palette.link),
                    rgb(palette.hover),
                ))
                .child(if is_open {
                    small_button(
                        palette,
                        format!("left-tunnel-close-{}", tunnel.id),
                        "Close",
                        on_close,
                    )
                    .into_any_element()
                } else {
                    small_button(
                        palette,
                        format!("left-tunnel-open-{}", tunnel.id),
                        "Open",
                        on_open,
                    )
                    .into_any_element()
                }),
        )
}

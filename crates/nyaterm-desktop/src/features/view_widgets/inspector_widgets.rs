use super::*;

pub(in crate::features) fn inspector_card(palette: ThemePalette, title: &'static str) -> gpui::Div {
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

pub(in crate::features) fn inspector_status_line(
    palette: ThemePalette,
    text: String,
) -> impl IntoElement {
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

pub(in crate::features) fn compact_network_rows(
    palette: ThemePalette,
    networks: &[NetworkInfo],
) -> impl IntoElement {
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

pub(in crate::features) fn compact_process_rows(
    palette: ThemePalette,
    processes: &[RemoteProcess],
) -> impl IntoElement {
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

pub(in crate::features) fn compact_docker_container_rows(
    palette: ThemePalette,
    containers: &[DockerContainer],
) -> impl IntoElement {
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
pub(in crate::features) fn empty_workspace_action(
    palette: ThemePalette,
    label: impl Into<SharedString>,
    shortcut: impl Into<String>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    // Tauri EmptyWorkspaceState: primary label left, Kbd chips right with "+" separators.
    let label = label.into();
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
                .text_color(rgb(palette.primary))
                .hover(|this| this.text_color(rgb(palette.text)))
                .child(label),
        )
        .child(keys)
        .on_click(on_click)
}

pub(in crate::features) fn tab_menu_item(
    palette: ThemePalette,
    id: impl Into<String>,
    label: impl Into<String>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    tab_menu_item_enabled(palette, id, label, true, on_click)
}

pub(in crate::features) fn tab_menu_item_enabled(
    palette: ThemePalette,
    id: impl Into<String>,
    label: impl Into<String>,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let id = id.into();
    let label = label.into();
    let icon_path = match id.as_str() {
        "tab-ctx-rename" => Some("icons/session/rename.svg"),
        "tab-ctx-color-reset" => Some("icons/window/close.svg"),
        "tab-ctx-copy-name" | "tab-ctx-copy-ip" | "tab-ctx-copy-ssh" => Some("icons/copy.svg"),
        "tab-ctx-duplicate" => Some("icons/transfer/play.svg"),
        "tab-ctx-duplicate-run" | "tab-ctx-multiplex-run" => Some("icons/commands.svg"),
        "tab-ctx-multiplex" | "tab-ctx-smart-split" => Some("icons/menu/split.svg"),
        "tab-ctx-reconnect" => Some("icons/session/reconnect.svg"),
        "tab-ctx-disconnect" => Some("icons/session/disconnect.svg"),
        "tab-ctx-ai-explain" | "tab-ctx-ai-analyze" => Some("icons/ai.svg"),
        "tab-ctx-split-h" | "tab-ctx-window-below" | "tab-ctx-tile-h" => {
            Some("icons/menu/horizontal.svg")
        }
        "tab-ctx-split-v" | "tab-ctx-window-right" | "tab-ctx-tile-v" | "tab-ctx-close-right" => {
            Some("icons/menu/vertical.svg")
        }
        "tab-ctx-unsplit" | "tab-ctx-window-flat" => Some("icons/menu/fit.svg"),
        "tab-ctx-close" => Some("icons/window/close.svg"),
        "tab-ctx-close-all" => Some("icons/transfer/clear-all.svg"),
        "tab-ctx-close-others" => Some("icons/sessions.svg"),
        "tab-ctx-info" => Some("icons/menu/info.svg"),
        _ => None,
    };
    let text_color = if enabled {
        rgb(palette.text)
    } else {
        rgb(palette.text_dimmed)
    };
    div()
        .id(SharedString::from(id))
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .gap_2()
        .text_size(px(12.))
        .text_color(text_color)
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(|this| this.bg(rgb(palette.hover)))
                .on_click(on_click)
        })
        .when_some(icon_path, |this, icon_path| {
            this.child(
                svg()
                    .size(px(14.))
                    .flex_none()
                    .path(icon_path)
                    .text_color(text_color),
            )
        })
        .child(div().min_w_0().flex_1().child(label))
}

pub(in crate::features) fn tab_menu_separator(palette: ThemePalette) -> impl IntoElement {
    div().h(px(1.)).my_1().mx_2().bg(rgb(palette.border))
}

pub(in crate::features) fn tab_action_button(
    palette: ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    detail: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    tab_action_button_enabled(palette, id, label, detail, true, on_click)
}

pub(in crate::features) fn tab_action_button_enabled(
    palette: ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    detail: &'static str,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
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
        .opacity(if enabled { 1.0 } else { 0.45 })
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(|this| this.bg(rgb(palette.hover)).border_color(rgb(0x3b82f6)))
        })
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight(800.))
                .text_color(if enabled {
                    rgb(palette.text)
                } else {
                    rgb(palette.text_dimmed)
                })
                .child(label),
        )
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(palette.text_muted))
                .child(detail),
        )
        .when(enabled, |this| this.on_click(on_click))
}

pub(in crate::features) fn split_divider(
    palette: ThemePalette,
    direction: WorkspaceSplitDirection,
) -> gpui::AnyElement {
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

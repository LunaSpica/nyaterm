use super::*;

#[derive(Clone)]
pub(super) enum ConnectionListRow {
    Separator,
    GroupHeader(ConnectionSection),
    EmptyGroup,
    Connection {
        connection: SavedConnection,
        indented: bool,
    },
}

impl ConnectionListRow {
    pub(super) fn height_px(&self) -> f32 {
        match self {
            Self::Separator => 10.,
            Self::GroupHeader(_) | Self::EmptyGroup => 28.,
            Self::Connection { .. } => 34.,
        }
    }
}

pub(super) fn flatten_connection_rows(
    sections: &[ConnectionSection],
    expanded_groups: &std::collections::HashSet<String>,
) -> Vec<ConnectionListRow> {
    let has_groups = sections.iter().any(|section| !section.is_root);
    let mut rows = Vec::new();
    for section in sections {
        if section.is_root {
            if has_groups && !section.connections.is_empty() {
                rows.push(ConnectionListRow::Separator);
            }
            for connection in &section.connections {
                rows.push(ConnectionListRow::Connection {
                    connection: connection.clone(),
                    indented: false,
                });
            }
            continue;
        }

        rows.push(ConnectionListRow::GroupHeader(section.clone()));
        let expanded = section
            .group_id
            .as_ref()
            .map(|id| expanded_groups.contains(id))
            .unwrap_or(true);
        if !expanded {
            continue;
        }
        if section.connections.is_empty() {
            rows.push(ConnectionListRow::EmptyGroup);
            continue;
        }
        for connection in &section.connections {
            rows.push(ConnectionListRow::Connection {
                connection: connection.clone(),
                indented: true,
            });
        }
    }
    rows
}

#[derive(Clone)]
pub(in crate::features) struct ConnectionSection {
    pub(super) group_id: Option<String>,
    pub(super) label: String,
    pub(super) is_root: bool,
    pub(super) connections: Vec<SavedConnection>,
}

pub(super) fn connection_sections(
    connections: &[SavedConnection],
    groups: &[Group],
    query: &str,
    sort_mode: ConnectionSortMode,
) -> Vec<ConnectionSection> {
    let mut by_group: HashMap<Option<String>, Vec<SavedConnection>> = HashMap::new();
    for connection in connections {
        if !connection_matches(connection, query) {
            continue;
        }
        by_group
            .entry(connection.group_id.clone())
            .or_default()
            .push(connection.clone());
    }
    for list in by_group.values_mut() {
        sort_connections(list, sort_mode);
    }

    let mut sections = Vec::new();
    let mut ordered_groups = groups.to_vec();
    ordered_groups.sort_by(|left, right| {
        left.sort_order.cmp(&right.sort_order).then_with(|| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        })
    });
    for group in ordered_groups {
        let connections = by_group.remove(&Some(group.id.clone())).unwrap_or_default();
        if !query.is_empty() && connections.is_empty() {
            continue;
        }
        sections.push(ConnectionSection {
            group_id: Some(group.id),
            label: group.name,
            is_root: false,
            connections,
        });
    }
    let root = by_group.remove(&None).unwrap_or_default();
    // Tauri: folders first, then ungrouped connections (no "Ungrouped" header).
    if !root.is_empty() || sections.is_empty() {
        sections.push(ConnectionSection {
            group_id: None,
            label: "Ungrouped".to_string(),
            is_root: true,
            connections: root,
        });
    }
    sections
}

pub(super) fn connection_matches(connection: &SavedConnection, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let haystack = format!(
        "{} {} {} {} {}",
        connection.name,
        connection.endpoint(),
        connection.kind_label(),
        connection.description.clone().unwrap_or_default(),
        connection.id
    )
    .to_ascii_lowercase();
    haystack.contains(query)
}

pub(super) fn sort_connections(connections: &mut [SavedConnection], mode: ConnectionSortMode) {
    match mode {
        ConnectionSortMode::Default => connections.sort_by(|left, right| {
            left.sort_order.cmp(&right.sort_order).then_with(|| {
                left.name
                    .to_ascii_lowercase()
                    .cmp(&right.name.to_ascii_lowercase())
            })
        }),
        ConnectionSortMode::NameAsc => connections.sort_by(|left, right| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        }),
        ConnectionSortMode::NameDesc => connections.sort_by(|left, right| {
            right
                .name
                .to_ascii_lowercase()
                .cmp(&left.name.to_ascii_lowercase())
        }),
        ConnectionSortMode::Recent => connections.sort_by(|left, right| {
            right
                .last_used_at_ms
                .unwrap_or(0)
                .cmp(&left.last_used_at_ms.unwrap_or(0))
                .then_with(|| {
                    left.name
                        .to_ascii_lowercase()
                        .cmp(&right.name.to_ascii_lowercase())
                })
        }),
    }
}

pub(super) fn kind_chip(
    palette: crate::theme::ThemePalette,
    label: &'static str,
    selected: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("connection-kind-{label}")))
        .h(px(24.))
        .px_2()
        .flex()
        .items_center()
        .rounded_sm()
        .text_xs()
        .font_weight(FontWeight(700.))
        .cursor_pointer()
        .text_color(if selected {
            rgb(0xffffff)
        } else {
            rgb(palette.text_muted)
        })
        .bg(if selected {
            rgb(palette.success)
        } else {
            rgb(palette.surface_elevated)
        })
        .hover(|this| this.bg(rgb(palette.border)))
        .child(label)
        .on_click(on_click)
}

pub(super) fn toggle_chip(
    palette: crate::theme::ThemePalette,
    label: &'static str,
    selected: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("connection-toggle-{label}")))
        .h(px(22.))
        .px_2()
        .flex()
        .items_center()
        .rounded_sm()
        .text_size(px(10.))
        .font_weight(FontWeight(700.))
        .cursor_pointer()
        .text_color(if selected {
            rgb(palette.success)
        } else {
            rgb(palette.text_muted)
        })
        .bg(if selected {
            rgb(0x12261a)
        } else {
            rgb(palette.surface_elevated)
        })
        .hover(|this| this.bg(rgb(palette.border)))
        .child(label)
        .on_click(on_click)
}

pub(super) fn editor_field(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    value: String,
    active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    transfer_input(id, label, value, active, palette).on_click(on_click)
}

pub(super) fn icon_action_button(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    // label may be a glyph fallback or an icons/*.svg path.
    let is_svg = label.starts_with("icons/") && label.ends_with(".svg");
    div()
        .id(SharedString::from(id.into()))
        .size(px(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_size(px(11.))
        .text_color(rgb(palette.text_muted))
        .cursor_pointer()
        .hover(|this| {
            this.bg(rgb(palette.surface_elevated))
                .text_color(rgb(palette.text))
        })
        .on_click(on_click)
        .when(is_svg, |this| {
            this.child(svg().size(px(14.)).flex_none().path(label))
        })
        .when(!is_svg, |this| this.child(label))
}

pub(super) fn menu_separator(palette: crate::theme::ThemePalette) -> impl IntoElement {
    div().h(px(1.)).mx_2().my_1().bg(rgb(palette.border))
}

pub(super) fn menu_item(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .text_size(px(12.))
        .text_color(rgb(palette.text))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface_elevated)))
        .on_click(on_click)
        .child(label)
}

pub(super) fn menu_item_owned(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: String,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .text_size(px(12.))
        .text_color(rgb(palette.text))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface_elevated)))
        .on_click(on_click)
        .child(label)
}

pub(super) fn connection_detail_rows(
    connection: &SavedConnection,
    all_connections: &[SavedConnection],
    proxies: &[ProxyConfig],
) -> Vec<(&'static str, String)> {
    let description = connection
        .description
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("—")
        .to_string();
    let mut rows = vec![
        ("Type", connection.kind_label().to_string()),
        ("Name", connection.name.clone()),
    ];
    match &connection.config {
        nyaterm_core::ConnectionType::Ssh {
            host,
            port,
            username,
            backspace_mode,
            x11_forwarding,
            ..
        } => {
            rows.push(("Host", host.clone()));
            rows.push(("Port", port.to_string()));
            rows.push(("User", username.clone()));
            rows.push((
                "BS",
                match backspace_mode.as_str() {
                    "ctrl-h" | "bs" | "ctrl_h" => "Ctrl+H".to_string(),
                    _ => "DEL".to_string(),
                },
            ));
            if *x11_forwarding {
                rows.push(("X11", "on".to_string()));
            }
            if let Some(network) = connection.network.as_ref() {
                if let Some(proxy_id) = network.proxy_id.as_deref() {
                    let proxy_label = proxies
                        .iter()
                        .find(|proxy| proxy.id == proxy_id)
                        .map(|proxy| proxy.name.clone())
                        .unwrap_or_else(|| truncate_preview(proxy_id, 16));
                    rows.push(("Proxy", proxy_label));
                }
                if network.proxy_jump_id.is_some() {
                    let chain = format_jump_host_chain(connection, all_connections);
                    rows.push(("Jump", chain));
                }
            }
        }
        nyaterm_core::ConnectionType::LocalTerminal {
            shell_path,
            shell_args,
            working_dir,
            ..
        } => {
            rows.push((
                "Shell",
                if shell_path.trim().is_empty() {
                    "system".to_string()
                } else {
                    shell_path.clone()
                },
            ));
            if !shell_args.trim().is_empty() {
                rows.push(("Args", truncate_preview(shell_args, 28)));
            }
            rows.push((
                "CWD",
                working_dir
                    .clone()
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| "—".to_string()),
            ));
        }
        nyaterm_core::ConnectionType::Telnet {
            host,
            port,
            backspace_mode,
            raw_tcp_cli,
            local_echo,
            ..
        } => {
            rows.push(("Host", host.clone()));
            rows.push(("Port", port.to_string()));
            rows.push((
                "BS",
                match backspace_mode.as_str() {
                    "ctrl-h" | "bs" | "ctrl_h" => "Ctrl+H".to_string(),
                    _ => "DEL".to_string(),
                },
            ));
            if *raw_tcp_cli {
                rows.push(("Mode", "raw tcp".to_string()));
            }
            if *local_echo {
                rows.push(("Echo", "local".to_string()));
            }
        }
        nyaterm_core::ConnectionType::Serial {
            port_name,
            baud_rate,
            data_bits,
            parity,
            stop_bits,
            backspace_mode,
            ..
        } => {
            rows.push(("Port", port_name.clone()));
            rows.push(("Baud", baud_rate.to_string()));
            rows.push(("Data", data_bits.to_string()));
            rows.push(("Parity", parity.clone()));
            rows.push(("Stop", stop_bits.clone()));
            rows.push((
                "BS",
                match backspace_mode.as_str() {
                    "ctrl-h" | "bs" | "ctrl_h" => "Ctrl+H".to_string(),
                    _ => "DEL".to_string(),
                },
            ));
        }
    }
    rows.push(("Last", format_last_used_ms(connection.last_used_at_ms)));
    rows.push(("Desc", description));
    rows
}

pub(super) fn format_jump_host_chain(
    connection: &SavedConnection,
    all_connections: &[SavedConnection],
) -> String {
    let Some(mut jump_id) = connection
        .network
        .as_ref()
        .and_then(|network| network.proxy_jump_id.clone())
    else {
        return "—".to_string();
    };
    let by_id: std::collections::HashMap<&str, &SavedConnection> = all_connections
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect();
    let mut seen = std::collections::HashSet::new();
    seen.insert(connection.id.clone());
    let mut labels = Vec::new();
    loop {
        if !seen.insert(jump_id.clone()) {
            labels.push("↺ cycle".to_string());
            break;
        }
        let Some(jump) = by_id.get(jump_id.as_str()) else {
            labels.push(format!("missing:{jump_id}"));
            break;
        };
        labels.push(jump.name.clone());
        match jump
            .network
            .as_ref()
            .and_then(|network| network.proxy_jump_id.clone())
        {
            Some(next) => jump_id = next,
            None => break,
        }
    }
    if labels.is_empty() {
        "—".to_string()
    } else {
        labels.join(" → ")
    }
}

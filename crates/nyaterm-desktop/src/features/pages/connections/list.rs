use super::*;

#[derive(Clone)]
pub(super) enum ConnectionListRow {
    Separator,
    GroupHeader(ConnectionSection),
    EmptyGroup {
        depth: usize,
    },
    Connection {
        connection: SavedConnection,
        depth: usize,
    },
}

impl ConnectionListRow {
    pub(super) fn height_px(&self) -> f32 {
        match self {
            Self::Separator => 10.,
            Self::GroupHeader(_) | Self::EmptyGroup { .. } => 28.,
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
    let mut children_by_parent: HashMap<Option<String>, Vec<ConnectionSection>> = HashMap::new();
    let mut root_connections = Vec::new();
    for section in sections {
        if section.is_root {
            root_connections.extend(section.connections.clone());
            continue;
        }
        children_by_parent
            .entry(section.parent_id.clone())
            .or_default()
            .push(section.clone());
    }

    for section in children_by_parent.get(&None).cloned().unwrap_or_default() {
        append_connection_section_rows(section, &children_by_parent, expanded_groups, &mut rows);
    }

    if has_groups && !root_connections.is_empty() {
        rows.push(ConnectionListRow::Separator);
    }
    for connection in root_connections {
        rows.push(ConnectionListRow::Connection {
            connection,
            depth: 0,
        });
    }
    rows
}

fn append_connection_section_rows(
    section: ConnectionSection,
    children_by_parent: &HashMap<Option<String>, Vec<ConnectionSection>>,
    expanded_groups: &std::collections::HashSet<String>,
    rows: &mut Vec<ConnectionListRow>,
) {
    let children = children_by_parent
        .get(&section.group_id)
        .cloned()
        .unwrap_or_default();
    rows.push(ConnectionListRow::GroupHeader(section.clone()));
    let expanded = section
        .group_id
        .as_ref()
        .map(|id| expanded_groups.contains(id))
        .unwrap_or(true);
    if !expanded {
        return;
    }

    for child in &children {
        append_connection_section_rows(child.clone(), children_by_parent, expanded_groups, rows);
    }
    if section.connections.is_empty() && children.is_empty() {
        rows.push(ConnectionListRow::EmptyGroup {
            depth: section.depth + 1,
        });
        return;
    }
    for connection in section.connections {
        rows.push(ConnectionListRow::Connection {
            connection,
            depth: section.depth + 1,
        });
    }
}

#[derive(Clone)]
pub(in crate::features) struct ConnectionSection {
    pub(super) group_id: Option<String>,
    pub(super) parent_id: Option<String>,
    pub(super) label: String,
    pub(super) is_root: bool,
    pub(super) depth: usize,
    pub(super) total_count: usize,
    pub(super) has_child_groups: bool,
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
    let group_ids = groups
        .iter()
        .map(|group| group.id.clone())
        .collect::<std::collections::HashSet<_>>();
    let mut children_by_parent: HashMap<Option<String>, Vec<Group>> = HashMap::new();
    for group in groups {
        let parent_id = group
            .parent_id
            .clone()
            .filter(|parent_id| group_ids.contains(parent_id));
        let mut group = group.clone();
        group.parent_id = parent_id.clone();
        children_by_parent.entry(parent_id).or_default().push(group);
    }
    for children in children_by_parent.values_mut() {
        sort_groups(children);
    }

    let mut visited = std::collections::HashSet::new();
    for group in children_by_parent.get(&None).cloned().unwrap_or_default() {
        let (mut group_sections, _) = build_connection_group_sections(
            group,
            0,
            &children_by_parent,
            &mut by_group,
            query,
            &mut visited,
        );
        sections.append(&mut group_sections);
    }

    let mut root = by_group.remove(&None).unwrap_or_default();
    for mut list in by_group.into_values() {
        root.append(&mut list);
    }
    sort_connections(&mut root, sort_mode);
    // Tauri: folders first, then ungrouped connections (no "Ungrouped" header).
    if !root.is_empty() || sections.is_empty() {
        let total_count = root.len();
        sections.push(ConnectionSection {
            group_id: None,
            parent_id: None,
            label: "Ungrouped".to_string(),
            is_root: true,
            depth: 0,
            total_count,
            has_child_groups: false,
            connections: root,
        });
    }
    sections
}

fn build_connection_group_sections(
    group: Group,
    depth: usize,
    children_by_parent: &HashMap<Option<String>, Vec<Group>>,
    by_group: &mut HashMap<Option<String>, Vec<SavedConnection>>,
    query: &str,
    visited: &mut std::collections::HashSet<String>,
) -> (Vec<ConnectionSection>, usize) {
    if !visited.insert(group.id.clone()) {
        return (Vec::new(), 0);
    }

    let direct_connections = by_group.remove(&Some(group.id.clone())).unwrap_or_default();
    let mut child_sections = Vec::new();
    let mut total_count = direct_connections.len();
    for child in children_by_parent
        .get(&Some(group.id.clone()))
        .cloned()
        .unwrap_or_default()
    {
        let (mut sections, child_count) = build_connection_group_sections(
            child,
            depth + 1,
            children_by_parent,
            by_group,
            query,
            visited,
        );
        total_count += child_count;
        child_sections.append(&mut sections);
    }

    if !query.is_empty() && total_count == 0 {
        return (Vec::new(), 0);
    }

    let has_child_groups = !child_sections.is_empty();
    let mut sections = vec![ConnectionSection {
        group_id: Some(group.id),
        parent_id: group.parent_id,
        label: group.name,
        is_root: false,
        depth,
        total_count,
        has_child_groups,
        connections: direct_connections,
    }];
    sections.append(&mut child_sections);
    (sections, total_count)
}

fn sort_groups(groups: &mut [Group]) {
    groups.sort_by(|left, right| {
        left.sort_order.cmp(&right.sort_order).then_with(|| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
        })
    });
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
    }
}

pub(super) fn connection_tree_indent_px(depth: usize) -> f32 {
    if depth == 0 {
        8.
    } else {
        8. + depth as f32 * 16. + 16.
    }
}

pub(super) fn connection_kind_tab(
    palette: crate::theme::ThemePalette,
    label: impl Into<SharedString>,
    selected: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let label = label.into();
    div()
        .id(SharedString::from(format!("connection-kind-tab-{label}")))
        .h(px(28.))
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(if selected {
            rgba((palette.primary << 8) | 0x66)
        } else {
            rgba(0x00000000)
        })
        .text_xs()
        .font_weight(FontWeight(600.))
        .cursor_pointer()
        .text_color(if selected {
            rgb(palette.primary)
        } else {
            rgb(palette.text_muted)
        })
        .bg(if selected {
            rgba((palette.primary << 8) | 0x18)
        } else {
            rgba(0x00000000)
        })
        .hover(|this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
        .child(label)
        .on_click(on_click)
}

#[derive(Clone)]
pub(super) struct ConnectionEditorChoice {
    pub value: Option<String>,
    pub label: String,
    pub selected: bool,
}

pub(super) fn connection_editor_select(
    palette: crate::theme::ThemePalette,
    id: &'static str,
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
    menu: ConnectionEditorMenu,
    open: bool,
    options: Vec<ConnectionEditorChoice>,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let label = label.into();
    let show_label = !label.is_empty();
    let value = value.into();
    let mut option_list = div()
        .id(SharedString::from(format!("{id}-options")))
        .max_h(px(144.))
        .flex()
        .flex_col()
        .gap_1()
        .overflow_scroll();
    for (index, option) in options.into_iter().enumerate() {
        let option_value = option.value.clone();
        option_list = option_list.child(
            div()
                .id(SharedString::from(format!("{id}-option-{index}")))
                .min_h(px(26.))
                .px_2()
                .flex()
                .items_center()
                .rounded_sm()
                .text_xs()
                .cursor_pointer()
                .text_color(if option.selected {
                    rgb(palette.primary)
                } else {
                    rgb(palette.text)
                })
                .bg(if option.selected {
                    rgba((palette.primary << 8) | 0x18)
                } else {
                    rgba(0x00000000)
                })
                .hover(|this| this.bg(rgb(palette.hover)))
                .child(option.label)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.set_connection_editor_menu_value(menu, option_value.as_deref(), cx);
                })),
        );
    }

    div()
        .id(SharedString::from(id))
        .relative()
        .min_w_0()
        .flex_1()
        .flex()
        .flex_col()
        .gap_1()
        .when(show_label, |this| {
            this.child(
                div()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .child(label),
            )
        })
        .child(
            div()
                .id(SharedString::from(format!("{id}-trigger")))
                .h(px(30.))
                .min_w_0()
                .px_2()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .rounded_sm()
                .border_1()
                .border_color(if open {
                    rgb(palette.primary)
                } else {
                    rgb(palette.border)
                })
                .bg(rgb(palette.input))
                .text_xs()
                .text_color(rgb(palette.text))
                .cursor_pointer()
                .hover(|this| this.bg(rgb(palette.hover)))
                .child(div().min_w_0().overflow_hidden().child(value))
                .child(
                    div()
                        .flex_none()
                        .text_size(px(10.))
                        .text_color(rgb(palette.text_dimmed))
                        .child(if open { "▴" } else { "▾" }),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.toggle_connection_editor_menu(menu, cx);
                })),
        )
        .when(open, |this| {
            this.child(
                div()
                    .mt_1()
                    .p_1()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface_elevated))
                    .shadow_lg()
                    .child(option_list),
            )
        })
}

pub(super) fn toggle_chip(
    palette: crate::theme::ThemePalette,
    label: &'static str,
    selected: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("connection-toggle-{label}")))
        .h(px(28.))
        .px_2()
        .flex()
        .items_center()
        .gap_2()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .text_size(px(10.))
        .font_weight(FontWeight(500.))
        .cursor_pointer()
        .text_color(if selected {
            rgb(palette.text)
        } else {
            rgb(palette.text_muted)
        })
        .bg(rgb(palette.input))
        .hover(|this| this.bg(rgb(palette.hover)))
        .child(div().min_w_0().child(label))
        .child(
            div()
                .w(px(28.))
                .h(px(16.))
                .flex()
                .items_center()
                .justify_start()
                .when(selected, |this| this.justify_end())
                .px(px(2.))
                .rounded_full()
                .bg(if selected {
                    rgb(palette.primary)
                } else {
                    rgb(palette.border)
                })
                .child(div().size(px(12.)).rounded_full().bg(if selected {
                    rgb(palette.on_primary)
                } else {
                    rgb(palette.text_dimmed)
                })),
        )
        .on_click(on_click)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_groups_render_children_before_connections_and_root_last() {
        let groups = vec![
            group("parent", "Parent", None, 0),
            group("child", "Child", Some("parent"), 0),
        ];
        let connections = vec![
            connection("parent-conn", "Parent Conn", Some("parent"), 0),
            connection("child-conn", "Child Conn", Some("child"), 0),
            connection("root-conn", "Root Conn", None, 0),
        ];
        let sections = connection_sections(
            &connections,
            &groups,
            "",
            crate::models::ConnectionSortMode::Default,
        );
        let expanded = ["parent".to_string(), "child".to_string()]
            .into_iter()
            .collect();
        let rows = flatten_connection_rows(&sections, &expanded);

        let labels = rows
            .iter()
            .map(|row| match row {
                ConnectionListRow::GroupHeader(section) => {
                    format!("group:{}:{}", section.label, section.depth)
                }
                ConnectionListRow::Connection { connection, depth } => {
                    format!("conn:{}:{depth}", connection.id)
                }
                ConnectionListRow::Separator => "separator".to_string(),
                ConnectionListRow::EmptyGroup { depth } => format!("empty:{depth}"),
            })
            .collect::<Vec<_>>();

        assert_eq!(
            labels,
            vec![
                "group:Parent:0",
                "group:Child:1",
                "conn:child-conn:2",
                "conn:parent-conn:1",
                "separator",
                "conn:root-conn:0",
            ]
        );
    }

    fn group(id: &str, name: &str, parent_id: Option<&str>, sort_order: i32) -> Group {
        Group {
            id: id.to_string(),
            name: name.to_string(),
            parent_id: parent_id.map(ToOwned::to_owned),
            sort_order,
            created_at_ms: None,
            updated_at_ms: None,
        }
    }

    fn connection(
        id: &str,
        name: &str,
        group_id: Option<&str>,
        sort_order: i32,
    ) -> SavedConnection {
        SavedConnection {
            id: id.to_string(),
            name: name.to_string(),
            config: nyaterm_core::ConnectionType::LocalTerminal {
                shell_path: String::new(),
                shell_args: String::new(),
                working_dir: None,
                ai_execution_profile: nyaterm_core::AiExecutionProfile::Auto,
            },
            group_id: group_id.map(ToOwned::to_owned),
            description: None,
            sort_order,
            icon: None,
            auth: None,
            network: None,
            post_login: None,
            created_at_ms: None,
            updated_at_ms: None,
            last_used_at_ms: None,
        }
    }
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

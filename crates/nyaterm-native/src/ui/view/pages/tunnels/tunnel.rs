use super::*;
use super::common::{network_dialog_footer, network_modal_shell};

#[derive(Debug, Clone)]
pub(super) struct TunnelSection {
    id: String,
    label: String,
    group: Option<TunnelGroup>,
    tunnels: Vec<TunnelConfig>,
}

pub(super) fn tunnel_sections(
    tunnels: &[TunnelConfig],
    groups: &[TunnelGroup],
) -> Vec<TunnelSection> {
    let valid_group_ids = groups
        .iter()
        .map(|group| group.id.as_str())
        .collect::<HashSet<_>>();
    let mut by_group = HashMap::<String, Vec<TunnelConfig>>::new();
    let mut ungrouped = Vec::<TunnelConfig>::new();

    for tunnel in tunnels {
        match tunnel.group_id.as_deref() {
            Some(group_id) if valid_group_ids.contains(group_id) => {
                by_group
                    .entry(group_id.to_string())
                    .or_default()
                    .push(tunnel.clone());
            }
            _ => ungrouped.push(tunnel.clone()),
        }
    }

    let mut sections = groups
        .iter()
        .cloned()
        .map(|group| TunnelSection {
            id: group.id.clone(),
            label: group.name.clone(),
            tunnels: by_group.remove(&group.id).unwrap_or_default(),
            group: Some(group),
        })
        .collect::<Vec<_>>();

    if !ungrouped.is_empty() || sections.is_empty() {
        sections.push(TunnelSection {
            id: "__ungrouped__".to_string(),
            label: "Ungrouped".to_string(),
            group: None,
            tunnels: ungrouped,
        });
    }

    sections
}

pub(super) fn tunnel_section(
    section: TunnelSection,
    open_tunnels: &HashMap<String, SshTunnelInfo>,
    app: &NyaTermApp,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
        let palette = cx.entity().read(cx).theme_palette();
    let item_count = section.tunnels.len();
    let open_count = section
        .tunnels
        .iter()
        .filter(|tunnel| open_tunnels.contains_key(&tunnel.id))
        .count();
    let section_key = format!("tunnel:{}", section.id);
    let collapsed = !app.network_expanded_sections.contains(&section_key);
    let section_id_for_toggle = section.id.clone();
    let mut rows = div().flex().flex_col();
    if section.tunnels.is_empty() {
        rows = rows.child(
            div()
                .border_t_1()
                .border_color(rgb(palette.border))
                .px_2()
                .py_2()
                .text_size(px(11.))
                .text_color(rgb(palette.text_muted))
                .child("No tunnels in this group."),
        );
    } else {
        for tunnel in section.tunnels {
            let open_info = open_tunnels.get(&tunnel.id).cloned();
            let pending = app.pending_tunnels.iter().any(|id| id == &tunnel.id);
            let connection_label = tunnel
                .connection_id
                .as_deref()
                .and_then(|id| {
                    app.connections
                        .iter()
                        .find(|connection| connection.id == id)
                        .map(|connection| connection.name.clone())
                })
                .unwrap_or_else(|| "Missing connection".to_string());
            let tunnel_for_open = tunnel.clone();
            let tunnel_id_for_close = tunnel.id.clone();
            let tunnel_id_for_edit = tunnel.id.clone();
            let tunnel_id_for_move = tunnel.id.clone();
            let tunnel_id_for_delete = tunnel.id.clone();
            let tunnel_label_for_delete = tunnel_name(&tunnel);
            let move_picker_open = app
                .network_move_picker
                .as_ref()
                .is_some_and(|picker| picker.tab == NetworkTab::Tunnels && picker.id == tunnel.id);
            let current_group_id = tunnel.group_id.clone();
            rows = rows.child(
                div()
                    .flex()
                    .flex_col()
                    .child(tunnel_network_row(
                        &tunnel,
                        connection_label,
                        open_info,
                        pending,
                        app.tunnel_groups.len(),
                        cx.listener(move |this, _, window, cx| {
                            this.start_tunnel_job(tunnel_for_open.clone(), window, cx);
                        }),
                        cx.listener(move |this, _, _, cx| {
                            this.close_tunnel_job(tunnel_id_for_close.clone(), cx);
                        }),
                        cx.listener(move |this, _, window, cx| {
                            this.open_network_tunnel_editor(
                                Some(tunnel_id_for_edit.clone()),
                                window,
                                cx,
                            );
                        }),
                        cx.listener(move |this, _, _, cx| {
                            this.open_network_move_picker(
                                NetworkTab::Tunnels,
                                tunnel_id_for_move.clone(),
                                cx,
                            );
                        }),
                        cx.listener(move |this, _, _, cx| {
                            this.open_network_delete_confirm(
                                NetworkTab::Tunnels,
                                tunnel_id_for_delete.clone(),
                                tunnel_label_for_delete.clone(),
                                cx,
                            );
                        }),
                    ))
                    .when(move_picker_open, |this| {
                        this.child(tunnel_move_picker(
                            tunnel.id.clone(),
                            current_group_id,
                            &app.tunnel_groups,
                            cx,
                        ))
                    }),
            );
        }
    }

    div()
        .id(gpui::SharedString::from(format!(
            "tunnel-section-{}",
            section.id
        )))
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.surface))
        .overflow_hidden()
        .child(
            div()
                .id(gpui::SharedString::from(format!("tunnel-section-header-{}", section.id)))
                .h(px(30.))
                .px_3()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .bg(rgb(palette.input))
                .cursor_pointer()
                .hover(|this| this.bg(rgb(palette.hover)))
                .on_click({
                    let section_id_for_toggle = section_id_for_toggle.clone();
                    cx.listener(move |this, _, _, cx| {
                        this.toggle_network_section(
                            NetworkTab::Tunnels,
                            section_id_for_toggle.clone(),
                            cx,
                        );
                    })
                })
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(palette.text_muted))
                                .child(if collapsed { "▸" } else { "▾" }),
                        )
                        .child(
                            div()
                                .text_xs()
                                .font_weight(FontWeight(600.))
                                .text_color(rgb(palette.text))
                                .child(truncate_preview(&section.label, 48)),
                        )
                        .child(
                            div()
                                .rounded_full()
                                .px_1()
                                .text_size(px(10.))
                                .text_color(rgb(palette.text_muted))
                                .bg(rgb(palette.surface_elevated))
                                .child(item_count.to_string()),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .when(open_count > 0, |this| {
                            this.child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(palette.success))
                                    .child(format!("{open_count} open")),
                            )
                        }),
                )
                .when_some(section.group.clone(), |this, group| {
                    let rename_id = group.id.clone();
                    let delete_id = group.id.clone();
                    let delete_label = group.name.clone();
                    this.child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(crate::ui::theme::theme_palette("github-dark"), 
                                format!("tunnel-group-rename-{}", group.id),
                                "Rename",
                                cx.listener(move |this, _, _, cx| {
                                    this.open_network_group_editor(
                                        NetworkTab::Tunnels,
                                        Some(rename_id.clone()),
                                        cx,
                                    );
                                }),
                            ))
                            .child(small_button(crate::ui::theme::theme_palette("github-dark"), 
                                format!("tunnel-group-delete-{}", group.id),
                                "Delete",
                                cx.listener(move |this, _, _, cx| {
                                    this.open_network_group_delete_confirm(
                                        NetworkTab::Tunnels,
                                        delete_id.clone(),
                                        delete_label.clone(),
                                        item_count,
                                        cx,
                                    );
                                }),
                            )),
                    )
                }),
        )
        .when(!collapsed, |this| this.child(rows))
}

fn tunnel_move_picker(
    tunnel_id: String,
    current_group_id: Option<String>,
    groups: &[TunnelGroup],
    cx: &mut Context<NyaTermApp>,
) -> gpui::Div {
        let palette = cx.entity().read(cx).theme_palette();
    let mut targets = div().flex().flex_wrap().items_center().gap_2();
    if current_group_id.is_none() {
        targets = targets.child(status_pill(
            "Ungrouped · current",
            rgb(palette.accent),
            rgb(palette.hover),
        ));
    } else {
        let target_id = tunnel_id.clone();
        targets = targets.child(small_button(crate::ui::theme::theme_palette("github-dark"), 
            format!("network-tunnel-move-{tunnel_id}-ungrouped"),
            "Ungrouped",
            cx.listener(move |this, _, _, cx| {
                this.move_tunnel_to_group(target_id.clone(), None, cx);
            }),
        ));
    }

    for group in groups {
        if current_group_id.as_deref() == Some(group.id.as_str()) {
            targets = targets.child(status_pill("current", rgb(palette.success), rgb(palette.hover)));
            targets = targets.child(
                div()
                    .text_xs()
                    .text_color(rgb(palette.text))
                    .child(truncate_preview(&group.name, 36)),
            );
        } else {
            let target_id = tunnel_id.clone();
            let group_id = group.id.clone();
            targets = targets.child(small_button(crate::ui::theme::theme_palette("github-dark"), 
                format!("network-tunnel-move-{tunnel_id}-{}", group.id),
                "Move Here",
                cx.listener(move |this, _, _, cx| {
                    this.move_tunnel_to_group(target_id.clone(), Some(group_id.clone()), cx);
                }),
            ));
            targets = targets.child(
                div()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .child(truncate_preview(&group.name, 36)),
            );
        }
    }

    div()
        .border_t_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .px_3()
        .py_2()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .flex_none()
                .text_xs()
                .font_weight(FontWeight(700.))
                .text_color(rgb(palette.text))
                .child("Move to"),
        )
        .child(targets)
}

pub(super) fn tunnel_network_row(
    tunnel: &TunnelConfig,
    connection_label: String,
    open_info: Option<SshTunnelInfo>,
    pending: bool,
    group_count: usize,
    on_open: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_edit: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_move: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_delete: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let palette = crate::ui::theme::theme_palette("github-dark");
    let supported = tunnel_mode(tunnel).is_some();
    let is_open = open_info.is_some();
    let status = if pending {
        "pending"
    } else if is_open {
        "open"
    } else if supported {
        "closed"
    } else {
        "porting"
    };
    let (status_color, status_bg) = tunnel_status_style(pending, is_open, supported);
    let bind = if tunnel.bind_localhost {
        "127.0.0.1"
    } else {
        "0.0.0.0"
    };
    let listen = open_info
        .as_ref()
        .map(|info| format!("{}:{}", info.bind_host, info.listen_port))
        .unwrap_or_else(|| format!("{bind}:{}", tunnel.listen_port));
    // Tauri TunnelRow: 3-line left stack, StatusBadge, Switch, overflow actions.
    let toggle = if pending {
        status_pill("…", rgb(palette.warning), rgb(palette.hover)).into_any_element()
    } else if is_open {
        network_switch_button(
            format!("network-tunnel-close-{}", tunnel.id),
            true,
            on_close,
        )
        .into_any_element()
    } else if supported {
        network_switch_button(
            format!("network-tunnel-open-{}", tunnel.id),
            false,
            on_open,
        )
        .into_any_element()
    } else {
        status_pill("porting", rgb(palette.warning), rgb(palette.hover)).into_any_element()
    };

    // Tauri: px-3 py-2.5; side-panel density uses slightly tighter mono stack.
    div()
        .border_t_1()
        .border_color(rgb(palette.surface_elevated))
        .px_2()
        .py_2()
        .flex()
        .items_center()
        .gap_2()
        .hover(|this| this.bg(rgb(palette.hover)))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap_0()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .min_w_0()
                                .text_size(px(12.))
                                .font_weight(FontWeight(600.))
                                .text_color(rgb(palette.text))
                                .overflow_hidden()
                                .child(truncate_preview(&tunnel_name(tunnel), 52)),
                        )
                        .child(status_pill(status, status_color, status_bg))
                        .when(tunnel.auto_open, |this| {
                            this.child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(palette.success))
                                    .child("auto"),
                            )
                        }),
                )
                .child(
                    div()
                        .mt(px(1.))
                        .text_size(px(11.))
                        .text_color(rgb(palette.text_muted))
                        .overflow_hidden()
                        .child(format!(
                            "{} · {}",
                            truncate_preview(&connection_label, 44),
                            tunnel_mode_label(tunnel)
                        )),
                )
                .child(
                    div()
                        .mt(px(1.))
                        .font_family("JetBrains Mono")
                        .text_size(px(10.))
                        .text_color(rgb(palette.text_dimmed))
                        .overflow_hidden()
                        .child(truncate_preview(&tunnel_endpoint(tunnel, &listen), 88)),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(toggle)
                .child(network_icon_action(
                    format!("network-tunnel-edit-{}", tunnel.id),
                    "icons/net/edit.svg",
                    on_edit,
                ))
                .when(group_count > 0, |this| {
                    this.child(network_icon_action(
                        format!("network-tunnel-move-{}", tunnel.id),
                        "icons/net/move.svg",
                        on_move,
                    ))
                })
                .child(network_icon_action(
                    format!("network-tunnel-delete-{}", tunnel.id),
                    "icons/net/delete.svg",
                    on_delete,
                )),
        )
}

fn network_switch_button(
    id: impl Into<String>,
    on: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let palette = crate::ui::theme::theme_palette("github-dark");
    // Compact switch stand-in for Tauri Switch next to tunnel rows.
    div()
        .id(gpui::SharedString::from(id.into()))
        .w(px(34.))
        .h(px(18.))
        .rounded_full()
        .border_1()
        .border_color(if on { rgb(palette.success) } else { rgb(palette.border) })
        .bg(if on { rgb(palette.success) } else { rgb(palette.surface_elevated) })
        .flex()
        .items_center()
        .px(px(2.))
        .cursor_pointer()
        .hover(|this| this.opacity(0.9))
        .child(
            div()
                .size(px(12.))
                .rounded_full()
                .bg(rgb(0xffffff))
                .when(on, |this| this.ml_auto())
                .when(!on, |this| this.mr_auto()),
        )
        .on_click(on_click)
}

fn network_icon_action(
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let palette = crate::ui::theme::theme_palette("github-dark");
    div()
        .id(gpui::SharedString::from(id.into()))
        .size(px(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_size(px(12.))
        .text_color(rgb(palette.text_muted))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface_elevated)).text_color(rgb(palette.text)))
        .child(
            svg()
                .size(px(14.))
                .flex_none()
                .path(label),
        )
        .on_click(on_click)
}

pub(super) fn network_tunnel_editor_panel(
    editor: NetworkTunnelEditorState,
    app: &NyaTermApp,
    focus: &gpui::FocusHandle,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
        let palette = cx.entity().read(cx).theme_palette();
    let connection_label = editor
        .connection_id
        .as_deref()
        .and_then(|id| {
            app.connections
                .iter()
                .find(|connection| connection.id == id)
                .map(|connection| connection.name.clone())
        })
        .unwrap_or_else(|| "Select SSH connection".to_string());
    let group_label = editor
        .group_id
        .as_deref()
        .and_then(|id| {
            app.tunnel_groups
                .iter()
                .find(|group| group.id == id)
                .map(|group| group.name.clone())
        })
        .unwrap_or_else(|| "Ungrouped".to_string());
    let mode_label = match editor.tunnel_type.as_str() {
        "remote" => "Remote",
        "dynamic" => "Dynamic",
        _ => "Local",
    };
    let preview = tunnel_editor_preview(&editor);

    let card = div()
        .p_4()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(15.))
                                .font_weight(FontWeight(700.))
                                .text_color(rgb(palette.text))
                                .child(if editor.id.is_some() {
                                    "Edit Tunnel"
                                } else {
                                    "New Tunnel"
                                }),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(palette.text_muted))
                                .child("Configure SSH local, remote, or dynamic port forwarding."),
                        ),
                )
                .child(status_pill(mode_label, rgb(palette.accent), rgb(palette.hover))),
        )
        .child(
            div()
                .grid()
                .grid_cols(3)
                .gap_2()
                .child(tunnel_editor_input(
                    "network-tunnel-editor-name",
                    "Tunnel name",
                    editor.name.clone(),
                    editor.focused_field == NetworkTunnelEditorField::Name,
                    NetworkTunnelEditorField::Name,
                    focus,
                    cx,
                ))
                .child(tunnel_editor_selector(
                    "network-tunnel-editor-type",
                    "Type",
                    mode_label.to_string(),
                    cx.listener(|this, _, _, cx| {
                        this.cycle_network_tunnel_type(cx);
                    }),
                ))
                .child(tunnel_editor_selector(
                    "network-tunnel-editor-group",
                    "Group",
                    group_label,
                    cx.listener(|this, _, _, cx| {
                        this.cycle_network_tunnel_group(cx);
                    }),
                )),
        )
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap_2()
                .child(tunnel_editor_selector(
                    "network-tunnel-editor-connection",
                    "SSH connection",
                    connection_label,
                    cx.listener(|this, _, _, cx| {
                        this.cycle_network_tunnel_connection(cx);
                    }),
                ))
                .child(tunnel_editor_input(
                    "network-tunnel-editor-listen-port",
                    match editor.tunnel_type.as_str() {
                        "remote" => "Remote listen port",
                        "dynamic" => "SOCKS listen port",
                        _ => "Local listen port",
                    },
                    editor.listen_port.clone(),
                    editor.focused_field == NetworkTunnelEditorField::ListenPort,
                    NetworkTunnelEditorField::ListenPort,
                    focus,
                    cx,
                )),
        )
        .when(!editor.is_dynamic(), |this| {
            this.child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_2()
                    .child(tunnel_editor_input(
                        "network-tunnel-editor-target-host",
                        match editor.tunnel_type.as_str() {
                            "remote" => "Remote target host",
                            _ => "Local target host",
                        },
                        editor.target_host.clone(),
                        editor.focused_field == NetworkTunnelEditorField::TargetHost,
                        NetworkTunnelEditorField::TargetHost,
                        focus,
                        cx,
                    ))
                    .child(tunnel_editor_input(
                        "network-tunnel-editor-target-port",
                        match editor.tunnel_type.as_str() {
                            "remote" => "Remote target port",
                            _ => "Local target port",
                        },
                        editor.target_port.clone(),
                        editor.focused_field == NetworkTunnelEditorField::TargetPort,
                        NetworkTunnelEditorField::TargetPort,
                        focus,
                        cx,
                    )),
            )
        })
        .child(
            div()
                .grid()
                .grid_cols(3)
                .gap_2()
                .child(tunnel_editor_option(
                    "network-tunnel-editor-bind-local",
                    "Localhost only",
                    "127.0.0.1",
                    editor.bind_localhost,
                    cx.listener(|this, _, _, cx| {
                        this.set_network_tunnel_bind_localhost(true, cx);
                    }),
                ))
                .child(tunnel_editor_option(
                    "network-tunnel-editor-bind-all",
                    "All interfaces",
                    "0.0.0.0",
                    !editor.bind_localhost,
                    cx.listener(|this, _, _, cx| {
                        this.set_network_tunnel_bind_localhost(false, cx);
                    }),
                ))
                .child(tunnel_editor_option(
                    "network-tunnel-editor-auto",
                    "Auto open",
                    "with connection",
                    editor.auto_open,
                    cx.listener(|this, _, _, cx| {
                        this.toggle_network_tunnel_auto_open(cx);
                    }),
                )),
        )
        .child(
            div()
                .rounded_sm()
                .border_1()
                .border_color(rgb(palette.border))
                .bg(rgb(palette.input))
                .p_3()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_xs().text_color(rgb(palette.text_muted)).child("Preview"))
                .child(
                    div()
                        .font_family("JetBrains Mono")
                        .text_xs()
                        .text_color(rgb(palette.text))
                        .child(preview),
                ),
        )
        .when_some(editor.error.clone(), |this, error| {
            this.child(div().text_xs().text_color(rgb(palette.danger)).child(error))
        })
        .child(network_dialog_footer(
            "network-tunnel-editor-cancel",
            "network-tunnel-editor-save",
            "Save",
            cx.listener(|this, _, _, cx| {
                this.close_network_tunnel_editor(cx);
            }),
            cx.listener(|this, _, _, cx| {
                this.save_network_tunnel_editor(cx);
            }),
        ));

    network_modal_shell("network-tunnel-editor-modal", 640., card)
}

pub(super) fn tunnel_editor_input(
    id: impl Into<String>,
    label: &'static str,
    value: String,
    active: bool,
    field: NetworkTunnelEditorField,
    focus: &gpui::FocusHandle,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    transfer_input(id, label, value, active, crate::ui::theme::theme_palette(&cx.entity().read(cx).settings.theme))
        .track_focus(focus)
        .on_click(cx.listener(move |this, _, window, cx| {
            this.focus_network_tunnel_editor_field(field, window, cx);
        }))
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
            cx.stop_propagation();
            this.handle_network_tunnel_editor_key_down(event, cx);
        }))
}

pub(super) fn tunnel_editor_selector(
    id: impl Into<String>,
    label: &'static str,
    value: String,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let palette = crate::ui::theme::theme_palette("github-dark");
    div()
        .id(gpui::SharedString::from(id.into()))
        .h(px(52.))
        .px_3()
        .py_2()
        .flex()
        .flex_col()
        .gap_1()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.bg))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface)))
        .child(div().text_size(px(11.)).text_color(rgb(palette.text_muted)).child(label))
        .child(
            div()
                .font_family("JetBrains Mono")
                .text_size(px(12.))
                .text_color(rgb(palette.text))
                .child(truncate_preview(&value, 42)),
        )
        .on_click(on_click)
}

pub(super) fn tunnel_editor_option(
    id: impl Into<String>,
    title: &'static str,
    detail: &'static str,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let palette = crate::ui::theme::theme_palette("github-dark");
    // Tauri-like selectable option cards for bind host / auto open.
    div()
        .id(gpui::SharedString::from(id.into()))
        .rounded_md()
        .border_1()
        .border_color(if active { rgb(palette.accent) } else { rgb(palette.border) })
        .bg(if active { rgb(palette.hover) } else { rgb(palette.bg) })
        .px_3()
        .py_2()
        .flex()
        .flex_col()
        .gap_1()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface)))
        .child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight(600.))
                .text_color(if active { rgb(palette.accent) } else { rgb(palette.text) })
                .child(title),
        )
        .child(div().text_size(px(11.)).text_color(rgb(palette.text_muted)).child(detail))
        .on_click(on_click)
}

pub(super) fn tunnel_editor_preview(editor: &NetworkTunnelEditorState) -> String {
    let bind_host = if editor.bind_localhost {
        "127.0.0.1"
    } else {
        "0.0.0.0"
    };
    let listen_port = editor.listen_port.trim();
    let listen_port = if listen_port.is_empty() {
        "?"
    } else {
        listen_port
    };
    if editor.is_dynamic() {
        return format!("SOCKS {bind_host}:{listen_port}");
    }

    let target_host = editor.target_host.trim();
    let target_host = if target_host.is_empty() {
        "?"
    } else {
        target_host
    };
    let target_port = editor.target_port.trim();
    let target_port = if target_port.is_empty() {
        "?"
    } else {
        target_port
    };
    if editor.tunnel_type == "remote" {
        format!("remote {bind_host}:{listen_port} -> {target_host}:{target_port}")
    } else {
        format!("local {bind_host}:{listen_port} -> {target_host}:{target_port}")
    }
}

pub(super) fn tunnel_status_style(pending: bool, is_open: bool, supported: bool) -> (Hsla, Hsla) {
    let palette = crate::ui::theme::theme_palette("github-dark");
    if pending {
        (rgb(palette.warning).into(), rgb(palette.hover).into())
    } else if is_open {
        (rgb(palette.success).into(), rgb(palette.hover).into())
    } else if supported {
        (rgb(palette.accent).into(), rgb(palette.hover).into())
    } else {
        (rgb(palette.warning).into(), rgb(palette.hover).into())
    }
}

pub(super) fn tunnel_matches(tunnel: &TunnelConfig, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    format!(
        "{} {} {} {} {} {} {}",
        tunnel.id,
        tunnel.name,
        tunnel.tunnel_type,
        tunnel.connection_id.as_deref().unwrap_or_default(),
        tunnel.listen_port,
        tunnel.target_host,
        tunnel.target_port
    )
    .to_ascii_lowercase()
    .contains(query)
}

use super::*;

pub(in crate::features::pages::tunnels) fn tunnel_network_row(
    palette: crate::theme::ThemePalette,
    tunnel: &TunnelConfig,
    connection_label: String,
    open_info: Option<SshTunnelInfo>,
    pending: bool,
    group_count: usize,
    menu_open: bool,
    more_label: &'static str,
    edit_label: &'static str,
    move_label: &'static str,
    delete_label: &'static str,
    open_status_label: &'static str,
    closed_status_label: &'static str,
    mode_label: &'static str,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_open: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_edit: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_move: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_delete: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let supported = tunnel_mode(tunnel).is_some();
    let is_open = open_info.is_some();
    let status = if pending {
        "pending"
    } else if is_open {
        open_status_label
    } else if supported {
        closed_status_label
    } else {
        "porting"
    };
    let (status_color, status_bg) = tunnel_status_style(palette, pending, is_open, supported);
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
            palette,
            format!("network-tunnel-close-{}", tunnel.id),
            true,
            on_close,
        )
        .into_any_element()
    } else if supported {
        network_switch_button(
            palette,
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
                            mode_label
                        )),
                )
                .child(
                    div()
                        .mt(px(1.))
                        .font_family(crate::features::gpui_code_font_family())
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
                .child(network_item_overflow_menu(
                    palette,
                    format!("network-tunnel-actions-{}", tunnel.id),
                    menu_open,
                    more_label,
                    edit_label,
                    move_label,
                    delete_label,
                    group_count > 0,
                    on_toggle,
                    on_edit,
                    on_move,
                    on_delete,
                )),
        )
}

fn network_switch_button(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    on: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    // Compact switch stand-in for Tauri Switch next to tunnel rows.
    div()
        .id(gpui::SharedString::from(id.into()))
        .w(px(34.))
        .h(px(18.))
        .rounded_full()
        .border_1()
        .border_color(if on {
            rgb(palette.success)
        } else {
            rgb(palette.border)
        })
        .bg(if on {
            rgb(palette.success)
        } else {
            rgb(palette.surface_elevated)
        })
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

pub(super) fn tunnel_status_style(
    palette: crate::theme::ThemePalette,
    pending: bool,
    is_open: bool,
    supported: bool,
) -> (Hsla, Hsla) {
    if pending {
        (rgb(palette.warning).into(), rgb(palette.hover).into())
    } else if is_open {
        (rgb(palette.success).into(), rgb(palette.hover).into())
    } else if supported {
        (rgb(palette.link).into(), rgb(palette.hover).into())
    } else {
        (rgb(palette.warning).into(), rgb(palette.hover).into())
    }
}

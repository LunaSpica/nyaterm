use super::tunnel::tunnel_editor_selector;
use super::*;
use super::common::{network_dialog_footer, network_modal_shell};

#[derive(Debug, Clone)]
pub(super) struct ProxySection {
    id: String,
    label: String,
    group: Option<ProxyGroup>,
    proxies: Vec<ProxyConfig>,
}

pub(super) fn proxy_sections(proxies: &[ProxyConfig], groups: &[ProxyGroup]) -> Vec<ProxySection> {
    let valid_group_ids = groups
        .iter()
        .map(|group| group.id.as_str())
        .collect::<HashSet<_>>();
    let mut by_group = HashMap::<String, Vec<ProxyConfig>>::new();
    let mut ungrouped = Vec::<ProxyConfig>::new();

    for proxy in proxies {
        match proxy.group_id.as_deref() {
            Some(group_id) if valid_group_ids.contains(group_id) => {
                by_group
                    .entry(group_id.to_string())
                    .or_default()
                    .push(proxy.clone());
            }
            _ => ungrouped.push(proxy.clone()),
        }
    }

    let mut sections = groups
        .iter()
        .cloned()
        .map(|group| ProxySection {
            id: group.id.clone(),
            label: group.name.clone(),
            proxies: by_group.remove(&group.id).unwrap_or_default(),
            group: Some(group),
        })
        .collect::<Vec<_>>();

    if !ungrouped.is_empty() || sections.is_empty() {
        sections.push(ProxySection {
            id: "__ungrouped__".to_string(),
            label: "Ungrouped".to_string(),
            group: None,
            proxies: ungrouped,
        });
    }

    sections
}

pub(super) fn proxy_section(
    section: ProxySection,
    app: &NyaTermApp,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
        let palette = cx.entity().read(cx).theme_palette();
    let item_count = section.proxies.len();
    let command_count = section
        .proxies
        .iter()
        .filter(|proxy| proxy.protocol == "proxycommand")
        .count();
    let section_key = format!("proxy:{}", section.id);
    let collapsed = !app.network_expanded_sections.contains(&section_key);
    let section_id_for_toggle = section.id.clone();
    let mut rows = div().flex().flex_col();
    if section.proxies.is_empty() {
        rows = rows.child(
            div()
                .border_t_1()
                .border_color(rgb(palette.border))
                .px_2()
                .py_2()
                .text_size(px(11.))
                .text_color(rgb(palette.text_muted))
                .child("No proxies in this group."),
        );
    } else {
        for proxy in section.proxies {
            let move_picker_open = app
                .network_move_picker
                .as_ref()
                .is_some_and(|picker| picker.tab == NetworkTab::Proxies && picker.id == proxy.id);
            rows = rows.child(
                div()
                    .flex()
                    .flex_col()
                    .child(proxy_network_row(&proxy, app, cx))
                    .when(move_picker_open, |this| {
                        this.child(proxy_move_picker(
                            proxy.id.clone(),
                            proxy.group_id.clone(),
                            &app.proxy_groups,
                            cx,
                        ))
                    }),
            );
        }
    }

    div()
        .id(gpui::SharedString::from(format!(
            "proxy-section-{}",
            section.id
        )))
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.surface))
        .overflow_hidden()
        .child(
            div()
                .id(gpui::SharedString::from(format!("proxy-section-header-{}", section.id)))
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
                            NetworkTab::Proxies,
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
                        .when(command_count > 0, |this| {
                            this.child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(0xa371f7))
                                    .child(format!("{command_count} cmd")),
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
                                format!("proxy-group-rename-{}", group.id),
                                "Rename",
                                cx.listener(move |this, _, _, cx| {
                                    this.open_network_group_editor(
                                        NetworkTab::Proxies,
                                        Some(rename_id.clone()),
                                        cx,
                                    );
                                }),
                            ))
                            .child(small_button(crate::ui::theme::theme_palette("github-dark"), 
                                format!("proxy-group-delete-{}", group.id),
                                "Delete",
                                cx.listener(move |this, _, _, cx| {
                                    this.open_network_group_delete_confirm(
                                        NetworkTab::Proxies,
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

pub(super) fn proxy_network_row(
    proxy: &ProxyConfig,
    app: &NyaTermApp,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
        let palette = cx.entity().read(cx).theme_palette();
    let is_command = proxy.protocol == "proxycommand";
    let address = if is_command {
        proxy
            .command
            .as_deref()
            .filter(|command| !command.trim().is_empty())
            .unwrap_or("ProxyCommand not configured")
            .to_string()
    } else if let Some(username) = proxy.username.as_deref().filter(|value| !value.is_empty()) {
        format!("{username}@{}:{}", proxy.host, proxy.port)
    } else {
        format!("{}:{}", proxy.host, proxy.port)
    };
    let proxy_id_for_move = proxy.id.clone();
    let proxy_id_for_edit = proxy.id.clone();
    let proxy_id_for_delete = proxy.id.clone();
    let proxy_label_for_delete = proxy.name.clone();

    // Tauri ProxyRow: name, protocol, address; overflow actions on the right.
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
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight(600.))
                        .text_color(rgb(palette.text))
                        .overflow_hidden()
                        .child(truncate_preview(&proxy.name, 52)),
                )
                .child(
                    div()
                        .mt(px(1.))
                        .text_size(px(11.))
                        .text_color(rgb(palette.text_muted))
                        .child(proxy_protocol_label(&proxy.protocol)),
                )
                .child(
                    div()
                        .mt(px(1.))
                        .font_family("JetBrains Mono")
                        .text_size(px(10.))
                        .text_color(rgb(palette.text_dimmed))
                        .overflow_hidden()
                        .child(truncate_preview(&address, 92)),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(proxy_icon_action(
                    format!("proxy-edit-{}", proxy.id),
                    "icons/net/edit.svg",
                    cx.listener(move |this, _, window, cx| {
                        this.open_network_proxy_editor(Some(proxy_id_for_edit.clone()), window, cx);
                    }),
                ))
                .when(!app.proxy_groups.is_empty(), |this| {
                    this.child(proxy_icon_action(
                        format!("proxy-move-group-{}", proxy.id),
                        "icons/net/move.svg",
                        cx.listener(move |this, _, _, cx| {
                            this.open_network_move_picker(
                                NetworkTab::Proxies,
                                proxy_id_for_move.clone(),
                                cx,
                            );
                        }),
                    ))
                })
                .child(proxy_icon_action(
                    format!("proxy-delete-{}", proxy.id),
                    "icons/net/delete.svg",
                    cx.listener(move |this, _, _, cx| {
                        this.open_network_delete_confirm(
                            NetworkTab::Proxies,
                            proxy_id_for_delete.clone(),
                            proxy_label_for_delete.clone(),
                            cx,
                        );
                    }),
                )),
        )
}

fn proxy_icon_action(
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

fn proxy_move_picker(
    proxy_id: String,
    current_group_id: Option<String>,
    groups: &[ProxyGroup],
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
        let target_id = proxy_id.clone();
        targets = targets.child(small_button(crate::ui::theme::theme_palette("github-dark"), 
            format!("network-proxy-move-{proxy_id}-ungrouped"),
            "Ungrouped",
            cx.listener(move |this, _, _, cx| {
                this.move_proxy_to_group(target_id.clone(), None, cx);
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
            let target_id = proxy_id.clone();
            let group_id = group.id.clone();
            targets = targets.child(small_button(crate::ui::theme::theme_palette("github-dark"), 
                format!("network-proxy-move-{proxy_id}-{}", group.id),
                "Move Here",
                cx.listener(move |this, _, _, cx| {
                    this.move_proxy_to_group(target_id.clone(), Some(group_id.clone()), cx);
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

pub(super) fn proxy_protocol_label(protocol: &str) -> &'static str {
    match protocol {
        "socks5" => "SOCKS5",
        "http" => "HTTP",
        "proxycommand" => "ProxyCommand",
        _ => "Proxy",
    }
}

pub(super) fn proxy_matches(proxy: &ProxyConfig, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    format!(
        "{} {} {} {} {} {} {}",
        proxy.id,
        proxy.name,
        proxy.protocol,
        proxy.host,
        proxy.port,
        proxy.command.as_deref().unwrap_or_default(),
        proxy.username.as_deref().unwrap_or_default()
    )
    .to_ascii_lowercase()
    .contains(query)
}

pub(super) fn network_proxy_editor_panel(
    editor: NetworkProxyEditorState,
    app: &NyaTermApp,
    focus: &gpui::FocusHandle,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
        let palette = cx.entity().read(cx).theme_palette();
    let protocol_label = proxy_protocol_label(&editor.protocol);
    let group_label = editor
        .group_id
        .as_deref()
        .and_then(|id| {
            app.proxy_groups
                .iter()
                .find(|group| group.id == id)
                .map(|group| group.name.clone())
        })
        .unwrap_or_else(|| "Ungrouped".to_string());
    let password_value = if editor.password.is_empty() {
        if editor.existing_password.is_some() || editor.password_id.is_some() {
            "keep existing".to_string()
        } else {
            String::new()
        }
    } else {
        "*".repeat(editor.password.chars().count().max(1))
    };

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
                                    "Edit Proxy"
                                } else {
                                    "New Proxy"
                                }),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(palette.text_muted))
                                .child("Configure SOCKS, HTTP, or ProxyCommand routing for SSH connections."),
                        ),
                )
                .child(status_pill(protocol_label, rgb(palette.accent), rgb(palette.hover))),
        )
        .child(
            div()
                .grid()
                .grid_cols(3)
                .gap_2()
                .child(tunnel_editor_selector(
                    "network-proxy-editor-protocol",
                    "Protocol",
                    protocol_label.to_string(),
                    cx.listener(|this, _, _, cx| {
                        this.cycle_network_proxy_protocol(cx);
                    }),
                ))
                .child(proxy_editor_input(
                    "network-proxy-editor-name",
                    "Proxy name",
                    editor.name.clone(),
                    editor.focused_field == NetworkProxyEditorField::Name,
                    NetworkProxyEditorField::Name,
                    focus,
                    cx,
                ))
                .child(tunnel_editor_selector(
                    "network-proxy-editor-group",
                    "Group",
                    group_label,
                    cx.listener(|this, _, _, cx| {
                        this.cycle_network_proxy_group(cx);
                    }),
                )),
        )
        .when(editor.is_proxy_command(), |this| {
            this.child(proxy_editor_input(
                "network-proxy-editor-command",
                "ProxyCommand",
                editor.command.clone(),
                editor.focused_field == NetworkProxyEditorField::Command,
                NetworkProxyEditorField::Command,
                focus,
                cx,
            ))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .child("Use Shift+Enter for a new line. Enter saves the proxy profile."),
            )
        })
        .when(!editor.is_proxy_command(), |this| {
            this.child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_2()
                    .child(proxy_editor_input(
                        "network-proxy-editor-host",
                        "Host",
                        editor.host.clone(),
                        editor.focused_field == NetworkProxyEditorField::Host,
                        NetworkProxyEditorField::Host,
                        focus,
                        cx,
                    ))
                    .child(proxy_editor_input(
                        "network-proxy-editor-port",
                        "Port",
                        editor.port.clone(),
                        editor.focused_field == NetworkProxyEditorField::Port,
                        NetworkProxyEditorField::Port,
                        focus,
                        cx,
                    )),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_2()
                    .child(proxy_editor_input(
                        "network-proxy-editor-username",
                        "Username",
                        editor.username.clone(),
                        editor.focused_field == NetworkProxyEditorField::Username,
                        NetworkProxyEditorField::Username,
                        focus,
                        cx,
                    ))
                    .child(proxy_editor_input(
                        "network-proxy-editor-password",
                        "Password",
                        password_value,
                        editor.focused_field == NetworkProxyEditorField::Password,
                        NetworkProxyEditorField::Password,
                        focus,
                        cx,
                    )),
            )
        })
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
                        .child(proxy_editor_preview(&editor)),
                ),
        )
        .when_some(editor.error.clone(), |this, error| {
            this.child(div().text_xs().text_color(rgb(palette.danger)).child(error))
        })
        .child(network_dialog_footer(cx.entity().read(cx).theme_palette(), 
            "network-proxy-editor-cancel",
            "network-proxy-editor-save",
            "Save",
            cx.listener(|this, _, _, cx| {
                this.close_network_proxy_editor(cx);
            }),
            cx.listener(|this, _, _, cx| {
                this.save_network_proxy_editor(cx);
            }),
        ));

    network_modal_shell(cx.entity().read(cx).theme_palette(), "network-proxy-editor-modal", 520., card)
}

pub(super) fn proxy_editor_input(
    id: impl Into<String>,
    label: &'static str,
    value: String,
    active: bool,
    field: NetworkProxyEditorField,
    focus: &gpui::FocusHandle,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    transfer_input(id, label, value, active, crate::ui::theme::theme_palette(&cx.entity().read(cx).settings.theme))
        .track_focus(focus)
        .on_click(cx.listener(move |this, _, window, cx| {
            this.focus_network_proxy_editor_field(field, window, cx);
        }))
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
            cx.stop_propagation();
            this.handle_network_proxy_editor_key_down(event, cx);
        }))
}

pub(super) fn proxy_editor_preview(editor: &NetworkProxyEditorState) -> String {
    if editor.is_proxy_command() {
        let command = editor.command.trim();
        if command.is_empty() {
            return "ProxyCommand ?".to_string();
        }
        return truncate_preview(command, 120);
    }

    let host = editor.host.trim();
    let host = if host.is_empty() { "?" } else { host };
    let port = editor.port.trim();
    let port = if port.is_empty() { "?" } else { port };
    if editor.username.trim().is_empty() {
        format!("{} {host}:{port}", proxy_protocol_label(&editor.protocol))
    } else {
        format!(
            "{} {}@{host}:{port}",
            proxy_protocol_label(&editor.protocol),
            editor.username.trim()
        )
    }
}

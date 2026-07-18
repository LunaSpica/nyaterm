use super::*;

pub(in crate::features::pages::tunnels) fn network_proxy_editor_panel(
    palette: crate::theme::ThemePalette,
    editor: NetworkProxyEditorState,
    app: &NyaTermApp,
    focus: &gpui::FocusHandle,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
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
        .unwrap_or_else(|| app.tr("network.ungrouped").to_string());
    let password_value = if editor.password.is_empty() {
        if editor.existing_password.is_some() || editor.password_id.is_some() {
            app.tr("network.proxyPasswordKeep").to_string()
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
                                    app.tr("network.editProxy")
                                } else {
                                    app.tr("network.newProxy")
                                }),
                        )
                        .child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(palette.text_muted))
                                .child(app.tr("network.proxyDialogDescription")),
                        ),
                )
                .child(status_pill(
                    protocol_label,
                    rgb(palette.link),
                    rgb(palette.hover),
                )),
        )
        .child(
            div()
                .grid()
                .grid_cols(3)
                .gap_2()
                .child(tunnel_editor_selector(
                    palette,
                    "network-proxy-editor-protocol",
                    app.tr("network.protocol"),
                    protocol_label.to_string(),
                    cx.listener(|this, _, _, cx| {
                        this.cycle_network_proxy_protocol(cx);
                    }),
                ))
                .child(proxy_editor_input(
                    palette,
                    "network-proxy-editor-name",
                    app.tr("network.proxyName"),
                    editor.name.clone(),
                    editor.focused_field == NetworkProxyEditorField::Name,
                    NetworkProxyEditorField::Name,
                    focus,
                    cx,
                ))
                .child(tunnel_editor_selector(
                    palette,
                    "network-proxy-editor-group",
                    app.tr("network.group"),
                    group_label,
                    cx.listener(|this, _, _, cx| {
                        this.cycle_network_proxy_group(cx);
                    }),
                )),
        )
        .when(editor.is_proxy_command(), |this| {
            this.child(proxy_editor_input(
                palette,
                "network-proxy-editor-command",
                app.tr("network.proxyCommand"),
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
                    .child(app.tr("network.proxyCommandHint")),
            )
        })
        .when(!editor.is_proxy_command(), |this| {
            this.child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_2()
                    .child(proxy_editor_input(
                        palette,
                        "network-proxy-editor-host",
                        app.tr("dialog.host"),
                        editor.host.clone(),
                        editor.focused_field == NetworkProxyEditorField::Host,
                        NetworkProxyEditorField::Host,
                        focus,
                        cx,
                    ))
                    .child(proxy_editor_input(
                        palette,
                        "network-proxy-editor-port",
                        app.tr("dialog.port"),
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
                        palette,
                        "network-proxy-editor-username",
                        app.tr("network.proxyUsername"),
                        editor.username.clone(),
                        editor.focused_field == NetworkProxyEditorField::Username,
                        NetworkProxyEditorField::Username,
                        focus,
                        cx,
                    ))
                    .child(proxy_editor_input(
                        palette,
                        "network-proxy-editor-password",
                        app.tr("network.proxyPassword"),
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
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette.text_muted))
                        .child(app.tr("network.tunnelPreview")),
                )
                .child(
                    div()
                        .font_family(crate::features::gpui_code_font_family())
                        .text_xs()
                        .text_color(rgb(palette.text))
                        .child(proxy_editor_preview(&editor)),
                ),
        )
        .when_some(editor.error.clone(), |this, error| {
            this.child(div().text_xs().text_color(rgb(palette.danger)).child(error))
        })
        .child(network_dialog_footer(
            app,
            palette,
            "network-proxy-editor-cancel",
            "network-proxy-editor-save",
            app.tr("common.save"),
            cx.listener(|this, _, _, cx| {
                this.close_network_proxy_editor(cx);
            }),
            cx.listener(|this, _, _, cx| {
                this.save_network_proxy_editor(cx);
            }),
        ));

    network_modal_shell(palette, "network-proxy-editor-modal", 520., card)
}

pub(in crate::features::pages::tunnels) fn proxy_editor_input(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    value: String,
    active: bool,
    field: NetworkProxyEditorField,
    focus: &gpui::FocusHandle,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    transfer_input(id, label, value, active, palette)
        .track_focus(focus)
        .on_click(cx.listener(move |this, _, window, cx| {
            this.focus_network_proxy_editor_field(field, window, cx);
        }))
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
            cx.stop_propagation();
            this.handle_network_proxy_editor_key_down(event, cx);
        }))
}

pub(in crate::features::pages::tunnels) fn proxy_editor_preview(
    editor: &NetworkProxyEditorState,
) -> String {
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

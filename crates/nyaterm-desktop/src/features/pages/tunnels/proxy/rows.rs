use super::*;

pub(in crate::features::pages::tunnels) fn proxy_network_row(
    proxy: &ProxyConfig,
    app: &NyaTermApp,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let palette = app.theme_palette();
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
                        .font_family(crate::features::gpui_code_font_family())
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
                    palette,
                    format!("proxy-edit-{}", proxy.id),
                    "icons/net/edit.svg",
                    cx.listener(move |this, _, window, cx| {
                        this.open_network_proxy_editor(Some(proxy_id_for_edit.clone()), window, cx);
                    }),
                ))
                .when(!app.proxy_groups.is_empty(), |this| {
                    this.child(proxy_icon_action(
                        palette,
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
                    palette,
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
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
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
        .hover(|this| {
            this.bg(rgb(palette.surface_elevated))
                .text_color(rgb(palette.text))
        })
        .child(svg().size(px(14.)).flex_none().path(label))
        .on_click(on_click)
}

pub(in crate::features::pages::tunnels) fn proxy_move_picker(
    palette: crate::theme::ThemePalette,
    proxy_id: String,
    current_group_id: Option<String>,
    groups: &[ProxyGroup],
    cx: &mut Context<NyaTermApp>,
) -> gpui::Div {
    let mut targets = div().flex().flex_wrap().items_center().gap_2();
    if current_group_id.is_none() {
        targets = targets.child(status_pill(
            "Ungrouped · current",
            rgb(palette.accent),
            rgb(palette.hover),
        ));
    } else {
        let target_id = proxy_id.clone();
        targets = targets.child(small_button(
            palette,
            format!("network-proxy-move-{proxy_id}-ungrouped"),
            "Ungrouped",
            cx.listener(move |this, _, _, cx| {
                this.move_proxy_to_group(target_id.clone(), None, cx);
            }),
        ));
    }

    for group in groups {
        if current_group_id.as_deref() == Some(group.id.as_str()) {
            targets = targets.child(status_pill(
                "current",
                rgb(palette.success),
                rgb(palette.hover),
            ));
            targets = targets.child(
                div()
                    .text_xs()
                    .text_color(rgb(palette.text))
                    .child(truncate_preview(&group.name, 36)),
            );
        } else {
            let target_id = proxy_id.clone();
            let group_id = group.id.clone();
            targets = targets.child(small_button(
                palette,
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

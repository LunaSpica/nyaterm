use super::*;

#[derive(Debug, Clone)]
pub(in crate::features::pages::tunnels) struct ProxySection {
    id: String,
    label: String,
    group: Option<ProxyGroup>,
    proxies: Vec<ProxyConfig>,
}

pub(in crate::features::pages::tunnels) fn proxy_sections(
    proxies: &[ProxyConfig],
    groups: &[ProxyGroup],
    ungrouped_label: &'static str,
) -> Vec<ProxySection> {
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
            label: ungrouped_label.to_string(),
            group: None,
            proxies: ungrouped,
        });
    }

    sections
}

pub(in crate::features::pages::tunnels) fn proxy_section(
    palette: crate::theme::ThemePalette,
    section: ProxySection,
    app: &NyaTermApp,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
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
                .child(app.tr("network.groupEmpty")),
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
                            palette,
                            proxy.id.clone(),
                            proxy.group_id.clone(),
                            &app.proxy_groups,
                            app,
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
        .child(
            div()
                .id(gpui::SharedString::from(format!(
                    "proxy-section-header-{}",
                    section.id
                )))
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
                    let menu_id = format!("group:{}", group.id);
                    let menu_open = app
                        .network_item_menu
                        .as_ref()
                        .is_some_and(|menu| menu.tab == NetworkTab::Proxies && menu.id == menu_id);
                    this.child(network_item_overflow_menu(
                        palette,
                        format!("proxy-group-actions-{}", group.id),
                        menu_open,
                        app.tr("common.more"),
                        app.tr("network.renameGroup"),
                        app.tr("network.moveToGroup"),
                        app.tr("network.deleteGroup"),
                        false,
                        cx.listener({
                            let id = menu_id.clone();
                            move |this, _, _, cx| {
                                this.toggle_network_item_menu(NetworkTab::Proxies, id.clone(), cx);
                            }
                        }),
                        cx.listener(move |this, _, _, cx| {
                            this.open_network_group_editor(
                                NetworkTab::Proxies,
                                Some(rename_id.clone()),
                                cx,
                            );
                        }),
                        cx.listener(|_, _, _, _| {}),
                        cx.listener(move |this, _, _, cx| {
                            this.open_network_group_delete_confirm(
                                NetworkTab::Proxies,
                                delete_id.clone(),
                                delete_label.clone(),
                                item_count,
                                cx,
                            );
                        }),
                    ))
                }),
        )
        .when(!collapsed, |this| this.child(rows))
}

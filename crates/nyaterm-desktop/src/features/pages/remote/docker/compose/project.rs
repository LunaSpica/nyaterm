use super::*;

pub(in crate::features::pages::remote) fn docker_compose_panel(
    palette: crate::theme::ThemePalette,
    menu_bg: gpui::Rgba,
    projects: &[DockerComposeProject],
    expanded_projects: &HashSet<String>,
    services_by_project: &HashMap<String, Vec<DockerComposeService>>,
    service_errors: &HashMap<String, String>,
    open_menu_id: Option<&str>,
    labels: DockerLabels,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    // Tauri Compose tab: dense project rows (≈74px) + chevron + ⋮ overflow; services ≈58px.
    let mut rows = div().flex().flex_col().gap_1();
    if projects.is_empty() {
        rows = rows.child(empty_panel(labels.no_matches, palette));
    } else {
        for project in projects {
            let config_files = Some(project.config_files.clone()).filter(|value| {
                !value.trim().is_empty() && value.trim().to_ascii_lowercase() != "n/a"
            });
            let key = docker_compose_project_key(&project.name, config_files.as_deref());
            let expanded = expanded_projects.contains(&key);
            let services = services_by_project.get(&key).cloned();
            let error = service_errors.get(&key).cloned();
            let project_menu_id = format!("compose-project:{key}");
            let project_menu_open = open_menu_id == Some(project_menu_id.as_str());
            rows = rows.child(docker_compose_project_row(
                palette,
                menu_bg,
                project,
                &key,
                expanded,
                project_menu_open,
                project_menu_id,
                services,
                error,
                open_menu_id,
                labels,
                cx,
            ));
        }
    }

    docker_resource_static_panel(palette, "Compose", projects.len(), rows)
}

pub(in crate::features::pages::remote) fn docker_compose_project_row(
    palette: crate::theme::ThemePalette,
    menu_bg: gpui::Rgba,
    project: &DockerComposeProject,
    project_key: &str,
    expanded: bool,
    menu_open: bool,
    menu_id: String,
    services: Option<Vec<DockerComposeService>>,
    error: Option<String>,
    open_menu_id: Option<&str>,
    labels: DockerLabels,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let project_name = project.name.clone();
    let config_files = Some(project.config_files.clone())
        .filter(|value| !value.trim().is_empty() && value.trim().to_ascii_lowercase() != "n/a");
    let status_label = compose_status_label(&project.status);
    let status_color = compose_status_color(palette, status_label);
    let display_status = if project.status.trim().is_empty() {
        "-"
    } else {
        labels.state_label(status_label)
    };
    let key_for_toggle = project_key.to_string();

    div()
        .id(SharedString::from(format!(
            "docker-compose-project-{project_key}"
        )))
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.section_header))
        .hover(|this| this.bg(rgb(palette.hover)))
        .overflow_hidden()
        .flex()
        .flex_col()
        .child(
            div()
                .relative()
                .h(px(74.))
                .px_2()
                .flex()
                .items_start()
                .gap_2()
                .child(
                    div()
                        .id(SharedString::from(format!(
                            "docker-compose-chevron-{project_key}"
                        )))
                        .mt(px(10.))
                        .h(px(24.))
                        .w(px(24.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_md()
                        .text_color(rgb(palette.text_muted))
                        .cursor_pointer()
                        .hover(|this| {
                            this.bg(rgb(palette.surface_elevated))
                                .text_color(rgb(palette.text))
                        })
                        .child(svg().size(px(14.)).path(if expanded {
                            "icons/chevron-down.svg"
                        } else {
                            "icons/fe/forward.svg"
                        }))
                        .on_click(cx.listener({
                            let project_name = project_name.clone();
                            let config_files = config_files.clone();
                            move |this, _, window, cx| {
                                this.docker_compose_menu_id = None;
                                this.toggle_docker_compose_project(
                                    project_name.clone(),
                                    config_files.clone(),
                                    window,
                                    cx,
                                );
                            }
                        })),
                )
                .child(
                    div()
                        .id(SharedString::from(format!(
                            "docker-compose-body-{project_key}"
                        )))
                        .min_w_0()
                        .flex_1()
                        .pt(px(8.))
                        .pr(px(34.))
                        .flex()
                        .flex_col()
                        .gap_1()
                        .cursor_pointer()
                        .on_click(cx.listener({
                            let project_name = project_name.clone();
                            let config_files = config_files.clone();
                            move |this, _, window, cx| {
                                this.docker_compose_menu_id = None;
                                this.toggle_docker_compose_project(
                                    project_name.clone(),
                                    config_files.clone(),
                                    window,
                                    cx,
                                );
                            }
                        }))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(palette.text))
                                        .overflow_hidden()
                                        .child(truncate_preview(&project.name, 42)),
                                )
                                .child(status_pill(display_status, status_color, rgb(0x17233a))),
                        )
                        .child(
                            div()
                                .font_family(crate::features::gpui_code_font_family())
                                .text_size(px(10.))
                                .text_color(rgb(palette.text_dimmed))
                                .overflow_hidden()
                                .child(truncate_preview(&project.config_files, 64)),
                        ),
                )
                .child(
                    div().absolute().top(px(8.)).right(px(6.)).child(
                        div()
                            .relative()
                            .child(svg_icon_button(
                                format!("docker-compose-menu-{project_key}"),
                                "icons/session/more.svg",
                                14.,
                                palette,
                                cx.listener({
                                    let menu_id = menu_id.clone();
                                    move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        if this.docker_compose_menu_id.as_deref()
                                            == Some(menu_id.as_str())
                                        {
                                            this.docker_compose_menu_id = None;
                                        } else {
                                            this.docker_compose_menu_id = Some(menu_id.clone());
                                        }
                                        cx.notify();
                                    }
                                }),
                            ))
                            .when(menu_open, |this| {
                                this.child(docker_compose_project_action_menu(
                                    palette,
                                    menu_bg,
                                    project_name.clone(),
                                    config_files.clone(),
                                    &key_for_toggle,
                                    labels,
                                    cx,
                                ))
                            }),
                    ),
                ),
        )
        .when(expanded, |this| {
            this.child(docker_compose_services_panel(
                palette,
                menu_bg,
                project_name,
                config_files,
                project_key.to_string(),
                services,
                error,
                open_menu_id,
                labels,
                cx,
            ))
        })
}

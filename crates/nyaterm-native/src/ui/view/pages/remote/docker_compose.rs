use super::*;

pub(in crate::ui::view::pages::remote) fn docker_compose_panel(
    projects: &[DockerComposeProject],
    expanded_projects: &HashSet<String>,
    services_by_project: &HashMap<String, Vec<DockerComposeService>>,
    service_errors: &HashMap<String, String>,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let mut rows = div().mt_3().flex().flex_col().gap_2();
    if projects.is_empty() {
        rows = rows.child(empty_panel("No compose projects loaded."));
    } else {
        for project in projects {
            let project_name = project.name.clone();
            let config_files = Some(project.config_files.clone()).filter(|value| {
                !value.trim().is_empty() && value.trim().to_ascii_lowercase() != "n/a"
            });
            let key = docker_compose_project_key(&project.name, config_files.as_deref());
            let expanded = expanded_projects.contains(&key);
            let services = services_by_project.get(&key).cloned();
            let error = service_errors.get(&key).cloned();
            rows = rows.child(
                div()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(0x303848))
                    .bg(rgb(0x0d1320))
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .font_family("JetBrains Mono")
                                            .text_xs()
                                            .text_color(rgb(0xe5edf7))
                                            .child(truncate_preview(&project.name, 48)),
                                    )
                                    .child(div().text_xs().text_color(rgb(0x98a3b8)).child(
                                        format!(
                                            "{} · {}",
                                            project.status,
                                            truncate_preview(&project.config_files, 56)
                                        ),
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(small_button(
                                        format!("docker-compose-toggle-{project_name}"),
                                        if expanded { "Close" } else { "Open" },
                                        cx.listener({
                                            let project_name = project_name.clone();
                                            let config_files = config_files.clone();
                                            move |this, _, window, cx| {
                                                this.toggle_docker_compose_project(
                                                    project_name.clone(),
                                                    config_files.clone(),
                                                    window,
                                                    cx,
                                                );
                                            }
                                        }),
                                    ))
                                    .child(small_button(
                                        format!("docker-compose-reload-{project_name}"),
                                        "Reload",
                                        cx.listener({
                                            let project_name = project_name.clone();
                                            let config_files = config_files.clone();
                                            move |this, _, window, cx| {
                                                this.load_docker_compose_services(
                                                    project_name.clone(),
                                                    config_files.clone(),
                                                    window,
                                                    cx,
                                                );
                                            }
                                        }),
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_1()
                            .child(compose_action_button(
                                "up",
                                project_name.clone(),
                                config_files.clone(),
                                cx,
                            ))
                            .child(compose_action_button(
                                "restart",
                                project_name.clone(),
                                config_files.clone(),
                                cx,
                            ))
                            .child(compose_action_button(
                                "down",
                                project_name.clone(),
                                config_files.clone(),
                                cx,
                            )),
                    )
                    .when(expanded, |this| {
                        this.child(docker_compose_services_panel(
                            project_name,
                            config_files,
                            services,
                            error,
                            cx,
                        ))
                    }),
            );
        }
    }

    docker_resource_panel("Compose", projects.len(), rows)
}

pub(in crate::ui::view::pages::remote) fn docker_compose_services_panel(
    project_name: String,
    config_files: Option<String>,
    services: Option<Vec<DockerComposeService>>,
    error: Option<String>,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let mut rows = div()
        .border_t_1()
        .border_color(rgb(0x303848))
        .pt_2()
        .flex()
        .flex_col()
        .gap_2();
    if let Some(error) = error {
        rows = rows.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0xfca5a5))
                        .child(truncate_preview(&error, 80)),
                )
                .child(small_button(
                    format!("docker-compose-retry-{project_name}"),
                    "Retry",
                    cx.listener({
                        let project_name = project_name.clone();
                        let config_files = config_files.clone();
                        move |this, _, window, cx| {
                            this.load_docker_compose_services(
                                project_name.clone(),
                                config_files.clone(),
                                window,
                                cx,
                            );
                        }
                    }),
                )),
        );
    } else if let Some(services) = services {
        if services.is_empty() {
            rows = rows.child(empty_panel("No compose services reported."));
        } else {
            for service in services {
                rows = rows.child(docker_compose_service_row(
                    project_name.clone(),
                    config_files.clone(),
                    service,
                    cx,
                ));
            }
        }
    } else {
        rows = rows.child(
            div()
                .text_xs()
                .text_color(rgb(0x98a3b8))
                .child("Loading compose services..."),
        );
    }
    rows
}

pub(in crate::ui::view::pages::remote) fn docker_compose_service_row(
    project_name: String,
    config_files: Option<String>,
    service: DockerComposeService,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let container_summary = if service.containers.is_empty() {
        "no containers".to_string()
    } else {
        service
            .containers
            .iter()
            .take(3)
            .map(|container| {
                let name = if container.name.trim().is_empty() {
                    compact_id(&container.id)
                } else {
                    truncate_preview(&container.name, 24)
                };
                format!("{name} {}", docker_state_label(&container.state))
            })
            .collect::<Vec<_>>()
            .join(" · ")
    };
    let service_name = service.name.clone();
    let service_status_lower = service.status.to_ascii_lowercase();
    let service_status_label = if service.status.trim().is_empty() {
        "not created"
    } else if service_status_lower.contains("running") {
        "running"
    } else if service_status_lower.contains("exited")
        || service_status_lower.contains("stopped")
        || service_status_lower.contains("created")
    {
        "stopped"
    } else {
        "status"
    };
    let service_status_color = if service_status_label == "running" {
        rgb(0x6ee7b7)
    } else {
        rgb(0x98a3b8)
    };
    let running_container_id = service
        .containers
        .iter()
        .filter(|container| container.state.eq_ignore_ascii_case("running"))
        .min_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)))
        .map(|container| container.id.clone());

    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x273244))
        .bg(rgb(0x10151e))
        .p_2()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .child(
            div()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .font_family("JetBrains Mono")
                                .text_xs()
                                .text_color(rgb(0xe5edf7))
                                .child(truncate_preview(&service.name, 42)),
                        )
                        .child(status_pill(
                            service_status_label,
                            service_status_color,
                            rgb(0x17233a),
                        )),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x98a3b8))
                        .child(truncate_preview(&container_summary, 88)),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(small_button(
                    format!("docker-compose-service-logs-{project_name}-{service_name}"),
                    "Logs",
                    cx.listener({
                        let project_name = project_name.clone();
                        let config_files = config_files.clone();
                        let service_name = service_name.clone();
                        move |this, _, _, cx| {
                            this.send_docker_compose_service_logs_to_terminal(
                                project_name.clone(),
                                config_files.clone(),
                                service_name.clone(),
                                cx,
                            );
                        }
                    }),
                ))
                .when_some(running_container_id, |this, container_id| {
                    this.child(small_button(
                        format!("docker-compose-service-enter-{project_name}-{service_name}"),
                        "Enter",
                        cx.listener(move |this, _, _, cx| {
                            this.enter_docker_container_terminal(container_id.clone(), cx);
                        }),
                    ))
                })
                .child(compose_service_action_button(
                    "up",
                    project_name.clone(),
                    config_files.clone(),
                    service_name.clone(),
                    cx,
                ))
                .child(compose_service_action_button(
                    "stop",
                    project_name.clone(),
                    config_files.clone(),
                    service_name.clone(),
                    cx,
                ))
                .child(compose_service_action_button(
                    "restart",
                    project_name,
                    config_files,
                    service_name,
                    cx,
                )),
        )
}

pub(in crate::ui::view::pages::remote) fn compose_service_action_button(
    action: &'static str,
    project_name: String,
    config_files: Option<String>,
    service_name: String,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    small_button(
        format!("docker-compose-service-{action}-{project_name}-{service_name}"),
        match action {
            "up" => "Up",
            "stop" => "Stop",
            "restart" => "Restart",
            _ => "Run",
        },
        cx.listener(move |this, _, window, cx| {
            this.docker_compose_service_action(
                project_name.clone(),
                config_files.clone(),
                service_name.clone(),
                action,
                window,
                cx,
            );
        }),
    )
}

pub(in crate::ui::view::pages::remote) fn compose_action_button(
    action: &'static str,
    project_name: String,
    config_files: Option<String>,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    small_button(
        format!("docker-compose-{action}-{project_name}"),
        match action {
            "up" => "Up",
            "restart" => "Restart",
            "down" => "Down",
            _ => "Run",
        },
        cx.listener(move |this, _, window, cx| {
            if action == "down" {
                this.request_docker_confirm(
                    DockerConfirmState {
                        title: format!("Compose {action} {project_name}"),
                        detail: format!("docker compose {action} for project {project_name}"),
                        action: DockerConfirmAction::ComposeAction {
                            project_name: project_name.clone(),
                            config_files: config_files.clone(),
                            action,
                        },
                    },
                    cx,
                );
            } else {
                this.docker_compose_action(
                    project_name.clone(),
                    config_files.clone(),
                    action,
                    window,
                    cx,
                );
            }
        }),
    )
}

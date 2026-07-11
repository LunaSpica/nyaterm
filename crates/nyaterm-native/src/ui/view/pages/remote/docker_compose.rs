use super::*;
use gpui::SharedString;

pub(in crate::ui::view::pages::remote) fn docker_compose_panel(
    projects: &[DockerComposeProject],
    expanded_projects: &HashSet<String>,
    services_by_project: &HashMap<String, Vec<DockerComposeService>>,
    service_errors: &HashMap<String, String>,
    open_menu_id: Option<&str>,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    // Tauri Compose tab: dense project rows (≈74px) + chevron + ⋮ overflow; services ≈58px.
    let mut rows = div().flex().flex_col().gap_1();
    if projects.is_empty() {
        rows = rows.child(empty_panel("No compose projects loaded.", super::cx_theme_palette(cx)));
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
                project,
                &key,
                expanded,
                project_menu_open,
                project_menu_id,
                services,
                error,
                open_menu_id,
                cx,
            ));
        }
    }

    docker_resource_static_panel("Compose", projects.len(), rows)
}

fn docker_compose_project_row(
    project: &DockerComposeProject,
    project_key: &str,
    expanded: bool,
    menu_open: bool,
    menu_id: String,
    services: Option<Vec<DockerComposeService>>,
    error: Option<String>,
    open_menu_id: Option<&str>,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let project_name = project.name.clone();
    let config_files = Some(project.config_files.clone()).filter(|value| {
        !value.trim().is_empty() && value.trim().to_ascii_lowercase() != "n/a"
    });
    let status_label = compose_status_label(&project.status);
    let status_color = compose_status_color(status_label);
    let chevron = if expanded { "▾" } else { "▸" };
    let key_for_toggle = project_key.to_string();

    div()
        .id(SharedString::from(format!("docker-compose-project-{project_key}")))
        .rounded_md()
        .border_1()
        .border_color(rgb(0x30363d))
        .bg(rgb(0x12171f))
        .hover(|this| this.bg(rgb(0x18202b)))
        .overflow_hidden()
        .flex()
        .flex_col()
        .child(
            div()
                .relative()
                .h(px(60.))
                .px_2()
                .flex()
                .items_start()
                .gap_2()
                .child(
                    div()
                        .id(SharedString::from(format!(
                            "docker-compose-chevron-{project_key}"
                        )))
                        .mt(px(6.))
                        .h(px(24.))
                        .w(px(24.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_md()
                        .text_size(px(12.))
                        .text_color(rgb(0x8b949e))
                        .cursor_pointer()
                        .hover(|this| this.bg(rgb(0x21262d)).text_color(rgb(0xe5edf7)))
                        .child(chevron)
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
                                        .text_color(rgb(0xe5edf7))
                                        .overflow_hidden()
                                        .child(truncate_preview(&project.name, 42)),
                                )
                                .child(status_pill(status_label, status_color, rgb(0x17233a))),
                        )
                        .child(
                            div()
                                .font_family("JetBrains Mono")
                                .text_size(px(10.))
                                .text_color(rgb(0x6e7681))
                                .overflow_hidden()
                                .child(truncate_preview(&project.config_files, 64)),
                        ),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(8.))
                        .right(px(6.))
                        .child(
                            div()
                                .relative()
                                .child(icon_button(
                                    format!("docker-compose-menu-{project_key}"),
                                    "⋮", super::cx_theme_palette(cx),cx.listener({
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
                                        project_name.clone(),
                                        config_files.clone(),
                                        &key_for_toggle,
                                        cx,
                                    ))
                                }),
                        ),
                ),
        )
        .when(expanded, |this| {
            this.child(docker_compose_services_panel(
                project_name,
                config_files,
                project_key.to_string(),
                services,
                error,
                open_menu_id,
                cx,
            ))
        })
}

pub(in crate::ui::view::pages::remote) fn docker_compose_services_panel(
    project_name: String,
    config_files: Option<String>,
    project_key: String,
    services: Option<Vec<DockerComposeService>>,
    error: Option<String>,
    open_menu_id: Option<&str>,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let mut rows = div()
        .border_t_1()
        .border_color(rgb(0x30363d))
        .px_2()
        .pb_2()
        .pt_1()
        .flex()
        .flex_col()
        .gap_1();
    if let Some(error) = error {
        rows = rows.child(
            div()
                .h(px(36.))
                .px_2()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .text_size(px(11.))
                        .text_color(rgb(0xfca5a5))
                        .overflow_hidden()
                        .child(truncate_preview(&error, 80)),
                )
                .child(small_button(crate::ui::theme::theme_palette("github-dark"), 
                    format!("docker-compose-retry-{project_name}"),
                    "Retry",
                    cx.listener({
                        let project_name = project_name.clone();
                        let config_files = config_files.clone();
                        move |this, _, window, cx| {
                            this.docker_compose_menu_id = None;
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
            rows = rows.child(
                div()
                    .h(px(40.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(12.))
                    .text_color(rgb(0x6e7681))
                    .child("No compose services reported."),
            );
        } else {
            for service in services {
                let service_menu_id =
                    format!("compose-service:{project_key}:{}", service.name);
                let menu_open = open_menu_id == Some(service_menu_id.as_str());
                rows = rows.child(docker_compose_service_row(
                    project_name.clone(),
                    config_files.clone(),
                    service,
                    menu_open,
                    service_menu_id,
                    cx,
                ));
            }
        }
    } else {
        rows = rows.child(
            div()
                .h(px(36.))
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(11.))
                .text_color(rgb(0x8b949e))
                .child("Loading compose services…"),
        );
    }
    rows
}

pub(in crate::ui::view::pages::remote) fn docker_compose_service_row(
    project_name: String,
    config_files: Option<String>,
    service: DockerComposeService,
    menu_open: bool,
    menu_id: String,
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
    let service_status_label = compose_status_label(&service.status);
    let service_status_color = compose_status_color(service_status_label);
    let running_container_id = service
        .containers
        .iter()
        .filter(|container| container.state.eq_ignore_ascii_case("running"))
        .min_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)))
        .map(|container| container.id.clone());
    let can_enter = running_container_id.is_some();
    let row_id = format!("docker-compose-service-{project_name}-{service_name}");

    div()
        .id(SharedString::from(row_id.clone()))
        .relative()
        .h(px(48.))
        .rounded_md()
        .bg(rgb(0x0d1117))
        .hover(|this| this.bg(rgb(0x151b24)))
        .px_2()
        .pr(px(36.))
        .flex()
        .items_center()
        .child(
            div()
                .min_w_0()
                .flex_1()
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
                                .min_w_0()
                                .flex_1()
                                .text_size(px(12.))
                                .font_weight(FontWeight(600.))
                                .text_color(rgb(0xe5edf7))
                                .overflow_hidden()
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
                        .font_family("JetBrains Mono")
                        .text_size(px(10.))
                        .text_color(rgb(0x6e7681))
                        .overflow_hidden()
                        .child(truncate_preview(&container_summary, 72)),
                ),
        )
        .child(
            div()
                .absolute()
                .top(px(14.))
                .right(px(4.))
                .child(
                    div()
                        .relative()
                        .child(icon_button(
                            format!("{row_id}-menu"),
                            "⋮", super::cx_theme_palette(cx),cx.listener({
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
                            this.child(docker_compose_service_action_menu(
                                project_name,
                                config_files,
                                service_name,
                                running_container_id,
                                can_enter,
                                cx,
                            ))
                        }),
                ),
        )
}

fn docker_compose_project_action_menu(
    project_name: String,
    config_files: Option<String>,
    project_key: &str,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let short = project_key.replace(['/', ':', ' '], "-");
    div()
        .id(SharedString::from(format!("docker-compose-project-menu-{short}")))
        .absolute()
        .top(px(28.))
        .right_0()
        .w(px(140.))
        .rounded_md()
        .border_1()
        .border_color(rgb(0x30363d))
        .bg(rgb(0x161b22))
        .shadow_lg()
        .py_1()
        .flex()
        .flex_col()
        .on_mouse_down(gpui::MouseButton::Left, |_, _, _| {})
        .child(compose_menu_item(
            format!("compose-up-{short}"),
            "Up",
            false,
            cx.listener({
                let project_name = project_name.clone();
                let config_files = config_files.clone();
                move |this, _, window, cx| {
                    this.docker_compose_menu_id = None;
                    this.docker_compose_action(
                        project_name.clone(),
                        config_files.clone(),
                        "up",
                        window,
                        cx,
                    );
                }
            }),
        ))
        .child(compose_menu_item(
            format!("compose-restart-{short}"),
            "Restart",
            false,
            cx.listener({
                let project_name = project_name.clone();
                let config_files = config_files.clone();
                move |this, _, window, cx| {
                    this.docker_compose_menu_id = None;
                    this.docker_compose_action(
                        project_name.clone(),
                        config_files.clone(),
                        "restart",
                        window,
                        cx,
                    );
                }
            }),
        ))
        .child(compose_menu_separator())
        .child(compose_menu_item(
            format!("compose-down-{short}"),
            "Down",
            false,
            cx.listener(move |this, _, _, cx| {
                this.docker_compose_menu_id = None;
                this.request_docker_confirm(
                    DockerConfirmState {
                        title: format!("Compose down {project_name}"),
                        detail: format!("docker compose down for project {project_name}"),
                        action: DockerConfirmAction::ComposeAction {
                            project_name: project_name.clone(),
                            config_files: config_files.clone(),
                            action: "down",
                        },
                    },
                    cx,
                );
            }),
        ))
}

fn docker_compose_service_action_menu(
    project_name: String,
    config_files: Option<String>,
    service_name: String,
    running_container_id: Option<String>,
    can_enter: bool,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let short = format!("{project_name}-{service_name}")
        .replace(['/', ':', ' '], "-");
    div()
        .id(SharedString::from(format!("docker-compose-service-menu-{short}")))
        .absolute()
        .top(px(28.))
        .right_0()
        .w(px(140.))
        .rounded_md()
        .border_1()
        .border_color(rgb(0x30363d))
        .bg(rgb(0x161b22))
        .shadow_lg()
        .py_1()
        .flex()
        .flex_col()
        .on_mouse_down(gpui::MouseButton::Left, |_, _, _| {})
        .child(compose_menu_item(
            format!("compose-svc-logs-{short}"),
            "Logs",
            false,
            cx.listener({
                let project_name = project_name.clone();
                let config_files = config_files.clone();
                let service_name = service_name.clone();
                move |this, _, _, cx| {
                    this.docker_compose_menu_id = None;
                    this.send_docker_compose_service_logs_to_terminal(
                        project_name.clone(),
                        config_files.clone(),
                        service_name.clone(),
                        cx,
                    );
                }
            }),
        ))
        .child(compose_menu_item(
            format!("compose-svc-enter-{short}"),
            "Enter",
            !can_enter,
            cx.listener(move |this, _, _, cx| {
                this.docker_compose_menu_id = None;
                if let Some(container_id) = running_container_id.clone() {
                    this.enter_docker_container_terminal(container_id, cx);
                }
            }),
        ))
        .child(compose_menu_separator())
        .child(compose_menu_item(
            format!("compose-svc-up-{short}"),
            "Up",
            false,
            cx.listener({
                let project_name = project_name.clone();
                let config_files = config_files.clone();
                let service_name = service_name.clone();
                move |this, _, window, cx| {
                    this.docker_compose_menu_id = None;
                    this.docker_compose_service_action(
                        project_name.clone(),
                        config_files.clone(),
                        service_name.clone(),
                        "up",
                        window,
                        cx,
                    );
                }
            }),
        ))
        .child(compose_menu_item(
            format!("compose-svc-stop-{short}"),
            "Stop",
            false,
            cx.listener({
                let project_name = project_name.clone();
                let config_files = config_files.clone();
                let service_name = service_name.clone();
                move |this, _, window, cx| {
                    this.docker_compose_menu_id = None;
                    this.docker_compose_service_action(
                        project_name.clone(),
                        config_files.clone(),
                        service_name.clone(),
                        "stop",
                        window,
                        cx,
                    );
                }
            }),
        ))
        .child(compose_menu_item(
            format!("compose-svc-restart-{short}"),
            "Restart",
            false,
            cx.listener({
                let project_name = project_name.clone();
                let config_files = config_files.clone();
                let service_name = service_name.clone();
                move |this, _, window, cx| {
                    this.docker_compose_menu_id = None;
                    this.docker_compose_service_action(
                        project_name.clone(),
                        config_files.clone(),
                        service_name.clone(),
                        "restart",
                        window,
                        cx,
                    );
                }
            }),
        ))
}

fn compose_menu_item(
    id: impl Into<String>,
    label: &'static str,
    disabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .text_size(px(12.))
        .text_color(if disabled {
            rgb(0x484f58)
        } else {
            rgb(0xc9d1d9)
        })
        .when(!disabled, |this| {
            this.cursor_pointer()
                .hover(|s| s.bg(rgb(0x21262d)))
                .on_click(on_click)
        })
        .when(disabled, |this| this.opacity(0.5))
        .child(label)
}

fn compose_menu_separator() -> impl IntoElement {
    div().h(px(1.)).mx_2().my_1().bg(rgb(0x30363d))
}

fn compose_status_label(status: &str) -> &'static str {
    let lower = status.trim().to_ascii_lowercase();
    if lower.is_empty() || lower == "-" {
        "—"
    } else if lower.contains("running") || lower == "up" {
        "running"
    } else if lower.contains("exited") || lower.contains("stopped") || lower.contains("down") {
        "stopped"
    } else if lower.contains("created") {
        "created"
    } else if lower.contains("paused") {
        "paused"
    } else if lower.contains("not created") {
        "not created"
    } else {
        "status"
    }
}

fn compose_status_color(status: &str) -> gpui::Hsla {
    match status {
        "running" => rgb(0x6ee7b7).into(),
        "stopped" => rgb(0xfca5a5).into(),
        "created" | "paused" => rgb(0xfbbf24).into(),
        "not created" => rgb(0x98a3b8).into(),
        _ => rgb(0x98a3b8).into(),
    }
}

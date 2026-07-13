use super::*;

pub(in crate::features::pages::remote) fn docker_compose_services_panel(
    palette: crate::theme::ThemePalette,
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
        .border_color(rgb(palette.border))
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
                .child(small_button(
                    palette,
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
                    .text_color(rgb(palette.text_dimmed))
                    .child("No compose services reported."),
            );
        } else {
            for service in services {
                let service_menu_id = format!("compose-service:{project_key}:{}", service.name);
                let menu_open = open_menu_id == Some(service_menu_id.as_str());
                rows = rows.child(docker_compose_service_row(
                    palette,
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
                .text_color(rgb(palette.text_muted))
                .child("Loading compose services…"),
        );
    }
    rows
}

pub(in crate::features::pages::remote) fn docker_compose_service_row(
    palette: crate::theme::ThemePalette,
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
    let service_status_color = compose_status_color(palette, service_status_label);
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
        .bg(rgb(palette.bg))
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
                                .text_color(rgb(palette.text))
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
                        .text_color(rgb(palette.text_dimmed))
                        .overflow_hidden()
                        .child(truncate_preview(&container_summary, 72)),
                ),
        )
        .child(
            div().absolute().top(px(14.)).right(px(4.)).child(
                div()
                    .relative()
                    .child(icon_button(
                        format!("{row_id}-menu"),
                        "⋮",
                        palette,
                        cx.listener({
                            let menu_id = menu_id.clone();
                            move |this, _, _, cx| {
                                cx.stop_propagation();
                                if this.docker_compose_menu_id.as_deref() == Some(menu_id.as_str())
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
                            palette,
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

use super::*;

pub(in crate::features::pages::remote) fn docker_compose_project_action_menu(
    palette: crate::theme::ThemePalette,
    project_name: String,
    config_files: Option<String>,
    project_key: &str,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let short = project_key.replace(['/', ':', ' '], "-");
    div()
        .id(SharedString::from(format!(
            "docker-compose-project-menu-{short}"
        )))
        .absolute()
        .top(px(28.))
        .right_0()
        .w(px(140.))
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.surface))
        .shadow_lg()
        .py_1()
        .flex()
        .flex_col()
        .on_mouse_down(gpui::MouseButton::Left, |_, _, _| {})
        .child(compose_menu_item(
            palette,
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
            palette,
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
        .child(compose_menu_separator(palette))
        .child(compose_menu_item(
            palette,
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

pub(in crate::features::pages::remote) fn docker_compose_service_action_menu(
    palette: crate::theme::ThemePalette,
    project_name: String,
    config_files: Option<String>,
    service_name: String,
    running_container_id: Option<String>,
    can_enter: bool,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let short = format!("{project_name}-{service_name}").replace(['/', ':', ' '], "-");
    div()
        .id(SharedString::from(format!(
            "docker-compose-service-menu-{short}"
        )))
        .absolute()
        .top(px(28.))
        .right_0()
        .w(px(140.))
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.surface))
        .shadow_lg()
        .py_1()
        .flex()
        .flex_col()
        .on_mouse_down(gpui::MouseButton::Left, |_, _, _| {})
        .child(compose_menu_item(
            palette,
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
            palette,
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
        .child(compose_menu_separator(palette))
        .child(compose_menu_item(
            palette,
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
            palette,
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
            palette,
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

pub(in crate::features::pages::remote) fn compose_menu_item(
    palette: crate::theme::ThemePalette,
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
            rgb(palette.border)
        } else {
            rgb(palette.text)
        })
        .when(!disabled, |this| {
            this.cursor_pointer()
                .hover(|s| s.bg(rgb(palette.surface_elevated)))
                .on_click(on_click)
        })
        .when(disabled, |this| this.opacity(0.5))
        .child(label)
}

pub(in crate::features::pages::remote) fn compose_menu_separator(
    palette: crate::theme::ThemePalette,
) -> impl IntoElement {
    div().h(px(1.)).mx_2().my_1().bg(rgb(palette.border))
}

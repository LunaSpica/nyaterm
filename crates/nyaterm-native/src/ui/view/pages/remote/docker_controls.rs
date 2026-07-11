use super::*;

pub(in crate::ui::view::pages::remote) fn docker_tab_bar(
    active_tab: DockerTab,
    overview: &nyaterm_session::RemoteDockerOverview,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    // Dense tab strip similar to Tauri Docker manager tabs.
    div()
        .h(px(32.))
        .flex_none()
        .px_2()
        .border_b_1()
        .border_color(rgb(0x30363d))
        .bg(rgb(0x12171f))
        .flex()
        .items_center()
        .gap_1()
        .overflow_hidden()
        .child(docker_tab_button(
            "docker-tab-containers",
            format!(
                "{} {}",
                DockerTab::Containers.label(),
                overview.containers.len()
            ),
            active_tab == DockerTab::Containers,
            cx.listener(|this, _, _, cx| {
                this.set_docker_tab(DockerTab::Containers, cx);
            }),
        ))
        .child(docker_tab_button(
            "docker-tab-images",
            format!("{} {}", DockerTab::Images.label(), overview.images.len()),
            active_tab == DockerTab::Images,
            cx.listener(|this, _, _, cx| {
                this.set_docker_tab(DockerTab::Images, cx);
            }),
        ))
        .child(docker_tab_button(
            "docker-tab-volumes",
            format!("{} {}", DockerTab::Volumes.label(), overview.volumes.len()),
            active_tab == DockerTab::Volumes,
            cx.listener(|this, _, _, cx| {
                this.set_docker_tab(DockerTab::Volumes, cx);
            }),
        ))
        .child(docker_tab_button(
            "docker-tab-networks",
            format!(
                "{} {}",
                DockerTab::Networks.label(),
                overview.networks.len()
            ),
            active_tab == DockerTab::Networks,
            cx.listener(|this, _, _, cx| {
                this.set_docker_tab(DockerTab::Networks, cx);
            }),
        ))
        .child(
            div()
                .when(!overview.compose_available, |this| this.opacity(0.45))
                .child(docker_tab_button(
                    "docker-tab-compose",
                    if overview.compose_available {
                        format!(
                            "{} {}",
                            DockerTab::Compose.label(),
                            overview.compose_projects.len()
                        )
                    } else {
                        format!("{} off", DockerTab::Compose.label())
                    },
                    active_tab == DockerTab::Compose,
                    cx.listener(|this, _, _, cx| {
                        this.set_docker_tab(DockerTab::Compose, cx);
                    }),
                )),
        )
}

fn docker_tab_button(
    id: &'static str,
    label: String,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(gpui::SharedString::from(id))
        .h(px(24.))
        .px_2()
        .flex()
        .items_center()
        .rounded_sm()
        .bg(if active { rgb(0x21262d) } else { rgb(0x0d1117) })
        .text_color(if active { rgb(0xc9d1d9) } else { rgb(0x8b949e) })
        .text_size(px(11.))
        .font_weight(if active { FontWeight(600.) } else { FontWeight(500.) })
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x1c2128)).text_color(rgb(0xc9d1d9)))
        .child(label)
        .on_click(on_click)
}

pub(in crate::ui::view::pages::remote) fn docker_confirm_panel(
    confirm: DockerConfirmState,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xfb7185))
        .bg(rgb(0x2a121a))
        .p_3()
        .flex()
        .items_center()
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
                        .text_sm()
                        .font_weight(FontWeight(800.))
                        .text_color(rgb(0xfda4af))
                        .child(confirm.title),
                )
                .child(
                    div()
                        .font_family("JetBrains Mono")
                        .text_xs()
                        .text_color(rgb(0xfecdd3))
                        .child(confirm.detail),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(small_button(crate::ui::theme::theme_palette("github-dark"), 
                    "docker-confirm-cancel",
                    "Cancel",
                    cx.listener(|this, _, _, cx| {
                        this.cancel_docker_confirm(cx);
                    }),
                ))
                .child(small_button(crate::ui::theme::theme_palette("github-dark"), 
                    "docker-confirm-run",
                    "Confirm",
                    cx.listener(|this, _, window, cx| {
                        this.confirm_docker_action(window, cx);
                    }),
                )),
        )
}

pub(in crate::ui::view::pages::remote) fn docker_container_confirm_button(
    action: &'static str,
    label: &'static str,
    container_id: String,
    container_name: String,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    small_button(crate::ui::theme::theme_palette("github-dark"), 
        format!("docker-{action}-{}", compact_id(&container_id)),
        label,
        cx.listener(move |this, _, _, cx| {
            let target = if container_name.trim().is_empty() {
                compact_id(&container_id)
            } else {
                container_name.clone()
            };
            this.request_docker_confirm(
                DockerConfirmState {
                    title: format!("{label} container {target}"),
                    detail: format!(
                        "docker {} {}",
                        if action == "remove" { "rm" } else { action },
                        compact_id(&container_id)
                    ),
                    action: DockerConfirmAction::ContainerAction {
                        container_id: container_id.clone(),
                        action,
                    },
                },
                cx,
            );
        }),
    )
}

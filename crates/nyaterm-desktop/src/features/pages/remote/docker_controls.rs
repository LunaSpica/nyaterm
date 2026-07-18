use super::*;

pub(in crate::features::pages::remote) fn docker_tab_bar(
    palette: crate::theme::ThemePalette,
    active_tab: DockerTab,
    overview: &nyaterm_transport::RemoteDockerOverview,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    // Dense tab strip similar to Tauri Docker manager tabs.
    div()
        .h(px(32.))
        .flex_none()
        .px_2()
        .border_b_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.section_header))
        .flex()
        .items_center()
        .gap_1()
        .overflow_hidden()
        .child(docker_tab_button(
            palette,
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
            palette,
            "docker-tab-images",
            format!("{} {}", DockerTab::Images.label(), overview.images.len()),
            active_tab == DockerTab::Images,
            cx.listener(|this, _, _, cx| {
                this.set_docker_tab(DockerTab::Images, cx);
            }),
        ))
        .child(docker_tab_button(
            palette,
            "docker-tab-volumes",
            format!("{} {}", DockerTab::Volumes.label(), overview.volumes.len()),
            active_tab == DockerTab::Volumes,
            cx.listener(|this, _, _, cx| {
                this.set_docker_tab(DockerTab::Volumes, cx);
            }),
        ))
        .child(docker_tab_button(
            palette,
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
                    palette,
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
    palette: crate::theme::ThemePalette,
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
        .bg(if active {
            rgb(palette.surface_elevated)
        } else {
            rgb(palette.bg)
        })
        .text_color(if active {
            rgb(palette.text)
        } else {
            rgb(palette.text_muted)
        })
        .text_size(px(11.))
        .font_weight(if active {
            FontWeight(600.)
        } else {
            FontWeight(500.)
        })
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
        .child(label)
        .on_click(on_click)
}

pub(in crate::features::pages::remote) fn docker_confirm_panel(
    palette: crate::theme::ThemePalette,
    confirm: DockerConfirmState,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let card = div()
        .p_4()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .text_size(px(15.))
                .font_weight(FontWeight(800.))
                .text_color(rgb(0xfda4af))
                .child(confirm.title),
        )
        .child(
            div()
                .font_family(crate::features::gpui_code_font_family())
                .text_xs()
                .line_height(px(17.))
                .text_color(rgb(0xfecdd3))
                .child(confirm.detail),
        )
        .child(
            div()
                .pt_2()
                .flex()
                .justify_end()
                .gap_2()
                .child(small_button(
                    palette,
                    "docker-confirm-cancel",
                    "Cancel",
                    cx.listener(|this, _, _, cx| {
                        this.cancel_docker_confirm(cx);
                    }),
                ))
                .child(small_button(
                    palette,
                    "docker-confirm-run",
                    "Confirm",
                    cx.listener(|this, _, window, cx| {
                        this.confirm_docker_action(window, cx);
                    }),
                )),
        );
    modal_dialog_shell(palette, "docker-confirm-modal", 420., card)
}

pub(in crate::features::pages::remote) fn docker_container_confirm_button(
    palette: crate::theme::ThemePalette,
    action: &'static str,
    label: &'static str,
    container_id: String,
    container_name: String,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    small_button(
        palette,
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

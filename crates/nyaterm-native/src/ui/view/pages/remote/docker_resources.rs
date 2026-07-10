use super::*;

pub(in crate::ui::view::pages::remote) fn docker_images_panel(
    images: &[DockerImage],
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let mut rows = div().mt_3().flex().flex_col().gap_2();
    if images.is_empty() {
        rows = rows.child(empty_panel("No images loaded."));
    } else {
        for image in images {
            let image_id = image.id.clone();
            let label = docker_image_label(image);
            rows = rows.child(
                docker_resource_row(
                    label.clone(),
                    format!(
                        "{} · {} · {}",
                        compact_id(&image.id),
                        image.created_since,
                        image.size
                    ),
                )
                .child(small_button(
                    format!("docker-image-remove-{}", compact_id(&image_id)),
                    "Remove",
                    cx.listener(move |this, _, _, cx| {
                        this.request_docker_confirm(
                            DockerConfirmState {
                                title: format!("Remove image {label}"),
                                detail: format!("docker image rm {}", compact_id(&image_id)),
                                action: DockerConfirmAction::ImageRemove {
                                    image_id: image_id.clone(),
                                    force: false,
                                },
                            },
                            cx,
                        );
                    }),
                )),
            );
        }
    }

    docker_resource_panel("Images", images.len(), rows)
}

pub(in crate::ui::view::pages::remote) fn docker_volumes_panel(
    volumes: &[DockerVolume],
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let mut rows = div().mt_3().flex().flex_col().gap_2();
    if volumes.is_empty() {
        rows = rows.child(empty_panel("No volumes loaded."));
    } else {
        for volume in volumes {
            let volume_name = volume.name.clone();
            rows = rows.child(
                docker_resource_row(volume.name.clone(), format!("driver {}", volume.driver))
                    .child(small_button(
                        format!("docker-volume-remove-{volume_name}"),
                        "Remove",
                        cx.listener(move |this, _, _, cx| {
                            this.request_docker_confirm(
                                DockerConfirmState {
                                    title: format!("Remove volume {volume_name}"),
                                    detail: format!("docker volume rm {volume_name}"),
                                    action: DockerConfirmAction::VolumeRemove {
                                        volume_name: volume_name.clone(),
                                        force: false,
                                    },
                                },
                                cx,
                            );
                        }),
                    )),
            );
        }
    }

    docker_resource_panel("Volumes", volumes.len(), rows)
}

pub(in crate::ui::view::pages::remote) fn docker_networks_panel(
    networks: &[DockerNetwork],
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let mut rows = div().mt_3().flex().flex_col().gap_2();
    if networks.is_empty() {
        rows = rows.child(empty_panel("No networks loaded."));
    } else {
        for network in networks {
            let network_id = network.id.clone();
            let name = network.name.clone();
            rows = rows.child(
                docker_resource_row(
                    network.name.clone(),
                    format!(
                        "{} · {} · {}",
                        compact_id(&network.id),
                        network.driver,
                        network.scope
                    ),
                )
                .child(small_button(
                    format!("docker-network-remove-{}", compact_id(&network_id)),
                    "Remove",
                    cx.listener(move |this, _, _, cx| {
                        this.request_docker_confirm(
                            DockerConfirmState {
                                title: format!("Remove network {name}"),
                                detail: format!("docker network rm {}", compact_id(&network_id)),
                                action: DockerConfirmAction::NetworkRemove {
                                    network_id: network_id.clone(),
                                },
                            },
                            cx,
                        );
                    }),
                )),
            );
        }
    }

    docker_resource_panel("Networks", networks.len(), rows)
}

pub(in crate::ui::view::pages::remote) fn docker_resource_panel(
    title: &'static str,
    count: usize,
    rows: impl IntoElement,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x151923))
        .p_4()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(div().text_sm().font_weight(FontWeight(700.)).child(title))
                .child(
                    div()
                        .font_family("JetBrains Mono")
                        .text_xs()
                        .text_color(rgb(0x98a3b8))
                        .child(count.to_string()),
                ),
        )
        .child(rows)
}

pub(in crate::ui::view::pages::remote) fn docker_resource_row(
    title: String,
    detail: String,
) -> gpui::Div {
    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x303848))
        .bg(rgb(0x0d1320))
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
                        .font_family("JetBrains Mono")
                        .text_xs()
                        .text_color(rgb(0xe5edf7))
                        .child(truncate_preview(&title, 48)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x98a3b8))
                        .child(truncate_preview(&detail, 72)),
                ),
        )
}

pub(in crate::ui::view::pages::remote) fn docker_image_label(image: &DockerImage) -> String {
    match (
        image.repository.trim().is_empty(),
        image.tag.trim().is_empty(),
    ) {
        (true, true) => compact_id(&image.id),
        (false, true) => image.repository.clone(),
        (true, false) => image.tag.clone(),
        (false, false) => format!("{}:{}", image.repository, image.tag),
    }
}

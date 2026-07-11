use super::*;

pub(in crate::ui::view::pages::remote) fn docker_images_panel(
    images: &[DockerImage],
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let mut rows = div().flex().flex_col().gap_1();
    if images.is_empty() {
        rows = rows.child(empty_panel("No images loaded."));
    } else {
        const WINDOW: usize = 80;
        let total = images.len();
        let slice = if total > WINDOW { &images[..WINDOW] } else { images };
        for image in slice {
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
                .child(icon_button(
                    format!("docker-image-remove-{}", compact_id(&image_id)),
                    "×",
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
        if total > WINDOW {
            rows = rows.child(
                div()
                    .px_2()
                    .py_1()
                    .text_size(px(10.))
                    .text_color(rgb(0x6e7681))
                    .child(format!("Showing first {WINDOW} of {total} images · refine search")),
            );
        }
    }

    docker_resource_panel("Images", images.len(), rows)
}

pub(in crate::ui::view::pages::remote) fn docker_volumes_panel(
    volumes: &[DockerVolume],
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let mut rows = div().flex().flex_col().gap_1();
    if volumes.is_empty() {
        rows = rows.child(empty_panel("No volumes loaded."));
    } else {
        const WINDOW: usize = 80;
        let total = volumes.len();
        let slice = if total > WINDOW { &volumes[..WINDOW] } else { volumes };
        for volume in slice {
            let volume_name = volume.name.clone();
            rows = rows.child(
                docker_resource_row(volume.name.clone(), format!("driver {}", volume.driver))
                    .child(icon_button(
                        format!("docker-volume-remove-{volume_name}"),
                        "×",
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
        if total > WINDOW {
            rows = rows.child(
                div()
                    .px_2()
                    .py_1()
                    .text_size(px(10.))
                    .text_color(rgb(0x6e7681))
                    .child(format!("Showing first {WINDOW} of {total} volumes · refine search")),
            );
        }
    }

    docker_resource_panel("Volumes", volumes.len(), rows)
}

pub(in crate::ui::view::pages::remote) fn docker_networks_panel(
    networks: &[DockerNetwork],
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let mut rows = div().flex().flex_col().gap_1();
    if networks.is_empty() {
        rows = rows.child(empty_panel("No networks loaded."));
    } else {
        const WINDOW: usize = 80;
        let total = networks.len();
        let slice = if total > WINDOW { &networks[..WINDOW] } else { networks };
        for network in slice {
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
                .child(icon_button(
                    format!("docker-network-remove-{}", compact_id(&network_id)),
                    "×",
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
        if total > WINDOW {
            rows = rows.child(
                div()
                    .px_2()
                    .py_1()
                    .text_size(px(10.))
                    .text_color(rgb(0x6e7681))
                    .child(format!("Showing first {WINDOW} of {total} networks · refine search")),
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
    // Tauri resource tabs: full-height list, no nested section card header.
    let _ = title;
    div()
        .id(gpui::SharedString::from(format!(
            "docker-resource-{}",
            title.to_ascii_lowercase()
        )))
        .size_full()
        .overflow_scroll()
        .scrollbar_width(px(6.))
        .p_2()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .h(px(22.))
                .flex_none()
                .px_1()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(10.))
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(0x6e7681))
                        .child(format!("{title} · {count}")),
                ),
        )
        .child(rows)
}

pub(in crate::ui::view::pages::remote) fn docker_resource_row(
    title: String,
    detail: String,
) -> gpui::Div {
    // ~64px Tauri SIMPLE_ROW_HEIGHT-ish dense resource row (slightly tighter chrome).
    div()
        .h(px(52.))
        .rounded_md()
        .border_1()
        .border_color(rgb(0x30363d))
        .bg(rgb(0x12171f))
        .px_3()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .hover(|this| this.bg(rgb(0x18202b)))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight(600.))
                        .text_color(rgb(0xc9d1d9))
                        .overflow_hidden()
                        .child(truncate_preview(&title, 48)),
                )
                .child(
                    div()
                        .font_family("JetBrains Mono")
                        .text_size(px(10.))
                        .text_color(rgb(0x6e7681))
                        .overflow_hidden()
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

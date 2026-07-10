use super::*;

pub(in crate::ui::view::pages::remote) fn docker_details_panel(
    container_id: Option<String>,
    details: Option<DockerContainerDetails>,
    container: Option<DockerContainer>,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let Some(details) = details else {
        return div()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x2a3140))
            .bg(rgb(0x151923))
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_sm()
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(0xe5edf7))
                    .child("Container Details"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(0x98a3b8))
                    .child("Select Details on a container to load inspect data, live stats, mounts, and networks."),
            );
    };

    let mut mounts = div().flex().flex_col().gap_1();
    if details.mounts.is_empty() {
        mounts = mounts.child(empty_panel("No mounts reported."));
    } else {
        for mount in &details.mounts {
            mounts = mounts.child(
                div()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(0x303848))
                    .bg(rgb(0x0d1320))
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .font_family("JetBrains Mono")
                            .text_xs()
                            .text_color(rgb(0xe5edf7))
                            .child(format!(
                                "{} -> {}",
                                truncate_preview(&mount.source, 52),
                                truncate_preview(&mount.destination, 52)
                            )),
                    )
                    .child(div().text_xs().text_color(rgb(0x98a3b8)).child(format!(
                        "{} · {} · {}",
                        mount.kind,
                        mount.mode,
                        if mount.rw { "rw" } else { "ro" }
                    ))),
            );
        }
    }

    let mut networks = div().flex().flex_col().gap_1();
    if details.networks.is_empty() {
        networks = networks.child(empty_panel("No networks reported."));
    } else {
        for network in &details.networks {
            let ip_address = if network.ip_address.trim().is_empty() {
                "no ip".to_string()
            } else {
                network.ip_address.clone()
            };
            networks = networks.child(
                div()
                    .mt_2()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_sm()
                    .child(
                        div()
                            .text_color(rgb(0xcbd5e1))
                            .child(truncate_preview(&network.name, 36)),
                    )
                    .child(div().text_xs().text_color(rgb(0x98a3b8)).child(ip_address)),
            );
        }
    }

    let details_id = container_id
        .as_deref()
        .map(compact_id)
        .unwrap_or_else(|| "unknown".to_string());
    let details_title = container
        .as_ref()
        .map(|container| {
            format!(
                "Container Details · {}",
                truncate_preview(&container.name, 40)
            )
        })
        .unwrap_or_else(|| "Container Details".to_string());
    let details_state = container.as_ref().map(|container| container.state.clone());
    let networks_value = docker_networks_value(&details);
    let mounts_value = docker_mounts_value(&details);
    let mut actions = div().flex().items_center().gap_2();
    if let Some(container_id) = container_id.clone() {
        actions = actions
            .child(small_button(
                format!("docker-details-refresh-{}", compact_id(&container_id)),
                "Refresh",
                cx.listener(move |this, _, window, cx| {
                    this.load_docker_details(container_id.clone(), window, cx);
                }),
            ))
            .child(small_button(
                "docker-details-close",
                "Close",
                cx.listener(|this, _, _, cx| {
                    this.close_docker_details(cx);
                }),
            ));
    }
    actions = actions.child(
        div()
            .rounded_sm()
            .px_2()
            .py_1()
            .text_xs()
            .text_color(rgb(0x93c5fd))
            .bg(rgb(0x17233a))
            .child(details_id),
    );

    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x151923))
        .p_4()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .min_w_0()
                                .text_sm()
                                .font_weight(FontWeight(700.))
                                .text_color(rgb(0xe5edf7))
                                .child(details_title),
                        )
                        .when_some(details_state, |this, state| {
                            this.child(status_pill(
                                docker_state_label(&state),
                                docker_state_color(&state),
                                rgb(0x17233a),
                            ))
                        }),
                )
                .child(actions),
        )
        .child(
            div()
                .grid()
                .grid_cols(5)
                .gap_2()
                .child(metric(
                    "CPU",
                    details
                        .stats
                        .as_ref()
                        .map(|stats| format!("{:.1}%", stats.cpu_percent))
                        .unwrap_or_else(|| "n/a".to_string()),
                ))
                .child(metric(
                    "Memory",
                    details
                        .stats
                        .as_ref()
                        .map(|stats| format!("{:.1}%", stats.memory_percent))
                        .unwrap_or_else(|| "n/a".to_string()),
                ))
                .child(metric(
                    "Net IO",
                    details
                        .stats
                        .as_ref()
                        .map(|stats| truncate_preview(&stats.net_io, 24))
                        .unwrap_or_else(|| "n/a".to_string()),
                ))
                .child(metric(
                    "Block IO",
                    details
                        .stats
                        .as_ref()
                        .map(|stats| truncate_preview(&stats.block_io, 24))
                        .unwrap_or_else(|| "n/a".to_string()),
                ))
                .child(metric(
                    "PIDs",
                    details
                        .stats
                        .as_ref()
                        .map(|stats| stats.pids.clone())
                        .unwrap_or_else(|| "n/a".to_string()),
                )),
        )
        .when_some(container, |this, container| {
            this.child(
                div()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(0x303848))
                    .bg(rgb(0x0d1320))
                    .p_3()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(0xe5edf7))
                            .child("Identity"),
                    )
                    .child(docker_detail_line(
                        "Name",
                        container.name.clone(),
                        truncate_preview(&container.name, 72),
                        true,
                        cx,
                    ))
                    .child(docker_detail_line(
                        "ID",
                        container.id.clone(),
                        truncate_preview(&container.id, 72),
                        true,
                        cx,
                    ))
                    .child(docker_detail_line(
                        "Image",
                        container.image.clone(),
                        truncate_preview(&container.image, 72),
                        true,
                        cx,
                    ))
                    .child(docker_detail_line(
                        "Status",
                        if container.status.trim().is_empty() {
                            container.state.clone()
                        } else {
                            container.status.clone()
                        },
                        truncate_preview(
                            if container.status.trim().is_empty() {
                                &container.state
                            } else {
                                &container.status
                            },
                            72,
                        ),
                        false,
                        cx,
                    ))
                    .child(docker_detail_line(
                        "Created",
                        container.created_at.clone(),
                        truncate_preview(&container.created_at, 72),
                        false,
                        cx,
                    ))
                    .child(docker_detail_line(
                        "Size",
                        if container.size.trim().is_empty() {
                            "-".to_string()
                        } else {
                            container.size.clone()
                        },
                        if container.size.trim().is_empty() {
                            "-".to_string()
                        } else {
                            container.size.clone()
                        },
                        false,
                        cx,
                    ))
                    .child(docker_detail_line(
                        "Ports",
                        if container.ports.trim().is_empty() {
                            "-".to_string()
                        } else {
                            docker_ports_value(&container.ports)
                        },
                        if container.ports.trim().is_empty() {
                            "-".to_string()
                        } else {
                            truncate_preview(&docker_ports_value(&container.ports), 96)
                        },
                        true,
                        cx,
                    )),
            )
        })
        .child(
            div()
                .grid()
                .grid_cols(2)
                .gap_3()
                .child(
                    div()
                        .rounded_sm()
                        .border_1()
                        .border_color(rgb(0x303848))
                        .bg(rgb(0x0d1320))
                        .p_3()
                        .child(docker_detail_line(
                            "Started",
                            details.started_at.clone(),
                            truncate_preview(&details.started_at, 52),
                            false,
                            cx,
                        ))
                        .child(docker_detail_line(
                            "Finished",
                            details.finished_at.clone(),
                            truncate_preview(&details.finished_at, 52),
                            false,
                            cx,
                        ))
                        .child(docker_detail_line(
                            "Restarts",
                            details.restart_count.to_string(),
                            details.restart_count.to_string(),
                            false,
                            cx,
                        ))
                        .child(docker_detail_line(
                            "Entrypoint",
                            details.entrypoint.clone(),
                            truncate_preview(&details.entrypoint, 72),
                            true,
                            cx,
                        ))
                        .child(docker_detail_line(
                            "Command",
                            details.command.clone(),
                            truncate_preview(&details.command, 72),
                            true,
                            cx,
                        )),
                )
                .child(
                    div()
                        .rounded_sm()
                        .border_1()
                        .border_color(rgb(0x303848))
                        .bg(rgb(0x0d1320))
                        .p_3()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight(700.))
                                .text_color(rgb(0xe5edf7))
                                .child("Networks"),
                        )
                        .child(docker_detail_line(
                            "Networks",
                            networks_value.clone(),
                            truncate_preview(&networks_value, 96),
                            true,
                            cx,
                        ))
                        .child(networks),
                ),
        )
        .child(
            div()
                .rounded_sm()
                .border_1()
                .border_color(rgb(0x303848))
                .bg(rgb(0x0d1320))
                .p_3()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(0xe5edf7))
                        .child("Mounts"),
                )
                .child(docker_detail_line(
                    "Mounts",
                    mounts_value.clone(),
                    truncate_preview(&mounts_value, 120),
                    true,
                    cx,
                ))
                .child(mounts),
        )
}

fn docker_detail_line(
    label: &'static str,
    value: String,
    display_value: String,
    copyable: bool,
    cx: &mut Context<NyaTermApp>,
) -> gpui::Div {
    let copy_value = value.clone();
    div()
        .mt_2()
        .flex()
        .items_start()
        .justify_between()
        .gap_3()
        .text_sm()
        .child(
            div()
                .w(px(86.))
                .flex_none()
                .text_color(rgb(0xcbd5e1))
                .child(label),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .font_family("JetBrains Mono")
                .text_xs()
                .line_height(px(18.))
                .text_color(rgb(0x98a3b8))
                .child(display_value),
        )
        .when(copyable && value.trim() != "-", |this| {
            this.child(small_button(
                format!("docker-details-copy-{label}"),
                "Copy",
                cx.listener(move |this, _, _, cx| {
                    this.copy_docker_text(copy_value.clone(), label, cx);
                }),
            ))
        })
}

fn docker_ports_value(ports: &str) -> String {
    let value = ports
        .split(',')
        .map(str::trim)
        .filter(|port| !port.is_empty())
        .map(|port| port.replace("->", " -> "))
        .collect::<Vec<_>>()
        .join("\n");
    if value.trim().is_empty() {
        "-".to_string()
    } else {
        value
    }
}

fn docker_networks_value(details: &DockerContainerDetails) -> String {
    let value = details
        .networks
        .iter()
        .map(|network| {
            if network.ip_address.trim().is_empty() {
                network.name.clone()
            } else {
                format!("{}: {}", network.name, network.ip_address)
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if value.trim().is_empty() {
        "-".to_string()
    } else {
        value
    }
}

fn docker_mounts_value(details: &DockerContainerDetails) -> String {
    let value = details
        .mounts
        .iter()
        .map(|mount| {
            let access = if mount.rw { "rw" } else { "ro" };
            let mode = if mount.mode.trim().is_empty() {
                access.to_string()
            } else {
                format!("{access},{}", mount.mode)
            };
            format!(
                "{} {} -> {} ({mode})",
                if mount.kind.trim().is_empty() {
                    "mount"
                } else {
                    &mount.kind
                },
                if mount.source.trim().is_empty() {
                    "-"
                } else {
                    &mount.source
                },
                if mount.destination.trim().is_empty() {
                    "-"
                } else {
                    &mount.destination
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    if value.trim().is_empty() {
        "-".to_string()
    } else {
        value
    }
}

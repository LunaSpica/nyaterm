use super::*;

pub(in crate::ui::view::pages::remote) fn docker_containers_panel(
    has_snapshot: bool,
    has_session: bool,
    docker_available: bool,
    filtered_containers: &[DockerContainer],
    query_empty: bool,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let mut rows = div().mt_3().flex().flex_col().gap_2();
    if !has_snapshot {
        return rows
            .child(empty_panel(if has_session {
                "No Docker snapshot loaded."
            } else {
                "Start an SSH session to inspect remote Docker."
            }))
            .into_any_element();
    }
    if !docker_available {
        return rows
            .child(empty_panel(
                "Docker is not installed or the daemon is not reachable.",
            ))
            .into_any_element();
    }
    if filtered_containers.is_empty() {
        return rows
            .child(empty_panel(if query_empty {
                "No containers found."
            } else {
                "No containers match the Docker search."
            }))
            .into_any_element();
    }

    let mut containers = filtered_containers.to_vec();
    containers.sort_by(|left, right| {
        docker_state_rank(&left.state)
            .cmp(&docker_state_rank(&right.state))
            .then(left.name.cmp(&right.name))
    });
    for container in containers {
        rows = rows.child(docker_container_row(container, cx));
    }
    rows.into_any_element()
}

fn docker_container_row(
    container: DockerContainer,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let container_id = container.id.clone();
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x2a3140))
        .bg(rgb(0x151923))
        .p_3()
        .flex()
        .flex_col()
        .gap_2()
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
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight(700.))
                                .text_color(rgb(0xe5edf7))
                                .child(truncate_preview(&container.name, 48)),
                        )
                        .child(
                            div()
                                .text_xs()
                                .font_family("JetBrains Mono")
                                .text_color(rgb(0x98a3b8))
                                .child(format!(
                                    "{} · {}",
                                    compact_id(&container.id),
                                    truncate_preview(&container.image, 64)
                                )),
                        ),
                )
                .child(status_pill(
                    docker_state_label(&container.state),
                    docker_state_color(&container.state),
                    rgb(0x17233a),
                )),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0xaeb7c8))
                .line_height(px(18.))
                .child(truncate_preview(&container.status, 120)),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(0x64748b))
                .child(truncate_preview(&container.ports, 120)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap_1()
                .child(small_button(
                    format!("docker-details-{}", compact_id(&container_id)),
                    "Details",
                    cx.listener({
                        let container_id = container_id.clone();
                        move |this, _, window, cx| {
                            this.load_docker_details(container_id.clone(), window, cx);
                        }
                    }),
                ))
                .child(small_button(
                    format!("docker-logs-{}", compact_id(&container_id)),
                    "Logs",
                    cx.listener({
                        let container_id = container_id.clone();
                        move |this, _, window, cx| {
                            this.load_docker_logs(container_id.clone(), window, cx);
                        }
                    }),
                ))
                .child(small_button(
                    format!("docker-follow-{}", compact_id(&container_id)),
                    "Follow",
                    cx.listener({
                        let container_id = container_id.clone();
                        move |this, _, _, cx| {
                            this.send_docker_container_logs_to_terminal(container_id.clone(), cx);
                        }
                    }),
                ))
                .child(small_button(
                    format!("docker-enter-{}", compact_id(&container_id)),
                    "Enter",
                    cx.listener({
                        let container_id = container_id.clone();
                        move |this, _, _, cx| {
                            this.enter_docker_container_terminal(container_id.clone(), cx);
                        }
                    }),
                ))
                .child(small_button(
                    format!("docker-start-{}", compact_id(&container_id)),
                    "Start",
                    cx.listener({
                        let container_id = container_id.clone();
                        move |this, _, window, cx| {
                            this.docker_container_action(container_id.clone(), "start", window, cx);
                        }
                    }),
                ))
                .child(small_button(
                    format!("docker-stop-{}", compact_id(&container_id)),
                    "Stop",
                    cx.listener({
                        let container_id = container_id.clone();
                        move |this, _, window, cx| {
                            this.docker_container_action(container_id.clone(), "stop", window, cx);
                        }
                    }),
                ))
                .child(small_button(
                    format!("docker-restart-{}", compact_id(&container_id)),
                    "Restart",
                    cx.listener({
                        let container_id = container_id.clone();
                        move |this, _, window, cx| {
                            this.docker_container_action(
                                container_id.clone(),
                                "restart",
                                window,
                                cx,
                            );
                        }
                    }),
                ))
                .child(docker_container_confirm_button(
                    "kill",
                    "Kill",
                    container_id.clone(),
                    container.name.clone(),
                    cx,
                ))
                .child(docker_container_confirm_button(
                    "remove",
                    "Remove",
                    container_id,
                    container.name,
                    cx,
                )),
        )
}

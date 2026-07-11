use super::*;
use gpui::{SharedString, prelude::*};

pub(in crate::ui::view::pages::remote) fn docker_containers_panel(
    has_snapshot: bool,
    has_session: bool,
    docker_available: bool,
    filtered_containers: &[DockerContainer],
    query_empty: bool,
    open_menu_id: Option<&str>,
    list_offset: usize,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    // Tauri Docker containers tab: dense ~66px rows, left accent, ⋮ action menu.
    if !has_snapshot {
        return div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(empty_panel(if has_session {
                "No Docker snapshot loaded."
            } else {
                "Start an SSH session to inspect remote Docker."
            }))
            .into_any_element();
    }
    if !docker_available {
        return div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(empty_panel(
                "Docker is not installed or the daemon is not reachable.",
            ))
            .into_any_element();
    }
    if filtered_containers.is_empty() {
        return div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
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

    // Tauri-like virtual list: fixed row slot, overscan window, spacer padding + wheel.
    const DOCKER_ROW_PX: f32 = 52.; // 48px row + ~4px gap
    const DOCKER_VIEWPORT_ROWS: usize = 16;
    const DOCKER_OVERSCAN: usize = 6;
    let total = containers.len();
    let window_capacity = DOCKER_VIEWPORT_ROWS + DOCKER_OVERSCAN * 2;
    let max_offset = total.saturating_sub(DOCKER_VIEWPORT_ROWS.min(total));
    let scroll_row = list_offset.min(max_offset);
    let window_start = scroll_row.saturating_sub(DOCKER_OVERSCAN);
    let window_end = (window_start + window_capacity).min(total);
    let visible = containers
        .get(window_start..window_end)
        .unwrap_or(&[])
        .to_vec();
    let pad_top = (window_start as f32) * DOCKER_ROW_PX;
    let pad_bottom = ((total.saturating_sub(window_end)) as f32) * DOCKER_ROW_PX;

    let mut rows = div().flex().flex_col().gap_1().p_2();
    if pad_top > 0. {
        rows = rows.child(div().h(px(pad_top)).w_full().flex_none());
    }
    for container in visible {
        let menu_open = open_menu_id == Some(container.id.as_str());
        rows = rows.child(docker_container_row(container, menu_open, cx));
    }
    if pad_bottom > 0. {
        rows = rows.child(div().h(px(pad_bottom)).w_full().flex_none());
    }
    if total > DOCKER_VIEWPORT_ROWS {
        rows = rows.child(
            div()
                .mt_1()
                .px_2()
                .py_1()
                .rounded_md()
                .border_1()
                .border_color(rgb(0x21262d))
                .bg(rgb(0x0d1117))
                .text_size(px(10.))
                .text_color(rgb(0x6e7681))
                .child(format!(
                    "Rows {window_start}-{window_end}/{total} · scroll or refine search"
                )),
        );
    }

    div()
        .id(SharedString::from("docker-containers-scroll"))
        .size_full()
        .overflow_hidden()
        .flex()
        .flex_col()
        .on_scroll_wheel(cx.listener(move |this, event: &ScrollWheelEvent, _, cx| {
            let max_offset = total.saturating_sub(DOCKER_VIEWPORT_ROWS.min(total));
            if max_offset == 0 {
                return;
            }
            let delta_rows = match event.delta {
                ScrollDelta::Lines(delta) => delta.y,
                ScrollDelta::Pixels(delta) => f32::from(delta.y) / DOCKER_ROW_PX,
            };
            let next = (this.docker_list_offset as f32 - delta_rows)
                .round()
                .clamp(0., max_offset as f32) as usize;
            if next != this.docker_list_offset {
                this.docker_list_offset = next;
                cx.stop_propagation();
                cx.notify();
            }
        }))
        .child(rows)
        .into_any_element()
}

fn docker_container_row(
    container: DockerContainer,
    menu_open: bool,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let container_id = container.id.clone();
    let details_id = container.id.clone();
    let menu_id = container.id.clone();
    let state = container.state.clone();
    let running = state.trim().eq_ignore_ascii_case("running");
    let accent = docker_state_border_color(&state);
    let short = compact_id(&container.id);

    div()
        .id(SharedString::from(format!("docker-container-{short}")))
        .relative()
        .h(px(48.))
        .rounded_md()
        .border_1()
        .border_color(rgb(0x30363d))
        // Left accent bar painted as absolute child.
        .bg(rgb(0x12171f))
        .hover(|this| this.bg(rgb(0x18202b)))
        .cursor_pointer()
        .overflow_hidden()
        .child(
            // Left state accent
            div()
                .absolute()
                .left_0()
                .top_0()
                .bottom_0()
                .w(px(3.))
                .bg(accent),
        )
        .child(
            div()
                .size_full()
                .px_3()
                .py_2()
                .pl(px(12.))
                .pr(px(36.))
                .flex()
                .flex_col()
                .justify_center()
                .gap(px(2.))
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
                                .text_color(rgb(0xc9d1d9))
                                .overflow_hidden()
                                .child(truncate_preview(&container.name, 40)),
                        )
                        .child(status_pill(
                            docker_state_label(&container.state),
                            docker_state_color(&container.state),
                            rgb(0x17233a),
                        )),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .font_family("JetBrains Mono")
                        .text_size(px(10.))
                        .text_color(rgb(0x6e7681))
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .overflow_hidden()
                                .child(truncate_preview(&container.image, 48)),
                        )
                        .child(div().flex_none().child(short.clone())),
                ),
        )
        .on_click(cx.listener(move |this, _, window, cx| {
            this.docker_container_menu_id = None;
            this.load_docker_details(details_id.clone(), window, cx);
        }))
        .child(
            div()
                .absolute()
                .top(px(8.))
                .right(px(6.))
                .child(
                    div()
                        .relative()
                        .child(icon_button(
                            format!("docker-menu-toggle-{short}"),
                            "⋮",
                            cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                if this.docker_container_menu_id.as_deref() == Some(menu_id.as_str())
                                {
                                    this.docker_container_menu_id = None;
                                } else {
                                    this.docker_container_menu_id = Some(menu_id.clone());
                                }
                                cx.notify();
                            }),
                        ))
                        .when(menu_open, |this| {
                            this.child(docker_container_action_menu(
                                container_id.clone(),
                                container.name.clone(),
                                running,
                                cx,
                            ))
                        }),
                ),
        )
}

fn docker_container_action_menu(
    container_id: String,
    container_name: String,
    running: bool,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let short = compact_id(&container_id);
    let logs_id = container_id.clone();
    let enter_id = container_id.clone();
    let start_id = container_id.clone();
    let stop_id = container_id.clone();
    let restart_id = container_id.clone();
    let kill_id = container_id.clone();
    let remove_id = container_id.clone();
    let kill_name = container_name.clone();
    let remove_name = container_name;

    div()
        .id(SharedString::from(format!("docker-menu-{short}")))
        .absolute()
        .top(px(28.))
        .right_0()
        .w(px(148.))
        .rounded_md()
        .border_1()
        .border_color(rgb(0x30363d))
        .bg(rgb(0x161b22))
        .shadow_lg()
        .py_1()
        .flex()
        .flex_col()
        .on_mouse_down(gpui::MouseButton::Left, |_, _, _| {})
        .child(docker_menu_item(
            format!("docker-menu-logs-{short}"),
            "Logs",
            false,
            cx.listener(move |this, _, window, cx| {
                this.docker_container_menu_id = None;
                this.load_docker_logs(logs_id.clone(), window, cx);
            }),
        ))
        .child(docker_menu_item(
            format!("docker-menu-enter-{short}"),
            "Enter",
            !running,
            cx.listener(move |this, _, _, cx| {
                this.docker_container_menu_id = None;
                this.enter_docker_container_terminal(enter_id.clone(), cx);
            }),
        ))
        .child(docker_menu_separator())
        .child(docker_menu_item(
            format!("docker-menu-start-{short}"),
            "Start",
            running,
            cx.listener(move |this, _, window, cx| {
                this.docker_container_menu_id = None;
                this.docker_container_action(start_id.clone(), "start", window, cx);
            }),
        ))
        .child(docker_menu_item(
            format!("docker-menu-stop-{short}"),
            "Stop",
            !running,
            cx.listener(move |this, _, window, cx| {
                this.docker_container_menu_id = None;
                this.docker_container_action(stop_id.clone(), "stop", window, cx);
            }),
        ))
        .child(docker_menu_item(
            format!("docker-menu-restart-{short}"),
            "Restart",
            false,
            cx.listener(move |this, _, window, cx| {
                this.docker_container_menu_id = None;
                this.docker_container_action(restart_id.clone(), "restart", window, cx);
            }),
        ))
        .child(docker_menu_separator())
        .child(docker_menu_item(
            format!("docker-menu-kill-{short}"),
            "Kill",
            !running,
            cx.listener(move |this, _, _, cx| {
                this.docker_container_menu_id = None;
                let target = if kill_name.trim().is_empty() {
                    compact_id(&kill_id)
                } else {
                    kill_name.clone()
                };
                this.request_docker_confirm(
                    DockerConfirmState {
                        title: format!("Kill container {target}"),
                        detail: format!("docker kill {}", compact_id(&kill_id)),
                        action: DockerConfirmAction::ContainerAction {
                            container_id: kill_id.clone(),
                            action: "kill",
                        },
                    },
                    cx,
                );
            }),
        ))
        .child(docker_menu_item(
            format!("docker-menu-remove-{short}"),
            "Remove",
            false,
            cx.listener(move |this, _, _, cx| {
                this.docker_container_menu_id = None;
                let target = if remove_name.trim().is_empty() {
                    compact_id(&remove_id)
                } else {
                    remove_name.clone()
                };
                this.request_docker_confirm(
                    DockerConfirmState {
                        title: format!("Remove container {target}"),
                        detail: format!("docker rm {}", compact_id(&remove_id)),
                        action: DockerConfirmAction::ContainerAction {
                            container_id: remove_id.clone(),
                            action: "remove",
                        },
                    },
                    cx,
                );
            }),
        ))
}

fn docker_menu_item(
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

fn docker_menu_separator() -> impl IntoElement {
    div().h(px(1.)).mx_2().my_1().bg(rgb(0x30363d))
}

fn docker_state_border_color(state: &str) -> gpui::Hsla {
    match state.trim().to_ascii_lowercase().as_str() {
        "running" => rgb(0x22c55e).into(),
        "restarting" | "paused" => rgb(0xf59e0b).into(),
        "exited" | "dead" => rgb(0xef4444).into(),
        "created" => rgb(0x3b82f6).into(),
        _ => rgb(0x6e7681).into(),
    }
}

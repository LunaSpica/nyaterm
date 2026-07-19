use super::*;
use gpui::SharedString;

pub(in crate::features::pages::remote) fn docker_containers_panel(
    palette: crate::theme::ThemePalette,
    menu_bg: gpui::Rgba,
    has_snapshot: bool,
    has_session: bool,
    docker_available: bool,
    filtered_containers: &[DockerContainer],
    query_empty: bool,
    open_menu_id: Option<&str>,
    list_offset: usize,
    labels: DockerLabels,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    // Tauri Docker containers tab: dense ~66px rows, left accent, ⋮ action menu.
    if !has_snapshot {
        return div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(empty_panel(
                if has_session {
                    labels.error
                } else {
                    labels.no_session
                },
                palette,
            ))
            .into_any_element();
    }
    if !docker_available {
        return div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(empty_panel(labels.unavailable, palette))
            .into_any_element();
    }
    if filtered_containers.is_empty() {
        return div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(empty_panel(
                if query_empty {
                    labels.no_containers
                } else {
                    labels.no_matches
                },
                palette,
            ))
            .into_any_element();
    }

    let mut containers = filtered_containers.to_vec();
    containers.sort_by(|left, right| {
        docker_state_rank(&left.state)
            .cmp(&docker_state_rank(&right.state))
            .then(left.name.cmp(&right.name))
    });

    // Tauri-like virtual list: fixed row slot, overscan window, spacer padding + wheel.
    const DOCKER_ROW_PX: f32 = 68.; // 66px Tauri row + gap
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
        rows = rows.child(docker_container_row(
            palette, menu_bg, container, menu_open, labels, cx,
        ));
    }
    if pad_bottom > 0. {
        rows = rows.child(div().h(px(pad_bottom)).w_full().flex_none());
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
    palette: crate::theme::ThemePalette,
    menu_bg: gpui::Rgba,
    container: DockerContainer,
    menu_open: bool,
    labels: DockerLabels,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let container_id = container.id.clone();
    let details_id = container.id.clone();
    let menu_id = container.id.clone();
    let state = container.state.clone();
    let running = state.trim().eq_ignore_ascii_case("running");
    let accent = docker_state_border_color(palette, &state);
    let short = compact_id(&container.id);

    div()
        .id(SharedString::from(format!("docker-container-{short}")))
        .relative()
        .h(px(66.))
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        // Left accent bar painted as absolute child.
        .bg(rgb(palette.section_header))
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
                                .text_color(rgb(palette.text))
                                .overflow_hidden()
                                .child(truncate_preview(&container.name, 40)),
                        )
                        .child(status_pill(
                            labels.state_label(&container.state),
                            docker_state_color(palette, &container.state),
                            rgb(0x17233a),
                        )),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .font_family(crate::features::gpui_code_font_family())
                        .text_size(px(10.))
                        .text_color(rgb(palette.text_dimmed))
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .overflow_hidden()
                                .child(truncate_preview(&container.image, 48)),
                        )
                        .child(div().flex_none().child(short.clone())),
                )
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .font_family(crate::features::gpui_code_font_family())
                        .text_size(px(10.))
                        .text_color(rgb(palette.border))
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .overflow_hidden()
                                .child(truncate_preview(
                                    if container.ports.trim().is_empty() {
                                        container.status.as_str()
                                    } else {
                                        container.ports.as_str()
                                    },
                                    56,
                                )),
                        )
                        .when(!container.created_at.trim().is_empty(), |this| {
                            this.child(
                                div()
                                    .flex_none()
                                    .child(truncate_preview(&container.created_at, 18)),
                            )
                        }),
                ),
        )
        .on_click(cx.listener(move |this, _, window, cx| {
            this.docker_container_menu_id = None;
            this.load_docker_details(details_id.clone(), window, cx);
        }))
        .child(
            div().absolute().top(px(8.)).right(px(6.)).child(
                div()
                    .relative()
                    .child(svg_icon_button(
                        format!("docker-menu-toggle-{short}"),
                        "icons/session/more.svg",
                        14.,
                        palette,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            if this.docker_container_menu_id.as_deref() == Some(menu_id.as_str()) {
                                this.docker_container_menu_id = None;
                            } else {
                                this.docker_container_menu_id = Some(menu_id.clone());
                            }
                            cx.notify();
                        }),
                    ))
                    .when(menu_open, |this| {
                        this.child(docker_container_action_menu(
                            palette,
                            menu_bg,
                            container_id.clone(),
                            container.name.clone(),
                            running,
                            labels,
                            cx,
                        ))
                    }),
            ),
        )
}

fn docker_container_action_menu(
    palette: crate::theme::ThemePalette,
    menu_bg: gpui::Rgba,
    container_id: String,
    container_name: String,
    running: bool,
    labels: DockerLabels,
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
        .border_color(rgb(palette.border))
        .bg(menu_bg)
        .shadow_lg()
        .py_1()
        .flex()
        .flex_col()
        .on_mouse_down(gpui::MouseButton::Left, |_, _, _| {})
        .child(docker_menu_item(
            palette,
            format!("docker-menu-logs-{short}"),
            labels.logs,
            false,
            cx.listener(move |this, _, _, cx| {
                this.docker_container_menu_id = None;
                this.send_docker_container_logs_to_terminal(logs_id.clone(), cx);
            }),
        ))
        .child(docker_menu_item(
            palette,
            format!("docker-menu-enter-{short}"),
            labels.enter,
            !running,
            cx.listener(move |this, _, _, cx| {
                this.docker_container_menu_id = None;
                this.enter_docker_container_terminal(enter_id.clone(), cx);
            }),
        ))
        .child(docker_menu_separator(palette))
        .child(docker_menu_item(
            palette,
            format!("docker-menu-start-{short}"),
            labels.start,
            running,
            cx.listener(move |this, _, window, cx| {
                this.docker_container_menu_id = None;
                this.docker_container_action(start_id.clone(), "start", window, cx);
            }),
        ))
        .child(docker_menu_item(
            palette,
            format!("docker-menu-stop-{short}"),
            labels.stop,
            !running,
            cx.listener(move |this, _, window, cx| {
                this.docker_container_menu_id = None;
                this.docker_container_action(stop_id.clone(), "stop", window, cx);
            }),
        ))
        .child(docker_menu_item(
            palette,
            format!("docker-menu-restart-{short}"),
            labels.restart,
            false,
            cx.listener(move |this, _, window, cx| {
                this.docker_container_menu_id = None;
                this.docker_container_action(restart_id.clone(), "restart", window, cx);
            }),
        ))
        .child(docker_menu_separator(palette))
        .child(docker_menu_item(
            palette,
            format!("docker-menu-kill-{short}"),
            labels.kill,
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
                        title: labels.confirm_action_title.to_string(),
                        detail: labels.confirm_description(labels.kill, &target),
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
            palette,
            format!("docker-menu-remove-{short}"),
            labels.delete,
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
                        title: labels.confirm_action_title.to_string(),
                        detail: labels.confirm_description(labels.delete, &target),
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

fn docker_menu_separator(palette: crate::theme::ThemePalette) -> impl IntoElement {
    div().h(px(1.)).mx_2().my_1().bg(rgb(palette.border))
}

fn docker_state_border_color(palette: crate::theme::ThemePalette, state: &str) -> gpui::Hsla {
    match state.trim().to_ascii_lowercase().as_str() {
        "running" => rgb(0x22c55e).into(),
        "restarting" | "paused" => rgb(0xf59e0b).into(),
        "exited" | "dead" => rgb(0xef4444).into(),
        "created" => rgb(0x3b82f6).into(),
        _ => rgb(palette.text_dimmed).into(),
    }
}

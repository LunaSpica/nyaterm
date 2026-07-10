use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn docker_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let can_run = self.active_ssh_config.is_some() && !self.docker_pending;
        let overview = self.docker_overview.clone().unwrap_or_default();
        let active_tab = if self.docker_tab == DockerTab::Compose && !overview.compose_available {
            DockerTab::Containers
        } else {
            self.docker_tab
        };
        let query = self.docker_search_draft.trim().to_ascii_lowercase();
        let filtered_containers = overview
            .containers
            .iter()
            .filter(|container| docker_container_matches(container, &query))
            .cloned()
            .collect::<Vec<_>>();
        let filtered_images = overview
            .images
            .iter()
            .filter(|image| docker_image_matches(image, &query))
            .cloned()
            .collect::<Vec<_>>();
        let filtered_volumes = overview
            .volumes
            .iter()
            .filter(|volume| docker_volume_matches(volume, &query))
            .cloned()
            .collect::<Vec<_>>();
        let filtered_networks = overview
            .networks
            .iter()
            .filter(|network| docker_network_matches(network, &query))
            .cloned()
            .collect::<Vec<_>>();
        let filtered_compose_projects = overview
            .compose_projects
            .iter()
            .filter(|project| docker_compose_project_matches(project, &query))
            .cloned()
            .collect::<Vec<_>>();

        let logs = if self.docker_logs.trim().is_empty() {
            "No logs loaded.".to_string()
        } else {
            self.docker_logs.clone()
        };
        let logs_title = self
            .docker_logs_container_id
            .as_deref()
            .map(|id| format!("Recent Logs · {}", compact_id(id)))
            .unwrap_or_else(|| "Recent Logs".to_string());
        let docker_content = match active_tab {
            DockerTab::Containers => docker_containers_panel(
                self.docker_overview.is_some(),
                self.active_ssh_config.is_some(),
                overview.available,
                &filtered_containers,
                query.is_empty(),
                cx,
            )
            .into_any_element(),
            DockerTab::Images => docker_images_panel(&filtered_images, cx).into_any_element(),
            DockerTab::Volumes => docker_volumes_panel(&filtered_volumes, cx).into_any_element(),
            DockerTab::Networks => docker_networks_panel(&filtered_networks, cx).into_any_element(),
            DockerTab::Compose => docker_compose_panel(
                &filtered_compose_projects,
                &self.docker_compose_expanded,
                &self.docker_compose_services,
                &self.docker_compose_service_errors,
                cx,
            )
            .into_any_element(),
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_5()
            .gap_4()
            .child(section_header(
                "Docker",
                "Native SSH exec Docker manager for the active remote session.",
            ))
            .child(
                div()
                    .grid()
                    .grid_cols(6)
                    .gap_3()
                    .child(metric(
                        "SSH",
                        if self.active_ssh_config.is_some() {
                            "ready".to_string()
                        } else {
                            "none".to_string()
                        },
                    ))
                    .child(metric(
                        "Docker",
                        if overview.available {
                            "available".to_string()
                        } else {
                            "unknown".to_string()
                        },
                    ))
                    .child(metric(
                        "Version",
                        if overview.version.trim().is_empty() {
                            "n/a".to_string()
                        } else {
                            overview.version.clone()
                        },
                    ))
                    .child(metric("Containers", overview.containers.len().to_string()))
                    .child(metric("Images", overview.images.len().to_string()))
                    .child(metric(
                        "Compose",
                        if overview.compose_available {
                            overview.compose_projects.len().to_string()
                        } else {
                            "off".to_string()
                        },
                    )),
            )
            .child(
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
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xe5edf7))
                                    .child(self.docker_status.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .when(!can_run, |this| this.opacity(0.45))
                                    .child(small_button(
                                        "docker-refresh",
                                        "Refresh",
                                        cx.listener(|this, _, window, cx| {
                                            this.refresh_docker(window, cx);
                                        }),
                                    ))
                                    .child(small_button(
                                        "docker-prune",
                                        "Prune",
                                        cx.listener(|this, _, _, cx| {
                                            this.prune_docker_system(cx);
                                        }),
                                    )),
                            ),
                    ),
            )
            .when_some(self.docker_confirm.clone(), |this, confirm| {
                this.child(docker_confirm_panel(confirm, cx))
            })
            .child(docker_tab_bar(active_tab, &overview, cx))
            .child(
                transfer_input(
                    "docker-search-input",
                    "Search",
                    self.docker_search_draft.clone(),
                    true,
                )
                .track_focus(&self.docker_search_focus)
                .on_click(cx.listener(|this, _, window, cx| {
                    window.focus(&this.docker_search_focus);
                    cx.notify();
                }))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.handle_docker_search_key_down(event, cx);
                })),
            )
            .child(docker_content)
            .child(docker_details_panel(
                self.docker_details_container_id.clone(),
                self.docker_details.clone(),
                self.docker_details_container_id
                    .as_deref()
                    .and_then(|id| {
                        overview
                            .containers
                            .iter()
                            .find(|container| container.id == id)
                    })
                    .cloned(),
                cx,
            ))
            .child(
                div()
                    .grid()
                    .grid_cols(2)
                    .gap_3()
                    .child(
                        div()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x2a3140))
                            .bg(rgb(0x151923))
                            .p_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("Resources"),
                            )
                            .child(capability_line(
                                "Volumes",
                                overview.volumes.len().to_string(),
                            ))
                            .child(capability_line(
                                "Networks",
                                overview.networks.len().to_string(),
                            ))
                            .child(capability_line(
                                "Compose Projects",
                                overview.compose_projects.len().to_string(),
                            )),
                    )
                    .child(
                        div()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x2a3140))
                            .bg(rgb(0x151923))
                            .p_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child(logs_title),
                            )
                            .child(
                                div()
                                    .mt_3()
                                    .max_h(px(180.))
                                    .overflow_hidden()
                                    .font_family("JetBrains Mono")
                                    .text_xs()
                                    .line_height(px(18.))
                                    .text_color(rgb(0xaeb7c8))
                                    .child(logs),
                            ),
                    ),
            )
    }
}

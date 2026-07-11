use super::*;
use gpui::{SharedString, prelude::*};

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
        // Keep virtual-list offset valid after search/filter changes.
        {
            const DOCKER_VIEWPORT_ROWS: usize = 16;
            let total = filtered_containers.len();
            let max_offset = total.saturating_sub(DOCKER_VIEWPORT_ROWS.min(total));
            if self.docker_list_offset > max_offset {
                self.docker_list_offset = max_offset;
            }
        }

        let docker_content = match active_tab {
            DockerTab::Containers => docker_containers_panel(
                self.docker_overview.is_some(),
                self.active_ssh_config.is_some(),
                overview.available,
                &filtered_containers,
                query.is_empty(),
                self.docker_container_menu_id.as_deref(),
                self.docker_list_offset,
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
                self.docker_compose_menu_id.as_deref(),
                cx,
            )
            .into_any_element(),
        };

        // Tauri DockerManager shell: dense toolbar (search+actions) + tabs + flex list body.
        // Shared PanelHeader already shows title/meta; avoid page-like section headers.
        let status_short = truncate_preview(&self.docker_status, 36);
        div()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(rgb(0x161b22))
            .child(
                div()
                    .h(px(36.))
                    .flex_none()
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(0x30363d))
                    .bg(rgb(0x12171f))
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .child(
                                transfer_input(
                                    "docker-search-input",
                                    "Search containers…",
                                    self.docker_search_draft.clone(),
                                    true,
                                )
                                .h(px(28.))
                                .track_focus(&self.docker_search_focus)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    window.focus(&this.docker_search_focus);
                                    cx.notify();
                                }))
                                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                    cx.stop_propagation();
                                    this.handle_docker_search_key_down(event, cx);
                                })),
                            ),
                    )
                    .child(
                        div()
                            .when(!can_run, |this| this.opacity(0.45))
                            .flex()
                            .items_center()
                            .gap_0()
                            .child(compact_remote_svg_button(
                                "docker-refresh",
                                "icons/fe/refresh.svg",
                                cx.listener(|this, _, window, cx| {
                                    this.refresh_docker(window, cx);
                                }),
                            ))
                            .child(compact_remote_svg_button(
                                "docker-prune",
                                "icons/fe/delete.svg",
                                cx.listener(|this, _, _, cx| {
                                    this.prune_docker_system(cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .ml_1()
                            .text_size(px(10.))
                            .text_color(rgb(0x6e7681))
                            .child(status_short),
                    ),
            )
            .when_some(self.docker_confirm.clone(), |this, confirm| {
                this.child(docker_confirm_panel(confirm, cx))
            })
            .child(docker_tab_bar(active_tab, &overview, cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(docker_content),
            )
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
            .when(!logs.trim().is_empty() && logs != "No logs loaded.", |this| {
                this.child(
                    div()
                        .flex_none()
                        .max_h(px(140.))
                        .border_t_1()
                        .border_color(rgb(0x30363d))
                        .bg(rgb(0x0d1117))
                        .px_2()
                        .py_1()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .text_size(px(10.))
                                .font_weight(FontWeight(700.))
                                .text_color(rgb(0x8b949e))
                                .child(logs_title),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_h_0()
                                .id(SharedString::from("docker-logs-scroll"))
                                .overflow_scroll()
                                .scrollbar_width(px(6.))
                                .font_family("JetBrains Mono")
                                .text_size(px(10.))
                                .line_height(px(16.))
                                .text_color(rgb(0xaeb7c8))
                                .child(logs),
                        ),
                )
            })
    }
}

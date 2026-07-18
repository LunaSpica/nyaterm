use super::*;

impl NyaTermApp {
    pub(in crate::features) fn docker_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
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

        // Keep virtual-list offsets valid after search/filter/tab changes.
        {
            const DOCKER_VIEWPORT_ROWS: usize = 16;
            let total = filtered_containers.len();
            let max_offset = total.saturating_sub(DOCKER_VIEWPORT_ROWS.min(total));
            if self.docker_list_offset > max_offset {
                self.docker_list_offset = max_offset;
            }
        }
        {
            const DOCKER_RESOURCE_VIEWPORT_ROWS: usize = 14;
            let total = match active_tab {
                DockerTab::Images => filtered_images.len(),
                DockerTab::Volumes => filtered_volumes.len(),
                DockerTab::Networks => filtered_networks.len(),
                _ => 0,
            };
            let max_offset = total.saturating_sub(DOCKER_RESOURCE_VIEWPORT_ROWS.min(total));
            if self.docker_resource_list_offset > max_offset {
                self.docker_resource_list_offset = max_offset;
            }
        }

        let palette = self.theme_palette();
        let docker_content = match active_tab {
            DockerTab::Containers => docker_containers_panel(
                palette,
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
            DockerTab::Images => docker_images_panel(
                palette,
                &filtered_images,
                self.docker_resource_list_offset,
                cx,
            )
            .into_any_element(),
            DockerTab::Volumes => docker_volumes_panel(
                palette,
                &filtered_volumes,
                self.docker_resource_list_offset,
                cx,
            )
            .into_any_element(),
            DockerTab::Networks => docker_networks_panel(
                palette,
                &filtered_networks,
                self.docker_resource_list_offset,
                cx,
            )
            .into_any_element(),
            DockerTab::Compose => docker_compose_panel(
                palette,
                &filtered_compose_projects,
                &self.docker_compose_expanded,
                &self.docker_compose_services,
                &self.docker_compose_service_errors,
                self.docker_compose_menu_id.as_deref(),
                cx,
            )
            .into_any_element(),
        };

        // Tauri DockerManager shell: header actions + dense search + tabs + flex list body.
        // Shared PanelHeader already shows title/meta; avoid page-like section headers.
        div()
            .flex()
            .flex_col()
            .size_full()
            .relative()
            .overflow_hidden()
            .bg(rgb(palette.surface))
            .when(
                self.docker_overview
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.available),
                |this| {
                    this.child(docker_overview_strip(
                        palette,
                        &overview,
                        [
                            self.tr("dockerManager.running").to_string(),
                            self.tr("dockerManager.stopped").to_string(),
                            self.tr("dockerManager.images").to_string(),
                        ],
                    ))
                },
            )
            .child(
                div()
                    .h(px(36.))
                    .flex_none()
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.section_header))
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div().flex_1().min_w_0().child(
                            transfer_input(
                                "docker-search-input",
                                "Search containers…",
                                self.docker_search_draft.clone(),
                                true,
                                self.theme_palette(),
                            )
                            .h(px(28.))
                            .track_focus(&self.docker_search_focus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                window.focus(&this.docker_search_focus);
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(
                                |this, event: &KeyDownEvent, _, cx| {
                                    cx.stop_propagation();
                                    this.handle_docker_search_key_down(event, cx);
                                },
                            )),
                        ),
                    ),
            )
            .child(docker_tab_bar(
                palette,
                active_tab,
                &overview,
                [
                    self.tr("dockerManager.containers").to_string(),
                    self.tr("dockerManager.images").to_string(),
                    self.tr("dockerManager.volumes").to_string(),
                    self.tr("dockerManager.networks").to_string(),
                    self.tr("dockerManager.compose").to_string(),
                ],
                self.tr("common.more").to_string(),
                self.right_panel_width,
                self.docker_tab_menu_open,
                cx,
            ))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(docker_content),
            )
            .when_some(
                self.docker_details_container_id.clone(),
                |this, container_id| {
                    this.child(docker_details_panel(
                        palette,
                        Some(container_id.clone()),
                        self.docker_details.clone(),
                        overview
                            .containers
                            .iter()
                            .find(|container| container.id == container_id)
                            .cloned(),
                        cx,
                    ))
                },
            )
            .when_some(self.docker_confirm.clone(), |this, confirm| {
                this.child(docker_confirm_panel(palette, confirm, cx))
            })
    }
}

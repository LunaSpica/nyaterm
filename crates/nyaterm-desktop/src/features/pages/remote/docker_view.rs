use gpui::{Context, IntoElement, div, prelude::*, px, rgb};

use crate::features::{NyaTermApp, TextInputSetup};
use crate::models::DockerTab;
use crate::widgets::empty_panel;

use super::docker::{
    DockerLabels, docker_compose_panel, docker_compose_project_matches, docker_confirm_panel,
    docker_container_matches, docker_containers_panel, docker_details_panel, docker_image_matches,
    docker_images_panel, docker_network_matches, docker_networks_panel, docker_overview_strip,
    docker_tab_bar, docker_volume_matches, docker_volumes_panel,
};

impl NyaTermApp {
    pub(in crate::features) fn docker_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let labels = DockerLabels {
            search: self.tr("dockerManager.search"),
            no_session: self.tr("dockerManager.noSession"),
            error: self.tr("dockerManager.error"),
            unavailable: self.tr("dockerManager.unavailable"),
            no_matches: self.tr("dockerManager.noMatches"),
            logs: self.tr("dockerManager.logs"),
            enter: self.tr("dockerManager.enter"),
            start: self.tr("dockerManager.start"),
            stop: self.tr("dockerManager.stop"),
            restart: self.tr("dockerManager.restart"),
            kill: self.tr("dockerManager.kill"),
            delete: self.tr("common.delete"),
            confirm_action_title: self.tr("dockerManager.confirmActionTitle"),
            confirm_action_desc: self.tr("dockerManager.confirmActionDesc"),
            networks: self.tr("dockerManager.networks"),
            remove_image: self.tr("dockerManager.removeImage"),
            remove_volume: self.tr("dockerManager.removeVolume"),
            remove_network: self.tr("dockerManager.removeNetwork"),
            volume_driver: self.tr("dockerManager.volumeDriver"),
            up: self.tr("dockerManager.up"),
            down: self.tr("dockerManager.down"),
            loading_services: self.tr("dockerManager.loadingServices"),
            service_load_failed: self.tr("dockerManager.serviceLoadFailed"),
            no_services: self.tr("dockerManager.noServices"),
            no_containers: self.tr("dockerManager.noContainers"),
            not_created: self.tr("dockerManager.notCreated"),
            retry: self.tr("common.retry"),
            loading: self.tr("common.loading"),
            container_details: self.tr("dockerManager.containerDetails"),
            identity: self.tr("dockerManager.identity"),
            container_name: self.tr("dockerManager.containerName"),
            container_id: self.tr("dockerManager.containerId"),
            image: self.tr("dockerManager.image"),
            status: self.tr("dockerManager.status"),
            created_at: self.tr("dockerManager.createdAt"),
            size: self.tr("dockerManager.size"),
            started_at: self.tr("dockerManager.startedAt"),
            finished_at: self.tr("dockerManager.finishedAt"),
            restart_count: self.tr("dockerManager.restartCount"),
            entrypoint: self.tr("dockerManager.entrypoint"),
            command: self.tr("dockerManager.command"),
            networking: self.tr("dockerManager.networking"),
            ports: self.tr("dockerManager.ports"),
            io: self.tr("dockerManager.io"),
            net_io: self.tr("dockerManager.netIo"),
            block_io: self.tr("dockerManager.blockIo"),
            mounts: self.tr("dockerManager.mounts"),
            cpu: self.tr("dockerManager.cpu"),
            memory: self.tr("dockerManager.memory"),
            pids: self.tr("dockerManager.pids"),
            copy: self.tr("common.copyToClipboard"),
            refresh: self.tr("common.refresh"),
            close: self.tr("common.close"),
            cancel: self.tr("common.cancel"),
            confirm: self.tr("common.confirm"),
            state_created: self.tr("dockerManager.stateLabels.created"),
            state_dead: self.tr("dockerManager.stateLabels.dead"),
            state_exited: self.tr("dockerManager.stateLabels.exited"),
            state_paused: self.tr("dockerManager.stateLabels.paused"),
            state_removing: self.tr("dockerManager.stateLabels.removing"),
            state_restarting: self.tr("dockerManager.stateLabels.restarting"),
            state_running: self.tr("dockerManager.stateLabels.running"),
            state_unknown: self.tr("dockerManager.stateLabels.unknown"),
        };
        // Built before the view, which reads `self` throughout: creating the
        // box needs it mutably.
        let docker_search_input = self
            .text_input_box(
                "remote.docker.filter",
                &self.remote_ops.docker.search_draft.clone(),
                TextInputSetup::placeholder(labels.search),
                cx,
            )
            .into_any_element();
        if self.active_ssh_config.is_none() {
            return div()
                .size_full()
                .bg(self.shell_transparent_color(palette.surface))
                .child(empty_panel(labels.no_session, palette));
        }
        let Some(overview) = self.remote_ops.docker.overview.clone() else {
            let message = if self.remote_ops.docker.pending
                || !self.remote_ops.docker.status.contains("failed")
            {
                labels.loading
            } else {
                labels.error
            };
            return div()
                .size_full()
                .bg(self.shell_transparent_color(palette.surface))
                .child(empty_panel(message, palette));
        };
        if !overview.available {
            return div()
                .size_full()
                .bg(self.shell_transparent_color(palette.surface))
                .child(empty_panel(labels.unavailable, palette));
        }
        let active_tab =
            if self.remote_ops.docker.tab == DockerTab::Compose && !overview.compose_available {
                DockerTab::Containers
            } else {
                self.remote_ops.docker.tab
            };
        let query = self
            .remote_ops
            .docker
            .search_draft
            .trim()
            .to_ascii_lowercase();
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
            if self.remote_ops.docker.list_offset > max_offset {
                self.remote_ops.docker.list_offset = max_offset;
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
            if self.remote_ops.docker.resource_list_offset > max_offset {
                self.remote_ops.docker.resource_list_offset = max_offset;
            }
        }

        let menu_bg = self.shell_surface_color(palette.surface);
        let dialog_bg = self.shell_surface_color(palette.bg);
        let docker_content = match active_tab {
            DockerTab::Containers => docker_containers_panel(
                palette,
                menu_bg,
                self.remote_ops.docker.overview.is_some(),
                self.active_ssh_config.is_some(),
                overview.available,
                &filtered_containers,
                query.is_empty(),
                self.remote_ops.docker.container_menu_id.as_deref(),
                self.remote_ops.docker.list_offset,
                labels,
                cx,
            )
            .into_any_element(),
            DockerTab::Images => docker_images_panel(
                palette,
                &filtered_images,
                self.remote_ops.docker.resource_list_offset,
                labels,
                cx,
            )
            .into_any_element(),
            DockerTab::Volumes => docker_volumes_panel(
                palette,
                &filtered_volumes,
                self.remote_ops.docker.resource_list_offset,
                labels,
                cx,
            )
            .into_any_element(),
            DockerTab::Networks => docker_networks_panel(
                palette,
                &filtered_networks,
                self.remote_ops.docker.resource_list_offset,
                labels,
                cx,
            )
            .into_any_element(),
            DockerTab::Compose => docker_compose_panel(
                palette,
                menu_bg,
                &filtered_compose_projects,
                &self.remote_ops.docker.compose_expanded,
                &self.remote_ops.docker.compose_services,
                &self.remote_ops.docker.compose_service_errors,
                self.remote_ops.docker.compose_menu_id.as_deref(),
                labels,
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
            .p(px(10.))
            .gap(px(10.))
            .bg(self.shell_transparent_color(palette.surface))
            .when(
                self.remote_ops
                    .docker
                    .overview
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
                    .h(px(32.))
                    .flex_none()
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_transparent_color(palette.section_header))
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(div().flex_1().min_w_0().child(docker_search_input)),
            )
            .child(docker_tab_bar(
                palette,
                menu_bg,
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
                self.remote_ops.docker.tab_menu_open,
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
                self.remote_ops.docker.details_container_id.clone(),
                |this, container_id| {
                    this.child(docker_details_panel(
                        palette,
                        dialog_bg,
                        Some(container_id.clone()),
                        self.remote_ops.docker.details.clone(),
                        overview
                            .containers
                            .iter()
                            .find(|container| container.id == container_id)
                            .cloned(),
                        labels,
                        cx,
                    ))
                },
            )
            .when_some(self.remote_ops.docker.confirm.clone(), |this, confirm| {
                this.child(docker_confirm_panel(
                    palette, dialog_bg, confirm, labels, cx,
                ))
            })
    }
}

use crate::features::docker_state_label;

#[derive(Clone, Copy)]
pub(in crate::features::pages::remote) struct DockerLabels {
    pub search: &'static str,
    pub no_session: &'static str,
    pub error: &'static str,
    pub unavailable: &'static str,
    pub no_matches: &'static str,
    pub logs: &'static str,
    pub enter: &'static str,
    pub start: &'static str,
    pub stop: &'static str,
    pub restart: &'static str,
    pub kill: &'static str,
    pub delete: &'static str,
    pub confirm_action_title: &'static str,
    pub confirm_action_desc: &'static str,
    pub networks: &'static str,
    pub remove_image: &'static str,
    pub remove_volume: &'static str,
    pub remove_network: &'static str,
    pub volume_driver: &'static str,
    pub up: &'static str,
    pub down: &'static str,
    pub loading_services: &'static str,
    pub service_load_failed: &'static str,
    pub no_services: &'static str,
    pub no_containers: &'static str,
    pub not_created: &'static str,
    pub retry: &'static str,
    pub loading: &'static str,
    pub container_details: &'static str,
    pub identity: &'static str,
    pub container_name: &'static str,
    pub container_id: &'static str,
    pub image: &'static str,
    pub status: &'static str,
    pub created_at: &'static str,
    pub size: &'static str,
    pub started_at: &'static str,
    pub finished_at: &'static str,
    pub restart_count: &'static str,
    pub entrypoint: &'static str,
    pub command: &'static str,
    pub networking: &'static str,
    pub ports: &'static str,
    pub io: &'static str,
    pub net_io: &'static str,
    pub block_io: &'static str,
    pub mounts: &'static str,
    pub cpu: &'static str,
    pub memory: &'static str,
    pub pids: &'static str,
    pub copy: &'static str,
    pub refresh: &'static str,
    pub close: &'static str,
    pub cancel: &'static str,
    pub confirm: &'static str,
    pub state_created: &'static str,
    pub state_dead: &'static str,
    pub state_exited: &'static str,
    pub state_paused: &'static str,
    pub state_removing: &'static str,
    pub state_restarting: &'static str,
    pub state_running: &'static str,
    pub state_unknown: &'static str,
}

impl DockerLabels {
    pub fn confirm_description(self, action: &str, target: &str) -> String {
        self.confirm_action_desc
            .replace("{{action}}", action)
            .replace("{{target}}", target)
    }

    pub fn volume_driver_label(self, driver: &str) -> String {
        self.volume_driver.replace("{{driver}}", driver)
    }

    pub fn state_label(self, state: &str) -> &'static str {
        let normalized = state.trim().to_ascii_lowercase();
        let legacy_label = docker_state_label(state);
        if normalized == "running" || normalized == "up" || legacy_label == "running" {
            self.state_running
        } else if normalized == "exited" || normalized == "stopped" || normalized == "down" {
            self.state_exited
        } else if normalized == "created" || legacy_label == "created" {
            self.state_created
        } else if normalized == "dead" || legacy_label == "dead" {
            self.state_dead
        } else if normalized == "paused" || legacy_label == "paused" {
            self.state_paused
        } else if normalized == "removing" {
            self.state_removing
        } else if normalized == "restarting" || legacy_label == "restart" {
            self.state_restarting
        } else {
            self.state_unknown
        }
    }
}

mod compose;
mod containers;
mod controls;
mod details;
mod matchers;
mod resources;

pub(super) use compose::docker_compose_panel;
pub(super) use containers::docker_containers_panel;
pub(super) use controls::{docker_confirm_panel, docker_overview_strip, docker_tab_bar};
pub(super) use details::docker_details_panel;
pub(super) use matchers::{
    docker_compose_project_matches, docker_container_matches, docker_image_matches,
    docker_network_matches, docker_volume_matches,
};
pub(super) use resources::{docker_images_panel, docker_networks_panel, docker_volumes_panel};

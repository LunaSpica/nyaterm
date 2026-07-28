use std::{collections::HashSet, time::Instant};

use gpui::{Context, Window};
use nyaterm_transport::{DockerService, RemoteDockerOverview};

use crate::features::NyaTermApp;
use crate::features::formatting::{compact_id, docker_compose_project_key};
use crate::features::runtime_jobs::{DockerJobOutput, DockerJobResult, remote_job_event_matches};
use crate::models::{DockerConfirmAction, DockerConfirmState, NavItem};

use super::helpers::{
    DOCKER_SHELL_SELECTOR, docker_compose_terminal_base, docker_overview_status, shell_quote,
};

const DOCKER_EVENT_DRAIN_LIMIT: usize = 16;

impl NyaTermApp {
    pub(in crate::features) fn refresh_docker(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.session.active_ssh_config.clone() else {
            self.remote_ops.docker.status =
                "start an SSH session before inspecting Docker".to_string();
            self.terminal.view.status = self.remote_ops.docker.status.clone();
            cx.notify();
            return;
        };
        let Some(job_session_id) = self.session.active_id.clone() else {
            self.remote_ops.docker.status =
                "start an SSH session before inspecting Docker".to_string();
            cx.notify();
            return;
        };
        if self.remote_ops.docker.pending
            && self.remote_ops.docker.job_session_id.as_deref() == Some(job_session_id.as_str())
        {
            self.remote_ops.docker.status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        let job_id = self.begin_docker_job(job_session_id.clone());
        self.remote_ops.docker.last_refresh_at = Some(Instant::now());
        self.remote_ops.docker.status = "loading Docker overview".to_string();
        let tx = self.remote_ops.docker.tx.clone();
        std::thread::spawn(move || {
            let result = DockerService::new(config)
                .overview()
                .map(DockerJobOutput::Overview)
                .map_err(|error| error.to_string());
            let _ = tx.send(DockerJobResult {
                job_id,
                session_id: job_session_id,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn docker_container_action(
        &mut self,
        container_id: String,
        action: &'static str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.session.active_ssh_config.clone() else {
            self.remote_ops.docker.status =
                "start an SSH session before changing containers".to_string();
            self.terminal.view.status = self.remote_ops.docker.status.clone();
            cx.notify();
            return;
        };
        let Some(job_session_id) = self.session.active_id.clone() else {
            self.remote_ops.docker.status =
                "start an SSH session before changing containers".to_string();
            cx.notify();
            return;
        };
        if self.remote_ops.docker.pending
            && self.remote_ops.docker.job_session_id.as_deref() == Some(job_session_id.as_str())
        {
            self.remote_ops.docker.status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        let job_id = self.begin_docker_job(job_session_id.clone());
        self.remote_ops.docker.status = format!("Docker {action} {}", compact_id(&container_id));
        self.remote_ops.docker.details = None;
        self.remote_ops.docker.details_container_id = None;
        let tx = self.remote_ops.docker.tx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let service = DockerService::new(config);
                service.container_action(&container_id, action)?;
                let overview = service.overview()?;
                Ok(DockerJobOutput::RefreshedAfterAction {
                    label: format!("Docker {action} {}", compact_id(&container_id)),
                    overview,
                })
            })()
            .map_err(|error: anyhow::Error| error.to_string());
            let _ = tx.send(DockerJobResult {
                job_id,
                session_id: job_session_id,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn load_docker_details(
        &mut self,
        container_id: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.session.active_ssh_config.clone() else {
            self.remote_ops.docker.status =
                "start an SSH session before reading Docker details".to_string();
            self.terminal.view.status = self.remote_ops.docker.status.clone();
            cx.notify();
            return;
        };
        let Some(job_session_id) = self.session.active_id.clone() else {
            self.remote_ops.docker.status =
                "start an SSH session before reading Docker details".to_string();
            cx.notify();
            return;
        };
        if self.remote_ops.docker.pending
            && self.remote_ops.docker.job_session_id.as_deref() == Some(job_session_id.as_str())
        {
            self.remote_ops.docker.status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        let job_id = self.begin_docker_job(job_session_id.clone());
        self.remote_ops.docker.details_container_id = Some(container_id.clone());
        self.remote_ops.docker.details_last_refresh_at = Some(Instant::now());
        self.remote_ops.docker.status =
            format!("loading details for {}", compact_id(&container_id));
        let tx = self.remote_ops.docker.tx.clone();
        std::thread::spawn(move || {
            let result = DockerService::new(config)
                .container_details(&container_id)
                .map(|details| DockerJobOutput::Details {
                    container_id,
                    details,
                })
                .map_err(|error| error.to_string());
            let _ = tx.send(DockerJobResult {
                job_id,
                session_id: job_session_id,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn close_docker_details(&mut self, cx: &mut Context<Self>) {
        self.remote_ops.docker.details = None;
        self.remote_ops.docker.details_container_id = None;
        self.remote_ops.docker.details_last_refresh_at = None;
        self.remote_ops.docker.status = "container details closed".to_string();
        self.terminal.view.status = self.remote_ops.docker.status.clone();
        cx.notify();
    }

    pub(in crate::features) fn send_docker_container_logs_to_terminal(
        &mut self,
        container_id: String,
        cx: &mut Context<Self>,
    ) {
        self.send_docker_terminal_command(
            format!("docker logs -f --tail 100 {}", shell_quote(&container_id)),
            format!("following logs for {}", compact_id(&container_id)),
            cx,
        );
    }

    pub(in crate::features) fn enter_docker_container_terminal(
        &mut self,
        container_id: String,
        cx: &mut Context<Self>,
    ) {
        self.send_docker_terminal_command(
            format!(
                "docker exec -it {} sh -lc {}",
                shell_quote(&container_id),
                shell_quote(DOCKER_SHELL_SELECTOR)
            ),
            format!("entering container {}", compact_id(&container_id)),
            cx,
        );
    }

    pub(in crate::features) fn send_docker_compose_service_logs_to_terminal(
        &mut self,
        project_name: String,
        config_files: Option<String>,
        service_name: String,
        cx: &mut Context<Self>,
    ) {
        self.send_docker_terminal_command(
            format!(
                "{} logs -f --tail 100 {}",
                docker_compose_terminal_base(&project_name, config_files.as_deref()),
                shell_quote(&service_name)
            ),
            format!("following compose logs for {service_name}"),
            cx,
        );
    }

    pub(in crate::features) fn send_docker_terminal_command(
        &mut self,
        mut command: String,
        status: String,
        cx: &mut Context<Self>,
    ) {
        if self.session.active_id.is_none() {
            self.remote_ops.docker.status =
                "start a terminal session before sending Docker commands".to_string();
            self.terminal.view.status = self.remote_ops.docker.status.clone();
            cx.notify();
            return;
        }
        if !command.ends_with('\n') {
            command.push('\n');
        }
        self.selected_nav = NavItem::Workspace;
        if self.send_terminal_input(command.into_bytes(), cx) {
            self.remote_ops.docker.status = status;
            self.terminal.view.status = self.remote_ops.docker.status.clone();
            cx.notify();
        } else {
            self.remote_ops.docker.status = self.terminal.view.status.clone();
        }
    }

    pub(in crate::features) fn toggle_docker_compose_project(
        &mut self,
        project_name: String,
        config_files: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = docker_compose_project_key(&project_name, config_files.as_deref());
        if self.remote_ops.docker.compose_expanded.remove(&key) {
            self.remote_ops.docker.status = format!("collapsed compose project {project_name}");
            cx.notify();
            return;
        }

        self.remote_ops.docker.compose_expanded.insert(key.clone());
        self.remote_ops.docker.status = format!("expanded compose project {project_name}");
        if !self.remote_ops.docker.compose_services.contains_key(&key)
            && !self
                .remote_ops
                .docker
                .compose_service_errors
                .contains_key(&key)
        {
            self.load_docker_compose_services(project_name, config_files, window, cx);
        } else {
            cx.notify();
        }
    }

    pub(in crate::features) fn load_docker_compose_services(
        &mut self,
        project_name: String,
        config_files: Option<String>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.session.active_ssh_config.clone() else {
            self.remote_ops.docker.status =
                "start an SSH session before reading compose services".to_string();
            self.terminal.view.status = self.remote_ops.docker.status.clone();
            cx.notify();
            return;
        };
        let Some(job_session_id) = self.session.active_id.clone() else {
            self.remote_ops.docker.status =
                "start an SSH session before reading compose services".to_string();
            cx.notify();
            return;
        };
        if self.remote_ops.docker.pending
            && self.remote_ops.docker.job_session_id.as_deref() == Some(job_session_id.as_str())
        {
            self.remote_ops.docker.status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        let key = docker_compose_project_key(&project_name, config_files.as_deref());
        let job_id = self.begin_docker_job(job_session_id.clone());
        self.remote_ops.docker.status = format!("loading compose services for {project_name}");
        self.remote_ops.docker.compose_service_errors.remove(&key);
        let tx = self.remote_ops.docker.tx.clone();
        std::thread::spawn(move || {
            let result = DockerService::new(config)
                .compose_services(&project_name, config_files.as_deref())
                .map(|services| DockerJobOutput::ComposeServices {
                    key,
                    project_name,
                    services,
                })
                .map_err(|error| error.to_string());
            let _ = tx.send(DockerJobResult {
                job_id,
                session_id: job_session_id,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn docker_compose_service_action(
        &mut self,
        project_name: String,
        config_files: Option<String>,
        service_name: String,
        action: &'static str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.session.active_ssh_config.clone() else {
            self.remote_ops.docker.status =
                "start an SSH session before changing compose services".to_string();
            self.terminal.view.status = self.remote_ops.docker.status.clone();
            cx.notify();
            return;
        };
        let Some(job_session_id) = self.session.active_id.clone() else {
            self.remote_ops.docker.status =
                "start an SSH session before changing compose services".to_string();
            cx.notify();
            return;
        };
        if self.remote_ops.docker.pending
            && self.remote_ops.docker.job_session_id.as_deref() == Some(job_session_id.as_str())
        {
            self.remote_ops.docker.status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        let key = docker_compose_project_key(&project_name, config_files.as_deref());
        let job_id = self.begin_docker_job(job_session_id.clone());
        self.remote_ops.docker.status = format!("compose {action} {service_name}");
        let tx = self.remote_ops.docker.tx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let service = DockerService::new(config);
                service.compose_service_action(
                    &project_name,
                    config_files.as_deref(),
                    &service_name,
                    action,
                )?;
                let overview = service.overview()?;
                let services = service.compose_services(&project_name, config_files.as_deref())?;
                Ok(DockerJobOutput::ComposeServiceAction {
                    key,
                    service_name,
                    action: action.to_string(),
                    overview,
                    services,
                })
            })()
            .map_err(|error: anyhow::Error| error.to_string());
            let _ = tx.send(DockerJobResult {
                job_id,
                session_id: job_session_id,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn docker_compose_action(
        &mut self,
        project_name: String,
        config_files: Option<String>,
        action: &'static str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.session.active_ssh_config.clone() else {
            self.remote_ops.docker.status =
                "start an SSH session before changing compose projects".to_string();
            self.terminal.view.status = self.remote_ops.docker.status.clone();
            cx.notify();
            return;
        };
        let Some(job_session_id) = self.session.active_id.clone() else {
            self.remote_ops.docker.status =
                "start an SSH session before changing compose projects".to_string();
            cx.notify();
            return;
        };
        if self.remote_ops.docker.pending
            && self.remote_ops.docker.job_session_id.as_deref() == Some(job_session_id.as_str())
        {
            self.remote_ops.docker.status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        let key = docker_compose_project_key(&project_name, config_files.as_deref());
        let job_id = self.begin_docker_job(job_session_id.clone());
        self.remote_ops.docker.status = format!("compose {action} {project_name}");
        self.remote_ops.docker.compose_service_errors.remove(&key);
        let tx = self.remote_ops.docker.tx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let service = DockerService::new(config);
                service.compose_action(&project_name, config_files.as_deref(), action)?;
                let overview = service.overview()?;
                let service_result =
                    service.compose_services(&project_name, config_files.as_deref());
                let (services, service_error) = match service_result {
                    Ok(services) => (Some(services), None),
                    Err(error) => (None, Some(error.to_string())),
                };
                Ok(DockerJobOutput::ComposeProjectAction {
                    key,
                    project_name,
                    action: action.to_string(),
                    overview,
                    services,
                    service_error,
                })
            })()
            .map_err(|error: anyhow::Error| error.to_string());
            let _ = tx.send(DockerJobResult {
                job_id,
                session_id: job_session_id,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn request_docker_confirm(
        &mut self,
        confirm: DockerConfirmState,
        cx: &mut Context<Self>,
    ) {
        self.remote_ops.docker.confirm = Some(confirm);
        self.remote_ops.docker.status = "confirm Docker operation".to_string();
        cx.notify();
    }

    pub(in crate::features) fn cancel_docker_confirm(&mut self, cx: &mut Context<Self>) {
        self.remote_ops.docker.confirm = None;
        self.remote_ops.docker.status = "Docker operation cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn confirm_docker_action(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(confirm) = self.remote_ops.docker.confirm.clone() else {
            return;
        };
        let Some(config) = self.session.active_ssh_config.clone() else {
            self.remote_ops.docker.status =
                "start an SSH session before changing Docker resources".to_string();
            self.terminal.view.status = self.remote_ops.docker.status.clone();
            cx.notify();
            return;
        };
        let Some(job_session_id) = self.session.active_id.clone() else {
            self.remote_ops.docker.status =
                "start an SSH session before changing Docker resources".to_string();
            cx.notify();
            return;
        };
        if self.remote_ops.docker.pending
            && self.remote_ops.docker.job_session_id.as_deref() == Some(job_session_id.as_str())
        {
            self.remote_ops.docker.status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        let job_id = self.begin_docker_job(job_session_id.clone());
        self.remote_ops.docker.status = format!("running {}", confirm.title);
        let tx = self.remote_ops.docker.tx.clone();
        std::thread::spawn(move || {
            let result = (|| {
                let label = confirm.title.clone();
                let service = DockerService::new(config);
                match confirm.action {
                    DockerConfirmAction::ContainerAction {
                        container_id,
                        action,
                    } => {
                        service.container_action(&container_id, action)?;
                    }
                    DockerConfirmAction::ImageRemove { image_id, force } => {
                        service.image_remove(&image_id, force)?;
                    }
                    DockerConfirmAction::VolumeRemove { volume_name, force } => {
                        service.volume_remove(&volume_name, force)?;
                    }
                    DockerConfirmAction::NetworkRemove { network_id } => {
                        service.network_remove(&network_id)?;
                    }
                    DockerConfirmAction::ComposeAction {
                        project_name,
                        config_files,
                        action,
                    } => {
                        service.compose_action(&project_name, config_files.as_deref(), action)?;
                        let key =
                            docker_compose_project_key(&project_name, config_files.as_deref());
                        let overview = service.overview()?;
                        let service_result =
                            service.compose_services(&project_name, config_files.as_deref());
                        let (services, service_error) = match service_result {
                            Ok(services) => (Some(services), None),
                            Err(error) => (None, Some(error.to_string())),
                        };
                        return Ok(DockerJobOutput::ComposeProjectAction {
                            key,
                            project_name,
                            action: action.to_string(),
                            overview,
                            services,
                            service_error,
                        });
                    }
                    DockerConfirmAction::Prune { volumes } => {
                        service.system_prune(volumes)?;
                    }
                }
                let overview = service.overview()?;
                Ok(DockerJobOutput::RefreshedAfterAction { label, overview })
            })()
            .map_err(|error: anyhow::Error| error.to_string());
            let _ = tx.send(DockerJobResult {
                job_id,
                session_id: job_session_id,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn prune_docker_system(&mut self, cx: &mut Context<Self>) {
        self.request_docker_confirm(
            DockerConfirmState {
                title: "Docker system prune".to_string(),
                detail: "docker system prune -f --volumes".to_string(),
                action: DockerConfirmAction::Prune { volumes: true },
            },
            cx,
        );
    }

    pub(in crate::features) fn drain_docker_events(&mut self) -> bool {
        let mut dirty = false;
        for _ in 0..DOCKER_EVENT_DRAIN_LIMIT {
            let Ok(event) = self.remote_ops.docker.rx.try_recv() else {
                break;
            };
            if !remote_job_event_matches(
                self.remote_ops.docker.job_id,
                self.remote_ops.docker.job_session_id.as_deref(),
                event.job_id,
                &event.session_id,
            ) {
                continue;
            }
            dirty = true;
            self.remote_ops.docker.pending = false;
            self.remote_ops.docker.job_session_id = None;
            if self.session.active_id.as_deref() != Some(event.session_id.as_str()) {
                continue;
            }
            let was_overview_refresh = self.remote_ops.docker.status == "loading Docker overview";
            match event.result {
                Ok(DockerJobOutput::Overview(overview)) => {
                    self.remote_ops.docker.consecutive_refresh_failures = 0;
                    self.remote_ops.docker.status = docker_overview_status(&overview);
                    self.terminal.view.status = self.remote_ops.docker.status.clone();
                    self.apply_docker_overview(overview);
                }
                Ok(DockerJobOutput::Details {
                    container_id,
                    details,
                }) => {
                    self.remote_ops.docker.status =
                        format!("loaded details for {}", compact_id(&container_id));
                    self.terminal.view.status = self.remote_ops.docker.status.clone();
                    self.remote_ops.docker.details = Some(details);
                    self.remote_ops.docker.details_container_id = Some(container_id);
                }
                Ok(DockerJobOutput::ComposeServices {
                    key,
                    project_name,
                    services,
                }) => {
                    self.remote_ops.docker.status =
                        format!("loaded {} service(s) for {project_name}", services.len());
                    self.terminal.view.status = self.remote_ops.docker.status.clone();
                    self.remote_ops.docker.compose_service_errors.remove(&key);
                    self.remote_ops
                        .docker
                        .compose_services
                        .insert(key, services);
                }
                Ok(DockerJobOutput::ComposeServiceAction {
                    key,
                    service_name,
                    action,
                    overview,
                    services,
                }) => {
                    self.remote_ops.docker.status = format!("compose {action} {service_name}");
                    self.terminal.view.status = self.remote_ops.docker.status.clone();
                    self.apply_docker_overview(overview);
                    self.remote_ops
                        .docker
                        .compose_services
                        .insert(key.clone(), services);
                    self.remote_ops.docker.compose_service_errors.remove(&key);
                }
                Ok(DockerJobOutput::ComposeProjectAction {
                    key,
                    project_name,
                    action,
                    overview,
                    services,
                    service_error,
                }) => {
                    self.remote_ops.docker.status = format!("compose {action} {project_name}");
                    self.terminal.view.status = self.remote_ops.docker.status.clone();
                    self.apply_docker_overview(overview);
                    if let Some(services) = services {
                        self.remote_ops
                            .docker
                            .compose_services
                            .insert(key.clone(), services);
                        self.remote_ops.docker.compose_service_errors.remove(&key);
                    } else if let Some(error) = service_error {
                        self.remote_ops.docker.compose_services.remove(&key);
                        self.remote_ops
                            .docker
                            .compose_service_errors
                            .insert(key.clone(), error);
                    }
                    self.remote_ops.docker.confirm = None;
                }
                Ok(DockerJobOutput::RefreshedAfterAction { label, overview }) => {
                    let container_count = overview.containers.len();
                    self.apply_docker_overview(overview);
                    self.remote_ops.docker.status =
                        format!("{label} completed · {container_count} container(s)");
                    self.terminal.view.status = self.remote_ops.docker.status.clone();
                    self.remote_ops.docker.confirm = None;
                }
                Err(error) => {
                    if was_overview_refresh {
                        self.remote_ops.docker.consecutive_refresh_failures = self
                            .remote_ops
                            .docker
                            .consecutive_refresh_failures
                            .saturating_add(1);
                        if self.remote_ops.docker.consecutive_refresh_failures >= 3 {
                            self.remote_ops.docker.overview = None;
                        }
                    }
                    self.remote_ops.docker.status = format!("Docker operation failed: {error}");
                    self.terminal.view.status = self.remote_ops.docker.status.clone();
                }
            }
        }
        dirty
    }

    fn begin_docker_job(&mut self, session_id: String) -> u64 {
        self.remote_ops.docker.job_id = self.remote_ops.docker.job_id.wrapping_add(1).max(1);
        self.remote_ops.docker.job_session_id = Some(session_id);
        self.remote_ops.docker.pending = true;
        self.remote_ops.docker.job_id
    }

    pub(in crate::features) fn apply_docker_overview(&mut self, overview: RemoteDockerOverview) {
        if let Some(details_id) = self.remote_ops.docker.details_container_id.as_deref()
            && !overview
                .containers
                .iter()
                .any(|container| container.id == details_id)
        {
            self.remote_ops.docker.details = None;
            self.remote_ops.docker.details_container_id = None;
        }
        let active_compose_keys = overview
            .compose_projects
            .iter()
            .map(|project| {
                docker_compose_project_key(&project.name, Some(project.config_files.as_str()))
            })
            .collect::<HashSet<_>>();
        self.remote_ops
            .docker
            .compose_expanded
            .retain(|key| active_compose_keys.contains(key));
        self.remote_ops
            .docker
            .compose_services
            .retain(|key, _| active_compose_keys.contains(key));
        self.remote_ops
            .docker
            .compose_service_errors
            .retain(|key, _| active_compose_keys.contains(key));
        self.remote_ops.docker.overview = Some(overview);
    }
}

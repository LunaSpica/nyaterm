use super::*;

const DOCKER_EVENT_DRAIN_LIMIT: usize = 16;

impl NyaTermApp {
    pub(in crate::features) fn refresh_docker(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status = "start an SSH session before inspecting Docker".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        self.docker_pending = true;
        self.docker_last_refresh_at = Some(Instant::now());
        self.docker_status = "loading Docker overview".to_string();
        let tx = self.docker_tx.clone();
        std::thread::spawn(move || {
            let result = DockerService::new(config)
                .overview()
                .map(DockerJobOutput::Overview)
                .map_err(|error| error.to_string());
            let _ = tx.send(DockerJobResult { result });
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
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status = "start an SSH session before changing containers".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        self.docker_pending = true;
        self.docker_status = format!("Docker {action} {}", compact_id(&container_id));
        self.docker_details = None;
        self.docker_details_container_id = None;
        let tx = self.docker_tx.clone();
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
            let _ = tx.send(DockerJobResult { result });
        });
        cx.notify();
    }

    pub(in crate::features) fn load_docker_details(
        &mut self,
        container_id: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status = "start an SSH session before reading Docker details".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        self.docker_pending = true;
        self.docker_details_container_id = Some(container_id.clone());
        self.docker_details_last_refresh_at = Some(Instant::now());
        self.docker_status = format!("loading details for {}", compact_id(&container_id));
        let tx = self.docker_tx.clone();
        std::thread::spawn(move || {
            let result = DockerService::new(config)
                .container_details(&container_id)
                .map(|details| DockerJobOutput::Details {
                    container_id,
                    details,
                })
                .map_err(|error| error.to_string());
            let _ = tx.send(DockerJobResult { result });
        });
        cx.notify();
    }

    pub(in crate::features) fn close_docker_details(&mut self, cx: &mut Context<Self>) {
        self.docker_details = None;
        self.docker_details_container_id = None;
        self.docker_details_last_refresh_at = None;
        self.docker_status = "container details closed".to_string();
        self.terminal_status = self.docker_status.clone();
        cx.notify();
    }

    pub(in crate::features) fn load_docker_logs(
        &mut self,
        container_id: String,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status = "start an SSH session before reading Docker logs".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        self.docker_pending = true;
        self.docker_status = format!("loading logs for {}", compact_id(&container_id));
        let tx = self.docker_tx.clone();
        std::thread::spawn(move || {
            let result = DockerService::new(config)
                .container_logs(&container_id, 200)
                .map(|output| DockerJobOutput::Logs {
                    container_id,
                    text: if output.stderr.trim().is_empty() {
                        output.stdout
                    } else {
                        format!("{}\n{}", output.stdout, output.stderr)
                    },
                })
                .map_err(|error| error.to_string());
            let _ = tx.send(DockerJobResult { result });
        });
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
        if self.active_session_id.is_none() {
            self.docker_status =
                "start a terminal session before sending Docker commands".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        }
        if !command.ends_with('\n') {
            command.push('\n');
        }
        self.selected_nav = NavItem::Workspace;
        if self.send_terminal_input(command.into_bytes(), cx) {
            self.docker_status = status;
            self.terminal_status = self.docker_status.clone();
            cx.notify();
        } else {
            self.docker_status = self.terminal_status.clone();
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
        if self.docker_compose_expanded.remove(&key) {
            self.docker_status = format!("collapsed compose project {project_name}");
            cx.notify();
            return;
        }

        self.docker_compose_expanded.insert(key.clone());
        self.docker_status = format!("expanded compose project {project_name}");
        if !self.docker_compose_services.contains_key(&key)
            && !self.docker_compose_service_errors.contains_key(&key)
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
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status = "start an SSH session before reading compose services".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        let key = docker_compose_project_key(&project_name, config_files.as_deref());
        self.docker_pending = true;
        self.docker_status = format!("loading compose services for {project_name}");
        self.docker_compose_service_errors.remove(&key);
        let tx = self.docker_tx.clone();
        std::thread::spawn(move || {
            let result = DockerService::new(config)
                .compose_services(&project_name, config_files.as_deref())
                .map(|services| DockerJobOutput::ComposeServices {
                    key,
                    project_name,
                    services,
                })
                .map_err(|error| error.to_string());
            let _ = tx.send(DockerJobResult { result });
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
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status =
                "start an SSH session before changing compose services".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        let key = docker_compose_project_key(&project_name, config_files.as_deref());
        self.docker_pending = true;
        self.docker_status = format!("compose {action} {service_name}");
        let tx = self.docker_tx.clone();
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
            let _ = tx.send(DockerJobResult { result });
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
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status =
                "start an SSH session before changing compose projects".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        let key = docker_compose_project_key(&project_name, config_files.as_deref());
        self.docker_pending = true;
        self.docker_status = format!("compose {action} {project_name}");
        self.docker_compose_service_errors.remove(&key);
        let tx = self.docker_tx.clone();
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
            let _ = tx.send(DockerJobResult { result });
        });
        cx.notify();
    }

    pub(in crate::features) fn request_docker_confirm(
        &mut self,
        confirm: DockerConfirmState,
        cx: &mut Context<Self>,
    ) {
        self.docker_confirm = Some(confirm);
        self.docker_status = "confirm Docker operation".to_string();
        cx.notify();
    }

    pub(in crate::features) fn cancel_docker_confirm(&mut self, cx: &mut Context<Self>) {
        self.docker_confirm = None;
        self.docker_status = "Docker operation cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn confirm_docker_action(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(confirm) = self.docker_confirm.clone() else {
            return;
        };
        let Some(config) = self.active_ssh_config.clone() else {
            self.docker_status =
                "start an SSH session before changing Docker resources".to_string();
            self.terminal_status = self.docker_status.clone();
            cx.notify();
            return;
        };
        if self.docker_pending {
            self.docker_status = "Docker operation already running".to_string();
            cx.notify();
            return;
        }

        self.docker_pending = true;
        self.docker_status = format!("running {}", confirm.title);
        let tx = self.docker_tx.clone();
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
            let _ = tx.send(DockerJobResult { result });
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
            let Ok(event) = self.docker_rx.try_recv() else {
                break;
            };
            dirty = true;
            self.docker_pending = false;
            match event.result {
                Ok(DockerJobOutput::Overview(overview)) => {
                    self.docker_status = docker_overview_status(&overview);
                    self.terminal_status = self.docker_status.clone();
                    self.apply_docker_overview(overview);
                }
                Ok(DockerJobOutput::Details {
                    container_id,
                    details,
                }) => {
                    self.docker_status =
                        format!("loaded details for {}", compact_id(&container_id));
                    self.terminal_status = self.docker_status.clone();
                    self.docker_details = Some(details);
                    self.docker_details_container_id = Some(container_id);
                }
                Ok(DockerJobOutput::Logs { container_id, text }) => {
                    self.docker_status = format!("loaded logs for {}", compact_id(&container_id));
                    self.terminal_status = self.docker_status.clone();
                    self.docker_logs_container_id = Some(container_id);
                    self.docker_logs = truncate_preview(&text, 4000);
                }
                Ok(DockerJobOutput::ComposeServices {
                    key,
                    project_name,
                    services,
                }) => {
                    self.docker_status =
                        format!("loaded {} service(s) for {project_name}", services.len());
                    self.terminal_status = self.docker_status.clone();
                    self.docker_compose_service_errors.remove(&key);
                    self.docker_compose_services.insert(key, services);
                }
                Ok(DockerJobOutput::ComposeServiceAction {
                    key,
                    service_name,
                    action,
                    overview,
                    services,
                }) => {
                    self.docker_status = format!("compose {action} {service_name}");
                    self.terminal_status = self.docker_status.clone();
                    self.apply_docker_overview(overview);
                    self.docker_compose_services.insert(key.clone(), services);
                    self.docker_compose_service_errors.remove(&key);
                }
                Ok(DockerJobOutput::ComposeProjectAction {
                    key,
                    project_name,
                    action,
                    overview,
                    services,
                    service_error,
                }) => {
                    self.docker_status = format!("compose {action} {project_name}");
                    self.terminal_status = self.docker_status.clone();
                    self.apply_docker_overview(overview);
                    if let Some(services) = services {
                        self.docker_compose_services.insert(key.clone(), services);
                        self.docker_compose_service_errors.remove(&key);
                    } else if let Some(error) = service_error {
                        self.docker_compose_services.remove(&key);
                        self.docker_compose_service_errors
                            .insert(key.clone(), error);
                    }
                    self.docker_confirm = None;
                }
                Ok(DockerJobOutput::RefreshedAfterAction { label, overview }) => {
                    let container_count = overview.containers.len();
                    self.apply_docker_overview(overview);
                    self.docker_status =
                        format!("{label} completed · {container_count} container(s)");
                    self.terminal_status = self.docker_status.clone();
                    self.docker_confirm = None;
                }
                Err(error) => {
                    self.docker_status = format!("Docker operation failed: {error}");
                    self.terminal_status = self.docker_status.clone();
                }
            }
        }
        dirty
    }

    pub(in crate::features) fn apply_docker_overview(&mut self, overview: RemoteDockerOverview) {
        if let Some(details_id) = self.docker_details_container_id.as_deref()
            && !overview
                .containers
                .iter()
                .any(|container| container.id == details_id)
        {
            self.docker_details = None;
            self.docker_details_container_id = None;
        }
        let active_compose_keys = overview
            .compose_projects
            .iter()
            .map(|project| {
                docker_compose_project_key(&project.name, Some(project.config_files.as_str()))
            })
            .collect::<HashSet<_>>();
        self.docker_compose_expanded
            .retain(|key| active_compose_keys.contains(key));
        self.docker_compose_services
            .retain(|key, _| active_compose_keys.contains(key));
        self.docker_compose_service_errors
            .retain(|key, _| active_compose_keys.contains(key));
        self.docker_overview = Some(overview);
    }
}

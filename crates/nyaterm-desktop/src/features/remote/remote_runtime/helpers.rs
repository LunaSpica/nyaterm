use super::*;

pub(super) const DOCKER_SHELL_SELECTOR: &str = "if command -v bash >/dev/null 2>&1; then exec bash; elif command -v zsh >/dev/null 2>&1; then exec zsh; elif command -v fish >/dev/null 2>&1; then exec fish; elif command -v ash >/dev/null 2>&1; then exec ash; else exec sh; fi";

pub(super) fn docker_compose_terminal_base(
    project_name: &str,
    config_files: Option<&str>,
) -> String {
    let mut command = String::from("docker compose");
    for file in config_files.unwrap_or_default().split(',') {
        let file = file.trim();
        if !file.is_empty() && !file.eq_ignore_ascii_case("n/a") {
            command.push_str(" -f ");
            command.push_str(&shell_quote(file));
        }
    }
    command.push_str(" -p ");
    command.push_str(&shell_quote(project_name));
    command
}

pub(super) fn docker_overview_status(overview: &RemoteDockerOverview) -> String {
    if overview.available {
        format!(
            "Docker {} · {} container(s)",
            if overview.version.trim().is_empty() {
                "available".to_string()
            } else {
                overview.version.clone()
            },
            overview.containers.len()
        )
    } else {
        "Docker is not available on this SSH host".to_string()
    }
}

pub(super) fn shell_quote(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

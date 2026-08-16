//! Command risk classification.
//!
//! Split out of `ai.rs` by domain. This decides how dangerous a shell command
//! looks before the agent is allowed to run it, so the pattern lists and the
//! escalation rules are behaviour, not formatting: they are unchanged here.

use super::RiskLevel;

pub fn parse_risk_level_label(value: &str) -> Option<RiskLevel> {
    match value.trim().replace('-', "_").to_ascii_lowercase().as_str() {
        "low" => Some(RiskLevel::Low),
        "medium" | "moderate" => Some(RiskLevel::Medium),
        "high" => Some(RiskLevel::High),
        "critical" | "danger" | "dangerous" => Some(RiskLevel::Critical),
        _ => None,
    }
}

pub fn risk_label(risk: &RiskLevel) -> &'static str {
    match risk {
        RiskLevel::Low => "low",
        RiskLevel::Medium => "medium",
        RiskLevel::High => "high",
        RiskLevel::Critical => "critical",
    }
}

pub(super) fn max_risk(a: RiskLevel, b: RiskLevel) -> RiskLevel {
    if a >= b { a } else { b }
}

pub fn assess_local_command_risk(command: &str) -> (RiskLevel, String) {
    let normalized = normalize_command(command);
    let compact = normalized.split_whitespace().collect::<Vec<_>>().join(" ");

    if compact.is_empty() {
        return (RiskLevel::Medium, "empty command".to_string());
    }

    if is_root_rm_command(&compact) || is_dangerous_dd_command(&compact) {
        return (
            RiskLevel::Critical,
            "matches irreversible or system-disruptive command pattern".to_string(),
        );
    }

    let critical_patterns = [
        "mkfs",
        "wipefs",
        ":(){",
        "shutdown",
        "poweroff",
        "reboot",
        "halt",
        "systemctl stop ssh",
        "systemctl stop sshd",
        "service ssh stop",
        "service sshd stop",
    ];
    if command_contains_any(&compact, &critical_patterns) {
        return (
            RiskLevel::Critical,
            "matches irreversible or system-disruptive command pattern".to_string(),
        );
    }

    let high_patterns = [
        "rm -r",
        "rm -f",
        " rmdir ",
        " chmod -r",
        " chown -r",
        "systemctl restart",
        "systemctl stop",
        "service ",
        "apt install",
        "apt remove",
        "apt purge",
        "yum install",
        "yum remove",
        "dnf install",
        "dnf remove",
        "pacman -s",
        "pacman -r",
        "brew install",
        "brew uninstall",
        "npm install -g",
        "pip install",
        "docker rm",
        "docker rmi",
        "docker system prune",
        "kubectl delete",
        "kubectl drain",
        "kubectl apply",
        "kubectl replace",
        "git reset --hard",
        "git clean -fd",
    ];
    if compact.starts_with("sudo ") || command_contains_any(&compact, &high_patterns) {
        return (
            RiskLevel::High,
            "matches privileged, destructive, restart, package, container, or cluster mutation pattern"
                .to_string(),
        );
    }

    let medium_patterns = [
        " > ",
        ">>",
        " tee ",
        " touch ",
        " mkdir ",
        " cp ",
        " mv ",
        " chmod ",
        " chown ",
        " setfacl ",
        " export ",
        "git checkout",
        "git switch",
        "git pull",
        "git merge",
        "npm run",
        "make install",
    ];
    if command_contains_any(&format!(" {compact} "), &medium_patterns) {
        return (
            RiskLevel::Medium,
            "matches local write or state-changing command pattern".to_string(),
        );
    }

    let readonly_prefixes = [
        "ls",
        "pwd",
        "whoami",
        "id",
        "uname",
        "cat",
        "less",
        "head",
        "tail",
        "grep",
        "rg",
        "find",
        "df",
        "du",
        "free",
        "top",
        "ps",
        "ss",
        "netstat",
        "ip ",
        "journalctl",
        "systemctl status",
        "docker ps",
        "docker logs",
        "kubectl get",
        "kubectl describe",
        "git status",
        "git log",
        "git diff",
    ];
    if readonly_prefixes
        .iter()
        .any(|prefix| compact == prefix.trim() || compact.starts_with(&format!("{prefix} ")))
    {
        return (
            RiskLevel::Low,
            "matches read-only diagnostic pattern".to_string(),
        );
    }

    (
        RiskLevel::Medium,
        "no explicit read-only pattern matched; defaulting to medium".to_string(),
    )
}

fn normalize_command(command: &str) -> String {
    command
        .trim()
        .replace("\r\n", "\n")
        .replace('\n', " ")
        .to_ascii_lowercase()
}

fn command_contains_any(command: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| command.contains(pattern))
}

fn is_root_rm_command(command: &str) -> bool {
    let tokens: Vec<&str> = command.split_whitespace().collect();
    if tokens.first() != Some(&"rm") {
        return false;
    }
    let has_recursive_force = tokens
        .iter()
        .any(|token| token.starts_with('-') && token.contains('r') && token.contains('f'));
    has_recursive_force
        && tokens
            .iter()
            .skip(1)
            .any(|token| matches!(*token, "/" | "/*" | "--no-preserve-root"))
}

fn is_dangerous_dd_command(command: &str) -> bool {
    command.starts_with("dd ") && command.contains("of=/dev/")
}

#[cfg(test)]
mod tests {
    use super::RiskLevel;
    use crate::{assess_agent_command_risk, parse_agent_model_output};

    #[test]
    fn local_agent_risk_overrides_unsafe_model_risk() {
        let parsed = parse_agent_model_output(
            r#"{"thought":"danger","action":"execute_command","command":"rm -rf /","riskLevel":"low","riskReason":"claimed safe"}"#,
        )
        .expect("agent response");
        let assessment = assess_agent_command_risk(&parsed, "rm -rf /");

        assert_eq!(assessment.model_risk, RiskLevel::Low);
        assert_eq!(assessment.local_risk, RiskLevel::Critical);
        assert_eq!(assessment.effective_risk, RiskLevel::Critical);
        assert!(assessment.risk_reason.unwrap().contains("claimed safe"));
    }
}

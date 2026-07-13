#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TemporarySshLinkConfig {
    pub(crate) name: String,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) username: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TemporarySshLinkError {
    Empty,
    UnsupportedOption,
    MissingHost,
    InlinePassword,
    InvalidPort,
    InvalidInput,
}

impl TemporarySshLinkError {
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::Empty => "Paste an ssh:// URL or an ssh command.",
            Self::UnsupportedOption => {
                "This temporary link uses an SSH option that is not supported yet."
            }
            Self::MissingHost => "The temporary SSH link is missing a host.",
            Self::InlinePassword => "Inline SSH passwords are not allowed.",
            Self::InvalidPort => "The SSH port must be between 1 and 65535.",
            Self::InvalidInput => "The temporary SSH link is not valid.",
        }
    }
}

const DEFAULT_USERNAME: &str = "root";
const DEFAULT_PORT: u16 = 22;
const UNSUPPORTED_OPTIONS: [&str; 14] = [
    "-J", "-L", "-R", "-D", "-W", "-b", "-c", "-F", "-I", "-i", "-m", "-o", "-S", "-w",
];
const UNSUPPORTED_OPTION_CHARS: [char; 14] = [
    'J', 'L', 'R', 'D', 'W', 'b', 'c', 'F', 'I', 'i', 'm', 'O', 'S', 'w',
];
const UNSUPPORTED_LONG_OPTIONS: [&str; 7] = [
    "proxyjump",
    "proxycommand",
    "localforward",
    "remoteforward",
    "dynamicforward",
    "identityfile",
    "controlpath",
];

pub(crate) fn parse_temporary_ssh_link(
    input: &str,
) -> Result<TemporarySshLinkConfig, TemporarySshLinkError> {
    let text = input.trim();
    if text.is_empty() {
        return Err(TemporarySshLinkError::Empty);
    }

    if text.to_ascii_lowercase().starts_with("ssh://") {
        return parse_ssh_url(text);
    }

    let tokens = tokenize_shell_like(text);
    if tokens.is_empty() {
        return Err(TemporarySshLinkError::Empty);
    }

    let command_tokens = if tokens.first().is_some_and(|token| token == "ssh") {
        &tokens[1..]
    } else {
        &tokens[..]
    };
    let mut username = None;
    let mut host_spec = None;
    let mut port = None;
    let mut index = 0;

    while index < command_tokens.len() {
        let token = command_tokens[index].as_str();
        if token.is_empty() {
            index += 1;
            continue;
        }

        if token == "--" {
            host_spec = find_host_spec(&command_tokens[index + 1..]).or(host_spec);
            break;
        }

        if token == "-p" {
            if let Some(next) = command_tokens.get(index + 1)
                && let Ok(parsed) = parse_port_token(next)
            {
                port = Some(parsed);
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }

        if let Some(inline_port) = token.strip_prefix("-p").filter(|value| !value.is_empty()) {
            if let Ok(parsed) = parse_port_token(inline_port) {
                port = Some(parsed);
            }
            index += 1;
            continue;
        }

        if token == "-l" {
            if let Some(next) = command_tokens.get(index + 1)
                && !next.starts_with('-')
                && !next.contains('@')
            {
                username = Some(next.clone());
                index += 2;
                continue;
            }
            index += 1;
            continue;
        }

        if let Some(inline_user) = token.strip_prefix("-l").filter(|value| !value.is_empty()) {
            username = Some(inline_user.to_string());
            index += 1;
            continue;
        }

        if is_unsupported_option(token) {
            return Err(TemporarySshLinkError::UnsupportedOption);
        }

        if token.starts_with('-') {
            if option_consumes_value(token) {
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }

        if host_spec.is_none() {
            host_spec = Some(token.to_string());
        }
        index += 1;
    }

    let Some(host_spec) = host_spec else {
        return Err(TemporarySshLinkError::MissingHost);
    };
    build_config(&host_spec, username, port)
}

fn parse_ssh_url(text: &str) -> Result<TemporarySshLinkConfig, TemporarySshLinkError> {
    let rest = text
        .get(6..)
        .ok_or(TemporarySshLinkError::InvalidInput)?
        .trim();
    let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let authority = &rest[..authority_end];
    if authority.is_empty() {
        return Err(TemporarySshLinkError::MissingHost);
    }

    let (username, host_port) = match authority.rsplit_once('@') {
        Some((user_info, host_port)) => {
            if user_info.contains(':') {
                return Err(TemporarySshLinkError::InlinePassword);
            }
            let username = percent_decode_basic(user_info).unwrap_or_else(|| user_info.to_string());
            (
                if username.is_empty() {
                    DEFAULT_USERNAME.to_string()
                } else {
                    username
                },
                host_port,
            )
        }
        None => (DEFAULT_USERNAME.to_string(), authority),
    };
    let parsed = parse_host_port(host_port)?;
    if parsed.host.is_empty() {
        return Err(TemporarySshLinkError::MissingHost);
    }
    create_config(parsed.host, username, parsed.port.unwrap_or(DEFAULT_PORT))
}

fn build_config(
    host_spec: &str,
    explicit_username: Option<String>,
    explicit_port: Option<u16>,
) -> Result<TemporarySshLinkConfig, TemporarySshLinkError> {
    if host_spec.contains("://") && !host_spec.to_ascii_lowercase().starts_with("ssh://") {
        return Err(TemporarySshLinkError::InvalidInput);
    }
    let (username, target) = match host_spec.rsplit_once('@') {
        Some((user_part, target)) => {
            if user_part.contains(':') {
                return Err(TemporarySshLinkError::InlinePassword);
            }
            (
                if user_part.is_empty() {
                    explicit_username
                } else {
                    Some(user_part.to_string())
                },
                target,
            )
        }
        None => (explicit_username, host_spec),
    };
    let parsed = parse_host_port(target)?;
    if parsed.host.is_empty() {
        return Err(TemporarySshLinkError::MissingHost);
    }
    create_config(
        parsed.host,
        username.unwrap_or_else(|| DEFAULT_USERNAME.to_string()),
        explicit_port.or(parsed.port).unwrap_or(DEFAULT_PORT),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedHostPort {
    host: String,
    port: Option<u16>,
}

fn parse_host_port(target: &str) -> Result<ParsedHostPort, TemporarySshLinkError> {
    if let Some(rest) = target.strip_prefix('[') {
        let Some(end) = rest.find(']') else {
            return Ok(ParsedHostPort {
                host: target.to_string(),
                port: None,
            });
        };
        let host = rest[..end].to_string();
        let tail = &rest[end + 1..];
        let port = tail.strip_prefix(':').map(parse_port_token).transpose()?;
        return Ok(ParsedHostPort { host, port });
    }

    if target.matches(':').count() == 1 {
        let (host, port_text) = target
            .split_once(':')
            .ok_or(TemporarySshLinkError::InvalidInput)?;
        let port = if port_text.is_empty() {
            None
        } else {
            Some(parse_port_token(port_text)?)
        };
        return Ok(ParsedHostPort {
            host: host.to_string(),
            port,
        });
    }

    Ok(ParsedHostPort {
        host: target.to_string(),
        port: None,
    })
}

fn create_config(
    host: String,
    username: String,
    port: u16,
) -> Result<TemporarySshLinkConfig, TemporarySshLinkError> {
    if host.trim().is_empty() || username.trim().is_empty() {
        return Err(TemporarySshLinkError::MissingHost);
    }
    let host = host
        .trim()
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host.trim())
        .to_string();
    let username = username.trim().to_string();
    let name = format!("{username}@{host}:{port}");
    Ok(TemporarySshLinkConfig {
        name,
        host,
        port,
        username,
    })
}

fn tokenize_shell_like(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaping = false;

    for character in text.chars() {
        if escaping {
            current.push(character);
            escaping = false;
            continue;
        }
        if character == '\\' && quote != Some('\'') {
            escaping = true;
            continue;
        }
        if matches!(character, '\'' | '"') && quote.is_none() {
            quote = Some(character);
            continue;
        }
        if quote == Some(character) {
            quote = None;
            continue;
        }
        if character.is_whitespace() && quote.is_none() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(character);
    }

    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn find_host_spec(tokens: &[String]) -> Option<String> {
    tokens
        .iter()
        .find(|token| !token.is_empty() && !token.starts_with('-'))
        .cloned()
}

fn is_unsupported_option(token: &str) -> bool {
    if UNSUPPORTED_OPTIONS.contains(&token) {
        return true;
    }
    if token.starts_with("-o") {
        return is_unsupported_open_ssh_option(
            token.trim_start_matches("-o").trim_start_matches('='),
        );
    }
    token
        .strip_prefix('-')
        .and_then(|value| value.chars().next())
        .is_some_and(|flag| UNSUPPORTED_OPTION_CHARS.contains(&flag))
}

fn is_unsupported_open_ssh_option(option_text: &str) -> bool {
    let option = option_text
        .split('=')
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    UNSUPPORTED_LONG_OPTIONS.contains(&option.as_str())
}

fn option_consumes_value(token: &str) -> bool {
    matches!(token, "-A" | "-a" | "-E" | "-e" | "-Q")
}

fn parse_port_token(value: &str) -> Result<u16, TemporarySshLinkError> {
    value
        .parse::<u16>()
        .ok()
        .filter(|port| *port >= 1)
        .ok_or(TemporarySshLinkError::InvalidPort)
}

fn percent_decode_basic(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hi = bytes.get(index + 1).copied()?;
            let lo = bytes.get(index + 2).copied()?;
            let hex = [hi, lo];
            let decoded = std::str::from_utf8(&hex)
                .ok()
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())?;
            output.push(decoded);
            index += 3;
        } else {
            output.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(output).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ssh_url() {
        let parsed = parse_temporary_ssh_link("ssh://deploy@example.com:2200").unwrap();
        assert_eq!(parsed.name, "deploy@example.com:2200");
        assert_eq!(parsed.host, "example.com");
        assert_eq!(parsed.port, 2200);
        assert_eq!(parsed.username, "deploy");
    }

    #[test]
    fn parses_openssh_command() {
        let parsed = parse_temporary_ssh_link("ssh -p 2222 -l admin example.test").unwrap();
        assert_eq!(parsed.name, "admin@example.test:2222");
        assert_eq!(parsed.host, "example.test");
        assert_eq!(parsed.port, 2222);
        assert_eq!(parsed.username, "admin");
    }

    #[test]
    fn parses_user_host_port() {
        let parsed = parse_temporary_ssh_link("kang@[2001:db8::1]:2022").unwrap();
        assert_eq!(parsed.name, "kang@2001:db8::1:2022");
        assert_eq!(parsed.host, "2001:db8::1");
        assert_eq!(parsed.port, 2022);
        assert_eq!(parsed.username, "kang");
    }

    #[test]
    fn rejects_inline_password() {
        let error = parse_temporary_ssh_link("root:secret@example.com").unwrap_err();
        assert_eq!(error, TemporarySshLinkError::InlinePassword);
    }

    #[test]
    fn rejects_unsupported_options() {
        let error =
            parse_temporary_ssh_link("ssh -J jump.example.com root@example.com").unwrap_err();
        assert_eq!(error, TemporarySshLinkError::UnsupportedOption);
    }
}

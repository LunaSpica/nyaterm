use super::*;

pub(in crate::features::pages::tunnels) fn proxy_protocol_label(protocol: &str) -> &'static str {
    match protocol {
        "socks5" => "SOCKS5",
        "http" => "HTTP",
        "proxycommand" => "ProxyCommand",
        _ => "Proxy",
    }
}

pub(in crate::features::pages::tunnels) fn proxy_matches(proxy: &ProxyConfig, query: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    format!(
        "{} {} {} {} {} {} {}",
        proxy.id,
        proxy.name,
        proxy.protocol,
        proxy.host,
        proxy.port,
        proxy.command.as_deref().unwrap_or_default(),
        proxy.username.as_deref().unwrap_or_default()
    )
    .to_ascii_lowercase()
    .contains(query)
}

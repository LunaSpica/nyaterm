use super::*;

pub(in crate::features::pages::tunnels) fn tunnel_matches(
    tunnel: &TunnelConfig,
    query: &str,
) -> bool {
    if query.is_empty() {
        return true;
    }
    format!(
        "{} {} {} {} {} {} {}",
        tunnel.id,
        tunnel.name,
        tunnel.tunnel_type,
        tunnel.connection_id.as_deref().unwrap_or_default(),
        tunnel.listen_port,
        tunnel.target_host,
        tunnel.target_port
    )
    .to_ascii_lowercase()
    .contains(query)
}

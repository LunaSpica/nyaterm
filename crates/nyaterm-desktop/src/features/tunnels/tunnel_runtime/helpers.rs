use super::*;

pub(super) fn next_network_group_id<'a>(
    current_group_id: Option<&str>,
    group_ids: impl Iterator<Item = &'a str>,
) -> Option<String> {
    let mut cycle = std::iter::once(None)
        .chain(group_ids.map(Some))
        .collect::<Vec<_>>();
    if cycle.is_empty() {
        return None;
    }
    let current_index = cycle
        .iter()
        .position(|group_id| *group_id == current_group_id)
        .unwrap_or(0);
    cycle
        .remove((current_index + 1) % cycle.len())
        .map(ToOwned::to_owned)
}

pub(super) fn network_group_label<T>(group_id: Option<&str>, groups: &[T]) -> String
where
    T: NetworkGroupLike,
{
    group_id
        .and_then(|id| groups.iter().find(|group| group.network_group_id() == id))
        .map(|group| group.network_group_name().to_string())
        .unwrap_or_else(|| "Ungrouped".to_string())
}

pub(super) fn network_section_key(tab: NetworkTab, section_id: &str) -> String {
    match tab {
        NetworkTab::Tunnels => format!("tunnel:{section_id}"),
        NetworkTab::Proxies => format!("proxy:{section_id}"),
    }
}

pub(super) fn parse_port(value: &str) -> Option<u16> {
    let port = value.trim().parse::<u16>().ok()?;
    (port > 0).then_some(port)
}

pub(super) trait NetworkGroupLike {
    fn network_group_id(&self) -> &str;
    fn network_group_name(&self) -> &str;
}

impl NetworkGroupLike for TunnelGroup {
    fn network_group_id(&self) -> &str {
        &self.id
    }

    fn network_group_name(&self) -> &str {
        &self.name
    }
}

impl NetworkGroupLike for ProxyGroup {
    fn network_group_id(&self) -> &str {
        &self.id
    }

    fn network_group_name(&self) -> &str {
        &self.name
    }
}

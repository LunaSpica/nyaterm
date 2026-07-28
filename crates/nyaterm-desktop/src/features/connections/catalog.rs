//! Authoritative saved-connection catalog and runtime discovery results.
//!
//! Persistence remains implemented by `nyaterm-core::ConnectionStore`; this
//! state only owns the validated values currently presented by the desktop.

use nyaterm_core::{Group, SavedConnection};

pub(in crate::features) struct ConnectionCatalogState {
    pub connections: Vec<SavedConnection>,
    pub groups: Vec<Group>,
    pub serial_ports: Vec<String>,
}

impl ConnectionCatalogState {
    pub(in crate::features) fn new(connections: Vec<SavedConnection>, groups: Vec<Group>) -> Self {
        Self {
            connections,
            groups,
            serial_ports: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ConnectionCatalogState;

    #[test]
    fn catalog_keeps_runtime_discovery_separate_from_loaded_data() {
        let mut catalog = ConnectionCatalogState::new(Vec::new(), Vec::new());

        assert!(catalog.connections.is_empty());
        assert!(catalog.groups.is_empty());
        assert!(catalog.serial_ports.is_empty());

        catalog.serial_ports = vec!["ttyUSB0".to_string()];
        assert_eq!(catalog.serial_ports, ["ttyUSB0"]);
    }
}

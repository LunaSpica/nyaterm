//! Authoritative saved-connection catalog and runtime discovery results.
//!
//! Persistence remains implemented by `nyaterm-core::ConnectionStore`; this
//! state only owns the validated values currently presented by the desktop.

use nyaterm_core::{Group, SavedConnection};

pub(super) struct ConnectionCatalogState {
    connections: Vec<SavedConnection>,
    groups: Vec<Group>,
    serial_ports: Vec<String>,
}

impl ConnectionCatalogState {
    pub(super) fn new(connections: Vec<SavedConnection>, groups: Vec<Group>) -> Self {
        Self {
            connections,
            groups,
            serial_ports: Vec::new(),
        }
    }

    pub(super) fn connections(&self) -> &[SavedConnection] {
        &self.connections
    }

    pub(super) fn groups(&self) -> &[Group] {
        &self.groups
    }

    pub(super) fn serial_ports(&self) -> &[String] {
        &self.serial_ports
    }

    pub(super) fn replace_loaded(&mut self, connections: Vec<SavedConnection>, groups: Vec<Group>) {
        self.connections = connections;
        self.groups = groups;
    }

    pub(super) fn clear_loaded(&mut self) {
        self.connections.clear();
        self.groups.clear();
    }

    pub(super) fn clear_connections(&mut self) {
        self.connections.clear();
    }

    pub(super) fn replace_connections(&mut self, connections: Vec<SavedConnection>) {
        self.connections = connections;
    }

    pub(super) fn replace_serial_ports(&mut self, serial_ports: Vec<String>) {
        self.serial_ports = serial_ports;
    }

    pub(super) fn update_connection(&mut self, updated: SavedConnection) -> bool {
        let Some(connection) = self
            .connections
            .iter_mut()
            .find(|connection| connection.id == updated.id)
        else {
            return false;
        };
        *connection = updated;
        true
    }

    pub(super) fn connections_reordered_into_group(
        &self,
        source_ids: &[String],
        group_id: &Option<String>,
    ) -> Vec<SavedConnection> {
        let mut staying = self
            .connections
            .iter()
            .filter(|connection| {
                &connection.group_id == group_id && !source_ids.contains(&connection.id)
            })
            .cloned()
            .collect::<Vec<_>>();
        staying.sort_by_key(|connection| connection.sort_order);

        for connection in self
            .connections
            .iter()
            .filter(|connection| source_ids.contains(&connection.id))
        {
            let mut moved = connection.clone();
            moved.group_id = group_id.clone();
            staying.push(moved);
        }
        staying
    }

    pub(super) fn group_is_descendant(&self, candidate_id: &str, ancestor_id: &str) -> bool {
        let mut current = Some(candidate_id);
        for _ in 0..=64 {
            let Some(id) = current else {
                return false;
            };
            if id == ancestor_id {
                return true;
            }
            current = self
                .groups
                .iter()
                .find(|group| group.id == id)
                .and_then(|group| group.parent_id.as_deref());
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use nyaterm_core::{AiExecutionProfile, ConnectionType, Group, SavedConnection};

    use super::ConnectionCatalogState;

    fn connection(id: &str, group_id: Option<&str>, sort_order: i32) -> SavedConnection {
        SavedConnection {
            id: id.to_string(),
            name: id.to_string(),
            config: ConnectionType::LocalTerminal {
                shell_path: String::new(),
                shell_args: String::new(),
                working_dir: None,
                ai_execution_profile: AiExecutionProfile::Auto,
                encoding: String::new(),
            },
            group_id: group_id.map(ToOwned::to_owned),
            description: None,
            sort_order,
            icon: None,
            icon_auto_detect: None,
            auth: None,
            recording: None,
            ssh_algorithms: None,
            ssh_profile: Default::default(),
            terminal_type: None,
            sftp: Default::default(),
            network: None,
            post_login: None,
            created_at_ms: None,
            updated_at_ms: None,
            last_used_at_ms: None,
        }
    }

    #[test]
    fn catalog_keeps_runtime_discovery_separate_from_loaded_data() {
        let mut catalog = ConnectionCatalogState::new(Vec::new(), Vec::new());

        assert!(catalog.connections().is_empty());
        assert!(catalog.groups().is_empty());
        assert!(catalog.serial_ports().is_empty());

        catalog.replace_serial_ports(vec!["ttyUSB0".to_string()]);
        catalog.replace_loaded(
            vec![connection("connection-1", None, 0)],
            vec![Group {
                id: "group-1".to_string(),
                name: "Group".to_string(),
                parent_id: None,
                sort_order: 0,
                created_at_ms: None,
                updated_at_ms: None,
            }],
        );
        catalog.clear_loaded();

        assert!(catalog.connections().is_empty());
        assert!(catalog.groups().is_empty());
        assert_eq!(catalog.serial_ports(), ["ttyUSB0"]);
    }

    #[test]
    fn clearing_connections_preserves_groups_and_runtime_discovery() {
        let group = Group {
            id: "group-1".to_string(),
            name: "Group".to_string(),
            parent_id: None,
            sort_order: 0,
            created_at_ms: None,
            updated_at_ms: None,
        };
        let mut catalog = ConnectionCatalogState::new(
            vec![connection("connection-1", Some("group-1"), 0)],
            vec![group.clone()],
        );
        catalog.replace_serial_ports(vec!["ttyUSB0".to_string()]);

        catalog.clear_connections();

        assert!(catalog.connections().is_empty());
        assert_eq!(catalog.groups(), [group]);
        assert_eq!(catalog.serial_ports(), ["ttyUSB0"]);
    }

    #[test]
    fn connection_updates_replace_only_loaded_entries() {
        let mut catalog =
            ConnectionCatalogState::new(vec![connection("connection-1", None, 0)], Vec::new());
        let mut updated = connection("connection-1", Some("group-1"), 3);
        updated.name = "Updated".to_string();

        assert!(catalog.update_connection(updated));
        assert_eq!(catalog.connections()[0].name, "Updated");
        assert_eq!(
            catalog.connections()[0].group_id.as_deref(),
            Some("group-1")
        );
        assert!(!catalog.update_connection(connection("missing", None, 0)));
    }

    #[test]
    fn connection_move_candidates_preserve_catalog_until_persisted() {
        let catalog = ConnectionCatalogState::new(
            vec![
                connection("a", None, 0),
                connection("target-1", Some("target"), 1),
                connection("b", None, 2),
                connection("target-0", Some("target"), 0),
            ],
            Vec::new(),
        );

        let ordered = catalog.connections_reordered_into_group(
            &["b".to_string(), "a".to_string()],
            &Some("target".to_string()),
        );

        assert_eq!(
            ordered
                .iter()
                .map(|connection| connection.id.as_str())
                .collect::<Vec<_>>(),
            ["target-0", "target-1", "a", "b"]
        );
        assert!(catalog.connections()[0].group_id.is_none());
    }

    #[test]
    fn descendant_checks_stop_at_cycles_and_find_real_ancestors() {
        let catalog = ConnectionCatalogState::new(
            Vec::new(),
            vec![
                Group {
                    id: "parent".to_string(),
                    name: "Parent".to_string(),
                    parent_id: None,
                    sort_order: 0,
                    created_at_ms: None,
                    updated_at_ms: None,
                },
                Group {
                    id: "child".to_string(),
                    name: "Child".to_string(),
                    parent_id: Some("parent".to_string()),
                    sort_order: 0,
                    created_at_ms: None,
                    updated_at_ms: None,
                },
                Group {
                    id: "cycle-a".to_string(),
                    name: "Cycle A".to_string(),
                    parent_id: Some("cycle-b".to_string()),
                    sort_order: 0,
                    created_at_ms: None,
                    updated_at_ms: None,
                },
                Group {
                    id: "cycle-b".to_string(),
                    name: "Cycle B".to_string(),
                    parent_id: Some("cycle-a".to_string()),
                    sort_order: 0,
                    created_at_ms: None,
                    updated_at_ms: None,
                },
            ],
        );

        assert!(catalog.group_is_descendant("child", "parent"));
        assert!(!catalog.group_is_descendant("parent", "child"));
        assert!(!catalog.group_is_descendant("cycle-a", "missing"));
    }
}

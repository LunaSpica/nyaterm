use std::collections::HashSet;
use std::time::Instant;

use gpui::FocusHandle;
use nyaterm_core::{AppSettingsSummary, Group};

use super::connections::ConnectionDropTarget;
use crate::models::{
    ConnectionContextMenuState, ConnectionGroupContextMenuState, ConnectionGroupOpenConfirmState,
    ConnectionImportSource, ConnectionSortMode,
};

pub(in crate::features) struct ConnectionListFeatureState {
    pub search_draft: String,
    pub search_focus: FocusHandle,
    pub sort_mode: ConnectionSortMode,
    pub more_menu_open: bool,
    pub clear_all_confirm_open: bool,
    pub context_menu: Option<ConnectionContextMenuState>,
    pub group_context_menu: Option<ConnectionGroupContextMenuState>,
    pub hovered_connection_id: Option<String>,
    pub hover_pending: Option<(String, Instant)>,
    pub drop_target: Option<ConnectionDropTarget>,
    pub hovered_group_id: Option<String>,
    pub expanded_group_ids: HashSet<String>,
    pub group_open_confirm: Option<ConnectionGroupOpenConfirmState>,
    pub group_open_confirm_focus: FocusHandle,
    pub import_dialog_open: bool,
    pub import_path_prompt: Option<ConnectionImportSource>,
    pub import_focus: FocusHandle,
    pub selected_ids: HashSet<String>,
    pub last_selected_id: Option<String>,
}

impl ConnectionListFeatureState {
    pub fn new(
        settings: &AppSettingsSummary,
        groups: &[Group],
        search_focus: FocusHandle,
        group_open_confirm_focus: FocusHandle,
        import_focus: FocusHandle,
    ) -> Self {
        Self {
            search_draft: String::new(),
            search_focus,
            sort_mode: ConnectionSortMode::from_setting(&settings.ui_saved_connections_sort_mode),
            more_menu_open: false,
            clear_all_confirm_open: false,
            context_menu: None,
            group_context_menu: None,
            hovered_connection_id: None,
            hover_pending: None,
            drop_target: None,
            hovered_group_id: None,
            expanded_group_ids: expanded_group_ids(groups),
            group_open_confirm: None,
            group_open_confirm_focus,
            import_dialog_open: false,
            import_path_prompt: None,
            import_focus,
            selected_ids: HashSet::new(),
            last_selected_id: None,
        }
    }

    pub fn clear_selection(&mut self) {
        self.selected_ids.clear();
        self.last_selected_id = None;
    }
}

fn expanded_group_ids(groups: &[Group]) -> HashSet<String> {
    groups.iter().map(|group| group.id.clone()).collect()
}

#[cfg(test)]
mod tests {
    use super::expanded_group_ids;
    use nyaterm_core::Group;

    #[test]
    fn expanded_group_ids_include_all_loaded_groups() {
        let groups = vec![
            Group {
                id: "root".to_string(),
                name: "Root".to_string(),
                parent_id: None,
                sort_order: 0,
                created_at_ms: Some(10),
                updated_at_ms: Some(20),
            },
            Group {
                id: "child".to_string(),
                name: "Child".to_string(),
                parent_id: Some("root".to_string()),
                sort_order: 1,
                created_at_ms: Some(30),
                updated_at_ms: Some(40),
            },
        ];

        let expanded = expanded_group_ids(&groups);

        assert!(expanded.contains("root"));
        assert!(expanded.contains("child"));
        assert_eq!(expanded.len(), 2);
    }
}

use std::collections::{HashMap, HashSet};

use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Pixels, ScrollHandle, SharedString,
    Subscription, WindowHandle,
};
use nyaterm_core::{AppSettingsSummary, ConnectionType, Group, SavedConnection};

use super::catalog::ConnectionCatalogState;
use super::connection_runtime::{ConnectionEditorToggle, ConnectionEditorWindow};
use super::interaction::{ConnectionDragKind, ConnectionDropPosition, ConnectionDropTarget};
use crate::features::NyaTermApp;
use crate::models::{
    ConnectionContextMenuState, ConnectionDeleteConfirmState, ConnectionEditorAdvancedTab,
    ConnectionEditorField, ConnectionEditorMenu, ConnectionEditorPasswordSource,
    ConnectionEditorState, ConnectionEditorTelnetTab, ConnectionGroupContextMenuState,
    ConnectionGroupDeleteConfirmState, ConnectionGroupEditorState, ConnectionGroupOpenConfirmState,
    ConnectionImportSource, ConnectionKindTab, ConnectionListContextMenuState, ConnectionSortMode,
    NetworkDeleteConfirmState, NetworkGroupDeleteConfirmState, NetworkGroupEditorState,
    NetworkItemMenuState, NetworkMovePickerState, NetworkProxyEditorField, NetworkProxyEditorState,
    NetworkTab, NetworkTunnelEditorField, NetworkTunnelEditorState,
};
use nyaterm_ui::{TextField, TextFieldEvent};

mod editor_logic;
mod list_logic;
mod network_logic;

use self::editor_logic::{
    advance_connection_editor_focus, apply_connection_editor_shell_path,
    apply_connection_editor_text_key, apply_connection_editor_working_dir,
    apply_connection_group_editor_name_key, clear_connection_editor_group_menu_draft,
    clear_connection_editor_runtime_state, commit_connection_editor_new_group,
    connection_editor_inline_panel_draft, connection_editor_window_open_or_pending,
    editor_field_seeds, insert_connection_editor_description_newline,
    select_saved_connection_after_editor_save, set_connection_editor_advanced_tab,
    set_connection_editor_error, set_connection_editor_field_text, set_connection_editor_icon,
    set_connection_editor_icon_auto_detect, set_connection_editor_kind,
    set_connection_editor_menu_value, set_connection_editor_password_source,
    set_connection_editor_telnet_tab, set_connection_group_editor_error,
    toggle_connection_editor_flag,
};
use self::list_logic::{
    clear_connection_list_runtime_state, clear_selected_connection_ids, close_connection_more_menu,
    connection_drop_position_for_target, cycle_connection_sort_mode,
    remove_connection_list_references, remove_group_list_references,
    retain_loaded_connection_references, retain_loaded_group_list_references,
    saved_connections_in_group_tree_for_list_state, select_connection_ids,
    selected_connections_for_list_state, set_connection_drop_target_if_changed,
    set_connection_group_hover, sync_connection_search_expansion,
    visible_connection_ids_for_list_state,
};
use self::network_logic::{
    clear_network_proxy_editor, clear_network_tunnel_editor, cycle_network_proxy_group,
    cycle_network_proxy_protocol, cycle_network_tunnel_connection, cycle_network_tunnel_group,
    cycle_network_tunnel_type, remove_network_group_references, remove_network_item_references,
    set_network_group_editor_error, set_network_group_editor_name, set_network_proxy_editor_error,
    set_network_proxy_editor_field, set_network_tunnel_bind_localhost,
    set_network_tunnel_editor_error, set_network_tunnel_editor_field,
    toggle_network_item_menu_state, toggle_network_move_picker_state,
    toggle_network_tunnel_auto_open,
};

pub(in crate::features) struct ConnectionFeatureState {
    catalog: ConnectionCatalogState,
    list: ConnectionListState,
    import: ConnectionImportState,
    editor: ConnectionEditorFeatureState,
    group_editor: ConnectionGroupEditorFeatureState,
    confirmations: ConnectionConfirmationState,
    network: NetworkFeatureState,
}

pub(in crate::features) struct ConnectionFeatureFocus {
    /// Placeholder for the filter box, resolved by the caller so this struct
    /// stays free of the i18n lookup.
    pub filter_placeholder: SharedString,
    pub import: FocusHandle,
    pub editor: FocusHandle,
    pub group_editor: FocusHandle,
    pub group_open_confirm: FocusHandle,
    pub network_tunnel_editor: FocusHandle,
    pub network_proxy_editor: FocusHandle,
}

struct ConnectionListState {
    /// The editable field. It owns the caret, selection and composition; this
    /// struct only caches what it last reported so filtering stays synchronous.
    search_field: Entity<TextField>,
    search_draft: String,
    /// Kept alive for as long as the field is, so edits keep arriving.
    _search_subscription: Subscription,
    sort_mode: ConnectionSortMode,
    more_menu_open: bool,
    context_menu: Option<ConnectionContextMenuState>,
    group_context_menu: Option<ConnectionGroupContextMenuState>,
    list_context_menu: Option<ConnectionListContextMenuState>,
    /// The "move to group" flyout hanging off whichever context menu is open.
    move_submenu_open: bool,
    /// Row the arrow keys are currently on while filtering. Distinct from the
    /// selection: walking results must not clobber a multi-select.
    keyboard_active_connection_id: Option<String>,
    drop_target: Option<ConnectionDropTarget>,
    hovered_group_id: Option<String>,
    expanded_group_ids: HashSet<String>,
    /// Expansion to restore once the filter box empties again.
    search_expanded_base: Option<HashSet<String>>,
    /// Filter text the auto-expand has already been applied for.
    search_applied_query: Option<String>,
    selected_ids: HashSet<String>,
    last_selected_id: Option<String>,
}

struct ConnectionImportState {
    import_dialog_open: bool,
    import_path_prompt: Option<ConnectionImportSource>,
    import_focus: FocusHandle,
}

struct ConnectionEditorFeatureState {
    draft: Option<ConnectionEditorState>,
    /// One editable field per text input, built when the editor opens.
    ///
    /// The draft above stays the source of truth for saving; these own the
    /// caret, selection and composition, and write back through their
    /// subscriptions. Keeping them out of `ConnectionEditorState` keeps that
    /// model a plain value the runtime can clone.
    fields: HashMap<ConnectionEditorField, Entity<TextField>>,
    field_subscriptions: Vec<Subscription>,
    window: Option<WindowHandle<ConnectionEditorWindow>>,
    window_open_pending: bool,
    focus: FocusHandle,
    icon_picker_open: bool,
    menu: Option<ConnectionEditorMenu>,
    /// The open popover claims focus so it can own the arrow keys; without a
    /// handle of its own the keys would reach whichever field was last focused.
    menu_focus: FocusHandle,
    /// Which option the keyboard is on, as an index into the open menu's list.
    menu_highlight: usize,
    /// Lets the highlight scroll itself into view in a long list.
    menu_scroll: ScrollHandle,
}

struct ConnectionGroupEditorFeatureState {
    draft: Option<ConnectionGroupEditorState>,
    /// The folder-name input, built with the draft it mirrors.
    field: Option<Entity<TextField>>,
    field_subscription: Option<Subscription>,
    focus: FocusHandle,
}

struct ConnectionConfirmationState {
    clear_all_open: bool,
    delete: Option<ConnectionDeleteConfirmState>,
    group_delete: Option<ConnectionGroupDeleteConfirmState>,
    group_open: Option<ConnectionGroupOpenConfirmState>,
    group_open_focus: FocusHandle,
}

struct NetworkFeatureState {
    tab: NetworkTab,
    delete_confirm: Option<NetworkDeleteConfirmState>,
    group_editor: Option<NetworkGroupEditorState>,
    group_delete_confirm: Option<NetworkGroupDeleteConfirmState>,
    item_menu: Option<NetworkItemMenuState>,
    move_picker: Option<NetworkMovePickerState>,
    expanded_sections: HashSet<String>,
    tunnel_editor: Option<NetworkTunnelEditorState>,
    proxy_editor: Option<NetworkProxyEditorState>,
    tunnel_editor_focus: FocusHandle,
    proxy_editor_focus: FocusHandle,
}

impl ConnectionFeatureState {
    pub fn new(
        connections: Vec<SavedConnection>,
        groups: Vec<Group>,
        settings: &AppSettingsSummary,
        focus: ConnectionFeatureFocus,
        cx: &mut Context<NyaTermApp>,
    ) -> Self {
        let filter_placeholder = focus.filter_placeholder;
        let search_field =
            cx.new(|cx| TextField::new(cx, String::new()).placeholder(filter_placeholder));
        // The field owns the text; the panel only needs to know when it changed.
        let search_subscription = cx.subscribe(
            &search_field,
            |app: &mut NyaTermApp, _, event: &TextFieldEvent, cx| {
                let TextFieldEvent::Changed(text) = event;
                app.connection_state.set_list_search_text(text.clone());
                app.sync_connection_keyboard_active(cx);
                cx.notify();
            },
        );
        Self {
            catalog: ConnectionCatalogState::new(connections, groups),
            list: ConnectionListState {
                search_field,
                search_draft: String::new(),
                _search_subscription: search_subscription,
                sort_mode: ConnectionSortMode::from_setting(
                    &settings.ui_saved_connections_sort_mode,
                ),
                more_menu_open: false,
                context_menu: None,
                group_context_menu: None,
                list_context_menu: None,
                move_submenu_open: false,
                keyboard_active_connection_id: None,
                drop_target: None,
                hovered_group_id: None,
                // Tauri opens the panel with every folder closed; the tree is
                // long enough that seeding it expanded buries the folder list.
                expanded_group_ids: HashSet::new(),
                search_expanded_base: None,
                search_applied_query: None,
                selected_ids: HashSet::new(),
                last_selected_id: None,
            },
            import: ConnectionImportState {
                import_dialog_open: false,
                import_path_prompt: None,
                import_focus: focus.import,
            },
            editor: ConnectionEditorFeatureState {
                draft: None,
                fields: HashMap::new(),
                field_subscriptions: Vec::new(),
                window: None,
                window_open_pending: false,
                focus: focus.editor,
                icon_picker_open: false,
                menu: None,
                menu_focus: cx.focus_handle(),
                menu_highlight: 0,
                menu_scroll: ScrollHandle::new(),
            },
            group_editor: ConnectionGroupEditorFeatureState {
                draft: None,
                field: None,
                field_subscription: None,
                focus: focus.group_editor,
            },
            confirmations: ConnectionConfirmationState {
                clear_all_open: false,
                delete: None,
                group_delete: None,
                group_open: None,
                group_open_focus: focus.group_open_confirm,
            },
            network: NetworkFeatureState {
                tab: NetworkTab::Tunnels,
                delete_confirm: None,
                group_editor: None,
                group_delete_confirm: None,
                item_menu: None,
                move_picker: None,
                expanded_sections: HashSet::new(),
                tunnel_editor: None,
                proxy_editor: None,
                tunnel_editor_focus: focus.network_tunnel_editor,
                proxy_editor_focus: focus.network_proxy_editor,
            },
        }
    }

    pub fn connections(&self) -> &[SavedConnection] {
        self.catalog.connections()
    }

    pub fn groups(&self) -> &[Group] {
        self.catalog.groups()
    }

    pub fn serial_ports(&self) -> &[String] {
        self.catalog.serial_ports()
    }

    pub fn replace_loaded(&mut self, connections: Vec<SavedConnection>, groups: Vec<Group>) {
        self.catalog.replace_loaded(connections, groups);
        self.retain_list_references_for_catalog();
    }

    pub fn clear_loaded(&mut self) {
        self.catalog.clear_loaded();
        self.retain_list_references_for_catalog();
    }

    pub fn clear_connections(&mut self) {
        self.catalog.clear_connections();
        self.retain_list_references_for_catalog();
    }

    pub fn replace_connections(&mut self, connections: Vec<SavedConnection>) {
        self.catalog.replace_connections(connections);
        self.retain_list_references_for_catalog();
    }

    pub fn replace_serial_ports(&mut self, serial_ports: Vec<String>) {
        self.catalog.replace_serial_ports(serial_ports);
    }

    pub fn update_connection(&mut self, updated: SavedConnection) -> bool {
        self.catalog.update_connection(updated)
    }

    pub fn connections_reordered_into_group(
        &self,
        source_ids: &[String],
        group_id: &Option<String>,
    ) -> Vec<SavedConnection> {
        self.catalog
            .connections_reordered_into_group(source_ids, group_id)
    }

    pub fn group_is_descendant(&self, candidate_id: &str, ancestor_id: &str) -> bool {
        self.catalog.group_is_descendant(candidate_id, ancestor_id)
    }

    fn retain_list_references_for_catalog(&mut self) {
        let connection_ids = self
            .catalog
            .connections()
            .iter()
            .map(|connection| connection.id.clone())
            .collect::<HashSet<_>>();
        let group_ids = self
            .catalog
            .groups()
            .iter()
            .map(|group| group.id.clone())
            .collect::<HashSet<_>>();
        self.list
            .retain_loaded_references(&connection_ids, &group_ids);
    }

    pub fn list_search_query(&self) -> String {
        self.list.search_query()
    }

    pub fn list_search_is_empty(&self) -> bool {
        self.list.search_is_empty()
    }

    pub fn list_search_field(&self) -> Entity<TextField> {
        self.list.search_field()
    }

    pub fn set_list_search_text(&mut self, text: String) {
        self.list.set_search_text(text);
    }

    pub fn list_sort_mode(&self) -> ConnectionSortMode {
        self.list.sort_mode()
    }

    pub fn list_more_menu_is_open(&self) -> bool {
        self.list.more_menu_is_open()
    }

    pub fn list_has_selection(&self) -> bool {
        self.list.has_selection()
    }

    pub fn list_contains_selected_id(&self, connection_id: &str) -> bool {
        self.list.contains_selected_id(connection_id)
    }

    pub fn selected_connections(&self) -> Vec<SavedConnection> {
        selected_connections_for_list_state(self.catalog.connections(), &self.list.selected_ids)
    }

    pub fn saved_connections_in_group_tree(&self, group_id: &str) -> Vec<SavedConnection> {
        saved_connections_in_group_tree_for_list_state(
            self.catalog.connections(),
            self.catalog.groups(),
            group_id,
        )
    }

    pub fn visible_connection_ids(&self) -> Vec<String> {
        visible_connection_ids_for_list_state(
            self.catalog.connections(),
            self.catalog.groups(),
            &self.list.search_query(),
            self.list.sort_mode,
            &self.list.expanded_group_ids,
        )
    }

    pub fn list_context_menu_is_open(&self) -> bool {
        self.list.context_menu_is_open()
    }

    pub fn active_list_connection_context_menu(&self) -> Option<ConnectionContextMenuState> {
        self.list.active_context_menu()
    }

    pub fn list_group_context_menu_is_open(&self) -> bool {
        self.list.group_context_menu_is_open()
    }

    pub fn active_list_group_context_menu(&self) -> Option<ConnectionGroupContextMenuState> {
        self.list.active_group_context_menu()
    }

    pub fn list_background_context_menu_is_open(&self) -> bool {
        self.list.list_context_menu_is_open()
    }

    pub fn active_list_background_context_menu(&self) -> Option<ConnectionListContextMenuState> {
        self.list.active_list_context_menu()
    }

    pub fn list_expanded_group_ids(&self) -> &HashSet<String> {
        self.list.expanded_group_ids()
    }

    pub fn list_group_is_expanded(&self, group_id: Option<&str>) -> bool {
        self.list.group_is_expanded(group_id)
    }

    pub fn list_group_is_hovered(&self, group_id: Option<&str>) -> bool {
        self.list.group_is_hovered(group_id)
    }

    pub fn list_drop_position_for_kind_target(
        &self,
        kind: ConnectionDragKind,
        target_id: Option<&str>,
    ) -> Option<ConnectionDropPosition> {
        self.list.drop_position_for_kind_target(kind, target_id)
    }

    pub fn select_list_connection(
        &mut self,
        connection_id: String,
        visible_ids: &[String],
        additive: bool,
        range: bool,
    ) -> usize {
        self.list
            .select_connection(connection_id, visible_ids, additive, range)
    }

    pub fn clear_list_selection(&mut self) {
        self.list.clear_selection();
    }

    pub fn toggle_list_more_menu(&mut self) {
        self.list.toggle_more_menu();
    }

    pub fn close_list_more_menu(&mut self) -> bool {
        self.list.close_more_menu()
    }

    pub fn cycle_list_sort_mode(&mut self) -> ConnectionSortMode {
        self.list.cycle_sort_mode()
    }

    pub fn set_list_group_hover(&mut self, group_id: String, hovered: bool) -> bool {
        self.list.set_group_hover(group_id, hovered)
    }

    pub fn list_keyboard_active_connection_id(&self) -> Option<&str> {
        self.list.keyboard_active_connection_id()
    }

    pub fn list_connection_is_keyboard_active(&self, connection_id: &str) -> bool {
        self.list.connection_is_keyboard_active(connection_id)
    }

    pub fn set_list_keyboard_active_connection_id(&mut self, connection_id: Option<String>) {
        self.list.set_keyboard_active_connection_id(connection_id);
    }

    pub fn list_move_submenu_is_open(&self) -> bool {
        self.list.move_submenu_is_open()
    }

    pub fn toggle_list_move_submenu(&mut self) {
        self.list.toggle_move_submenu();
    }

    pub fn open_list_background_context_menu(&mut self, x: Pixels, y: Pixels) {
        self.list.open_list_context_menu(x, y);
    }

    pub fn open_list_connection_context_menu(
        &mut self,
        connection_id: String,
        x: Pixels,
        y: Pixels,
    ) {
        self.list.open_context_menu(connection_id, x, y);
    }

    pub fn open_list_group_context_menu(&mut self, group_id: String, x: Pixels, y: Pixels) {
        self.list.open_group_context_menu(group_id, x, y);
    }

    pub fn close_list_context_menus(&mut self) {
        self.list.close_context_menus();
    }

    pub fn toggle_list_group_expanded(&mut self, group_id: String) -> bool {
        self.list.toggle_group_expanded(group_id)
    }

    pub fn expand_list_group(&mut self, group_id: String) {
        self.list.expand_group(group_id);
    }

    pub fn expand_all_catalog_groups(&mut self) {
        let group_ids = self
            .catalog
            .groups()
            .iter()
            .map(|group| group.id.clone())
            .collect::<Vec<_>>();
        self.list.expand_groups(group_ids);
    }

    pub fn sync_list_search_expansion(
        &mut self,
        query: &str,
        matching_group_ids: impl IntoIterator<Item = String>,
    ) -> bool {
        self.list.sync_search_expansion(query, matching_group_ids)
    }

    pub fn set_list_drop_target_if_changed(&mut self, target: ConnectionDropTarget) -> bool {
        self.list.set_drop_target_if_changed(target)
    }

    pub fn list_drop_position_for_target(
        &self,
        target_id: &str,
        fallback: ConnectionDropPosition,
    ) -> ConnectionDropPosition {
        self.list.drop_position_for_target(target_id, fallback)
    }

    pub fn clear_list_drop_target(&mut self) {
        self.list.clear_drop_target();
    }

    pub fn clear_list_runtime_state(&mut self) {
        self.list.clear_runtime_state();
    }

    pub fn remove_list_connection_references(&mut self, connection_id: &str) {
        self.list.remove_connection_references(connection_id);
    }

    pub fn remove_list_group_references(&mut self, group_id: &str) {
        self.list.remove_group_references(group_id);
    }

    /// The editor's fields, for handing to the render sections.
    pub fn editor_fields(&self) -> &HashMap<ConnectionEditorField, Entity<TextField>> {
        &self.editor.fields
    }

    /// Build a field per input and wire each back into the draft.
    ///
    /// Called when the editor opens, so the entities live exactly as long as the
    /// draft they mirror and never leak between edits.
    pub fn build_editor_fields(&mut self, cx: &mut Context<NyaTermApp>) {
        self.editor.fields.clear();
        self.editor.field_subscriptions.clear();
        let Some(draft) = self.editor.draft.as_ref() else {
            return;
        };
        for (field, value, masked, placeholder) in editor_field_seeds(draft) {
            let entity = cx.new(|cx| {
                TextField::new(cx, value)
                    .masked(masked)
                    .placeholder(placeholder)
                    // The description is the one box that takes newlines.
                    .multi_line(field == ConnectionEditorField::Description)
            });
            let subscription = cx.subscribe(&entity, move |app: &mut NyaTermApp, _, event, cx| {
                let TextFieldEvent::Changed(text) = event;
                app.apply_connection_editor_field_text(field, text.clone(), cx);
            });
            self.editor.fields.insert(field, entity);
            self.editor.field_subscriptions.push(subscription);
        }
    }

    /// Push a value the runtime changed back down into its field.
    ///
    /// Edits normally flow field → draft; this is the other direction, for the
    /// cases where the runtime consumes what was typed — committing a new folder
    /// empties the box it was typed into.
    pub fn reset_editor_field(&mut self, field: ConnectionEditorField, text: &str, cx: &mut App) {
        if let Some(entity) = self.editor.fields.get(&field) {
            entity.update(cx, |entity, cx| entity.set_content(text, cx));
        }
    }

    /// Push every draft value back into its field.
    ///
    /// For the changes the runtime makes on the draft's behalf — switching the
    /// connection kind rewrites the port — where the boxes would otherwise keep
    /// showing what the draft no longer says. `set_content` is a no-op when the
    /// text already matches, so this cannot disturb what is being typed.
    pub fn sync_editor_fields_from_draft(&mut self, cx: &mut App) {
        let Some(draft) = self.editor.draft.as_ref() else {
            return;
        };
        for (field, value, _, _) in editor_field_seeds(draft) {
            if let Some(entity) = self.editor.fields.get(&field) {
                entity.update(cx, |entity, cx| entity.set_content(value, cx));
            }
        }
    }

    pub fn set_editor_field_text(&mut self, field: ConnectionEditorField, text: String) {
        if let Some(draft) = self.editor.draft.as_mut() {
            set_connection_editor_field_text(draft, field, text);
        }
    }

    pub fn begin_editor(&mut self, draft: ConnectionEditorState) {
        self.editor.begin_edit(draft);
    }

    pub fn active_editor_draft(&self) -> Option<ConnectionEditorState> {
        self.editor.active_draft()
    }

    pub fn active_editor_menu(&self) -> Option<ConnectionEditorMenu> {
        self.editor.active_menu()
    }

    pub fn editor_icon_picker_is_open(&self) -> bool {
        self.editor.icon_picker_is_open()
    }

    pub fn editor_menu_is_open(&self) -> bool {
        self.editor.menu_is_open()
    }

    pub fn editor_description_is_focused(&self) -> bool {
        self.editor.description_is_focused()
    }

    pub fn editor_new_group_field_is_focused(&self, cx: &App) -> bool {
        self.editor.new_group_field_is_focused(cx)
    }

    pub fn editor_new_group_name_focused_in_group_menu(&self) -> bool {
        self.editor.new_group_name_focused_in_group_menu()
    }

    pub fn inline_editor_panel_draft(&self) -> Option<ConnectionEditorState> {
        self.editor.inline_panel_draft()
    }

    pub fn editor_is_editing_saved_connection(&self) -> bool {
        self.editor.is_editing_saved_connection()
    }

    pub fn editor_has_draft(&self) -> bool {
        self.editor.has_draft()
    }

    pub fn editor_focus_handle(&self) -> FocusHandle {
        self.editor.focus_handle()
    }

    pub(in crate::features::connections) fn editor_window_handle(
        &self,
    ) -> Option<WindowHandle<ConnectionEditorWindow>> {
        self.editor.window_handle()
    }

    pub fn editor_has_window(&self) -> bool {
        self.editor.has_window()
    }

    pub fn editor_window_open_pending(&self) -> bool {
        self.editor.window_open_pending()
    }

    pub fn editor_modal_window_open_or_pending(&self) -> bool {
        self.editor.modal_window_open_or_pending()
    }

    pub fn close_editor_popovers(&mut self) {
        self.editor.close_popovers();
    }

    pub fn toggle_editor_icon_picker(&mut self) {
        self.editor.toggle_icon_picker();
    }

    pub fn toggle_editor_menu(&mut self, menu: ConnectionEditorMenu) {
        self.editor.toggle_menu(menu);
    }

    pub fn editor_menu_focus_handle(&self) -> FocusHandle {
        self.editor.menu_focus_handle()
    }

    pub fn editor_menu_scroll_handle(&self) -> ScrollHandle {
        self.editor.menu_scroll_handle()
    }

    pub fn editor_menu_highlight(&self) -> usize {
        self.editor.menu_highlight()
    }

    pub fn set_editor_menu_highlight(&mut self, index: usize) {
        self.editor.set_menu_highlight(index);
    }

    pub fn step_editor_menu_highlight(&mut self, delta: isize, len: usize) {
        self.editor.step_menu_highlight(delta, len);
    }

    pub fn set_editor_icon(&mut self, icon: Option<&str>) -> bool {
        self.editor.set_icon(icon)
    }

    pub fn set_editor_icon_auto_detect(&mut self, enabled: bool) -> bool {
        self.editor.set_icon_auto_detect(enabled)
    }

    pub fn set_editor_menu_value(
        &mut self,
        menu: ConnectionEditorMenu,
        value: Option<String>,
    ) -> bool {
        self.editor.set_menu_value(menu, value)
    }

    pub fn set_editor_password_source(&mut self, source: ConnectionEditorPasswordSource) -> bool {
        self.editor.set_password_source(source)
    }

    pub fn set_editor_advanced_tab(&mut self, tab: ConnectionEditorAdvancedTab) -> bool {
        self.editor.set_advanced_tab(tab)
    }

    pub fn set_editor_telnet_tab(&mut self, tab: ConnectionEditorTelnetTab) -> bool {
        self.editor.set_telnet_tab(tab)
    }

    pub fn set_editor_kind(&mut self, kind: ConnectionKindTab) -> bool {
        self.editor.set_kind(kind)
    }

    pub fn commit_editor_new_group(&mut self, required_message: String) -> bool {
        self.editor.commit_new_group(required_message)
    }

    pub fn toggle_editor_flag(&mut self, flag: ConnectionEditorToggle) -> bool {
        self.editor.toggle_flag(flag)
    }

    pub fn insert_editor_description_newline(&mut self) -> bool {
        self.editor.insert_description_newline()
    }

    pub fn apply_editor_text_key(&mut self, key: &str, input: Option<&str>) -> bool {
        self.editor.apply_text_key(key, input)
    }

    pub fn advance_editor_focus(&mut self) -> bool {
        self.editor.advance_focus()
    }

    pub fn close_editor_popovers_and_cancel_group_draft(&mut self) {
        self.editor.close_popovers_and_cancel_group_draft();
    }

    pub fn set_editor_error(&mut self, error: String) -> bool {
        self.editor.set_error(error)
    }

    pub fn apply_editor_shell_path(&mut self, shell_path: String) -> bool {
        self.editor.apply_shell_path(shell_path)
    }

    pub fn apply_editor_working_dir(&mut self, working_dir: String) -> bool {
        self.editor.apply_working_dir(working_dir)
    }

    pub fn close_editor(&mut self) {
        self.editor.close();
    }

    pub(in crate::features::connections) fn clear_editor_window_if_current(
        &mut self,
        window: WindowHandle<ConnectionEditorWindow>,
    ) -> bool {
        self.editor.clear_window_if_current(window)
    }

    pub fn mark_editor_window_pending(&mut self) {
        self.editor.mark_window_pending();
    }

    pub fn clear_editor_window_pending(&mut self) {
        self.editor.clear_window_pending();
    }

    pub(in crate::features::connections) fn attach_editor_window(
        &mut self,
        window: WindowHandle<ConnectionEditorWindow>,
    ) {
        self.editor.attach_window(window);
    }

    pub fn clear_editor_window(&mut self) {
        self.editor.clear_window();
    }

    pub fn group_editor_field(&self) -> Option<Entity<TextField>> {
        self.group_editor.field.clone()
    }

    pub fn build_group_editor_field(&mut self, cx: &mut Context<NyaTermApp>) {
        let Some(draft) = self.group_editor.draft.as_ref() else {
            self.group_editor.field = None;
            self.group_editor.field_subscription = None;
            return;
        };
        let entity = cx.new(|cx| TextField::new(cx, draft.name.clone()));
        let subscription = cx.subscribe(&entity, |app: &mut NyaTermApp, _, event, cx| {
            let TextFieldEvent::Changed(text) = event;
            app.connection_state.set_group_editor_name(text.clone());
            cx.notify();
        });
        self.group_editor.field = Some(entity);
        self.group_editor.field_subscription = Some(subscription);
    }

    pub fn clear_group_editor_field(&mut self) {
        self.group_editor.field = None;
        self.group_editor.field_subscription = None;
    }

    pub fn set_group_editor_name(&mut self, name: String) {
        if let Some(draft) = self.group_editor.draft.as_mut() {
            draft.name = name;
            draft.error = None;
        }
    }

    pub fn active_group_editor_draft(&self) -> Option<ConnectionGroupEditorState> {
        self.group_editor.active_draft()
    }

    pub fn group_editor_focus_handle(&self) -> FocusHandle {
        self.group_editor.focus_handle()
    }

    pub fn begin_group_editor(&mut self, draft: ConnectionGroupEditorState) {
        self.group_editor.begin_edit(draft);
    }

    pub fn apply_group_editor_name_key(&mut self, key: &str, input: Option<&str>) -> bool {
        self.group_editor.apply_name_key(key, input)
    }

    pub fn set_group_editor_error(&mut self, error: String) -> bool {
        self.group_editor.set_error(error)
    }

    pub fn close_group_editor(&mut self) {
        self.group_editor.close();
    }

    pub fn open_clear_all(&mut self) {
        self.confirmations.open_clear_all();
    }

    pub fn close_clear_all(&mut self) {
        self.confirmations.close_clear_all();
    }

    pub fn clear_all_is_open(&self) -> bool {
        self.confirmations.clear_all_is_open()
    }

    pub fn active_delete_confirm(&self) -> Option<ConnectionDeleteConfirmState> {
        self.confirmations.active_delete()
    }

    pub fn open_delete_confirm(&mut self, confirm: ConnectionDeleteConfirmState) {
        self.confirmations.open_delete(confirm);
    }

    pub fn close_delete_confirm(&mut self) {
        self.confirmations.close_delete();
    }

    pub fn take_delete_confirm(&mut self) -> Option<ConnectionDeleteConfirmState> {
        self.confirmations.take_delete()
    }

    pub fn active_group_delete_confirm(&self) -> Option<ConnectionGroupDeleteConfirmState> {
        self.confirmations.active_group_delete()
    }

    pub fn open_group_delete_confirm(&mut self, confirm: ConnectionGroupDeleteConfirmState) {
        self.confirmations.open_group_delete(confirm);
    }

    pub fn close_group_delete_confirm(&mut self) {
        self.confirmations.close_group_delete();
    }

    pub fn take_group_delete_confirm(&mut self) -> Option<ConnectionGroupDeleteConfirmState> {
        self.confirmations.take_group_delete()
    }

    pub fn active_group_open_confirm(&self) -> Option<ConnectionGroupOpenConfirmState> {
        self.confirmations.active_group_open()
    }

    pub fn open_group_open_confirm(&mut self, confirm: ConnectionGroupOpenConfirmState) {
        self.confirmations.open_group_open(confirm);
    }

    pub fn close_group_open_confirm(&mut self) {
        self.confirmations.close_group_open();
    }

    pub fn take_group_open_confirm(&mut self) -> Option<ConnectionGroupOpenConfirmState> {
        self.confirmations.take_group_open()
    }

    pub fn group_open_focus_handle(&self) -> FocusHandle {
        self.confirmations.group_open_focus_handle()
    }

    pub fn network_active_tab(&self) -> NetworkTab {
        self.network.active_tab()
    }

    pub fn network_tab_is(&self, tab: NetworkTab) -> bool {
        self.network.tab_is(tab)
    }

    pub fn network_section_is_expanded(&self, section_key: &str) -> bool {
        self.network.section_is_expanded(section_key)
    }

    pub fn network_item_menu_is_open(&self, tab: NetworkTab, id: &str) -> bool {
        self.network.item_menu_is_open(tab, id)
    }

    pub fn network_move_picker_is_open(&self, tab: NetworkTab, id: &str) -> bool {
        self.network.move_picker_is_open(tab, id)
    }

    pub fn active_network_delete_confirm(&self) -> Option<NetworkDeleteConfirmState> {
        self.network.active_delete_confirm()
    }

    pub fn active_network_group_editor(&self) -> Option<NetworkGroupEditorState> {
        self.network.active_group_editor()
    }

    pub fn active_network_group_delete_confirm(&self) -> Option<NetworkGroupDeleteConfirmState> {
        self.network.active_group_delete_confirm()
    }

    pub fn active_network_tunnel_editor(&self) -> Option<NetworkTunnelEditorState> {
        self.network.active_tunnel_editor()
    }

    pub fn active_network_proxy_editor(&self) -> Option<NetworkProxyEditorState> {
        self.network.active_proxy_editor()
    }

    pub fn network_tunnel_editor_focus_handle(&self) -> FocusHandle {
        self.network.tunnel_editor_focus_handle()
    }

    pub fn network_proxy_editor_focus_handle(&self) -> FocusHandle {
        self.network.proxy_editor_focus_handle()
    }

    pub fn set_network_tab(&mut self, tab: NetworkTab) {
        self.network.set_tab(tab);
    }

    pub fn toggle_network_section(&mut self, section_key: String) -> bool {
        self.network.toggle_section(section_key)
    }

    pub fn toggle_network_item_menu(&mut self, tab: NetworkTab, id: String) -> bool {
        self.network.toggle_item_menu(tab, id)
    }

    pub fn toggle_network_move_picker(&mut self, tab: NetworkTab, id: String) -> bool {
        self.network.toggle_move_picker(tab, id)
    }

    pub fn close_network_move_picker(&mut self) {
        self.network.close_move_picker();
    }

    pub fn open_network_delete_confirm(&mut self, confirm: NetworkDeleteConfirmState) {
        self.network.open_delete_confirm(confirm);
    }

    pub fn close_network_delete_confirm(&mut self) {
        self.network.close_delete_confirm();
    }

    pub fn begin_network_group_edit(&mut self, draft: NetworkGroupEditorState) {
        self.network.begin_group_edit(draft);
    }

    pub fn set_network_group_editor_name(&mut self, text: String) -> bool {
        self.network.set_group_editor_name(text)
    }

    pub fn set_network_group_editor_error(&mut self, error: String) -> bool {
        self.network.set_group_editor_error(error)
    }

    pub fn close_network_group_editor(&mut self) {
        self.network.close_group_editor();
    }

    pub fn open_network_group_delete_confirm(&mut self, confirm: NetworkGroupDeleteConfirmState) {
        self.network.open_group_delete_confirm(confirm);
    }

    pub fn close_network_group_delete_confirm(&mut self) {
        self.network.close_group_delete_confirm();
    }

    pub fn begin_network_tunnel_edit(&mut self, draft: NetworkTunnelEditorState) {
        self.network.begin_tunnel_edit(draft);
    }

    pub fn close_network_tunnel_editor(&mut self) {
        self.network.close_tunnel_editor();
    }

    pub fn set_network_tunnel_editor_field(
        &mut self,
        field: NetworkTunnelEditorField,
        text: String,
    ) -> bool {
        self.network.set_tunnel_editor_field(field, text)
    }

    pub fn cycle_network_tunnel_type(&mut self) -> Option<String> {
        self.network.cycle_tunnel_type()
    }

    pub fn cycle_network_tunnel_connection(&mut self) -> bool {
        let connection_ids = self
            .catalog
            .connections()
            .iter()
            .filter(|connection| matches!(&connection.config, ConnectionType::Ssh { .. }))
            .map(|connection| connection.id.clone())
            .collect::<Vec<_>>();
        self.network
            .cycle_tunnel_connection(connection_ids.iter().map(String::as_str))
    }

    pub fn cycle_network_tunnel_group<'a>(
        &mut self,
        group_ids: impl IntoIterator<Item = &'a str>,
    ) -> bool {
        self.network.cycle_tunnel_group(group_ids)
    }

    pub fn set_network_tunnel_bind_localhost(&mut self, bind_localhost: bool) -> bool {
        self.network.set_tunnel_bind_localhost(bind_localhost)
    }

    pub fn toggle_network_tunnel_auto_open(&mut self) -> Option<bool> {
        self.network.toggle_tunnel_auto_open()
    }

    pub fn set_network_tunnel_editor_error(&mut self, error: String) -> bool {
        self.network.set_tunnel_editor_error(error)
    }

    pub fn begin_network_proxy_edit(&mut self, draft: NetworkProxyEditorState) {
        self.network.begin_proxy_edit(draft);
    }

    pub fn close_network_proxy_editor(&mut self) {
        self.network.close_proxy_editor();
    }

    pub fn set_network_proxy_editor_field(
        &mut self,
        field: NetworkProxyEditorField,
        text: String,
    ) -> bool {
        self.network.set_proxy_editor_field(field, text)
    }

    pub fn cycle_network_proxy_protocol(&mut self) -> Option<String> {
        self.network.cycle_proxy_protocol()
    }

    pub fn cycle_network_proxy_group<'a>(
        &mut self,
        group_ids: impl IntoIterator<Item = &'a str>,
    ) -> bool {
        self.network.cycle_proxy_group(group_ids)
    }

    pub fn set_network_proxy_editor_error(&mut self, error: String) -> bool {
        self.network.set_proxy_editor_error(error)
    }

    pub fn remove_network_item_references(&mut self, tab: NetworkTab, id: &str) {
        self.network.remove_item_references(tab, id);
    }

    pub fn remove_network_group_references(
        &mut self,
        tab: NetworkTab,
        group_id: &str,
        deleted_item_ids: &[String],
    ) {
        self.network
            .remove_group_references(tab, group_id, deleted_item_ids);
    }

    pub fn clear_editor_fields(&mut self) {
        self.editor.fields.clear();
        self.editor.field_subscriptions.clear();
    }

    pub fn finish_editor_save(&mut self, connection_id: String, group_id: Option<String>) {
        clear_connection_editor_runtime_state(
            &mut self.editor.draft,
            &mut self.editor.icon_picker_open,
            &mut self.editor.menu,
            &mut self.editor.window,
            &mut self.editor.window_open_pending,
        );
        select_saved_connection_after_editor_save(
            &mut self.list.selected_ids,
            &mut self.list.last_selected_id,
            &mut self.list.expanded_group_ids,
            connection_id,
            group_id,
        );
    }

    pub fn import_dialog_is_open(&self) -> bool {
        self.import.is_dialog_open()
    }

    pub fn import_path_prompt_active(&self) -> bool {
        self.import.path_prompt_active()
    }

    pub fn import_focus_handle(&self) -> FocusHandle {
        self.import.focus_handle()
    }

    pub fn open_import_dialog(&mut self) {
        self.import.open_dialog();
    }

    pub fn close_import_dialog(&mut self) {
        self.import.close_dialog();
    }

    pub fn begin_import_path_prompt(&mut self, source: ConnectionImportSource) {
        self.import.begin_path_prompt(source);
    }

    pub fn finish_import_path_prompt(&mut self) {
        self.import.finish_path_prompt();
    }
}

impl ConnectionListState {
    pub fn search_query(&self) -> String {
        self.search_draft.trim().to_ascii_lowercase()
    }

    pub fn search_is_empty(&self) -> bool {
        self.search_draft.is_empty()
    }

    pub fn search_field(&self) -> Entity<TextField> {
        self.search_field.clone()
    }

    /// Cache what the field just reported. Filtering runs on every keystroke and
    /// from paths without an `App`, so it reads this rather than the entity.
    pub fn set_search_text(&mut self, text: String) {
        self.search_draft = text;
    }

    pub fn sort_mode(&self) -> ConnectionSortMode {
        self.sort_mode
    }

    pub fn more_menu_is_open(&self) -> bool {
        self.more_menu_open
    }

    pub fn has_selection(&self) -> bool {
        !self.selected_ids.is_empty()
    }

    pub fn contains_selected_id(&self, connection_id: &str) -> bool {
        self.selected_ids.contains(connection_id)
    }

    pub fn context_menu_is_open(&self) -> bool {
        self.context_menu.is_some()
    }

    pub fn active_context_menu(&self) -> Option<ConnectionContextMenuState> {
        self.context_menu.clone()
    }

    pub fn group_context_menu_is_open(&self) -> bool {
        self.group_context_menu.is_some()
    }

    pub fn active_group_context_menu(&self) -> Option<ConnectionGroupContextMenuState> {
        self.group_context_menu.clone()
    }

    pub fn expanded_group_ids(&self) -> &HashSet<String> {
        &self.expanded_group_ids
    }

    pub fn group_is_expanded(&self, group_id: Option<&str>) -> bool {
        group_id
            .map(|id| self.expanded_group_ids.contains(id))
            .unwrap_or(true)
    }

    pub fn group_is_hovered(&self, group_id: Option<&str>) -> bool {
        group_id.is_some_and(|id| self.hovered_group_id.as_deref() == Some(id))
    }

    pub fn drop_position_for_kind_target(
        &self,
        kind: ConnectionDragKind,
        target_id: Option<&str>,
    ) -> Option<ConnectionDropPosition> {
        self.drop_target.as_ref().and_then(|target| {
            (target.kind == kind && target.id.as_deref() == target_id).then_some(target.position)
        })
    }

    pub fn select_connection(
        &mut self,
        connection_id: String,
        visible_ids: &[String],
        additive: bool,
        range: bool,
    ) -> usize {
        select_connection_ids(
            &mut self.selected_ids,
            &mut self.last_selected_id,
            connection_id,
            visible_ids,
            additive,
            range,
        )
    }

    pub fn select_only(&mut self, connection_id: String) {
        self.selected_ids.clear();
        self.selected_ids.insert(connection_id.clone());
        self.last_selected_id = Some(connection_id);
    }

    pub fn clear_selection(&mut self) {
        clear_selected_connection_ids(&mut self.selected_ids, &mut self.last_selected_id);
    }

    pub fn toggle_more_menu(&mut self) {
        self.more_menu_open = !self.more_menu_open;
    }

    pub fn close_more_menu(&mut self) -> bool {
        self.move_submenu_open = false;
        close_connection_more_menu(&mut self.more_menu_open)
    }

    /// Record an in-flight IME composition and where its selection sits.
    pub fn cycle_sort_mode(&mut self) -> ConnectionSortMode {
        cycle_connection_sort_mode(&mut self.sort_mode)
    }

    pub fn set_group_hover(&mut self, group_id: String, hovered: bool) -> bool {
        set_connection_group_hover(&mut self.hovered_group_id, group_id, hovered)
    }

    pub fn list_context_menu_is_open(&self) -> bool {
        self.list_context_menu.is_some()
    }

    pub fn active_list_context_menu(&self) -> Option<ConnectionListContextMenuState> {
        self.list_context_menu.clone()
    }

    /// Whether the open context menu is showing its "move to group" flyout.
    pub fn keyboard_active_connection_id(&self) -> Option<&str> {
        self.keyboard_active_connection_id.as_deref()
    }

    pub fn connection_is_keyboard_active(&self, connection_id: &str) -> bool {
        self.keyboard_active_connection_id.as_deref() == Some(connection_id)
    }

    pub fn set_keyboard_active_connection_id(&mut self, connection_id: Option<String>) {
        self.keyboard_active_connection_id = connection_id;
    }

    pub fn move_submenu_is_open(&self) -> bool {
        self.move_submenu_open
    }

    pub fn toggle_move_submenu(&mut self) {
        self.move_submenu_open = !self.move_submenu_open;
    }

    pub fn open_list_context_menu(&mut self, x: Pixels, y: Pixels) {
        self.close_more_menu();
        self.context_menu = None;
        self.group_context_menu = None;
        self.move_submenu_open = false;
        self.list_context_menu = Some(ConnectionListContextMenuState { x, y });
    }

    pub fn open_context_menu(&mut self, connection_id: String, x: Pixels, y: Pixels) {
        self.close_more_menu();
        self.group_context_menu = None;
        self.list_context_menu = None;
        self.move_submenu_open = false;
        if !self.selected_ids.contains(&connection_id) {
            self.select_only(connection_id.clone());
        }
        self.context_menu = Some(ConnectionContextMenuState {
            connection_id,
            x,
            y,
        });
    }

    pub fn open_group_context_menu(&mut self, group_id: String, x: Pixels, y: Pixels) {
        self.close_more_menu();
        self.context_menu = None;
        self.list_context_menu = None;
        self.move_submenu_open = false;
        self.group_context_menu = Some(ConnectionGroupContextMenuState { group_id, x, y });
    }

    pub fn close_context_menus(&mut self) {
        self.context_menu = None;
        self.group_context_menu = None;
        self.list_context_menu = None;
        self.move_submenu_open = false;
    }

    pub fn toggle_group_expanded(&mut self, group_id: String) -> bool {
        if self.expanded_group_ids.remove(&group_id) {
            return false;
        }
        self.expanded_group_ids.insert(group_id);
        true
    }

    pub fn expand_group(&mut self, group_id: String) {
        self.expanded_group_ids.insert(group_id);
    }

    pub fn expand_groups(&mut self, group_ids: impl IntoIterator<Item = String>) {
        self.expanded_group_ids.extend(group_ids);
    }

    pub fn sync_search_expansion(
        &mut self,
        query: &str,
        matching_group_ids: impl IntoIterator<Item = String>,
    ) -> bool {
        sync_connection_search_expansion(
            &mut self.expanded_group_ids,
            &mut self.search_expanded_base,
            &mut self.search_applied_query,
            query,
            matching_group_ids,
        )
    }

    pub fn set_drop_target_if_changed(&mut self, target: ConnectionDropTarget) -> bool {
        set_connection_drop_target_if_changed(&mut self.drop_target, target)
    }

    pub fn drop_position_for_target(
        &self,
        target_id: &str,
        fallback: ConnectionDropPosition,
    ) -> ConnectionDropPosition {
        connection_drop_position_for_target(&self.drop_target, target_id, fallback)
    }

    pub fn clear_drop_target(&mut self) {
        self.drop_target = None;
    }

    pub fn clear_runtime_state(&mut self) {
        self.search_expanded_base = None;
        self.search_applied_query = None;
        self.list_context_menu = None;
        self.move_submenu_open = false;
        self.keyboard_active_connection_id = None;
        clear_connection_list_runtime_state(
            &mut self.selected_ids,
            &mut self.last_selected_id,
            &mut self.expanded_group_ids,
            &mut self.context_menu,
            &mut self.group_context_menu,
            &mut self.drop_target,
            &mut self.hovered_group_id,
        );
    }

    pub fn remove_connection_references(&mut self, connection_id: &str) {
        remove_connection_list_references(
            &mut self.selected_ids,
            &mut self.last_selected_id,
            &mut self.context_menu,
            &mut self.drop_target,
            connection_id,
        );
    }

    pub fn remove_group_references(&mut self, group_id: &str) {
        remove_group_list_references(
            &mut self.expanded_group_ids,
            &mut self.hovered_group_id,
            &mut self.group_context_menu,
            &mut self.drop_target,
            group_id,
        );
    }

    pub fn retain_loaded_references(
        &mut self,
        connection_ids: &HashSet<String>,
        group_ids: &HashSet<String>,
    ) {
        if let Some(base) = self.search_expanded_base.as_mut() {
            base.retain(|id| group_ids.contains(id));
        }
        retain_loaded_connection_references(
            &mut self.selected_ids,
            &mut self.last_selected_id,
            &mut self.context_menu,
            &mut self.drop_target,
            connection_ids,
        );
        retain_loaded_group_list_references(
            &mut self.expanded_group_ids,
            &mut self.hovered_group_id,
            &mut self.group_context_menu,
            &mut self.drop_target,
            group_ids,
        );
    }
}

impl ConnectionImportState {
    pub fn is_dialog_open(&self) -> bool {
        self.import_dialog_open
    }

    pub fn path_prompt_active(&self) -> bool {
        self.import_path_prompt.is_some()
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.import_focus.clone()
    }

    pub fn open_dialog(&mut self) {
        self.import_dialog_open = true;
    }

    pub fn close_dialog(&mut self) {
        self.import_dialog_open = false;
    }

    pub fn begin_path_prompt(&mut self, source: ConnectionImportSource) {
        self.import_dialog_open = false;
        self.import_path_prompt = Some(source);
    }

    pub fn finish_path_prompt(&mut self) {
        self.import_path_prompt = None;
    }
}

/// Where the highlight lands after `delta` steps through `len` options.
///
/// Wraps at both ends, and refuses to move in an empty list — the caller has
/// nothing to highlight there, not even index zero.
fn stepped_menu_highlight(current: usize, delta: isize, len: usize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    // Clamp first: a stale highlight from a longer list would otherwise land
    // somewhere arbitrary after the modulo.
    let current = current.min(len - 1) as isize;
    Some((current + delta).rem_euclid(len as isize) as usize)
}

impl ConnectionEditorFeatureState {
    pub fn begin_edit(&mut self, draft: ConnectionEditorState) {
        self.icon_picker_open = false;
        self.menu = None;
        self.draft = Some(draft);
    }

    pub fn active_draft(&self) -> Option<ConnectionEditorState> {
        self.draft.clone()
    }

    pub fn active_menu(&self) -> Option<ConnectionEditorMenu> {
        self.menu
    }

    pub fn icon_picker_is_open(&self) -> bool {
        self.icon_picker_open
    }

    pub fn menu_is_open(&self) -> bool {
        self.menu.is_some()
    }

    pub fn description_is_focused(&self) -> bool {
        self.draft
            .as_ref()
            .is_some_and(|editor| editor.focused_field == ConnectionEditorField::Description)
    }

    /// Whether the caret is in the group popover's "new folder" box, which
    /// decides what Enter means while that popover is open.
    pub fn new_group_field_is_focused(&self, cx: &App) -> bool {
        self.fields
            .get(&ConnectionEditorField::NewGroupName)
            .is_some_and(|field| field.read(cx).has_focus())
    }

    pub fn new_group_name_focused_in_group_menu(&self) -> bool {
        self.menu == Some(ConnectionEditorMenu::Group)
            && self
                .draft
                .as_ref()
                .is_some_and(|editor| editor.focused_field == ConnectionEditorField::NewGroupName)
    }

    pub fn inline_panel_draft(&self) -> Option<ConnectionEditorState> {
        connection_editor_inline_panel_draft(
            &self.draft,
            self.has_window(),
            self.window_open_pending,
        )
    }

    pub fn is_editing_saved_connection(&self) -> bool {
        self.draft
            .as_ref()
            .is_some_and(|editor| editor.id.is_some())
    }

    pub fn has_draft(&self) -> bool {
        self.draft.is_some()
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    pub fn window_handle(&self) -> Option<WindowHandle<ConnectionEditorWindow>> {
        self.window
    }

    pub fn has_window(&self) -> bool {
        self.window.is_some()
    }

    pub fn window_open_pending(&self) -> bool {
        self.window_open_pending
    }

    pub fn modal_window_open_or_pending(&self) -> bool {
        connection_editor_window_open_or_pending(self.has_window(), self.window_open_pending)
    }

    pub fn close_popovers(&mut self) {
        self.icon_picker_open = false;
        self.menu = None;
        self.menu_highlight = 0;
    }

    pub fn toggle_icon_picker(&mut self) {
        let opening = !self.icon_picker_open;
        self.close_popovers();
        self.icon_picker_open = opening;
    }

    pub fn toggle_menu(&mut self, menu: ConnectionEditorMenu) {
        self.icon_picker_open = false;
        let opening = self.menu != Some(menu);
        self.menu = opening.then_some(menu);
        self.menu_highlight = 0;
        if !opening && menu == ConnectionEditorMenu::Group {
            clear_connection_editor_group_menu_draft(&mut self.draft);
        }
    }

    pub fn menu_focus_handle(&self) -> FocusHandle {
        self.menu_focus.clone()
    }

    pub fn menu_scroll_handle(&self) -> ScrollHandle {
        self.menu_scroll.clone()
    }

    pub fn menu_highlight(&self) -> usize {
        self.menu_highlight
    }

    /// Put the highlight on `index`, and scroll it into view.
    pub fn set_menu_highlight(&mut self, index: usize) {
        self.menu_highlight = index;
        self.menu_scroll.scroll_to_item(index);
    }

    /// Step the highlight by `delta` options, wrapping at both ends the way a
    /// native combo box does.
    pub fn step_menu_highlight(&mut self, delta: isize, len: usize) {
        if let Some(next) = stepped_menu_highlight(self.menu_highlight, delta, len) {
            self.set_menu_highlight(next);
        }
    }

    pub fn set_icon(&mut self, icon: Option<&str>) -> bool {
        self.close_popovers();
        set_connection_editor_icon(&mut self.draft, icon)
    }

    pub fn set_icon_auto_detect(&mut self, enabled: bool) -> bool {
        set_connection_editor_icon_auto_detect(&mut self.draft, enabled)
    }

    pub fn set_menu_value(&mut self, menu: ConnectionEditorMenu, value: Option<String>) -> bool {
        let changed = set_connection_editor_menu_value(&mut self.draft, menu, value);
        if changed {
            self.close_popovers();
        }
        changed
    }

    pub fn set_password_source(&mut self, source: ConnectionEditorPasswordSource) -> bool {
        let changed = set_connection_editor_password_source(&mut self.draft, source);
        if changed {
            self.close_popovers();
        }
        changed
    }

    pub fn set_advanced_tab(&mut self, tab: ConnectionEditorAdvancedTab) -> bool {
        let changed = set_connection_editor_advanced_tab(&mut self.draft, tab);
        if changed {
            self.close_popovers();
        }
        changed
    }

    pub fn set_telnet_tab(&mut self, tab: ConnectionEditorTelnetTab) -> bool {
        let changed = set_connection_editor_telnet_tab(&mut self.draft, tab);
        if changed {
            self.close_popovers();
        }
        changed
    }

    pub fn set_kind(&mut self, kind: ConnectionKindTab) -> bool {
        self.close_popovers();
        set_connection_editor_kind(&mut self.draft, kind)
    }

    pub fn commit_new_group(&mut self, required_message: String) -> bool {
        let changed = commit_connection_editor_new_group(&mut self.draft, required_message);
        if changed {
            self.close_popovers();
        }
        changed
    }

    pub fn toggle_flag(&mut self, flag: ConnectionEditorToggle) -> bool {
        let changed = toggle_connection_editor_flag(&mut self.draft, flag);
        if changed && flag == ConnectionEditorToggle::Advanced {
            self.close_popovers();
        }
        changed
    }

    pub fn insert_description_newline(&mut self) -> bool {
        insert_connection_editor_description_newline(&mut self.draft)
    }

    pub fn apply_text_key(&mut self, key: &str, input: Option<&str>) -> bool {
        apply_connection_editor_text_key(&mut self.draft, key, input)
    }

    pub fn advance_focus(&mut self) -> bool {
        advance_connection_editor_focus(&mut self.draft)
    }

    pub fn close_popovers_and_cancel_group_draft(&mut self) {
        self.close_popovers();
        clear_connection_editor_group_menu_draft(&mut self.draft);
    }

    pub fn set_error(&mut self, error: String) -> bool {
        set_connection_editor_error(&mut self.draft, error)
    }

    pub fn apply_shell_path(&mut self, shell_path: String) -> bool {
        apply_connection_editor_shell_path(&mut self.draft, shell_path)
    }

    pub fn apply_working_dir(&mut self, working_dir: String) -> bool {
        apply_connection_editor_working_dir(&mut self.draft, working_dir)
    }

    pub fn close(&mut self) {
        clear_connection_editor_runtime_state(
            &mut self.draft,
            &mut self.icon_picker_open,
            &mut self.menu,
            &mut self.window,
            &mut self.window_open_pending,
        );
    }

    pub fn clear_window_if_current(
        &mut self,
        window: WindowHandle<ConnectionEditorWindow>,
    ) -> bool {
        if self.window.is_some_and(|current| current == window) {
            self.window = None;
            return true;
        }
        false
    }

    pub fn mark_window_pending(&mut self) {
        self.window_open_pending = true;
    }

    pub fn clear_window_pending(&mut self) {
        self.window_open_pending = false;
    }

    pub fn attach_window(&mut self, window: WindowHandle<ConnectionEditorWindow>) {
        self.window = Some(window);
        self.window_open_pending = false;
    }

    pub fn clear_window(&mut self) {
        self.window = None;
        self.window_open_pending = false;
    }
}

impl ConnectionGroupEditorFeatureState {
    pub fn active_draft(&self) -> Option<ConnectionGroupEditorState> {
        self.draft.clone()
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    pub fn begin_edit(&mut self, draft: ConnectionGroupEditorState) {
        self.draft = Some(draft);
    }

    pub fn apply_name_key(&mut self, key: &str, input: Option<&str>) -> bool {
        apply_connection_group_editor_name_key(&mut self.draft, key, input)
    }

    pub fn set_error(&mut self, error: String) -> bool {
        set_connection_group_editor_error(&mut self.draft, error)
    }

    pub fn close(&mut self) {
        self.draft = None;
    }
}

impl ConnectionConfirmationState {
    pub fn open_clear_all(&mut self) {
        self.clear_all_open = true;
    }

    pub fn close_clear_all(&mut self) {
        self.clear_all_open = false;
    }

    pub fn clear_all_is_open(&self) -> bool {
        self.clear_all_open
    }

    pub fn active_delete(&self) -> Option<ConnectionDeleteConfirmState> {
        self.delete.clone()
    }

    pub fn open_delete(&mut self, confirm: ConnectionDeleteConfirmState) {
        self.delete = Some(confirm);
    }

    pub fn close_delete(&mut self) {
        self.delete = None;
    }

    pub fn take_delete(&mut self) -> Option<ConnectionDeleteConfirmState> {
        self.delete.take()
    }

    pub fn active_group_delete(&self) -> Option<ConnectionGroupDeleteConfirmState> {
        self.group_delete.clone()
    }

    pub fn open_group_delete(&mut self, confirm: ConnectionGroupDeleteConfirmState) {
        self.group_delete = Some(confirm);
    }

    pub fn close_group_delete(&mut self) {
        self.group_delete = None;
    }

    pub fn take_group_delete(&mut self) -> Option<ConnectionGroupDeleteConfirmState> {
        self.group_delete.take()
    }

    pub fn active_group_open(&self) -> Option<ConnectionGroupOpenConfirmState> {
        self.group_open.clone()
    }

    pub fn open_group_open(&mut self, confirm: ConnectionGroupOpenConfirmState) {
        self.group_open = Some(confirm);
    }

    pub fn close_group_open(&mut self) {
        self.group_open = None;
    }

    pub fn take_group_open(&mut self) -> Option<ConnectionGroupOpenConfirmState> {
        self.group_open.take()
    }

    pub fn group_open_focus_handle(&self) -> FocusHandle {
        self.group_open_focus.clone()
    }
}

impl NetworkFeatureState {
    pub fn active_tab(&self) -> NetworkTab {
        self.tab
    }

    pub fn tab_is(&self, tab: NetworkTab) -> bool {
        self.tab == tab
    }

    pub fn section_is_expanded(&self, section_key: &str) -> bool {
        self.expanded_sections.contains(section_key)
    }

    pub fn item_menu_is_open(&self, tab: NetworkTab, id: &str) -> bool {
        self.item_menu
            .as_ref()
            .is_some_and(|menu| menu.tab == tab && menu.id == id)
    }

    pub fn move_picker_is_open(&self, tab: NetworkTab, id: &str) -> bool {
        self.move_picker
            .as_ref()
            .is_some_and(|picker| picker.tab == tab && picker.id == id)
    }

    pub fn active_delete_confirm(&self) -> Option<NetworkDeleteConfirmState> {
        self.delete_confirm.clone()
    }

    pub fn active_group_editor(&self) -> Option<NetworkGroupEditorState> {
        self.group_editor.clone()
    }

    pub fn active_group_delete_confirm(&self) -> Option<NetworkGroupDeleteConfirmState> {
        self.group_delete_confirm.clone()
    }

    pub fn active_tunnel_editor(&self) -> Option<NetworkTunnelEditorState> {
        self.tunnel_editor.clone()
    }

    pub fn active_proxy_editor(&self) -> Option<NetworkProxyEditorState> {
        self.proxy_editor.clone()
    }

    pub fn tunnel_editor_focus_handle(&self) -> FocusHandle {
        self.tunnel_editor_focus.clone()
    }

    pub fn proxy_editor_focus_handle(&self) -> FocusHandle {
        self.proxy_editor_focus.clone()
    }

    pub fn set_tab(&mut self, tab: NetworkTab) {
        self.tab = tab;
        self.item_menu = None;
        self.move_picker = None;
    }

    pub fn toggle_section(&mut self, section_key: String) -> bool {
        self.item_menu = None;
        if self.expanded_sections.remove(&section_key) {
            self.move_picker = None;
            return false;
        }
        self.expanded_sections.insert(section_key);
        true
    }

    pub fn toggle_item_menu(&mut self, tab: NetworkTab, id: String) -> bool {
        toggle_network_item_menu_state(&mut self.item_menu, &mut self.move_picker, tab, id)
    }

    pub fn toggle_move_picker(&mut self, tab: NetworkTab, id: String) -> bool {
        toggle_network_move_picker_state(&mut self.item_menu, &mut self.move_picker, tab, id)
    }

    pub fn close_move_picker(&mut self) {
        self.move_picker = None;
    }

    pub fn open_delete_confirm(&mut self, confirm: NetworkDeleteConfirmState) {
        self.item_menu = None;
        self.delete_confirm = Some(confirm);
    }

    pub fn close_delete_confirm(&mut self) {
        self.delete_confirm = None;
    }

    pub fn begin_group_edit(&mut self, draft: NetworkGroupEditorState) {
        self.item_menu = None;
        self.group_editor = Some(draft);
    }

    /// Write the group draft's name, clearing any stale validation.
    pub fn set_group_editor_name(&mut self, text: String) -> bool {
        set_network_group_editor_name(&mut self.group_editor, text)
    }

    pub fn set_group_editor_error(&mut self, error: String) -> bool {
        set_network_group_editor_error(&mut self.group_editor, error)
    }

    pub fn close_group_editor(&mut self) {
        self.group_editor = None;
    }

    pub fn open_group_delete_confirm(&mut self, confirm: NetworkGroupDeleteConfirmState) {
        self.item_menu = None;
        self.group_delete_confirm = Some(confirm);
    }

    pub fn close_group_delete_confirm(&mut self) {
        self.group_delete_confirm = None;
    }

    pub fn begin_tunnel_edit(&mut self, draft: NetworkTunnelEditorState) {
        self.item_menu = None;
        self.tab = NetworkTab::Tunnels;
        self.tunnel_editor = Some(draft);
    }

    pub fn close_tunnel_editor(&mut self) {
        clear_network_tunnel_editor(&mut self.tunnel_editor);
    }

    /// Write one field of the tunnel draft, clearing any stale validation.
    pub fn set_tunnel_editor_field(
        &mut self,
        field: NetworkTunnelEditorField,
        text: String,
    ) -> bool {
        set_network_tunnel_editor_field(&mut self.tunnel_editor, field, text)
    }

    pub fn cycle_tunnel_type(&mut self) -> Option<String> {
        cycle_network_tunnel_type(&mut self.tunnel_editor)
    }

    pub fn cycle_tunnel_connection<'a>(
        &mut self,
        connection_ids: impl IntoIterator<Item = &'a str>,
    ) -> bool {
        cycle_network_tunnel_connection(&mut self.tunnel_editor, connection_ids)
    }

    pub fn cycle_tunnel_group<'a>(&mut self, group_ids: impl IntoIterator<Item = &'a str>) -> bool {
        cycle_network_tunnel_group(&mut self.tunnel_editor, group_ids)
    }

    pub fn set_tunnel_bind_localhost(&mut self, bind_localhost: bool) -> bool {
        set_network_tunnel_bind_localhost(&mut self.tunnel_editor, bind_localhost)
    }

    pub fn toggle_tunnel_auto_open(&mut self) -> Option<bool> {
        toggle_network_tunnel_auto_open(&mut self.tunnel_editor)
    }

    pub fn set_tunnel_editor_error(&mut self, error: String) -> bool {
        set_network_tunnel_editor_error(&mut self.tunnel_editor, error)
    }

    pub fn begin_proxy_edit(&mut self, draft: NetworkProxyEditorState) {
        self.item_menu = None;
        self.tab = NetworkTab::Proxies;
        self.proxy_editor = Some(draft);
    }

    pub fn close_proxy_editor(&mut self) {
        clear_network_proxy_editor(&mut self.proxy_editor);
    }

    /// Write one field of the proxy draft, clearing any stale validation.
    pub fn set_proxy_editor_field(&mut self, field: NetworkProxyEditorField, text: String) -> bool {
        set_network_proxy_editor_field(&mut self.proxy_editor, field, text)
    }

    pub fn cycle_proxy_protocol(&mut self) -> Option<String> {
        cycle_network_proxy_protocol(&mut self.proxy_editor)
    }

    pub fn cycle_proxy_group<'a>(&mut self, group_ids: impl IntoIterator<Item = &'a str>) -> bool {
        cycle_network_proxy_group(&mut self.proxy_editor, group_ids)
    }

    pub fn set_proxy_editor_error(&mut self, error: String) -> bool {
        set_network_proxy_editor_error(&mut self.proxy_editor, error)
    }

    pub fn remove_item_references(&mut self, tab: NetworkTab, id: &str) {
        remove_network_item_references(
            &mut self.delete_confirm,
            &mut self.item_menu,
            &mut self.move_picker,
            &mut self.tunnel_editor,
            &mut self.proxy_editor,
            tab,
            id,
        );
    }

    pub fn remove_group_references(
        &mut self,
        tab: NetworkTab,
        group_id: &str,
        deleted_item_ids: &[String],
    ) {
        remove_network_group_references(
            &mut self.group_editor,
            &mut self.group_delete_confirm,
            &mut self.expanded_sections,
            tab,
            group_id,
        );
        for item_id in deleted_item_ids {
            self.remove_item_references(tab, item_id);
        }
    }
}

#[cfg(test)]
mod tests;

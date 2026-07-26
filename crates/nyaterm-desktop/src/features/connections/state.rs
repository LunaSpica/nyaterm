use std::collections::{HashMap, HashSet};

use gpui::{
    App, AppContext as _, Context, Entity, FocusHandle, Pixels, ScrollHandle, SharedString,
    Subscription, WindowHandle,
};
use nyaterm_core::AppSettingsSummary;

use super::connections::{ConnectionDragKind, ConnectionDropPosition, ConnectionDropTarget};
use crate::features::{ConnectionEditorToggle, ConnectionEditorWindow, NyaTermApp};
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
    editor_field_seeds, finish_connection_editor_save_state,
    insert_connection_editor_description_newline, set_connection_editor_advanced_tab,
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
    retain_loaded_connection_list_references, select_connection_ids,
    set_connection_drop_target_if_changed, set_connection_group_hover,
    sync_connection_search_expansion,
};
#[cfg(test)]
use self::network_logic::remove_network_group_references;
use self::network_logic::{
    advance_network_proxy_editor_focus, advance_network_tunnel_editor_focus,
    apply_network_group_editor_name_key, apply_network_proxy_editor_key,
    apply_network_tunnel_editor_key, clear_network_proxy_editor, clear_network_tunnel_editor,
    cycle_network_proxy_group, cycle_network_proxy_protocol, cycle_network_tunnel_connection,
    cycle_network_tunnel_group, cycle_network_tunnel_type, focus_network_proxy_editor_field,
    focus_network_tunnel_editor_field, insert_network_proxy_command_newline,
    remove_network_group_and_item_references, remove_network_item_references,
    set_network_group_editor_error, set_network_proxy_editor_error,
    set_network_tunnel_bind_localhost, set_network_tunnel_editor_error,
    toggle_network_item_menu_state, toggle_network_move_picker_state,
    toggle_network_tunnel_auto_open,
};

pub(in crate::features) struct ConnectionFeatureState {
    pub list: ConnectionListState,
    pub import: ConnectionImportState,
    pub editor: ConnectionEditorFeatureState,
    pub group_editor: ConnectionGroupEditorFeatureState,
    pub confirmations: ConnectionConfirmationState,
    pub network: NetworkFeatureState,
}

pub(in crate::features) struct ConnectionFeatureFocus {
    /// Placeholder for the filter box, resolved by the caller so this struct
    /// stays free of the i18n lookup.
    pub filter_placeholder: SharedString,
    pub import: FocusHandle,
    pub editor: FocusHandle,
    pub group_editor: FocusHandle,
    pub group_open_confirm: FocusHandle,
    pub network_group_editor: FocusHandle,
    pub network_tunnel_editor: FocusHandle,
    pub network_proxy_editor: FocusHandle,
}

pub(in crate::features) struct ConnectionListState {
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

pub(in crate::features) struct ConnectionImportState {
    import_dialog_open: bool,
    import_path_prompt: Option<ConnectionImportSource>,
    import_focus: FocusHandle,
}

pub(in crate::features) struct ConnectionEditorFeatureState {
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

pub(in crate::features) struct ConnectionGroupEditorFeatureState {
    draft: Option<ConnectionGroupEditorState>,
    /// The folder-name input, built with the draft it mirrors.
    field: Option<Entity<TextField>>,
    field_subscription: Option<Subscription>,
    focus: FocusHandle,
}

pub(in crate::features) struct ConnectionConfirmationState {
    clear_all_open: bool,
    delete: Option<ConnectionDeleteConfirmState>,
    group_delete: Option<ConnectionGroupDeleteConfirmState>,
    group_open: Option<ConnectionGroupOpenConfirmState>,
    group_open_focus: FocusHandle,
}

pub(in crate::features) struct NetworkFeatureState {
    tab: NetworkTab,
    delete_confirm: Option<NetworkDeleteConfirmState>,
    group_editor: Option<NetworkGroupEditorState>,
    group_delete_confirm: Option<NetworkGroupDeleteConfirmState>,
    item_menu: Option<NetworkItemMenuState>,
    move_picker: Option<NetworkMovePickerState>,
    expanded_sections: HashSet<String>,
    tunnel_editor: Option<NetworkTunnelEditorState>,
    proxy_editor: Option<NetworkProxyEditorState>,
    group_editor_focus: FocusHandle,
    tunnel_editor_focus: FocusHandle,
    proxy_editor_focus: FocusHandle,
}

impl ConnectionFeatureState {
    pub fn new(
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
                app.connection_state.list.set_search_text(text.clone());
                app.sync_connection_keyboard_active(cx);
                cx.notify();
            },
        );
        Self {
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
                group_editor_focus: focus.network_group_editor,
                tunnel_editor_focus: focus.network_tunnel_editor,
                proxy_editor_focus: focus.network_proxy_editor,
            },
        }
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

    pub fn clear_editor_fields(&mut self) {
        self.editor.fields.clear();
        self.editor.field_subscriptions.clear();
    }

    pub fn finish_editor_save(&mut self, connection_id: String, group_id: Option<String>) {
        finish_connection_editor_save_state(
            &mut self.editor.draft,
            &mut self.editor.icon_picker_open,
            &mut self.editor.menu,
            &mut self.editor.window,
            &mut self.editor.window_open_pending,
            &mut self.list.selected_ids,
            &mut self.list.last_selected_id,
            &mut self.list.expanded_group_ids,
            connection_id,
            group_id,
        );
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

    pub fn selected_connection_ids(&self) -> impl Iterator<Item = &str> {
        self.selected_ids.iter().map(String::as_str)
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
        retain_loaded_connection_list_references(
            &mut self.selected_ids,
            &mut self.last_selected_id,
            &mut self.context_menu,
            &mut self.expanded_group_ids,
            &mut self.hovered_group_id,
            &mut self.group_context_menu,
            &mut self.drop_target,
            connection_ids,
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

    pub fn group_editor_focus_handle(&self) -> FocusHandle {
        self.group_editor_focus.clone()
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

    pub fn apply_group_editor_name_key(&mut self, key: &str, input: Option<&str>) -> bool {
        apply_network_group_editor_name_key(&mut self.group_editor, key, input)
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

    pub fn focus_tunnel_editor_field(&mut self, field: NetworkTunnelEditorField) -> bool {
        focus_network_tunnel_editor_field(&mut self.tunnel_editor, field)
    }

    pub fn advance_tunnel_editor_focus(&mut self) -> bool {
        advance_network_tunnel_editor_focus(&mut self.tunnel_editor)
    }

    pub fn apply_tunnel_editor_key(&mut self, key: &str, input: Option<&str>) -> bool {
        apply_network_tunnel_editor_key(&mut self.tunnel_editor, key, input)
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

    pub fn focus_proxy_editor_field(&mut self, field: NetworkProxyEditorField) -> bool {
        focus_network_proxy_editor_field(&mut self.proxy_editor, field)
    }

    pub fn insert_proxy_command_newline(&mut self) -> bool {
        insert_network_proxy_command_newline(&mut self.proxy_editor)
    }

    pub fn advance_proxy_editor_focus(&mut self) -> bool {
        advance_network_proxy_editor_focus(&mut self.proxy_editor)
    }

    pub fn apply_proxy_editor_key(&mut self, key: &str, input: Option<&str>) -> bool {
        apply_network_proxy_editor_key(&mut self.proxy_editor, key, input)
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
        remove_network_group_and_item_references(
            &mut self.group_editor,
            &mut self.group_delete_confirm,
            &mut self.expanded_sections,
            &mut self.delete_confirm,
            &mut self.item_menu,
            &mut self.move_picker,
            &mut self.tunnel_editor,
            &mut self.proxy_editor,
            tab,
            group_id,
            deleted_item_ids,
        );
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{
        advance_network_proxy_editor_focus, advance_network_tunnel_editor_focus,
        apply_connection_editor_shell_path, apply_connection_editor_text_key,
        apply_connection_editor_working_dir, apply_connection_group_editor_name_key,
        apply_network_group_editor_name_key, apply_network_proxy_editor_key,
        apply_network_tunnel_editor_key, clear_connection_editor_group_menu_draft,
        clear_connection_editor_runtime_state, clear_connection_list_runtime_state,
        clear_network_proxy_editor, clear_network_tunnel_editor, clear_selected_connection_ids,
        close_connection_more_menu, commit_connection_editor_new_group,
        connection_drop_position_for_target, connection_editor_inline_panel_draft,
        connection_editor_window_open_or_pending, cycle_connection_sort_mode,
        cycle_network_proxy_group, cycle_network_proxy_protocol, cycle_network_tunnel_connection,
        cycle_network_tunnel_group, cycle_network_tunnel_type, finish_connection_editor_save_state,
        focus_network_proxy_editor_field, focus_network_tunnel_editor_field,
        insert_connection_editor_description_newline, insert_network_proxy_command_newline,
        remove_connection_list_references, remove_group_list_references,
        remove_network_group_and_item_references, remove_network_group_references,
        remove_network_item_references, retain_loaded_connection_list_references,
        select_connection_ids, set_connection_drop_target_if_changed,
        set_connection_editor_advanced_tab, set_connection_editor_error,
        set_connection_editor_field_text, set_connection_editor_icon, set_connection_editor_kind,
        set_connection_editor_menu_value, set_connection_editor_password_source,
        set_connection_editor_telnet_tab, set_connection_group_editor_error,
        set_connection_group_hover, set_network_group_editor_error, set_network_proxy_editor_error,
        set_network_tunnel_bind_localhost, set_network_tunnel_editor_error, stepped_menu_highlight,
        sync_connection_search_expansion, toggle_connection_editor_flag,
        toggle_network_item_menu_state, toggle_network_move_picker_state,
        toggle_network_tunnel_auto_open,
    };
    use crate::features::{
        ConnectionDragKind, ConnectionDropPosition, ConnectionDropTarget, ConnectionEditorToggle,
    };
    use crate::models::{
        ConnectionContextMenuState, ConnectionEditorAdvancedTab, ConnectionEditorField,
        ConnectionEditorMenu, ConnectionEditorPasswordSource, ConnectionEditorState,
        ConnectionEditorTelnetTab, ConnectionGroupContextMenuState, ConnectionGroupEditorState,
        ConnectionKindTab, ConnectionSortMode, NetworkDeleteConfirmState,
        NetworkGroupDeleteConfirmState, NetworkGroupEditorState, NetworkItemMenuState,
        NetworkMovePickerState, NetworkProxyEditorField, NetworkProxyEditorState, NetworkTab,
        NetworkTunnelEditorField, NetworkTunnelEditorState,
    };

    #[test]
    fn search_expansion_opens_matches_and_restores_the_prior_tree() {
        let mut expanded = HashSet::from(["kept".to_string()]);
        let mut base = None;
        let mut applied = None;

        assert!(sync_connection_search_expansion(
            &mut expanded,
            &mut base,
            &mut applied,
            "web",
            ["hit".to_string()],
        ));
        assert_eq!(
            expanded,
            HashSet::from(["kept".to_string(), "hit".to_string()])
        );

        // Clearing the filter must not leave the auto-opened folder behind.
        assert!(sync_connection_search_expansion(
            &mut expanded,
            &mut base,
            &mut applied,
            "",
            Vec::new(),
        ));
        assert_eq!(expanded, HashSet::from(["kept".to_string()]));
        assert!(base.is_none());
    }

    #[test]
    fn search_expansion_lets_a_folder_stay_collapsed_within_one_keyword() {
        let mut expanded = HashSet::new();
        let mut base = None;
        let mut applied = None;

        sync_connection_search_expansion(
            &mut expanded,
            &mut base,
            &mut applied,
            "web",
            ["hit".to_string()],
        );
        expanded.remove("hit");

        // Same keyword re-rendering must not re-open what the user just closed.
        assert!(!sync_connection_search_expansion(
            &mut expanded,
            &mut base,
            &mut applied,
            "web",
            ["hit".to_string()],
        ));
        assert!(expanded.is_empty());

        // A new keyword is a fresh search, so it expands again.
        assert!(sync_connection_search_expansion(
            &mut expanded,
            &mut base,
            &mut applied,
            "webs",
            ["hit".to_string()],
        ));
        assert!(expanded.contains("hit"));
    }

    #[test]
    fn clear_selected_connection_ids_clears_selection_and_anchor() {
        let mut selected_ids = HashSet::from(["one".to_string(), "two".to_string()]);
        let mut last_selected_id = Some("two".to_string());

        clear_selected_connection_ids(&mut selected_ids, &mut last_selected_id);

        assert!(selected_ids.is_empty());
        assert_eq!(last_selected_id, None);
    }

    #[test]
    fn close_connection_more_menu_reports_whether_menu_was_open() {
        let mut more_menu_open = true;

        assert!(close_connection_more_menu(&mut more_menu_open));
        assert!(!more_menu_open);
        assert!(!close_connection_more_menu(&mut more_menu_open));
    }

    #[test]
    fn cycle_connection_sort_mode_updates_and_returns_next_mode() {
        let mut sort_mode = ConnectionSortMode::Default;

        let next = cycle_connection_sort_mode(&mut sort_mode);

        assert_eq!(sort_mode, next);
        assert_eq!(sort_mode, ConnectionSortMode::NameAsc);
    }

    #[test]
    fn set_connection_group_hover_is_idempotent() {
        let mut hovered_group_id = None;

        assert!(set_connection_group_hover(
            &mut hovered_group_id,
            "group-a".to_string(),
            true,
        ));
        assert_eq!(hovered_group_id.as_deref(), Some("group-a"));
        assert!(!set_connection_group_hover(
            &mut hovered_group_id,
            "group-a".to_string(),
            true,
        ));
        assert!(!set_connection_group_hover(
            &mut hovered_group_id,
            "group-b".to_string(),
            false,
        ));
        assert!(set_connection_group_hover(
            &mut hovered_group_id,
            "group-a".to_string(),
            false,
        ));
        assert_eq!(hovered_group_id, None);
    }

    #[test]
    fn set_connection_drop_target_if_changed_ignores_repeated_target() {
        let target = ConnectionDropTarget {
            id: Some("one".to_string()),
            kind: ConnectionDragKind::Connection,
            position: ConnectionDropPosition::After,
        };
        let mut drop_target = None;

        assert!(set_connection_drop_target_if_changed(
            &mut drop_target,
            target.clone()
        ));
        assert_eq!(drop_target, Some(target.clone()));
        assert!(!set_connection_drop_target_if_changed(
            &mut drop_target,
            target
        ));
    }

    #[test]
    fn connection_drop_position_for_target_uses_matching_target_or_fallback() {
        let drop_target = Some(ConnectionDropTarget {
            id: Some("group-a".to_string()),
            kind: ConnectionDragKind::Group,
            position: ConnectionDropPosition::Inside,
        });

        assert_eq!(
            connection_drop_position_for_target(
                &drop_target,
                "group-a",
                ConnectionDropPosition::Before
            ),
            ConnectionDropPosition::Inside
        );
        assert_eq!(
            connection_drop_position_for_target(
                &drop_target,
                "group-b",
                ConnectionDropPosition::Before
            ),
            ConnectionDropPosition::Before
        );
    }

    #[test]
    fn clear_connection_list_runtime_state_removes_transient_ui_references() {
        let mut selected_ids = HashSet::from(["one".to_string()]);
        let mut last_selected_id = Some("one".to_string());
        let mut expanded_group_ids = HashSet::from(["group-a".to_string()]);
        let mut context_menu = Some(ConnectionContextMenuState {
            connection_id: "one".to_string(),
            x: gpui::px(1.),
            y: gpui::px(2.),
        });
        let mut group_context_menu = Some(ConnectionGroupContextMenuState {
            group_id: "group-a".to_string(),
            x: gpui::px(3.),
            y: gpui::px(4.),
        });
        let mut drop_target = Some(ConnectionDropTarget {
            id: Some("group-a".to_string()),
            kind: ConnectionDragKind::Group,
            position: ConnectionDropPosition::Inside,
        });
        let mut hovered_group_id = Some("group-a".to_string());

        clear_connection_list_runtime_state(
            &mut selected_ids,
            &mut last_selected_id,
            &mut expanded_group_ids,
            &mut context_menu,
            &mut group_context_menu,
            &mut drop_target,
            &mut hovered_group_id,
        );

        assert!(selected_ids.is_empty());
        assert_eq!(last_selected_id, None);
        assert!(expanded_group_ids.is_empty());
        assert_eq!(context_menu, None);
        assert_eq!(group_context_menu, None);
        assert_eq!(drop_target, None);
        assert_eq!(hovered_group_id, None);
    }

    #[test]
    fn select_connection_ids_replaces_toggles_and_tracks_anchor() {
        let visible_ids = vec!["one".to_string(), "two".to_string(), "three".to_string()];
        let mut selected_ids = HashSet::from(["old".to_string()]);
        let mut last_selected_id = Some("old".to_string());

        let count = select_connection_ids(
            &mut selected_ids,
            &mut last_selected_id,
            "two".to_string(),
            &visible_ids,
            false,
            false,
        );

        assert_eq!(count, 1);
        assert_eq!(selected_ids, HashSet::from(["two".to_string()]));
        assert_eq!(last_selected_id.as_deref(), Some("two"));

        let count = select_connection_ids(
            &mut selected_ids,
            &mut last_selected_id,
            "three".to_string(),
            &visible_ids,
            true,
            false,
        );

        assert_eq!(count, 2);
        assert_eq!(
            selected_ids,
            HashSet::from(["two".to_string(), "three".to_string()])
        );
        assert_eq!(last_selected_id.as_deref(), Some("three"));

        let count = select_connection_ids(
            &mut selected_ids,
            &mut last_selected_id,
            "three".to_string(),
            &visible_ids,
            true,
            false,
        );

        assert_eq!(count, 1);
        assert_eq!(selected_ids, HashSet::from(["two".to_string()]));
        assert_eq!(last_selected_id.as_deref(), Some("three"));
    }

    #[test]
    fn select_connection_ids_ranges_from_anchor() {
        let visible_ids = vec![
            "one".to_string(),
            "two".to_string(),
            "three".to_string(),
            "four".to_string(),
        ];
        let mut selected_ids = HashSet::from(["one".to_string()]);
        let mut last_selected_id = Some("two".to_string());

        let count = select_connection_ids(
            &mut selected_ids,
            &mut last_selected_id,
            "four".to_string(),
            &visible_ids,
            false,
            true,
        );

        assert_eq!(count, 3);
        assert_eq!(
            selected_ids,
            HashSet::from(["two".to_string(), "three".to_string(), "four".to_string()])
        );
        assert_eq!(last_selected_id.as_deref(), Some("four"));
    }

    #[test]
    fn remove_connection_references_clears_invalid_list_state() {
        let mut selected_ids = HashSet::from(["one".to_string(), "two".to_string()]);
        let mut last_selected_id = Some("one".to_string());
        let mut context_menu = Some(ConnectionContextMenuState {
            connection_id: "one".to_string(),
            x: gpui::px(4.),
            y: gpui::px(8.),
        });
        let mut drop_target = Some(ConnectionDropTarget {
            id: Some("one".to_string()),
            kind: ConnectionDragKind::Connection,
            position: ConnectionDropPosition::After,
        });

        remove_connection_list_references(
            &mut selected_ids,
            &mut last_selected_id,
            &mut context_menu,
            &mut drop_target,
            "one",
        );

        assert_eq!(selected_ids, HashSet::from(["two".to_string()]));
        assert_eq!(last_selected_id, None);
        assert_eq!(context_menu, None);
        assert_eq!(drop_target, None);
    }

    #[test]
    fn remove_group_references_clears_invalid_list_state() {
        let mut expanded_group_ids = HashSet::from(["root".to_string(), "child".to_string()]);
        let mut hovered_group_id = Some("child".to_string());
        let mut group_context_menu = Some(ConnectionGroupContextMenuState {
            group_id: "child".to_string(),
            x: gpui::px(4.),
            y: gpui::px(8.),
        });
        let mut drop_target = Some(ConnectionDropTarget {
            id: Some("child".to_string()),
            kind: ConnectionDragKind::Group,
            position: ConnectionDropPosition::Inside,
        });

        remove_group_list_references(
            &mut expanded_group_ids,
            &mut hovered_group_id,
            &mut group_context_menu,
            &mut drop_target,
            "child",
        );

        assert_eq!(expanded_group_ids, HashSet::from(["root".to_string()]));
        assert_eq!(hovered_group_id, None);
        assert_eq!(group_context_menu, None);
        assert_eq!(drop_target, None);
    }

    #[test]
    fn retain_loaded_connection_list_references_prunes_stale_refresh_state() {
        let mut selected_ids = HashSet::from(["kept".to_string(), "stale".to_string()]);
        let mut last_selected_id = Some("stale".to_string());
        let mut context_menu = Some(ConnectionContextMenuState {
            connection_id: "stale".to_string(),
            x: gpui::px(4.),
            y: gpui::px(8.),
        });
        let mut expanded_group_ids =
            HashSet::from(["kept-group".to_string(), "stale-group".to_string()]);
        let mut hovered_group_id = Some("stale-group".to_string());
        let mut group_context_menu = Some(ConnectionGroupContextMenuState {
            group_id: "stale-group".to_string(),
            x: gpui::px(4.),
            y: gpui::px(8.),
        });
        let mut drop_target = Some(ConnectionDropTarget {
            id: Some("stale-group".to_string()),
            kind: ConnectionDragKind::Group,
            position: ConnectionDropPosition::Inside,
        });
        let connection_ids = HashSet::from(["kept".to_string()]);
        let group_ids = HashSet::from(["kept-group".to_string()]);

        retain_loaded_connection_list_references(
            &mut selected_ids,
            &mut last_selected_id,
            &mut context_menu,
            &mut expanded_group_ids,
            &mut hovered_group_id,
            &mut group_context_menu,
            &mut drop_target,
            &connection_ids,
            &group_ids,
        );

        assert_eq!(selected_ids, HashSet::from(["kept".to_string()]));
        assert_eq!(last_selected_id, None);
        assert_eq!(context_menu, None);
        assert_eq!(
            expanded_group_ids,
            HashSet::from(["kept-group".to_string()])
        );
        assert_eq!(hovered_group_id, None);
        assert_eq!(group_context_menu, None);
        assert_eq!(drop_target, None);
    }

    #[test]
    fn clear_connection_editor_runtime_state_clears_secret_draft_and_overlays() {
        let mut draft = Some(connection_editor_state_with_secret_draft());
        let mut icon_picker_open = true;
        let mut menu = Some(ConnectionEditorMenu::Authentication);
        let mut window = None;
        let mut window_open_pending = true;

        clear_connection_editor_runtime_state(
            &mut draft,
            &mut icon_picker_open,
            &mut menu,
            &mut window,
            &mut window_open_pending,
        );

        assert_eq!(draft, None);
        assert!(!icon_picker_open);
        assert_eq!(menu, None);
        assert_eq!(window, None);
        assert!(!window_open_pending);
    }

    #[test]
    fn finish_connection_editor_save_state_clears_editor_and_selects_saved_connection() {
        let mut draft = Some(connection_editor_state_with_secret_draft());
        let mut icon_picker_open = true;
        let mut menu = Some(ConnectionEditorMenu::Group);
        let mut window = None;
        let mut window_open_pending = true;
        let mut selected_ids = HashSet::from(["old".to_string()]);
        let mut last_selected_id = Some("old".to_string());
        let mut expanded_group_ids = HashSet::from(["existing-group".to_string()]);

        finish_connection_editor_save_state(
            &mut draft,
            &mut icon_picker_open,
            &mut menu,
            &mut window,
            &mut window_open_pending,
            &mut selected_ids,
            &mut last_selected_id,
            &mut expanded_group_ids,
            "conn-b".to_string(),
            Some("group-b".to_string()),
        );

        assert_eq!(draft, None);
        assert!(!icon_picker_open);
        assert_eq!(menu, None);
        assert_eq!(window, None);
        assert!(!window_open_pending);
        assert_eq!(selected_ids, HashSet::from(["conn-b".to_string()]));
        assert_eq!(last_selected_id.as_deref(), Some("conn-b"));
        assert!(expanded_group_ids.contains("existing-group"));
        assert!(expanded_group_ids.contains("group-b"));
    }

    #[test]
    fn connection_editor_inline_panel_draft_requires_draft_without_window() {
        let draft = Some(connection_editor_state_with_secret_draft());

        assert_eq!(
            connection_editor_inline_panel_draft(&draft, false, false)
                .as_ref()
                .map(|editor| editor.id.as_deref()),
            Some(Some("conn"))
        );
        assert!(connection_editor_inline_panel_draft(&draft, true, false).is_none());
        assert!(connection_editor_inline_panel_draft(&draft, false, true).is_none());
        assert!(connection_editor_inline_panel_draft(&None, false, false).is_none());
        assert!(connection_editor_window_open_or_pending(true, false));
        assert!(connection_editor_window_open_or_pending(false, true));
        assert!(!connection_editor_window_open_or_pending(false, false));
    }

    #[test]
    fn clear_connection_editor_group_menu_draft_resets_group_field_only() {
        let mut draft = Some(ConnectionEditorState {
            new_group_name: "scratch".to_string(),
            focused_field: ConnectionEditorField::NewGroupName,
            error: Some("keep validation".to_string()),
            ..connection_editor_state_with_secret_draft()
        });

        clear_connection_editor_group_menu_draft(&mut draft);

        let editor = draft.expect("editor remains open");
        assert!(editor.new_group_name.is_empty());
        assert_eq!(editor.focused_field, ConnectionEditorField::Name);
        assert_eq!(editor.error.as_deref(), Some("keep validation"));
    }

    #[test]
    fn editing_a_field_clears_a_stale_validation_error() {
        let mut draft = ConnectionEditorState {
            error: Some("SSH host is required".to_string()),
            ..connection_editor_state_with_secret_draft()
        };

        set_connection_editor_field_text(
            &mut draft,
            ConnectionEditorField::Host,
            "10.0.0.5".to_string(),
        );

        assert_eq!(draft.host, "10.0.0.5");
        assert_eq!(draft.error, None);
    }

    #[test]
    fn set_connection_editor_icon_trims_empty_values_and_clears_error() {
        let mut draft = Some(ConnectionEditorState {
            error: Some("stale validation".to_string()),
            ..connection_editor_state_with_secret_draft()
        });

        assert!(set_connection_editor_icon(&mut draft, Some("  server  ")));
        assert_eq!(
            draft.as_ref().and_then(|editor| editor.icon.as_deref()),
            Some("server")
        );
        assert_eq!(
            draft.as_ref().and_then(|editor| editor.error.as_deref()),
            None
        );

        assert!(set_connection_editor_icon(&mut draft, Some("  ")));
        assert_eq!(
            draft.as_ref().and_then(|editor| editor.icon.as_deref()),
            None
        );
    }

    #[test]
    fn set_connection_editor_auth_none_clears_password_and_key_state() {
        let mut draft = Some(ConnectionEditorState {
            password_id: Some("saved-password".to_string()),
            key_id: Some("key-a".to_string()),
            error: Some("stale validation".to_string()),
            ..connection_editor_state_with_secret_draft()
        });

        assert!(set_connection_editor_menu_value(
            &mut draft,
            ConnectionEditorMenu::Authentication,
            Some("none".to_string()),
        ));

        let editor = draft.expect("editor remains open");
        assert_eq!(editor.auth_mode, "none");
        assert_eq!(editor.password_source, ConnectionEditorPasswordSource::Ask);
        assert_eq!(editor.password_id, None);
        assert!(editor.password.is_empty());
        assert_eq!(editor.existing_password, None);
        assert_eq!(editor.key_id, None);
        assert_eq!(editor.error, None);
    }

    #[test]
    fn set_connection_editor_group_menu_value_clears_group_draft() {
        let mut draft = Some(ConnectionEditorState {
            new_group_name: "scratch".to_string(),
            pending_group_name: Some("pending".to_string()),
            pending_group_parent_id: Some("parent".to_string()),
            focused_field: ConnectionEditorField::NewGroupName,
            error: Some("stale validation".to_string()),
            ..connection_editor_state_with_secret_draft()
        });

        assert!(set_connection_editor_menu_value(
            &mut draft,
            ConnectionEditorMenu::Group,
            Some("group-a".to_string()),
        ));

        let editor = draft.expect("editor remains open");
        assert_eq!(editor.group_id.as_deref(), Some("group-a"));
        assert!(editor.new_group_name.is_empty());
        assert_eq!(editor.pending_group_name, None);
        assert_eq!(editor.pending_group_parent_id, None);
        assert_eq!(editor.focused_field, ConnectionEditorField::Name);
        assert_eq!(editor.error, None);
    }

    #[test]
    fn set_connection_editor_password_source_clears_secret_drafts() {
        let mut draft = Some(ConnectionEditorState {
            password_id: Some("saved-password".to_string()),
            ..connection_editor_state_with_secret_draft()
        });

        assert!(set_connection_editor_password_source(
            &mut draft,
            ConnectionEditorPasswordSource::Saved
        ));

        let editor = draft.as_ref().expect("editor remains open");
        assert_eq!(
            editor.password_source,
            ConnectionEditorPasswordSource::Saved
        );
        assert!(editor.password.is_empty());
        assert_eq!(editor.existing_password, None);

        assert!(set_connection_editor_password_source(
            &mut draft,
            ConnectionEditorPasswordSource::Ask
        ));

        let editor = draft.expect("editor remains open");
        assert_eq!(editor.password_source, ConnectionEditorPasswordSource::Ask);
        assert_eq!(editor.password_id, None);
        assert!(editor.password.is_empty());
        assert_eq!(editor.existing_password, None);
    }

    #[test]
    fn set_connection_editor_advanced_tab_resets_hidden_post_login_focus() {
        let mut draft = Some(ConnectionEditorState {
            focused_field: ConnectionEditorField::PostLoginCommand,
            advanced_behavior_tab: ConnectionEditorAdvancedTab::PostLogin,
            ..connection_editor_state_with_secret_draft()
        });

        assert!(set_connection_editor_advanced_tab(
            &mut draft,
            ConnectionEditorAdvancedTab::X11
        ));

        let editor = draft.expect("editor remains open");
        assert_eq!(
            editor.advanced_behavior_tab,
            ConnectionEditorAdvancedTab::X11
        );
        assert_eq!(editor.focused_field, ConnectionEditorField::Name);
    }

    #[test]
    fn set_connection_editor_kind_updates_default_ports_and_clears_error() {
        let mut draft = Some(ConnectionEditorState {
            port: "22".to_string(),
            error: Some("stale validation".to_string()),
            ..connection_editor_state_with_secret_draft()
        });

        assert!(set_connection_editor_kind(
            &mut draft,
            ConnectionKindTab::Telnet
        ));

        let editor = draft.expect("editor remains open");
        assert_eq!(editor.kind, ConnectionKindTab::Telnet);
        assert_eq!(editor.port, "23");
        assert_eq!(editor.focused_field, ConnectionEditorField::Name);
        assert_eq!(editor.error, None);
    }

    #[test]
    fn set_connection_editor_telnet_tab_clears_error() {
        let mut draft = Some(ConnectionEditorState {
            error: Some("stale validation".to_string()),
            ..connection_editor_state_with_secret_draft()
        });

        assert!(set_connection_editor_telnet_tab(
            &mut draft,
            ConnectionEditorTelnetTab::Compatibility
        ));

        let editor = draft.expect("editor remains open");
        assert_eq!(
            editor.telnet_advanced_tab,
            ConnectionEditorTelnetTab::Compatibility
        );
        assert_eq!(editor.error, None);
    }

    #[test]
    fn stepped_menu_highlight_wraps_at_both_ends() {
        assert_eq!(stepped_menu_highlight(0, 1, 3), Some(1));
        assert_eq!(stepped_menu_highlight(2, 1, 3), Some(0));
        assert_eq!(stepped_menu_highlight(0, -1, 3), Some(2));
    }

    #[test]
    fn stepped_menu_highlight_refuses_an_empty_list() {
        assert_eq!(stepped_menu_highlight(0, 1, 0), None);
    }

    #[test]
    fn stepped_menu_highlight_clamps_a_highlight_left_over_from_a_longer_list() {
        // Switching the connection kind swaps every select underneath.
        assert_eq!(stepped_menu_highlight(9, 1, 3), Some(0));
        assert_eq!(stepped_menu_highlight(9, -1, 3), Some(1));
    }

    #[test]
    fn commit_connection_editor_new_group_requires_non_empty_name() {
        let mut draft = Some(ConnectionEditorState {
            new_group_name: "  ".to_string(),
            error: None,
            ..connection_editor_state_with_secret_draft()
        });

        assert!(commit_connection_editor_new_group(
            &mut draft,
            "Group name is required".to_string()
        ));

        let editor = draft.expect("editor remains open");
        assert_eq!(editor.error.as_deref(), Some("Group name is required"));
        assert_eq!(editor.pending_group_name, None);
    }

    #[test]
    fn commit_connection_editor_new_group_captures_parent_and_clears_draft() {
        let mut draft = Some(ConnectionEditorState {
            group_id: Some("parent".to_string()),
            new_group_name: "  staging  ".to_string(),
            focused_field: ConnectionEditorField::NewGroupName,
            error: Some("stale validation".to_string()),
            ..connection_editor_state_with_secret_draft()
        });

        assert!(commit_connection_editor_new_group(
            &mut draft,
            "Group name is required".to_string()
        ));

        let editor = draft.expect("editor remains open");
        assert_eq!(editor.pending_group_name.as_deref(), Some("staging"));
        assert_eq!(editor.pending_group_parent_id.as_deref(), Some("parent"));
        assert_eq!(editor.group_id, None);
        assert!(editor.new_group_name.is_empty());
        assert_eq!(editor.focused_field, ConnectionEditorField::Name);
        assert_eq!(editor.error, None);
    }

    #[test]
    fn toggle_connection_editor_raw_tcp_forces_cr_enter_mode() {
        let mut draft = Some(ConnectionEditorState {
            raw_tcp_cli: false,
            telnet_enter_mode: "lf".to_string(),
            error: Some("stale validation".to_string()),
            ..connection_editor_state_with_secret_draft()
        });

        assert!(toggle_connection_editor_flag(
            &mut draft,
            ConnectionEditorToggle::RawTcp
        ));

        let editor = draft.expect("editor remains open");
        assert!(editor.raw_tcp_cli);
        assert_eq!(editor.telnet_enter_mode, "cr");
        assert_eq!(editor.error, None);
    }

    #[test]
    fn toggle_connection_editor_advanced_closed_resets_hidden_focus() {
        let mut draft = Some(ConnectionEditorState {
            advanced_open: true,
            focused_field: ConnectionEditorField::PostLoginDelay,
            error: Some("stale validation".to_string()),
            ..connection_editor_state_with_secret_draft()
        });

        assert!(toggle_connection_editor_flag(
            &mut draft,
            ConnectionEditorToggle::Advanced
        ));

        let editor = draft.expect("editor remains open");
        assert!(!editor.advanced_open);
        assert_eq!(editor.focused_field, ConnectionEditorField::Name);
        assert_eq!(editor.error, None);
    }

    #[test]
    fn connection_editor_text_key_filters_numeric_fields_and_clears_error() {
        let mut draft = Some(ConnectionEditorState {
            focused_field: ConnectionEditorField::Port,
            port: String::new(),
            error: Some("stale validation".to_string()),
            ..connection_editor_state_with_secret_draft()
        });

        assert!(apply_connection_editor_text_key(
            &mut draft,
            "7",
            Some("7x8")
        ));

        let editor = draft.expect("editor remains open");
        assert_eq!(editor.port, "78");
        assert_eq!(editor.error, None);
    }

    #[test]
    fn connection_editor_text_key_backspace_updates_focused_field() {
        let mut draft = Some(ConnectionEditorState {
            focused_field: ConnectionEditorField::Name,
            name: "prod".to_string(),
            error: Some("stale validation".to_string()),
            ..connection_editor_state_with_secret_draft()
        });

        assert!(apply_connection_editor_text_key(
            &mut draft,
            "backspace",
            None
        ));

        let editor = draft.expect("editor remains open");
        assert_eq!(editor.name, "pro");
        assert_eq!(editor.error, None);
    }

    #[test]
    fn insert_connection_editor_description_newline_only_when_description_focused() {
        let mut draft = Some(ConnectionEditorState {
            focused_field: ConnectionEditorField::Description,
            description: "first".to_string(),
            error: Some("stale validation".to_string()),
            ..connection_editor_state_with_secret_draft()
        });

        assert!(insert_connection_editor_description_newline(&mut draft));

        let editor = draft.as_ref().expect("editor remains open");
        assert_eq!(editor.description, "first\n");
        assert_eq!(editor.error, None);

        draft.as_mut().expect("editor remains open").focused_field = ConnectionEditorField::Name;
        assert!(!insert_connection_editor_description_newline(&mut draft));
    }

    #[test]
    fn set_connection_editor_error_updates_active_draft() {
        let mut draft = Some(connection_editor_state_with_secret_draft());

        assert!(set_connection_editor_error(
            &mut draft,
            "SSH host is required".to_string()
        ));

        assert_eq!(
            draft.and_then(|editor| editor.error),
            Some("SSH host is required".to_string())
        );
    }

    #[test]
    fn apply_connection_editor_paths_update_field_and_clear_error() {
        let mut draft = Some(ConnectionEditorState {
            error: Some("stale validation".to_string()),
            ..connection_editor_state_with_secret_draft()
        });

        assert!(apply_connection_editor_shell_path(
            &mut draft,
            "/bin/zsh".to_string()
        ));
        assert!(apply_connection_editor_working_dir(
            &mut draft,
            "/home/kang".to_string()
        ));

        let editor = draft.expect("editor remains open");
        assert_eq!(editor.shell_path, "/bin/zsh");
        assert_eq!(editor.working_dir, "/home/kang");
        assert_eq!(editor.error, None);
    }

    #[test]
    fn connection_group_editor_name_key_updates_name_and_clears_error() {
        let mut draft = Some(ConnectionGroupEditorState {
            id: Some("group-a".to_string()),
            name: "prod".to_string(),
            parent_id: Some("root".to_string()),
            error: Some("stale validation".to_string()),
        });

        assert!(apply_connection_group_editor_name_key(
            &mut draft,
            "x",
            Some("x")
        ));
        assert!(apply_connection_group_editor_name_key(
            &mut draft,
            "backspace",
            None
        ));

        let editor = draft.expect("group editor remains open");
        assert_eq!(editor.name, "prod");
        assert_eq!(editor.parent_id.as_deref(), Some("root"));
        assert_eq!(editor.error, None);
    }

    #[test]
    fn connection_group_editor_name_key_ignores_empty_input_without_draft() {
        let mut draft = Some(ConnectionGroupEditorState {
            id: None,
            name: "root".to_string(),
            parent_id: None,
            error: Some("keep validation".to_string()),
        });

        assert!(!apply_connection_group_editor_name_key(
            &mut draft, "shift", None
        ));
        assert_eq!(
            draft.as_ref().and_then(|editor| editor.error.as_deref()),
            Some("keep validation")
        );

        let mut missing = None;
        assert!(!apply_connection_group_editor_name_key(
            &mut missing,
            "x",
            Some("x")
        ));
    }

    #[test]
    fn set_connection_group_editor_error_updates_active_draft() {
        let mut draft = Some(ConnectionGroupEditorState {
            id: None,
            name: String::new(),
            parent_id: None,
            error: None,
        });

        assert!(set_connection_group_editor_error(
            &mut draft,
            "Folder name is required".to_string()
        ));

        assert_eq!(
            draft.and_then(|editor| editor.error),
            Some("Folder name is required".to_string())
        );
    }

    #[test]
    fn network_group_editor_name_key_updates_name_and_clears_error() {
        let mut group_editor = Some(NetworkGroupEditorState {
            tab: NetworkTab::Tunnels,
            id: Some("group-a".to_string()),
            name: "prod".to_string(),
            error: Some("stale validation".to_string()),
        });

        assert!(apply_network_group_editor_name_key(
            &mut group_editor,
            "x",
            Some("x")
        ));
        assert!(apply_network_group_editor_name_key(
            &mut group_editor,
            "backspace",
            None
        ));

        let editor = group_editor.expect("network group editor remains open");
        assert_eq!(editor.name, "prod");
        assert_eq!(editor.error, None);
    }

    #[test]
    fn set_network_group_editor_error_updates_active_draft() {
        let mut group_editor = Some(NetworkGroupEditorState {
            tab: NetworkTab::Proxies,
            id: None,
            name: String::new(),
            error: None,
        });

        assert!(set_network_group_editor_error(
            &mut group_editor,
            "Group name is required".to_string()
        ));

        assert_eq!(
            group_editor.and_then(|editor| editor.error),
            Some("Group name is required".to_string())
        );
    }

    #[test]
    fn network_tunnel_editor_key_filters_ports_and_clears_error() {
        let mut tunnel_editor = Some(NetworkTunnelEditorState {
            focused_field: NetworkTunnelEditorField::ListenPort,
            listen_port: String::new(),
            error: Some("stale validation".to_string()),
            ..network_tunnel_editor("tunnel-a")
        });

        assert!(apply_network_tunnel_editor_key(
            &mut tunnel_editor,
            "8",
            Some("8x0")
        ));

        let editor = tunnel_editor.expect("tunnel editor remains open");
        assert_eq!(editor.listen_port, "80");
        assert_eq!(editor.error, None);
    }

    #[test]
    fn network_tunnel_type_cycle_resets_hidden_dynamic_focus() {
        let mut tunnel_editor = Some(NetworkTunnelEditorState {
            tunnel_type: "remote".to_string(),
            focused_field: NetworkTunnelEditorField::TargetPort,
            error: Some("stale validation".to_string()),
            ..network_tunnel_editor("tunnel-a")
        });

        assert_eq!(
            cycle_network_tunnel_type(&mut tunnel_editor).as_deref(),
            Some("dynamic")
        );

        let editor = tunnel_editor.expect("tunnel editor remains open");
        assert_eq!(editor.tunnel_type, "dynamic");
        assert_eq!(editor.focused_field, NetworkTunnelEditorField::ListenPort);
        assert_eq!(editor.error, None);
    }

    #[test]
    fn network_tunnel_cycles_connection_group_and_flags() {
        let mut tunnel_editor = Some(NetworkTunnelEditorState {
            connection_id: Some("conn-a".to_string()),
            group_id: None,
            auto_open: false,
            bind_localhost: false,
            error: Some("stale validation".to_string()),
            ..network_tunnel_editor("tunnel-a")
        });

        assert!(cycle_network_tunnel_connection(
            &mut tunnel_editor,
            ["conn-a", "conn-b"]
        ));
        assert!(cycle_network_tunnel_group(&mut tunnel_editor, ["group-a"]));
        assert!(set_network_tunnel_bind_localhost(&mut tunnel_editor, true));
        assert_eq!(
            toggle_network_tunnel_auto_open(&mut tunnel_editor),
            Some(true)
        );

        let editor = tunnel_editor.expect("tunnel editor remains open");
        assert_eq!(editor.connection_id.as_deref(), Some("conn-b"));
        assert_eq!(editor.group_id.as_deref(), Some("group-a"));
        assert!(editor.bind_localhost);
        assert!(editor.auto_open);
        assert_eq!(editor.error, None);
    }

    #[test]
    fn network_tunnel_focus_advances_with_dynamic_rules() {
        let mut tunnel_editor = Some(NetworkTunnelEditorState {
            tunnel_type: "dynamic".to_string(),
            focused_field: NetworkTunnelEditorField::ListenPort,
            error: Some("stale validation".to_string()),
            ..network_tunnel_editor("tunnel-a")
        });

        assert!(advance_network_tunnel_editor_focus(&mut tunnel_editor));

        let editor = tunnel_editor.expect("tunnel editor remains open");
        assert_eq!(editor.focused_field, NetworkTunnelEditorField::Name);
        assert_eq!(editor.error, None);
    }

    #[test]
    fn network_proxy_editor_key_filters_port_and_preserves_password_draft() {
        let mut proxy_editor = Some(NetworkProxyEditorState {
            focused_field: NetworkProxyEditorField::Port,
            port: String::new(),
            password: "draft-password".to_string(),
            existing_password: Some("existing-password".to_string()),
            error: Some("stale validation".to_string()),
            ..network_proxy_editor("proxy-a")
        });

        assert!(apply_network_proxy_editor_key(
            &mut proxy_editor,
            "1",
            Some("1x2")
        ));

        let editor = proxy_editor.expect("proxy editor remains open");
        assert_eq!(editor.port, "12");
        assert_eq!(editor.password, "draft-password");
        assert_eq!(
            editor.existing_password.as_deref(),
            Some("existing-password")
        );
        assert_eq!(editor.error, None);
    }

    #[test]
    fn network_proxy_command_newline_only_when_command_is_focused() {
        let mut proxy_editor = Some(NetworkProxyEditorState {
            focused_field: NetworkProxyEditorField::Name,
            command: "ssh -W".to_string(),
            error: Some("keep validation".to_string()),
            ..network_proxy_editor("proxy-a")
        });

        assert!(!insert_network_proxy_command_newline(&mut proxy_editor));
        assert_eq!(
            proxy_editor
                .as_ref()
                .and_then(|editor| editor.error.as_deref()),
            Some("keep validation")
        );

        proxy_editor
            .as_mut()
            .expect("proxy editor remains open")
            .focused_field = NetworkProxyEditorField::Command;
        assert!(insert_network_proxy_command_newline(&mut proxy_editor));

        let editor = proxy_editor.expect("proxy editor remains open");
        assert_eq!(editor.command, "ssh -W\n");
        assert_eq!(editor.error, None);
    }

    #[test]
    fn network_proxy_protocol_cycle_resets_hidden_focus() {
        let mut proxy_editor = Some(NetworkProxyEditorState {
            protocol: "http".to_string(),
            focused_field: NetworkProxyEditorField::Password,
            error: Some("stale validation".to_string()),
            ..network_proxy_editor("proxy-a")
        });

        assert_eq!(
            cycle_network_proxy_protocol(&mut proxy_editor).as_deref(),
            Some("proxycommand")
        );

        let editor = proxy_editor.expect("proxy editor remains open");
        assert_eq!(editor.protocol, "proxycommand");
        assert_eq!(editor.focused_field, NetworkProxyEditorField::Command);
        assert_eq!(editor.error, None);
    }

    #[test]
    fn network_proxy_focus_and_group_cycle_use_editor_rules() {
        let mut proxy_editor = Some(NetworkProxyEditorState {
            protocol: "proxycommand".to_string(),
            focused_field: NetworkProxyEditorField::Command,
            group_id: None,
            error: Some("stale validation".to_string()),
            ..network_proxy_editor("proxy-a")
        });

        assert!(advance_network_proxy_editor_focus(&mut proxy_editor));
        assert!(cycle_network_proxy_group(&mut proxy_editor, ["group-a"]));

        let editor = proxy_editor.expect("proxy editor remains open");
        assert_eq!(editor.focused_field, NetworkProxyEditorField::Name);
        assert_eq!(editor.group_id.as_deref(), Some("group-a"));
        assert_eq!(editor.error, None);
    }

    #[test]
    fn toggle_network_item_menu_closes_move_picker_and_toggles_same_item() {
        let mut item_menu = None;
        let mut move_picker = Some(NetworkMovePickerState {
            tab: NetworkTab::Tunnels,
            id: "old".to_string(),
        });

        assert!(toggle_network_item_menu_state(
            &mut item_menu,
            &mut move_picker,
            NetworkTab::Tunnels,
            "one".to_string(),
        ));

        assert_eq!(
            item_menu,
            Some(NetworkItemMenuState {
                tab: NetworkTab::Tunnels,
                id: "one".to_string(),
            })
        );
        assert_eq!(move_picker, None);

        assert!(!toggle_network_item_menu_state(
            &mut item_menu,
            &mut move_picker,
            NetworkTab::Tunnels,
            "one".to_string(),
        ));

        assert_eq!(item_menu, None);
    }

    #[test]
    fn toggle_network_move_picker_closes_item_menu_and_toggles_same_item() {
        let mut item_menu = Some(NetworkItemMenuState {
            tab: NetworkTab::Proxies,
            id: "proxy".to_string(),
        });
        let mut move_picker = None;

        assert!(toggle_network_move_picker_state(
            &mut item_menu,
            &mut move_picker,
            NetworkTab::Proxies,
            "proxy".to_string(),
        ));

        assert_eq!(item_menu, None);
        assert_eq!(
            move_picker,
            Some(NetworkMovePickerState {
                tab: NetworkTab::Proxies,
                id: "proxy".to_string(),
            })
        );

        assert!(!toggle_network_move_picker_state(
            &mut item_menu,
            &mut move_picker,
            NetworkTab::Proxies,
            "proxy".to_string(),
        ));

        assert_eq!(move_picker, None);
    }

    #[test]
    fn remove_network_item_references_clears_only_matching_tab_and_id() {
        let mut delete_confirm = Some(NetworkDeleteConfirmState {
            tab: NetworkTab::Tunnels,
            id: "one".to_string(),
            label: "One".to_string(),
        });
        let mut item_menu = Some(NetworkItemMenuState {
            tab: NetworkTab::Tunnels,
            id: "one".to_string(),
        });
        let mut move_picker = Some(NetworkMovePickerState {
            tab: NetworkTab::Tunnels,
            id: "one".to_string(),
        });
        let mut tunnel_editor = Some(network_tunnel_editor("one"));
        let mut proxy_editor = Some(network_proxy_editor("one"));

        remove_network_item_references(
            &mut delete_confirm,
            &mut item_menu,
            &mut move_picker,
            &mut tunnel_editor,
            &mut proxy_editor,
            NetworkTab::Tunnels,
            "one",
        );

        assert_eq!(delete_confirm, None);
        assert_eq!(item_menu, None);
        assert_eq!(move_picker, None);
        assert_eq!(tunnel_editor, None);
        assert_eq!(proxy_editor, Some(network_proxy_editor("one")));
    }

    #[test]
    fn remove_network_group_references_clears_matching_group_state() {
        let mut group_editor = Some(NetworkGroupEditorState {
            tab: NetworkTab::Tunnels,
            id: Some("group-a".to_string()),
            name: "Group A".to_string(),
            error: Some("stale".to_string()),
        });
        let mut group_delete_confirm = Some(NetworkGroupDeleteConfirmState {
            tab: NetworkTab::Tunnels,
            id: "group-a".to_string(),
            label: "Group A".to_string(),
            item_count: 2,
        });
        let mut expanded_sections = HashSet::from([
            "tunnel:group-a".to_string(),
            "proxy:group-a".to_string(),
            "tunnel:group-b".to_string(),
        ]);

        remove_network_group_references(
            &mut group_editor,
            &mut group_delete_confirm,
            &mut expanded_sections,
            NetworkTab::Tunnels,
            "group-a",
        );

        assert_eq!(group_editor, None);
        assert_eq!(group_delete_confirm, None);
        assert_eq!(
            expanded_sections,
            HashSet::from(["proxy:group-a".to_string(), "tunnel:group-b".to_string()])
        );
    }

    #[test]
    fn remove_network_group_and_item_references_clears_deleted_child_state() {
        let mut group_editor = Some(NetworkGroupEditorState {
            tab: NetworkTab::Proxies,
            id: Some("group-a".to_string()),
            name: "Group A".to_string(),
            error: None,
        });
        let mut group_delete_confirm = Some(NetworkGroupDeleteConfirmState {
            tab: NetworkTab::Proxies,
            id: "group-a".to_string(),
            label: "Group A".to_string(),
            item_count: 1,
        });
        let mut expanded_sections = HashSet::from(["proxy:group-a".to_string()]);
        let mut delete_confirm = Some(NetworkDeleteConfirmState {
            tab: NetworkTab::Proxies,
            id: "proxy-a".to_string(),
            label: "Proxy A".to_string(),
        });
        let mut item_menu = Some(NetworkItemMenuState {
            tab: NetworkTab::Proxies,
            id: "proxy-a".to_string(),
        });
        let mut move_picker = Some(NetworkMovePickerState {
            tab: NetworkTab::Proxies,
            id: "proxy-a".to_string(),
        });
        let mut tunnel_editor = Some(network_tunnel_editor("proxy-a"));
        let mut proxy_editor = Some(network_proxy_editor("proxy-a"));

        remove_network_group_and_item_references(
            &mut group_editor,
            &mut group_delete_confirm,
            &mut expanded_sections,
            &mut delete_confirm,
            &mut item_menu,
            &mut move_picker,
            &mut tunnel_editor,
            &mut proxy_editor,
            NetworkTab::Proxies,
            "group-a",
            &["proxy-a".to_string()],
        );

        assert_eq!(group_editor, None);
        assert_eq!(group_delete_confirm, None);
        assert!(expanded_sections.is_empty());
        assert_eq!(delete_confirm, None);
        assert_eq!(item_menu, None);
        assert_eq!(move_picker, None);
        assert_eq!(proxy_editor, None);
        assert_eq!(tunnel_editor, Some(network_tunnel_editor("proxy-a")));
    }

    #[test]
    fn clear_network_tunnel_editor_closes_active_draft() {
        let mut tunnel_editor = Some(network_tunnel_editor("tunnel-a"));

        clear_network_tunnel_editor(&mut tunnel_editor);

        assert_eq!(tunnel_editor, None);
    }

    #[test]
    fn focus_network_tunnel_editor_field_clears_existing_error() {
        let mut tunnel_editor = Some(NetworkTunnelEditorState {
            error: Some("stale validation".to_string()),
            ..network_tunnel_editor("tunnel-a")
        });

        assert!(focus_network_tunnel_editor_field(
            &mut tunnel_editor,
            NetworkTunnelEditorField::TargetPort
        ));

        let editor = tunnel_editor.expect("tunnel editor remains open");
        assert_eq!(editor.focused_field, NetworkTunnelEditorField::TargetPort);
        assert_eq!(editor.error, None);
    }

    #[test]
    fn set_network_tunnel_editor_error_updates_active_editor() {
        let mut tunnel_editor = Some(network_tunnel_editor("tunnel-a"));

        assert!(set_network_tunnel_editor_error(
            &mut tunnel_editor,
            "Tunnel name is required".to_string()
        ));

        assert_eq!(
            tunnel_editor.and_then(|editor| editor.error),
            Some("Tunnel name is required".to_string())
        );
    }

    #[test]
    fn clear_network_proxy_editor_clears_secret_draft() {
        let mut proxy_editor = Some(network_proxy_editor("proxy-a"));

        clear_network_proxy_editor(&mut proxy_editor);

        assert_eq!(proxy_editor, None);
    }

    #[test]
    fn focus_network_proxy_editor_field_clears_existing_error() {
        let mut proxy_editor = Some(NetworkProxyEditorState {
            error: Some("stale validation".to_string()),
            ..network_proxy_editor("proxy-a")
        });

        assert!(focus_network_proxy_editor_field(
            &mut proxy_editor,
            NetworkProxyEditorField::Password
        ));

        let editor = proxy_editor.expect("proxy editor remains open");
        assert_eq!(editor.focused_field, NetworkProxyEditorField::Password);
        assert_eq!(editor.error, None);
    }

    #[test]
    fn set_network_proxy_editor_error_updates_active_editor() {
        let mut proxy_editor = Some(network_proxy_editor("proxy-a"));

        assert!(set_network_proxy_editor_error(
            &mut proxy_editor,
            "Proxy host is required".to_string()
        ));

        assert_eq!(
            proxy_editor.and_then(|editor| editor.error),
            Some("Proxy host is required".to_string())
        );
    }

    fn connection_editor_state_with_secret_draft() -> ConnectionEditorState {
        ConnectionEditorState {
            id: Some("conn".to_string()),
            kind: ConnectionKindTab::Ssh,
            name: "prod".to_string(),
            description: String::new(),
            icon: None,
            icon_auto_detect: true,
            group_id: None,
            new_group_name: String::new(),
            pending_group_name: None,
            pending_group_parent_id: None,
            host: "example.test".to_string(),
            port: "22".to_string(),
            username: "root".to_string(),
            auth_mode: "password".to_string(),
            password_source: ConnectionEditorPasswordSource::Direct,
            password_id: None,
            password: "draft-secret".to_string(),
            existing_password: Some("existing-secret".to_string()),
            key_id: None,
            otp_id: None,
            auto_fill_otp: false,
            proxy_id: None,
            proxy_jump_id: None,
            x11_forwarding: false,
            backspace_mode: "del".to_string(),
            shell_path: String::new(),
            shell_args: String::new(),
            working_dir: String::new(),
            serial_port: String::new(),
            baud_rate: "115200".to_string(),
            data_bits: "8".to_string(),
            parity: "none".to_string(),
            stop_bits: "1".to_string(),
            raw_tcp_cli: false,
            telnet_enter_mode: "cr".to_string(),
            local_echo: false,
            local_line_edit: false,
            force_character_at_a_time: false,
            send_naws: true,
            send_sga: true,
            post_login_enabled: false,
            post_login_command: String::new(),
            post_login_delay_ms: "1000".to_string(),
            advanced_open: false,
            advanced_network_tab: ConnectionEditorAdvancedTab::Proxy,
            advanced_behavior_tab: ConnectionEditorAdvancedTab::PostLogin,
            telnet_advanced_tab: ConnectionEditorTelnetTab::Input,
            connect_after_save: false,
            focused_field: ConnectionEditorField::Name,
            error: None,
        }
    }

    fn network_tunnel_editor(id: &str) -> NetworkTunnelEditorState {
        NetworkTunnelEditorState {
            id: Some(id.to_string()),
            is_open: false,
            name: "Tunnel".to_string(),
            tunnel_type: "local".to_string(),
            connection_id: Some("conn".to_string()),
            listen_port: "8080".to_string(),
            target_host: "127.0.0.1".to_string(),
            target_port: "80".to_string(),
            auto_open: false,
            bind_localhost: true,
            group_id: None,
            focused_field: NetworkTunnelEditorField::Name,
            error: None,
        }
    }

    fn network_proxy_editor(id: &str) -> NetworkProxyEditorState {
        NetworkProxyEditorState {
            id: Some(id.to_string()),
            name: "Proxy".to_string(),
            protocol: "socks5".to_string(),
            host: "127.0.0.1".to_string(),
            port: "1080".to_string(),
            command: String::new(),
            username: String::new(),
            password: "draft-password".to_string(),
            existing_password: Some("existing-password".to_string()),
            password_id: None,
            group_id: None,
            focused_field: NetworkProxyEditorField::Name,
            error: None,
        }
    }
}

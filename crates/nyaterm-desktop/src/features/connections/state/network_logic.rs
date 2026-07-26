use std::collections::HashSet;

use crate::models::{
    NetworkDeleteConfirmState, NetworkGroupDeleteConfirmState, NetworkGroupEditorState,
    NetworkItemMenuState, NetworkMovePickerState, NetworkProxyEditorField, NetworkProxyEditorState,
    NetworkTab, NetworkTunnelEditorField, NetworkTunnelEditorState,
};

pub(super) fn apply_network_group_editor_name_key(
    group_editor: &mut Option<NetworkGroupEditorState>,
    key: &str,
    input: Option<&str>,
) -> bool {
    let Some(editor) = group_editor.as_mut() else {
        return false;
    };
    match key {
        "backspace" => {
            editor.name.pop();
            editor.error = None;
            true
        }
        _ => {
            let Some(input) = input.filter(|input| !input.is_empty()) else {
                return false;
            };
            editor.name.push_str(input);
            editor.error = None;
            true
        }
    }
}

pub(super) fn set_network_group_editor_error(
    group_editor: &mut Option<NetworkGroupEditorState>,
    error: String,
) -> bool {
    let Some(editor) = group_editor.as_mut() else {
        return false;
    };
    editor.error = Some(error);
    true
}

pub(super) fn toggle_network_item_menu_state(
    item_menu: &mut Option<NetworkItemMenuState>,
    move_picker: &mut Option<NetworkMovePickerState>,
    tab: NetworkTab,
    id: String,
) -> bool {
    if item_menu
        .as_ref()
        .is_some_and(|menu| menu.tab == tab && menu.id == id)
    {
        *item_menu = None;
        return false;
    }

    *item_menu = Some(NetworkItemMenuState { tab, id });
    *move_picker = None;
    true
}

pub(super) fn toggle_network_move_picker_state(
    item_menu: &mut Option<NetworkItemMenuState>,
    move_picker: &mut Option<NetworkMovePickerState>,
    tab: NetworkTab,
    id: String,
) -> bool {
    *item_menu = None;
    if move_picker
        .as_ref()
        .is_some_and(|picker| picker.tab == tab && picker.id == id)
    {
        *move_picker = None;
        return false;
    }

    *move_picker = Some(NetworkMovePickerState { tab, id });
    true
}

pub(super) fn remove_network_item_references(
    delete_confirm: &mut Option<NetworkDeleteConfirmState>,
    item_menu: &mut Option<NetworkItemMenuState>,
    move_picker: &mut Option<NetworkMovePickerState>,
    tunnel_editor: &mut Option<NetworkTunnelEditorState>,
    proxy_editor: &mut Option<NetworkProxyEditorState>,
    tab: NetworkTab,
    id: &str,
) {
    if delete_confirm
        .as_ref()
        .is_some_and(|confirm| confirm.tab == tab && confirm.id == id)
    {
        *delete_confirm = None;
    }
    if item_menu
        .as_ref()
        .is_some_and(|menu| menu.tab == tab && menu.id == id)
    {
        *item_menu = None;
    }
    if move_picker
        .as_ref()
        .is_some_and(|picker| picker.tab == tab && picker.id == id)
    {
        *move_picker = None;
    }
    match tab {
        NetworkTab::Tunnels => {
            if tunnel_editor
                .as_ref()
                .is_some_and(|editor| editor.id.as_deref() == Some(id))
            {
                *tunnel_editor = None;
            }
        }
        NetworkTab::Proxies => {
            if proxy_editor
                .as_ref()
                .is_some_and(|editor| editor.id.as_deref() == Some(id))
            {
                *proxy_editor = None;
            }
        }
    }
}

pub(super) fn remove_network_group_references(
    group_editor: &mut Option<NetworkGroupEditorState>,
    group_delete_confirm: &mut Option<NetworkGroupDeleteConfirmState>,
    expanded_sections: &mut HashSet<String>,
    tab: NetworkTab,
    group_id: &str,
) {
    expanded_sections.remove(&network_section_key(tab, group_id));
    if group_editor
        .as_ref()
        .is_some_and(|editor| editor.tab == tab && editor.id.as_deref() == Some(group_id))
    {
        *group_editor = None;
    }
    if group_delete_confirm
        .as_ref()
        .is_some_and(|confirm| confirm.tab == tab && confirm.id == group_id)
    {
        *group_delete_confirm = None;
    }
}

pub(super) fn remove_network_group_and_item_references(
    group_editor: &mut Option<NetworkGroupEditorState>,
    group_delete_confirm: &mut Option<NetworkGroupDeleteConfirmState>,
    expanded_sections: &mut HashSet<String>,
    delete_confirm: &mut Option<NetworkDeleteConfirmState>,
    item_menu: &mut Option<NetworkItemMenuState>,
    move_picker: &mut Option<NetworkMovePickerState>,
    tunnel_editor: &mut Option<NetworkTunnelEditorState>,
    proxy_editor: &mut Option<NetworkProxyEditorState>,
    tab: NetworkTab,
    group_id: &str,
    deleted_item_ids: &[String],
) {
    remove_network_group_references(
        group_editor,
        group_delete_confirm,
        expanded_sections,
        tab,
        group_id,
    );
    for item_id in deleted_item_ids {
        remove_network_item_references(
            delete_confirm,
            item_menu,
            move_picker,
            tunnel_editor,
            proxy_editor,
            tab,
            item_id,
        );
    }
}

pub(super) fn clear_network_tunnel_editor(tunnel_editor: &mut Option<NetworkTunnelEditorState>) {
    *tunnel_editor = None;
}

pub(super) fn focus_network_tunnel_editor_field(
    tunnel_editor: &mut Option<NetworkTunnelEditorState>,
    field: NetworkTunnelEditorField,
) -> bool {
    let Some(editor) = tunnel_editor.as_mut() else {
        return false;
    };
    editor.focused_field = field;
    editor.error = None;
    true
}

pub(super) fn advance_network_tunnel_editor_focus(
    tunnel_editor: &mut Option<NetworkTunnelEditorState>,
) -> bool {
    let Some(editor) = tunnel_editor.as_mut() else {
        return false;
    };
    editor.focused_field = editor.focused_field.next(editor.is_dynamic());
    editor.error = None;
    true
}

/// Write one field of the tunnel draft.
///
/// A port field keeps only digits: the boxes accept anything typed, and the
/// draft is what gets validated and saved.
pub(super) fn set_network_tunnel_editor_field(
    tunnel_editor: &mut Option<NetworkTunnelEditorState>,
    field: NetworkTunnelEditorField,
    text: String,
) -> bool {
    let Some(editor) = tunnel_editor.as_mut() else {
        return false;
    };
    editor.focused_field = field;
    let text = match field {
        NetworkTunnelEditorField::ListenPort | NetworkTunnelEditorField::TargetPort => {
            text.chars().filter(char::is_ascii_digit).collect()
        }
        _ => text,
    };
    *network_tunnel_editor_field_mut(editor) = text;
    editor.error = None;
    true
}

fn network_tunnel_editor_field_mut(editor: &mut NetworkTunnelEditorState) -> &mut String {
    match editor.focused_field {
        NetworkTunnelEditorField::Name => &mut editor.name,
        NetworkTunnelEditorField::ListenPort => &mut editor.listen_port,
        NetworkTunnelEditorField::TargetHost => &mut editor.target_host,
        NetworkTunnelEditorField::TargetPort => &mut editor.target_port,
    }
}

pub(super) fn cycle_network_tunnel_type(
    tunnel_editor: &mut Option<NetworkTunnelEditorState>,
) -> Option<String> {
    let editor = tunnel_editor.as_mut()?;
    editor.tunnel_type = match editor.tunnel_type.as_str() {
        "local" => "remote",
        "remote" => "dynamic",
        _ => "local",
    }
    .to_string();
    if editor.is_dynamic() {
        editor.focused_field = match editor.focused_field {
            NetworkTunnelEditorField::TargetHost | NetworkTunnelEditorField::TargetPort => {
                NetworkTunnelEditorField::ListenPort
            }
            field => field,
        };
    }
    editor.error = None;
    Some(editor.tunnel_type.clone())
}

pub(super) fn cycle_network_tunnel_connection<'a>(
    tunnel_editor: &mut Option<NetworkTunnelEditorState>,
    connection_ids: impl IntoIterator<Item = &'a str>,
) -> bool {
    let Some(editor) = tunnel_editor.as_mut() else {
        return false;
    };
    editor.connection_id =
        next_network_optional_id(editor.connection_id.as_deref(), connection_ids);
    editor.error = None;
    true
}

pub(super) fn cycle_network_tunnel_group<'a>(
    tunnel_editor: &mut Option<NetworkTunnelEditorState>,
    group_ids: impl IntoIterator<Item = &'a str>,
) -> bool {
    let Some(editor) = tunnel_editor.as_mut() else {
        return false;
    };
    editor.group_id = next_network_optional_id(editor.group_id.as_deref(), group_ids);
    editor.error = None;
    true
}

pub(super) fn set_network_tunnel_bind_localhost(
    tunnel_editor: &mut Option<NetworkTunnelEditorState>,
    bind_localhost: bool,
) -> bool {
    let Some(editor) = tunnel_editor.as_mut() else {
        return false;
    };
    editor.bind_localhost = bind_localhost;
    editor.error = None;
    true
}

pub(super) fn toggle_network_tunnel_auto_open(
    tunnel_editor: &mut Option<NetworkTunnelEditorState>,
) -> Option<bool> {
    let editor = tunnel_editor.as_mut()?;
    editor.auto_open = !editor.auto_open;
    editor.error = None;
    Some(editor.auto_open)
}

pub(super) fn set_network_tunnel_editor_error(
    tunnel_editor: &mut Option<NetworkTunnelEditorState>,
    error: String,
) -> bool {
    let Some(editor) = tunnel_editor.as_mut() else {
        return false;
    };
    editor.error = Some(error);
    true
}

pub(super) fn clear_network_proxy_editor(proxy_editor: &mut Option<NetworkProxyEditorState>) {
    *proxy_editor = None;
}

pub(super) fn focus_network_proxy_editor_field(
    proxy_editor: &mut Option<NetworkProxyEditorState>,
    field: NetworkProxyEditorField,
) -> bool {
    let Some(editor) = proxy_editor.as_mut() else {
        return false;
    };
    editor.focused_field = field;
    editor.error = None;
    true
}

pub(super) fn insert_network_proxy_command_newline(
    proxy_editor: &mut Option<NetworkProxyEditorState>,
) -> bool {
    let Some(editor) = proxy_editor.as_mut() else {
        return false;
    };
    if editor.focused_field != NetworkProxyEditorField::Command {
        return false;
    }
    editor.command.push('\n');
    editor.error = None;
    true
}

pub(super) fn advance_network_proxy_editor_focus(
    proxy_editor: &mut Option<NetworkProxyEditorState>,
) -> bool {
    let Some(editor) = proxy_editor.as_mut() else {
        return false;
    };
    editor.focused_field = editor.focused_field.next(editor.is_proxy_command());
    editor.error = None;
    true
}

pub(super) fn apply_network_proxy_editor_key(
    proxy_editor: &mut Option<NetworkProxyEditorState>,
    key: &str,
    input: Option<&str>,
) -> bool {
    let Some(editor) = proxy_editor.as_mut() else {
        return false;
    };
    match key {
        "backspace" => {
            network_proxy_editor_field_mut(editor).pop();
            editor.error = None;
            true
        }
        _ => {
            let Some(input) = input.filter(|input| !input.is_empty()) else {
                return false;
            };
            let field = editor.focused_field;
            let target = network_proxy_editor_field_mut(editor);
            match field {
                NetworkProxyEditorField::Port => {
                    target.extend(input.chars().filter(|character| character.is_ascii_digit()));
                }
                _ => target.push_str(input),
            }
            editor.error = None;
            true
        }
    }
}

fn network_proxy_editor_field_mut(editor: &mut NetworkProxyEditorState) -> &mut String {
    match editor.focused_field {
        NetworkProxyEditorField::Name => &mut editor.name,
        NetworkProxyEditorField::Host => &mut editor.host,
        NetworkProxyEditorField::Port => &mut editor.port,
        NetworkProxyEditorField::Command => &mut editor.command,
        NetworkProxyEditorField::Username => &mut editor.username,
        NetworkProxyEditorField::Password => &mut editor.password,
    }
}

pub(super) fn cycle_network_proxy_protocol(
    proxy_editor: &mut Option<NetworkProxyEditorState>,
) -> Option<String> {
    let editor = proxy_editor.as_mut()?;
    editor.protocol = match editor.protocol.as_str() {
        "socks5" => "http",
        "http" => "proxycommand",
        _ => "socks5",
    }
    .to_string();
    if editor.is_proxy_command() {
        editor.focused_field = match editor.focused_field {
            NetworkProxyEditorField::Host
            | NetworkProxyEditorField::Port
            | NetworkProxyEditorField::Username
            | NetworkProxyEditorField::Password => NetworkProxyEditorField::Command,
            field => field,
        };
    } else if editor.focused_field == NetworkProxyEditorField::Command {
        editor.focused_field = NetworkProxyEditorField::Host;
    }
    editor.error = None;
    Some(editor.protocol.clone())
}

pub(super) fn cycle_network_proxy_group<'a>(
    proxy_editor: &mut Option<NetworkProxyEditorState>,
    group_ids: impl IntoIterator<Item = &'a str>,
) -> bool {
    let Some(editor) = proxy_editor.as_mut() else {
        return false;
    };
    editor.group_id = next_network_optional_id(editor.group_id.as_deref(), group_ids);
    editor.error = None;
    true
}

pub(super) fn set_network_proxy_editor_error(
    proxy_editor: &mut Option<NetworkProxyEditorState>,
    error: String,
) -> bool {
    let Some(editor) = proxy_editor.as_mut() else {
        return false;
    };
    editor.error = Some(error);
    true
}

fn next_network_optional_id<'a>(
    current_id: Option<&str>,
    ids: impl IntoIterator<Item = &'a str>,
) -> Option<String> {
    let mut cycle = std::iter::once(None)
        .chain(ids.into_iter().map(Some))
        .collect::<Vec<_>>();
    if cycle.is_empty() {
        return None;
    }
    let current_index = cycle.iter().position(|id| *id == current_id).unwrap_or(0);
    cycle
        .remove((current_index + 1) % cycle.len())
        .map(ToOwned::to_owned)
}

fn network_section_key(tab: NetworkTab, section_id: &str) -> String {
    match tab {
        NetworkTab::Tunnels => format!("tunnel:{section_id}"),
        NetworkTab::Proxies => format!("proxy:{section_id}"),
    }
}

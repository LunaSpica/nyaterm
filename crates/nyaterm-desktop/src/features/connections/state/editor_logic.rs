use std::collections::HashSet;

use gpui::WindowHandle;

use crate::features::{ConnectionEditorToggle, ConnectionEditorWindow};
use crate::models::{
    ConnectionEditorAdvancedTab, ConnectionEditorField, ConnectionEditorMenu,
    ConnectionEditorPasswordSource, ConnectionEditorState, ConnectionEditorTelnetTab,
    ConnectionGroupEditorState, ConnectionKindTab,
};

pub(super) fn clear_connection_editor_runtime_state(
    draft: &mut Option<ConnectionEditorState>,
    icon_picker_open: &mut bool,
    menu: &mut Option<ConnectionEditorMenu>,
    window: &mut Option<WindowHandle<ConnectionEditorWindow>>,
    window_open_pending: &mut bool,
) {
    *icon_picker_open = false;
    *menu = None;
    *draft = None;
    *window = None;
    *window_open_pending = false;
}

pub(super) fn finish_connection_editor_save_state(
    draft: &mut Option<ConnectionEditorState>,
    icon_picker_open: &mut bool,
    menu: &mut Option<ConnectionEditorMenu>,
    window: &mut Option<WindowHandle<ConnectionEditorWindow>>,
    window_open_pending: &mut bool,
    selected_ids: &mut HashSet<String>,
    last_selected_id: &mut Option<String>,
    expanded_group_ids: &mut HashSet<String>,
    connection_id: String,
    group_id: Option<String>,
) {
    clear_connection_editor_runtime_state(
        draft,
        icon_picker_open,
        menu,
        window,
        window_open_pending,
    );
    selected_ids.clear();
    selected_ids.insert(connection_id.clone());
    *last_selected_id = Some(connection_id);
    if let Some(group_id) = group_id {
        expanded_group_ids.insert(group_id);
    }
}

pub(super) fn connection_editor_inline_panel_draft(
    draft: &Option<ConnectionEditorState>,
    has_window: bool,
    window_open_pending: bool,
) -> Option<ConnectionEditorState> {
    if has_window || window_open_pending {
        return None;
    }
    draft.clone()
}

pub(super) fn connection_editor_window_open_or_pending(
    has_window: bool,
    window_open_pending: bool,
) -> bool {
    has_window || window_open_pending
}

pub(super) fn clear_connection_editor_group_menu_draft(draft: &mut Option<ConnectionEditorState>) {
    let Some(editor) = draft.as_mut() else {
        return;
    };
    editor.new_group_name.clear();
    if editor.focused_field == ConnectionEditorField::NewGroupName {
        editor.focused_field = ConnectionEditorField::Name;
    }
}

pub(super) fn focus_connection_editor_field(
    draft: &mut Option<ConnectionEditorState>,
    field: ConnectionEditorField,
) -> bool {
    let Some(editor) = draft.as_mut() else {
        return false;
    };
    editor.focused_field = field;
    editor.error = None;
    true
}

pub(super) fn set_connection_editor_icon(
    draft: &mut Option<ConnectionEditorState>,
    icon: Option<&str>,
) -> bool {
    let Some(editor) = draft.as_mut() else {
        return false;
    };
    editor.icon = icon
        .map(str::trim)
        .filter(|icon| !icon.is_empty())
        .map(ToOwned::to_owned);
    // Choosing an icon by hand is an explicit decision, so stop letting the
    // remote system overwrite it. Clearing the icon hands control back.
    editor.icon_auto_detect = editor.icon.is_none();
    editor.error = None;
    true
}

pub(super) fn set_connection_editor_icon_auto_detect(
    draft: &mut Option<ConnectionEditorState>,
    enabled: bool,
) -> bool {
    let Some(editor) = draft.as_mut() else {
        return false;
    };
    if editor.icon_auto_detect == enabled {
        return false;
    }
    editor.icon_auto_detect = enabled;
    true
}

pub(super) fn set_connection_editor_menu_value(
    draft: &mut Option<ConnectionEditorState>,
    menu: ConnectionEditorMenu,
    value: Option<String>,
) -> bool {
    let Some(editor) = draft.as_mut() else {
        return false;
    };
    match menu {
        ConnectionEditorMenu::Authentication => {
            editor.auth_mode = value.unwrap_or_else(|| "password".to_string());
            if editor.auth_mode == "none" {
                clear_connection_editor_password_secret(editor);
                editor.key_id = None;
            }
        }
        ConnectionEditorMenu::Group => {
            editor.group_id = value;
            editor.new_group_name.clear();
            editor.pending_group_name = None;
            editor.pending_group_parent_id = None;
            editor.focused_field = ConnectionEditorField::Name;
        }
        ConnectionEditorMenu::SavedPassword => editor.password_id = value,
        ConnectionEditorMenu::SshKey => editor.key_id = value,
        ConnectionEditorMenu::Otp => {
            editor.otp_id = value;
            if editor.otp_id.is_none() {
                editor.auto_fill_otp = false;
            }
        }
        ConnectionEditorMenu::Proxy => editor.proxy_id = value,
        ConnectionEditorMenu::ProxyJump => editor.proxy_jump_id = value,
        ConnectionEditorMenu::Backspace => {
            editor.backspace_mode = value.unwrap_or_else(|| "del".to_string());
        }
        ConnectionEditorMenu::TelnetEnterMode => {
            editor.telnet_enter_mode = value.unwrap_or_else(|| "cr".to_string());
        }
        ConnectionEditorMenu::Shell => {
            editor.shell_path = value.unwrap_or_else(|| "powershell.exe".to_string());
        }
        ConnectionEditorMenu::SerialPort => editor.serial_port = value.unwrap_or_default(),
        ConnectionEditorMenu::BaudRate => {
            editor.baud_rate = value.unwrap_or_else(|| "115200".to_string());
        }
        ConnectionEditorMenu::DataBits => {
            editor.data_bits = value.unwrap_or_else(|| "8".to_string());
        }
        ConnectionEditorMenu::Parity => {
            editor.parity = value.unwrap_or_else(|| "none".to_string());
        }
        ConnectionEditorMenu::StopBits => {
            editor.stop_bits = value.unwrap_or_else(|| "1".to_string());
        }
    }
    editor.error = None;
    true
}

pub(super) fn set_connection_editor_password_source(
    draft: &mut Option<ConnectionEditorState>,
    source: ConnectionEditorPasswordSource,
) -> bool {
    let Some(editor) = draft.as_mut() else {
        return false;
    };
    editor.password_source = source;
    match source {
        ConnectionEditorPasswordSource::Ask => clear_connection_editor_password_secret(editor),
        ConnectionEditorPasswordSource::Direct => editor.password_id = None,
        ConnectionEditorPasswordSource::Saved => {
            editor.password.clear();
            editor.existing_password = None;
        }
    }
    editor.error = None;
    true
}

pub(super) fn set_connection_editor_advanced_tab(
    draft: &mut Option<ConnectionEditorState>,
    tab: ConnectionEditorAdvancedTab,
) -> bool {
    let Some(editor) = draft.as_mut() else {
        return false;
    };
    match tab {
        ConnectionEditorAdvancedTab::Proxy
        | ConnectionEditorAdvancedTab::JumpHost
        | ConnectionEditorAdvancedTab::TwoFactor => editor.advanced_network_tab = tab,
        ConnectionEditorAdvancedTab::PostLogin
        | ConnectionEditorAdvancedTab::X11
        | ConnectionEditorAdvancedTab::Backspace => editor.advanced_behavior_tab = tab,
    }
    if matches!(
        editor.focused_field,
        ConnectionEditorField::PostLoginCommand | ConnectionEditorField::PostLoginDelay
    ) && tab != ConnectionEditorAdvancedTab::PostLogin
    {
        editor.focused_field = ConnectionEditorField::Name;
    }
    true
}

pub(super) fn set_connection_editor_telnet_tab(
    draft: &mut Option<ConnectionEditorState>,
    tab: ConnectionEditorTelnetTab,
) -> bool {
    let Some(editor) = draft.as_mut() else {
        return false;
    };
    editor.telnet_advanced_tab = tab;
    editor.error = None;
    true
}

pub(super) fn set_connection_editor_kind(
    draft: &mut Option<ConnectionEditorState>,
    kind: ConnectionKindTab,
) -> bool {
    let Some(editor) = draft.as_mut() else {
        return false;
    };
    editor.kind = kind;
    editor.focused_field = ConnectionEditorField::Name;
    editor.port = match kind {
        ConnectionKindTab::Ssh => {
            if editor.port.trim().is_empty() || editor.port == "23" {
                "22".to_string()
            } else {
                editor.port.clone()
            }
        }
        ConnectionKindTab::Telnet => {
            if editor.port.trim().is_empty() || editor.port == "22" {
                "23".to_string()
            } else {
                editor.port.clone()
            }
        }
        _ => editor.port.clone(),
    };
    editor.error = None;
    true
}

pub(super) fn commit_connection_editor_new_group(
    draft: &mut Option<ConnectionEditorState>,
    required_message: String,
) -> bool {
    let Some(editor) = draft.as_mut() else {
        return false;
    };
    let name = editor.new_group_name.trim().to_string();
    if name.is_empty() {
        editor.error = Some(required_message);
        return true;
    }
    editor.pending_group_parent_id = if editor.pending_group_name.is_some() {
        editor.pending_group_parent_id.clone()
    } else {
        editor.group_id.clone()
    };
    editor.pending_group_name = Some(name);
    editor.group_id = None;
    editor.new_group_name.clear();
    editor.focused_field = ConnectionEditorField::Name;
    editor.error = None;
    true
}

pub(super) fn toggle_connection_editor_flag(
    draft: &mut Option<ConnectionEditorState>,
    flag: ConnectionEditorToggle,
) -> bool {
    let Some(editor) = draft.as_mut() else {
        return false;
    };
    match flag {
        ConnectionEditorToggle::AutoFillOtp => {
            editor.auto_fill_otp = editor.otp_id.is_some() && !editor.auto_fill_otp;
        }
        ConnectionEditorToggle::X11 => editor.x11_forwarding = !editor.x11_forwarding,
        ConnectionEditorToggle::RawTcp => {
            editor.raw_tcp_cli = !editor.raw_tcp_cli;
            if editor.raw_tcp_cli {
                editor.telnet_enter_mode = "cr".to_string();
            }
        }
        ConnectionEditorToggle::LocalEcho => editor.local_echo = !editor.local_echo,
        ConnectionEditorToggle::LocalLineEdit => {
            editor.local_line_edit = !editor.local_line_edit;
        }
        ConnectionEditorToggle::ForceCharacterAtATime => {
            editor.force_character_at_a_time = !editor.force_character_at_a_time;
        }
        ConnectionEditorToggle::SendNaws => {
            if !editor.raw_tcp_cli {
                editor.send_naws = !editor.send_naws;
            }
        }
        ConnectionEditorToggle::SendSga => {
            if !editor.raw_tcp_cli {
                editor.send_sga = !editor.send_sga;
            }
        }
        ConnectionEditorToggle::PostLogin => {
            editor.post_login_enabled = !editor.post_login_enabled;
        }
        ConnectionEditorToggle::Advanced => {
            editor.advanced_open = !editor.advanced_open;
            if !editor.advanced_open
                && matches!(
                    editor.focused_field,
                    ConnectionEditorField::PostLoginCommand | ConnectionEditorField::PostLoginDelay
                )
            {
                editor.focused_field = ConnectionEditorField::Name;
            }
        }
    }
    editor.error = None;
    true
}

pub(super) fn insert_connection_editor_description_newline(
    draft: &mut Option<ConnectionEditorState>,
) -> bool {
    let Some(editor) = draft.as_mut() else {
        return false;
    };
    if editor.focused_field != ConnectionEditorField::Description {
        return false;
    }
    editor.description.push('\n');
    editor.error = None;
    true
}

pub(super) fn apply_connection_editor_text_key(
    draft: &mut Option<ConnectionEditorState>,
    key: &str,
    input: Option<&str>,
) -> bool {
    let Some(editor) = draft.as_mut() else {
        return false;
    };
    match key {
        "backspace" => {
            connection_editor_field_mut(editor).pop();
            editor.error = None;
            true
        }
        _ => {
            let Some(input) = input.filter(|input| !input.is_empty()) else {
                return false;
            };
            let field = editor.focused_field;
            let target = connection_editor_field_mut(editor);
            match field {
                ConnectionEditorField::Port
                | ConnectionEditorField::BaudRate
                | ConnectionEditorField::PostLoginDelay => {
                    target.extend(input.chars().filter(|character| character.is_ascii_digit()));
                }
                _ => target.push_str(input),
            }
            editor.error = None;
            true
        }
    }
}

pub(super) fn advance_connection_editor_focus(draft: &mut Option<ConnectionEditorState>) -> bool {
    let Some(editor) = draft.as_mut() else {
        return false;
    };
    let password_field_visible = editor.auth_mode == "password"
        && editor.password_source == ConnectionEditorPasswordSource::Direct;
    let post_login_fields_visible = editor.advanced_open
        && editor.post_login_enabled
        && editor.advanced_behavior_tab == ConnectionEditorAdvancedTab::PostLogin;
    editor.focused_field = editor.focused_field.next(
        editor.kind,
        editor.auth_mode.as_str(),
        password_field_visible,
        post_login_fields_visible,
    );
    editor.error = None;
    true
}

fn clear_connection_editor_password_secret(editor: &mut ConnectionEditorState) {
    editor.password_source = ConnectionEditorPasswordSource::Ask;
    editor.password_id = None;
    editor.password.clear();
    editor.existing_password = None;
}

fn connection_editor_field_mut(editor: &mut ConnectionEditorState) -> &mut String {
    match editor.focused_field {
        ConnectionEditorField::Name => &mut editor.name,
        ConnectionEditorField::NewGroupName => &mut editor.new_group_name,
        ConnectionEditorField::Description => &mut editor.description,
        ConnectionEditorField::Host => &mut editor.host,
        ConnectionEditorField::Port => &mut editor.port,
        ConnectionEditorField::Username => &mut editor.username,
        ConnectionEditorField::Password => &mut editor.password,
        ConnectionEditorField::ShellPath => &mut editor.shell_path,
        ConnectionEditorField::ShellArgs => &mut editor.shell_args,
        ConnectionEditorField::WorkingDir => &mut editor.working_dir,
        ConnectionEditorField::SerialPort => &mut editor.serial_port,
        ConnectionEditorField::BaudRate => &mut editor.baud_rate,
        ConnectionEditorField::PostLoginCommand => &mut editor.post_login_command,
        ConnectionEditorField::PostLoginDelay => &mut editor.post_login_delay_ms,
    }
}

pub(super) fn set_connection_editor_error(
    draft: &mut Option<ConnectionEditorState>,
    error: String,
) -> bool {
    let Some(editor) = draft.as_mut() else {
        return false;
    };
    editor.error = Some(error);
    true
}

pub(super) fn apply_connection_editor_shell_path(
    draft: &mut Option<ConnectionEditorState>,
    shell_path: String,
) -> bool {
    let Some(editor) = draft.as_mut() else {
        return false;
    };
    editor.shell_path = shell_path;
    editor.error = None;
    true
}

pub(super) fn apply_connection_editor_working_dir(
    draft: &mut Option<ConnectionEditorState>,
    working_dir: String,
) -> bool {
    let Some(editor) = draft.as_mut() else {
        return false;
    };
    editor.working_dir = working_dir;
    editor.error = None;
    true
}

pub(super) fn apply_connection_group_editor_name_key(
    draft: &mut Option<ConnectionGroupEditorState>,
    key: &str,
    input: Option<&str>,
) -> bool {
    let Some(editor) = draft.as_mut() else {
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

pub(super) fn set_connection_group_editor_error(
    draft: &mut Option<ConnectionGroupEditorState>,
    error: String,
) -> bool {
    let Some(editor) = draft.as_mut() else {
        return false;
    };
    editor.error = Some(error);
    true
}

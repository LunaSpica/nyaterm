use std::collections::HashSet;

use nyaterm_ui::NyaWindowHandle;

use super::super::connection_runtime::ConnectionEditorToggle;
use crate::models::{
    ConnectionEditorAdvancedTab, ConnectionEditorField, ConnectionEditorPasswordSource,
    ConnectionEditorSelect, ConnectionEditorState, ConnectionEditorTelnetTab,
    ConnectionGroupEditorState, ConnectionKindTab,
};

pub(super) fn clear_connection_editor_runtime_state(
    draft: &mut Option<ConnectionEditorState>,
    icon_picker_open: &mut bool,
    group_select_open: &mut bool,
    window: &mut Option<NyaWindowHandle>,
    window_open_pending: &mut bool,
) {
    *icon_picker_open = false;
    *group_select_open = false;
    *draft = None;
    *window = None;
    *window_open_pending = false;
}

pub(super) fn select_saved_connection_after_editor_save(
    selected_ids: &mut HashSet<String>,
    last_selected_id: &mut Option<String>,
    expanded_group_ids: &mut HashSet<String>,
    connection_id: String,
    group_id: Option<String>,
) {
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

pub(super) fn set_connection_editor_select_value(
    draft: &mut Option<ConnectionEditorState>,
    select: ConnectionEditorSelect,
    value: Option<String>,
) -> bool {
    let Some(editor) = draft.as_mut() else {
        return false;
    };
    match select {
        ConnectionEditorSelect::Authentication => {
            editor.auth_mode = value.unwrap_or_else(|| "password".to_string());
            if editor.auth_mode == "none" {
                clear_connection_editor_password_secret(editor);
                editor.key_id = None;
            }
        }
        ConnectionEditorSelect::Group => {
            editor.group_id = value;
            editor.new_group_name.clear();
            editor.pending_group_name = None;
            editor.pending_group_parent_id = None;
            editor.focused_field = ConnectionEditorField::Name;
        }
        ConnectionEditorSelect::SavedPassword => editor.password_id = value,
        ConnectionEditorSelect::SshKey => editor.key_id = value,
        ConnectionEditorSelect::Otp => {
            editor.otp_id = value;
            if editor.otp_id.is_none() {
                editor.auto_fill_otp = false;
            }
        }
        ConnectionEditorSelect::Proxy => editor.proxy_id = value,
        ConnectionEditorSelect::ProxyJump => editor.proxy_jump_id = value,
        ConnectionEditorSelect::Backspace => {
            editor.backspace_mode = value.unwrap_or_else(|| "del".to_string());
        }
        ConnectionEditorSelect::Encoding => {
            editor.encoding = value.unwrap_or_else(|| "global".to_string());
        }
        ConnectionEditorSelect::SftpCwdFollowMode => {
            editor.sftp_cwd_follow_mode = value.unwrap_or_else(|| "shell_integration".to_string());
        }
        ConnectionEditorSelect::SftpFilenameEncoding => {
            editor.sftp_filename_encoding = value.unwrap_or_else(|| "terminal".to_string());
        }
        ConnectionEditorSelect::SshAlgorithmMode => {
            editor.ssh_algorithm_mode = value.unwrap_or_else(|| "compatible".to_string());
        }
        ConnectionEditorSelect::TelnetEnterMode => {
            editor.telnet_enter_mode = value.unwrap_or_else(|| "cr".to_string());
        }
        ConnectionEditorSelect::Shell => {
            editor.shell_path = value.unwrap_or_else(|| "powershell.exe".to_string());
        }
        ConnectionEditorSelect::SerialPort => editor.serial_port = value.unwrap_or_default(),
        ConnectionEditorSelect::BaudRate => {
            editor.baud_rate = value.unwrap_or_else(|| "115200".to_string());
        }
        ConnectionEditorSelect::DataBits => {
            editor.data_bits = value.unwrap_or_else(|| "8".to_string());
        }
        ConnectionEditorSelect::Parity => {
            editor.parity = value.unwrap_or_else(|| "none".to_string());
        }
        ConnectionEditorSelect::StopBits => {
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
        | ConnectionEditorAdvancedTab::Terminal
        | ConnectionEditorAdvancedTab::Sftp
        | ConnectionEditorAdvancedTab::Algorithms
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
        ConnectionEditorToggle::SftpEnabled => editor.sftp_enabled = !editor.sftp_enabled,
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
        ConnectionEditorToggle::TelnetAutoLoginEnabled => {
            editor.telnet_auto_login_enabled = !editor.telnet_auto_login_enabled;
        }
        ConnectionEditorToggle::TelnetAutoLoginSendWakeEnter => {
            editor.telnet_auto_login_send_wake_enter = !editor.telnet_auto_login_send_wake_enter;
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

/// Which draft strings become editable fields, and which are secrets.
///
/// Driven off the draft rather than a fixed list so a field that does not apply
/// to the current kind is simply never built.
pub(super) fn editor_field_seeds(
    draft: &ConnectionEditorState,
) -> Vec<(ConnectionEditorField, String, bool, &'static str)> {
    vec![
        (ConnectionEditorField::Name, draft.name.clone(), false, ""),
        (
            ConnectionEditorField::Description,
            draft.description.clone(),
            false,
            "e.g. Web server for project X",
        ),
        (
            ConnectionEditorField::NewGroupName,
            draft.new_group_name.clone(),
            false,
            "New group...",
        ),
        (
            ConnectionEditorField::Host,
            draft.host.clone(),
            false,
            "192.168.1.100",
        ),
        (ConnectionEditorField::Port, draft.port.clone(), false, ""),
        (
            ConnectionEditorField::Username,
            draft.username.clone(),
            false,
            "",
        ),
        (
            ConnectionEditorField::Password,
            draft.password.clone(),
            true,
            "",
        ),
        (
            ConnectionEditorField::ShellPath,
            draft.shell_path.clone(),
            false,
            "",
        ),
        (
            ConnectionEditorField::ShellArgs,
            draft.shell_args.clone(),
            false,
            "e.g. --login -i or -NoLogo",
        ),
        (
            ConnectionEditorField::WorkingDir,
            draft.working_dir.clone(),
            false,
            r"e.g. C:\Projects or ~/workspace",
        ),
        (
            ConnectionEditorField::SerialPort,
            draft.serial_port.clone(),
            false,
            "",
        ),
        (
            ConnectionEditorField::BaudRate,
            draft.baud_rate.clone(),
            false,
            "e.g. 74880",
        ),
        (
            ConnectionEditorField::PostLoginCommand,
            draft.post_login_command.clone(),
            false,
            "cd /opt/app",
        ),
        (
            ConnectionEditorField::PostLoginDelay,
            draft.post_login_delay_ms.clone(),
            false,
            "",
        ),
        (
            ConnectionEditorField::SftpShellDetectionTimeout,
            draft.sftp_shell_detection_timeout_ms.clone(),
            false,
            "",
        ),
        (
            ConnectionEditorField::TelnetAutoLoginTimeout,
            draft.telnet_auto_login_timeout_ms.clone(),
            false,
            "",
        ),
        (
            ConnectionEditorField::TelnetAutoLoginUsernamePrompt,
            draft.telnet_auto_login_username_prompt_regex.clone(),
            false,
            "",
        ),
        (
            ConnectionEditorField::TelnetAutoLoginPasswordPrompt,
            draft.telnet_auto_login_password_prompt_regex.clone(),
            false,
            "",
        ),
        (
            ConnectionEditorField::TelnetAutoLoginSuccessPrompt,
            draft.telnet_auto_login_success_prompt_regex.clone(),
            false,
            "",
        ),
        (
            ConnectionEditorField::TelnetAutoLoginFailurePrompt,
            draft.telnet_auto_login_failure_prompt_regex.clone(),
            false,
            "",
        ),
        (
            ConnectionEditorField::TelnetAutoLoginMaxRetries,
            draft.telnet_auto_login_max_retries.clone(),
            false,
            "",
        ),
    ]
}

/// Write an edited field back into the draft, clearing any stale validation.
///
/// Previously the error was cleared when a field took focus; a field now takes
/// focus on its own, so the edit itself is what says the message is out of date.
pub(super) fn set_connection_editor_field_text(
    draft: &mut ConnectionEditorState,
    field: ConnectionEditorField,
    text: String,
) {
    draft.error = None;
    match field {
        ConnectionEditorField::Name => draft.name = text,
        ConnectionEditorField::Description => draft.description = text,
        ConnectionEditorField::NewGroupName => draft.new_group_name = text,
        ConnectionEditorField::Host => draft.host = text,
        ConnectionEditorField::Port => draft.port = text,
        ConnectionEditorField::Username => draft.username = text,
        ConnectionEditorField::Password => draft.password = text,
        ConnectionEditorField::ShellPath => draft.shell_path = text,
        ConnectionEditorField::ShellArgs => draft.shell_args = text,
        ConnectionEditorField::WorkingDir => draft.working_dir = text,
        ConnectionEditorField::SerialPort => draft.serial_port = text,
        ConnectionEditorField::BaudRate => draft.baud_rate = text,
        ConnectionEditorField::PostLoginCommand => draft.post_login_command = text,
        ConnectionEditorField::PostLoginDelay => draft.post_login_delay_ms = text,
        ConnectionEditorField::SftpShellDetectionTimeout => {
            draft.sftp_shell_detection_timeout_ms = text
        }
        ConnectionEditorField::TelnetAutoLoginTimeout => {
            draft.telnet_auto_login_timeout_ms = text;
        }
        ConnectionEditorField::TelnetAutoLoginUsernamePrompt => {
            draft.telnet_auto_login_username_prompt_regex = text;
        }
        ConnectionEditorField::TelnetAutoLoginPasswordPrompt => {
            draft.telnet_auto_login_password_prompt_regex = text;
        }
        ConnectionEditorField::TelnetAutoLoginSuccessPrompt => {
            draft.telnet_auto_login_success_prompt_regex = text;
        }
        ConnectionEditorField::TelnetAutoLoginFailurePrompt => {
            draft.telnet_auto_login_failure_prompt_regex = text;
        }
        ConnectionEditorField::TelnetAutoLoginMaxRetries => {
            draft.telnet_auto_login_max_retries = text;
        }
    }
}

use super::*;

#[path = "panels/about_overlay.rs"]
mod about_overlay;
#[path = "panels/active_session_menu_overlay.rs"]
mod active_session_menu_overlay;
#[path = "panels/connection_import_overlay.rs"]
mod connection_import_overlay;
#[path = "panels/helpers.rs"]
mod helpers;
#[path = "panels/lock_screen_overlay.rs"]
mod lock_screen_overlay;
#[path = "panels/multi_line_paste_overlay.rs"]
mod multi_line_paste_overlay;
#[path = "panels/quick_command_category_menu_overlay.rs"]
mod quick_command_category_menu_overlay;
#[path = "panels/quick_command_category_overlays.rs"]
mod quick_command_category_overlays;
#[path = "panels/quick_command_delete_overlay.rs"]
mod quick_command_delete_overlay;
#[path = "panels/quick_command_details_overlay.rs"]
mod quick_command_details_overlay;
#[path = "panels/quick_command_editor_overlay.rs"]
mod quick_command_editor_overlay;
#[path = "panels/quick_command_import_overlay.rs"]
mod quick_command_import_overlay;
#[path = "panels/quick_command_row_menu_overlay.rs"]
mod quick_command_row_menu_overlay;
#[path = "panels/quick_command_variable_overlay.rs"]
mod quick_command_variable_overlay;
#[path = "panels/quick_commands_panel.rs"]
mod quick_commands_panel;
#[path = "panels/quick_switch_overlay.rs"]
mod quick_switch_overlay;
#[path = "panels/recording_panel.rs"]
mod recording_panel;
#[path = "panels/session_confirm_overlays.rs"]
mod session_confirm_overlays;
#[path = "panels/session_overlays.rs"]
mod session_overlays;
#[path = "panels/sync_groups_overlay.rs"]
mod sync_groups_overlay;
#[path = "panels/tab_actions_overlay.rs"]
mod tab_actions_overlay;
#[path = "panels/temporary_ssh_link_overlay.rs"]
mod temporary_ssh_link_overlay;
#[path = "panels/terminal_actions_overlay.rs"]
mod terminal_actions_overlay;
#[path = "panels/update_overlay.rs"]
mod update_overlay;

pub(in crate::features::panels) use helpers::*;

#[path = "panels/send_command_helpers.rs"]
mod send_command_helpers;
use send_command_helpers::*;

#[path = "panels/send_command_bar.rs"]
mod send_command_bar;

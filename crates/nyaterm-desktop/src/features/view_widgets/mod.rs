mod chrome;
pub(in crate::features) use chrome::{
    bounded_dialog_width, child_window_header, child_window_titlebar, dialog_action_button,
    logo_mark, modal_close_icon_button, modal_dialog_footer_localized,
    modal_dialog_footer_localized_danger, modal_dialog_shell, panel_header_with_actions,
    window_control_button,
};

mod inspector_widgets;
pub(in crate::features) use inspector_widgets::{
    empty_workspace_action, tab_action_button, tab_menu_item, tab_menu_item_enabled,
    tab_menu_separator,
};

mod stats;
pub(in crate::features) use stats::stats_progress_bar;
mod rows;
pub(in crate::features) use rows::{CloudSyncHistoryRowLabels, cloud_sync_history_row};

mod icons;
pub(in crate::features) use icons::{
    activity_icon, color_icon, connection_type_icon, mono_icon, nyaterm_logo_mark, themed_icon,
    transfer_entry_icon,
};

mod markdown;
pub(in crate::features) use markdown::markdown_content_view;

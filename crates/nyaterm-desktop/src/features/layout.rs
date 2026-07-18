use super::*;

#[path = "layout/prompts.rs"]
mod prompts;
#[path = "layout/view_helpers.rs"]
mod view_helpers;
use view_helpers::*;

#[path = "layout/security_editors.rs"]
mod security_editors;
#[path = "layout/security_panel.rs"]
mod security_panel;
#[path = "layout/sidebar.rs"]
mod sidebar;
#[path = "layout/sync_history_panel.rs"]
mod sync_history_panel;
#[path = "layout/workspace.rs"]
mod workspace;

#[path = "layout/title_menu_helpers.rs"]
mod title_menu_helpers;
use title_menu_helpers::*;

#[path = "layout/activity_bar.rs"]
mod activity_bar;
#[path = "layout/title_bar.rs"]
mod title_bar;

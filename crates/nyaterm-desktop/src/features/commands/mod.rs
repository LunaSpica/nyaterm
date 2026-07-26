//! Command history/suggestion runtime and quick command runtime.

use super::*;

mod command_runtime;
mod quick_command_runtime;

pub(in crate::features) use quick_command_runtime::{
    QUICK_COMMAND_COLOR_OPTIONS, QUICK_COMMAND_ICON_OPTIONS, quick_command_category_label,
    quick_command_sort_mode_from_setting, quick_command_view_mode_from_setting,
    sorted_quick_commands,
};

use super::*;
use variables::parse_quick_command_variables;

mod import;
mod variables;

pub(in crate::features) const QUICK_COMMAND_COLOR_OPTIONS: [Option<&str>; 6] = [
    None,
    Some("red"),
    Some("green"),
    Some("blue"),
    Some("yellow"),
    Some("purple"),
];

mod helpers;
pub(in crate::features) use helpers::*;

mod catalog;
mod dialogs;
mod editor;
mod run;

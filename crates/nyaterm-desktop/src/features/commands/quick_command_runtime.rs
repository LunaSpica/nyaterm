use super::*;
use variables::parse_quick_command_variables;

#[path = "quick_command_runtime/import.rs"]
mod import;
#[path = "quick_command_runtime/variables.rs"]
mod variables;

pub(in crate::features) const QUICK_COMMAND_COLOR_OPTIONS: [Option<&str>; 6] = [
    None,
    Some("red"),
    Some("green"),
    Some("blue"),
    Some("yellow"),
    Some("purple"),
];
pub(in crate::features) const QUICK_COMMAND_ICON_OPTIONS: [Option<&str>; 31] = [
    None,
    Some("terminal"),
    Some("code"),
    Some("server"),
    Some("folder"),
    Some("sparkles"),
    Some("bolt"),
    Some("docker"),
    Some("k8s"),
    Some("linux"),
    Some("ubuntu"),
    Some("debian"),
    Some("centos"),
    Some("fedora"),
    Some("apple"),
    Some("github"),
    Some("gitlab"),
    Some("nginx"),
    Some("redis"),
    Some("postgres"),
    Some("mysql"),
    Some("mongodb"),
    Some("python"),
    Some("js"),
    Some("ts"),
    Some("rust"),
    Some("go"),
    Some("node"),
    Some("php"),
    Some("aws"),
    Some("gcp"),
];

#[path = "quick_command_runtime/helpers.rs"]
mod helpers;
pub(in crate::features) use helpers::*;

#[path = "quick_command_runtime/catalog.rs"]
mod catalog;
#[path = "quick_command_runtime/dialogs.rs"]
mod dialogs;
#[path = "quick_command_runtime/editor.rs"]
mod editor;
#[path = "quick_command_runtime/run.rs"]
mod run;

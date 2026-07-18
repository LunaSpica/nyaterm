use super::*;

#[path = "inspector/helpers.rs"]
mod helpers;
pub(in crate::features) use helpers::*;

#[path = "inspector/ai_ask.rs"]
mod ai_ask;
#[path = "inspector/ai_widgets.rs"]
mod ai_widgets;
#[path = "inspector/commands.rs"]
mod commands;
#[path = "inspector/right_domain.rs"]
mod right_domain;
#[path = "inspector/right_shell.rs"]
mod right_shell;

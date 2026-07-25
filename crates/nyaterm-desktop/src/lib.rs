//! GPUI presentation crate for NyaTerm.

mod action_links;
mod i18n;
mod send_command;
mod shortcuts;
mod temporary_ssh_link;

pub mod app_shell;
pub mod entities;
pub mod features;
pub mod http;
pub mod models;
pub mod terminal;
pub mod theme;
pub mod widgets;

pub use app_shell::AppShell;

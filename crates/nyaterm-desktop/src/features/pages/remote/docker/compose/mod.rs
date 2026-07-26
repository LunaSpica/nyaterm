use super::*;
use gpui::SharedString;

mod menus;
mod project;
mod service;
mod status;

pub(in crate::features::pages::remote) use menus::*;
pub(in crate::features::pages::remote) use project::*;
pub(in crate::features::pages::remote) use service::*;
pub(in crate::features::pages::remote) use status::*;

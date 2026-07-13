use super::*;
use gpui::SharedString;

#[path = "docker_compose/menus.rs"]
mod menus;
#[path = "docker_compose/project.rs"]
mod project;
#[path = "docker_compose/service.rs"]
mod service;
#[path = "docker_compose/status.rs"]
mod status;

pub(in crate::features::pages::remote) use menus::*;
pub(in crate::features::pages::remote) use project::*;
pub(in crate::features::pages::remote) use service::*;
pub(in crate::features::pages::remote) use status::*;

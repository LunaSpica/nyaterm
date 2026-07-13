use super::*;
use crate::theme::ThemePalette;

#[path = "process/data.rs"]
mod data;
#[path = "process/details.rs"]
mod details;
#[path = "process/resources.rs"]
mod resources;
#[path = "process/table.rs"]
mod table;

pub(super) use data::*;
pub(super) use details::*;
pub(super) use resources::*;
pub(super) use table::*;

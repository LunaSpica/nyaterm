use super::*;
use gpui::{App, ClickEvent, SharedString, Window};

#[path = "terminal/helpers.rs"]
mod helpers;
use helpers::*;

#[path = "terminal/general.rs"]
mod general;
#[path = "terminal/keywords.rs"]
mod keywords;
#[path = "terminal/search.rs"]
mod search;

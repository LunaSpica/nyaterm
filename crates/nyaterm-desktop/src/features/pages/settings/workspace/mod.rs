use super::*;
use crate::shortcuts::{
    SHORTCUT_CATEGORIES, SHORTCUT_REGISTRY, ShortcutCategory, ShortcutDefinition,
    ShortcutNativeStatus, format_hotkey_for_display, shortcut_keys_for,
};
use crate::theme::{APPEARANCE_THEME_IDS, appearance_theme_label};

mod appearance;
mod general;
mod interaction;
mod keybindings;

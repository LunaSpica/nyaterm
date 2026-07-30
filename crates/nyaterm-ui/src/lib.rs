//! Shared GPUI theme tokens and reusable presentation widgets for NyaTerm.

mod button;
mod dialog;
mod input;
mod menu;
mod root;
mod selection;
mod tabs;
mod theme;
mod theme_bridge;
mod tooltip;
mod widgets;

pub use button::{NyaButton, NyaButtonVariant, NyaIconButton};
pub use dialog::{NyaConfirmDialog, NyaDialog, NyaDialogFooter, NyaDialogWindowExt};
pub use input::{NyaInput, NyaInputEvent, NyaInputState, NyaTextArea};
pub use menu::{NyaContextMenu, NyaDropdownMenu, NyaMenuAnchor, NyaMenuItem};
pub use root::{NyaRoot, NyaWindowHandle, nya_root};
pub use selection::{
    NyaCheckbox, NyaRadioGroup, NyaSelect, NyaSelectEvent, NyaSelectOption, NyaSelectState,
    NyaSwitch,
};
pub use tabs::{NyaTabItem, NyaTabs, NyaTabsVariant};
pub use theme::{APPEARANCE_THEME_IDS, ThemePalette, appearance_theme_label, theme_palette};
pub use theme_bridge::apply_component_theme;
pub use tooltip::NyaTooltip;
pub use widgets::{
    capability_line, empty_panel, mode_button, section_header, session_info_row, small_button,
    status_pill, svg_icon_button,
};

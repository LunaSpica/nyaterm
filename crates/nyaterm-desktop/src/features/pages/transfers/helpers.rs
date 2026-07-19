use super::*;

#[path = "helpers/browser.rs"]
mod browser;
#[path = "helpers/editor.rs"]
mod editor;
#[path = "helpers/job_row.rs"]
mod job_row;
#[path = "helpers/paths.rs"]
mod paths;
#[path = "helpers/properties.rs"]
mod properties;
#[path = "helpers/queue.rs"]
mod queue;

pub(super) use browser::*;
pub(super) use editor::*;
pub(super) use job_row::*;
pub(super) use paths::*;
pub(super) use properties::*;
pub(super) use queue::*;

pub(super) fn transfer_dialog_button(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    danger: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let background = if danger {
        palette.danger
    } else {
        palette.primary
    };
    let hover_background = if danger {
        palette.danger
    } else {
        palette.primary_hover
    };
    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .rounded_sm()
        .bg(rgb(background))
        .text_color(rgb(palette.bg))
        .text_xs()
        .cursor_pointer()
        .hover(move |this| this.bg(rgb(hover_background)))
        .child(label)
        .on_click(on_click)
}

use super::*;

pub(super) fn title_menu_item(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: impl Into<String>,
    shortcut: Option<String>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let label = label.into();
    let mut row = div()
        .id(SharedString::from(id.into()))
        .h(px(30.))
        .px_3()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .text_size(px(12.))
        .text_color(rgb(palette.text))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface_elevated)))
        .on_click(on_click)
        .child(div().min_w_0().flex_1().child(label));
    if let Some(shortcut) = shortcut {
        row = row.child(
            div()
                .text_size(px(10.))
                .text_color(rgb(palette.text_dimmed))
                .child(shortcut),
        );
    }
    row
}

pub(super) fn title_menu_separator(palette: crate::theme::ThemePalette) -> impl IntoElement {
    div().h(px(1.)).mx_2().my_1().bg(rgb(palette.border))
}

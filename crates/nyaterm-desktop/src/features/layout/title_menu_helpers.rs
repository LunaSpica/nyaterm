use super::*;

pub(super) fn title_menu_item(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    icon: Option<&'static str>,
    checked: bool,
    label: impl Into<String>,
    shortcut: Option<String>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let label = label.into();
    let icon = if checked { None } else { icon };
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
        .child(
            div()
                .w(px(16.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .when(checked, |this| {
                    this.child(svg().size(px(13.)).path("icons/check.svg"))
                })
                .when_some(icon, |this, icon_path| {
                    this.child(
                        svg()
                            .size(px(14.))
                            .path(icon_path)
                            .text_color(rgb(palette.text_dimmed)),
                    )
                }),
        )
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

pub(super) fn title_menu_submenu_trigger(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    icon: Option<&'static str>,
    label: impl Into<String>,
    open: bool,
    on_hover: impl Fn(&bool, &mut Window, &mut App) + 'static,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
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
        .when(open, |this| this.bg(rgb(palette.surface_elevated)))
        .hover(|this| this.bg(rgb(palette.surface_elevated)))
        .on_hover(on_hover)
        .on_click(on_click)
        .child(
            div()
                .w(px(16.))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .when_some(icon, |this, icon_path| {
                    this.child(
                        svg()
                            .size(px(14.))
                            .path(icon_path)
                            .text_color(rgb(palette.text_dimmed)),
                    )
                }),
        )
        .child(div().min_w_0().flex_1().child(label.into()))
        .child(
            svg()
                .size(px(12.))
                .path("icons/fe/forward.svg")
                .text_color(rgb(palette.text_dimmed)),
        )
}

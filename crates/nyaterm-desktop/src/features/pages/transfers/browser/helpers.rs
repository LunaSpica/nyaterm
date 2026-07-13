use super::*;

pub(super) fn compact_transfer_footer_button(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    icon_path: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    // Tauri footer icons: h-6 w-6 (24px)
    div()
        .id(SharedString::from(id.into()))
        .size(px(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(rgb(palette.text_muted))
        .cursor_pointer()
        .hover(|this| {
            this.bg(rgb(palette.surface_elevated))
                .text_color(rgb(palette.text))
        })
        .child(svg().size(px(14.)).flex_none().path(icon_path))
        .on_click(on_click)
}

pub(super) fn compact_transfer_footer_button_active(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    icon_path: &'static str,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let color = if active {
        rgb(palette.accent)
    } else {
        rgb(palette.text_muted)
    };
    div()
        .id(SharedString::from(id.into()))
        .size(px(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .bg(if active {
            rgb(palette.hover)
        } else {
            rgb(palette.surface)
        })
        .text_color(color)
        .cursor_pointer()
        .hover(|this| {
            this.bg(rgb(palette.surface_elevated))
                .text_color(if active {
                    rgb(0x79b8ff)
                } else {
                    rgb(palette.text)
                })
        })
        .child(svg().size(px(14.)).flex_none().path(icon_path))
        .on_click(on_click)
}

pub(super) fn compact_transfer_upload_menu_button(
    palette: crate::theme::ThemePalette,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    // Tauri: single Upload icon opens DropdownMenu (Upload Files / Upload Folder).
    div()
        .id(SharedString::from("transfer-browser-upload"))
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(rgb(palette.text_muted))
        .cursor_pointer()
        .hover(|this| {
            this.bg(rgb(palette.surface_elevated))
                .text_color(rgb(palette.text))
        })
        .child(svg().size(px(16.)).flex_none().path("icons/fe/upload.svg"))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, event: &MouseDownEvent, _, cx| {
                this.open_transfer_browser_upload_menu(event, cx);
            }),
        )
}

pub(super) fn compact_transfer_toolbar_button(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    icon_path: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    // Tauri FileExplorerToolbar: h-7 ghost icon buttons, muted until hover.
    div()
        .id(SharedString::from(id.into()))
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(rgb(palette.text_muted))
        .cursor_pointer()
        .hover(|this| {
            this.bg(rgb(palette.surface_elevated))
                .text_color(rgb(palette.text))
        })
        .child(svg().size(px(16.)).flex_none().path(icon_path))
        .on_click(on_click)
}

pub(super) fn compact_transfer_toolbar_button_active(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    icon_path: &'static str,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let color = if active {
        rgb(palette.accent)
    } else {
        rgb(palette.text_muted)
    };
    div()
        .id(SharedString::from(id.into()))
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .bg(if active {
            rgb(palette.hover)
        } else {
            rgb(palette.surface)
        })
        .text_color(color)
        .cursor_pointer()
        .hover(|this| {
            this.bg(rgb(palette.surface_elevated))
                .text_color(if active {
                    rgb(0x79b8ff)
                } else {
                    rgb(palette.text)
                })
        })
        .child(svg().size(px(16.)).flex_none().path(icon_path))
        .on_click(on_click)
}

pub(super) fn transfer_dynamic_toolbar_button(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: impl Into<String>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .max_w(px(116.))
        .px_3()
        .flex()
        .items_center()
        .overflow_hidden()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.surface))
        .text_color(rgb(palette.text))
        .text_xs()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.hover)))
        .child(label.into())
        .on_click(on_click)
}

pub(super) fn transfer_toolbar_divider(palette: crate::theme::ThemePalette) -> impl IntoElement {
    div()
        .h(px(16.))
        .w(px(1.))
        .mx_1()
        .rounded_sm()
        .bg(rgb(palette.border))
}

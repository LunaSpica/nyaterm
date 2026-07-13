use super::*;

/// Monochrome activity-bar SVG icon with glyph fallback.
pub(in crate::features) fn activity_icon(
    path: Option<&'static str>,
    glyph: &'static str,
    color: gpui::Hsla,
    size_px: f32,
) -> gpui::AnyElement {
    let size = px(size_px);
    if let Some(path) = path {
        svg()
            .size(size)
            .flex_none()
            .path(path)
            .text_color(color)
            .into_any_element()
    } else {
        div()
            .size(size)
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(size_px * 0.72))
            .font_weight(FontWeight(700.))
            .text_color(color)
            .child(glyph)
            .into_any_element()
    }
}

/// Compact ghost icon button with bundled SVG (Tauri ToolbarIconButton h-7).
pub(in crate::features) fn toolbar_svg_button(
    palette: ThemePalette,
    id: impl Into<SharedString>,
    icon_path: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id.into())
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
        .child(
            svg()
                .size(px(16.))
                .flex_none()
                .path(icon_path)
                .text_color(rgb(palette.text_muted)),
        )
        .on_click(on_click)
}

/// Faded NyaTerm logo used by empty workspace (Tauri EmptyWorkspaceState).
pub(in crate::features) fn nyaterm_logo_mark(
    palette: ThemePalette,
    size_px: f32,
    opacity: f32,
) -> impl IntoElement {
    let size = px(size_px);
    div()
        .size(size)
        .flex_none()
        .opacity(opacity)
        .flex()
        .items_center()
        .justify_center()
        .child(
            svg()
                .size(size)
                .path("icons/logo.svg")
                .text_color(rgb(palette.text_muted)),
        )
}

/// Colored connection/OS icon for saved connection rows (Tauri resolveConnectionIcon).
pub(in crate::features) fn connection_type_icon(
    palette: ThemePalette,
    def: ConnectionIconDef,
    selected: bool,
    size_px: f32,
) -> gpui::AnyElement {
    let size = px(size_px);
    let color = if selected {
        rgb(palette.accent)
    } else {
        rgb(def.color)
    };
    svg()
        .size(size)
        .flex_none()
        .path(def.path)
        .text_color(color)
        .into_any_element()
}

/// Tauri file explorer entry icon (folder / symlink / file).
pub(in crate::features) fn transfer_entry_icon(
    is_directory: bool,
    is_symlink: bool,
    selected: bool,
) -> gpui::AnyElement {
    let (path, color) = if is_symlink {
        ("icons/conn/symlink.svg", 0x67e8f9u32)
    } else if is_directory {
        ("icons/conn/folder.svg", 0xfbbf24u32)
    } else {
        ("icons/conn/file.svg", 0x94a3b8u32)
    };
    let color = if selected { 0x58a6ffu32 } else { color };
    svg()
        .size(px(14.))
        .flex_none()
        .path(path)
        .text_color(rgb(color))
        .into_any_element()
}

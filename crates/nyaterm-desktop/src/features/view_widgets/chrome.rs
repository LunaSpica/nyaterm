use gpui::{
    AnyElement, App, ClickEvent, FontWeight, IntoElement, SharedString, TitlebarOptions, Window,
    WindowControlArea, div, prelude::*, px, rgb, rgba, svg,
};

use crate::theme::ThemePalette;
use nyaterm_ui::{NyaButton, NyaButtonVariant, NyaIconButton};

pub(in crate::features) fn logo_mark(palette: ThemePalette) -> impl IntoElement {
    div()
        .size(px(22.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(
            svg()
                .size(px(18.))
                .path("icons/logo.svg")
                .text_color(rgb(palette.link)),
        )
}

pub(in crate::features) fn window_control_button(
    palette: ThemePalette,
    id: &'static str,
    icon_path: &'static str,
    area: WindowControlArea,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let hovered_color = if matches!(area, WindowControlArea::Close) {
        0xffffff
    } else {
        palette.text
    };
    div()
        .id(SharedString::from(id))
        .group(SharedString::from(id))
        .w(px(46.))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .text_color(rgb(palette.text_muted))
        .window_control_area(area)
        .cursor_pointer()
        .hover(|this| {
            if matches!(area, WindowControlArea::Close) {
                this.bg(rgb(0xe81123)).text_color(rgb(0xffffff))
            } else {
                this.bg(rgb(palette.hover)).text_color(rgb(palette.text))
            }
        })
        .child(
            svg()
                .size(px(16.))
                .flex_none()
                .path(icon_path)
                .text_color(rgb(palette.text_muted))
                .group_hover(SharedString::from(id), move |this| {
                    this.text_color(rgb(hovered_color))
                }),
        )
        .on_click(on_click)
}

pub(in crate::features) fn child_window_header(
    palette: ThemePalette,
    title: impl Into<SharedString>,
    icon_path: Option<&'static str>,
    window_controls: bool,
    is_maximized: bool,
    on_close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let title = title.into();
    div()
        .h(px(40.))
        .flex_none()
        .flex()
        .items_center()
        .border_b_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.surface))
        .child(
            div()
                .h_full()
                .min_w_0()
                .flex_1()
                .flex()
                .items_center()
                .gap_2()
                .px_3()
                .when(cfg!(target_os = "macos"), |this| this.pl(px(70.)))
                .window_control_area(WindowControlArea::Drag)
                .when_some(icon_path, |this, icon_path| {
                    this.child(
                        svg()
                            .size(px(16.))
                            .flex_none()
                            .path(icon_path)
                            .text_color(rgb(palette.primary)),
                    )
                })
                .child(
                    div()
                        .min_w_0()
                        .overflow_hidden()
                        .text_sm()
                        .font_weight(FontWeight(500.))
                        .text_color(rgb(palette.text))
                        .child(title),
                ),
        )
        .child(
            div()
                .h_full()
                .flex_none()
                .flex()
                .items_center()
                .when(!cfg!(target_os = "macos") && window_controls, |this| {
                    this.child(window_control_button(
                        palette,
                        "child-window-min",
                        "icons/window/minimize.svg",
                        WindowControlArea::Min,
                        |_, window, _| window.minimize_window(),
                    ))
                    .child(window_control_button(
                        palette,
                        "child-window-max",
                        if is_maximized {
                            "icons/window/restore.svg"
                        } else {
                            "icons/window/maximize.svg"
                        },
                        WindowControlArea::Max,
                        |_, window, _| window.zoom_window(),
                    ))
                })
                .when(!cfg!(target_os = "macos"), |this| {
                    this.child(window_control_button(
                        palette,
                        "child-window-close",
                        "icons/window/close.svg",
                        WindowControlArea::Close,
                        on_close,
                    ))
                }),
        )
}

pub(in crate::features) fn child_window_titlebar(
    title: impl Into<SharedString>,
) -> Option<TitlebarOptions> {
    cfg!(target_os = "macos").then(|| TitlebarOptions {
        title: Some(title.into()),
        appears_transparent: true,
        ..Default::default()
    })
}

pub(in crate::features) fn panel_header_with_actions(
    title: impl Into<SharedString>,
    meta: impl Into<SharedString>,
    palette: ThemePalette,
    background: gpui::Rgba,
    actions: Option<AnyElement>,
) -> impl IntoElement {
    // Tauri PanelHeader: min-h-9, uppercase tracked title + dimmed meta/actions.
    let title = title.into();
    let meta = meta.into();
    let show_meta = !meta.is_empty();
    div()
        .h(px(36.))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .px_3()
        .border_b_1()
        .border_color(rgb(palette.border))
        .bg(background)
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .items_baseline()
                .gap_2()
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(palette.text_muted))
                        .child(title.to_uppercase()),
                )
                .when(show_meta, |this| {
                    this.child(
                        div()
                            .min_w_0()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .opacity(0.85)
                            .overflow_hidden()
                            .child(meta),
                    )
                }),
        )
        .when_some(actions, |this, actions| {
            this.child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(actions),
            )
        })
}

/// Dimmed full-area modal shell (Tauri Dialog backdrop + centered card).
/// A dialog centred over whatever hosts it, dimming what is behind it.
///
/// The overlay fills its nearest positioned ancestor, so a dialog belongs to a
/// host that spans the window — the app root — rather than to the panel that
/// owns the state. A panel is often a couple of hundred pixels wide, and a form
/// laid out in there wraps every caption onto its own lines.
pub(in crate::features) fn modal_dialog_shell(
    palette: ThemePalette,
    background: gpui::Rgba,
    id: impl Into<String>,
    width: f32,
    content: impl IntoElement,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .absolute()
        .top_0()
        .bottom_0()
        .left_0()
        .right_0()
        .occlude()
        .bg(rgba(0x00000080))
        .flex()
        .items_center()
        .justify_center()
        .p_3()
        .child(
            div()
                .w(px(width))
                .max_w_full()
                .max_h_full()
                .rounded_md()
                .border_1()
                .border_color(rgb(palette.border))
                .bg(background)
                .shadow_lg()
                .child(content),
        )
}

/// Bounds a dialog to the available viewport while preserving the historical
/// fallback to the preferred width when GPUI has not published a finite size.
pub(in crate::features) fn bounded_dialog_width(
    viewport_width: f32,
    horizontal_inset: f32,
    minimum_width: f32,
    preferred_width: f32,
) -> f32 {
    let available_width = viewport_width - horizontal_inset;
    if available_width.is_nan() || available_width > preferred_width {
        preferred_width
    } else if available_width < minimum_width {
        minimum_width
    } else {
        available_width
    }
}

pub(in crate::features) fn dialog_action_button(
    _palette: ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    danger: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let variant = if danger {
        NyaButtonVariant::Danger
    } else {
        NyaButtonVariant::Primary
    };

    NyaButton::new(id.into(), label)
        .variant(variant)
        .small()
        .compact()
        .on_click(on_click)
}

pub(in crate::features) fn modal_close_icon_button(
    _palette: ThemePalette,
    id: impl Into<String>,
    tooltip: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    NyaIconButton::new(id.into(), "icons/window/close.svg")
        .icon_size(px(14.))
        .tooltip(tooltip)
        .on_click(on_click)
}

#[cfg(test)]
mod tests {
    use super::bounded_dialog_width;

    #[test]
    fn dialog_width_bounds_available_space_and_keeps_non_finite_fallback() {
        assert_eq!(bounded_dialog_width(1280., 32., 280., 448.), 448.);
        assert_eq!(bounded_dialog_width(400., 32., 280., 448.), 368.);
        assert_eq!(bounded_dialog_width(200., 32., 280., 448.), 280.);
        assert_eq!(bounded_dialog_width(f32::NAN, 32., 280., 448.), 448.);
    }
}

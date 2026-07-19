use super::*;
use gpui::AnyElement;

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

pub(in crate::features) fn menu_bar_button(
    palette: ThemePalette,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("menu-{label}")))
        .h(px(26.))
        .px_2()
        .flex()
        .items_center()
        .rounded_sm()
        .text_size(px(12.))
        .text_color(rgb(palette.text_muted))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
        .child(label)
        .on_click(on_click)
}

pub(in crate::features) fn window_control_button(
    palette: ThemePalette,
    id: &'static str,
    icon_path: &'static str,
    area: WindowControlArea,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id))
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
        .child(svg().size(px(16.)).flex_none().path(icon_path))
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

pub(in crate::features) fn dialog_action_button(
    palette: ThemePalette,
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

/// Tauri ActionFooter-like Cancel/Save row.
pub(in crate::features) fn modal_dialog_footer(
    palette: ThemePalette,
    cancel_id: impl Into<String>,
    save_id: impl Into<String>,
    save_label: &'static str,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_save: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .mt_1()
        .pt_3()
        .border_t_1()
        .border_color(rgb(palette.border))
        .flex()
        .items_center()
        .justify_end()
        .gap_2()
        .child(small_button(palette, cancel_id, "Cancel", on_cancel))
        .child(small_button(palette, save_id, save_label, on_save))
}

pub(in crate::features) fn modal_dialog_footer_localized(
    palette: ThemePalette,
    cancel_id: impl Into<String>,
    save_id: impl Into<String>,
    cancel_label: &'static str,
    save_label: &'static str,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_save: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .mt_1()
        .pt_3()
        .border_t_1()
        .border_color(rgb(palette.border))
        .flex()
        .items_center()
        .justify_end()
        .gap_2()
        .child(small_button(palette, cancel_id, cancel_label, on_cancel))
        .child(small_button(palette, save_id, save_label, on_save))
}

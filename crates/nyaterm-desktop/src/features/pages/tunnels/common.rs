use super::*;

use crate::features::ChromeTooltip;

pub(super) fn network_tab_button(
    id: impl Into<String>,
    label: String,
    active: bool,
    palette: crate::theme::ThemePalette,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    // Tauri TabsTrigger inside TabsList grid-cols-2 h-8.
    div()
        .id(gpui::SharedString::from(id.into()))
        .h_full()
        .flex_1()
        .px_3()
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .bg(if active {
            rgb(palette.surface_elevated)
        } else {
            rgb(palette.input)
        })
        .text_color(if active {
            rgb(palette.text)
        } else {
            rgb(palette.text_muted)
        })
        .text_size(px(12.))
        .font_weight(if active {
            FontWeight(600.)
        } else {
            FontWeight(500.)
        })
        .cursor_pointer()
        .hover(move |this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
        .child(label)
        .on_click(on_click)
}

pub(super) fn network_item_overflow_menu(
    palette: crate::theme::ThemePalette,
    menu_background: gpui::Rgba,
    id: impl Into<String>,
    open: bool,
    more_label: &'static str,
    edit_label: &'static str,
    move_label: &'static str,
    delete_label: &'static str,
    can_move: bool,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_edit: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_move: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_delete: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let id = id.into();
    div()
        .relative()
        .child(
            div()
                .id(gpui::SharedString::from(format!("{id}-trigger")))
                .size(px(26.))
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
                .when(open, |this| this.bg(rgb(palette.surface_elevated)))
                .child(
                    svg()
                        .size(px(14.))
                        .flex_none()
                        .path("icons/session/more.svg"),
                )
                .tooltip(move |_, cx| cx.new(|_| ChromeTooltip::new(more_label)).into())
                .on_click(on_toggle),
        )
        .when(open, |this| {
            this.child(
                div()
                    .id(gpui::SharedString::from(format!("{id}-menu")))
                    .absolute()
                    .top(px(28.))
                    .right_0()
                    .w(px(164.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(menu_background)
                    .shadow_lg()
                    .py_1()
                    .flex()
                    .flex_col()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(network_item_menu_entry(
                        palette,
                        format!("{id}-edit"),
                        "icons/net/edit.svg",
                        edit_label,
                        false,
                        on_edit,
                    ))
                    .when(can_move, |this| {
                        this.child(network_item_menu_entry(
                            palette,
                            format!("{id}-move"),
                            "icons/net/move.svg",
                            move_label,
                            false,
                            on_move,
                        ))
                    })
                    .child(div().h(px(1.)).mx_2().my_1().bg(rgb(palette.border)))
                    .child(network_item_menu_entry(
                        palette,
                        format!("{id}-delete"),
                        "icons/net/delete.svg",
                        delete_label,
                        true,
                        on_delete,
                    )),
            )
        })
}

fn network_item_menu_entry(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    icon_path: &'static str,
    label: &'static str,
    danger: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(gpui::SharedString::from(id.into()))
        .h(px(30.))
        .px_3()
        .flex()
        .items_center()
        .gap_2()
        .text_size(px(12.))
        .text_color(rgb(if danger { palette.danger } else { palette.text }))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.hover)))
        .child(svg().size(px(14.)).flex_none().path(icon_path))
        .child(label)
        .on_click(on_click)
}

pub(super) fn network_delete_confirm_panel(
    app: &NyaTermApp,
    confirm: NetworkDeleteConfirmState,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let palette = app.theme_palette();
    let type_label = match confirm.tab {
        NetworkTab::Tunnels => app.tr("network.tunnelConfig"),
        NetworkTab::Proxies => app.tr("network.proxyConfig"),
    };
    let card = div()
        .p_6()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .text_size(px(15.))
                .font_weight(FontWeight(700.))
                .text_color(rgb(palette.text))
                .child(format!("{} {type_label}", app.tr("common.delete"))),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(palette.text))
                .child(
                    app.tr("common.deletingConfirm")
                        .replace("{{name}}", &confirm.label),
                ),
        )
        .child(modal_dialog_footer_localized_danger(
            palette,
            "network-delete-cancel",
            "network-delete-confirm",
            app.tr("common.cancel"),
            app.tr("common.delete"),
            cx.listener(|this, _, _, cx| {
                this.cancel_network_delete(cx);
            }),
            cx.listener(|this, _, _, cx| {
                this.confirm_network_delete(cx);
            }),
        ));
    network_modal_shell(
        palette,
        app.shell_surface_color(palette.bg),
        "network-delete-confirm-modal",
        384.,
        card,
    )
}

pub(super) fn network_group_editor_panel(
    app: &NyaTermApp,
    editor: NetworkGroupEditorState,
    focus: &gpui::FocusHandle,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let palette = app.theme_palette();
    let card = div()
        .p_6()
        .flex()
        .flex_col()
        .gap_4()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .text_size(px(15.))
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(palette.text))
                        .child(if editor.id.is_some() {
                            app.tr("network.renameGroup")
                        } else {
                            app.tr("network.newGroup")
                        }),
                ),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(palette.text_muted))
                .child(app.tr("network.groupDialogDescription")),
        )
        .child(
            transfer_input(
                "network-group-editor-name",
                app.tr("network.groupName"),
                editor.name.clone(),
                true,
                palette,
            )
            .track_focus(focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.network_group_editor_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                this.handle_network_group_editor_key_down(event, cx);
            })),
        )
        .when_some(editor.error.clone(), |this, error| {
            this.child(
                div()
                    .text_size(px(12.))
                    .text_color(rgb(0xfda4af))
                    .child(error),
            )
        })
        .child(network_dialog_footer(
            app,
            palette,
            "network-group-editor-cancel",
            "network-group-editor-save",
            app.tr("common.save"),
            cx.listener(|this, _, _, cx| {
                this.close_network_group_editor(cx);
            }),
            cx.listener(|this, _, _, cx| {
                this.save_network_group_editor(cx);
            }),
        ));
    network_modal_shell(
        palette,
        app.shell_surface_color(palette.bg),
        "network-group-editor-modal",
        420.,
        card,
    )
}

pub(super) fn network_group_delete_confirm_panel(
    app: &NyaTermApp,
    confirm: NetworkGroupDeleteConfirmState,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let palette = app.theme_palette();
    let description = app
        .tr("network.deleteGroupConfirm")
        .replace("{{name}}", &confirm.label)
        .replace("{{count}}", &confirm.item_count.to_string());
    let card = div()
        .p_6()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .text_size(px(15.))
                .font_weight(FontWeight(700.))
                .text_color(rgb(palette.text))
                .child(app.tr("network.deleteGroup")),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(palette.text))
                .child(description),
        )
        .child(modal_dialog_footer_localized_danger(
            palette,
            "network-group-delete-cancel",
            "network-group-delete-confirm",
            app.tr("common.cancel"),
            app.tr("common.delete"),
            cx.listener(|this, _, _, cx| {
                this.cancel_network_group_delete(cx);
            }),
            cx.listener(|this, _, _, cx| {
                this.confirm_network_group_delete(cx);
            }),
        ));
    network_modal_shell(
        palette,
        app.shell_surface_color(palette.bg),
        "network-group-delete-modal",
        384.,
        card,
    )
}

pub(super) fn network_modal_shell(
    palette: crate::theme::ThemePalette,
    background: gpui::Rgba,
    id: impl Into<String>,
    width: f32,
    content: impl IntoElement,
) -> impl IntoElement {
    modal_dialog_shell(palette, background, id, width, content)
}

pub(super) fn network_dialog_footer(
    app: &NyaTermApp,
    palette: crate::theme::ThemePalette,
    cancel_id: impl Into<String>,
    save_id: impl Into<String>,
    save_label: &'static str,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_save: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    modal_dialog_footer_localized(
        palette,
        cancel_id,
        save_id,
        app.tr("common.cancel"),
        save_label,
        on_cancel,
        on_save,
    )
}

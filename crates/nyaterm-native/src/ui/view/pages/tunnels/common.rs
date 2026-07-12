use super::*;

pub(super) fn network_tab_button(
    id: impl Into<String>,
    label: &'static str,
    active: bool,
    palette: crate::ui::theme::ThemePalette,
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
        .font_weight(if active { FontWeight(600.) } else { FontWeight(500.) })
        .cursor_pointer()
        .hover(move |this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
        .child(label)
        .on_click(on_click)
}

pub(super) fn network_delete_confirm_panel(
    palette: crate::ui::theme::ThemePalette,
    confirm: NetworkDeleteConfirmState,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let card = div()
        .p_4()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .text_size(px(15.))
                .font_weight(FontWeight(700.))
                .text_color(rgb(0xfda4af))
                .child(format!("Delete {} profile?", confirm.tab.label())),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(palette.text))
                .child(format!(
                    "{} · {}",
                    truncate_preview(&confirm.label, 72),
                    truncate_preview(&confirm.id, 32)
                )),
        )
        .child(network_dialog_footer(palette, 
            "network-delete-cancel",
            "network-delete-confirm",
            "Delete",
            cx.listener(|this, _, _, cx| {
                this.cancel_network_delete(cx);
            }),
            cx.listener(|this, _, _, cx| {
                this.confirm_network_delete(cx);
            }),
        ));
    network_modal_shell(palette, "network-delete-confirm-modal", 420., card)
}

pub(super) fn network_group_editor_panel(
    palette: crate::ui::theme::ThemePalette,
    editor: NetworkGroupEditorState,
    focus: &gpui::FocusHandle,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let card = div()
        .p_4()
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
                            format!("Rename {} group", editor.tab.label())
                        } else {
                            format!("New {} group", editor.tab.label())
                        }),
                )
                .child(status_pill(
                    editor.tab.label(),
                    rgb(0x93c5fd),
                    rgb(0x17233a),
                )),
        )
        .child(
            transfer_input(
                "network-group-editor-name",
                "Group name",
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
            this.child(div().text_size(px(12.)).text_color(rgb(0xfda4af)).child(error))
        })
        .child(network_dialog_footer(palette, 
            "network-group-editor-cancel",
            "network-group-editor-save",
            "Save",
            cx.listener(|this, _, _, cx| {
                this.close_network_group_editor(cx);
            }),
            cx.listener(|this, _, _, cx| {
                this.save_network_group_editor(cx);
            }),
        ));
    network_modal_shell(palette, "network-group-editor-modal", 420., card)
}

pub(super) fn network_group_delete_confirm_panel(
    palette: crate::ui::theme::ThemePalette,
    confirm: NetworkGroupDeleteConfirmState,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let card = div()
        .p_4()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .text_size(px(15.))
                .font_weight(FontWeight(700.))
                .text_color(rgb(0xfda4af))
                .child(format!("Delete {} group?", confirm.tab.label())),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(palette.text))
                .child(format!(
                    "{} · {} item(s) will be removed",
                    truncate_preview(&confirm.label, 72),
                    confirm.item_count
                )),
        )
        .child(network_dialog_footer(palette, 
            "network-group-delete-cancel",
            "network-group-delete-confirm",
            "Delete",
            cx.listener(|this, _, _, cx| {
                this.cancel_network_group_delete(cx);
            }),
            cx.listener(|this, _, _, cx| {
                this.confirm_network_group_delete(cx);
            }),
        ));
    network_modal_shell(palette, "network-group-delete-modal", 420., card)
}


pub(super) fn network_modal_shell(
    palette: crate::ui::theme::ThemePalette,
    id: impl Into<String>,
    width: f32,
    content: impl IntoElement,
) -> impl IntoElement {
    modal_dialog_shell(palette, id, width, content)
}

pub(super) fn network_dialog_footer(
    palette: crate::ui::theme::ThemePalette,
    cancel_id: impl Into<String>,
    save_id: impl Into<String>,
    save_label: &'static str,
    on_cancel: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_save: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    modal_dialog_footer(palette, cancel_id, save_id, save_label, on_cancel, on_save)
}


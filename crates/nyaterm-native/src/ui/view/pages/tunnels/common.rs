use super::*;

pub(super) fn network_tab_button(
    id: impl Into<String>,
    label: &'static str,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(gpui::SharedString::from(id.into()))
        .h(px(30.))
        .px_3()
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(if active { rgb(0x38bdf8) } else { rgb(0x303848) })
        .bg(if active { rgb(0x102a3d) } else { rgb(0x0d1320) })
        .text_color(if active { rgb(0x7dd3fc) } else { rgb(0x98a3b8) })
        .text_sm()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x18202b)))
        .child(label)
        .on_click(on_click)
}

pub(super) fn network_delete_confirm_panel(
    confirm: NetworkDeleteConfirmState,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xfb7185))
        .bg(rgb(0x2a121a))
        .p_3()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            div()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight(800.))
                        .text_color(rgb(0xfda4af))
                        .child(format!("Delete {} profile?", confirm.tab.label())),
                )
                .child(div().text_xs().text_color(rgb(0xfecdd3)).child(format!(
                    "{} · {}",
                    truncate_preview(&confirm.label, 72),
                    truncate_preview(&confirm.id, 32)
                ))),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(small_button(
                    "network-delete-cancel",
                    "Cancel",
                    cx.listener(|this, _, _, cx| {
                        this.cancel_network_delete(cx);
                    }),
                ))
                .child(small_button(
                    "network-delete-confirm",
                    "Delete",
                    cx.listener(|this, _, _, cx| {
                        this.confirm_network_delete(cx);
                    }),
                )),
        )
}

pub(super) fn network_group_editor_panel(
    editor: NetworkGroupEditorState,
    focus: &gpui::FocusHandle,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0x38bdf8))
        .bg(rgb(0x102033))
        .p_3()
        .flex()
        .flex_col()
        .gap_3()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight(800.))
                        .text_color(rgb(0xe5edf7))
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
            this.child(div().text_xs().text_color(rgb(0xfda4af)).child(error))
        })
        .child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap_2()
                .child(small_button(
                    "network-group-editor-cancel",
                    "Cancel",
                    cx.listener(|this, _, _, cx| {
                        this.close_network_group_editor(cx);
                    }),
                ))
                .child(small_button(
                    "network-group-editor-save",
                    "Save",
                    cx.listener(|this, _, _, cx| {
                        this.save_network_group_editor(cx);
                    }),
                )),
        )
}

pub(super) fn network_group_delete_confirm_panel(
    confirm: NetworkGroupDeleteConfirmState,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(0xfb7185))
        .bg(rgb(0x2a121a))
        .p_3()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .child(
            div()
                .min_w_0()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight(800.))
                        .text_color(rgb(0xfda4af))
                        .child(format!("Delete {} group?", confirm.tab.label())),
                )
                .child(div().text_xs().text_color(rgb(0xfecdd3)).child(format!(
                    "{} · {} item(s) will be removed",
                    truncate_preview(&confirm.label, 72),
                    confirm.item_count
                ))),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(small_button(
                    "network-group-delete-cancel",
                    "Cancel",
                    cx.listener(|this, _, _, cx| {
                        this.cancel_network_group_delete(cx);
                    }),
                ))
                .child(small_button(
                    "network-group-delete-confirm",
                    "Delete",
                    cx.listener(|this, _, _, cx| {
                        this.confirm_network_group_delete(cx);
                    }),
                )),
        )
}

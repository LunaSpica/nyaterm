use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn quick_switch_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let items = self.filtered_quick_switch_items();
        if self.quick_switch_selected_index >= items.len() && !items.is_empty() {
            self.quick_switch_selected_index = items.len() - 1;
        }
        let selected_index = self.quick_switch_selected_index;
        let query_display = if self.quick_switch_query.is_empty() {
            "Search sessions and saved connections".to_string()
        } else {
            self.quick_switch_query.clone()
        };
        let mut rows = div().max_h(px(384.)).overflow_hidden().flex().flex_col();

        if items.is_empty() {
            rows = rows.child(
                div()
                    .h(px(120.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .child(if self.quick_switch_items().is_empty() {
                        "No sessions or saved connections."
                    } else {
                        "No matches."
                    }),
            );
        } else {
            for (index, item) in items.into_iter().enumerate().take(12) {
                let selected = index == selected_index;
                let item_for_click = item.clone();
                let badge = match &item {
                    QuickSwitchItem::Session { active, unread, .. } => {
                        if *active {
                            status_pill("active", rgb(palette.success), rgb(palette.hover)).into_any_element()
                        } else if *unread {
                            status_pill("unread", rgb(0xfacc15), rgb(0x3a2f14)).into_any_element()
                        } else {
                            status_pill("open", rgb(0x93c5fd), rgb(0x17233a)).into_any_element()
                        }
                    }
                    QuickSwitchItem::Connection { .. } => {
                        status_pill("saved", rgb(0xc4b5fd), rgb(0x2b2142)).into_any_element()
                    }
                    QuickSwitchItem::Pending { .. } => {
                        status_pill("connecting", rgb(0xfacc15), rgb(0x3a2f14)).into_any_element()
                    }
                };

                rows = rows.child(
                    div()
                        .id(SharedString::from(format!(
                            "quick-switch-item-{}",
                            item.id()
                        )))
                        .min_h(px(48.))
                        .px_3()
                        .py_2()
                        .flex()
                        .items_center()
                        .gap_3()
                        .border_b_1()
                        .border_color(rgb(palette.border))
                        .bg(if selected {
                            rgb(0x17233a)
                        } else {
                            rgb(palette.input)
                        })
                        .cursor_pointer()
                        .hover(|this| this.bg(rgb(0x151b24)))
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap_1()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight(800.))
                                        .text_color(rgb(palette.text))
                                        .overflow_hidden()
                                        .child(truncate_preview(item.title(), 54)),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .overflow_hidden()
                                        .child(truncate_preview(item.subtitle(), 78)),
                                ),
                        )
                        .child(badge)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.quick_switch_selected_index = index;
                            this.select_quick_switch_item(item_for_click.clone(), window, cx);
                        })),
                );
            }
        }

        div()
            .id(SharedString::from("quick-switch-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_start()
            .justify_center()
            .pt(px(96.))
            .track_focus(&self.quick_switch_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.quick_switch_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                this.handle_quick_switch_key_down(event, window, cx);
            }))
            .child(
                div()
                    .id(SharedString::from("quick-switch-dialog"))
                    .w(px(640.))
                    .max_w_full()
                    .mx_4()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .overflow_hidden()
                    .child(
                        div()
                            .h(px(44.))
                            .flex()
                            .items_center()
                            .gap_3()
                            .px_3()
                            .border_b_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(0x0f131a))
                            .child(
                                div()
                                    .size(px(18.))
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(rgb(palette.border))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .text_xs()
                                    .text_color(rgb(0x93c5fd))
                                    .child("/"),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .text_sm()
                                    .text_color(if self.quick_switch_query.is_empty() {
                                        rgb(palette.text_muted)
                                    } else {
                                        rgb(palette.text)
                                    })
                                    .child(query_display),
                            ),
                    )
                    .child(rows)
                    .child(
                        div()
                            .h(px(40.))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .px_3()
                            .border_t_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(0x0f131a))
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(palette.text_muted))
                                    .child("Enter open / Esc close / Up Down navigate"),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(small_button(palette, 
                                        "quick-switch-new-ssh",
                                        "New SSH",
                                        cx.listener(|this, _, _, cx| {
                                            this.quick_switch_open = false;
                                            this.open_page(NavItem::Connections, cx);
                                            this.terminal_status =
                                                "new SSH session page opened".to_string();
                                        }),
                                    ))
                                    .child(small_button(palette, 
                                        "quick-switch-close",
                                        "Close",
                                        cx.listener(|this, _, _, cx| {
                                            this.close_quick_switch(cx);
                                        }),
                                    )),
                            ),
                    ),
            )
    }
}

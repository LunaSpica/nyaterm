use gpui::{Context, FontWeight, IntoElement, div, prelude::*, rgb};

use crate::ui::components::{section_header, small_button};

use super::super::{NyaTermApp, connection_row};

impl NyaTermApp {
    pub(in crate::ui::view) fn connections_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let mut list = div().flex().flex_col().gap_2();
        let selected_count = self.selected_connections().len();
        if self.connections.is_empty() {
            list = list.child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .text_color(rgb(0xaeb7c8))
                    .child("No saved connections were found in the native runtime directory yet."),
            );
        } else {
            for connection in &self.connections {
                let connection_for_click = connection.clone();
                let connection_id_for_select = connection.id.clone();
                let selected = self.selected_connection_ids.contains(&connection.id);
                list = list.child(connection_row(
                    connection,
                    selected,
                    cx.listener(move |this, _, _, cx| {
                        this.toggle_connection_selected(connection_id_for_select.clone(), cx);
                    }),
                    cx.listener(move |this, _, window, cx| {
                        this.start_saved_connection(connection_for_click.clone(), window, cx);
                    }),
                ));
            }
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_5()
            .gap_4()
            .child(section_header(
                "Connections",
                "Compatible with the saved connection schema from the Tauri app.",
            ))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x10151e))
                    .p_3()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(0xe5edf7))
                            .child(format!("{selected_count} selected")),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(small_button(
                                "connections-select-all",
                                "Select All",
                                cx.listener(|this, _, _, cx| {
                                    this.select_all_connections(cx);
                                }),
                            ))
                            .child(small_button(
                                "connections-copy-selected",
                                "Copy Selected",
                                cx.listener(|this, _, _, cx| {
                                    this.copy_selected_connections(cx);
                                }),
                            ))
                            .child(small_button(
                                "connections-clear-selection",
                                "Clear",
                                cx.listener(|this, _, _, cx| {
                                    this.clear_selected_connections(cx);
                                }),
                            )),
                    ),
            )
            .child(list)
    }
}

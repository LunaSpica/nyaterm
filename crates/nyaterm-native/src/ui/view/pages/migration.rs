use gpui::{FontWeight, IntoElement, div, prelude::*, rgb};

use crate::ui::components::section_header;

use super::super::{NyaTermApp, metric, service_status};

impl NyaTermApp {
    pub(in crate::ui::view) fn migration_view(&self) -> impl IntoElement {
        let mut capabilities = div().flex().flex_col().gap_2();
        for capability in self.services.capabilities() {
            capabilities = capabilities.child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_4()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_3()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child(capability.area),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(0x98a3b8))
                                    .child(capability.note),
                            ),
                    )
                    .child(service_status(capability.status)),
            );
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_5()
            .gap_4()
            .child(section_header(
                "Migration",
                "Inventory of the ignored Tauri source and the native replacement boundary.",
            ))
            .child(
                div()
                    .grid()
                    .grid_cols(4)
                    .gap_3()
                    .child(metric(
                        "Legacy source",
                        if self.inventory.exists {
                            "found"
                        } else {
                            "missing"
                        },
                    ))
                    .child(metric("Rust files", self.inventory.rust_files.to_string()))
                    .child(metric(
                        "Frontend files",
                        self.inventory.frontend_files.to_string(),
                    ))
                    .child(metric(
                        "Command modules",
                        self.inventory.command_modules.to_string(),
                    )),
            )
            .child(capabilities)
    }
}

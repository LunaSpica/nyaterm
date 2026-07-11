use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn quick_command_category_delete_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let delete =
            self.quick_command_category_delete
                .clone()
                .unwrap_or(QuickCommandCategoryDeleteState {
                    id: String::new(),
                    name: "category".to_string(),
                    command_count: 0,
                });

        div()
            .id(SharedString::from("quick-command-category-delete-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .id(SharedString::from("quick-command-category-delete-dialog"))
                    .w(px(420.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x7f1d1d))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(0xfca5a5))
                            .child("Delete Quick Command Category"),
                    )
                    .child(
                        div()
                            .mt_3()
                            .text_xs()
                            .line_height(px(18.))
                            .text_color(rgb(0xcbd5e1))
                            .child(format!(
                                "Delete '{}' and {} command(s) in it? This cannot be undone.",
                                delete.name, delete.command_count
                            )),
                    )
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(small_button(palette, 
                                "quick-command-category-delete-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_delete_quick_command_category(cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "quick-command-category-delete-confirm",
                                "Delete",
                                cx.listener(|this, _, _, cx| {
                                    this.confirm_delete_quick_command_category(cx);
                                }),
                            )),
                    ),
            )
    }

    pub(in crate::ui::view) fn quick_command_category_rename_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let rename =
            self.quick_command_category_rename
                .clone()
                .unwrap_or(QuickCommandCategoryRenameState {
                    id: String::new(),
                    original_name: "category".to_string(),
                    draft: String::new(),
                    error: None,
                });

        div()
            .id(SharedString::from("quick-command-category-rename-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .id(SharedString::from("quick-command-category-rename-dialog"))
                    .w(px(420.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child("Rename Quick Command Category"),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .line_height(px(18.))
                            .text_color(rgb(palette.text_muted))
                            .child(format!("Current name: {}", rename.original_name)),
                    )
                    .child(
                        transfer_input(
                            "quick-command-category-rename-input",
                            "Category Name",
                            rename.draft,
                            true,
                    self.theme_palette(),
                )
                        .mt_3()
                        .track_focus(&self.quick_command_category_rename_focus)
                        .on_click(cx.listener(|this, _, window, cx| {
                            window.focus(&this.quick_command_category_rename_focus);
                            cx.notify();
                        }))
                        .on_key_down(cx.listener(
                            |this, event: &KeyDownEvent, _, cx| {
                                cx.stop_propagation();
                                this.handle_quick_command_category_rename_key_down(event, cx);
                            },
                        )),
                    )
                    .when(rename.error.is_some(), |this| {
                        this.child(
                            div()
                                .mt_2()
                                .text_xs()
                                .text_color(rgb(0xfca5a5))
                                .child(rename.error.unwrap_or_default()),
                        )
                    })
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(small_button(palette, 
                                "quick-command-category-rename-cancel",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_rename_quick_command_category(cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "quick-command-category-rename-confirm",
                                "Rename",
                                cx.listener(|this, _, _, cx| {
                                    this.confirm_rename_quick_command_category(cx);
                                }),
                            )),
                    ),
            )
    }
}

use super::*;

use crate::models::{QuickCommandCategoryDeleteState, QuickCommandCategoryRenameState};

impl NyaTermApp {
    pub(in crate::features) fn quick_command_category_delete_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let delete = self
            .quick_command_state
            .dialogs
            .category_delete
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
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .id(SharedString::from("quick-command-category-delete-dialog"))
                    .w(px((self.last_viewport_size.0 - 32.).clamp(280., 384.)))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x7f1d1d))
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .p_6()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("quickCommands.deleteCategory")),
                    )
                    .child(
                        div()
                            .mt_3()
                            .text_xs()
                            .line_height(px(18.))
                            .text_color(rgb(palette.text_muted))
                            .child(
                                self.tr("quickCommands.deleteCategoryConfirm")
                                    .replace("{{name}}", &delete.name)
                                    .replace("{{count}}", &delete.command_count.to_string()),
                            ),
                    )
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "quick-command-category-delete-cancel",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_delete_quick_command_category(cx);
                                }),
                            ))
                            .child(dialog_action_button(
                                palette,
                                "quick-command-category-delete-confirm",
                                self.tr("common.delete"),
                                true,
                                cx.listener(|this, _, _, cx| {
                                    this.confirm_delete_quick_command_category(cx);
                                }),
                            )),
                    ),
            )
    }

    pub(in crate::features) fn quick_command_category_rename_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let rename = self
            .quick_command_state
            .dialogs
            .category_rename
            .clone()
            .unwrap_or(QuickCommandCategoryRenameState {
                id: String::new(),
                original_name: "category".to_string(),
                draft: String::new(),
                error: None,
            });
        // Built before the overlay, which reads `self` throughout: creating the
        // box needs it mutably.
        let rename_input = self
            .text_input_box(
                "quick-command.category-rename",
                &rename.draft,
                TextInputSetup::placeholder(self.tr("quickCommands.categoryName")),
                cx,
            )
            .into_any_element();

        div()
            .id(SharedString::from("quick-command-category-rename-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x00000080))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_quick_command_category_rename_key_down(event, cx);
            }))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .id(SharedString::from("quick-command-category-rename-dialog"))
                    .w(px((self.last_viewport_size.0 - 32.).clamp(280., 384.)))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .p_6()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("quickCommands.renameCategory")),
                    )
                    .child(div().mt_4().child(rename_input))
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
                            .child(small_button(
                                palette,
                                "quick-command-category-rename-cancel",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_rename_quick_command_category(cx);
                                }),
                            ))
                            .child(dialog_action_button(
                                palette,
                                "quick-command-category-rename-confirm",
                                self.tr("common.confirm"),
                                false,
                                cx.listener(|this, _, _, cx| {
                                    this.confirm_rename_quick_command_category(cx);
                                }),
                            )),
                    ),
            )
    }
}

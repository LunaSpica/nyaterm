use gpui::{
    AnyElement, Context, IntoElement as _, ParentElement as _, Styled as _, div,
    prelude::FluentBuilder as _, px, rgb,
};

use crate::features::{NyaTermApp, TextInputSetup};
use crate::models::QuickCommandCategoryRenameState;

impl NyaTermApp {
    pub(in crate::features) fn quick_command_category_rename_dialog_content(
        &mut self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.theme_palette();
        let rename = self.commands.quick_category_rename().cloned().unwrap_or(
            QuickCommandCategoryRenameState {
                id: String::new(),
                original_name: String::new(),
                draft: String::new(),
                error: None,
            },
        );
        let input = self
            .text_input_box(
                "quick-command.category-rename",
                &rename.draft,
                TextInputSetup::placeholder(self.tr("quickCommands.categoryName")),
                cx,
            )
            .into_any_element();

        div()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_2()
            .child(input)
            .when_some(rename.error, |this, error| {
                this.child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette.danger))
                        .child(error),
                )
            })
            .into_any_element()
    }
}

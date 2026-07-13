use super::*;

impl NyaTermApp {
    pub(super) fn quick_command_category_sidebar(
        &mut self,
        categories: Vec<QuickCommandCategoryOption>,
        palette: crate::theme::ThemePalette,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let mut category_sidebar = div()
            .w(px(140.))
            .flex_shrink_0()
            .pr_2()
            .border_r_1()
            .border_color(rgb(palette.border))
            .flex()
            .flex_col()
            .gap_1();
        for option in categories {
            let id = option.id.clone();
            let rename_id = option.id.clone();
            let delete_id = option.id.clone();
            let selected = self.quick_command_selected_category == option.id;
            let manageable = option.manageable;
            category_sidebar = category_sidebar
                .child(
                    div()
                        .id(SharedString::from(format!(
                            "quick-command-category-{}",
                            option.id
                        )))
                        .h(px(32.))
                        .px_2()
                        .flex()
                        .items_center()
                        .gap_2()
                        .rounded_sm()
                        .bg(if selected {
                            rgb(palette.section_header)
                        } else {
                            rgba(0x00000000)
                        })
                        .text_xs()
                        .text_color(if selected {
                            rgb(palette.accent)
                        } else {
                            rgb(palette.text)
                        })
                        .cursor_pointer()
                        .hover(move |this| this.bg(rgb(palette.hover)))
                        .child(div().size(px(6.)).rounded_full().bg(if selected {
                            rgb(palette.accent)
                        } else {
                            rgb(palette.text_dimmed)
                        }))
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .overflow_hidden()
                                .font_weight(FontWeight(700.))
                                .child(option.label),
                        )
                        .child(
                            div()
                                .rounded_sm()
                                .px_2()
                                .py(px(1.))
                                .bg(if selected {
                                    rgb(0x203456)
                                } else {
                                    rgb(palette.border)
                                })
                                .text_size(px(10.))
                                .text_color(if selected {
                                    rgb(0xbfdbfe)
                                } else {
                                    rgb(palette.text_muted)
                                })
                                .child(option.count.to_string()),
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.quick_command_selected_category = id.clone();
                            this.quick_command_menu_id = None;
                            cx.notify();
                        })),
                )
                .when(selected && manageable, |this| {
                    this.child(
                        div()
                            .pl_4()
                            .pb_1()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
                                palette,
                                format!("quick-command-category-rename-{}", rename_id),
                                "Rename",
                                cx.listener(move |this, _, window, cx| {
                                    this.open_rename_quick_command_category(
                                        rename_id.clone(),
                                        window,
                                        cx,
                                    );
                                }),
                            ))
                            .child(small_button(
                                palette,
                                format!("quick-command-category-delete-{}", delete_id),
                                "Delete",
                                cx.listener(move |this, _, _, cx| {
                                    this.open_delete_quick_command_category_confirm(
                                        delete_id.clone(),
                                        cx,
                                    );
                                }),
                            )),
                    )
                });
        }
        category_sidebar
    }
}

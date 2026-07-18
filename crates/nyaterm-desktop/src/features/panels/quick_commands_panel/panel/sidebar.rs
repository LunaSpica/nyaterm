use super::*;

impl NyaTermApp {
    pub(super) fn quick_command_category_sidebar(
        &mut self,
        categories: Vec<QuickCommandCategoryOption>,
        palette: crate::theme::ThemePalette,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let mut category_sidebar = div()
            .w(px(176.))
            .flex_shrink_0()
            .p_1()
            .border_r_1()
            .border_color(rgb(palette.border))
            .flex()
            .flex_col()
            .gap_1();
        for option in categories {
            let id = option.id.clone();
            let menu_id = option.id.clone();
            let selected = self.quick_command_selected_category == option.id;
            let manageable = option.manageable;
            let menu_open = self
                .quick_command_category_menu
                .as_ref()
                .is_some_and(|menu| menu.category_id == option.id);
            category_sidebar = category_sidebar.child(
                div()
                    .id(SharedString::from(format!(
                        "quick-command-category-{}",
                        option.id
                    )))
                    .relative()
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
                        this.quick_command_menu = None;
                        this.quick_command_category_menu = None;
                        cx.notify();
                    }))
                    .when(manageable, |this| {
                        this.on_mouse_down(
                            MouseButton::Right,
                            cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                                cx.stop_propagation();
                                this.quick_command_menu = None;
                                this.quick_command_category_menu =
                                    Some(QuickCommandCategoryMenuState {
                                        category_id: menu_id.clone(),
                                        x: event.position.x,
                                        y: event.position.y,
                                    });
                                cx.notify();
                            }),
                        )
                    })
                    .when(menu_open && manageable, |this| this.bg(rgb(palette.hover))),
            );
        }
        category_sidebar
    }
}

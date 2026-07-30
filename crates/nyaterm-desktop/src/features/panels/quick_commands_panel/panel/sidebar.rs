use gpui::{Context, FontWeight, SharedString, div, prelude::*, px, rgb, rgba};
use nyaterm_ui::{NyaContextMenu, NyaMenuItem};

use super::super::super::QuickCommandCategoryOption;
use crate::features::NyaTermApp;

impl NyaTermApp {
    pub(super) fn quick_command_category_sidebar(
        &mut self,
        categories: Vec<QuickCommandCategoryOption>,
        palette: crate::theme::ThemePalette,
        cx: &mut Context<Self>,
    ) -> gpui::Stateful<gpui::Div> {
        let mut category_sidebar = div()
            .id(SharedString::from("quick-command-category-scroll"))
            .w(px(176.))
            .h_full()
            .flex_shrink_0()
            .overflow_scroll()
            .scrollbar_width(px(6.))
            .p(px(6.))
            .border_r_1()
            .border_color(rgb(palette.border))
            .flex()
            .flex_col()
            .gap_1();
        for option in categories {
            let id = option.id.clone();
            let selected = self.commands.quick_selected_category() == option.id;
            let manageable = option.manageable;
            let menu_items =
                manageable.then(|| self.quick_command_category_menu_items(option.id.clone(), cx));
            let row = div()
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
                .rounded_md()
                .bg(if selected {
                    rgb(palette.hover)
                } else {
                    rgba(0x00000000)
                })
                .text_xs()
                .text_color(if selected {
                    rgb(palette.link)
                } else {
                    rgb(palette.text)
                })
                .cursor_pointer()
                .hover(move |this| this.bg(rgb(palette.hover)))
                .child(div().size(px(6.)).rounded_full().bg(if selected {
                    rgb(palette.link)
                } else {
                    rgb(palette.text_dimmed)
                }))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .overflow_hidden()
                        .font_weight(FontWeight(500.))
                        .child(option.label),
                )
                .child(
                    div()
                        .rounded_sm()
                        .px_2()
                        .py(px(1.))
                        .bg(if selected {
                            rgba((palette.primary << 8) | 0x24)
                        } else {
                            rgb(palette.hover)
                        })
                        .text_size(px(10.))
                        .text_color(if selected {
                            rgb(palette.primary)
                        } else {
                            rgb(palette.text_muted)
                        })
                        .child(option.count.to_string()),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.commands.select_quick_category(id.clone());
                    cx.notify();
                }));
            category_sidebar = category_sidebar.child(if let Some(menu_items) = menu_items {
                NyaContextMenu::new(row, menu_items).into_any_element()
            } else {
                row.into_any_element()
            });
        }
        category_sidebar
    }

    fn quick_command_category_menu_items(
        &self,
        category_id: String,
        cx: &mut Context<Self>,
    ) -> Vec<NyaMenuItem> {
        let rename_id = category_id.clone();
        let delete_id = category_id;
        vec![
            NyaMenuItem::action(self.tr("quickCommands.edit"))
                .icon("icons/net/edit.svg")
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_rename_quick_command_category(rename_id.clone(), window, cx);
                })),
            NyaMenuItem::separator(),
            NyaMenuItem::action(self.tr("common.delete"))
                .icon("icons/net/delete.svg")
                .danger()
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_delete_quick_command_category_confirm(delete_id.clone(), window, cx);
                })),
        ]
    }
}

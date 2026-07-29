use gpui::{
    AnyElement, App, ClickEvent, Context, FontWeight, IntoElement, KeyDownEvent, MouseButton,
    SharedString, Window, div, prelude::*, px, rgb, rgba, svg, uniform_list,
};

use super::super::{filtered_quick_commands, quick_command_category_options};
use crate::features::{ChromeTooltip, NyaTermApp, TextInputSetup};
use crate::models::{QuickCommandSortMode, QuickCommandViewMode};
use crate::widgets::small_button;

mod rows;
use rows::quick_command_tile_column_count;
mod sidebar;
impl NyaTermApp {
    pub(in crate::features) fn quick_commands_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let filtered_commands = filtered_quick_commands(
            self.commands.quick_commands(),
            self.commands.quick_command_categories(),
            self.commands.quick_search_draft(),
            self.commands.quick_selected_category(),
            self.commands.quick_sort_mode(),
        );
        let total_commands = self.commands.quick_commands().len();
        let visible_commands = filtered_commands.len();
        let _pinned_commands = self
            .commands
            .quick_commands()
            .iter()
            .filter(|command| command.pinned.unwrap_or_default())
            .count();
        let categories = quick_command_category_options(
            self.commands.quick_commands(),
            self.commands.quick_command_categories(),
            self.tr("quickCommands.allCategories"),
            self.tr("quickCommands.uncategorized"),
        );
        let palette = self.theme_palette();
        let popover_bg = self.shell_surface_color(palette.surface);
        let input_bg = self.shell_surface_color(palette.bg);
        let search_draft = self.commands.quick_search_draft().to_string();
        let ai_prompt_draft = self.commands.quick_ai_prompt_draft().to_string();
        let search_field = self.text_input(
            "quick-command.search",
            &search_draft,
            TextInputSetup::placeholder(self.tr("quickCommands.search")),
            cx,
        );
        let search_focus = search_field.read(cx).focus_handle();
        let ai_prompt_input = self
            .text_input_box(
                "quick-command.ai-prompt",
                &ai_prompt_draft,
                TextInputSetup::placeholder(self.tr("ai.placeholder")),
                cx,
            )
            .into_any_element();
        let view_icon = match self.commands.quick_view_mode() {
            QuickCommandViewMode::List => "icons/view-list.svg",
            QuickCommandViewMode::Compact => "icons/view-compact.svg",
            QuickCommandViewMode::Tile => "icons/view-grid.svg",
        };

        let category_sidebar = self.quick_command_category_sidebar(categories, palette, cx);
        let view_mode = self.commands.quick_view_mode();
        let tile_columns = quick_command_tile_column_count(
            self.shell.viewport_size().0,
            self.shell.left_panel_width(),
            self.shell.right_panel_width(),
            !self.shell.left_panel_collapsed(),
            !self.shell.right_panel_collapsed(),
        );
        let logical_row_height = match view_mode {
            QuickCommandViewMode::Tile => 32.,
            QuickCommandViewMode::Compact => 38.,
            QuickCommandViewMode::List => 50.,
        };
        let rows = if filtered_commands.is_empty() {
            div()
                .id(SharedString::from("quick-command-rows-scroll"))
                .flex_1()
                .min_h_0()
                .overflow_scroll()
                .scrollbar_width(px(6.))
                .child(
                    div()
                        .mt_8()
                        .mx_auto()
                        .w_full()
                        .max_w(px(384.))
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(palette.border))
                        .p_4()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .gap_2()
                        .text_xs()
                        .text_color(rgb(palette.text_muted))
                        .line_height(px(18.))
                        .opacity(0.72)
                        .child(
                            svg()
                                .size(px(24.))
                                .flex_none()
                                .text_color(rgb(palette.text_muted))
                                .path("icons/conn/terminal.svg"),
                        )
                        .child(self.tr("quickCommands.noCommandsFound"))
                        .when(total_commands == 0, |this| {
                            this.child(small_button(
                                palette,
                                "quick-command-empty-add",
                                self.tr("quickCommands.addCommand"),
                                cx.listener(|this, _, window, cx| {
                                    this.open_new_quick_command_editor(window, cx);
                                }),
                            ))
                        }),
                )
                .into_any_element()
        } else {
            let logical_row_count = match view_mode {
                QuickCommandViewMode::Tile => filtered_commands.len().div_ceil(tile_columns),
                QuickCommandViewMode::Compact | QuickCommandViewMode::List => {
                    filtered_commands.len()
                }
            };
            uniform_list(
                "quick-command-rows-scroll",
                logical_row_count,
                cx.processor(move |this, range: std::ops::Range<usize>, _, cx| {
                    let mut rows = Vec::with_capacity(range.len());
                    for logical_index in range {
                        let (start, end) = match view_mode {
                            QuickCommandViewMode::Tile => {
                                let start = logical_index.saturating_mul(tile_columns);
                                (
                                    start,
                                    start
                                        .saturating_add(tile_columns)
                                        .min(filtered_commands.len()),
                                )
                            }
                            QuickCommandViewMode::Compact | QuickCommandViewMode::List => {
                                (logical_index, logical_index.saturating_add(1))
                            }
                        };
                        if start >= end || end > filtered_commands.len() {
                            continue;
                        }
                        let command_items =
                            this.quick_command_items(&filtered_commands[start..end], palette, cx);
                        let row = div()
                            .h(px(logical_row_height))
                            .w_full()
                            .flex_none()
                            .when(view_mode == QuickCommandViewMode::Tile, |this| {
                                this.grid().grid_cols(tile_columns as u16).gap_1()
                            })
                            .when(view_mode != QuickCommandViewMode::Tile, |this| {
                                this.flex().items_start()
                            })
                            .children(command_items);
                        rows.push(row);
                    }
                    rows
                }),
            )
            .flex_1()
            .min_h_0()
            .into_any_element()
        };

        // Tauri QuickCommands: PanelHeader-like strip with search + compact actions,
        // then category sidebar + command list (no page metrics cards).
        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(self.shell_transparent_color(palette.surface))
            .child(
                div()
                    .h(px(36.))
                    .flex_none()
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_transparent_color(palette.section_header))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text_muted))
                            .child(self.tr("panel.quickCommands")),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(if visible_commands == total_commands {
                                total_commands.to_string()
                            } else {
                                format!("{visible_commands}/{total_commands}")
                            }),
                    )
                    .child(div().flex_1())
                    .child(
                        div()
                            .id(SharedString::from("quick-command-search-input"))
                            .h(px(26.))
                            .w(px(144.))
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .bg(input_bg)
                            .px_2()
                            .flex()
                            .items_center()
                            .gap_1()
                            .cursor_text()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, _, window, cx| {
                                    this.close_quick_command_toolbar_popovers();
                                    window.focus(&search_focus);
                                    cx.notify();
                                }),
                            )
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                if event.keystroke.key == "escape" {
                                    cx.stop_propagation();
                                    this.commands.clear_quick_filters();
                                    this.reset_text_input("quick-command.search", "", cx);
                                    this.shell.status = "quick command filters cleared".to_string();
                                    cx.notify();
                                }
                            }))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close_quick_command_toolbar_popovers();
                                cx.notify();
                            }))
                            .child(
                                svg()
                                    .size(px(14.))
                                    .flex_none()
                                    .path("icons/fe/search.svg")
                                    .text_color(rgb(palette.text_muted)),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette.text))
                                    .child(search_field),
                            ),
                    )
                    .child(quick_command_toolbar_divider(palette))
                    .child(quick_command_sort_menu_button(
                        palette,
                        popover_bg,
                        self.commands.quick_sort_mode(),
                        self.commands.quick_sort_menu_is_open(),
                        self.tr("quickCommands.sort"),
                        self.tr("quickCommands.sortByCreated"),
                        self.tr("quickCommands.sortByName"),
                        self.tr("quickCommands.sortByUseCount"),
                        cx,
                    ))
                    .child(quick_command_view_menu_button(
                        palette,
                        popover_bg,
                        self.commands.quick_view_mode(),
                        view_icon,
                        self.commands.quick_view_menu_is_open(),
                        self.tr("quickCommands.viewMode"),
                        self.tr("quickCommands.listMode"),
                        self.tr("quickCommands.compactListMode"),
                        self.tr("quickCommands.tileMode"),
                        cx,
                    ))
                    .child(quick_command_toolbar_divider(palette))
                    .child(quick_command_toolbar_icon_button(
                        palette,
                        "quick-command-add",
                        "icons/conn/add.svg",
                        false,
                        self.tr("quickCommands.addCommand"),
                        cx.listener(|this, _, window, cx| {
                            this.close_quick_command_toolbar_popovers();
                            this.open_new_quick_command_editor(window, cx);
                        }),
                    ))
                    .child(quick_command_toolbar_icon_button(
                        palette,
                        "quick-command-import",
                        "icons/import.svg",
                        self.commands.quick_import_path_prompt().is_some(),
                        self.tr("quickCommands.import"),
                        cx.listener(|this, _, window, cx| {
                            this.close_quick_command_toolbar_popovers();
                            this.open_quick_command_import_dialog(window, cx);
                        }),
                    ))
                    .child(quick_command_toolbar_divider(palette))
                    .child(quick_command_ai_popover_button(
                        palette,
                        popover_bg,
                        self.commands.quick_ai_popover_is_open(),
                        self.commands.quick_ai_prompt_draft().to_string(),
                        ai_prompt_input,
                        self.tr("ai.generateCommand"),
                        self.tr("ai.generate"),
                        cx,
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            let changed = this.commands.close_quick_toolbar_popovers();
                            if changed {
                                cx.notify();
                            }
                        }),
                    )
                    .child(category_sidebar)
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .min_h_0()
                            .h_full()
                            .p(px(6.))
                            .flex()
                            .flex_col()
                            .child(rows),
                    ),
            )
    }
}

fn quick_command_sort_menu_button(
    palette: crate::theme::ThemePalette,
    popover_bg: gpui::Rgba,
    current: QuickCommandSortMode,
    open: bool,
    sort_label: &'static str,
    created_label: &'static str,
    name_label: &'static str,
    usage_label: &'static str,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    div()
        .relative()
        .child(quick_command_toolbar_icon_button(
            palette,
            "quick-command-sort",
            "icons/conn/sort.svg",
            current != QuickCommandSortMode::Created || open,
            format!(
                "{} · {}",
                sort_label,
                quick_command_sort_mode_label(current, created_label, name_label, usage_label)
            ),
            cx.listener(|this, _, _, cx| {
                this.commands.toggle_quick_sort_menu();
                cx.notify();
            }),
        ))
        .when(open, |this| {
            this.child(
                quick_command_toolbar_dropdown(palette, popover_bg, "quick-command-sort-menu")
                    .child(quick_command_toolbar_menu_item(
                        palette,
                        "quick-command-sort-created",
                        created_label,
                        current == QuickCommandSortMode::Created,
                        cx.listener(|this, _, _, cx| {
                            this.set_quick_command_sort_mode(QuickCommandSortMode::Created, cx);
                        }),
                    ))
                    .child(quick_command_toolbar_menu_item(
                        palette,
                        "quick-command-sort-name",
                        name_label,
                        current == QuickCommandSortMode::Name,
                        cx.listener(|this, _, _, cx| {
                            this.set_quick_command_sort_mode(QuickCommandSortMode::Name, cx);
                        }),
                    ))
                    .child(quick_command_toolbar_menu_item(
                        palette,
                        "quick-command-sort-usage",
                        usage_label,
                        current == QuickCommandSortMode::Usage,
                        cx.listener(|this, _, _, cx| {
                            this.set_quick_command_sort_mode(QuickCommandSortMode::Usage, cx);
                        }),
                    )),
            )
        })
}

fn quick_command_view_menu_button(
    palette: crate::theme::ThemePalette,
    popover_bg: gpui::Rgba,
    current: QuickCommandViewMode,
    icon_path: &'static str,
    open: bool,
    view_label: &'static str,
    list_label: &'static str,
    compact_label: &'static str,
    tile_label: &'static str,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    div()
        .relative()
        .child(quick_command_toolbar_icon_button(
            palette,
            "quick-command-view",
            icon_path,
            true,
            format!(
                "{} · {}",
                view_label,
                quick_command_view_mode_label(current, list_label, compact_label, tile_label)
            ),
            cx.listener(|this, _, _, cx| {
                this.commands.toggle_quick_view_menu();
                cx.notify();
            }),
        ))
        .when(open, |this| {
            this.child(
                quick_command_toolbar_dropdown(palette, popover_bg, "quick-command-view-menu")
                    .child(quick_command_toolbar_menu_item(
                        palette,
                        "quick-command-view-list",
                        list_label,
                        current == QuickCommandViewMode::List,
                        cx.listener(|this, _, _, cx| {
                            this.set_quick_command_view_mode(QuickCommandViewMode::List, cx);
                        }),
                    ))
                    .child(quick_command_toolbar_menu_item(
                        palette,
                        "quick-command-view-compact",
                        compact_label,
                        current == QuickCommandViewMode::Compact,
                        cx.listener(|this, _, _, cx| {
                            this.set_quick_command_view_mode(QuickCommandViewMode::Compact, cx);
                        }),
                    ))
                    .child(quick_command_toolbar_menu_item(
                        palette,
                        "quick-command-view-tile",
                        tile_label,
                        current == QuickCommandViewMode::Tile,
                        cx.listener(|this, _, _, cx| {
                            this.set_quick_command_view_mode(QuickCommandViewMode::Tile, cx);
                        }),
                    )),
            )
        })
}

fn quick_command_ai_popover_button(
    palette: crate::theme::ThemePalette,
    popover_bg: gpui::Rgba,
    open: bool,
    prompt: String,
    prompt_input: AnyElement,
    button_label: &'static str,
    generate_label: &'static str,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let can_generate = !prompt.trim().is_empty();
    div()
        .relative()
        .child(quick_command_toolbar_icon_button(
            palette,
            "quick-command-ai",
            "icons/ai.svg",
            open,
            button_label,
            cx.listener(|this, _, window, cx| {
                this.toggle_quick_command_ai_popover(window, cx);
            }),
        ))
        .when(open, |this| {
            this.child(
                div()
                    .id(SharedString::from("quick-command-ai-popover"))
                    .absolute()
                    .top(px(28.))
                    .right_0()
                    .w(px(320.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(popover_bg)
                    .shadow_lg()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight(600.))
                            .text_color(rgb(palette.text))
                            .child(button_label),
                    )
                    .child(
                        div()
                            .id(SharedString::from("quick-command-ai-input"))
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                                cx.stop_propagation();
                                this.handle_quick_command_ai_prompt_key_down(event, window, cx);
                            }))
                            .child(prompt_input),
                    )
                    .child(
                        div().flex().justify_end().child(
                            div()
                                .id(SharedString::from("quick-command-ai-submit"))
                                .h(px(24.))
                                .px_2()
                                .rounded_md()
                                .flex()
                                .items_center()
                                .gap_1()
                                .text_size(px(11.))
                                .font_weight(FontWeight(600.))
                                .text_color(if prompt.trim().is_empty() {
                                    rgb(palette.text_dimmed)
                                } else {
                                    rgb(palette.text)
                                })
                                .bg(if prompt.trim().is_empty() {
                                    rgb(palette.input)
                                } else {
                                    rgb(palette.hover)
                                })
                                .when(can_generate, |this| {
                                    this.cursor_pointer().hover(|this| {
                                        this.bg(rgb(palette.surface_elevated))
                                            .text_color(rgb(palette.text))
                                    })
                                })
                                .child(
                                    svg()
                                        .size(px(13.))
                                        .flex_none()
                                        .path("icons/ai.svg")
                                        .text_color(if prompt.trim().is_empty() {
                                            rgb(palette.text_dimmed)
                                        } else {
                                            rgb(palette.text)
                                        }),
                                )
                                .child(generate_label)
                                .when(can_generate, |this| {
                                    this.on_click(cx.listener(|this, _, window, cx| {
                                        this.submit_quick_command_ai_prompt(window, cx);
                                    }))
                                }),
                        ),
                    ),
            )
        })
}

fn quick_command_toolbar_icon_button(
    palette: crate::theme::ThemePalette,
    id: &'static str,
    icon_path: &'static str,
    active: bool,
    tooltip: impl Into<String>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let tooltip = tooltip.into();
    div()
        .id(SharedString::from(id))
        .size(px(24.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(if active {
            rgb(palette.link)
        } else {
            rgb(palette.text_muted)
        })
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
        .child(
            svg()
                .size(px(15.))
                .flex_none()
                .path(icon_path)
                .text_color(if active {
                    rgb(palette.link)
                } else {
                    rgb(palette.text_muted)
                }),
        )
        .tooltip(move |_, cx| cx.new(|_| ChromeTooltip::new(tooltip.clone())).into())
        .on_click(on_click)
}

fn quick_command_toolbar_dropdown(
    palette: crate::theme::ThemePalette,
    popover_bg: gpui::Rgba,
    id: &'static str,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(SharedString::from(id))
        .absolute()
        .top(px(28.))
        .right_0()
        .w(px(154.))
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(popover_bg)
        .shadow_lg()
        .py_1()
        .flex()
        .flex_col()
}

fn quick_command_toolbar_menu_item(
    palette: crate::theme::ThemePalette,
    id: &'static str,
    label: &'static str,
    active: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id))
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .text_size(px(12.))
        .text_color(if active {
            rgb(palette.link)
        } else {
            rgb(palette.text)
        })
        .bg(if active {
            rgb(palette.hover)
        } else {
            rgba(0x00000000)
        })
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface_elevated)))
        .child(div().min_w_0().flex_1().child(label))
        .when(active, |this| {
            this.child(
                div()
                    .text_size(px(10.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(palette.link))
                    .child("ON"),
            )
        })
        .on_click(on_click)
}

fn quick_command_toolbar_divider(palette: crate::theme::ThemePalette) -> impl IntoElement {
    div()
        .mx_1()
        .h(px(16.))
        .w(px(1.))
        .flex_none()
        .bg(rgb(palette.border))
}

fn quick_command_view_mode_label(
    mode: QuickCommandViewMode,
    list_label: &'static str,
    compact_label: &'static str,
    tile_label: &'static str,
) -> &'static str {
    match mode {
        QuickCommandViewMode::List => list_label,
        QuickCommandViewMode::Compact => compact_label,
        QuickCommandViewMode::Tile => tile_label,
    }
}

fn quick_command_sort_mode_label(
    mode: QuickCommandSortMode,
    created_label: &'static str,
    name_label: &'static str,
    usage_label: &'static str,
) -> &'static str {
    match mode {
        QuickCommandSortMode::Created => created_label,
        QuickCommandSortMode::Name => name_label,
        QuickCommandSortMode::Usage => usage_label,
    }
}

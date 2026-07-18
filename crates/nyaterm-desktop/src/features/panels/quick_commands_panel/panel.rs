use super::*;

#[path = "panel/rows.rs"]
mod rows;
#[path = "panel/sidebar.rs"]
mod sidebar;
impl NyaTermApp {
    pub(in crate::features) fn quick_commands_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let filtered_commands = filtered_quick_commands(
            &self.quick_commands,
            &self.quick_command_categories,
            &self.quick_command_search_draft,
            &self.quick_command_selected_category,
            self.quick_command_sort_mode,
        );
        let total_commands = self.quick_commands.len();
        let visible_commands = filtered_commands.len();
        let _pinned_commands = self
            .quick_commands
            .iter()
            .filter(|command| command.pinned.unwrap_or_default())
            .count();
        let categories =
            quick_command_category_options(&self.quick_commands, &self.quick_command_categories);
        let palette = self.theme_palette();
        let view_icon = match self.quick_command_view_mode {
            QuickCommandViewMode::List => "icons/view-list.svg",
            QuickCommandViewMode::Compact => "icons/view-compact.svg",
            QuickCommandViewMode::Tile => "icons/view-grid.svg",
        };

        let category_sidebar = self.quick_command_category_sidebar(categories, palette, cx);
        let rows = self.quick_command_rows(filtered_commands, total_commands, palette, cx);

        // Tauri QuickCommands: PanelHeader-like strip with search + compact actions,
        // then category sidebar + command list (no page metrics cards).
        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(palette.surface))
            .child(
                div()
                    .h(px(36.))
                    .flex_none()
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.section_header))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(11.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text_muted))
                            .child("QUICK COMMANDS"),
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
                            .bg(rgb(palette.bg))
                            .px_2()
                            .flex()
                            .items_center()
                            .gap_1()
                            .cursor_text()
                            .track_focus(&self.quick_command_search_focus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.close_quick_command_toolbar_popovers();
                                window.focus(&this.quick_command_search_focus);
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                cx.stop_propagation();
                                this.handle_quick_command_search_key_down(event, cx);
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
                                    .text_color(if self.quick_command_search_draft.is_empty() {
                                        rgb(palette.text_dimmed)
                                    } else {
                                        rgb(palette.text)
                                    })
                                    .child(if self.quick_command_search_draft.is_empty() {
                                        "Search".to_string()
                                    } else {
                                        truncate_preview(&self.quick_command_search_draft, 18)
                                    }),
                            ),
                    )
                    .child(quick_command_toolbar_divider(palette))
                    .child(quick_command_sort_menu_button(
                        palette,
                        self.quick_command_sort_mode,
                        self.quick_command_sort_menu_open,
                        cx,
                    ))
                    .child(quick_command_view_menu_button(
                        palette,
                        self.quick_command_view_mode,
                        view_icon,
                        self.quick_command_view_menu_open,
                        cx,
                    ))
                    .child(quick_command_toolbar_divider(palette))
                    .child(quick_command_toolbar_icon_button(
                        palette,
                        "quick-command-add",
                        "icons/conn/add.svg",
                        false,
                        "Add command",
                        cx.listener(|this, _, window, cx| {
                            this.close_quick_command_toolbar_popovers();
                            this.open_new_quick_command_editor(window, cx);
                        }),
                    ))
                    .child(quick_command_toolbar_icon_button(
                        palette,
                        "quick-command-import",
                        "icons/import.svg",
                        self.quick_command_import_path_prompt.is_some(),
                        "Import",
                        cx.listener(|this, _, window, cx| {
                            this.close_quick_command_toolbar_popovers();
                            this.open_quick_command_import_dialog(window, cx);
                        }),
                    ))
                    .child(quick_command_toolbar_divider(palette))
                    .child(quick_command_ai_popover_button(
                        palette,
                        self.quick_command_ai_popover_open,
                        self.quick_command_ai_prompt_draft.clone(),
                        self.quick_command_ai_focus.clone(),
                        cx,
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .p_2()
                    .flex()
                    .items_start()
                    .gap_2()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            let changed = this.quick_command_sort_menu_open
                                || this.quick_command_view_menu_open
                                || this.quick_command_ai_popover_open;
                            this.close_quick_command_toolbar_popovers();
                            if changed {
                                cx.notify();
                            }
                        }),
                    )
                    .child(category_sidebar)
                    .child(
                        div().min_w_0().flex_1().flex().flex_col().gap_2().child(
                            div()
                                .id(SharedString::from("quick-command-rows-scroll"))
                                .flex_1()
                                .min_h_0()
                                .overflow_scroll()
                                .scrollbar_width(px(6.))
                                .child(rows),
                        ),
                    ),
            )
    }
}

fn quick_command_sort_menu_button(
    palette: crate::theme::ThemePalette,
    current: QuickCommandSortMode,
    open: bool,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    div()
        .relative()
        .child(quick_command_toolbar_icon_button(
            palette,
            "quick-command-sort",
            "icons/conn/sort.svg",
            current != QuickCommandSortMode::Created || open,
            format!("Sort · {}", quick_command_sort_mode_label(current)),
            cx.listener(|this, _, _, cx| {
                this.quick_command_sort_menu_open = !this.quick_command_sort_menu_open;
                if this.quick_command_sort_menu_open {
                    this.quick_command_view_menu_open = false;
                    this.quick_command_ai_popover_open = false;
                    this.quick_command_menu = None;
                }
                cx.notify();
            }),
        ))
        .when(open, |this| {
            this.child(
                quick_command_toolbar_dropdown(palette, "quick-command-sort-menu")
                    .child(quick_command_toolbar_menu_item(
                        palette,
                        "quick-command-sort-created",
                        "Created",
                        current == QuickCommandSortMode::Created,
                        cx.listener(|this, _, _, cx| {
                            this.set_quick_command_sort_mode(QuickCommandSortMode::Created, cx);
                        }),
                    ))
                    .child(quick_command_toolbar_menu_item(
                        palette,
                        "quick-command-sort-name",
                        "Name",
                        current == QuickCommandSortMode::Name,
                        cx.listener(|this, _, _, cx| {
                            this.set_quick_command_sort_mode(QuickCommandSortMode::Name, cx);
                        }),
                    ))
                    .child(quick_command_toolbar_menu_item(
                        palette,
                        "quick-command-sort-usage",
                        "Use count",
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
    current: QuickCommandViewMode,
    icon_path: &'static str,
    open: bool,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    div()
        .relative()
        .child(quick_command_toolbar_icon_button(
            palette,
            "quick-command-view",
            icon_path,
            true,
            format!("View · {}", quick_command_view_mode_label(current)),
            cx.listener(|this, _, _, cx| {
                this.quick_command_view_menu_open = !this.quick_command_view_menu_open;
                if this.quick_command_view_menu_open {
                    this.quick_command_sort_menu_open = false;
                    this.quick_command_ai_popover_open = false;
                    this.quick_command_menu = None;
                }
                cx.notify();
            }),
        ))
        .when(open, |this| {
            this.child(
                quick_command_toolbar_dropdown(palette, "quick-command-view-menu")
                    .child(quick_command_toolbar_menu_item(
                        palette,
                        "quick-command-view-list",
                        "List",
                        current == QuickCommandViewMode::List,
                        cx.listener(|this, _, _, cx| {
                            this.set_quick_command_view_mode(QuickCommandViewMode::List, cx);
                        }),
                    ))
                    .child(quick_command_toolbar_menu_item(
                        palette,
                        "quick-command-view-compact",
                        "Compact",
                        current == QuickCommandViewMode::Compact,
                        cx.listener(|this, _, _, cx| {
                            this.set_quick_command_view_mode(QuickCommandViewMode::Compact, cx);
                        }),
                    ))
                    .child(quick_command_toolbar_menu_item(
                        palette,
                        "quick-command-view-tile",
                        "Tile",
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
    open: bool,
    prompt: String,
    focus: FocusHandle,
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
            "Generate command",
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
                    .bg(rgb(palette.surface))
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
                            .child("Generate command"),
                    )
                    .child(
                        div()
                            .id(SharedString::from("quick-command-ai-input"))
                            .h(px(32.))
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(palette.bg))
                            .px_2()
                            .flex()
                            .items_center()
                            .cursor_text()
                            .track_focus(&focus)
                            .on_click(cx.listener(|this, _, window, cx| {
                                window.focus(&this.quick_command_ai_focus);
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                                cx.stop_propagation();
                                this.handle_quick_command_ai_prompt_key_down(event, window, cx);
                            }))
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .text_size(px(12.))
                                    .text_color(if prompt.trim().is_empty() {
                                        rgb(palette.text_dimmed)
                                    } else {
                                        rgb(palette.text)
                                    })
                                    .child(if prompt.trim().is_empty() {
                                        "Describe the command you want...".to_string()
                                    } else {
                                        truncate_preview(&prompt, 44)
                                    }),
                            ),
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
                                .child(svg().size(px(13.)).flex_none().path("icons/ai.svg"))
                                .child("Generate")
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
            rgb(palette.accent)
        } else {
            rgb(palette.text_muted)
        })
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
        .child(svg().size(px(15.)).flex_none().path(icon_path))
        .tooltip(move |_, cx| cx.new(|_| ChromeTooltip::new(tooltip.clone())).into())
        .on_click(on_click)
}

fn quick_command_toolbar_dropdown(
    palette: crate::theme::ThemePalette,
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
        .bg(rgb(palette.surface))
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
            rgb(palette.accent)
        } else {
            rgb(palette.text)
        })
        .bg(if active {
            rgb(palette.hover)
        } else {
            rgb(palette.surface)
        })
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface_elevated)))
        .child(div().min_w_0().flex_1().child(label))
        .when(active, |this| {
            this.child(
                div()
                    .text_size(px(10.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(palette.accent))
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

fn quick_command_view_mode_label(mode: QuickCommandViewMode) -> &'static str {
    match mode {
        QuickCommandViewMode::List => "List",
        QuickCommandViewMode::Compact => "Compact",
        QuickCommandViewMode::Tile => "Tile",
    }
}

fn quick_command_sort_mode_label(mode: QuickCommandSortMode) -> &'static str {
    match mode {
        QuickCommandSortMode::Created => "Created",
        QuickCommandSortMode::Name => "Name",
        QuickCommandSortMode::Usage => "Use count",
    }
}

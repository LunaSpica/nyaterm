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
        let can_send_to_all = self.live_session_count() > 1;
        let palette = self.theme_palette();

        let category_sidebar = self.quick_command_category_sidebar(categories, palette, cx);
        let rows = self.quick_command_rows(
            filtered_commands,
            total_commands,
            can_send_to_all,
            palette,
            cx,
        );

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
                    .child(mode_button(
                        "quick-command-view-list",
                        "List",
                        self.quick_command_view_mode == QuickCommandViewMode::List, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.set_quick_command_view_mode(QuickCommandViewMode::List, cx);
                        }),
                    ))
                    .child(mode_button(
                        "quick-command-view-compact",
                        "Cmp",
                        self.quick_command_view_mode == QuickCommandViewMode::Compact, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.set_quick_command_view_mode(QuickCommandViewMode::Compact, cx);
                        }),
                    ))
                    .child(mode_button(
                        "quick-command-view-tile",
                        "Tile",
                        self.quick_command_view_mode == QuickCommandViewMode::Tile, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.set_quick_command_view_mode(QuickCommandViewMode::Tile, cx);
                        }),
                    ))
                    .child(mode_button(
                        "quick-command-sort-created",
                        "New",
                        self.quick_command_sort_mode == QuickCommandSortMode::Created, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.set_quick_command_sort_mode(QuickCommandSortMode::Created, cx);
                        }),
                    ))
                    .child(mode_button(
                        "quick-command-sort-name",
                        "Name",
                        self.quick_command_sort_mode == QuickCommandSortMode::Name, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.set_quick_command_sort_mode(QuickCommandSortMode::Name, cx);
                        }),
                    ))
                    .child(mode_button(
                        "quick-command-sort-usage",
                        "Use",
                        self.quick_command_sort_mode == QuickCommandSortMode::Usage, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.set_quick_command_sort_mode(QuickCommandSortMode::Usage, cx);
                        }),
                    ))
                    .child(small_button(palette,
                        "quick-command-add",
                        "Add",
                        cx.listener(|this, _, window, cx| {
                            this.open_new_quick_command_editor(window, cx);
                        }),
                    ))
                    .child(small_button(palette,
                        "quick-command-import",
                        if self.quick_command_import_path_prompt.is_some() {
                            "..."
                        } else {
                            "Import"
                        },
                        cx.listener(|this, _, window, cx| {
                            this.open_quick_command_import_dialog(window, cx);
                        }),
                    ))
                    .child(small_button(
                        palette,
                        "quick-command-ai",
                        "AI",
                        cx.listener(|this, _, window, cx| {
                            // Tauri QuickCommands AI popover -> openAIAssistant(generate_command).
                            if this.ai_prompt_draft.trim().is_empty() {
                                this.ai_prompt_draft =
                                    "Generate a shell command for: ".to_string();
                            }
                            this.ai_response_preview =
                                "Describe the command you want, then Ask.".to_string();
                            this.ai_status = "quick command AI assist".to_string();
                            this.ensure_panel_open(NavItem::AiAssistant);
                            window.focus(&this.ai_chat_focus);
                            cx.notify();
                        }),
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
                    .child(category_sidebar)

                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .when(visible_commands > 12, |this| {
                                this.child(
                                    div()
                                        .rounded_sm()
                                        .border_1()
                                        .border_color(rgb(palette.border))
                                        .bg(rgb(palette.input))
                                        .p_2()
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .child(format!(
                                            "Showing all {visible_commands} matching quick commands."
                                        )),
                                )
                            })
                            .child(
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

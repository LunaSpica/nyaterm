use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn quick_command_editor_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let editor = self
            .quick_command_editor
            .clone()
            .unwrap_or_else(QuickCommandEditorState::blank);
        let title = if editor.original.is_some() {
            "Edit Quick Command"
        } else {
            "Add Quick Command"
        };
        let category_label = editor
            .category_id
            .as_deref()
            .and_then(|id| {
                self.quick_command_categories
                    .iter()
                    .find(|category| category.id == id)
            })
            .map(|category| category.name.clone())
            .unwrap_or_else(|| "Uncategorized".to_string());
        let category_draft = editor.category_draft.trim().to_string();
        let category_query = category_draft.to_lowercase();
        let exact_category_match = self
            .quick_command_categories
            .iter()
            .any(|category| category.name.eq_ignore_ascii_case(&category_draft));
        let category_display = if category_draft.is_empty() {
            category_label.clone()
        } else if exact_category_match {
            format!("Use: {category_draft}")
        } else {
            format!("Create: {category_draft}")
        };
        let color_label = editor
            .color_tag
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let icon_label = quick_command_icon_label(editor.icon_tag.as_deref());
        let mut color_swatches = div().mt_2().flex().items_center().gap_2().flex_wrap();
        for option in QUICK_COMMAND_COLOR_OPTIONS {
            let selected = editor.color_tag.as_deref() == option && editor.icon_tag.is_none();
            color_swatches = color_swatches.child(quick_command_color_swatch(
                option,
                selected,
                cx.listener(move |this, _, _, cx| {
                    this.set_quick_command_editor_color(option, cx);
                }),
            ));
        }
        let icon_options = QUICK_COMMAND_ICON_OPTIONS
            .iter()
            .copied()
            .filter_map(|icon| icon)
            .collect::<Vec<_>>();
        let mut icon_grid = div().mt_2().flex().items_center().gap_1().flex_wrap();
        for option in icon_options {
            let selected = editor.icon_tag.as_deref() == Some(option);
            icon_grid = icon_grid.child(quick_command_icon_option(
                option,
                editor.color_tag.as_deref(),
                selected,
                cx.listener(move |this, _, _, cx| {
                    this.set_quick_command_editor_icon(Some(option), cx);
                }),
            ));
        }
        let mut category_choices = div().mt_2().flex().items_center().gap_1().flex_wrap();
        if category_draft.is_empty() {
            let uncategorized_selected =
                editor.category_id.as_deref().unwrap_or_default().is_empty();
            category_choices = category_choices.child(quick_command_category_choice(
                "quick-command-editor-category-none".to_string(),
                "Uncategorized".to_string(),
                uncategorized_selected,
                cx.listener(|this, _, _, cx| {
                    this.set_quick_command_editor_category(None, cx);
                }),
            ));
        }
        for category in self
            .quick_command_categories
            .clone()
            .into_iter()
            .filter(|category| {
                category_query.is_empty() || category.name.to_lowercase().contains(&category_query)
            })
        {
            let category_id = category.id.clone();
            let selected = editor.category_draft.trim().is_empty()
                && editor.category_id.as_deref() == Some(category_id.as_str());
            category_choices = category_choices.child(quick_command_category_choice(
                format!("quick-command-editor-category-{}", category.id),
                truncate_preview(&category.name, 22),
                selected,
                cx.listener(move |this, _, _, cx| {
                    this.set_quick_command_editor_category(Some(category_id.clone()), cx);
                }),
            ));
        }
        if !category_draft.is_empty() && !exact_category_match {
            let label = format!("Create: {}", truncate_preview(&category_draft, 18));
            category_choices = category_choices.child(quick_command_category_choice(
                "quick-command-editor-category-draft".to_string(),
                label,
                true,
                cx.listener(|this, _, _, cx| {
                    this.confirm_quick_command_editor_category_draft(cx);
                }),
            ));
        }
        let can_save = !editor.label.trim().is_empty() && !editor.command.trim().is_empty();

        div()
            .id(SharedString::from("quick-command-editor-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.quick_command_editor_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.quick_command_editor_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                this.handle_quick_command_editor_key_down(event, cx);
            }))
            .child(
                div()
                    .id(SharedString::from("quick-command-editor-dialog"))
                    .w(px(560.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x303848))
                    .bg(rgb(0x0b0f16))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(0xe5edf7))
                                    .child(title),
                            )
                            .child(status_pill(
                                if can_save { "ready" } else { "required" },
                                if can_save {
                                    rgb(0x6ee7b7)
                                } else {
                                    rgb(0xfca5a5)
                                },
                                if can_save {
                                    rgb(0x12342a)
                                } else {
                                    rgb(0x3a1717)
                                },
                            )),
                    )
                    .when(editor.error.is_some(), |this| {
                        this.child(
                            div()
                                .mt_3()
                                .rounded_sm()
                                .border_1()
                                .border_color(rgb(0x7f1d1d))
                                .bg(rgb(0x1f0b0b))
                                .p_2()
                                .text_xs()
                                .text_color(rgb(0xfca5a5))
                                .child(editor.error.clone().unwrap_or_default()),
                        )
                    })
                    .child(
                        div()
                            .mt_3()
                            .grid()
                            .grid_cols(2)
                            .gap_3()
                            .child(quick_command_editor_field(
                                "quick-command-editor-label",
                                "Label",
                                "Command label",
                                editor.label.clone(),
                                editor.focused_field == QuickCommandEditorField::Label,
                                cx.listener(|this, _, window, cx| {
                                    this.focus_quick_command_editor_field(
                                        QuickCommandEditorField::Label,
                                        window,
                                        cx,
                                    );
                                }),
                            ))
                            .child(
                                div()
                                    .id("quick-command-editor-category")
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(
                                        if editor.focused_field == QuickCommandEditorField::Category {
                                            rgb(0x4ade80)
                                        } else {
                                            rgb(0x263142)
                                        },
                                    )
                                    .bg(
                                        if editor.focused_field == QuickCommandEditorField::Category {
                                            rgb(0x0f1f18)
                                        } else {
                                            rgb(0x0d1320)
                                        },
                                    )
                                    .p_2()
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.focus_quick_command_editor_field(
                                            QuickCommandEditorField::Category,
                                            window,
                                            cx,
                                        );
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(rgb(0x64748b))
                                            .child("Category"),
                                    )
                                    .child(
                                        div()
                                            .mt_1()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .min_w_0()
                                                    .text_xs()
                                                    .text_color(rgb(0xe5edf7))
                                                    .child(truncate_preview(&category_display, 26)),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(10.))
                                                    .text_color(rgb(0x98a3b8))
                                                    .child("Type to search/create"),
                                            ),
                                    )
                                    .child(category_choices),
                            ),
                    )
                    .child(div().mt_3().child(quick_command_editor_field(
                        "quick-command-editor-description",
                        "Description",
                        "Optional description",
                        editor.description.clone(),
                        editor.focused_field == QuickCommandEditorField::Description,
                        cx.listener(|this, _, window, cx| {
                            this.focus_quick_command_editor_field(
                                QuickCommandEditorField::Description,
                                window,
                                cx,
                            );
                        }),
                    )))
                    .child(
                        div()
                            .mt_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(0x263142))
                            .bg(rgb(0x0d1320))
                            .p_2()
                            .flex()
                            .items_start()
                            .gap_4()
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .text_size(px(10.))
                                                    .text_color(rgb(0x64748b))
                                                    .child("Color Tag"),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_1()
                                                    .child(quick_command_icon_mark(
                                                        editor.icon_tag.as_deref(),
                                                        editor.color_tag.as_deref(),
                                                    ))
                                                    .child(
                                                        div()
                                                            .text_size(px(10.))
                                                            .text_color(rgb(0x98a3b8))
                                                            .child(format!(
                                                                "{} / {}",
                                                                color_label, icon_label
                                                            )),
                                                    ),
                                            ),
                                    )
                                    .child(color_swatches)
                                    .child(icon_grid),
                            )
                            .child(
                                div()
                                    .pl_4()
                                    .border_l_1()
                                    .border_color(rgb(0x263142))
                                    .flex()
                                    .flex_col()
                                    .items_end()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(rgb(0x64748b))
                                            .child("Pin"),
                                    )
                                    .child(mode_button(
                                        "quick-command-editor-pinned",
                                        if editor.pinned { "Pinned" } else { "Pin" },
                                        editor.pinned, self.theme_palette(),cx.listener(|this, _, _, cx| {
                                            this.toggle_quick_command_editor_pinned(cx);
                                        }),
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .mt_3()
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(0x64748b))
                                    .child("Execution Mode"),
                            )
                            .child(
                                div()
                                    .mt_2()
                                    .grid()
                                    .grid_cols(2)
                                    .gap_2()
                                    .child(mode_button(
                                        "quick-command-editor-execute",
                                        "Execute Immediately",
                                        editor.execution_mode != "append", self.theme_palette(),cx.listener(|this, _, _, cx| {
                                            this.set_quick_command_editor_execution_mode(
                                                "execute", cx,
                                            );
                                        }),
                                    ))
                                    .child(mode_button(
                                        "quick-command-editor-append",
                                        "Append Only",
                                        editor.execution_mode == "append", self.theme_palette(),cx.listener(|this, _, _, cx| {
                                            this.set_quick_command_editor_execution_mode(
                                                "append", cx,
                                            );
                                        }),
                                    )),
                            )
                            .child(
                                div()
                                    .mt_1()
                                    .text_size(px(10.))
                                    .text_color(rgb(0x98a3b8))
                                    .child(if editor.execution_mode == "append" {
                                        "Insert the command into the terminal input without running it."
                                    } else {
                                        "Send the command to the terminal and execute it immediately."
                                    }),
                            ),
                    )
                    .child(div().mt_3().child(quick_command_editor_script_field(
                        "quick-command-editor-command",
                        "Command Script",
                        "Command text",
                        editor.command.clone(),
                        editor.focused_field == QuickCommandEditorField::Command,
                        cx.listener(|this, _, window, cx| {
                            this.focus_quick_command_editor_field(
                                QuickCommandEditorField::Command,
                                window,
                                cx,
                            );
                        }),
                    )))
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(0x64748b))
                                    .child("Tab switches fields. Enter saves outside Command. Cmd/Ctrl+S saves."),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(small_button(palette, 
                                        "quick-command-editor-cancel",
                                        "Cancel",
                                        cx.listener(|this, _, _, cx| {
                                            this.close_quick_command_editor(cx);
                                        }),
                                    ))
                                    .child(
                                        div().when(!can_save, |this| this.opacity(0.45)).child(
                                            small_button(palette, 
                                                "quick-command-editor-save",
                                                "Save",
                                                cx.listener(|this, _, _, cx| {
                                                    this.save_quick_command_editor(cx);
                                                }),
                                            ),
                                        ),
                                    ),
                            ),
                    ),
            )
    }
}

fn quick_command_category_choice(
    id: String,
    label: String,
    selected: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id))
        .h(px(24.))
        .max_w(px(148.))
        .px_2()
        .rounded_sm()
        .border_1()
        .border_color(if selected {
            rgb(0x4ade80)
        } else {
            rgb(0x263142)
        })
        .bg(if selected {
            rgb(0x10251a)
        } else {
            rgb(0x101827)
        })
        .cursor_pointer()
        .flex()
        .items_center()
        .text_size(px(10.))
        .text_color(if selected {
            rgb(0xbbf7d0)
        } else {
            rgb(0xcbd5e1)
        })
        .hover(|style| style.border_color(rgb(0x93c5fd)).bg(rgb(0x17233a)))
        .child(label)
        .on_click(on_click)
}

fn quick_command_color_swatch(
    color_tag: Option<&'static str>,
    selected: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let id = format!(
        "quick-command-color-swatch-{}",
        color_tag.unwrap_or("default")
    );
    div()
        .id(SharedString::from(id))
        .size(px(22.))
        .rounded_full()
        .border_2()
        .border_color(if selected {
            rgb(0xe5edf7)
        } else {
            rgb(0x263142)
        })
        .bg(quick_command_color(color_tag))
        .cursor_pointer()
        .on_click(on_click)
        .hover(|style| style.border_color(rgb(0x93c5fd)))
}

fn quick_command_icon_option(
    icon_tag: &'static str,
    color_tag: Option<&str>,
    selected: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!(
            "quick-command-icon-option-{icon_tag}"
        )))
        .size(px(24.))
        .rounded_sm()
        .border_1()
        .border_color(if selected {
            rgb(0xe5edf7)
        } else {
            rgb(0x263142)
        })
        .bg(if selected {
            rgb(0x17233a)
        } else {
            rgb(0x101827)
        })
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .on_click(on_click)
        .hover(|style| style.border_color(rgb(0x93c5fd)))
        .child(quick_command_icon_mark(Some(icon_tag), color_tag))
}

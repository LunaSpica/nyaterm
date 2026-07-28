use gpui::{
    AnyElement, App, ClickEvent, Context, FontWeight, IntoElement, KeyDownEvent, SharedString,
    Window, div, prelude::*, px, rgb, rgba,
};
use nyaterm_core::truncate_preview;

use super::{
    quick_command_color, quick_command_editor_field, quick_command_editor_script_field,
    quick_command_icon_mark,
};
use crate::features::{NyaTermApp, QUICK_COMMAND_COLOR_OPTIONS, QUICK_COMMAND_ICON_OPTIONS};
use crate::models::{QuickCommandEditorField, QuickCommandEditorState};
use crate::widgets::{mode_button, small_button};

impl NyaTermApp {
    pub(in crate::features) fn quick_command_editor_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.quick_command_editor_surface(self.last_viewport_size.0, false, cx)
    }

    pub(in crate::features) fn quick_command_editor_window_view(
        &mut self,
        viewport_width: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.quick_command_editor_surface(viewport_width, true, cx)
    }

    fn quick_command_editor_surface(
        &mut self,
        viewport_width: f32,
        native_window: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.theme_palette();
        let editor = self
            .quick_command_state
            .editor
            .draft
            .clone()
            .unwrap_or_else(QuickCommandEditorState::blank);
        let title = self.quick_command_editor_title();
        let uncategorized_label = self.tr("quickCommands.uncategorized");
        let category_label_text = self.tr("quickCommands.category");
        let category_search_label = self.tr("quickCommands.searchOrCreateCategory");
        let description_label = self.tr("quickCommands.description");
        let description_placeholder = self.tr("quickCommands.descriptionPlaceholder");
        let label_name = self.tr("quickCommands.labelName");
        let label_placeholder = self.tr("quickCommands.labelPlaceholder");
        let color_tag_label = self.tr("quickCommands.colorTag");
        let pin_label = self.tr("quickCommands.pin");
        let execution_mode_label = self.tr("quickCommands.executionMode");
        let execute_label = self.tr("quickCommands.executeImmediately");
        let append_label = self.tr("quickCommands.appendOnly");
        let execute_hint = self.tr("quickCommands.executeHint");
        let append_hint = self.tr("quickCommands.appendHint");
        let command_script_label = self.tr("quickCommands.commandScript");
        let command_placeholder = self.tr("quickCommands.commandPlaceholder");
        let cancel_label = self.tr("common.cancel");
        let save_label = self.tr("common.save");
        let wide_fields = viewport_width >= 768.;
        // Built before the card, which reads `self` all the way down: creating
        // an input needs it mutably.
        let label_input = quick_command_editor_field(
            self,
            QuickCommandEditorField::Label,
            label_name,
            label_placeholder,
            editor.label.clone(),
            cx,
        );
        let description_input = quick_command_editor_field(
            self,
            QuickCommandEditorField::Description,
            description_label,
            description_placeholder,
            editor.description.clone(),
            cx,
        );
        let command_input = quick_command_editor_script_field(
            self,
            QuickCommandEditorField::Command,
            command_script_label,
            command_placeholder,
            editor.command.clone(),
            cx,
        );
        let category_label = editor
            .category_id
            .as_deref()
            .and_then(|id| {
                self.quick_command_categories
                    .iter()
                    .find(|category| category.id == id)
            })
            .map(|category| category.name.clone())
            .unwrap_or_else(|| uncategorized_label.to_string());
        let category_draft = editor.category_draft.trim().to_string();
        let category_query = category_draft.to_lowercase();
        let exact_category_match = self
            .quick_command_categories
            .iter()
            .any(|category| category.name.eq_ignore_ascii_case(&category_draft));
        let category_display = if category_draft.is_empty() {
            category_label.clone()
        } else if exact_category_match {
            category_draft.clone()
        } else {
            self.tr("quickCommands.createCategory")
                .replace("{{name}}", &category_draft)
        };
        let mut color_swatches = div().mt_2().flex().items_center().gap_2().flex_wrap();
        for option in QUICK_COMMAND_COLOR_OPTIONS {
            let selected = editor.color_tag.as_deref() == option && editor.icon_tag.is_none();
            color_swatches = color_swatches.child(quick_command_color_swatch(
                palette,
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
                palette,
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
                palette,
                "quick-command-editor-category-none".to_string(),
                uncategorized_label.to_string(),
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
                palette,
                format!("quick-command-editor-category-{}", category.id),
                truncate_preview(&category.name, 22),
                selected,
                cx.listener(move |this, _, _, cx| {
                    this.set_quick_command_editor_category(Some(category_id.clone()), cx);
                }),
            ));
        }
        if !category_draft.is_empty() && !exact_category_match {
            let label = self
                .tr("quickCommands.createCategory")
                .replace("{{name}}", &truncate_preview(&category_draft, 18));
            category_choices = category_choices.child(quick_command_category_choice(
                palette,
                "quick-command-editor-category-draft".to_string(),
                label,
                true,
                cx.listener(|this, _, _, cx| {
                    this.confirm_quick_command_editor_category_draft(cx);
                }),
            ));
        }
        let can_save = !editor.label.trim().is_empty() && !editor.command.trim().is_empty();
        let dialog_bg = if native_window {
            rgb(palette.bg)
        } else {
            self.shell_surface_color(palette.bg)
        };

        div()
            .id(SharedString::from("quick-command-editor-overlay"))
            .when(!native_window, |this| {
                this.absolute()
                    .top_0()
                    .bottom_0()
                    .left_0()
                    .right_0()
                    .bg(rgba(0x00000080))
                    .p_4()
            })
            .when(native_window, |this| this.size_full().bg(rgb(palette.bg)))
            .flex()
            .flex_col()
            .items_center()
            .when(!native_window, |this| this.justify_center())
            .when(native_window, |this| this.justify_start())
            .overflow_y_scroll()
            .track_focus(&self.quick_command_state.editor.focus)
            // No blanket focus grab: it kept the surface "focused" for the old
            // label-div fields, and would now steal focus back from whichever
            // box the pointer just landed on, since click follows mouse-down.
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_quick_command_editor_key_down(event, cx);
            }))
            .child(
                div()
                    .id(SharedString::from("quick-command-editor-dialog"))
                    .w_full()
                    .max_w(px(560.))
                    .when(!native_window, |this| {
                        this.rounded_md()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .shadow_lg()
                    })
                    .bg(dialog_bg)
                    .p_4()
                    .when(!native_window, |this| {
                        this.child(
                            div().flex().items_center().gap_3().child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(palette.text))
                                    .child(title),
                            ),
                        )
                    })
                    .when(editor.error.is_some(), |this| {
                        this.child(
                            div()
                                .mt_3()
                                .rounded_sm()
                                .border_1()
                                .border_color(rgb(palette.danger))
                                .bg(rgb(0x1f0b0b))
                                .p_2()
                                .text_xs()
                                .text_color(rgb(palette.danger))
                                .child(editor.error.clone().unwrap_or_default()),
                        )
                    })
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .when(wide_fields, |this| this.flex_row())
                            .when(!wide_fields, |this| this.flex_col())
                            .gap_3()
                            .child(div().min_w_0().flex_1().child(label_input))
                            .child(
                                div()
                                    .id("quick-command-editor-category")
                                    .min_w_0()
                                    .flex_1()
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(
                                        if editor.focused_field == QuickCommandEditorField::Category
                                        {
                                            rgb(0x4ade80)
                                        } else {
                                            rgb(palette.border)
                                        },
                                    )
                                    .bg(
                                        if editor.focused_field == QuickCommandEditorField::Category
                                        {
                                            rgb(0x0f1f18)
                                        } else {
                                            rgb(palette.input)
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
                                            .text_color(rgb(palette.text_muted))
                                            .child(category_label_text),
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
                                                    .text_color(rgb(palette.text))
                                                    .child(truncate_preview(&category_display, 26)),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(10.))
                                                    .text_color(rgb(palette.text_muted))
                                                    .child(category_search_label),
                                            ),
                                    )
                                    .child(category_choices),
                            ),
                    )
                    .child(div().mt_3().child(description_input))
                    .child(
                        div()
                            .mt_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(palette.input))
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
                                                    .text_color(rgb(palette.text_muted))
                                                    .child(color_tag_label),
                                            )
                                            .child(div().flex().items_center().gap_1().child(
                                                quick_command_icon_mark(
                                                    palette,
                                                    editor.icon_tag.as_deref(),
                                                    editor.color_tag.as_deref(),
                                                ),
                                            )),
                                    )
                                    .child(color_swatches)
                                    .child(icon_grid),
                            )
                            .child(
                                div()
                                    .pl_4()
                                    .border_l_1()
                                    .border_color(rgb(palette.border))
                                    .flex()
                                    .flex_col()
                                    .items_end()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(rgb(palette.text_muted))
                                            .child(pin_label),
                                    )
                                    .child(mode_button(
                                        "quick-command-editor-pinned",
                                        pin_label,
                                        editor.pinned,
                                        self.theme_palette(),
                                        cx.listener(|this, _, _, cx| {
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
                                    .text_color(rgb(palette.text_muted))
                                    .child(execution_mode_label),
                            )
                            .child(
                                div()
                                    .mt_2()
                                    .grid()
                                    .grid_cols(2)
                                    .gap_2()
                                    .child(mode_button(
                                        "quick-command-editor-execute",
                                        execute_label,
                                        editor.execution_mode != "append",
                                        self.theme_palette(),
                                        cx.listener(|this, _, _, cx| {
                                            this.set_quick_command_editor_execution_mode(
                                                "execute", cx,
                                            );
                                        }),
                                    ))
                                    .child(mode_button(
                                        "quick-command-editor-append",
                                        append_label,
                                        editor.execution_mode == "append",
                                        self.theme_palette(),
                                        cx.listener(|this, _, _, cx| {
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
                                    .text_color(rgb(palette.text_muted))
                                    .child(if editor.execution_mode == "append" {
                                        append_hint
                                    } else {
                                        execute_hint
                                    }),
                            ),
                    )
                    .child(div().mt_3().child(command_input))
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(small_button(
                                        palette,
                                        "quick-command-editor-cancel",
                                        cancel_label,
                                        cx.listener(|this, _, _, cx| {
                                            this.close_quick_command_editor(cx);
                                        }),
                                    ))
                                    .child(
                                        div()
                                            .when(can_save, |this| {
                                                this.child(small_button(
                                                    palette,
                                                    "quick-command-editor-save",
                                                    save_label,
                                                    cx.listener(|this, _, _, cx| {
                                                        this.save_quick_command_editor(cx);
                                                    }),
                                                ))
                                            })
                                            .when(!can_save, |this| {
                                                this.child(
                                                    div()
                                                        .id("quick-command-editor-save-disabled")
                                                        .h(px(28.))
                                                        .px_3()
                                                        .flex()
                                                        .items_center()
                                                        .rounded_sm()
                                                        .border_1()
                                                        .border_color(rgb(palette.border))
                                                        .bg(rgb(palette.surface_elevated))
                                                        .text_color(rgb(palette.text_dimmed))
                                                        .text_xs()
                                                        .child(save_label),
                                                )
                                            }),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }
}

fn quick_command_category_choice(
    palette: crate::theme::ThemePalette,
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
            rgb(palette.border)
        })
        .bg(if selected {
            rgb(0x10251a)
        } else {
            rgb(palette.input)
        })
        .cursor_pointer()
        .flex()
        .items_center()
        .text_size(px(10.))
        .text_color(if selected {
            rgb(palette.success)
        } else {
            rgb(palette.text)
        })
        .hover(|style| style.border_color(rgb(palette.link)).bg(rgb(palette.hover)))
        .child(label)
        .on_click(on_click)
}

fn quick_command_color_swatch(
    palette: crate::theme::ThemePalette,
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
            rgb(palette.text)
        } else {
            rgb(palette.border)
        })
        .bg(quick_command_color(palette, color_tag))
        .cursor_pointer()
        .on_click(on_click)
        .hover(|style| style.border_color(rgb(palette.link)))
}

fn quick_command_icon_option(
    palette: crate::theme::ThemePalette,
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
            rgb(palette.text)
        } else {
            rgb(palette.border)
        })
        .bg(if selected {
            rgb(palette.hover)
        } else {
            rgb(palette.input)
        })
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .on_click(on_click)
        .hover(|style| style.border_color(rgb(palette.link)))
        .child(quick_command_icon_mark(palette, Some(icon_tag), color_tag))
}

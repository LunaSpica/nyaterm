use gpui::{
    AnyElement, ClickEvent, Context, FontWeight, MouseButton, SharedString, div, prelude::*, px,
    rgb, rgba,
};
use nyaterm_core::{QuickCommand, truncate_preview};

use super::super::super::{quick_command_icon_mark, quick_command_pin_mark};
use super::super::helpers::quick_command_row_actions;
use crate::features::{ChromeTooltip, NyaTermApp};
use crate::models::{QuickCommandRowMenuState, QuickCommandViewMode};

impl NyaTermApp {
    pub(super) fn quick_command_items(
        &mut self,
        commands: &[QuickCommand],
        palette: crate::theme::ThemePalette,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let mut items = Vec::with_capacity(commands.len());
        for command in commands.iter().cloned() {
            let command_id = command.id.clone();
            let run_command_id = command.id.clone();
            let compact_click_command_id = command.id.clone();
            let list_header_command_id = command.id.clone();
            let detail_command_id = command.id.clone();
            let execution_mode = if command.execution_mode.as_deref() == Some("append") {
                "append"
            } else {
                "execute"
            };
            let menu_open = self
                .quick_command_state
                .list
                .row_menu
                .as_ref()
                .is_some_and(|menu| menu.command_id == command.id);
            let command_item = match self.quick_command_state.list.view_mode {
                QuickCommandViewMode::Tile => div()
                    .id(SharedString::from(format!(
                        "quick-command-tile-{command_id}"
                    )))
                    .relative()
                    .max_w(px(220.))
                    .h(px(26.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgba((palette.border << 8) | 0x59))
                    .bg(rgba((palette.surface_elevated << 8) | 0x33))
                    .px_2()
                    .flex()
                    .items_center()
                    .gap_1()
                    .cursor_pointer()
                    .hover(move |this| this.bg(rgba((palette.surface_elevated << 8) | 0x80)))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener({
                            let menu_command_id = command_id.clone();
                            move |this, event: &gpui::MouseDownEvent, _, cx| {
                                cx.stop_propagation();
                                this.quick_command_state.list.row_menu =
                                    Some(QuickCommandRowMenuState {
                                        command_id: menu_command_id.clone(),
                                        x: event.position.x,
                                        y: event.position.y,
                                    });
                                cx.notify();
                            }
                        }),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.quick_command_state.list.row_menu = None;
                        this.run_quick_command_by_id(run_command_id.clone(), cx);
                    }))
                    .child(quick_command_icon_mark(
                        palette,
                        command.icon_tag.as_deref(),
                        command.color_tag.as_deref(),
                    ))
                    .when(command.pinned.unwrap_or_default(), |this| {
                        this.child(quick_command_pin_mark(palette))
                    })
                    .child(
                        div()
                            .min_w_0()
                            .text_size(px(11.))
                            .font_weight(FontWeight(500.))
                            .text_color(rgb(palette.text))
                            .overflow_hidden()
                            .child(truncate_preview(&command.label, 28)),
                    )
                    .tooltip({
                        let label = command.label.clone();
                        let command_text = command.command.clone();
                        move |_, cx| {
                            cx.new(|_| {
                                ChromeTooltip::new(format!(
                                    "{}\n{}",
                                    label,
                                    truncate_preview(&command_text, 120)
                                ))
                            })
                            .into()
                        }
                    })
                    .into_any_element(),
                QuickCommandViewMode::Compact => {
                    // Tauri compact: send + details + more (edit / send-all / delete).
                    let actions = quick_command_row_actions(
                        palette,
                        &command_id,
                        false,
                        execution_mode,
                        menu_open,
                        cx.listener(move |this, _, _, cx| {
                            this.quick_command_state.list.row_menu = None;
                            this.run_quick_command_by_id(run_command_id.clone(), cx);
                        }),
                        cx.listener(move |this, event: &ClickEvent, window, cx| {
                            this.quick_command_state.list.row_menu = None;
                            let position = event.position();
                            this.open_quick_command_details(
                                detail_command_id.clone(),
                                position.x,
                                position.y,
                                window,
                                cx,
                            );
                        }),
                        cx.listener({
                            let menu_command_id = command_id.clone();
                            move |this, event: &ClickEvent, _, cx| {
                                cx.stop_propagation();
                                if this
                                    .quick_command_state
                                    .list
                                    .row_menu
                                    .as_ref()
                                    .is_some_and(|menu| menu.command_id == menu_command_id)
                                {
                                    this.quick_command_state.list.row_menu = None;
                                } else {
                                    let position = event.position();
                                    this.quick_command_state.list.row_menu =
                                        Some(QuickCommandRowMenuState {
                                            command_id: menu_command_id.clone(),
                                            x: position.x,
                                            y: position.y,
                                        });
                                }
                                cx.notify();
                            }
                        }),
                    );

                    div()
                        .id(SharedString::from(format!(
                            "quick-command-compact-{command_id}"
                        )))
                        .relative()
                        .h(px(32.))
                        .w_full()
                        .rounded_sm()
                        .px_1()
                        .flex()
                        .items_center()
                        .gap_1()
                        .hover(move |this| this.bg(rgba((palette.surface_elevated << 8) | 0x73)))
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener({
                                let menu_command_id = command_id.clone();
                                move |this, event: &gpui::MouseDownEvent, _, cx| {
                                    cx.stop_propagation();
                                    this.quick_command_state.list.row_menu =
                                        Some(QuickCommandRowMenuState {
                                            command_id: menu_command_id.clone(),
                                            x: event.position.x,
                                            y: event.position.y,
                                        });
                                    cx.notify();
                                }
                            }),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!(
                                    "quick-command-compact-run-area-{command_id}"
                                )))
                                .min_w_0()
                                .flex_1()
                                .h_full()
                                .flex()
                                .items_center()
                                .gap_1()
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.quick_command_state.list.row_menu = None;
                                    this.run_quick_command_by_id(
                                        compact_click_command_id.clone(),
                                        cx,
                                    );
                                }))
                                .child(quick_command_icon_mark(
                                    palette,
                                    command.icon_tag.as_deref(),
                                    command.color_tag.as_deref(),
                                ))
                                .when(command.pinned.unwrap_or_default(), |this| {
                                    this.child(quick_command_pin_mark(palette))
                                })
                                .child(
                                    div()
                                        .min_w(px(64.))
                                        .max_w(px(140.))
                                        .text_size(px(11.))
                                        .font_weight(FontWeight(500.))
                                        .text_color(rgb(palette.text))
                                        .overflow_hidden()
                                        .child(truncate_preview(&command.label, 28)),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .font_family(crate::features::gpui_code_font_family())
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text_muted))
                                        .overflow_hidden()
                                        .child(truncate_preview(&command.command, 96)),
                                ),
                        )
                        .child(actions)
                        .into_any_element()
                }
                QuickCommandViewMode::List => {
                    // Tauri list: badge + send + details + more.
                    let actions = quick_command_row_actions(
                        palette,
                        &command_id,
                        true,
                        execution_mode,
                        menu_open,
                        cx.listener(move |this, _, _, cx| {
                            this.quick_command_state.list.row_menu = None;
                            this.run_quick_command_by_id(run_command_id.clone(), cx);
                        }),
                        cx.listener(move |this, event: &ClickEvent, window, cx| {
                            this.quick_command_state.list.row_menu = None;
                            let position = event.position();
                            this.open_quick_command_details(
                                detail_command_id.clone(),
                                position.x,
                                position.y,
                                window,
                                cx,
                            );
                        }),
                        cx.listener({
                            let menu_command_id = command_id.clone();
                            move |this, event: &ClickEvent, _, cx| {
                                cx.stop_propagation();
                                if this
                                    .quick_command_state
                                    .list
                                    .row_menu
                                    .as_ref()
                                    .is_some_and(|menu| menu.command_id == menu_command_id)
                                {
                                    this.quick_command_state.list.row_menu = None;
                                } else {
                                    let position = event.position();
                                    this.quick_command_state.list.row_menu =
                                        Some(QuickCommandRowMenuState {
                                            command_id: menu_command_id.clone(),
                                            x: position.x,
                                            y: position.y,
                                        });
                                }
                                cx.notify();
                            }
                        }),
                    );

                    div()
                        .id(SharedString::from(format!(
                            "quick-command-list-{command_id}"
                        )))
                        .relative()
                        .min_h(px(44.))
                        .w_full()
                        .rounded_md()
                        .border_1()
                        .border_color(rgba((palette.border << 8) | 0x59))
                        .bg(rgba((palette.surface_elevated << 8) | 0x26))
                        .px_2()
                        .py_1()
                        .flex()
                        .items_center()
                        .gap_2()
                        .hover(move |this| this.bg(rgba((palette.surface_elevated << 8) | 0x73)))
                        .on_mouse_down(
                            MouseButton::Right,
                            cx.listener({
                                let menu_command_id = command_id.clone();
                                move |this, event: &gpui::MouseDownEvent, _, cx| {
                                    cx.stop_propagation();
                                    this.quick_command_state.list.row_menu =
                                        Some(QuickCommandRowMenuState {
                                            command_id: menu_command_id.clone(),
                                            x: event.position.x,
                                            y: event.position.y,
                                        });
                                    cx.notify();
                                }
                            }),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!(
                                    "quick-command-list-run-head-{command_id}"
                                )))
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .items_center()
                                .gap_2()
                                .cursor_pointer()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.quick_command_state.list.row_menu = None;
                                    this.run_quick_command_by_id(
                                        list_header_command_id.clone(),
                                        cx,
                                    );
                                }))
                                .child(quick_command_icon_mark(
                                    palette,
                                    command.icon_tag.as_deref(),
                                    command.color_tag.as_deref(),
                                ))
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .flex()
                                        .flex_col()
                                        .gap(px(2.))
                                        .child(
                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .min_w_0()
                                                        .flex_1()
                                                        .text_xs()
                                                        .font_weight(FontWeight(500.))
                                                        .text_color(rgb(palette.text))
                                                        .overflow_hidden()
                                                        .child(truncate_preview(
                                                            &command.label,
                                                            40,
                                                        )),
                                                )
                                                .when(command.pinned.unwrap_or_default(), |this| {
                                                    this.child(quick_command_pin_mark(palette))
                                                }),
                                        )
                                        .child(
                                            div()
                                                .min_w_0()
                                                .font_family(
                                                    crate::features::gpui_code_font_family(),
                                                )
                                                .text_size(px(11.))
                                                .text_color(rgb(palette.text_muted))
                                                .overflow_hidden()
                                                .child(truncate_preview(&command.command, 120)),
                                        ),
                                ),
                        )
                        .child(actions)
                        .into_any_element()
                }
            };

            items.push(command_item);
        }
        items
    }
}

pub(super) fn quick_command_tile_column_count(
    viewport_width: f32,
    left_panel_width: f32,
    right_panel_width: f32,
    left_panel_visible: bool,
    right_panel_visible: bool,
) -> usize {
    const ACTIVITY_BARS_WIDTH: f32 = 80.;
    const CATEGORY_AND_PADDING_WIDTH: f32 = 188.;
    const TILE_TARGET_WIDTH: f32 = 190.;

    let side_panels_width = if left_panel_visible {
        left_panel_width
    } else {
        0.
    } + if right_panel_visible {
        right_panel_width
    } else {
        0.
    };
    let available_width =
        viewport_width - ACTIVITY_BARS_WIDTH - CATEGORY_AND_PADDING_WIDTH - side_panels_width;
    ((available_width / TILE_TARGET_WIDTH).floor() as usize).clamp(1, 6)
}

#[cfg(test)]
mod tests {
    use super::quick_command_tile_column_count;

    #[test]
    fn tile_columns_follow_the_available_workspace_width() {
        assert_eq!(
            quick_command_tile_column_count(1280., 288., 300., true, true),
            2
        );
        assert_eq!(
            quick_command_tile_column_count(1920., 288., 300., true, true),
            5
        );
        assert_eq!(
            quick_command_tile_column_count(800., 288., 300., true, true),
            1
        );
        assert_eq!(
            quick_command_tile_column_count(1280., 288., 300., false, false),
            5
        );
    }
}

use super::state::SendCommandBarViewState;
use super::*;

impl NyaTermApp {
    pub(super) fn send_command_bar_editor(
        &mut self,
        state: &SendCommandBarViewState,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let palette = state.palette;
        let input_hint = state.input_hint;
        let validation_error = state.validation_error;
        let preview = state.preview.clone();
        div()
            .flex_1()
            .min_h(px(72.))
            .flex()
            .gap_1()
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h(px(72.))
                    .when(
                        self.send_command_data_type == SendCommandDataType::Hex,
                        |this| {
                            // Tauri overlays dashed 4-byte guides per line above the hex textarea.
                            // Scroll-sync: wheel adjusts hexScroll.{top,left} approximation.
                            let guide_rows = send_command_hex_guide_rows(&self.send_command_draft);
                            const HEX_LINE_PX: f32 = 15.;
                            const HEX_CHAR_PX: f32 = 7.2;
                            const VIEWPORT_LINES: f32 = 5.;
                            const VIEWPORT_CHARS: f32 = 48.;
                            let display = format_send_command_hex_display(&self.send_command_draft);
                            let lines: Vec<&str> = display.lines().collect();
                            let line_count = lines.len().max(1) as f32;
                            let max_line_chars = lines
                                .iter()
                                .map(|line| line.chars().count())
                                .max()
                                .unwrap_or(0)
                                as f32;
                            let max_scroll_y =
                                ((line_count - VIEWPORT_LINES).max(0.)) * HEX_LINE_PX;
                            let max_scroll_x =
                                ((max_line_chars - VIEWPORT_CHARS).max(0.)) * HEX_CHAR_PX;
                            let scroll_y = self.send_command_hex_scroll_y.clamp(0., max_scroll_y);
                            let scroll_x = self.send_command_hex_scroll_x.clamp(0., max_scroll_x);
                            this.on_scroll_wheel(cx.listener(
                                move |this, event: &ScrollWheelEvent, _, cx| {
                                    let (delta_x, delta_y) = match event.delta {
                                        ScrollDelta::Lines(delta) => {
                                            (delta.x * HEX_CHAR_PX * 4., delta.y * HEX_LINE_PX)
                                        }
                                        ScrollDelta::Pixels(delta) => {
                                            (f32::from(delta.x), f32::from(delta.y))
                                        }
                                    };
                                    // Match GPUI / DOM: scroll offsets move opposite wheel delta.
                                    let next_y = (this.send_command_hex_scroll_y - delta_y)
                                        .clamp(0., max_scroll_y);
                                    let next_x = (this.send_command_hex_scroll_x - delta_x)
                                        .clamp(0., max_scroll_x);
                                    let changed = (next_y - this.send_command_hex_scroll_y).abs()
                                        > 0.01
                                        || (next_x - this.send_command_hex_scroll_x).abs() > 0.01;
                                    if changed {
                                        this.send_command_hex_scroll_y = next_y;
                                        this.send_command_hex_scroll_x = next_x;
                                        cx.stop_propagation();
                                        cx.notify();
                                    }
                                },
                            ))
                            .child(
                                div()
                                    .absolute()
                                    .top_0()
                                    .left_0()
                                    .right_0()
                                    .h(px(22.))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .border_b_1()
                                    .border_color(rgb(palette.surface_elevated))
                                    .bg(rgb(palette.bg))
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .font_weight(FontWeight(600.))
                                            .text_color(rgb(palette.text_dimmed))
                                            .child("HEX Editor"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(if validation_error {
                                                rgb(palette.danger)
                                            } else {
                                                rgb(palette.text_dimmed)
                                            })
                                            .child(if validation_error {
                                                "Invalid hex"
                                            } else {
                                                ""
                                            }),
                                    ),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .inset_0()
                                    .top(px(22.))
                                    .px_2()
                                    .py_1()
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .relative()
                                            .top(px(-scroll_y))
                                            .left(px(-scroll_x))
                                            .flex()
                                            .flex_col()
                                            .children(guide_rows.into_iter().map(|marks| {
                                                div()
                                                    .h(px(HEX_LINE_PX))
                                                    .relative()
                                                    .w_full()
                                                    .flex_none()
                                                    .children(marks.into_iter().map(|mark| {
                                                        div()
                                                            .absolute()
                                                            .top(px(0.))
                                                            .left(px(mark as f32 * 7.2))
                                                            .h(px(HEX_LINE_PX + 2.))
                                                            .w(px(2.))
                                                            .bg(rgb(0x1f6feb))
                                                            .opacity(0.55)
                                                    }))
                                            })),
                                    ),
                            )
                        },
                    )
                    .child(
                        transfer_input(
                            "bottom-command-send-input",
                            input_hint,
                            if self.send_command_data_type == SendCommandDataType::Hex {
                                format_send_command_hex_display(&self.send_command_draft)
                            } else {
                                self.send_command_draft.clone()
                            },
                            true,
                            self.theme_palette(),
                        )
                        .flex_1()
                        .min_h(px(72.))
                        .when(
                            self.send_command_data_type == SendCommandDataType::Hex,
                            |this| this.pt(px(22.)),
                        )
                        .font_family(crate::features::gpui_code_font_family())
                        .track_focus(&self.send_command_focus)
                        .on_click(cx.listener(|this, _, window, cx| {
                            window.focus(&this.send_command_focus);
                            cx.notify();
                        }))
                        .on_key_down(cx.listener(
                            |this, event: &KeyDownEvent, _, cx| {
                                cx.stop_propagation();
                                this.handle_send_command_key_down(event, cx);
                            },
                        )),
                    ),
            )
            .when(
                self.send_command_data_type == SendCommandDataType::Hex,
                |this| {
                    let byte_count = send_command_hex_byte_count(&self.send_command_draft);
                    let guide_count = send_command_hex_guide_count(&self.send_command_draft);
                    this.child(
                        div()
                            .w(px(180.))
                            .flex_none()
                            .min_h(px(72.))
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(palette.bg))
                            .px_2()
                            .py_1()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .font_weight(FontWeight(600.))
                                            .text_color(rgb(palette.text_dimmed))
                                            .child("Preview"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(rgb(palette.text_dimmed))
                                            .child(match byte_count {
                                                Some(n) => format!("{n} B"),
                                                None => "invalid".to_string(),
                                            }),
                                    ),
                            )
                            .when(guide_count > 0, |this| {
                                this.child(
                                    div()
                                        .text_size(px(10.))
                                        .text_color(rgb(0x388bfd))
                                        .child(format!("guides ×{guide_count} (4-byte)")),
                                )
                            })
                            .child(
                                div()
                                    .flex_1()
                                    .font_family(crate::features::gpui_code_font_family())
                                    .text_size(px(11.))
                                    .line_height(px(15.))
                                    .text_color(if validation_error {
                                        rgb(palette.danger)
                                    } else {
                                        rgb(palette.text)
                                    })
                                    .child(if preview.trim().is_empty() {
                                        "·".to_string()
                                    } else {
                                        preview.clone()
                                    }),
                            ),
                    )
                },
            )
            .into_any_element()
    }
}

use gpui::{Context, FontWeight, KeyDownEvent, div, prelude::*, px, rgb, rgba};

use crate::features::NyaTermApp;
use crate::features::view_widgets::dialog_action_button;
use crate::widgets::small_button;

impl NyaTermApp {
    pub(in crate::features) fn close_all_sessions_confirm_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let palette = self.theme_palette();
        let title_key = if self.session.dialogs.should_quit_after_close_all() {
            "dialog.confirmClose"
        } else {
            "tabCtx.closeAll"
        };
        let description_key = if self.session.dialogs.should_quit_after_close_all() {
            "dialog.confirmCloseDesc"
        } else {
            "tabCtx.closeAllConfirm"
        };
        let action_key = if self.session.dialogs.should_quit_after_close_all() {
            "dialog.confirmCloseAction"
        } else {
            "tabCtx.closeAll"
        };

        div()
            .id("close-all-sessions-confirm-overlay")
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .p_3()
            .track_focus(self.session.dialogs.close_all_sessions_confirm_focus())
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(this.session.dialogs.close_all_sessions_confirm_focus());
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                match event.keystroke.key.as_str() {
                    "escape" => this.cancel_close_all_sessions_confirm(cx),
                    "enter" => this.confirm_close_all_sessions(window, cx),
                    _ => {}
                }
            }))
            .child(
                div()
                    .id("close-all-sessions-confirm-dialog")
                    .w(px((self.shell.viewport.size.0 - 32.).clamp(280., 400.)))
                    .max_w_full()
                    .mx_4()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .p_6()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(palette.text))
                                    .child(self.tr(title_key)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .line_height(px(16.))
                                    .text_color(rgb(palette.text_muted))
                                    .child(self.tr(description_key)),
                            ),
                    )
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "close-all-sessions-cancel",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_close_all_sessions_confirm(cx);
                                }),
                            ))
                            .child(dialog_action_button(
                                palette,
                                "close-all-sessions-confirm",
                                self.tr(action_key),
                                true,
                                cx.listener(|this, _, window, cx| {
                                    this.confirm_close_all_sessions(window, cx);
                                }),
                            )),
                    ),
            )
            .into_any_element()
    }
}

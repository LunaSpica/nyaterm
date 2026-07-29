use gpui::{Context, FontWeight, IntoElement, SharedString, div, prelude::*, px, rgb, rgba};

use crate::features::NyaTermApp;
use crate::features::view_widgets::dialog_action_button;
use crate::models::QuickCommandDeleteState;
use crate::widgets::small_button;

impl NyaTermApp {
    pub(in crate::features) fn quick_command_delete_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let delete = self
            .commands
            .quick_delete()
            .cloned()
            .unwrap_or(QuickCommandDeleteState {
                id: String::new(),
                label: "command".to_string(),
            });

        div()
            .id(SharedString::from("quick-command-delete-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .id(SharedString::from("quick-command-delete-dialog"))
                    .w(px((self.shell.viewport_size().0 - 32.).clamp(280., 384.)))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x7f1d1d))
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .p_6()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("quickCommands.delete")),
                    )
                    .child(
                        div()
                            .mt_3()
                            .text_xs()
                            .line_height(px(18.))
                            .text_color(rgb(palette.text_muted))
                            .child(
                                self.tr("quickCommands.deleteConfirm")
                                    .replace("{{name}}", &delete.label),
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
                                "quick-command-delete-cancel",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.cancel_delete_quick_command(cx);
                                }),
                            ))
                            .child(dialog_action_button(
                                palette,
                                "quick-command-delete-confirm",
                                self.tr("common.delete"),
                                true,
                                cx.listener(|this, _, _, cx| {
                                    this.confirm_delete_quick_command(cx);
                                }),
                            )),
                    ),
            )
    }
}

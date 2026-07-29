use gpui::{
    Context, FontWeight, IntoElement, KeyDownEvent, SharedString, div, prelude::*, px, rgb, rgba,
};
use nyaterm_core::truncate_preview;

use crate::features::view_widgets::dialog_action_button;
use crate::features::{NyaTermApp, TAB_PRESET_COLORS, TextInputSetup};
use crate::models::StartupCommandAction;
use crate::widgets::{session_info_row, small_button};

impl NyaTermApp {
    pub(in crate::features) fn rename_session_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let rename_draft = self.session.dialogs.rename_draft().to_string();
        let rename_input = self
            .text_input_box(
                "session.rename",
                &rename_draft,
                TextInputSetup::placeholder(self.tr("tabCtx.renamePlaceholder")),
                cx,
            )
            .into_any_element();
        let can_save = !self.session.dialogs.rename_draft().trim().is_empty();
        let dialog_width = (self.shell.viewport.size.0 - 32.).clamp(280., 320.);

        div()
            .id(SharedString::from("rename-tab-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(self.session.dialogs.rename_focus())
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(this.session.dialogs.rename_focus());
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                this.handle_rename_key_down(event, cx);
            }))
            .child(
                div()
                    .id(SharedString::from("rename-tab-dialog"))
                    .w(px(dialog_width))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .p_6()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("tabCtx.renameTitle")),
                    )
                    .child(
                        div()
                            .mt_3()
                            .font_family(crate::features::gpui_code_font_family())
                            .text_sm()
                            .child(rename_input),
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
                                "rename-tab-cancel",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.close_rename_session(cx);
                                }),
                            ))
                            .child(div().when(!can_save, |this| this.opacity(0.45)).child(
                                dialog_action_button(
                                    palette,
                                    "rename-tab-save",
                                    self.tr("common.save"),
                                    false,
                                    cx.listener(|this, _, _, cx| {
                                        this.submit_rename_session(cx);
                                    }),
                                ),
                            )),
                    ),
            )
    }

    pub(in crate::features) fn tab_color_picker_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let active_color = self
            .session
            .active_id
            .as_deref()
            .and_then(|session_id| self.session.tab_color(session_id));
        let mut swatches = div().mt_3().grid().grid_cols(6).gap_2();
        for (name, color) in TAB_PRESET_COLORS {
            let selected = active_color == Some(color);
            swatches = swatches.child(
                div()
                    .id(SharedString::from(format!("tab-color-{name}")))
                    .size(px(28.))
                    .rounded_full()
                    .border_2()
                    .border_color(if selected {
                        rgb(0xffffff)
                    } else {
                        rgb(0x1f2937)
                    })
                    .bg(rgb(color))
                    .cursor_pointer()
                    .hover(|this| this.border_color(rgb(palette.text)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_active_session_tab_color(Some(color), cx);
                    })),
            );
        }

        div()
            .id(SharedString::from("tab-color-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(self.session.dialogs.color_picker_focus())
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(this.session.dialogs.color_picker_focus());
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                if event.keystroke.key == "escape" {
                    this.close_tab_color_picker(cx);
                }
            }))
            .child(
                div()
                    .id(SharedString::from("tab-color-dialog"))
                    .w(px(300.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(self.tr("tabCtx.setColor")),
                    )
                    .child(swatches)
                    .child(
                        div()
                            .mt_3()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(self.tr("tabCtx.colorHint")),
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
                                "tab-color-reset",
                                self.tr("tabCtx.resetColor"),
                                cx.listener(|this, _, _, cx| {
                                    this.set_active_session_tab_color(None, cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "tab-color-cancel",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.close_tab_color_picker(cx);
                                }),
                            )),
                    ),
            )
    }

    pub(in crate::features) fn session_info_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let details = self.active_session_info_details().unwrap_or_default();
        let title = details
            .iter()
            .find(|(label, _)| *label == self.tr("sessionInfo.name"))
            .map(|(_, value)| value.clone())
            .unwrap_or_else(|| self.tr("tabCtx.sessionInfo").to_string());
        let mut rows = div().mt_4().flex().flex_col().gap_2();
        if details.is_empty() {
            rows = rows.child(
                div()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.input))
                    .p_3()
                    .text_sm()
                    .text_color(rgb(palette.text_muted))
                    .child(self.tr("tabCtx.noSessionDetails")),
            );
        } else {
            for (label, value) in details {
                rows = rows.child(session_info_row(palette, label, value));
            }
        }

        div()
            .id(SharedString::from("session-info-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(self.session.dialogs.session_info_focus())
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(this.session.dialogs.session_info_focus());
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                if event.keystroke.key == "escape" {
                    this.close_active_session_info(cx);
                }
            }))
            .child(
                div()
                    .id(SharedString::from("session-info-dialog"))
                    .w(px(520.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .gap_3()
                            .child(
                                div()
                                    .size(px(10.))
                                    .mt_1()
                                    .rounded_full()
                                    .bg(rgb(palette.success)),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight(800.))
                                            .text_color(rgb(palette.text))
                                            .child(self.tr("tabCtx.sessionInfo")),
                                    )
                                    .child(
                                        div()
                                            .mt_1()
                                            .text_xs()
                                            .text_color(rgb(palette.text_muted))
                                            .child(truncate_preview(&title, 56)),
                                    ),
                            ),
                    )
                    .child(rows)
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "session-info-copy",
                                self.tr("common.copyToClipboard"),
                                cx.listener(|this, _, _, cx| {
                                    this.copy_active_session_info(cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "session-info-close",
                                self.tr("common.close"),
                                cx.listener(|this, _, _, cx| {
                                    this.close_active_session_info(cx);
                                }),
                            )),
                    ),
            )
    }

    pub(in crate::features) fn startup_command_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let action = self.session.dialogs.startup_command_action();
        let action_title = self.tr(match action {
            StartupCommandAction::Duplicate => "tabCtx.runCommandTitle",
            StartupCommandAction::Multiplex => "tabCtx.multiplexSshWithCommand",
        });
        let startup_command_draft = self.session.dialogs.startup_command_draft().to_string();
        let command_input = self
            .text_input_box(
                "session.startup-command",
                &startup_command_draft,
                TextInputSetup::placeholder(self.tr("tabCtx.commandRequired")),
                cx,
            )
            .into_any_element();
        let can_submit = !self
            .session
            .dialogs
            .startup_command_draft()
            .trim()
            .is_empty();
        let delay_label = format!("{} ms", self.session.dialogs.startup_command_delay_ms());
        let dialog_width = (self.shell.viewport.size.0 - 32.).clamp(280., 448.);

        div()
            .id(SharedString::from("startup-command-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(self.session.dialogs.startup_command_focus())
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(this.session.dialogs.startup_command_focus());
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                this.handle_startup_command_key_down(event, window, cx);
            }))
            .child(
                div()
                    .id(SharedString::from("startup-command-dialog"))
                    .w(px(dialog_width))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .p_6()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(action_title),
                    )
                    .child(
                        div()
                            .mt_3()
                            .text_xs()
                            .font_weight(FontWeight(600.))
                            .text_color(rgb(palette.text_muted))
                            .child(self.tr("tabCtx.commandInput")),
                    )
                    .child(
                        div()
                            .mt_1()
                            .font_family(crate::features::gpui_code_font_family())
                            .text_sm()
                            .child(command_input),
                    )
                    .child(
                        div()
                            .mt_3()
                            .text_xs()
                            .font_weight(FontWeight(600.))
                            .text_color(rgb(palette.text_muted))
                            .child(self.tr("tabCtx.commandDelay")),
                    )
                    .child(
                        div()
                            .mt_1()
                            .h(px(36.))
                            .px_3()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(palette.input))
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .font_family(crate::features::gpui_code_font_family())
                                    .text_sm()
                                    .text_color(rgb(palette.text))
                                    .child(delay_label),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(small_button(
                                        palette,
                                        "startup-delay-minus",
                                        "-100",
                                        cx.listener(|this, _, _, cx| {
                                            this.adjust_startup_command_delay(-100, cx);
                                        }),
                                    ))
                                    .child(small_button(
                                        palette,
                                        "startup-delay-zero",
                                        "0",
                                        cx.listener(|this, _, _, cx| {
                                            this.session.dialogs.reset_startup_command_delay();
                                            cx.notify();
                                        }),
                                    ))
                                    .child(small_button(
                                        palette,
                                        "startup-delay-plus",
                                        "+100",
                                        cx.listener(|this, _, _, cx| {
                                            this.adjust_startup_command_delay(100, cx);
                                        }),
                                    )),
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
                                "startup-command-cancel",
                                self.tr("common.cancel"),
                                cx.listener(|this, _, _, cx| {
                                    this.close_startup_command_dialog(cx);
                                }),
                            ))
                            .child(div().when(!can_submit, |this| this.opacity(0.45)).child(
                                dialog_action_button(
                                    palette,
                                    "startup-command-submit",
                                    self.tr("common.confirm"),
                                    false,
                                    cx.listener(|this, _, window, cx| {
                                        this.submit_startup_command_dialog(window, cx);
                                    }),
                                ),
                            )),
                    ),
            )
    }
}

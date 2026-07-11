use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn quick_command_import_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        div()
            .id(SharedString::from("quick-command-import-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.quick_command_import_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.quick_command_import_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                if event.keystroke.key == "escape" {
                    this.close_quick_command_import_dialog(cx);
                }
            }))
            .child(
                div()
                    .id(SharedString::from("quick-command-import-dialog"))
                    .w(px(420.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x303848))
                    .bg(rgb(0x0b0f16))
                    .shadow_lg()
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .min_w_0()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight(800.))
                                            .text_color(rgb(0xe5edf7))
                                            .child("Import Quick Commands"),
                                    )
                                    .child(
                                        div()
                                            .mt_1()
                                            .text_xs()
                                            .text_color(rgb(0x98a3b8))
                                            .child("WindTerm, Xshell, or NyaTerm JSON"),
                                    ),
                            )
                            .child(small_button(palette, 
                                "quick-command-import-close-top",
                                "Close",
                                cx.listener(|this, _, _, cx| {
                                    this.close_quick_command_import_dialog(cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .mt_4()
                            .grid()
                            .grid_cols(2)
                            .gap_3()
                            .child(quick_command_import_source_card(
                                "quick-command-import-windterm-card",
                                "WT",
                                "WindTerm Quickbar",
                                "quickbar.config",
                                0x60a5fa,
                                cx.listener(|this, _, _, cx| {
                                    this.select_quick_command_import_source(
                                        QuickCommandImportPathPromptKind::WindTermQuickbar,
                                        cx,
                                    );
                                }),
                            ))
                            .child(quick_command_import_source_card(
                                "quick-command-import-xshell-card",
                                "XS",
                                "Xshell XTS",
                                ".xts",
                                0xfacc15,
                                cx.listener(|this, _, _, cx| {
                                    this.select_quick_command_import_source(
                                        QuickCommandImportPathPromptKind::XshellXts,
                                        cx,
                                    );
                                }),
                            ))
                            .child(quick_command_import_source_card(
                                "quick-command-import-json-card",
                                "{}",
                                "NyaTerm JSON",
                                ".json",
                                0x6ee7b7,
                                cx.listener(|this, _, _, cx| {
                                    this.select_quick_command_import_source(
                                        QuickCommandImportPathPromptKind::NyatermJson,
                                        cx,
                                    );
                                }),
                            )),
                    )
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .line_height(px(16.))
                                    .text_color(rgb(0x98a3b8))
                                    .child("Imports merge with existing commands and update matching IDs."),
                            )
                            .child(small_button(palette, 
                                "quick-command-import-close",
                                "Cancel",
                                cx.listener(|this, _, _, cx| {
                                    this.close_quick_command_import_dialog(cx);
                                }),
                            )),
                    ),
            )
    }
}

fn quick_command_import_source_card(
    id: &'static str,
    monogram: &'static str,
    label: &'static str,
    hint: &'static str,
    accent: u32,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id))
        .min_h(px(124.))
        .rounded_md()
        .border_1()
        .border_color(rgb(0x263142))
        .bg(rgb(0x0d1320))
        .p_3()
        .cursor_pointer()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .on_click(on_click)
        .hover(|style| style.border_color(rgb(accent)).bg(rgb(0x111a2a)))
        .child(
            div()
                .size(px(42.))
                .rounded_sm()
                .border_1()
                .border_color(rgb(accent))
                .bg(rgb(0x101827))
                .flex()
                .items_center()
                .justify_center()
                .font_family("JetBrains Mono")
                .text_sm()
                .font_weight(FontWeight(800.))
                .text_color(rgb(accent))
                .child(monogram),
        )
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight(700.))
                .text_color(rgb(0xe5edf7))
                .child(label),
        )
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(0x98a3b8))
                .child(hint),
        )
}

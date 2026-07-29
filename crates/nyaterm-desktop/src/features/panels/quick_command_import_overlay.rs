use gpui::{
    App, ClickEvent, Context, FontWeight, IntoElement, KeyDownEvent, SharedString, Window, div,
    prelude::*, px, rgb, rgba,
};

use crate::features::NyaTermApp;
use crate::models::QuickCommandImportPathPromptKind;
use crate::widgets::small_button;

impl NyaTermApp {
    pub(in crate::features) fn quick_command_import_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let docs_url = if self
            .settings
            .summary
            .language
            .to_ascii_lowercase()
            .starts_with("zh")
        {
            "https://nyaterm.app/docs/guide/quick-commands#%E5%AF%BC%E5%85%A5%E5%BF%AB%E6%8D%B7%E5%91%BD%E4%BB%A4"
        } else {
            "https://nyaterm.app/docs/guide/quick-commands#import-quick-commands"
        };
        div()
            .id(SharedString::from("quick-command-import-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(self.commands.quick_import_focus())
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(this.commands.quick_import_focus());
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
                    .w(px((self.shell.viewport.size.0 - 32.).clamp(280., 380.)))
                    .max_w_full()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .p_6()
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
                                            .text_color(rgb(palette.text))
                                            .child(self.tr("quickCommands.importTitle")),
                                    )
                                    .child(
                                        div()
                                            .mt_1()
                                            .text_xs()
                                            .text_color(rgb(palette.text_muted))
                                            .child(self.tr("quickCommands.importSelectSource")),
                                    ),
                            )
                            .child(small_button(
                                palette,
                                "quick-command-import-close-top",
                                self.tr("common.close"),
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
                                palette,
                                "quick-command-import-windterm-card",
                                "WT",
                                self.tr("quickCommands.importWindTerm"),
                                self.tr("quickCommands.importWindTermHint"),
                                0x60a5fa,
                                cx.listener(|this, _, _, cx| {
                                    this.select_quick_command_import_source(
                                        QuickCommandImportPathPromptKind::WindTermQuickbar,
                                        cx,
                                    );
                                }),
                            ))
                            .child(quick_command_import_source_card(
                                palette,
                                "quick-command-import-xshell-card",
                                "XS",
                                self.tr("quickCommands.importXshell"),
                                self.tr("quickCommands.importXshellHint"),
                                0xfacc15,
                                cx.listener(|this, _, _, cx| {
                                    this.select_quick_command_import_source(
                                        QuickCommandImportPathPromptKind::XshellXts,
                                        cx,
                                    );
                                }),
                            ))
                            .child(quick_command_import_source_card(
                                palette,
                                "quick-command-import-json-card",
                                "{}",
                                self.tr("quickCommands.importNyaTermJson"),
                                self.tr("quickCommands.importNyaTermJsonHint"),
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
                                    .text_color(rgb(palette.text_muted))
                                    .child(self.tr("quickCommands.importMergeHint")),
                            )
                            .child(small_button(
                                palette,
                                "quick-command-import-docs",
                                self.tr("quickCommands.importDocs"),
                                cx.listener(move |this, _, _, cx| {
                                    this.open_external_url_for_ui(docs_url, cx);
                                }),
                            )),
                    ),
            )
    }
}

fn quick_command_import_source_card(
    palette: crate::theme::ThemePalette,
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
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
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
                .bg(rgb(palette.input))
                .flex()
                .items_center()
                .justify_center()
                .font_family(crate::features::gpui_code_font_family())
                .text_sm()
                .font_weight(FontWeight(800.))
                .text_color(rgb(accent))
                .child(monogram),
        )
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight(700.))
                .text_color(rgb(palette.text))
                .child(label),
        )
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(palette.text_muted))
                .child(hint),
        )
}

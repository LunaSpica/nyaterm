use gpui::{
    App, ClickEvent, Context, FontWeight, IntoElement, KeyDownEvent, Window, div, prelude::*, px,
    rgb, rgba, svg,
};

use crate::features::{NyaTermApp, color_icon, modal_close_icon_button, mono_icon};
use crate::models::ConnectionImportSource;
use crate::theme::ThemePalette;

impl NyaTermApp {
    pub(in crate::features) fn connection_import_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let narrow = self.last_viewport_size.0 < 520.;
        let width = (self.last_viewport_size.0 - 32.).clamp(280., 480.);
        let docs_url = if self
            .settings
            .language
            .to_ascii_lowercase()
            .starts_with("zh")
        {
            "https://nyaterm.app/docs/guide/ssh-connection#%E5%AF%BC%E5%85%A5%E5%85%B6%E4%BB%96%E5%AE%A2%E6%88%B7%E7%AB%AF%E7%9A%84%E4%BC%9A%E8%AF%9D"
        } else {
            "https://nyaterm.app/docs/guide/ssh-connection#import-sessions-from-other-clients"
        };

        div()
            .id("connection-import-overlay")
            .absolute()
            .inset_0()
            .bg(rgba(0x00000080))
            .flex()
            .items_center()
            .justify_center()
            .p_3()
            .track_focus(&self.connection_state.import_focus_handle())
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_connection_import_dialog(cx);
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                if event.keystroke.key == "escape" {
                    this.close_connection_import_dialog(cx);
                }
            }))
            .child(
                div()
                    .id("connection-import-dialog")
                    .w(px(width))
                    .max_w_full()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.bg))
                    .shadow_lg()
                    .p_6()
                    .flex()
                    .flex_col()
                    .gap_4()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight(800.))
                                            .text_color(rgb(palette.text))
                                            .child(self.tr("settings.importConfig")),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(palette.text_muted))
                                            .child(self.tr("savedConnections.importSelectSource")),
                                    ),
                            )
                            .child(modal_close_icon_button(
                                palette,
                                "connection-import-close",
                                self.tr("common.close"),
                                cx.listener(|this, _, _, cx| {
                                    this.close_connection_import_dialog(cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .grid()
                            .grid_cols(if narrow { 2 } else { 3 })
                            .gap_3()
                            .child(connection_import_source_card(
                                palette,
                                "connection-import-nyaterm",
                                "icons/logo.svg",
                                "NyaTerm",
                                ".nya",
                                cx.listener(|this, _, _, cx| {
                                    this.select_connection_import_source(
                                        ConnectionImportSource::NyatermBackup,
                                        cx,
                                    );
                                }),
                            ))
                            .child(connection_import_source_card(
                                palette,
                                "connection-import-xshell",
                                "color/brand/xshell.png",
                                "Xshell",
                                ".xts",
                                cx.listener(|this, _, _, cx| {
                                    this.select_connection_import_source(
                                        ConnectionImportSource::Xshell,
                                        cx,
                                    );
                                }),
                            ))
                            .child(connection_import_source_card(
                                palette,
                                "connection-import-mobaxterm",
                                "color/brand/mobaxterm.png",
                                "MobaXterm",
                                ".mxtsessions",
                                cx.listener(|this, _, _, cx| {
                                    this.select_connection_import_source(
                                        ConnectionImportSource::MobaXterm,
                                        cx,
                                    );
                                }),
                            ))
                            .child(connection_import_source_card(
                                palette,
                                "connection-import-windterm",
                                "color/brand/windterm.png",
                                "WindTerm",
                                ".sessions",
                                cx.listener(|this, _, _, cx| {
                                    this.select_connection_import_source(
                                        ConnectionImportSource::WindTerm,
                                        cx,
                                    );
                                }),
                            ))
                            .child(connection_import_source_card(
                                palette,
                                "connection-import-json",
                                "icons/files.svg",
                                "JSON",
                                ".json",
                                cx.listener(|this, _, _, cx| {
                                    this.select_connection_import_source(
                                        ConnectionImportSource::NyatermJson,
                                        cx,
                                    );
                                }),
                            )),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        svg()
                                            .size(px(14.))
                                            .flex_none()
                                            .path("icons/conn/terminal.svg")
                                            .text_color(rgb(palette.text_muted)),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(11.))
                                            .line_height(px(16.))
                                            .text_color(rgb(palette.text_muted))
                                            .child(self.tr("savedConnections.importMergeHint")),
                                    ),
                            )
                            .child(
                                div()
                                    .id("connection-import-docs")
                                    .h(px(28.))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .rounded_sm()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette.link))
                                    .cursor_pointer()
                                    .hover(|this| this.bg(rgb(palette.hover)))
                                    .child(self.tr("savedConnections.importDocs"))
                                    .child(
                                        svg()
                                            .size(px(12.))
                                            .path("icons/menu/export.svg")
                                            .text_color(rgb(palette.link)),
                                    )
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.open_external_url_for_ui(docs_url, cx);
                                    })),
                            ),
                    ),
            )
    }
}

fn connection_import_source_card(
    palette: ThemePalette,
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    hint: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let hover = rgba((palette.primary << 8) | 0x14);
    div()
        .id(id)
        .min_h(px(128.))
        .min_w_0()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .p_3()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_2()
        .text_center()
        .cursor_pointer()
        .hover(move |this| this.border_color(rgb(palette.primary)).bg(hover))
        .on_click(on_click)
        .child(if icon.starts_with("color/") {
            // Vendor logos are full-color rasters; they cannot go through svg().
            color_icon(icon, 40.).into_any_element()
        } else {
            mono_icon(icon, rgb(palette.text).into(), 40.).into_any_element()
        })
        .child(
            div()
                .max_w_full()
                .text_xs()
                .font_weight(FontWeight(700.))
                .text_color(rgb(palette.text))
                .child(label),
        )
        .child(
            div()
                .max_w_full()
                .text_size(px(10.))
                .text_color(rgb(palette.text_dimmed))
                .child(hint),
        )
}

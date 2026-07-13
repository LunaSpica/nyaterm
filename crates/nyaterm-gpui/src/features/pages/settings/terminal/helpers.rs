use super::*;

pub(super) fn terminal_feature_card(
    palette: crate::theme::ThemePalette,
    title: &'static str,
    detail: &'static str,
    enabled: bool,
) -> impl IntoElement {
    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_3()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .text_xs()
                        .font_weight(FontWeight(800.))
                        .text_color(rgb(palette.text))
                        .child(title),
                )
                .child(status_pill(
                    if enabled { "on" } else { "off" },
                    if enabled {
                        rgb(palette.success)
                    } else {
                        rgb(palette.text_muted)
                    },
                    if enabled {
                        rgb(palette.hover)
                    } else {
                        rgb(palette.border)
                    },
                )),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(10.))
                .text_color(rgb(palette.text_muted))
                .line_height(px(14.))
                .child(detail),
        )
}

pub(super) fn search_engine_hint(
    palette: crate::theme::ThemePalette,
    title: &'static str,
    detail: &'static str,
) -> impl IntoElement {
    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_3()
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight(800.))
                .text_color(rgb(palette.text))
                .child(title),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(10.))
                .text_color(rgb(palette.text_muted))
                .line_height(px(14.))
                .child(detail),
        )
}

pub(super) fn settings_toggle_button(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(32.))
        .px_3()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .rounded_sm()
        .border_1()
        .border_color(if enabled {
            rgb(palette.success)
        } else {
            rgb(palette.border)
        })
        .bg(if enabled {
            rgb(palette.hover)
        } else {
            rgb(palette.surface)
        })
        .text_color(if enabled {
            rgb(palette.success)
        } else {
            rgb(palette.text)
        })
        .text_xs()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x223047)))
        .child(label)
        .child(status_pill(
            if enabled { "on" } else { "off" },
            if enabled {
                rgb(palette.success)
            } else {
                rgb(palette.text_muted)
            },
            if enabled {
                rgb(0x0d241c)
            } else {
                rgb(palette.border)
            },
        ))
        .on_click(on_click)
}

pub(super) fn search_engine_icon_label(icon: Option<&str>) -> String {
    match icon.unwrap_or("default") {
        "google" => "G".into(),
        "bing" => "B".into(),
        "duckduckgo" => "D".into(),
        "github" => "GH".into(),
        "gitlab" => "GL".into(),
        "baidu" => "Bd".into(),
        "yahoo" => "Y!".into(),
        "youtube" => "YT".into(),
        "bilibili" => "Bi".into(),
        "zhihu" => "Zh".into(),
        "openai" => "AI".into(),
        "claude" => "Cl".into(),
        "gemini" => "Ge".into(),
        _ => "?".into(),
    }
}

pub(super) fn search_engine_icon_color(icon: Option<&str>) -> u32 {
    match icon.unwrap_or("default") {
        "google" => 0x4285f4,
        "bing" => 0x008373,
        "duckduckgo" => 0xde5833,
        "github" => 0x8b949e,
        "gitlab" => 0xfc6d26,
        "baidu" => 0x2932e1,
        "yahoo" => 0x410093,
        "youtube" => 0xff0000,
        "bilibili" => 0x00a1d6,
        "zhihu" => 0x0084ff,
        "openai" => 0x10a37f,
        "claude" => 0xd97757,
        "gemini" => 0x4285f4,
        _ => 0x8b949e,
    }
}

pub(super) fn parse_keyword_swatch(value: &str) -> Option<u32> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if hex.len() != 6 {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

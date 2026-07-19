use super::*;

pub(super) fn security_editor_field(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    value: String,
    active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    transfer_input(id, label, value, active, palette)
        .h(px(42.))
        .on_click(on_click)
}

pub(super) fn security_type_chip(
    palette: crate::theme::ThemePalette,
    label: &'static str,
    selected: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("security-type-{label}")))
        .h(px(22.))
        .px_2()
        .flex()
        .items_center()
        .rounded_sm()
        .text_size(px(10.))
        .font_weight(FontWeight(700.))
        .cursor_pointer()
        .text_color(if selected {
            rgb(palette.success)
        } else {
            rgb(palette.text_muted)
        })
        .bg(if selected {
            rgb(0x12261a)
        } else {
            rgb(palette.surface_elevated)
        })
        .hover(|this| this.bg(rgb(palette.border)))
        .child(label)
        .on_click(on_click)
}

pub(super) fn session_action_svg_button(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    icon_path: &'static str,
    tooltip: impl Into<String>,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    // Tauri ActiveSessions action icons: h-7 ghost.
    let tooltip = tooltip.into();
    div()
        .id(SharedString::from(id.into()))
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(rgb(if enabled {
            palette.text_muted
        } else {
            palette.text_dimmed
        }))
        .when(enabled, |this| {
            this.cursor_pointer().hover(|this| {
                this.bg(rgb(palette.surface_elevated))
                    .text_color(rgb(palette.text))
            })
        })
        .when(!enabled, |this| this.opacity(0.4))
        .child(svg().size(px(16.)).flex_none().path(icon_path))
        .tooltip(move |_, cx| {
            cx.new(|_| crate::features::ChromeTooltip::new(tooltip.clone()))
                .into()
        })
        .on_click(move |event, window, cx| {
            if enabled {
                on_click(event, window, cx);
            }
        })
}

pub(super) fn format_otp_code_display(code: &str) -> String {
    let trimmed = code.trim();
    if trimmed.is_empty() || trimmed == "------" {
        return "------".to_string();
    }
    let digits: String = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
    digits
        .as_bytes()
        .chunks(3)
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn session_kind_icon_path(kind: SessionKind) -> &'static str {
    match kind {
        SessionKind::Ssh => "icons/conn/server.svg",
        SessionKind::Telnet | SessionKind::RawTcp => "icons/conn/telnet.svg",
        SessionKind::Serial => "icons/conn/serial.svg",
        SessionKind::LocalPty => "icons/conn/terminal.svg",
    }
}

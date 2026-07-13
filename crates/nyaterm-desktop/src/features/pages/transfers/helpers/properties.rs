use super::*;

pub(in crate::features::pages::transfers) fn symlink_input_row(
    palette: crate::theme::ThemePalette,
    id: &'static str,
    label: &'static str,
    value: &str,
    focused: bool,
    invalid: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .mt_3()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(72.))
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child(label),
        )
        .child(
            div()
                .id(SharedString::from(id))
                .h(px(36.))
                .flex_1()
                .min_w_0()
                .rounded_sm()
                .border_1()
                .border_color(if invalid {
                    rgb(palette.danger)
                } else if focused {
                    rgb(palette.success)
                } else {
                    rgb(palette.border)
                })
                .bg(rgb(palette.input))
                .px_3()
                .flex()
                .items_center()
                .font_family("JetBrains Mono")
                .text_sm()
                .text_color(
                    if value.is_empty() || value == "Symlink name" || value == "/path/to/target" {
                        rgb(palette.text_muted)
                    } else {
                        rgb(palette.text)
                    },
                )
                .cursor_pointer()
                .on_click(on_click)
                .child(truncate_preview(value, 88)),
        )
}

pub(in crate::features::pages::transfers) fn property_row(
    palette: crate::theme::ThemePalette,
    label: &'static str,
    value: impl Into<SharedString>,
) -> impl IntoElement {
    div()
        .flex()
        .items_start()
        .gap_3()
        .text_xs()
        .child(
            div()
                .w(px(88.))
                .text_color(rgb(palette.text_muted))
                .child(format!("{label}:")),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .font_family("JetBrains Mono")
                .text_color(rgb(palette.text))
                .child(value.into()),
        )
}

pub(in crate::features::pages::transfers) fn property_input_row(
    palette: crate::theme::ThemePalette,
    id: &'static str,
    label: &'static str,
    value: &str,
    focused: bool,
    disabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_3()
        .child(
            div()
                .w(px(72.))
                .text_xs()
                .text_color(rgb(palette.text_muted))
                .child(label),
        )
        .child(
            div()
                .id(SharedString::from(id))
                .h(px(34.))
                .flex_1()
                .min_w_0()
                .rounded_sm()
                .border_1()
                .border_color(if focused {
                    rgb(palette.success)
                } else {
                    rgb(palette.border)
                })
                .bg(if disabled {
                    rgb(palette.surface)
                } else {
                    rgb(palette.input)
                })
                .px_3()
                .flex()
                .items_center()
                .font_family("JetBrains Mono")
                .text_xs()
                .text_color(if value.is_empty() {
                    rgb(palette.text_muted)
                } else {
                    rgb(palette.text)
                })
                .cursor_pointer()
                .on_click(on_click)
                .child(if value.is_empty() {
                    SharedString::from("-")
                } else {
                    SharedString::from(value.to_string())
                }),
        )
}

pub(in crate::features::pages::transfers) fn transfer_properties_state_from_entry(
    entry: SftpFileEntry,
) -> TransferPropertiesState {
    let mode_value = entry
        .permissions
        .map(format_permissions_octal)
        .unwrap_or_else(|| "0644".to_string());
    TransferPropertiesState {
        owner_value: String::new(),
        group_value: String::new(),
        entry,
        properties: None,
        mode_value,
        recursive: false,
        saving: false,
        error: None,
        focused_field: TransferPropertiesField::Mode,
    }
}

pub(in crate::features::pages::transfers) fn parse_transfer_mode(value: &str) -> Option<u32> {
    let value = value.trim();
    if !(3..=4).contains(&value.len()) || !value.chars().all(|ch| ('0'..='7').contains(&ch)) {
        return None;
    }
    u32::from_str_radix(value, 8).ok()
}

pub(in crate::features::pages::transfers) fn format_owner_group(
    name: &str,
    id: Option<u32>,
) -> String {
    match (name.trim().is_empty(), id) {
        (true, Some(id)) => id.to_string(),
        (true, None) => "-".to_string(),
        (false, Some(id)) => format!("{} [{}]", name.trim(), id),
        (false, None) => name.trim().to_string(),
    }
}

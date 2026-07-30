use gpui::{IntoElement, SharedString, div, prelude::*, px, rgb, svg};
use nyaterm_ui::{NyaDropdownMenu, NyaMenuItem};

use crate::widgets::status_pill;

pub(super) struct QuickCommandRowPresentation<'a> {
    pub command_id: &'a str,
    pub show_badge: bool,
    pub execution_mode: &'a str,
}

pub(super) struct QuickCommandRowHandlers<OnRun, OnDetails> {
    pub on_run: OnRun,
    pub on_details: OnDetails,
    pub menu_items: Vec<NyaMenuItem>,
}

pub(super) fn quick_command_row_actions<OnRun, OnDetails>(
    palette: crate::theme::ThemePalette,
    presentation: QuickCommandRowPresentation<'_>,
    handlers: QuickCommandRowHandlers<OnRun, OnDetails>,
) -> impl IntoElement
where
    OnRun: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
    OnDetails: Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
{
    let QuickCommandRowPresentation {
        command_id,
        show_badge,
        execution_mode,
    } = presentation;
    let QuickCommandRowHandlers {
        on_run,
        on_details,
        menu_items,
    } = handlers;
    // Tauri renderCommandActions: optional badge + Send + Details + More menu.
    div()
        .flex()
        .items_center()
        .gap_1()
        .flex_none()
        .when(show_badge, |this| {
            this.child(status_pill(
                if execution_mode == "append" {
                    "append"
                } else {
                    "exec"
                },
                if execution_mode == "append" {
                    rgb(palette.warning)
                } else {
                    rgb(palette.success)
                },
                if execution_mode == "append" {
                    rgb(0x32280f)
                } else {
                    rgb(palette.hover)
                },
            ))
        })
        .child(quick_command_action_icon_button(
            palette,
            format!("quick-command-run-{command_id}"),
            "icons/send.svg",
            on_run,
        ))
        .child(quick_command_action_icon_button(
            palette,
            format!("quick-command-detail-{command_id}"),
            "icons/eye.svg",
            on_details,
        ))
        .child(
            NyaDropdownMenu::new(format!("quick-command-more-{command_id}"))
                .icon("icons/session/more.svg")
                .icon_size(px(14.))
                .min_width(px(148.))
                .items(menu_items),
        )
}

fn quick_command_action_icon_button(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    icon_path: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .size(px(26.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(rgb(palette.text_muted))
        .cursor_pointer()
        .hover(|this| {
            this.bg(rgb(palette.surface_elevated))
                .text_color(rgb(palette.text))
        })
        .child(
            svg()
                .size(px(14.))
                .flex_none()
                .path(icon_path)
                .text_color(rgb(palette.text_muted)),
        )
        .on_click(on_click)
}

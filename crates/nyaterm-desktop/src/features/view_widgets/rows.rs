use gpui::{
    App, ClickEvent, FontWeight, IntoElement, SharedString, Window, div, prelude::*, px, rgb,
};
use nyaterm_core::{CloudSyncHistoryEntry, truncate_preview};

use crate::features::formatting::{
    cloud_sync_history_summary, cloud_sync_kind_text_color, cloud_sync_status_dot_color,
    cloud_sync_status_text_color, compact_id, format_history_timestamp_ms,
};
use crate::theme::ThemePalette;

pub(in crate::features) fn cloud_sync_history_row(
    palette: ThemePalette,
    entry: CloudSyncHistoryEntry,
    kind_label: String,
    status_label: String,
    trigger_label: String,
    provider_label: String,
    duration_label: String,
    revision_label: &'static str,
    view_details_label: &'static str,
    hide_details_label: &'static str,
    copy_message_label: &'static str,
    expanded: bool,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_copy: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let summary = cloud_sync_history_summary(&entry);
    let normalized = entry
        .message
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let is_problem = matches!(entry.status.as_str(), "failed" | "conflict");
    let has_message_details = !normalized.is_empty()
        && (is_problem || normalized != summary.split_whitespace().collect::<Vec<_>>().join(" "));
    let has_expandable = has_message_details
        || entry
            .revision
            .as_ref()
            .is_some_and(|r| !r.trim().is_empty());
    let kind_color = cloud_sync_kind_text_color(palette, &entry.kind);
    let status_color = cloud_sync_status_text_color(palette, &entry.status);
    let dot_color = cloud_sync_status_dot_color(palette, &entry.status);
    let timestamp = format_history_timestamp_ms(entry.timestamp_ms);
    let revision = entry
        .revision
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(compact_id);
    let message = entry.message.clone();

    // Tauri SyncBackupHistory list: dense row, compact meta chips, copy on expand.
    div()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(rgb(palette.surface_elevated))
        .child(
            div()
                .flex()
                .items_start()
                .gap_2()
                .child(
                    div()
                        .mt(px(5.))
                        .size(px(6.))
                        .rounded_full()
                        .flex_none()
                        .bg(dot_color),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap(px(2.))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .font_weight(FontWeight(700.))
                                        .text_color(kind_color)
                                        .child(kind_label),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.))
                                        .text_color(status_color)
                                        .child(status_label),
                                )
                                .child(
                                    div()
                                        .ml_auto()
                                        .flex_none()
                                        .text_size(px(10.))
                                        .text_color(rgb(palette.text_dimmed))
                                        .child(timestamp),
                                ),
                        )
                        .child(
                            div()
                                .text_size(px(11.))
                                .font_weight(FontWeight(600.))
                                .text_color(rgb(palette.text))
                                .overflow_hidden()
                                .child(truncate_preview(&summary, 96)),
                        )
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap_x_3()
                                .gap_y_1()
                                .text_size(px(10.))
                                .text_color(rgb(palette.text_dimmed))
                                .child(trigger_label)
                                .child(provider_label)
                                .child(duration_label),
                        )
                        .when(has_expandable, |this| {
                            this.child(
                                div()
                                    .mt_1()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(
                                        div()
                                            .id(SharedString::from(format!(
                                                "sync-history-toggle-{}",
                                                entry.id
                                            )))
                                            .h(px(22.))
                                            .flex()
                                            .items_center()
                                            .text_size(px(10.))
                                            .text_color(rgb(palette.text_muted))
                                            .cursor_pointer()
                                            .hover(|style| style.text_color(rgb(palette.text)))
                                            .child(if expanded {
                                                hide_details_label
                                            } else {
                                                view_details_label
                                            })
                                            .on_click(on_toggle),
                                    )
                                    .when(expanded && has_message_details, |this| {
                                        this.child(
                                            div()
                                                .id(SharedString::from(format!(
                                                    "sync-history-copy-{}",
                                                    entry.id
                                                )))
                                                .h(px(22.))
                                                .flex()
                                                .items_center()
                                                .text_size(px(10.))
                                                .text_color(rgb(palette.text_muted))
                                                .cursor_pointer()
                                                .hover(|style| style.text_color(rgb(palette.text)))
                                                .child(copy_message_label)
                                                .on_click(on_copy),
                                        )
                                    }),
                            )
                        })
                        .when(expanded && has_message_details, |this| {
                            this.child(
                                div()
                                    .mt_1()
                                    .rounded_md()
                                    .p_2()
                                    .bg(if is_problem {
                                        rgb(0x2a1215)
                                    } else {
                                        rgb(palette.surface)
                                    })
                                    .font_family(crate::features::gpui_code_font_family())
                                    .text_size(px(10.))
                                    .text_color(if is_problem {
                                        rgb(0xffa198)
                                    } else {
                                        rgb(palette.text_muted)
                                    })
                                    .child(message),
                            )
                        })
                        .when(expanded && revision.is_some(), |this| {
                            this.child(
                                div()
                                    .mt_1()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(palette.border))
                                    .bg(rgb(palette.bg))
                                    .px_2()
                                    .py_1()
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(rgb(palette.text_dimmed))
                                            .child(revision_label),
                                    )
                                    .child(
                                        div()
                                            .mt_0()
                                            .font_family(crate::features::gpui_code_font_family())
                                            .text_size(px(11.))
                                            .text_color(rgb(palette.text))
                                            .child(revision.unwrap_or_default()),
                                    ),
                            )
                        }),
                ),
        )
}

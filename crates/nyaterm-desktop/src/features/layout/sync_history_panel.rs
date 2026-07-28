use gpui::{
    ClipboardItem, Context, FontWeight, IntoElement, SharedString, div, prelude::*, px, rgb,
};
use nyaterm_core::truncate_preview;

use crate::features::NyaTermApp;
use crate::features::formatting::{
    cloud_sync_status_dot_color, cloud_sync_status_text_color, configured_cloud_sync_provider,
    format_cloud_provider, format_duration_ms,
};
use crate::features::view_widgets::{cloud_sync_history_row, dialog_action_button};
use crate::widgets::small_button;

impl NyaTermApp {
    pub(in crate::features) fn sync_backup_history_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        // Tauri SyncBackupHistoryPanel:
        // shared PanelHeader + status strip + optional conflict card + dense history list.
        let provider = configured_cloud_sync_provider(&self.cloud_sync_settings);
        let provider_label = format_cloud_provider(&provider);
        let enabled = self.cloud_sync_settings.enabled;
        let state = if !enabled {
            "disabled"
        } else if self.cloud_sync_conflict.is_some() {
            "conflict"
        } else if self.cloud_sync_status.to_ascii_lowercase().contains("fail") {
            "failed"
        } else if self.cloud_sync_status.to_ascii_lowercase().contains("push")
            || self.cloud_sync_status.to_ascii_lowercase().contains("pull")
            || self
                .cloud_sync_status
                .to_ascii_lowercase()
                .contains("running")
        {
            "running"
        } else if self
            .cloud_sync_status
            .to_ascii_lowercase()
            .contains("success")
            || self
                .cloud_sync_status
                .to_ascii_lowercase()
                .contains("synced")
            || self
                .cloud_sync_status
                .to_ascii_lowercase()
                .contains("ready")
        {
            "success"
        } else {
            "idle"
        };
        let state_label = match state {
            "disabled" => self.tr("settings.syncState.disabled"),
            "conflict" => self.tr("settings.syncState.conflict"),
            "failed" => self.tr("settings.syncState.failed"),
            "running" => self.tr("settings.syncState.running"),
            "success" => self.tr("settings.syncState.success"),
            _ => self.tr("settings.syncState.idle"),
        };
        let status_message = self.cloud_sync_status.clone();
        let history = self.cloud_sync_history.clone();
        let expanded = self.cloud_sync_history_expanded.clone();
        let conflict = self.cloud_sync_conflict.clone();

        let mut rows = div().flex().flex_col();
        if history.is_empty() {
            rows = rows.child(
                div()
                    .py_6()
                    .px_3()
                    .text_center()
                    .text_size(px(11.))
                    .text_color(rgb(palette.text_dimmed))
                    .child(self.tr("settings.historyNoEntries")),
            );
        } else {
            for entry in history {
                let entry_id = entry.id.clone();
                let is_open = expanded.contains(&entry_id);
                let copy_message = entry.message.clone();
                let kind_label = self.tr(match entry.kind.as_str() {
                    "sync" => "settings.historyKindSync",
                    "backup" => "settings.historyKindBackup",
                    _ => "settings.historyKindSync",
                });
                let status_label = self.tr(match entry.status.as_str() {
                    "success" => "settings.syncState.success",
                    "failed" => "settings.syncState.failed",
                    "conflict" => "settings.syncState.conflict",
                    "running" => "settings.syncState.running",
                    _ => "settings.syncState.idle",
                });
                let trigger_label = self
                    .tr("settings.historyTrigger")
                    .replace("{{value}}", &entry.trigger);
                let provider = entry
                    .provider
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                    .map(format_cloud_provider)
                    .unwrap_or_else(|| "-".to_string());
                let provider_label = self
                    .tr("settings.historyProvider")
                    .replace("{{value}}", &provider);
                let duration =
                    format_duration_ms(entry.duration_ms).unwrap_or_else(|| "-".to_string());
                let duration_label = self
                    .tr("settings.historyDuration")
                    .replace("{{value}}", &duration);
                rows = rows.child(cloud_sync_history_row(
                    palette,
                    entry,
                    kind_label.to_string(),
                    status_label.to_string(),
                    trigger_label,
                    provider_label,
                    duration_label,
                    self.tr("settings.historyRevision"),
                    self.tr("settings.historyViewDetails"),
                    self.tr("settings.historyHideDetails"),
                    self.tr("settings.historyCopyMessage"),
                    is_open,
                    cx.listener(move |this, _, _, cx| {
                        this.toggle_cloud_sync_history_details(&entry_id, cx);
                    }),
                    cx.listener(move |this, _, _, cx| {
                        if copy_message.trim().is_empty() {
                            this.terminal.view.status = "history entry has no message".to_string();
                        } else {
                            cx.write_to_clipboard(ClipboardItem::new_string(copy_message.clone()));
                            this.terminal.view.status = "sync history message copied".to_string();
                        }
                        cx.notify();
                    }),
                ));
            }
        }

        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(self.shell_transparent_color(palette.surface))
            .child(
                div()
                    .flex_none()
                    .px_3()
                    .py(px(10.))
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_transparent_color(palette.surface))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .flex()
                            .min_w_0()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .size(px(8.))
                                    .rounded_full()
                                    .flex_none()
                                    .bg(cloud_sync_status_dot_color(palette, state)),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette.text_muted))
                                    .child(self.tr("settings.historyCurrentState")),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(600.))
                                    .text_color(cloud_sync_status_text_color(palette, state))
                                    .overflow_hidden()
                                    .child(state_label),
                            )
                            .child(
                                div()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette.border))
                                    .child("·"),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette.text_muted))
                                    .overflow_hidden()
                                    .child(provider_label),
                            ),
                    )
                    .when(
                        !status_message.trim().is_empty() && conflict.is_none(),
                        |this| {
                            this.child(
                                div()
                                    .pl_4()
                                    .text_size(px(12.))
                                    .line_height(px(18.))
                                    .text_color(rgb(palette.text_muted))
                                    .child(truncate_preview(&status_message, 140)),
                            )
                        },
                    ),
            )
            .when_some(conflict, |this, conflict| {
                this.child(
                    div()
                        .flex_none()
                        .m_2()
                        .rounded_md()
                        .border_1()
                        .border_color(rgb(palette.warning))
                        .bg(rgb(palette.input))
                        .child(
                            div()
                                .px_3()
                                .py_2()
                                .border_b_1()
                                .border_color(rgb(palette.warning))
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(palette.warning))
                                        .child(self.tr("settings.syncConflictTitle")),
                                ),
                        )
                        .child(
                            div()
                                .px_3()
                                .py_2()
                                .text_size(px(11.))
                                .text_color(rgb(palette.text))
                                .child(conflict.message.clone()),
                        )
                        .child(
                            div().px_3().pb_2().grid().grid_cols(1).gap_2().child(
                                div()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(palette.border))
                                    .bg(rgb(palette.input))
                                    .px_2()
                                    .py_1()
                                    .child(
                                        div()
                                            .text_size(px(10.))
                                            .text_color(rgb(palette.text_muted))
                                            .child(self.tr("settings.providerLabel")),
                                    )
                                    .child(
                                        div()
                                            .mt_0()
                                            .font_family(crate::features::gpui_code_font_family())
                                            .text_size(px(11.))
                                            .text_color(rgb(palette.text))
                                            .child(format_cloud_provider(&conflict.provider)),
                                    ),
                            ),
                        )
                        .child(
                            div()
                                .px_3()
                                .pb_3()
                                .flex()
                                .gap_2()
                                .child(small_button(
                                    palette,
                                    "sync-panel-force-pull",
                                    self.tr("settings.downloadRemoteVersion"),
                                    cx.listener({
                                        let provider_action = conflict.provider_action;
                                        move |this, _, window, cx| {
                                            this.prompt_cloud_sync_force_pull(
                                                provider_action,
                                                window,
                                                cx,
                                            );
                                        }
                                    }),
                                ))
                                .child(dialog_action_button(
                                    palette,
                                    "sync-panel-force-push",
                                    self.tr("settings.uploadLocalVersion"),
                                    false,
                                    cx.listener({
                                        let provider_action = conflict.provider_action;
                                        move |this, _, window, cx| {
                                            this.prompt_cloud_sync_force_push(
                                                provider_action,
                                                window,
                                                cx,
                                            );
                                        }
                                    }),
                                )),
                        ),
                )
            })
            .child(
                div()
                    .id(SharedString::from("sync-backup-history-list"))
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(rows),
            )
    }
}

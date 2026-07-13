use super::*;

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
            "disabled" => "Disabled",
            "conflict" => "Conflict",
            "failed" => "Failed",
            "running" => "Running",
            "success" => "Success",
            _ => "Idle",
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
                    .child("No sync history yet"),
            );
        } else {
            for entry in history {
                let entry_id = entry.id.clone();
                let is_open = expanded.contains(&entry_id);
                let copy_message = entry.message.clone();
                rows = rows.child(cloud_sync_history_row(
                    palette,
                    entry,
                    is_open,
                    cx.listener(move |this, _, _, cx| {
                        this.toggle_cloud_sync_history_details(&entry_id, cx);
                    }),
                    cx.listener(move |this, _, _, cx| {
                        if copy_message.trim().is_empty() {
                            this.terminal_status = "history entry has no message".to_string();
                        } else {
                            cx.write_to_clipboard(ClipboardItem::new_string(copy_message.clone()));
                            this.terminal_status = "sync history message copied".to_string();
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
            .bg(rgb(palette.surface))
            .child(
                div()
                    .flex_none()
                    .h(px(36.))
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.section_header))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .flex()
                            .min_w_0()
                            .flex_1()
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
                                    .child("State"),
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
                            )
                            .child(toolbar_svg_button(
                                palette,
                                SharedString::from("sync-history-refresh"),
                                "icons/fe/refresh.svg",
                                cx.listener(|this, _, _, cx| {
                                    this.refresh_cloud_sync_history();
                                    this.terminal_status =
                                        "cloud sync history refreshed".to_string();
                                    cx.notify();
                                }),
                            )),
                    )
                    .when(
                        !status_message.trim().is_empty() && conflict.is_none(),
                        |this| {
                            this.child(
                                div()
                                    .mt_1()
                                    .pl_4()
                                    .text_size(px(11.))
                                    .text_color(rgb(palette.text_muted))
                                    .child(truncate_preview(&status_message, 120)),
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
                                        .child("Sync conflict"),
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
                                            .child("Provider"),
                                    )
                                    .child(
                                        div()
                                            .mt_0()
                                            .font_family("JetBrains Mono")
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
                                    "Use remote",
                                    cx.listener({
                                        let provider_action = conflict.provider_action;
                                        move |this, _, _, cx| {
                                            this.prompt_cloud_sync_force_pull(provider_action, cx);
                                        }
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    "sync-panel-force-push",
                                    "Use local",
                                    cx.listener({
                                        let provider_action = conflict.provider_action;
                                        move |this, _, _, cx| {
                                            this.prompt_cloud_sync_force_push(provider_action, cx);
                                        }
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    "sync-panel-conflict-dismiss",
                                    "Dismiss",
                                    cx.listener(|this, _, _, cx| {
                                        this.dismiss_cloud_sync_conflict(cx);
                                    }),
                                )),
                        ),
                )
            })
            .child(
                div()
                    .flex_none()
                    .px_3()
                    .py_1()
                    .border_b_1()
                    .border_color(rgb(palette.surface_elevated))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text_dimmed))
                            .child("HISTORY"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
                                palette,
                                "sync-panel-push",
                                "Push",
                                cx.listener(|this, _, _, cx| {
                                    if configured_cloud_sync_provider(&this.cloud_sync_settings)
                                        != "local_directory"
                                    {
                                        this.prompt_provider_cloud_sync_push(cx);
                                    } else {
                                        this.prompt_local_cloud_sync_push(cx);
                                    }
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "sync-panel-pull",
                                "Pull",
                                cx.listener(|this, _, _, cx| {
                                    if configured_cloud_sync_provider(&this.cloud_sync_settings)
                                        != "local_directory"
                                    {
                                        this.prompt_provider_cloud_sync_pull(cx);
                                    } else {
                                        this.prompt_local_cloud_sync_pull(cx);
                                    }
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "sync-panel-settings",
                                "Settings",
                                cx.listener(|this, _, _, cx| {
                                    this.settings_active_tab = SettingsTab::SyncBackup;
                                    this.open_page(NavItem::Settings, cx);
                                }),
                            )),
                    ),
            )
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

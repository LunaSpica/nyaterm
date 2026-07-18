use super::*;

impl NyaTermApp {
    pub(in crate::features) fn left_connections_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let mut rows = div().flex().flex_col().gap_2();
        if self.connections.is_empty() {
            rows = rows.child(empty_panel(
                "No saved connections imported yet.",
                self.theme_palette(),
            ));
        } else {
            for connection in self.connections.iter().take(8).cloned() {
                rows = rows.child(compact_connection_row(
                    palette,
                    &connection,
                    cx.listener({
                        let connection = connection.clone();
                        move |this, _, window, cx| {
                            this.start_saved_connection(connection.clone(), window, cx);
                        }
                    }),
                ));
            }
        }

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.input))
                    .p_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(palette.text_muted))
                                    .child("SAVED CONNECTIONS"),
                            )
                            .child(status_pill(
                                status_label(&self.terminal_status),
                                rgb(palette.link),
                                rgb(palette.hover),
                            )),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "left-connections-local",
                                "Local",
                                cx.listener(|this, _, window, cx| {
                                    this.start_local_session(window, cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "left-connections-refresh",
                                "Refresh",
                                cx.listener(|this, _, _, cx| {
                                    this.refresh_store_from_runtime();
                                    this.terminal_status = "connections refreshed".to_string();
                                    cx.notify();
                                }),
                            )),
                    ),
            )
            .child(rows)
    }

    pub(in crate::features) fn left_network_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let mut rows = div().flex().flex_col().gap_2();
        if self.tunnels.is_empty() {
            rows = rows.child(empty_panel(
                "No SSH tunnels configured.",
                self.theme_palette(),
            ));
        } else {
            for tunnel in self.tunnels.iter().take(8).cloned() {
                let is_pending = self.pending_tunnels.iter().any(|id| id == &tunnel.id);
                let is_open = self.tunnel_manager.is_open(&tunnel.id).unwrap_or(false);
                rows = rows.child(compact_tunnel_row(
                    palette,
                    &tunnel,
                    is_open,
                    is_pending,
                    cx.listener({
                        let tunnel = tunnel.clone();
                        move |this, _, window, cx| {
                            this.start_tunnel_job(tunnel.clone(), window, cx);
                        }
                    }),
                    cx.listener({
                        let tunnel_id = tunnel.id.clone();
                        move |this, _, _, cx| {
                            this.close_tunnel_job(tunnel_id.clone(), cx);
                        }
                    }),
                ));
            }
        }

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.input))
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text_muted))
                            .child("NETWORK"),
                    )
                    .child(capability_line(
                        palette,
                        "Configured Tunnels",
                        self.tunnels.len().to_string(),
                    ))
                    .child(capability_line(
                        palette,
                        "Pending",
                        self.pending_tunnels.len().to_string(),
                    ))
                    .child(capability_line(
                        palette,
                        "Active SSH",
                        if self.active_ssh_config.is_some() {
                            "ready"
                        } else {
                            "none"
                        },
                    )),
            )
            .child(rows)
    }

    pub(in crate::features) fn left_transfers_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let mut jobs = div().flex().flex_col().gap_2();
        if self.transfer_jobs.is_empty() {
            jobs = jobs.child(empty_panel(
                "No SFTP transfer jobs yet.",
                self.theme_palette(),
            ));
        } else {
            for job in self.transfer_jobs.iter().rev().take(5) {
                jobs = jobs.child(compact_transfer_job_row(palette, job));
            }
        }

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.input))
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text_muted))
                            .child("SFTP"),
                    )
                    .child(capability_line(
                        palette,
                        "SSH Session",
                        if self.active_ssh_config.is_some() {
                            "ready"
                        } else {
                            "none"
                        },
                    ))
                    .child(capability_line(
                        palette,
                        "Remote Path",
                        truncate_preview(&self.transfer_remote_path, 28),
                    ))
                    .child(capability_line(
                        palette,
                        "Duplicate Policy",
                        duplicate_policy_label(self.transfer_duplicate_policy),
                    ))
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "left-sftp-list",
                                "List",
                                cx.listener(|this, _, window, cx| {
                                    this.start_sftp_list_job(window, cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "left-sftp-download",
                                "Download",
                                cx.listener(|this, _, window, cx| {
                                    this.start_sftp_download_job(window, cx);
                                }),
                            )),
                    ),
            )
            .child(jobs)
    }

    pub(in crate::features) fn left_settings_panel(
        &mut self,
        _cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.input))
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text_muted))
                            .child("SETTINGS"),
                    )
                    .child(capability_line(
                        palette,
                        "Theme",
                        self.settings.theme.clone(),
                    ))
                    .child(capability_line(
                        palette,
                        "Terminal Font",
                        format!(
                            "{} {}",
                            self.settings.terminal_font_family, self.settings.terminal_font_size
                        ),
                    ))
                    .child(capability_line(
                        palette,
                        "Host Key Policy",
                        self.settings.host_key_policy.clone(),
                    ))
                    .child(capability_line(
                        palette,
                        "AI",
                        if self.ai_settings.enabled {
                            "enabled"
                        } else {
                            "disabled"
                        },
                    )),
            )
            .child(
                div()
                    .rounded_md()
                    .border_1()
                    .border_color(if self.store_status.ready {
                        rgb(palette.hover)
                    } else {
                        rgb(palette.hover)
                    })
                    .bg(rgb(palette.input))
                    .p_3()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text_muted))
                            .child("CONFIG STORE"),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_sm()
                            .text_color(if self.store_status.ready {
                                rgb(palette.success)
                            } else {
                                rgb(palette.danger)
                            })
                            .child(self.store_status.message.clone()),
                    )
                    .child(
                        div()
                            .mt_2()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(self.store_status.path.clone()),
                    ),
            )
    }
}

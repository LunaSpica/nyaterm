use super::*;

impl NyaTermApp {
    pub(in crate::features) fn right_stats_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let stats = self.remote_ops.stats.data.clone().unwrap_or_default();
        let memory_total = stats.memory.used.saturating_add(stats.memory.available);
        let memory_percent = if memory_total > 0 {
            stats.memory.used as f64 / memory_total as f64 * 100.
        } else {
            0.
        };
        let disk_summary = stats
            .disks
            .iter()
            .max_by_key(|disk| disk.use_percent)
            .map(|disk| format!("{} {}%", disk.mount, disk.use_percent))
            .unwrap_or_else(|| "n/a".to_string());

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                inspector_card(palette, "Resource Monitor")
                    .child(capability_line(
                        palette,
                        "SSH",
                        if self.active_ssh_config.is_some() {
                            "ready"
                        } else {
                            "none"
                        },
                    ))
                    .child(capability_line(
                        palette,
                        "Host",
                        if stats.system.hostname.trim().is_empty() {
                            "n/a".to_string()
                        } else {
                            truncate_preview(&stats.system.hostname, 34)
                        },
                    ))
                    .child(capability_line(
                        palette,
                        "CPU",
                        format!("{:.1}%", stats.cpu.usage),
                    ))
                    .child(capability_line(
                        palette,
                        "Memory",
                        format!("{memory_percent:.0}%"),
                    ))
                    .child(capability_line(palette, "Disk", disk_summary))
                    .child(div().mt_3().child(small_button(
                        palette,
                        "right-stats-refresh",
                        if self.remote_ops.stats.pending {
                            "Loading"
                        } else {
                            "Refresh"
                        },
                        cx.listener(|this, _, window, cx| {
                            this.refresh_stats(window, cx);
                        }),
                    ))),
            )
            .child(
                inspector_card(palette, "Networks")
                    .child(compact_network_rows(palette, &stats.networks)),
            )
            .child(inspector_status_line(
                palette,
                self.remote_ops.stats.status.clone(),
            ))
    }

    pub(in crate::features) fn right_process_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let top_process = self.remote_ops.process.items.iter().max_by(|left, right| {
            left.cpu_percent
                .partial_cmp(&right.cpu_percent)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let top_label = top_process
            .map(|process| {
                format!(
                    "{} {:.1}% CPU",
                    truncate_preview(&process.command, 18),
                    process.cpu_percent
                )
            })
            .unwrap_or_else(|| "n/a".to_string());

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                inspector_card(palette, "Process Manager")
                    .child(capability_line(
                        palette,
                        "SSH",
                        if self.active_ssh_config.is_some() {
                            "ready"
                        } else {
                            "none"
                        },
                    ))
                    .child(capability_line(
                        palette,
                        "Processes",
                        self.remote_ops.process.items.len().to_string(),
                    ))
                    .child(capability_line(palette, "Top CPU", top_label))
                    .child(div().mt_3().child(small_button(
                        palette,
                        "right-process-refresh",
                        if self.remote_ops.process.pending {
                            "Loading"
                        } else {
                            "Refresh"
                        },
                        cx.listener(|this, _, window, cx| {
                            this.refresh_processes(window, cx);
                        }),
                    ))),
            )
            .child(
                inspector_card(palette, "Hot Processes").child(compact_process_rows(
                    palette,
                    &self.remote_ops.process.items,
                )),
            )
            .child(inspector_status_line(
                palette,
                self.remote_ops.process.status.clone(),
            ))
    }

    pub(in crate::features) fn right_docker_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let overview = self.remote_ops.docker.overview.clone().unwrap_or_default();
        let running = overview
            .containers
            .iter()
            .filter(|container| container.state == "running")
            .count();

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                inspector_card(palette, "Docker")
                    .child(capability_line(
                        palette,
                        "SSH",
                        if self.active_ssh_config.is_some() {
                            "ready"
                        } else {
                            "none"
                        },
                    ))
                    .child(capability_line(
                        palette,
                        "Available",
                        if overview.available { "yes" } else { "no" },
                    ))
                    .child(capability_line(
                        palette,
                        "Version",
                        truncate_preview(&overview.version, 24),
                    ))
                    .child(capability_line(
                        palette,
                        "Containers",
                        overview.containers.len().to_string(),
                    ))
                    .child(capability_line(palette, "Running", running.to_string()))
                    .child(div().mt_3().child(small_button(
                        palette,
                        "right-docker-refresh",
                        if self.remote_ops.docker.pending {
                            "Loading"
                        } else {
                            "Refresh"
                        },
                        cx.listener(|this, _, window, cx| {
                            this.refresh_docker(window, cx);
                        }),
                    ))),
            )
            .child(
                inspector_card(palette, "Containers")
                    .child(compact_docker_container_rows(palette, &overview.containers)),
            )
            .child(inspector_status_line(
                palette,
                self.remote_ops.docker.status.clone(),
            ))
    }

    pub(in crate::features) fn right_translation_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let translated = self
            .translate_result
            .as_ref()
            .map(|result| truncate_preview(&result.translated, 180))
            .unwrap_or_else(|| "No translation result yet.".to_string());
        let detected = self
            .translate_result
            .as_ref()
            .map(|result| result.detected_language.clone())
            .unwrap_or_else(|| "n/a".to_string());

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                inspector_card(palette, "Translation")
                    .child(capability_line(
                        palette,
                        "Provider",
                        self.translate_provider.clone(),
                    ))
                    .child(capability_line(
                        palette,
                        "Target",
                        self.translate_target_language.clone(),
                    ))
                    .child(capability_line(palette, "Detected", detected))
                    .child(
                        div()
                            .mt_3()
                            .text_xs()
                            .line_height(px(18.))
                            .text_color(rgb(palette.text))
                            .child(translated),
                    )
                    .child(
                        div()
                            .mt_3()
                            .flex()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "right-translation-run",
                                if self.translate_pending {
                                    "Translating"
                                } else {
                                    "Translate"
                                },
                                cx.listener(|this, _, window, cx| {
                                    this.run_translation(window, cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "right-translation-save",
                                "Save",
                                cx.listener(|this, _, _, cx| {
                                    this.save_translation_settings(cx);
                                }),
                            )),
                    ),
            )
            .child(inspector_status_line(
                palette,
                self.translate_status.clone(),
            ))
    }
}

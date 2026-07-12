use super::*;
use gpui::SharedString;

impl NyaTermApp {
    pub(in crate::ui::view) fn stats_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let can_refresh = self.active_ssh_config.is_some() && !self.stats_pending;
        let stats = self.remote_stats.clone().unwrap_or_default();
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
        let net_summary = stats
            .networks
            .iter()
            .map(|net| net.rx_bytes_per_sec + net.tx_bytes_per_sec)
            .fold(0.0, f64::max);
        let total_rx_rate = stats
            .networks
            .iter()
            .map(|network| network.rx_bytes_per_sec)
            .sum::<f64>();
        let total_tx_rate = stats
            .networks
            .iter()
            .map(|network| network.tx_bytes_per_sec)
            .sum::<f64>();
        let busiest_disk = stats.disks.iter().max_by_key(|disk| disk.use_percent);
        let busiest_network = stats.networks.iter().max_by(|left, right| {
            (left.rx_bytes_per_sec + left.tx_bytes_per_sec)
                .partial_cmp(&(right.rx_bytes_per_sec + right.tx_bytes_per_sec))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut networks = div().flex().flex_col().gap_2();
        if self.remote_stats.is_none() {
            networks = networks.child(empty_panel(
                if self.active_ssh_config.is_some() {
                    "No stats snapshot loaded."
                } else {
                    "Start an SSH session to inspect remote stats."
                },
                self.theme_palette(),
            ));
        } else if stats.networks.is_empty() {
            networks = networks.child(empty_panel(
                "No active physical network interfaces found.",
                self.theme_palette(),
            ));
        } else {
            for network in &stats.networks {
                networks = networks.child(stats_resource_row(
                    palette,
                    &network.nic,
                    &format!(
                        "{} · rx {} · tx {}",
                        network.state,
                        format_rate(network.rx_bytes_per_sec),
                        format_rate(network.tx_bytes_per_sec)
                    ),
                    (network.rx_bytes_per_sec + network.tx_bytes_per_sec) / net_summary.max(1.0),
                ));
            }
        }

        let mut disks = div().flex().flex_col().gap_2();
        if self.remote_stats.is_some() && stats.disks.is_empty() {
            disks = disks.child(empty_panel(
                "No mounted block devices found.",
                self.theme_palette(),
            ));
        } else {
            for disk in &stats.disks {
                disks = disks.child(stats_resource_row(
                    palette,
                    &disk.mount,
                    &format!(
                        "{} · {} free of {}",
                        disk.device,
                        format_file_size(Some(disk.available)),
                        format_file_size(Some(disk.total))
                    ),
                    disk.use_percent as f64 / 100.,
                ));
            }
        }

        // Tauri ResourceMonitor: compact toolbar + scrollable gauges/lists.
        let host_label = if stats.system.hostname.trim().is_empty() {
            "remote".to_string()
        } else {
            truncate_preview(&stats.system.hostname, 24)
        };
        div()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .bg(rgb(self.theme_palette().surface))
            .child(
                div()
                    .h(px(36.))
                    .flex_none()
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(self.theme_palette().border))
                    .bg(rgb(self.theme_palette().section_header))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .overflow_hidden()
                            .child(format!(
                                "{host_label} · CPU {:.0}% · MEM {:.0}% · {}",
                                stats.cpu.usage, memory_percent, disk_summary
                            )),
                    )
                    .child(div().when(!can_refresh, |this| this.opacity(0.45)).child(
                        compact_remote_svg_button(
                            palette,
                            "stats-refresh",
                            "icons/fe/refresh.svg",
                            cx.listener(|this, _, window, cx| {
                                this.refresh_stats(window, cx);
                            }),
                        ),
                    )),
            )
            .child(
                div()
                    .id(SharedString::from("stats-scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_scroll()
                    .scrollbar_width(px(6.))
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .grid()
                            .grid_cols(4)
                            .gap_2()
                            .child(resource_gauge_card(
                                palette,
                                "CPU",
                                format!("{:.0}%", stats.cpu.usage.clamp(0., 100.)),
                                truncate_preview(&stats.cpu.model, 54),
                                stats.cpu.usage / 100.,
                            ))
                            .child(resource_gauge_card(
                                palette,
                                "Memory",
                                format!("{memory_percent:.0}%"),
                                format!(
                                    "{} used / {} total",
                                    format_file_size(Some(stats.memory.used)),
                                    format_file_size(Some(memory_total))
                                ),
                                memory_percent / 100.,
                            ))
                            .child(resource_summary_card(
                                palette,
                                "Load",
                                format!("{:.2}", stats.load.load1),
                                format!(
                                    "5m {:.2} · 15m {:.2} · {} core(s)",
                                    stats.load.load5, stats.load.load15, stats.cpu.cores
                                ),
                                load_ratio(stats.load.load1, stats.cpu.cores),
                            ))
                            .child(resource_summary_card(
                                palette,
                                "Network",
                                format!(
                                    "{} / {}",
                                    format_rate(total_rx_rate),
                                    format_rate(total_tx_rate)
                                ),
                                busiest_network
                                    .map(|network| {
                                        format!(
                                            "{} busiest · {}",
                                            network.nic,
                                            format_rate(
                                                network.rx_bytes_per_sec + network.tx_bytes_per_sec
                                            )
                                        )
                                    })
                                    .unwrap_or_else(|| "No active interfaces".to_string()),
                                (total_rx_rate + total_tx_rate) / net_summary.max(1.0),
                            )),
                    )
                    .child(
                        div()
                            .grid()
                            .grid_cols(3)
                            .gap_2()
                            .child(
                                div()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(palette.border))
                                    .bg(rgb(palette.bg))
                                    .p_2()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight(700.))
                                            .text_color(rgb(palette.text_muted))
                                            .child("System"),
                                    )
                                    .child(dense_capability_line(
                                        palette,
                                        "OS",
                                        truncate_preview(&stats.system.os, 52),
                                    ))
                                    .child(dense_capability_line(
                                        palette,
                                        "Arch",
                                        stats.system.arch.clone(),
                                    ))
                                    .child(dense_capability_line(
                                        palette,
                                        "Uptime",
                                        format_uptime(stats.system.uptime_sec),
                                    ))
                                    .child(dense_capability_line(
                                        palette,
                                        "CPU Model",
                                        truncate_preview(&stats.cpu.model, 52),
                                    ))
                                    .child(dense_capability_line(
                                        palette,
                                        "Cores",
                                        stats.cpu.cores.to_string(),
                                    )),
                            )
                            .child(
                                div()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(palette.border))
                                    .bg(rgb(palette.bg))
                                    .p_2()
                                    .child(
                                        div().text_sm().font_weight(FontWeight(700.)).child("Load"),
                                    )
                                    .child(dense_capability_line(
                                        palette,
                                        "1 min",
                                        format!("{:.2}", stats.load.load1),
                                    ))
                                    .child(dense_capability_line(
                                        palette,
                                        "5 min",
                                        format!("{:.2}", stats.load.load5),
                                    ))
                                    .child(dense_capability_line(
                                        palette,
                                        "15 min",
                                        format!("{:.2}", stats.load.load15),
                                    ))
                                    .when(!stats.cpu.per_core.is_empty(), |this| {
                                        this.child(cpu_core_summary(
                                            palette,
                                            &stats.cpu.per_core,
                                            self.stats_cpu_expanded,
                                            cx,
                                        ))
                                    }),
                            )
                            .child(
                                div()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(palette.border))
                                    .bg(rgb(palette.bg))
                                    .p_2()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight(700.))
                                            .child("Memory"),
                                    )
                                    .child(dense_capability_line(
                                        palette,
                                        "Used",
                                        format_file_size(Some(stats.memory.used)),
                                    ))
                                    .child(dense_capability_line(
                                        palette,
                                        "Available",
                                        format_file_size(Some(stats.memory.available)),
                                    ))
                                    .child(dense_capability_line(
                                        palette,
                                        "Cached",
                                        format_file_size(Some(stats.memory.cached)),
                                    ))
                                    .child(dense_capability_line(
                                        palette,
                                        "Total",
                                        format_file_size(Some(memory_total)),
                                    ))
                                    .child(stats_progress_bar(palette, memory_percent / 100.)),
                            ),
                    )
                    .child(
                        div()
                            .grid()
                            .grid_cols(2)
                            .gap_3()
                            .child(
                                div()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(palette.border))
                                    .bg(rgb(palette.bg))
                                    .p_2()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight(700.))
                                            .text_color(rgb(palette.text_muted))
                                            .child("Network"),
                                    )
                                    .child(networks),
                            )
                            .child(
                                div()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(palette.border))
                                    .bg(rgb(palette.bg))
                                    .p_2()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight(700.))
                                            .child("Disks"),
                                    )
                                    .when(busiest_disk.is_some(), |this| {
                                        let disk = busiest_disk.cloned().expect("checked is_some");
                                        this.child(
                                            div()
                                                .mt_2()
                                                .rounded_sm()
                                                .border_1()
                                                .border_color(usage_color(
                                                    palette,
                                                    disk.use_percent as f64 / 100.,
                                                ))
                                                .bg(rgb(palette.input))
                                                .p_2()
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .font_weight(FontWeight(700.))
                                                        .text_color(rgb(palette.text))
                                                        .child(format!(
                                                            "Busiest mount: {} ({}%)",
                                                            disk.mount, disk.use_percent
                                                        )),
                                                )
                                                .child(stats_progress_bar(
                                                    palette,
                                                    disk.use_percent as f64 / 100.,
                                                )),
                                        )
                                    })
                                    .child(disks),
                            ),
                    ),
            )
    }
}

fn cpu_core_summary(
    palette: crate::ui::theme::ThemePalette,
    per_core: &[f64],
    expanded: bool,
    cx: &mut Context<NyaTermApp>,
) -> gpui::Div {
    let visible_count = if expanded {
        per_core.len()
    } else {
        per_core.len().min(8)
    };
    let preview = per_core
        .iter()
        .take(visible_count)
        .map(|usage| format!("{usage:.0}%"))
        .collect::<Vec<_>>()
        .join(" ");
    let overflow = per_core.len().saturating_sub(visible_count);
    let summary = if overflow > 0 {
        format!("{preview} +{overflow}")
    } else {
        preview
    };

    let mut rows = div().mt_2().flex().flex_col().gap_2().child(
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .child(dense_capability_line(palette, "Per Core", summary))
            .child(small_button(
                palette,
                "stats-cpu-cores-toggle",
                if expanded { "Hide" } else { "Show" },
                cx.listener(|this, _, _, cx| {
                    this.toggle_stats_cpu_expanded(cx);
                }),
            )),
    );

    if expanded {
        let mut core_rows = div().grid().grid_cols(2).gap_2();
        for (index, usage) in per_core.iter().copied().enumerate() {
            core_rows = core_rows.child(cpu_core_row(palette, index + 1, usage));
        }
        rows = rows.child(core_rows);
    }

    rows
}

fn cpu_core_row(palette: crate::ui::theme::ThemePalette, index: usize, usage: f64) -> gpui::Div {
    let ratio = (usage / 100.).clamp(0., 1.);
    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .p_2()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .font_family("JetBrains Mono")
                        .text_xs()
                        .text_color(rgb(palette.text))
                        .child(format!("CPU {index}")),
                )
                .child(
                    div()
                        .font_family("JetBrains Mono")
                        .text_xs()
                        .text_color(usage_color(palette, ratio))
                        .child(format!("{usage:.1}%")),
                ),
        )
        .child(stats_progress_bar(palette, ratio))
}

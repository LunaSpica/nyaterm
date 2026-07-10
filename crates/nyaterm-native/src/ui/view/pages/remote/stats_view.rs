use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn stats_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
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
            networks = networks.child(empty_panel(if self.active_ssh_config.is_some() {
                "No stats snapshot loaded."
            } else {
                "Start an SSH session to inspect remote stats."
            }));
        } else if stats.networks.is_empty() {
            networks = networks.child(empty_panel("No active physical network interfaces found."));
        } else {
            for network in &stats.networks {
                networks = networks.child(stats_resource_row(
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
            disks = disks.child(empty_panel("No mounted block devices found."));
        } else {
            for disk in &stats.disks {
                disks = disks.child(stats_resource_row(
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

        div()
            .flex()
            .flex_col()
            .size_full()
            .p_5()
            .gap_4()
            .child(section_header(
                "Stats",
                "Native SSH exec system snapshot for the active remote session.",
            ))
            .child(
                div()
                    .grid()
                    .grid_cols(6)
                    .gap_3()
                    .child(metric(
                        "SSH",
                        if self.active_ssh_config.is_some() {
                            "ready".to_string()
                        } else {
                            "none".to_string()
                        },
                    ))
                    .child(metric(
                        "Host",
                        if stats.system.hostname.trim().is_empty() {
                            "n/a".to_string()
                        } else {
                            truncate_preview(&stats.system.hostname, 28)
                        },
                    ))
                    .child(metric("CPU", format!("{:.1}%", stats.cpu.usage)))
                    .child(metric("Load", format!("{:.2}", stats.load.load1)))
                    .child(metric("Memory", format!("{memory_percent:.0}%")))
                    .child(metric("Disk", disk_summary)),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(4)
                    .gap_3()
                    .child(resource_gauge_card(
                        "CPU",
                        format!("{:.0}%", stats.cpu.usage.clamp(0., 100.)),
                        truncate_preview(&stats.cpu.model, 54),
                        stats.cpu.usage / 100.,
                    ))
                    .child(resource_gauge_card(
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
                        "Load",
                        format!("{:.2}", stats.load.load1),
                        format!(
                            "5m {:.2} · 15m {:.2} · {} core(s)",
                            stats.load.load5, stats.load.load15, stats.cpu.cores
                        ),
                        load_ratio(stats.load.load1, stats.cpu.cores),
                    ))
                    .child(resource_summary_card(
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
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x2a3140))
                    .bg(rgb(0x151923))
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(rgb(0xe5edf7))
                                    .child(self.stats_status.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .when(!can_refresh, |this| this.opacity(0.45))
                                    .child(small_button(
                                        "stats-refresh",
                                        "Refresh",
                                        cx.listener(|this, _, window, cx| {
                                            this.refresh_stats(window, cx);
                                        }),
                                    )),
                            ),
                    ),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(3)
                    .gap_3()
                    .child(
                        div()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x2a3140))
                            .bg(rgb(0x151923))
                            .p_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("System"),
                            )
                            .child(capability_line(
                                "OS",
                                truncate_preview(&stats.system.os, 52),
                            ))
                            .child(capability_line("Arch", stats.system.arch.clone()))
                            .child(capability_line(
                                "Uptime",
                                format_uptime(stats.system.uptime_sec),
                            ))
                            .child(capability_line(
                                "CPU Model",
                                truncate_preview(&stats.cpu.model, 52),
                            ))
                            .child(capability_line("Cores", stats.cpu.cores.to_string())),
                    )
                    .child(
                        div()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x2a3140))
                            .bg(rgb(0x151923))
                            .p_4()
                            .child(div().text_sm().font_weight(FontWeight(700.)).child("Load"))
                            .child(capability_line("1 min", format!("{:.2}", stats.load.load1)))
                            .child(capability_line("5 min", format!("{:.2}", stats.load.load5)))
                            .child(capability_line(
                                "15 min",
                                format!("{:.2}", stats.load.load15),
                            ))
                            .when(!stats.cpu.per_core.is_empty(), |this| {
                                this.child(cpu_core_summary(
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
                            .border_color(rgb(0x2a3140))
                            .bg(rgb(0x151923))
                            .p_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("Memory"),
                            )
                            .child(capability_line(
                                "Used",
                                format_file_size(Some(stats.memory.used)),
                            ))
                            .child(capability_line(
                                "Available",
                                format_file_size(Some(stats.memory.available)),
                            ))
                            .child(capability_line(
                                "Cached",
                                format_file_size(Some(stats.memory.cached)),
                            ))
                            .child(capability_line(
                                "Total",
                                format_file_size(Some(memory_total)),
                            ))
                            .child(stats_progress_bar(memory_percent / 100.)),
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
                            .border_color(rgb(0x2a3140))
                            .bg(rgb(0x151923))
                            .p_4()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(700.))
                                    .child("Network"),
                            )
                            .child(networks),
                    )
                    .child(
                        div()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(0x2a3140))
                            .bg(rgb(0x151923))
                            .p_4()
                            .child(div().text_sm().font_weight(FontWeight(700.)).child("Disks"))
                            .when(busiest_disk.is_some(), |this| {
                                let disk = busiest_disk.cloned().expect("checked is_some");
                                this.child(
                                    div()
                                        .mt_2()
                                        .rounded_sm()
                                        .border_1()
                                        .border_color(usage_color(disk.use_percent as f64 / 100.))
                                        .bg(rgb(0x10151e))
                                        .p_2()
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight(700.))
                                                .text_color(rgb(0xe5edf7))
                                                .child(format!(
                                                    "Busiest mount: {} ({}%)",
                                                    disk.mount, disk.use_percent
                                                )),
                                        )
                                        .child(stats_progress_bar(disk.use_percent as f64 / 100.)),
                                )
                            })
                            .child(disks),
                    ),
            )
    }
}

fn cpu_core_summary(per_core: &[f64], expanded: bool, cx: &mut Context<NyaTermApp>) -> gpui::Div {
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
            .child(capability_line("Per Core", summary))
            .child(small_button(
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
            core_rows = core_rows.child(cpu_core_row(index + 1, usage));
        }
        rows = rows.child(core_rows);
    }

    rows
}

fn cpu_core_row(index: usize, usage: f64) -> gpui::Div {
    let ratio = (usage / 100.).clamp(0., 1.);
    div()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x303848))
        .bg(rgb(0x10151e))
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
                        .text_color(rgb(0xcbd5e1))
                        .child(format!("CPU {index}")),
                )
                .child(
                    div()
                        .font_family("JetBrains Mono")
                        .text_xs()
                        .text_color(usage_color(ratio))
                        .child(format!("{usage:.1}%")),
                ),
        )
        .child(stats_progress_bar(ratio))
}

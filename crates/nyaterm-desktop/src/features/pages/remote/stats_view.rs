use super::*;
use gpui::SharedString;

impl NyaTermApp {
    pub(in crate::features) fn stats_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let Some(stats) = self.remote_stats.clone() else {
            let message = if self.active_ssh_config.is_none() {
                "Start an SSH session to inspect remote stats."
            } else if self.stats_pending {
                "Loading remote system stats..."
            } else {
                "No stats snapshot loaded."
            };
            return div()
                .size_full()
                .bg(rgb(palette.surface))
                .child(empty_panel(message, palette));
        };

        let memory_total = stats.memory.used.saturating_add(stats.memory.available);
        let memory_percent = if memory_total > 0 {
            stats.memory.used as f64 / memory_total as f64 * 100.
        } else {
            0.
        };

        let mut network_rows = div().flex().flex_col();
        if stats.networks.is_empty() {
            network_rows = network_rows.child(resource_empty_value(palette));
        } else {
            for network in &stats.networks {
                network_rows = network_rows.child(resource_network_row(
                    palette,
                    &network.nic,
                    network.tx_bytes_per_sec,
                    network.rx_bytes_per_sec,
                ));
            }
        }

        let mut disk_rows = div().flex().flex_col();
        if stats.disks.is_empty() {
            disk_rows = disk_rows.child(resource_empty_value(palette));
        } else {
            for disk in &stats.disks {
                disk_rows = disk_rows.child(resource_disk_row(
                    palette,
                    &disk.mount,
                    disk.total,
                    disk.available,
                    disk.use_percent,
                ));
            }
        }

        div()
            .size_full()
            .overflow_hidden()
            .bg(rgb(palette.surface))
            .child(
                div()
                    .id(SharedString::from("stats-scroll"))
                    .size_full()
                    .overflow_scroll()
                    .scrollbar_width(px(6.))
                    .p_2()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(resource_section_card(
                        palette,
                        "System",
                        div()
                            .grid()
                            .grid_cols(2)
                            .gap_x_3()
                            .gap_y_1()
                            .child(resource_info_cell(
                                palette,
                                "Hostname",
                                if stats.system.hostname.trim().is_empty() {
                                    "remote".to_string()
                                } else {
                                    stats.system.hostname.clone()
                                },
                            ))
                            .child(resource_info_cell(
                                palette,
                                "Arch",
                                stats.system.arch.clone(),
                            ))
                            .child(resource_info_cell(palette, "OS", stats.system.os.clone()))
                            .child(resource_info_cell(
                                palette,
                                "Uptime",
                                format_uptime(stats.system.uptime_sec),
                            )),
                    ))
                    .child(resource_section_card(
                        palette,
                        "CPU",
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(resource_ring_gauge(
                                        palette,
                                        stats.cpu.usage,
                                        format!("{:.0}%", stats.cpu.usage.clamp(0., 100.)),
                                    ))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex_1()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_baseline()
                                                    .justify_between()
                                                    .gap_2()
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(rgb(palette.text_muted))
                                                            .child("Average usage"),
                                                    )
                                                    .child(
                                                        div()
                                                            .font_family(
                                                                crate::features::gpui_code_font_family(),
                                                            )
                                                            .text_size(px(13.))
                                                            .font_weight(FontWeight(700.))
                                                            .text_color(usage_color(
                                                                palette,
                                                                stats.cpu.usage / 100.,
                                                            ))
                                                            .child(format!("{:.1}%", stats.cpu.usage)),
                                                    ),
                                            )
                                            .child(resource_progress_bar(
                                                palette,
                                                stats.cpu.usage / 100.,
                                            ))
                                            .child(
                                                div()
                                                    .text_right()
                                                    .font_family(
                                                        crate::features::gpui_code_font_family(),
                                                    )
                                                    .text_size(px(10.))
                                                    .text_color(rgb(palette.text_dimmed))
                                                    .child(format!("{}C", stats.cpu.cores)),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .grid()
                                    .grid_cols(3)
                                    .gap_1()
                                    .child(resource_load_badge(palette, "Load1", stats.load.load1))
                                    .child(resource_load_badge(palette, "Load5", stats.load.load5))
                                    .child(resource_load_badge(
                                        palette,
                                        "Load15",
                                        stats.load.load15,
                                    )),
                            )
                            .when(!stats.cpu.per_core.is_empty(), |this| {
                                this.child(cpu_core_summary(
                                    palette,
                                    &stats.cpu.per_core,
                                    self.stats_cpu_expanded,
                                    cx,
                                ))
                            }),
                    ))
                    .child(resource_section_card(
                        palette,
                        "Memory",
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .child(resource_ring_gauge(
                                        palette,
                                        memory_percent,
                                        format!("{memory_percent:.0}%"),
                                    ))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex_1()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_baseline()
                                                    .justify_between()
                                                    .gap_2()
                                                    .child(
                                                        div()
                                                            .text_size(px(11.))
                                                            .text_color(rgb(palette.text_muted))
                                                            .child("RAM"),
                                                    )
                                                    .child(
                                                        div()
                                                            .font_family(
                                                                crate::features::gpui_code_font_family(),
                                                            )
                                                            .text_size(px(13.))
                                                            .font_weight(FontWeight(700.))
                                                            .text_color(usage_color(
                                                                palette,
                                                                memory_percent / 100.,
                                                            ))
                                                            .child(format!("{memory_percent:.0}%")),
                                                    ),
                                            )
                                            .child(resource_progress_bar(
                                                palette,
                                                memory_percent / 100.,
                                            ))
                                            .child(
                                                div()
                                                    .font_family(
                                                        crate::features::gpui_code_font_family(),
                                                    )
                                                    .text_size(px(10.))
                                                    .text_color(rgb(palette.text_muted))
                                                    .child(format!(
                                                        "{} / {}",
                                                        format_file_size(Some(stats.memory.used)),
                                                        format_file_size(Some(memory_total))
                                                    )),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_wrap()
                                    .gap_x_3()
                                    .gap_y_1()
                                    .child(resource_metric_chip(
                                        palette,
                                        "Available",
                                        format_file_size(Some(stats.memory.available)),
                                    ))
                                    .child(resource_metric_chip(
                                        palette,
                                        "Cached",
                                        format_file_size(Some(stats.memory.cached)),
                                    )),
                            ),
                    ))
                    .child(resource_section_card(palette, "Network", network_rows))
                    .child(resource_section_card(palette, "Disk", disk_rows)),
            )
    }
}

fn resource_section_card(
    palette: crate::theme::ThemePalette,
    title: &'static str,
    child: impl IntoElement,
) -> gpui::Div {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.bg))
        .px_3()
        .py_2()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .size(px(6.))
                        .rounded_full()
                        .bg(rgb(palette.text_muted)),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(palette.text))
                        .child(title),
                ),
        )
        .child(child)
}

fn resource_info_cell(
    palette: crate::theme::ThemePalette,
    label: &'static str,
    value: impl Into<String>,
) -> gpui::Div {
    div()
        .min_w_0()
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(palette.text_dimmed))
                .child(label),
        )
        .child(
            div()
                .font_family(crate::features::gpui_code_font_family())
                .text_size(px(12.))
                .font_weight(FontWeight(600.))
                .text_color(rgb(palette.text))
                .overflow_hidden()
                .child(truncate_preview(&value.into(), 42)),
        )
}

fn resource_ring_gauge(
    palette: crate::theme::ThemePalette,
    percent: f64,
    label: String,
) -> gpui::Div {
    let ratio = (percent / 100.).clamp(0., 1.);
    div()
        .size(px(56.))
        .rounded_full()
        .border_1()
        .border_color(usage_color(palette, ratio))
        .bg(rgb(palette.surface))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .font_family(crate::features::gpui_code_font_family())
                .text_size(px(12.))
                .font_weight(FontWeight(700.))
                .text_color(usage_color(palette, ratio))
                .child(label),
        )
}

fn resource_load_badge(
    palette: crate::theme::ThemePalette,
    label: &'static str,
    value: f64,
) -> gpui::Div {
    div()
        .min_w_0()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .px_2()
        .py_1()
        .text_center()
        .child(
            div()
                .font_family(crate::features::gpui_code_font_family())
                .text_size(px(12.))
                .font_weight(FontWeight(700.))
                .text_color(rgb(palette.text))
                .overflow_hidden()
                .child(format!("{value:.2}")),
        )
        .child(
            div()
                .mt(px(2.))
                .text_size(px(9.))
                .text_color(rgb(palette.text_dimmed))
                .overflow_hidden()
                .child(label),
        )
}

fn resource_metric_chip(
    palette: crate::theme::ThemePalette,
    label: &'static str,
    value: String,
) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap_1()
        .child(
            div()
                .text_size(px(10.))
                .text_color(rgb(palette.text_dimmed))
                .child(label),
        )
        .child(
            div()
                .font_family(crate::features::gpui_code_font_family())
                .text_size(px(11.))
                .text_color(rgb(palette.text_muted))
                .child(value),
        )
}

fn resource_network_row(
    palette: crate::theme::ThemePalette,
    nic: &str,
    tx: f64,
    rx: f64,
) -> gpui::Div {
    div()
        .py_2()
        .border_b_1()
        .border_color(rgb(palette.border))
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .min_w_0()
                .flex_1()
                .font_family(crate::features::gpui_code_font_family())
                .text_size(px(12.))
                .font_weight(FontWeight(600.))
                .text_color(rgb(palette.text))
                .overflow_hidden()
                .child(truncate_preview(nic, 34)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap_2()
                .flex_wrap()
                .child(rate_value(palette, "↑", tx, rgb(0x22c55e).into()))
                .child(rate_value(palette, "↓", rx, rgb(0x3b82f6).into())),
        )
}

fn resource_disk_row(
    palette: crate::theme::ThemePalette,
    mount: &str,
    total: u64,
    available: u64,
    use_percent: u32,
) -> gpui::Div {
    let ratio = use_percent as f64 / 100.;
    div()
        .py_2()
        .border_b_1()
        .border_color(rgb(palette.border))
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .flex()
                .items_baseline()
                .justify_between()
                .gap_2()
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .font_family(crate::features::gpui_code_font_family())
                        .text_size(px(12.))
                        .font_weight(FontWeight(600.))
                        .text_color(rgb(palette.text))
                        .overflow_hidden()
                        .child(truncate_preview(mount, 42)),
                )
                .child(
                    div()
                        .font_family(crate::features::gpui_code_font_family())
                        .text_size(px(12.))
                        .font_weight(FontWeight(700.))
                        .text_color(usage_color(palette, ratio))
                        .child(format!("{use_percent}%")),
                ),
        )
        .child(resource_progress_bar(palette, ratio))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_x_2()
                .gap_y_1()
                .child(
                    div()
                        .font_family(crate::features::gpui_code_font_family())
                        .text_size(px(10.))
                        .text_color(rgb(palette.text_dimmed))
                        .child(format_file_size(Some(total))),
                )
                .child(resource_metric_chip(
                    palette,
                    "Available",
                    format_file_size(Some(available)),
                )),
        )
}

fn rate_value(
    palette: crate::theme::ThemePalette,
    arrow: &'static str,
    value: f64,
    color: gpui::Hsla,
) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap_1()
        .font_family(crate::features::gpui_code_font_family())
        .text_size(px(11.))
        .text_color(rgb(palette.text_muted))
        .child(div().text_color(color).child(arrow))
        .child(format_rate(value))
}

fn resource_empty_value(palette: crate::theme::ThemePalette) -> gpui::Div {
    div()
        .py_2()
        .text_size(px(12.))
        .text_color(rgb(palette.text_dimmed))
        .child("-")
}

fn resource_progress_bar(palette: crate::theme::ThemePalette, ratio: f64) -> impl IntoElement {
    stats_progress_bar(palette, ratio)
}

fn cpu_core_summary(
    palette: crate::theme::ThemePalette,
    per_core: &[f64],
    expanded: bool,
    cx: &mut Context<NyaTermApp>,
) -> gpui::Div {
    let visible_count = if expanded {
        per_core.len()
    } else {
        per_core.len().min(8)
    };
    let overflow = per_core.len().saturating_sub(visible_count);
    let summary = if overflow > 0 {
        format!("{} CPU +{overflow}", per_core.len())
    } else {
        format!("{} CPU", per_core.len())
    };

    let mut rows = div().flex().flex_col().gap_1().child(
        div()
            .id(SharedString::from("stats-cpu-cores-toggle"))
            .flex()
            .items_center()
            .gap_1()
            .text_size(px(11.))
            .text_color(rgb(palette.text_muted))
            .cursor_pointer()
            .hover(|this| this.bg(rgb(palette.input)))
            .on_click(cx.listener(|this, _, _, cx| {
                this.toggle_stats_cpu_expanded(cx);
            }))
            .child(svg().size(px(13.)).path("icons/chevron-down.svg"))
            .child(summary),
    );

    if expanded {
        let mut core_rows = div().flex().flex_col().gap_1().pt_1();
        for (index, usage) in per_core.iter().copied().enumerate() {
            core_rows = core_rows.child(cpu_core_row(palette, index + 1, usage));
        }
        rows = rows.child(core_rows);
    }

    rows
}

fn cpu_core_row(palette: crate::theme::ThemePalette, index: usize, usage: f64) -> gpui::Div {
    let ratio = (usage / 100.).clamp(0., 1.);
    div()
        .h(px(22.))
        .flex()
        .items_center()
        .gap_1()
        .child(
            div()
                .w(px(24.))
                .text_right()
                .font_family(crate::features::gpui_code_font_family())
                .text_size(px(10.))
                .text_color(rgb(palette.text_muted))
                .child(index.to_string()),
        )
        .child(
            div()
                .size(px(6.))
                .rounded_full()
                .bg(usage_color(palette, ratio)),
        )
        .child(div().flex_1().child(resource_progress_bar(palette, ratio)))
        .child(
            div()
                .w(px(44.))
                .text_right()
                .font_family(crate::features::gpui_code_font_family())
                .text_size(px(10.))
                .text_color(rgb(palette.text_muted))
                .child(format!("{usage:.1}%")),
        )
}

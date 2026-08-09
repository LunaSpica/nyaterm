use gpui::{Context, FontWeight, IntoElement, SharedString, div, prelude::*, px, rgb, svg};
use nyaterm_core::truncate_preview;
use nyaterm_transport::{RemoteGpuOverview, RemoteNpuOverview};

use crate::features::{
    NyaTermApp, format_file_size, format_rate, format_uptime, gpui_code_font_family,
    stats_progress_bar,
};
use crate::widgets::empty_panel;

use super::process::usage_color;

#[derive(Clone, Copy)]
struct ResourceRowPosition {
    first: bool,
    last: bool,
}

impl NyaTermApp {
    pub(in crate::features) fn stats_view(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let stats_state = self.remote_ops.stats_presentation();
        let palette = self.theme_palette();
        if self.session.active_ssh_config().is_none() {
            return div()
                .size_full()
                .bg(self.shell_transparent_color(palette.surface))
                .child(empty_panel(
                    self.tr("panel.resourceMonitorNoSession"),
                    palette,
                ));
        }
        let Some(stats) = stats_state.data else {
            let message = if stats_state.pending {
                self.tr("common.loading")
            } else if stats_state.status.contains("failed") {
                self.tr("panel.resourceMonitorError")
            } else {
                self.tr("common.loading")
            };
            return div()
                .size_full()
                .bg(self.shell_transparent_color(palette.surface))
                .child(empty_panel(message, palette));
        };

        let memory_total = stats.memory.used.saturating_add(stats.memory.available);
        let memory_percent = if memory_total > 0 {
            stats.memory.used as f64 / memory_total as f64 * 100.
        } else {
            0.
        };
        let system_label = self.tr("resourceMonitor.system").to_string();
        let hostname_label = self.tr("resourceMonitor.hostname").to_string();
        let arch_label = self.tr("resourceMonitor.arch").to_string();
        let os_label = self.tr("resourceMonitor.os").to_string();
        let uptime_label = self.tr("resourceMonitor.uptime").to_string();
        let cpu_label = self.tr("resourceMonitor.cpu").to_string();
        let cpu_average_label = self.tr("resourceMonitor.cpuAvgUsage").to_string();
        let load_1_label = self.tr("resourceMonitor.Load1").to_string();
        let load_5_label = self.tr("resourceMonitor.Load5").to_string();
        let load_15_label = self.tr("resourceMonitor.Load15").to_string();
        let memory_label = self.tr("resourceMonitor.memory").to_string();
        let available_label = self.tr("resourceMonitor.available").to_string();
        let cached_label = self.tr("resourceMonitor.cached").to_string();
        let gpu_label = self.tr("resourceMonitor.gpu").to_string();
        let npu_label = self.tr("resourceMonitor.npu").to_string();
        let driver_label = self.tr("resourceMonitor.driver").to_string();
        let cuda_label = self.tr("resourceMonitor.cuda").to_string();
        let cann_label = self.tr("resourceMonitor.cann").to_string();
        let temp_label = self.tr("resourceMonitor.temperature").to_string();
        let power_label = self.tr("resourceMonitor.power").to_string();
        let processes_label = self.tr("resourceMonitor.processes").to_string();
        let network_label = self.tr("resourceMonitor.network").to_string();
        let disk_label = self.tr("resourceMonitor.disk").to_string();
        let gpu_state = self.remote_ops.gpu_presentation();
        let npu_state = self.remote_ops.npu_presentation();
        let accelerator_common = ResourceAcceleratorCommonLabels {
            memory: memory_label.clone(),
            temp: temp_label,
            power: power_label,
            processes: processes_label,
            loading: self.tr("common.loading").to_string(),
        };

        let mut network_rows = div().flex().flex_col();
        if stats.networks.is_empty() {
            network_rows = network_rows.child(resource_empty_value(palette));
        } else {
            let total = stats.networks.len();
            for (index, network) in stats.networks.iter().enumerate() {
                network_rows = network_rows.child(resource_network_row(
                    palette,
                    &network.nic,
                    network.tx_bytes_per_sec,
                    network.rx_bytes_per_sec,
                    ResourceRowPosition {
                        first: index == 0,
                        last: index + 1 == total,
                    },
                ));
            }
        }

        let mut disk_rows = div().flex().flex_col();
        if stats.disks.is_empty() {
            disk_rows = disk_rows.child(resource_empty_value(palette));
        } else {
            let total = stats.disks.len();
            for (index, disk) in stats.disks.iter().enumerate() {
                disk_rows = disk_rows.child(resource_disk_row(
                    palette,
                    &disk.mount,
                    disk.total,
                    disk.available,
                    disk.use_percent,
                    available_label.clone(),
                    ResourceRowPosition {
                        first: index == 0,
                        last: index + 1 == total,
                    },
                ));
            }
        }

        div()
            .size_full()
            .overflow_hidden()
            .bg(self.shell_transparent_color(palette.surface))
            .child(
                div()
                    .id(SharedString::from("stats-scroll"))
                    .size_full()
                    .overflow_scroll()
                    .scrollbar_width(px(6.))
                    .p(px(10.))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(resource_section_card(
                        palette,
                        system_label,
                        div()
                            .grid()
                            .grid_cols(2)
                            .gap_x_3()
                            .gap_y_1()
                            .child(resource_info_cell(
                                palette,
                                hostname_label,
                                if stats.system.hostname.trim().is_empty() {
                                    "remote".to_string()
                                } else {
                                    stats.system.hostname.clone()
                                },
                            ))
                            .child(resource_info_cell(
                                palette,
                                arch_label,
                                stats.system.arch.clone(),
                            ))
                            .child(resource_info_cell(palette, os_label, stats.system.os.clone()))
                            .child(resource_info_cell(
                                palette,
                                uptime_label,
                                format_uptime(stats.system.uptime_sec),
                            )),
                    ))
                    .child(resource_section_card(
                        palette,
                        cpu_label.clone(),
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
                                                            .child(cpu_average_label),
                                                    )
                                                    .child(
                                                        div()
                                                            .font_family(
                                                                gpui_code_font_family(),
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
                                    .child(resource_load_badge(
                                        palette,
                                        load_1_label,
                                        stats.load.load1,
                                    ))
                                    .child(resource_load_badge(
                                        palette,
                                        load_5_label,
                                        stats.load.load5,
                                    ))
                                    .child(resource_load_badge(
                                        palette,
                                        load_15_label,
                                        stats.load.load15,
                                    )),
                            )
                            .when(!stats.cpu.per_core.is_empty(), |this| {
                                this.child(cpu_core_summary(
                                    palette,
                                    &stats.cpu.per_core,
                                    stats_state.cpu_expanded,
                                    cpu_label,
                                    cx,
                                ))
                            }),
                    ))
                    .child(resource_section_card(
                        palette,
                        memory_label,
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
                                        available_label,
                                        format_file_size(Some(stats.memory.available)),
                                    ))
                                    .child(resource_metric_chip(
                                        palette,
                                        cached_label,
                                        format_file_size(Some(stats.memory.cached)),
                                    )),
                            ),
                    ))
                    .when(self.settings.summary().ui_show_gpu_monitor, |this| {
                        this.child(resource_section_card(
                            palette,
                            gpu_label,
                            resource_gpu_rows(
                                palette,
                                gpu_state.data.as_ref(),
                                &gpu_state.status,
                                gpu_state.pending,
                                ResourceGpuLabels {
                                    common: accelerator_common.clone(),
                                    driver: driver_label.clone(),
                                    cuda: cuda_label,
                                },
                            ),
                        ))
                    })
                    .when(
                        self.settings.summary().ui_show_ascend_npu_monitor,
                        |this| {
                            this.child(resource_section_card(
                                palette,
                                npu_label,
                                resource_npu_rows(
                                    palette,
                                    npu_state.data.as_ref(),
                                    &npu_state.status,
                                    npu_state.pending,
                                    ResourceNpuLabels {
                                        common: accelerator_common,
                                        driver: driver_label,
                                        cann: cann_label,
                                    },
                                ),
                            ))
                        },
                    )
                    .child(resource_section_card(palette, network_label, network_rows))
                    .child(resource_section_card(palette, disk_label, disk_rows)),
            )
    }
}

#[derive(Clone)]
struct ResourceAcceleratorCommonLabels {
    memory: String,
    temp: String,
    power: String,
    processes: String,
    loading: String,
}

struct ResourceGpuLabels {
    common: ResourceAcceleratorCommonLabels,
    driver: String,
    cuda: String,
}

struct ResourceNpuLabels {
    common: ResourceAcceleratorCommonLabels,
    driver: String,
    cann: String,
}

fn resource_section_card(
    palette: crate::theme::ThemePalette,
    title: String,
    child: impl IntoElement,
) -> gpui::Div {
    div()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.bg))
        .px_3()
        .py(px(10.))
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
    label: String,
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
    let track = rgb(palette.border);
    let accent = usage_color(palette, ratio);
    div()
        .relative()
        .size(px(56.))
        .rounded_full()
        .bg(rgb(palette.surface))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .child(
            gpui::canvas(
                move |_, _, _| {},
                move |bounds, _, window, _| {
                    let width = f32::from(bounds.size.width);
                    let height = f32::from(bounds.size.height);
                    let center = gpui::point(
                        bounds.origin.x + px(width / 2.),
                        bounds.origin.y + px(height / 2.),
                    );
                    let radius = width.min(height) / 2. - 3.;
                    if let Some(path) = resource_ring_path(center, radius, 1.) {
                        window.paint_path(path, track);
                    }
                    if ratio > 0.
                        && let Some(path) = resource_ring_path(center, radius, ratio as f32)
                    {
                        window.paint_path(path, accent);
                    }
                },
            )
            .absolute()
            .inset_0(),
        )
        .child(
            div()
                .font_family(crate::features::gpui_code_font_family())
                .text_size(px(12.))
                .font_weight(FontWeight(700.))
                .text_color(usage_color(palette, ratio))
                .child(label),
        )
}

fn resource_ring_path(
    center: gpui::Point<gpui::Pixels>,
    radius: f32,
    ratio: f32,
) -> Option<gpui::Path<gpui::Pixels>> {
    let ratio = ratio.clamp(0., 1.);
    let segments = (64. * ratio).ceil().max(1.) as usize;
    let mut builder = gpui::PathBuilder::stroke(px(5.));
    for index in 0..=segments {
        let progress = index as f32 / segments as f32 * ratio;
        let angle = -std::f32::consts::FRAC_PI_2 + progress * std::f32::consts::TAU;
        let point = gpui::point(
            center.x + px(angle.cos() * radius),
            center.y + px(angle.sin() * radius),
        );
        if index == 0 {
            builder.move_to(point);
        } else {
            builder.line_to(point);
        }
    }
    builder.build().ok()
}

fn resource_load_badge(
    palette: crate::theme::ThemePalette,
    label: String,
    value: f64,
) -> gpui::Div {
    div()
        .min_w_0()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .px_2()
        .py(px(6.))
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
    label: String,
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
    position: ResourceRowPosition,
) -> gpui::Div {
    let ResourceRowPosition { first, last } = position;
    div()
        .when(!first, |this| this.pt_2())
        .when(!last, |this| {
            this.pb_2().border_b_1().border_color(rgb(palette.border))
        })
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
                .child(rate_value(
                    palette,
                    "icons/fe/up.svg",
                    tx,
                    rgb(0x22c55e).into(),
                ))
                .child(rate_value(
                    palette,
                    "icons/arrow-down.svg",
                    rx,
                    rgb(0x3b82f6).into(),
                )),
        )
}

fn resource_disk_row(
    palette: crate::theme::ThemePalette,
    mount: &str,
    total: u64,
    available: u64,
    use_percent: u32,
    available_label: String,
    position: ResourceRowPosition,
) -> gpui::Div {
    let ResourceRowPosition { first, last } = position;
    let ratio = use_percent as f64 / 100.;
    div()
        .when(!first, |this| this.pt_2())
        .when(!last, |this| {
            this.pb_2().border_b_1().border_color(rgb(palette.border))
        })
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
                    available_label,
                    format_file_size(Some(available)),
                )),
        )
}

fn resource_gpu_rows(
    palette: crate::theme::ThemePalette,
    overview: Option<&RemoteGpuOverview>,
    status: &str,
    pending: bool,
    labels: ResourceGpuLabels,
) -> gpui::Div {
    let Some(overview) = overview else {
        return resource_status_value(palette, pending_status(status, pending, &labels.common));
    };
    if !overview.available {
        return resource_status_value(palette, status_value(status));
    }

    let mut rows = div().flex().flex_col().gap_2();
    rows = rows.child(
        div()
            .flex()
            .flex_wrap()
            .gap_x_3()
            .gap_y_1()
            .when(!overview.driver_version.trim().is_empty(), |this| {
                this.child(resource_metric_chip(
                    palette,
                    labels.driver.clone(),
                    overview.driver_version.clone(),
                ))
            })
            .when(!overview.cuda_version.trim().is_empty(), |this| {
                this.child(resource_metric_chip(
                    palette,
                    labels.cuda.clone(),
                    overview.cuda_version.clone(),
                ))
            }),
    );

    if overview.gpus.is_empty() {
        return rows.child(resource_empty_value(palette));
    }

    let total = overview.gpus.len();
    for (index, gpu) in overview.gpus.iter().enumerate() {
        let process_count = overview
            .processes
            .iter()
            .filter(|process| process.gpu_index == Some(gpu.index))
            .count();
        rows = rows.child(resource_accelerator_row(
            palette,
            &format!("{} {}", gpu.index, gpu.name),
            gpu.utilization_gpu_percent,
            gpu.memory_used_mb,
            gpu.memory_total_mb,
            gpu.temperature_c,
            gpu.power_draw_w,
            process_count,
            labels.common.clone(),
            ResourceRowPosition {
                first: index == 0,
                last: index + 1 == total,
            },
        ));
    }
    rows
}

fn resource_npu_rows(
    palette: crate::theme::ThemePalette,
    overview: Option<&RemoteNpuOverview>,
    status: &str,
    pending: bool,
    labels: ResourceNpuLabels,
) -> gpui::Div {
    let Some(overview) = overview else {
        return resource_status_value(palette, pending_status(status, pending, &labels.common));
    };
    if !overview.available {
        return resource_status_value(palette, status_value(status));
    }

    let mut rows = div().flex().flex_col().gap_2();
    rows = rows.child(
        div()
            .flex()
            .flex_wrap()
            .gap_x_3()
            .gap_y_1()
            .when(!overview.driver_version.trim().is_empty(), |this| {
                this.child(resource_metric_chip(
                    palette,
                    labels.driver.clone(),
                    overview.driver_version.clone(),
                ))
            })
            .when(!overview.cann_version.trim().is_empty(), |this| {
                this.child(resource_metric_chip(
                    palette,
                    labels.cann.clone(),
                    overview.cann_version.clone(),
                ))
            }),
    );

    if overview.npus.is_empty() {
        return rows.child(resource_empty_value(palette));
    }

    let total = overview.npus.len();
    for (index, npu) in overview.npus.iter().enumerate() {
        let process_count = overview
            .processes
            .iter()
            .filter(|process| process.npu_index == npu.index && process.chip_id == npu.chip_id)
            .count();
        rows = rows.child(resource_accelerator_row(
            palette,
            &format!("{} {}", npu.index, npu.name),
            npu.utilization_aicore_percent,
            npu.memory_used_mb,
            npu.memory_total_mb,
            npu.temperature_c,
            npu.power_draw_w,
            process_count,
            labels.common.clone(),
            ResourceRowPosition {
                first: index == 0,
                last: index + 1 == total,
            },
        ));
    }
    rows
}

fn resource_accelerator_row(
    palette: crate::theme::ThemePalette,
    name: &str,
    utilization_percent: Option<f64>,
    memory_used_mb: u64,
    memory_total_mb: u64,
    temperature_c: Option<f64>,
    power_draw_w: Option<f64>,
    process_count: usize,
    labels: ResourceAcceleratorCommonLabels,
    position: ResourceRowPosition,
) -> gpui::Div {
    let ResourceRowPosition { first, last } = position;
    let ratio = utilization_percent.unwrap_or(0.) / 100.;
    div()
        .when(!first, |this| this.pt_1())
        .when(!last, |this| {
            this.pb_2().border_b_1().border_color(rgb(palette.border))
        })
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
                        .child(truncate_preview(name, 42)),
                )
                .child(
                    div()
                        .font_family(crate::features::gpui_code_font_family())
                        .text_size(px(12.))
                        .font_weight(FontWeight(700.))
                        .text_color(usage_color(palette, ratio))
                        .child(match utilization_percent {
                            Some(value) => format!("{value:.0}%"),
                            None => "-".to_string(),
                        }),
                ),
        )
        .child(resource_progress_bar(palette, ratio))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap_x_3()
                .gap_y_1()
                .child(resource_metric_chip(
                    palette,
                    labels.memory,
                    format!("{memory_used_mb}/{memory_total_mb} MiB"),
                ))
                .when_some(temperature_c, |this, value| {
                    this.child(resource_metric_chip(
                        palette,
                        labels.temp.clone(),
                        format!("{value:.0}C"),
                    ))
                })
                .when_some(power_draw_w, |this, value| {
                    this.child(resource_metric_chip(
                        palette,
                        labels.power.clone(),
                        format!("{value:.0}W"),
                    ))
                })
                .child(resource_metric_chip(
                    palette,
                    labels.processes,
                    process_count.to_string(),
                )),
        )
}

fn pending_status(status: &str, pending: bool, labels: &ResourceAcceleratorCommonLabels) -> String {
    if pending {
        labels.loading.clone()
    } else {
        status_value(status)
    }
}

fn status_value(status: &str) -> String {
    if status.trim().is_empty() {
        "-".to_string()
    } else {
        status.to_string()
    }
}

fn resource_status_value(palette: crate::theme::ThemePalette, message: String) -> gpui::Div {
    div()
        .py_2()
        .text_size(px(12.))
        .text_color(rgb(palette.text_dimmed))
        .child(message)
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
        .child(crate::features::mono_icon(arrow, color, 11.))
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
    cpu_label: String,
    cx: &mut Context<NyaTermApp>,
) -> gpui::Div {
    let visible_count = if expanded {
        per_core.len()
    } else {
        per_core.len().min(8)
    };
    let overflow = per_core.len().saturating_sub(visible_count);
    let summary = if overflow > 0 {
        format!("{} {cpu_label} +{overflow}", per_core.len())
    } else {
        format!("{} {cpu_label}", per_core.len())
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
            .child(
                svg()
                    .size(px(13.))
                    .path("icons/chevron-down.svg")
                    .text_color(rgb(palette.text_muted)),
            )
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

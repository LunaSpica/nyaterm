use super::*;
use gpui::{ScrollDelta, ScrollWheelEvent, SharedString, prelude::*};

const DOCKER_RESOURCE_ROW_PX: f32 = 68.; // 64px Tauri row + gap
const DOCKER_RESOURCE_VIEWPORT_ROWS: usize = 14;
const DOCKER_RESOURCE_OVERSCAN: usize = 6;

pub(in crate::ui::view::pages::remote) fn docker_images_panel(
    images: &[DockerImage],
    list_offset: usize,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    if images.is_empty() {
        return docker_resource_empty("Images", "No images loaded.");
    }

    let total = images.len();
    let (window_start, window_end, pad_top, pad_bottom, scroll_offset) =
        docker_resource_window(total, list_offset);
    let mut rows = div().flex().flex_col().gap_1();
    if pad_top > 0. {
        rows = rows.child(div().h(px(pad_top)).w_full().flex_none());
    }
    for image in images.get(window_start..window_end).unwrap_or(&[]) {
        let image_id = image.id.clone();
        let label = docker_image_label(image);
        rows = rows.child(
            docker_resource_row(
                label.clone(),
                format!(
                    "{} · {} · {}",
                    compact_id(&image.id),
                    image.created_since,
                    image.size
                ),
            )
            .child(icon_button(
                format!("docker-image-remove-{}", compact_id(&image_id)),
                "×", super::cx_theme_palette(cx),cx.listener(move |this, _, _, cx| {
                    this.request_docker_confirm(
                        DockerConfirmState {
                            title: format!("Remove image {label}"),
                            detail: format!("docker image rm {}", compact_id(&image_id)),
                            action: DockerConfirmAction::ImageRemove {
                                image_id: image_id.clone(),
                                force: false,
                            },
                        },
                        cx,
                    );
                }),
            )),
        );
    }
    if pad_bottom > 0. {
        rows = rows.child(div().h(px(pad_bottom)).w_full().flex_none());
    }
    if total > DOCKER_RESOURCE_VIEWPORT_ROWS {
        rows = rows.child(docker_resource_range_footer(window_start, window_end, total));
    }

    docker_resource_panel("Images", total, rows, scroll_offset, cx)
}

pub(in crate::ui::view::pages::remote) fn docker_volumes_panel(
    volumes: &[DockerVolume],
    list_offset: usize,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    if volumes.is_empty() {
        return docker_resource_empty("Volumes", "No volumes loaded.");
    }

    let total = volumes.len();
    let (window_start, window_end, pad_top, pad_bottom, scroll_offset) =
        docker_resource_window(total, list_offset);
    let mut rows = div().flex().flex_col().gap_1();
    if pad_top > 0. {
        rows = rows.child(div().h(px(pad_top)).w_full().flex_none());
    }
    for volume in volumes.get(window_start..window_end).unwrap_or(&[]) {
        let volume_name = volume.name.clone();
        rows = rows.child(
            docker_resource_row(volume.name.clone(), format!("driver {}", volume.driver)).child(
                icon_button(
                    format!("docker-volume-remove-{volume_name}"),
                    "×", super::cx_theme_palette(cx),cx.listener(move |this, _, _, cx| {
                        this.request_docker_confirm(
                            DockerConfirmState {
                                title: format!("Remove volume {volume_name}"),
                                detail: format!("docker volume rm {volume_name}"),
                                action: DockerConfirmAction::VolumeRemove {
                                    volume_name: volume_name.clone(),
                                    force: false,
                                },
                            },
                            cx,
                        );
                    }),
                ),
            ),
        );
    }
    if pad_bottom > 0. {
        rows = rows.child(div().h(px(pad_bottom)).w_full().flex_none());
    }
    if total > DOCKER_RESOURCE_VIEWPORT_ROWS {
        rows = rows.child(docker_resource_range_footer(window_start, window_end, total));
    }

    docker_resource_panel("Volumes", total, rows, scroll_offset, cx)
}

pub(in crate::ui::view::pages::remote) fn docker_networks_panel(
    networks: &[DockerNetwork],
    list_offset: usize,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    if networks.is_empty() {
        return docker_resource_empty("Networks", "No networks loaded.");
    }

    let total = networks.len();
    let (window_start, window_end, pad_top, pad_bottom, scroll_offset) =
        docker_resource_window(total, list_offset);
    let mut rows = div().flex().flex_col().gap_1();
    if pad_top > 0. {
        rows = rows.child(div().h(px(pad_top)).w_full().flex_none());
    }
    for network in networks.get(window_start..window_end).unwrap_or(&[]) {
        let network_id = network.id.clone();
        let name = network.name.clone();
        rows = rows.child(
            docker_resource_row(
                network.name.clone(),
                format!(
                    "{} · {} · {}",
                    compact_id(&network.id),
                    network.driver,
                    network.scope
                ),
            )
            .child(icon_button(
                format!("docker-network-remove-{}", compact_id(&network_id)),
                "×", super::cx_theme_palette(cx),cx.listener(move |this, _, _, cx| {
                    this.request_docker_confirm(
                        DockerConfirmState {
                            title: format!("Remove network {name}"),
                            detail: format!("docker network rm {}", compact_id(&network_id)),
                            action: DockerConfirmAction::NetworkRemove {
                                network_id: network_id.clone(),
                            },
                        },
                        cx,
                    );
                }),
            )),
        );
    }
    if pad_bottom > 0. {
        rows = rows.child(div().h(px(pad_bottom)).w_full().flex_none());
    }
    if total > DOCKER_RESOURCE_VIEWPORT_ROWS {
        rows = rows.child(docker_resource_range_footer(window_start, window_end, total));
    }

    docker_resource_panel("Networks", total, rows, scroll_offset, cx)
}

fn docker_resource_window(
    total: usize,
    list_offset: usize,
) -> (usize, usize, f32, f32, usize) {
    let window_capacity = DOCKER_RESOURCE_VIEWPORT_ROWS + DOCKER_RESOURCE_OVERSCAN * 2;
    let max_offset = total.saturating_sub(DOCKER_RESOURCE_VIEWPORT_ROWS.min(total));
    let scroll_row = list_offset.min(max_offset);
    let window_start = scroll_row.saturating_sub(DOCKER_RESOURCE_OVERSCAN);
    let window_end = (window_start + window_capacity).min(total);
    let pad_top = (window_start as f32) * DOCKER_RESOURCE_ROW_PX;
    let pad_bottom = ((total.saturating_sub(window_end)) as f32) * DOCKER_RESOURCE_ROW_PX;
    (window_start, window_end, pad_top, pad_bottom, scroll_row)
}

fn docker_resource_range_footer(start: usize, end: usize, total: usize) -> impl IntoElement {
    let palette = crate::ui::theme::theme_palette("github-dark");
    div()
        .px_2()
        .py_1()
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.surface_elevated))
        .bg(rgb(palette.bg))
        .text_size(px(10.))
        .text_color(rgb(palette.text_dimmed))
        .child(format!("Rows {start}-{end}/{total} · scroll or refine search"))
}

fn docker_resource_empty(title: &'static str, message: &'static str) -> gpui::AnyElement {
    div()
        .id(SharedString::from(format!(
            "docker-resource-{}",
            title.to_ascii_lowercase()
        )))
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(empty_panel(message, crate::ui::theme::theme_palette("github-dark")))
        .into_any_element()
}

pub(in crate::ui::view::pages::remote) fn docker_resource_panel(
    title: &'static str,
    count: usize,
    rows: impl IntoElement,
    _scroll_offset: usize,
    cx: &mut Context<NyaTermApp>,
) -> gpui::AnyElement {
        let palette = cx.entity().read(cx).theme_palette();
    // Tauri resource tabs: full-height virtual list + wheel offset.
    let _ = title;
    let total_for_scroll = count;
    div()
        .id(SharedString::from(format!(
            "docker-resource-{}",
            title.to_ascii_lowercase()
        )))
        .size_full()
        .overflow_hidden()
        .flex()
        .flex_col()
        .on_scroll_wheel(cx.listener(move |this, event: &ScrollWheelEvent, _, cx| {
            let max_offset = total_for_scroll
                .saturating_sub(DOCKER_RESOURCE_VIEWPORT_ROWS.min(total_for_scroll));
            if max_offset == 0 {
                return;
            }
            let delta_rows = match event.delta {
                ScrollDelta::Lines(delta) => delta.y,
                ScrollDelta::Pixels(delta) => f32::from(delta.y) / DOCKER_RESOURCE_ROW_PX,
            };
            let next = (this.docker_resource_list_offset as f32 - delta_rows)
                .round()
                .clamp(0., max_offset as f32) as usize;
            if next != this.docker_resource_list_offset {
                this.docker_resource_list_offset = next;
                cx.stop_propagation();
                cx.notify();
            }
        }))
        .child(
            div()
                .h(px(22.))
                .flex_none()
                .px_2()
                .pt_1()
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .text_size(px(10.))
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(palette.text_dimmed))
                        .child(format!("{title} · {count}")),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_h_0()
                .px_2()
                .pb_2()
                .flex()
                .flex_col()
                .child(rows),
        )
        .into_any_element()
}

pub(in crate::ui::view::pages::remote) fn docker_resource_static_panel(
    title: &'static str,
    count: usize,
    rows: impl IntoElement,
) -> impl IntoElement {
    let palette = crate::ui::theme::theme_palette("github-dark");
    div()
        .id(SharedString::from(format!(
            "docker-resource-{}",
            title.to_ascii_lowercase()
        )))
        .size_full()
        .overflow_scroll()
        .scrollbar_width(px(6.))
        .p_2()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .h(px(22.))
                .flex_none()
                .px_1()
                .flex()
                .items_center()
                .child(
                    div()
                        .text_size(px(10.))
                        .font_weight(FontWeight(700.))
                        .text_color(rgb(palette.text_dimmed))
                        .child(format!("{title} · {count}")),
                ),
        )
        .child(rows)
}

pub(in crate::ui::view::pages::remote) fn docker_resource_row(
    title: String,
    detail: String,
) -> gpui::Div {
    let palette = crate::ui::theme::theme_palette("github-dark");
    // ~64px Tauri SIMPLE_ROW_HEIGHT-ish dense resource row (slightly tighter chrome).
    div()
        .h(px(64.))
        .rounded_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.section_header))
        .px_3()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .hover(|this| this.bg(rgb(palette.hover)))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(2.))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight(600.))
                        .text_color(rgb(palette.text))
                        .overflow_hidden()
                        .child(truncate_preview(&title, 48)),
                )
                .child(
                    div()
                        .font_family("JetBrains Mono")
                        .text_size(px(10.))
                        .text_color(rgb(palette.text_dimmed))
                        .overflow_hidden()
                        .child(truncate_preview(&detail, 72)),
                ),
        )
}

pub(in crate::ui::view::pages::remote) fn docker_image_label(image: &DockerImage) -> String {
    match (
        image.repository.trim().is_empty(),
        image.tag.trim().is_empty(),
    ) {
        (true, true) => compact_id(&image.id),
        (false, true) => image.repository.clone(),
        (true, false) => image.tag.clone(),
        (false, false) => format!("{}:{}", image.repository, image.tag),
    }
}

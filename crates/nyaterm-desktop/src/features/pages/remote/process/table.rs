use super::*;

pub(in crate::features::pages::remote) fn process_sort_button(
    palette: ThemePalette,
    id: impl Into<String>,
    label: &str,
    active: bool,
    direction: RemoteProcessSortDirection,
    numeric: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    // Flat sortable header cell (Tauri table header).
    div()
        .id(gpui::SharedString::from(id.into()))
        .h_full()
        .min_w_0()
        .px_1()
        .flex()
        .items_center()
        .when(numeric, |this| this.justify_end())
        .rounded_sm()
        .text_size(px(10.))
        .font_weight(if active {
            FontWeight(700.)
        } else {
            FontWeight(600.)
        })
        .text_color(if active {
            rgb(palette.text)
        } else {
            rgb(palette.text_dimmed)
        })
        .cursor_pointer()
        .hover(|this| {
            this.bg(rgb(palette.surface_elevated))
                .text_color(rgb(palette.text))
        })
        .child(if active {
            format!("{label} {}", direction.marker())
        } else {
            label.to_string()
        })
        .on_click(on_click)
}

pub(in crate::features::pages::remote) fn process_table_header(
    palette: ThemePalette,
    labels: ProcessTableLabels,
) -> impl IntoElement {
    // Static fallback header; live header uses process_sort_button grid in process_view.
    div()
        .grid()
        .grid_cols(6)
        .gap_1()
        .h(px(26.))
        .flex_none()
        .border_b_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.bg))
        .px_2()
        .items_center()
        .text_size(px(10.))
        .font_weight(FontWeight(700.))
        .text_color(rgb(palette.text_dimmed))
        .child(labels.process)
        .child(div().text_right().child(labels.pid))
        .child(div().text_right().child(labels.cpu))
        .child(div().text_right().child(labels.memory))
        .child(labels.user)
        .child("")
}

pub(in crate::features::pages::remote) fn process_table_row(
    palette: ThemePalette,
    menu_bg: gpui::Rgba,
    process: &RemoteProcess,
    mode: ProcessDisplayMode,
    labels: ProcessTableLabels,
    selected: bool,
    menu_open: bool,
    on_select: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_menu: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_copy_pid: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_copy_command: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_term: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_hup: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_stop: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_cont: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_kill: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui::Div {
    // Tauri ProcessManager: left accent + mode-aware columns (compact/narrow/medium/wide).
    let accent = if process.cpu_percent >= 80.0 {
        rgb(palette.danger)
    } else if process.memory_percent >= 80.0 {
        rgb(palette.warning)
    } else if selected {
        rgb(0x1f6feb)
    } else {
        rgb(palette.border)
    };
    let show_memory = !matches!(
        mode,
        ProcessDisplayMode::Narrow | ProcessDisplayMode::Compact
    );
    let show_user = matches!(mode, ProcessDisplayMode::Wide);
    let cols = match mode {
        ProcessDisplayMode::Compact => 2,
        ProcessDisplayMode::Narrow => 4,
        ProcessDisplayMode::Medium => 5,
        ProcessDisplayMode::Wide => 6,
    };
    let row_h = process_row_height_px(mode);

    let menu = div()
        .relative()
        .flex()
        .items_center()
        .justify_end()
        .child(compact_remote_svg_button(
            palette,
            format!("process-menu-{}", process.pid),
            "icons/conn/more.svg",
            labels.more,
            on_menu,
        ))
        .when(menu_open, |this| {
            this.child(
                div()
                    .id(gpui::SharedString::from(format!(
                        "process-menu-pop-{}",
                        process.pid
                    )))
                    .absolute()
                    .top(px(26.))
                    .right_0()
                    .w(px(148.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(menu_bg)
                    .shadow_lg()
                    .py_1()
                    .flex()
                    .flex_col()
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, _| {})
                    .child(process_menu_item(
                        palette,
                        format!("process-copy-pid-{}", process.pid),
                        labels.copy_pid,
                        on_copy_pid,
                    ))
                    .child(process_menu_item(
                        palette,
                        format!("process-copy-cmd-{}", process.pid),
                        labels.copy_command,
                        on_copy_command,
                    ))
                    .child(process_menu_sep(palette))
                    .child(process_menu_item(
                        palette,
                        format!("process-term-{}", process.pid),
                        labels.signal_term,
                        on_term,
                    ))
                    .child(process_menu_item(
                        palette,
                        format!("process-hup-{}", process.pid),
                        labels.signal_hup,
                        on_hup,
                    ))
                    .child(process_menu_item(
                        palette,
                        format!("process-stop-{}", process.pid),
                        labels.signal_stop,
                        on_stop,
                    ))
                    .child(process_menu_item(
                        palette,
                        format!("process-cont-{}", process.pid),
                        labels.signal_cont,
                        on_cont,
                    ))
                    .child(process_menu_item(
                        palette,
                        format!("process-kill-{}", process.pid),
                        labels.signal_kill,
                        on_kill,
                    )),
            )
        });

    let body = if mode == ProcessDisplayMode::Compact {
        // Tauri CompactProcessRow: command + PID/CPU mono line + menu.
        div()
            .id(gpui::SharedString::from(format!(
                "process-row-{}",
                process.pid
            )))
            .h(px(row_h))
            .px_2()
            .pl(px(10.))
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .on_click(on_select)
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight(600.))
                            .text_color(rgb(palette.text))
                            .overflow_hidden()
                            .child(truncate_preview(&process.command, 36)),
                    )
                    .child(
                        div()
                            .font_family(crate::features::gpui_code_font_family())
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_dimmed))
                            .overflow_hidden()
                            .child(format!("PID {} · {:.1}%", process.pid, process.cpu_percent)),
                    ),
            )
            .child(menu)
    } else {
        let mut grid = div()
            .grid()
            .id(gpui::SharedString::from(format!(
                "process-row-{}",
                process.pid
            )))
            .grid_cols(cols)
            .gap_1()
            .h(px(row_h))
            .px_2()
            .pl(px(10.))
            .items_center()
            .cursor_pointer()
            .on_click(on_select)
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight(600.))
                            .text_color(rgb(palette.text))
                            .overflow_hidden()
                            .child(truncate_preview(&process.command, 40)),
                    )
                    .child(
                        div()
                            .font_family(crate::features::gpui_code_font_family())
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_dimmed))
                            .overflow_hidden()
                            .child(truncate_preview(&process.command_line, 52)),
                    ),
            )
            .child(process_table_cell(
                palette,
                process.pid.to_string(),
                None,
                true,
            ))
            .child(process_table_cell(
                palette,
                format!("{:.1}%", process.cpu_percent),
                Some(usage_color(palette, process.cpu_percent / 100.)),
                true,
            ));
        if show_memory {
            grid = grid.child(process_table_cell(
                palette,
                format!("{:.1}%", process.memory_percent),
                Some(usage_color(palette, process.memory_percent / 100.)),
                true,
            ));
        }
        if show_user {
            grid = grid.child(process_table_cell(
                palette,
                truncate_preview(&process.user, 12),
                None,
                false,
            ));
        }
        grid.child(menu)
    };

    div()
        .relative()
        .border_b_1()
        .border_color(rgb(palette.surface_elevated))
        .bg(if selected {
            rgb(palette.hover)
        } else {
            rgb(palette.surface)
        })
        .hover(|this| this.bg(rgb(palette.hover)))
        .child(
            div()
                .absolute()
                .left_0()
                .top_0()
                .bottom_0()
                .w(px(2.))
                .bg(accent),
        )
        .child(body)
}

#[derive(Clone, Copy)]
pub(in crate::features::pages::remote) struct ProcessTableLabels {
    pub process: &'static str,
    pub pid: &'static str,
    pub cpu: &'static str,
    pub memory: &'static str,
    pub user: &'static str,
    pub more: &'static str,
    pub copy_pid: &'static str,
    pub copy_command: &'static str,
    pub signal_term: &'static str,
    pub signal_hup: &'static str,
    pub signal_stop: &'static str,
    pub signal_cont: &'static str,
    pub signal_kill: &'static str,
}

fn process_menu_item(
    palette: ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(gpui::SharedString::from(id.into()))
        .h(px(24.))
        .px_3()
        .flex()
        .items_center()
        .text_size(px(12.))
        .text_color(rgb(palette.text))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface_elevated)))
        .on_click(on_click)
        .child(label)
}

fn process_menu_sep(palette: ThemePalette) -> impl IntoElement {
    div().h(px(1.)).mx_2().my_1().bg(rgb(palette.border))
}

pub(in crate::features::pages::remote) fn process_table_cell(
    palette: ThemePalette,
    value: String,
    color: Option<gpui::Hsla>,
    numeric: bool,
) -> impl IntoElement {
    // Tauri ProcessManager numeric columns are mono + right-aligned.
    div()
        .min_w_0()
        .font_family(crate::features::gpui_code_font_family())
        .text_xs()
        .when(numeric, |this| this.text_right())
        .text_color(color.unwrap_or_else(|| rgb(palette.text).into()))
        .overflow_hidden()
        .child(value)
}

pub(in crate::features::pages::remote) fn icon_action_button(
    palette: ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(gpui::SharedString::from(id.into()))
        .h(px(24.))
        .px_2()
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(rgb(palette.input))
        .text_color(rgb(palette.text))
        .text_xs()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.hover)))
        .child(label)
        .on_click(on_click)
}

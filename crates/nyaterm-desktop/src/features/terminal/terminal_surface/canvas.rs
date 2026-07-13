use super::*;

impl NyaTermApp {
    pub(in crate::features) fn terminal_canvas(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let session_id = self.active_session_id.clone().unwrap_or_default();
        self.terminal_canvas_for(session_id, false, cx)
    }

    pub(in crate::features) fn terminal_canvas_for(
        &mut self,
        session_id: String,
        show_pane_chrome: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.terminal_theme_palette();
        let keyword_rules = self.resolved_keyword_highlight_rules();
        let action_link_matchers = self.settings.terminal_action_links_matchers.clone();
        let is_active = self.active_session_id.as_deref() == Some(session_id.as_str());
        let is_disconnected = !session_id.is_empty() && self.is_session_disconnected(&session_id);
        let scroll_offset = self
            .terminal_views
            .get(&session_id)
            .map(|view| view.scroll_offset)
            .unwrap_or(self.terminal_scroll_offset);
        let snapshot = self
            .terminal_views
            .get(&session_id)
            .map(|view| view.screen.viewport_snapshot(scroll_offset))
            .unwrap_or_else(|| self.terminal_screen.viewport_snapshot(scroll_offset));
        let lines = snapshot.lines.clone();
        let line_timestamps_ms = snapshot.line_timestamps_ms.clone();
        let hyperlink_lines = snapshot.hyperlink_lines.clone();
        let cursor_row = snapshot.cursor_row;
        let show_line_numbers = self.settings.terminal_show_line_numbers;
        let show_timestamps = self.settings.terminal_show_timestamps;
        let show_timestamp_ms = self.settings.terminal_show_timestamp_milliseconds;
        let gutter_enabled = show_line_numbers || show_timestamps;
        let show_cursor = is_active
            && !session_id.is_empty()
            && !is_disconnected
            && scroll_offset == 0
            && cursor_row != usize::MAX
            && (!self.settings.cursor_blink || self.terminal_runtime.cursor_blink_on);
        let cursor_style = self.settings.cursor_style.as_str();
        let search_matches = if is_active
            && self.terminal_search_open
            && self.terminal_search_mode == TerminalSearchMode::Buffer
        {
            self.terminal_buffer_matches().unwrap_or_default()
        } else {
            Vec::new()
        };
        // Buffer matches use absolute history indices; map into current viewport rows.
        let (abs_start, abs_end) = self
            .terminal_views
            .get(&session_id)
            .map(|view| view.screen.viewport_absolute_range(scroll_offset))
            .unwrap_or_else(|| self.terminal_screen.viewport_absolute_range(scroll_offset));
        let active_match_abs = search_matches
            .get(
                self.terminal_search_active_index
                    .min(search_matches.len().saturating_sub(1)),
            )
            .map(|search_match| search_match.line_index);
        let mut search_ranges_by_line: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
        let mut active_search_ranges_by_line: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
        for (match_index, search_match) in search_matches.iter().enumerate() {
            let abs = search_match.line_index;
            if abs < abs_start || abs >= abs_end {
                continue;
            }
            let view_row = abs - abs_start;
            let range = (search_match.start_col, search_match.end_col);
            search_ranges_by_line
                .entry(view_row)
                .or_default()
                .push(range);
            if Some(abs) == active_match_abs
                && match_index
                    == self
                        .terminal_search_active_index
                        .min(search_matches.len().saturating_sub(1))
            {
                active_search_ranges_by_line
                    .entry(view_row)
                    .or_default()
                    .push(range);
            }
        }
        let mut line_decorations = Vec::with_capacity(lines.len());
        for (line_index, line) in lines.iter().enumerate() {
            let line = if line.is_empty() {
                " ".to_string()
            } else {
                line.clone()
            };
            let selection_cols = if is_active {
                self.terminal_selection
                    .as_ref()
                    .and_then(|selection| selection.cols_for_row(line_index))
            } else {
                None
            };
            let mut link_ranges: Vec<(usize, usize)> =
                if self.settings.terminal_action_links_enabled {
                    if let Some(view) = self.terminal_views.get_mut(&session_id) {
                        view.render_cache
                            .action_link_ranges(&line, &action_link_matchers)
                    } else {
                        find_action_links(&line, &action_link_matchers, true)
                            .into_iter()
                            .map(|item| {
                                let start_chars =
                                    line[..item.start.min(line.len())].chars().count();
                                let end_chars = line[..item.end.min(line.len())].chars().count();
                                (start_chars, end_chars)
                            })
                            .collect()
                    }
                } else {
                    Vec::new()
                };
            // OSC 8 hyperlinks from the terminal model (always paint when present).
            if let Some(spans) = hyperlink_lines.get(line_index) {
                for span in spans {
                    let start = span.start_col;
                    let end = span.end_col.saturating_add(1);
                    if end > start {
                        link_ranges.push((start, end));
                    }
                }
            }
            let empty_ranges: [(usize, usize); 0] = [];
            let line_search_ranges = search_ranges_by_line
                .get(&line_index)
                .map(|ranges| ranges.as_slice())
                .unwrap_or(&empty_ranges);
            let line_active_search_ranges = active_search_ranges_by_line
                .get(&line_index)
                .map(|ranges| ranges.as_slice())
                .unwrap_or(&empty_ranges);
            line_decorations.push(TerminalLineDecorations {
                search_ranges: line_search_ranges.to_vec(),
                active_search_ranges: line_active_search_ranges.to_vec(),
                selection_cols,
                link_ranges,
            });
        }
        let (cell_w, cell_h) = self.terminal_cell_size();
        let grid = NyaTerminalElement::new(
            snapshot,
            keyword_rules,
            line_decorations,
            show_cursor,
            cursor_style,
            cell_w,
            cell_h,
            palette,
            self.settings.terminal_font_family.clone(),
            self.settings.terminal_font_size as f32,
            self.settings.terminal_font_weight as f32,
            self.settings.terminal_font_weight_bold as f32,
        );
        let output = if gutter_enabled {
            let ts_w = self.terminal_timestamp_gutter_width_px();
            let ln_w = self.terminal_line_number_gutter_width_px();
            let mut gutter = div().flex().flex_col().flex_none();
            for line_index in 0..lines.len() {
                let ts_label = if show_timestamps {
                    line_timestamps_ms
                        .get(line_index)
                        .copied()
                        .flatten()
                        .map(|ms| format_terminal_line_timestamp_ms(ms, show_timestamp_ms))
                        .unwrap_or_else(|| {
                            if show_timestamp_ms {
                                "             ".to_string()
                            } else {
                                "          ".to_string()
                            }
                        })
                } else {
                    String::new()
                };
                let line_label = if show_line_numbers {
                    format!("{:>5}", abs_start + line_index + 1)
                } else {
                    String::new()
                };
                gutter = gutter.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .min_h(px(cell_h))
                        .gap_1()
                        .flex_none()
                        .pr_1()
                        .text_color(rgb(palette.text_dimmed))
                        .font_family(self.settings.terminal_font_family.clone())
                        .text_size(px(self.settings.terminal_font_size as f32 * 0.85))
                        .when(show_timestamps, |this| {
                            this.child(div().w(px(ts_w)).flex_none().child(ts_label))
                        })
                        .when(show_line_numbers, |this| {
                            this.child(div().w(px(ln_w)).flex_none().child(line_label))
                        }),
                );
            }
            div().flex().flex_row().child(gutter).child(grid)
        } else {
            div().flex().flex_row().child(grid)
        };
        let pane_title = self
            .session_display_name(&session_id)
            .unwrap_or_else(|| short_id(&session_id).to_string());
        let sync_group_label = self.active_sync_group_label(&session_id);
        let active_sync_group = self.active_sync_group_for_session(&session_id);
        let show_sync_action_overlay = active_sync_group.is_some() && !session_id.is_empty();
        let sync_is_paused = self.is_session_paused_in_active_sync_group(&session_id);
        let sync_group_color = active_sync_group
            .map(|group| group.color)
            .unwrap_or(palette.accent);
        let sync_status_label = if sync_is_paused { "Paused" } else { "Syncing" };
        let output_session_id = session_id.clone();
        let terminal_font_family = self.settings.terminal_font_family.clone();
        let terminal_font_size = self.settings.terminal_font_size as f32;
        let has_new_while_scrolled = self
            .terminal_views
            .get(&session_id)
            .is_some_and(|view| view.has_new_while_scrolled);
        let performance_overlay = self
            .terminal_views
            .get(&session_id)
            .and_then(|view| view.performance_overlay);
        let skipped_output_chars = self
            .terminal_views
            .get(&session_id)
            .map(|view| view.skipped_output_chars)
            .unwrap_or(0);
        let (render_cache_hits, render_cache_misses) = self
            .terminal_views
            .get(&session_id)
            .map(|view| (view.render_cache.hits, view.render_cache.misses))
            .unwrap_or((0, 0));
        let show_scroll_to_bottom = is_active && scroll_offset > 0;
        let show_visual_bell = is_active && self.terminal_runtime.visual_bell_ticks > 0;
        let file_drop_hover = self
            .terminal_file_drop_hover
            .as_deref()
            .is_some_and(|id| id == session_id.as_str());
        let drop_session_kind = self
            .ordered_sessions()
            .into_iter()
            .find(|s| s.id == session_id)
            .map(|s| session_kind_label(s.kind))
            .unwrap_or("Local");
        let (drop_title, drop_hint) = nyaterm_core::terminal_drop_overlay_copy(drop_session_kind);

        div()
            .flex_1()
            .min_h_0()
            .font_family(terminal_font_family)
            .text_size(px(terminal_font_size))
            .font_weight(FontWeight(self.settings.terminal_font_weight as f32))
            .text_color(rgb(palette.terminal_fg))
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .relative()
                    .bg(rgb(palette.terminal_bg))
                    .when(show_pane_chrome, |this| {
                        this.border_1()
                            .border_color(if is_active {
                                rgb(palette.accent)
                            } else {
                                rgb(palette.border)
                            })
                            .child(
                                div()
                                    .h(px(28.))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    .px_2()
                                    .border_b_1()
                                    .border_color(rgb(palette.border))
                                    .bg(if is_active {
                                        rgb(palette.hover)
                                    } else {
                                        rgb(palette.input)
                                    })
                                    .child(
                                        div()
                                            .min_w_0()
                                            .text_xs()
                                            .font_weight(FontWeight(800.))
                                            .text_color(if is_active {
                                                rgb(palette.text)
                                            } else {
                                                rgb(palette.text_muted)
                                            })
                                            .child(truncate_preview(&pane_title, 42)),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_1()
                                            .when_some(sync_group_label.clone(), |this, label| {
                                                this.child(
                                                    div()
                                                        .rounded_sm()
                                                        .px_2()
                                                        .py_1()
                                                        .text_xs()
                                                        .text_color(rgb(palette.accent))
                                                        .bg(rgb(palette.hover))
                                                        .child(truncate_preview(&label, 18)),
                                                )
                                            })
                                            .child(status_pill(
                                                if is_active { "active" } else { "pane" },
                                                if is_active {
                                                    rgb(palette.success)
                                                } else {
                                                    rgb(palette.accent)
                                                },
                                                rgb(palette.hover),
                                            )),
                                    ),
                            )
                    })
                    .track_focus(&self.terminal_focus)
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                        cx.stop_propagation();
                        this.mark_user_activity();
                        if this.handle_global_shortcut(event, window, cx) {
                            return;
                        }
                        if this.handle_credential_suggestion_key(event, cx) {
                            return;
                        }
                        if this.handle_command_suggestion_key(event, cx) {
                            return;
                        }
                        if this.handle_terminal_scroll_key(event, cx) {
                            return;
                        }
                        if this.handle_smart_input_selection_key(event, cx) {
                            return;
                        }
                        // Disconnected tab: Enter reconnects; other keys show status (Tauri).
                        if let Some(session_id) = this.active_session_id.clone() {
                            if this.is_session_disconnected(&session_id) {
                                let keystroke = &event.keystroke;
                                if !keystroke.modifiers.control
                                    && !keystroke.modifiers.platform
                                    && !keystroke.modifiers.alt
                                    && keystroke.key.as_str() == "enter"
                                {
                                    this.reconnect_session(session_id, window, cx);
                                } else if keystroke.key.as_str() == "d"
                                    && keystroke.modifiers.control
                                    && !keystroke.modifiers.platform
                                    && !keystroke.modifiers.alt
                                {
                                    // Ctrl+D closes disconnected tab (Tauri onDisconnectedClose).
                                    this.close_session(session_id, cx);
                                } else {
                                    this.terminal_status =
                                        "session disconnected — press Enter to reconnect"
                                            .to_string();
                                    cx.notify();
                                }
                                return;
                            }
                        }
                        let keystroke = &event.keystroke;
                        let primary = keystroke.modifiers.control || keystroke.modifiers.platform;
                        if primary
                            && !keystroke.modifiers.alt
                            && !keystroke.modifiers.function
                            && matches!(keystroke.key.as_str(), "v" | "V")
                        {
                            this.paste_from_clipboard(window, cx);
                            return;
                        }
                        if let Some(bytes) = this.terminal_key_bytes_for_event(event) {
                            // When a non-smart buffer selection is painted, still send
                            // keystrokes but skip suggestion tracking so the selection
                            // edit path stays isolated (Tauri preserves selection).
                            let has_buffer_selection = this.terminal_selection.is_some()
                                && this.smart_cursor_selected_input_range().is_none();
                            if has_buffer_selection {
                                this.send_terminal_input_without_suggestion_track(bytes, cx);
                            } else {
                                this.send_terminal_input(bytes, cx);
                            }
                        }
                    }))
                    .when(is_disconnected, |this| {
                        this.child(
                            div()
                                .h(px(26.))
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_2()
                                .px_3()
                                .border_b_1()
                                .border_color(rgb(palette.border))
                                .bg(rgb(palette.input))
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(palette.danger))
                                        .child("Session disconnected"),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.warning))
                                        .child("Enter reconnect · Ctrl+D close"),
                                ),
                        )
                    })
                    .when(!session_id.is_empty() && !self.terminal_status.trim().is_empty() && !is_active, |this| {
                        this.child(
                            div()
                                .h(px(22.))
                                .flex()
                                .items_center()
                                .px_3()
                                .border_b_1()
                                .border_color(rgb(palette.border))
                                .bg(rgb(palette.input))
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .child(self.terminal_status.clone()),
                                ),
                        )
                    })
                    // Empty-workspace bootstrap actions stay available when no session is selected.
                    .when(session_id.is_empty(), |this| {
                        this.child(
                            div()
                                .h(px(36.))
                                .flex()
                                .items_center()
                                .gap_2()
                                .px_3()
                                .border_b_1()
                                .border_color(rgb(palette.border))
                                .bg(rgb(palette.input))
                                .child(small_button(
                                    palette,
                                    "terminal-start-local",
                                    "Start Local",
                                    cx.listener(|this, _, window, cx| {
                                        this.start_local_session(window, cx);
                                    }),
                                ))
                                .child(small_button(
                                    palette,
                                    "terminal-actions",
                                    "Actions",
                                    cx.listener(|this, _, window, cx| {
                                        this.open_terminal_actions(window, cx);
                                    }),
                                ))
                                .child(div().flex_1())
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .child(self.terminal_status.clone()),
                                ),
                        )
                    })
                    .child(
                        div()
                            .id(SharedString::from(format!(
                                "terminal-output-{output_session_id}"
                            )))
                            .relative()
                            .flex_1()
                            .min_h_0()
                            .when(
                                is_active
                                    && (self.action_link_tooltip.is_some()
                                        || self.action_link_hover_pending.is_some()),
                                |this| this.cursor_pointer(),
                            )
                            .p(if self.settings.terminal_show_workspace_padding {
                                px(16.)
                            } else {
                                px(8.)
                            })
                            .overflow_hidden()
                            .can_drop(|drag, _, _| drag.is::<gpui::ExternalPaths>())
                            .on_drag_move({
                                let session_id = output_session_id.clone();
                                cx.listener(
                                    move |this,
                                          event: &gpui::DragMoveEvent<gpui::ExternalPaths>,
                                          _,
                                          cx| {
                                        let _ = event;
                                        this.set_terminal_file_drop_hover(
                                            Some(session_id.clone()),
                                            cx,
                                        );
                                    },
                                )
                            })
                            .on_drop({
                                let session_id = output_session_id.clone();
                                cx.listener(
                                    move |this, paths: &gpui::ExternalPaths, _, cx| {
                                        this.handle_terminal_external_file_drop(
                                            session_id.clone(),
                                            paths.paths().to_vec(),
                                            cx,
                                        );
                                    },
                                )
                            })
                            .on_scroll_wheel(cx.listener(
                                move |this, event: &ScrollWheelEvent, _, cx| {
                                    // Positive wheel delta.y scrolls into history (larger offset).
                                    let delta = match event.delta {
                                        ScrollDelta::Lines(delta) => delta.y.round() as i32,
                                        ScrollDelta::Pixels(delta) => {
                                            let py = f32::from(delta.y);
                                            let (_, cell_h) = this.terminal_cell_size();
                                            (py / cell_h.max(1.)).round() as i32
                                        }
                                    };
                                    if delta == 0 {
                                        return;
                                    }
                                    // Mouse tracking: wheel becomes button 64/65 reports.
                                    if let Some(cell) = this.point_to_terminal_cell(event.position) {
                                        let button = if delta > 0 { 64u8 } else { 65u8 };
                                        let steps = delta.unsigned_abs().min(8);
                                        let mut reported = false;
                                        for _ in 0..steps {
                                            if this.maybe_send_mouse_report(
                                                button,
                                                cell.col as u16,
                                                cell.row as u16,
                                                true,
                                                cx,
                                            ) {
                                                reported = true;
                                            } else {
                                                break;
                                            }
                                        }
                                        if reported {
                                            cx.stop_propagation();
                                            return;
                                        }
                                    }
                                    this.scroll_terminal_by(delta, cx);
                                    cx.stop_propagation();
                                },
                            ))
                            .on_mouse_down(
                                MouseButton::Left,
                                {
                                    let session_id = output_session_id.clone();
                                    cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                                        this.activate_workspace_pane(session_id.clone(), cx);
                                        window.focus(&this.terminal_focus);
                                        this.close_terminal_context_menu(cx);
                                        this.close_action_link_menu(cx);
                                        let mods = event.modifiers;
                                        let skip_selection = this.settings.terminal_action_links_enabled
                                            && (mods.alt || mods.control || mods.platform);
                                        if !skip_selection {
                                            this.start_terminal_selection(event, cx);
                                        }
                                        cx.stop_propagation();
                                    })
                                },
                            )
                            .on_mouse_down(
                                MouseButton::Right,
                                {
                                    let session_id = output_session_id.clone();
                                    cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                                        this.activate_workspace_pane(session_id.clone(), cx);
                                        window.focus(&this.terminal_focus);
                                        if let Some(cell) = this.point_to_terminal_cell(event.position) {
                                            if this.maybe_send_mouse_report(
                                                2,
                                                cell.col as u16,
                                                cell.row as u16,
                                                true,
                                                cx,
                                            ) {
                                                cx.stop_propagation();
                                                return;
                                            }
                                        }
                                        if this.settings.interaction_right_click_paste {
                                            this.paste_from_clipboard(window, cx);
                                        } else {
                                            this.open_terminal_context_menu(event, cx);
                                        }
                                        cx.stop_propagation();
                                    })
                                },
                            )
                            .on_mouse_down(
                                MouseButton::Middle,
                                {
                                    let session_id = output_session_id.clone();
                                    cx.listener(move |this, event: &gpui::MouseDownEvent, window, cx| {
                                        // xterm/Linux middle-click paste convention.
                                        this.activate_workspace_pane(session_id.clone(), cx);
                                        window.focus(&this.terminal_focus);
                                        this.close_terminal_context_menu(cx);
                                        this.close_action_link_menu(cx);
                                        if let Some(cell) = this.point_to_terminal_cell(event.position) {
                                            if this.maybe_send_mouse_report(
                                                1,
                                                cell.col as u16,
                                                cell.row as u16,
                                                true,
                                                cx,
                                            ) {
                                                cx.stop_propagation();
                                                return;
                                            }
                                        }
                                        this.paste_from_clipboard(window, cx);
                                        cx.stop_propagation();
                                    })
                                },
                            )
                            .on_click({
                                let session_id = output_session_id.clone();
                                cx.listener(move |this, event: &ClickEvent, window, cx| {
                                this.activate_workspace_pane(session_id.clone(), cx);
                                if event.is_right_click() {
                                    // Right-click is handled on mouse_down for Tauri-like context menu.
                                    cx.stop_propagation();
                                    return;
                                }
                                window.focus(&this.terminal_focus);
                                let modifiers = event.modifiers();
                                if this.settings.terminal_action_links_enabled {
                                    if modifiers.alt {
                                        if this.try_open_action_link_menu_at_click(event, cx) {
                                            cx.stop_propagation();
                                            return;
                                        }
                                    } else if modifiers.control || modifiers.platform {
                                        if this.try_activate_action_link_at_click(event, cx) {
                                            cx.stop_propagation();
                                            return;
                                        }
                                    }
                                }
                                if this.terminal_selection.is_none() {
                                    this.terminal_status = "terminal focused".to_string();
                                }
                                cx.notify();
                            })
                            })
                            .child(
                                div()
                                    .size_full()
                                    .flex()
                                    .flex_row()
                                    .min_h_0()
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w_0()
                                            .min_h_0()
                                            .child(output),
                                    )
                                    .child(self.terminal_scrollbar_element(
                                        &session_id,
                                        is_active,
                                        scroll_offset,
                                        cx,
                                    )),
                            )
                            .when(show_visual_bell, |this| {
                                this.child(
                                    div()
                                        .absolute()
                                        .inset_0()
                                        .bg(rgba(0xffffff22))
                                        .border_2()
                                        .border_color(rgb(palette.warning))
                                        .rounded_sm(),
                                )
                            })
                            .when(file_drop_hover && !session_id.is_empty(), |this| {
                                this.child(
                                    div()
                                        .absolute()
                                        .inset_2()
                                        .rounded_lg()
                                        .border_2()
                                        .border_color(rgb(palette.accent))
                                        .bg(rgba(0x3b82f624))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .child(
                                            div()
                                                .max_w(px(320.))
                                                .rounded_lg()
                                                .border_1()
                                                .border_color(rgb(palette.accent))
                                                .bg(rgb(palette.surface))
                                                .px_6()
                                                .py_4()
                                                .shadow_lg()
                                                .flex()
                                                .flex_col()
                                                .items_center()
                                                .gap_1()
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .font_weight(FontWeight(700.))
                                                        .text_color(rgb(palette.text))
                                                        .child(drop_title),
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(rgb(palette.text_muted))
                                                        .child(drop_hint),
                                                ),
                                        ),
                                )
                            })
                            .when(show_sync_action_overlay, |this| {
                                let pause_session_id = output_session_id.clone();
                                let leave_session_id = output_session_id.clone();
                                let close_session_id = output_session_id.clone();
                                this.child(
                                    div()
                                        .id(SharedString::from(format!(
                                            "terminal-sync-overlay-{output_session_id}"
                                        )))
                                        .absolute()
                                        .right(px(8.))
                                        .top(px(4.))
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .rounded_md()
                                        .px_1()
                                        .py(px(2.))
                                        .border_1()
                                        .border_color(rgba((sync_group_color << 8) | 0x4d))
                                        .bg(rgba((palette.surface << 8) | 0xeb))
                                        .shadow_sm()
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight(700.))
                                                .text_color(rgb(sync_group_color))
                                                .mr(px(4.))
                                                .child(sync_status_label),
                                        )
                                        .child(
                                            div()
                                                .id(SharedString::from(format!(
                                                    "terminal-sync-pause-{output_session_id}"
                                                )))
                                                .rounded_sm()
                                                .px_1()
                                                .py(px(2.))
                                                .cursor_pointer()
                                                .hover(|style| style.bg(rgb(palette.hover)))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.toggle_session_paused_in_active_sync_group(
                                                        pause_session_id.clone(),
                                                        cx,
                                                    );
                                                }))
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(rgb(sync_group_color))
                                                        .child(if sync_is_paused {
                                                            "Resume"
                                                        } else {
                                                            "Pause"
                                                        }),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .id(SharedString::from(format!(
                                                    "terminal-sync-leave-{output_session_id}"
                                                )))
                                                .rounded_sm()
                                                .px_1()
                                                .py(px(2.))
                                                .cursor_pointer()
                                                .hover(|style| style.bg(rgb(palette.hover)))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.leave_active_sync_group(
                                                        leave_session_id.clone(),
                                                        cx,
                                                    );
                                                }))
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(rgb(sync_group_color))
                                                        .child("Leave"),
                                                ),
                                        )
                                        .child(
                                            div()
                                                .id(SharedString::from(format!(
                                                    "terminal-sync-close-{output_session_id}"
                                                )))
                                                .rounded_sm()
                                                .px_1()
                                                .py(px(2.))
                                                .cursor_pointer()
                                                .hover(|style| style.bg(rgba(0xef44441a)))
                                                .on_click(cx.listener(move |this, _, _, cx| {
                                                    this.close_active_sync_group_for_session(
                                                        close_session_id.clone(),
                                                        cx,
                                                    );
                                                }))
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(rgb(palette.danger))
                                                        .child("Close Group"),
                                                ),
                                        ),
                                )
                            })
                            .when_some(performance_overlay, |this, overlay| {
                                let stats_detail = format!(
                                    "Queued {} in {} event(s). Dropped {} total. Last drain {}. Link cache {}/{} hit/miss.",
                                    format_bytes(self.terminal_runtime.session_event_queued_output_bytes as u64),
                                    format_skipped_count(self.terminal_runtime.session_event_queued_events as u64),
                                    format_bytes(self.terminal_runtime.session_event_dropped_output_bytes),
                                    format_bytes(self.terminal_runtime.session_event_last_drained_output_bytes as u64),
                                    format_skipped_count(render_cache_hits),
                                    format_skipped_count(render_cache_misses),
                                );
                                let (title, detail) = match overlay {
                                    TerminalPerformanceOverlay::Overloaded => (
                                        "Large-output protection active",
                                        format!(
                                            "Rendering is prioritizing responsiveness. Skipped {} queued characters. {}",
                                            format_skipped_count(skipped_output_chars),
                                            stats_detail,
                                        ),
                                    ),
                                    TerminalPerformanceOverlay::Recovered => (
                                        "Large-output protection recovered",
                                        format!(
                                            "The terminal is responsive again. Skipped {} queued characters during overload. {}",
                                            format_skipped_count(skipped_output_chars),
                                            stats_detail,
                                        ),
                                    ),
                                };
                                this.child(
                                    div()
                                        .id(SharedString::from(format!(
                                            "terminal-perf-overlay-{output_session_id}"
                                        )))
                                        .absolute()
                                        .left(px(12.))
                                        .right(px(12.))
                                        .top(px(12.))
                                        .flex()
                                        .justify_end()
                                        .child(
                                            div()
                                                .max_w(px(360.))
                                                .rounded_md()
                                                .border_1()
                                                .border_color(rgb(palette.border))
                                                .bg(rgb(palette.surface))
                                                .px_3()
                                                .py_2()
                                                .shadow_lg()
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .font_weight(FontWeight(700.))
                                                        .text_color(rgb(palette.text))
                                                        .child(title),
                                                )
                                                .child(
                                                    div()
                                                        .mt_1()
                                                        .text_xs()
                                                        .text_color(rgb(palette.text_dimmed))
                                                        .child(detail),
                                                ),
                                        ),
                                )
                            })
                            .when(show_scroll_to_bottom, |this| {
                                this.child(
                                    div()
                                        .id(SharedString::from(format!(
                                            "terminal-scroll-bottom-{output_session_id}"
                                        )))
                                        .absolute()
                                        .right(px(22.))
                                        .bottom(px(14.))
                                        .h(px(30.))
                                        .px_3()
                                        .rounded_md()
                                        .border_1()
                                        .border_color(rgb(palette.border))
                                        .bg(rgb(palette.surface_elevated))
                                        .shadow_md()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .cursor_pointer()
                                        .hover(move |style| style.bg(rgb(palette.hover)))
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.scroll_terminal_to_bottom(cx);
                                            this.terminal_status =
                                                "scrolled to live output".to_string();
                                            cx.notify();
                                        }))
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_weight(FontWeight(700.))
                                                .text_color(rgb(
                                                    if has_new_while_scrolled {
                                                        palette.warning
                                                    } else {
                                                        palette.accent
                                                    },
                                                ))
                                                .child(if has_new_while_scrolled {
                                                    "↓ New"
                                                } else {
                                                    "↓ Live"
                                                }),
                                        ),
                                )
                            })
                            .child(terminal_bounds_tracker(cx.entity())),
                    )
                    .when(is_active && self.terminal_search_open, |this| {
                        this.child(self.terminal_search_bar(cx))
                    }),
            )
    }
}

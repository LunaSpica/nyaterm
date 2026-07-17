use super::*;
use crate::models::TerminalPerformanceMode;

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
        let render_started_at = Instant::now();
        self.ensure_paint_theme_caches();
        let palette = self.terminal_theme_palette();
        let is_active = self.active_session_id.as_deref() == Some(session_id.as_str());
        let is_disconnected = !session_id.is_empty() && self.is_session_disconnected(&session_id);
        let terminal_mouse_reporting = !session_id.is_empty()
            && self
                .terminal_protocol_state_for_session(&session_id)
                .mouse_reporting;
        let render_output_pressure = self.runtime_output_pressure_active();
        let render_pressure = self
            .terminal_views
            .get(&session_id)
            .map(|view| {
                terminal_render_pressure_active(
                    render_output_pressure,
                    view.output_burst_bytes,
                    view.performance_mode,
                )
            })
            .unwrap_or(render_output_pressure);
        if render_pressure && let Some(view) = self.terminal_views.get_mut(&session_id) {
            view.enter_render_degraded_mode();
        }
        let render_degraded = self
            .terminal_views
            .get(&session_id)
            .is_some_and(|view| view.render_degraded || render_pressure);
        let render_profile = terminal_render_profile(render_degraded);
        let (output_burst_bytes, performance_mode) = self
            .terminal_views
            .get(&session_id)
            .map(|view| (view.output_burst_bytes, view.performance_mode))
            .unwrap_or((0, TerminalPerformanceMode::Normal));
        let expensive_interactions_enabled = terminal_expensive_interactions_enabled(
            self.settings.terminal_action_links_enabled,
            is_active,
            render_degraded,
            render_output_pressure,
            output_burst_bytes,
            performance_mode,
        );
        let action_link_matcher_key = terminal_action_link_matcher_key(
            self.settings.terminal_action_links_enabled,
            &self.settings.terminal_action_links_matchers,
        );
        let keyword_rules = if session_id.is_empty() {
            if render_degraded || !is_active {
                std::sync::Arc::new(Vec::new())
            } else {
                self.resolved_keyword_highlight_rules()
            }
        } else {
            // Live sessions paint keywords on TerminalSurface.
            std::sync::Arc::new(Vec::new())
        };
        let snapshot_stage_started_at = Instant::now();
        let layout_cache = self
            .terminal_views
            .get(&session_id)
            .map(|view| view.render_cache.layout_cache.clone());
        let scroll_offset = self
            .terminal_views
            .get(&session_id)
            .map(|view| view.scroll_offset)
            .unwrap_or(self.terminal_scroll_offset);
        let display_offset = self.terminal_display_offset_for_session(
            (!session_id.is_empty()).then_some(session_id.as_str()),
        );
        let frame_action_links = if session_id.is_empty() {
            self.terminal_views
                .get(&session_id)
                .filter(|_| expensive_interactions_enabled)
                .and_then(|view| {
                    if display_offset == 0 {
                        view.frame_action_links.as_ref()
                    } else {
                        view.scrollback_action_links.get(&display_offset)
                    }
                })
                .filter(|links| links.matcher_key == action_link_matcher_key)
        } else {
            // Live surfaces own action-link paint; shell only needs links for empty session.
            None
        };
        // Live sessions already paint the grid on TerminalSurface. Skip cloning /
        // building a shell-side viewport snapshot except when IME preedit needs
        // cursor placement, or when there is no session (empty bootstrap canvas).
        let needs_shell_viewport_snapshot = session_id.is_empty()
            || (is_active
                && self.settings.interaction_mac_ime_compatibility
                && !self.terminal_ime_marked_text.is_empty());
        let snapshot = if needs_shell_viewport_snapshot {
            self.terminal_snapshot_for_session(
                (!session_id.is_empty()).then_some(session_id.as_str()),
                display_offset,
            )
        } else {
            // Cheap placeholder: chrome wrappers do not paint cells from this value.
            std::sync::Arc::new(TerminalScreen::default().viewport_snapshot(0))
        };
        let line_count = snapshot.lines.len();
        let cursor_row = snapshot.cursor_row;
        let cursor_col = snapshot.cursor_col;
        let snapshot_rows = snapshot.rows;
        let snapshot_cols = snapshot.cols;
        let viewport_snapshot_duration = snapshot_stage_started_at.elapsed();
        let show_line_numbers = self.settings.terminal_show_line_numbers;
        let show_timestamps = self.settings.terminal_show_timestamps;
        let show_timestamp_ms = self.settings.terminal_show_timestamp_milliseconds;
        let gutter_enabled = show_line_numbers || show_timestamps;
        // Prefer remote cursor visibility/shape from the terminal model; settings
        // supply the default paint style when the model reports a block cursor.
        let remote_cursor_visible = snapshot.cursor.visible
            && snapshot.cursor.shape != nyaterm_terminal::CursorShape::Hidden
            && cursor_row != usize::MAX;
        let blink_enabled = self.settings.cursor_blink || snapshot.cursor.blinking;
        let show_cursor = is_active
            && !session_id.is_empty()
            && !is_disconnected
            && display_offset == 0
            && remote_cursor_visible
            && (!blink_enabled || self.terminal_runtime.cursor_blink_on);
        let cursor_style = match snapshot.cursor.shape {
            nyaterm_terminal::CursorShape::Underline => "underline".to_string(),
            nyaterm_terminal::CursorShape::Beam => "bar".to_string(),
            nyaterm_terminal::CursorShape::Hidden => self.settings.cursor_style.clone(),
            nyaterm_terminal::CursorShape::Block => self.settings.cursor_style.clone(),
        };
        let (abs_start, abs_end) = terminal_snapshot_absolute_range(&snapshot);
        let _ = abs_end;
        let (
            line_decorations,
            search_mapping_duration,
            action_link_duration,
            decorations_duration,
            search_matches_len,
        ) = if !session_id.is_empty() {
            // Decorations live on TerminalSurface (sync_terminal_surface_paint).
            (
                Vec::new(),
                Duration::ZERO,
                Duration::ZERO,
                Duration::ZERO,
                0usize,
            )
        } else {
            let search_stage_started_at = Instant::now();
            let search_matches = if render_profile.enhanced_decorations_enabled()
                && is_active
                && self.terminal_search_open
                && self.terminal_search_mode == TerminalSearchMode::Buffer
            {
                self.terminal_buffer_matches().unwrap_or_default()
            } else {
                Vec::new()
            };
            // Buffer matches use absolute history indices; map into current viewport rows.
            let active_match_abs = search_matches
                .get(
                    self.terminal_search_active_index
                        .min(search_matches.len().saturating_sub(1)),
                )
                .map(|search_match| search_match.line_index);
            let mut search_ranges_by_line: HashMap<usize, Vec<(usize, usize)>> = HashMap::new();
            let mut active_search_ranges_by_line: HashMap<usize, Vec<(usize, usize)>> =
                HashMap::new();
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
            let search_mapping_duration = search_stage_started_at.elapsed();
            let decoration_stage_started_at = Instant::now();
            let mut action_link_duration = Duration::ZERO;
            let terminal_selection = if render_profile.enhanced_decorations_enabled() {
                is_active.then_some(self.terminal_selection).flatten()
            } else {
                None
            };
            let has_selection = terminal_selection.is_some();
            let has_search_decorations =
                !search_ranges_by_line.is_empty() || !active_search_ranges_by_line.is_empty();
            let has_frame_action_links = expensive_interactions_enabled
                && frame_action_links.is_some_and(|links| {
                    links
                        .cell_ranges_by_line
                        .iter()
                        .any(|ranges| !ranges.is_empty())
                });
            let has_hyperlinks = expensive_interactions_enabled
                && snapshot
                    .hyperlink_lines
                    .iter()
                    .any(|spans| !spans.is_empty());
            let include_command_marks = is_active
                && render_profile.enhanced_decorations_enabled()
                && !render_output_pressure;
            let has_command_marks =
                include_command_marks && snapshot.command_marks.iter().any(Option::is_some);
            let needs_line_decorations = terminal_line_decorations_needed(
                has_selection,
                has_search_decorations,
                has_frame_action_links,
                has_hyperlinks,
                has_command_marks,
            );
            let line_decorations = if needs_line_decorations {
                let include_action_links = expensive_interactions_enabled;
                let include_hyperlinks = expensive_interactions_enabled;
                let decoration_cache_key = terminal_line_decorations_cache_key(
                    &snapshot,
                    terminal_selection,
                    0,
                    &search_ranges_by_line,
                    &active_search_ranges_by_line,
                    frame_action_links,
                    include_action_links,
                    include_hyperlinks,
                    include_command_marks,
                );
                let mut build = || {
                    let action_link_started_at = Instant::now();
                    let decorations = build_terminal_line_decorations(
                        &snapshot,
                        terminal_selection,
                        0,
                        &search_ranges_by_line,
                        &active_search_ranges_by_line,
                        frame_action_links,
                        include_action_links,
                        include_hyperlinks,
                        include_command_marks,
                    );
                    action_link_duration += action_link_started_at.elapsed();
                    decorations
                };
                if let Some(view) = self.terminal_views.get(&session_id) {
                    view.render_cache
                        .line_decorations(decoration_cache_key, build)
                } else {
                    build()
                }
            } else {
                Vec::new()
            };
            let decorations_duration = decoration_stage_started_at.elapsed();
            (
                line_decorations,
                search_mapping_duration,
                action_link_duration,
                decorations_duration,
                search_matches.len(),
            )
        };
        let element_stage_started_at = Instant::now();
        let (cell_w, cell_h) = self.terminal_cell_size();
        let terminal_font_family = self.gpui_terminal_font_family();
        let ime_preedit_text = (is_active
            && !session_id.is_empty()
            && self.settings.interaction_mac_ime_compatibility
            && !self.terminal_ime_marked_text.is_empty())
        .then(|| self.terminal_ime_marked_text.clone());
        let ime_preedit_position = ime_preedit_text.as_ref().map(|_| {
            let pad = self.terminal_content_padding_px();
            let gutter = self.terminal_gutter_width_px();
            let row = if cursor_row == usize::MAX {
                line_count.saturating_sub(1)
            } else {
                cursor_row.min(snapshot_rows.saturating_sub(1))
            };
            let col = cursor_col.min(snapshot_cols.saturating_sub(1));
            (
                pad + gutter + col as f32 * cell_w,
                pad + row as f32 * cell_h,
            )
        });
        let gutter = if session_id.is_empty() && gutter_enabled {
            let ts_w = self.terminal_timestamp_gutter_width_px();
            let ln_w = self.terminal_line_number_gutter_width_px();
            let mut gutter = div().flex().flex_col().flex_none();
            for line_index in 0..line_count {
                let ts_label = if show_timestamps {
                    snapshot
                        .line_timestamps_ms
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
                        .font_family(terminal_font_family.clone())
                        .text_size(px(self.settings.terminal_font_size as f32 * 0.85))
                        .when(show_timestamps, |this| {
                            this.child(div().w(px(ts_w)).flex_none().child(ts_label))
                        })
                        .when(show_line_numbers, |this| {
                            this.child(div().w(px(ln_w)).flex_none().child(line_label))
                        }),
                );
            }
            Some(gutter)
        } else {
            None
        };
        // Grid paint is owned by TerminalSurface so frame notify does not rebuild
        // the full shell. Push current paint inputs then embed the entity.
        let surface_entity = if session_id.is_empty() {
            None
        } else {
            // Do not rebuild surface paint state on every shell paint — frames and
            // local interactions already sync the entity. Only cold-start once.
            let surface = self.ensure_terminal_surface(&session_id, cx);
            let needs_seed = !surface.read(cx).has_snapshot();
            if needs_seed {
                self.sync_terminal_surface_paint(&session_id, cx);
            }
            Some(surface)
        };
        let output = if let Some(surface) = surface_entity {
            div()
                .flex()
                .flex_row()
                .flex_1()
                .min_h_0()
                .min_w_0()
                .child(surface)
        } else if let Some(gutter) = gutter {
            // Fallback empty session: keep a local element path.
            let mut grid = NyaTerminalElement::new(
                snapshot,
                keyword_rules,
                line_decorations,
                show_cursor,
                cursor_style,
                cell_w,
                cell_h,
                palette,
                terminal_font_family.clone(),
                self.settings.terminal_font_size as f32,
                self.settings.terminal_font_weight as f32,
                self.settings.terminal_font_weight_bold as f32,
            );
            if let Some(cache) = layout_cache {
                grid = grid.with_layout_cache(cache);
            }
            div().flex().flex_row().child(gutter).child(grid)
        } else {
            let mut grid = NyaTerminalElement::new(
                snapshot,
                keyword_rules,
                line_decorations,
                show_cursor,
                cursor_style,
                cell_w,
                cell_h,
                palette,
                terminal_font_family.clone(),
                self.settings.terminal_font_size as f32,
                self.settings.terminal_font_weight as f32,
                self.settings.terminal_font_weight_bold as f32,
            );
            if let Some(cache) = layout_cache {
                grid = grid.with_layout_cache(cache);
            }
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
            .map(|view| view.render_cache.decoration_stats())
            .unwrap_or((0, 0));
        let (layout_cache_hits, layout_cache_misses, layout_shape_calls, layout_shape_duration_ms) =
            self.terminal_views
                .get(&session_id)
                .and_then(|view| {
                    view.render_cache.layout_cache.lock().ok().map(|cache| {
                        (
                            cache.hits,
                            cache.misses,
                            cache.shape_calls,
                            cache.shape_duration_us / 1_000,
                        )
                    })
                })
                .unwrap_or((0, 0, 0, 0));
        let show_scroll_to_bottom = session_id.is_empty() && is_active && scroll_offset > 0;
        let show_visual_bell = is_active && self.terminal_runtime.visual_bell_ticks > 0;
        let file_drop_hover = self
            .terminal_file_drop_hover
            .as_deref()
            .is_some_and(|id| id == session_id.as_str());
        let drop_session_kind = self
            .session_metadata
            .get(&session_id)
            .map(|metadata| terminal_canvas_session_kind_label(&metadata.launch_config))
            .unwrap_or("Local");
        let (drop_title, drop_hint) = nyaterm_core::terminal_drop_overlay_copy(drop_session_kind);

        let canvas = div()
            .flex_1()
            .min_h_0()
            .font_family(terminal_font_family.clone())
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
                        this.mark_user_activity();
                        if this.handle_global_shortcut(event, window, cx) {
                            cx.stop_propagation();
                            return;
                        }
                        if this.handle_credential_suggestion_key(event, cx) {
                            cx.stop_propagation();
                            return;
                        }
                        if this.handle_command_suggestion_key(event, cx) {
                            cx.stop_propagation();
                            return;
                        }
                        if this.handle_terminal_scroll_key(event, cx) {
                            cx.stop_propagation();
                            return;
                        }
                        if this.handle_smart_input_selection_key(event, cx) {
                            cx.stop_propagation();
                            return;
                        }
                        // Disconnected tab: Enter reconnects; other keys show status (Tauri).
                        if let Some(session_id) = this.active_session_id.clone() {
                            if this.is_session_disconnected(&session_id) {
                                cx.stop_propagation();
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
                            cx.stop_propagation();
                            this.paste_from_clipboard(window, cx);
                            return;
                        }
                        if this.terminal_should_defer_key_text_to_input_handler(event) {
                            return;
                        }
                        cx.stop_propagation();
                        // When a non-smart buffer selection is painted, still send
                        // keystrokes but skip suggestion tracking so the selection
                        // edit path stays isolated (Tauri preserves selection).
                        let has_buffer_selection = this.terminal_selection.is_some()
                            && this.smart_cursor_selected_input_range().is_none();
                        if has_buffer_selection {
                            this.send_terminal_key_event(event, false, cx);
                        } else {
                            this.send_terminal_key_event(event, true, cx);
                        }
                    }))
                    .on_key_up(cx.listener(|this, event: &KeyUpEvent, _window, cx| {
                        if this.send_terminal_key_release_event(event, cx) {
                            cx.stop_propagation();
                            this.mark_user_activity();
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
                            .when(!is_disconnected && !terminal_mouse_reporting, |this| {
                                this.cursor_text()
                            })
                            .when(is_active && self.action_link_tooltip.is_some(), |this| {
                                this.cursor_pointer()
                            })
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
                                        if let Some(cell) = this.point_to_terminal_cell_for_session(
                                            Some(session_id.as_str()),
                                            event.position,
                                        ) {
                                            if this.maybe_send_mouse_report_for_session(
                                                &session_id,
                                                2,
                                                cell.col as u16,
                                                cell.row as u16,
                                                true,
                                                false,
                                                event.modifiers,
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
                                        if let Some(cell) = this.point_to_terminal_cell_for_session(
                                            Some(session_id.as_str()),
                                            event.position,
                                        ) {
                                            if this.maybe_send_mouse_report_for_session(
                                                &session_id,
                                                1,
                                                cell.col as u16,
                                                cell.row as u16,
                                                true,
                                                false,
                                                event.modifiers,
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
                                if this.terminal_selection.is_none()
                                    && this.terminal_status != "terminal focused"
                                {
                                    this.terminal_status = "terminal focused".to_string();
                                    cx.notify();
                                }
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
                                    // Scrollbar is painted by TerminalSurface for live sessions.
                                    .when(session_id.is_empty(), |this| {
                                        this.child(self.terminal_scrollbar_element(
                                            &session_id,
                                            is_active,
                                            scroll_offset,
                                            cx,
                                        ))
                                    }),
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
                            .when_some(
                                ime_preedit_text
                                    .clone()
                                    .zip(ime_preedit_position),
                                |this, (marked_text, (x, y))| {
                                    this.child(
                                        div()
                                            .absolute()
                                            .left(px(x))
                                            .top(px(y))
                                            .h(px(cell_h))
                                            .max_w(px(360.))
                                            .px_1()
                                            .flex()
                                            .items_center()
                                            .overflow_hidden()
                                            .whitespace_nowrap()
                                            .border_b_2()
                                            .border_color(rgb(palette.accent))
                                            .bg(rgba((palette.terminal_cursor << 8) | 0x33))
                                            .text_color(rgb(palette.terminal_fg))
                                            .font_family(terminal_font_family.clone())
                                            .text_size(px(self.settings.terminal_font_size as f32))
                                            .child(marked_text),
                                    )
                                },
                            )
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
                                    "Queued {} in {} event(s). Dropped {} total. Last drain {}. Link cache {}/{} hit/miss. Layout cache {}/{} hit/miss. Shape {} calls/{}ms.",
                                    format_bytes(self.terminal_runtime.session_event_queued_output_bytes as u64),
                                    format_skipped_count(self.terminal_runtime.session_event_queued_events as u64),
                                    format_bytes(self.terminal_runtime.session_event_dropped_output_bytes),
                                    format_bytes(self.terminal_runtime.session_event_last_drained_output_bytes as u64),
                                    format_skipped_count(render_cache_hits),
                                    format_skipped_count(render_cache_misses),
                                    format_skipped_count(layout_cache_hits),
                                    format_skipped_count(layout_cache_misses),
                                    format_skipped_count(layout_shape_calls),
                                    format_skipped_count(layout_shape_duration_ms),
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
                            .child(terminal_bounds_tracker(
                                cx.entity(),
                                (!output_session_id.is_empty()).then_some(output_session_id),
                                is_active,
                            )),
                    )
                    .when(is_active && self.terminal_search_open, |this| {
                        this.child(self.terminal_search_bar(cx))
                    }),
            );
        let element_construction_duration = element_stage_started_at.elapsed();
        let total_duration = render_started_at.elapsed();
        let render_slow = viewport_snapshot_duration >= TERMINAL_RENDER_SLOW_STAGE
            || search_mapping_duration >= TERMINAL_RENDER_SLOW_STAGE
            || action_link_duration >= TERMINAL_RENDER_SLOW_STAGE
            || decorations_duration >= TERMINAL_RENDER_SLOW_STAGE
            || element_construction_duration >= TERMINAL_RENDER_SLOW_STAGE
            || total_duration >= TERMINAL_RENDER_SLOW_TOTAL;
        if render_slow
            && !render_degraded
            && (search_mapping_duration >= TERMINAL_RENDER_SLOW_STAGE
                || action_link_duration >= TERMINAL_RENDER_SLOW_STAGE
                || decorations_duration >= TERMINAL_RENDER_SLOW_STAGE)
            && let Some(view) = self.terminal_views.get_mut(&session_id)
        {
            view.enter_render_degraded_mode();
        }
        if render_slow && self.should_log_slow_diagnostic("terminal_render", Instant::now()) {
            tracing::warn!(
                diagnostic = "terminal_render",
                session_id = %session_id,
                rows = snapshot_rows,
                cols = snapshot_cols,
                line_count,
                is_active,
                render_degraded,
                render_profile = render_profile.label(),
                render_output_pressure,
                expensive_interactions_enabled,
                output_burst_bytes,
                ?performance_mode,
                action_links_enabled = self.settings.terminal_action_links_enabled,
                search_open = self.terminal_search_open,
                search_matches = search_matches_len,
                render_cache_hits,
                render_cache_misses,
                layout_cache_hits,
                layout_cache_misses,
                layout_shape_calls,
                layout_shape_duration_ms,
                viewport_snapshot_ms = viewport_snapshot_duration.as_millis(),
                search_mapping_ms = search_mapping_duration.as_millis(),
                action_links_ms = action_link_duration.as_millis(),
                decorations_ms = decorations_duration.as_millis(),
                element_construction_ms = element_construction_duration.as_millis(),
                total_ms = total_duration.as_millis(),
                "slow terminal render"
            );
        }
        canvas
    }
}

fn terminal_render_pressure_active(
    runtime_output_pressure: bool,
    output_burst_bytes: usize,
    performance_mode: TerminalPerformanceMode,
) -> bool {
    runtime_output_pressure
        || output_burst_bytes > 0
        || performance_mode == TerminalPerformanceMode::Overloaded
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TerminalRenderProfile {
    Full,
    PlainViewport,
}

impl TerminalRenderProfile {
    fn enhanced_decorations_enabled(self) -> bool {
        matches!(self, Self::Full)
    }

    fn label(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::PlainViewport => "plain_viewport",
        }
    }
}

fn terminal_render_profile(render_degraded: bool) -> TerminalRenderProfile {
    if render_degraded {
        TerminalRenderProfile::PlainViewport
    } else {
        TerminalRenderProfile::Full
    }
}

fn terminal_canvas_session_kind_label(config: &SessionLaunchConfig) -> &'static str {
    match config {
        SessionLaunchConfig::Local(_) => "Local",
        SessionLaunchConfig::Ssh(_) => "SSH",
        SessionLaunchConfig::Telnet(config) if config.raw_tcp => "Raw TCP",
        SessionLaunchConfig::Telnet(_) => "Telnet",
        SessionLaunchConfig::Serial(_) => "Serial",
    }
}

const TERMINAL_RENDER_SLOW_STAGE: Duration = Duration::from_millis(12);
const TERMINAL_RENDER_SLOW_TOTAL: Duration = Duration::from_millis(25);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_line_decorations_skip_plain_viewport() {
        assert!(!terminal_line_decorations_needed(
            false, false, false, false, false
        ));
    }

    #[test]
    fn terminal_line_decorations_keep_interactive_marks() {
        assert!(terminal_line_decorations_needed(
            true, false, false, false, false
        ));
        assert!(terminal_line_decorations_needed(
            false, true, false, false, false
        ));
        assert!(terminal_line_decorations_needed(
            false, false, true, false, false
        ));
        assert!(terminal_line_decorations_needed(
            false, false, false, true, false
        ));
        assert!(terminal_line_decorations_needed(
            false, false, false, false, true
        ));
    }

    #[test]
    fn terminal_render_pressure_tracks_runtime_bursts_and_overload() {
        assert!(terminal_render_pressure_active(
            true,
            0,
            TerminalPerformanceMode::Normal
        ));
        assert!(terminal_render_pressure_active(
            false,
            1,
            TerminalPerformanceMode::Normal
        ));
        assert!(terminal_render_pressure_active(
            false,
            0,
            TerminalPerformanceMode::Overloaded
        ));
        assert!(!terminal_render_pressure_active(
            false,
            0,
            TerminalPerformanceMode::Normal
        ));
    }

    #[test]
    fn terminal_render_profile_uses_plain_viewport_while_degraded() {
        assert_eq!(terminal_render_profile(false), TerminalRenderProfile::Full);
        assert_eq!(
            terminal_render_profile(true),
            TerminalRenderProfile::PlainViewport
        );
        assert!(terminal_render_profile(false).enhanced_decorations_enabled());
        assert!(!terminal_render_profile(true).enhanced_decorations_enabled());
        assert_eq!(terminal_render_profile(true).label(), "plain_viewport");
    }

    #[test]
    fn terminal_line_decorations_cache_key_tracks_selection() {
        let snapshot = TerminalScreen::default().viewport_snapshot(0);
        let search = HashMap::new();
        let active = HashMap::new();
        let without_selection = terminal_line_decorations_cache_key(
            &snapshot, None, 0, &search, &active, None, false, false, false,
        );
        let with_selection = terminal_line_decorations_cache_key(
            &snapshot,
            Some(TerminalSelection {
                anchor: TerminalCellPos::new(0, 1),
                head: TerminalCellPos::new(0, 3),
            }),
            0,
            &search,
            &active,
            None,
            false,
            false,
            false,
        );

        assert_ne!(without_selection, with_selection);
    }

    #[test]
    fn terminal_line_decorations_cache_key_tracks_action_links() {
        let snapshot = TerminalScreen::default().viewport_snapshot(0);
        let search = HashMap::new();
        let active = HashMap::new();
        let mut links = TerminalFrameActionLinks {
            matcher_key: 42,
            matches_by_line: Vec::new(),
            cell_ranges_by_line: vec![vec![(1, 4)]],
        };
        let first = terminal_line_decorations_cache_key(
            &snapshot,
            None,
            0,
            &search,
            &active,
            Some(&links),
            true,
            false,
            false,
        );
        links.cell_ranges_by_line[0] = vec![(2, 5)];
        let second = terminal_line_decorations_cache_key(
            &snapshot,
            None,
            0,
            &search,
            &active,
            Some(&links),
            true,
            false,
            false,
        );

        assert_ne!(first, second);
    }

    #[test]
    fn terminal_line_decorations_cache_key_tracks_command_mark_mode() {
        let snapshot = TerminalScreen::default().viewport_snapshot(0);
        let search = HashMap::new();
        let active = HashMap::new();
        let without_marks = terminal_line_decorations_cache_key(
            &snapshot, None, 0, &search, &active, None, false, false, false,
        );
        let with_marks = terminal_line_decorations_cache_key(
            &snapshot, None, 0, &search, &active, None, false, false, true,
        );

        assert_ne!(without_marks, with_marks);
    }
}

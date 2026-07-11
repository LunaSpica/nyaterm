use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn terminal_canvas(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let session_id = self.active_session_id.clone().unwrap_or_default();
        self.terminal_canvas_for(session_id, false, cx)
    }

    pub(in crate::ui::view) fn terminal_canvas_for(
        &self,
        session_id: String,
        show_pane_chrome: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let is_active = self.active_session_id.as_deref() == Some(session_id.as_str());
        let mut output = div().flex().flex_col();
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
        let lines = snapshot.lines;
        let styled_lines = snapshot.styled_lines;
        let line_timestamps_ms = snapshot.line_timestamps_ms;
        let cursor_row = snapshot.cursor_row;
        let cursor_col = snapshot.cursor_col;
        let show_line_numbers = self.settings.terminal_show_line_numbers;
        let show_timestamps = self.settings.terminal_show_timestamps;
        let show_timestamp_ms = self.settings.terminal_show_timestamp_milliseconds;
        let gutter_enabled = show_line_numbers || show_timestamps;
        let show_cursor = is_active
            && !session_id.is_empty()
            && scroll_offset == 0
            && cursor_row != usize::MAX
            && (!self.settings.cursor_blink || self.cursor_blink_on);
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
        let active_match_line = search_matches
            .get(
                self.terminal_search_active_index
                    .min(search_matches.len().saturating_sub(1)),
            )
            .map(|search_match| search_match.line_index)
            .and_then(|abs| {
                if abs >= abs_start && abs < abs_end {
                    Some(abs - abs_start)
                } else {
                    None
                }
            });
        let matched_lines = search_matches
            .iter()
            .filter_map(|search_match| {
                let abs = search_match.line_index;
                if abs >= abs_start && abs < abs_end {
                    Some(abs - abs_start)
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();
        for (line_index, line) in lines.into_iter().enumerate() {
            let line = if line.is_empty() {
                " ".to_string()
            } else {
                line
            };
            let ansi = styled_lines.get(line_index).map(|s| s.as_slice());
            let selection_cols = if is_active {
                self.terminal_selection
                    .as_ref()
                    .and_then(|selection| selection.cols_for_row(line_index))
            } else {
                None
            };
            let link_ranges: Vec<(usize, usize)> = if self.settings.terminal_action_links_enabled {
                find_action_links(
                    &line,
                    &self.settings.terminal_action_links_matchers,
                    true,
                )
                .into_iter()
                .map(|item| {
                    // Convert byte offsets from regex to char indices for painting.
                    let start_chars = line[..item.start.min(line.len())].chars().count();
                    let end_chars = line[..item.end.min(line.len())].chars().count();
                    (start_chars, end_chars)
                })
                .collect()
            } else {
                Vec::new()
            };
            let content = terminal_line_element(
                &line,
                ansi,
                &self.keyword_highlights,
                matched_lines.contains(&line_index),
                active_match_line == Some(line_index),
                if show_cursor && line_index == cursor_row {
                    Some(cursor_col)
                } else {
                    None
                },
                cursor_style,
                selection_cols,
                &link_ranges,
                self.terminal_cell_size().1,
                palette,
            );
            if gutter_enabled {
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
                let (_, cell_h) = self.terminal_cell_size();
                output = output.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .min_h(px(cell_h))
                        .child(
                            div()
                                .flex()
                                .flex_row()
                                .items_center()
                                .gap_1()
                                .flex_none()
                                .pr_1()
                                .text_color(rgb(palette.text_dimmed))
                                .font_family(self.settings.terminal_font_family.clone())
                                .text_size(px(self.settings.terminal_font_size as f32 * 0.85))
                                .when(show_timestamps, |this| {
                                    this.child(
                                        div()
                                            .w(px(if show_timestamp_ms { 96. } else { 72. }))
                                            .flex_none()
                                            .child(ts_label),
                                    )
                                })
                                .when(show_line_numbers, |this| {
                                    this.child(
                                        div()
                                            .w(px(40.))
                                            .flex_none()
                                            .child(line_label),
                                    )
                                }),
                        )
                        .child(content),
                );
            } else {
                output = output.child(content);
            }
        }
        let pane_title = self
            .session_display_name(&session_id)
            .unwrap_or_else(|| short_id(&session_id).to_string());
        let sync_group_label = self.active_sync_group_label(&session_id);
        let output_session_id = session_id.clone();
        let terminal_font_family = self.settings.terminal_font_family.clone();
        let terminal_font_size = self.settings.terminal_font_size as f32;

        div()
            .flex_1()
            .min_h_0()
            .font_family(terminal_font_family)
            .text_size(px(terminal_font_size))
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
                        if this.handle_terminal_scroll_key(event, cx) {
                            return;
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
                            this.send_terminal_input(bytes, cx);
                        }
                    }))
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
                                    if delta != 0 {
                                        this.scroll_terminal_by(delta, cx);
                                        cx.stop_propagation();
                                    }
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
                                    cx.listener(move |this, _event: &gpui::MouseDownEvent, window, cx| {
                                        // xterm/Linux middle-click paste convention.
                                        this.activate_workspace_pane(session_id.clone(), cx);
                                        window.focus(&this.terminal_focus);
                                        this.close_terminal_context_menu(cx);
                                        this.close_action_link_menu(cx);
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
                            .child(terminal_bounds_tracker(cx.entity())),
                    )
                    .when(is_active && self.terminal_search_open, |this| {
                        this.child(self.terminal_search_bar(cx))
                    }),
            )
    }

    fn terminal_scrollbar_element(
        &self,
        session_id: &str,
        is_active: bool,
        scroll_offset: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        use gpui::relative;
        let palette = self.theme_palette();
        let max = self
            .terminal_views
            .get(session_id)
            .map(|view| view.screen.scrollback_len())
            .unwrap_or_else(|| {
                if session_id.is_empty() {
                    self.terminal_screen.scrollback_len()
                } else {
                    0
                }
            });
        let viewport_rows = self
            .terminal_views
            .get(session_id)
            .map(|view| {
                // viewport height equals live screen rows
                view.screen.viewport_snapshot(0).lines.len().max(1)
            })
            .unwrap_or_else(|| self.active_terminal_page_rows().max(1));
        let show = max > 0;
        let thumb_ratio = if max == 0 {
            1.0
        } else {
            let viewport = viewport_rows as f32;
            (viewport / (viewport + max as f32)).clamp(0.12, 1.0)
        };
        let travel = (1.0 - thumb_ratio).max(0.0);
        let thumb_top_ratio = if max == 0 {
            0.0
        } else {
            travel * (1.0 - (scroll_offset as f32 / max as f32).clamp(0.0, 1.0))
        };
        let track_id = format!("terminal-scrollbar-track-{session_id}");
        let thumb_id = format!("terminal-scrollbar-thumb-{session_id}");

        div()
            .id(SharedString::from(format!("terminal-scrollbar-{session_id}")))
            .w(px(10.))
            .flex_none()
            .h_full()
            .py(px(2.))
            .pr(px(2.))
            .opacity(if show { 1.0 } else { 0.35 })
            .child(
                div()
                    .id(SharedString::from(track_id))
                    .relative()
                    .size_full()
                    .rounded_full()
                    .bg(rgb(palette.border))
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        {
                            let session_id = session_id.to_string();
                            cx.listener(move |this, event: &gpui::MouseDownEvent, _, cx| {
                            if !session_id.is_empty() {
                                this.activate_workspace_pane(session_id.clone(), cx);
                            }
                            this.begin_terminal_scrollbar_drag(cx);
                            let Some(bounds) = this.terminal_surface_bounds else {
                                return;
                            };
                            let height = f32::from(bounds.size.height).max(1.0);
                            let local_y = f32::from(event.position.y - bounds.origin.y);
                            let ratio = (local_y / height).clamp(0.0, 1.0);
                            this.set_terminal_scroll_from_track_ratio(ratio, cx);
                            cx.stop_propagation();
                        })
                        },
                    )
                    .when(show, |this| {
                        this.child(
                            div()
                                .id(SharedString::from(thumb_id))
                                .absolute()
                                .left(px(1.))
                                .right(px(1.))
                                .top(relative(thumb_top_ratio))
                                .h(relative(thumb_ratio))
                                .min_h(px(18.))
                                .rounded_full()
                                .bg(rgb(if is_active {
                                    palette.accent
                                } else {
                                    palette.text_muted
                                }))
                                .opacity(0.85),
                        )
                    }),
            )
    }

    fn terminal_search_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let buffer_matches = self.terminal_buffer_matches();
        let history_results = self.terminal_history_search_results();
        let (status, is_error) = match self.terminal_search_mode {
            TerminalSearchMode::Buffer => match &buffer_matches {
                Ok(matches) if self.terminal_search_query.trim().is_empty() => {
                    ("idle".to_string(), false)
                }
                Ok(matches) if matches.is_empty() => ("not found".to_string(), false),
                Ok(matches) => (
                    format!(
                        "{}/{}",
                        self.terminal_search_active_index
                            .min(matches.len().saturating_sub(1))
                            + 1,
                        matches.len()
                    ),
                    false,
                ),
                Err(error) => (truncate_preview(error, 40), true),
            },
            TerminalSearchMode::History => match &history_results {
                Ok(response) if self.terminal_search_query.trim().is_empty() => {
                    ("idle".to_string(), false)
                }
                Ok(response) if response.results.is_empty() => ("not found".to_string(), false),
                Ok(response) => (
                    format!(
                        "{} result(s){}",
                        response.total,
                        if response.truncated { " truncated" } else { "" }
                    ),
                    false,
                ),
                Err(error) => (truncate_preview(error, 40), true),
            },
        };
        let input_display = if self.terminal_search_query.is_empty() {
            "Find".to_string()
        } else {
            self.terminal_search_query.clone()
        };
        let mut history_rows = div().flex().flex_col().gap_1();
        if self.terminal_search_mode == TerminalSearchMode::History
            && !self.terminal_search_query.trim().is_empty()
        {
            match history_results {
                Ok(response) if response.results.is_empty() => {
                    history_rows = history_rows.child(
                        div()
                            .px_2()
                            .py_2()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child("No history matches."),
                    );
                }
                Ok(response) => {
                    for result in response.results.into_iter().take(5) {
                        history_rows = history_rows.child(
                            div()
                                .rounded_sm()
                                .border_1()
                                .border_color(rgb(palette.border))
                                .bg(rgb(palette.input))
                                .p_2()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_weight(FontWeight(800.))
                                        .text_color(rgb(palette.text))
                                        .child(format!("line {}", result.line_number)),
                                )
                                .child(
                                    div()
                                        .mt_1()
                                        .font_family("JetBrains Mono")
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .line_height(px(18.))
                                        .child(truncate_preview(&result.preview, 96)),
                                ),
                        );
                    }
                }
                Err(error) => {
                    history_rows = history_rows.child(
                        div()
                            .px_2()
                            .py_2()
                            .text_xs()
                            .text_color(rgb(palette.danger))
                            .child(truncate_preview(&error, 96)),
                    );
                }
            }
        }

        div()
            .id(SharedString::from("terminal-search-bar"))
            .absolute()
            .top(px(8.))
            .right(px(8.))
            .w(px(420.))
            .max_w_full()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.terminal_bg))
            .shadow_lg()
            .p_2()
            .track_focus(&self.terminal_search_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.terminal_search_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                this.handle_terminal_search_key_down(event, window, cx);
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(mode_button(
                        "terminal-search-mode-buffer",
                        TerminalSearchMode::Buffer.label(),
                        self.terminal_search_mode == TerminalSearchMode::Buffer, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.terminal_search_mode = TerminalSearchMode::Buffer;
                            this.terminal_search_active_index = 0;
                            cx.notify();
                        }),
                    ))
                    .child(mode_button(
                        "terminal-search-mode-history",
                        TerminalSearchMode::History.label(),
                        self.terminal_search_mode == TerminalSearchMode::History, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.terminal_search_mode = TerminalSearchMode::History;
                            this.terminal_search_active_index = 0;
                            cx.notify();
                        }),
                    ))
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_xs()
                            .text_color(if is_error {
                                rgb(palette.danger)
                            } else {
                                rgb(palette.text_muted)
                            })
                            .child(status),
                    )
                    .child(icon_button(
                        "terminal-search-close",
                        "x", self.theme_palette(),cx.listener(|this, _, window, cx| {
                            this.close_terminal_search(window, cx);
                        }),
                    )),
            )
            .child(
                div()
                    .mt_2()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .id(SharedString::from("terminal-search-input"))
                            .h(px(28.))
                            .min_w_0()
                            .flex_1()
                            .rounded_sm()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(palette.input))
                            .px_2()
                            .flex()
                            .items_center()
                            .text_xs()
                            .text_color(if self.terminal_search_query.is_empty() {
                                rgb(palette.text_muted)
                            } else {
                                rgb(palette.text)
                            })
                            .child(input_display),
                    )
                    .child(mode_button(
                        "terminal-search-case",
                        "Aa",
                        self.terminal_search_case_sensitive, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.terminal_search_case_sensitive =
                                !this.terminal_search_case_sensitive;
                            this.terminal_search_active_index = 0;
                            cx.notify();
                        }),
                    ))
                    .child(mode_button(
                        "terminal-search-regex",
                        ".*",
                        self.terminal_search_regex, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.terminal_search_regex = !this.terminal_search_regex;
                            this.terminal_search_active_index = 0;
                            cx.notify();
                        }),
                    ))
                    .child(mode_button(
                        "terminal-search-word",
                        "Word",
                        self.terminal_search_whole_word, self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.terminal_search_whole_word = !this.terminal_search_whole_word;
                            this.terminal_search_active_index = 0;
                            cx.notify();
                        }),
                    ))
                    .child(icon_button(
                        "terminal-search-prev",
                        "^", self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.navigate_terminal_search(-1, cx);
                        }),
                    ))
                    .child(icon_button(
                        "terminal-search-next",
                        "v", self.theme_palette(),cx.listener(|this, _, _, cx| {
                            this.navigate_terminal_search(1, cx);
                        }),
                    )),
            )
            .child(history_rows)
    }
}

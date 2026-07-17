use super::*;
use gpui::{Bounds, Pixels};

impl NyaTermApp {
    pub(in crate::features) fn dismiss_command_suggestions(&mut self, cx: &mut Context<Self>) {
        self.command_suggestion_search_gen = self.command_suggestion_search_gen.saturating_add(1);
        let mut changed = false;
        if self.command_suggestions.take().is_some() {
            changed = true;
        }
        // Keep draft for continued typing; only clear list visibility.
        if changed {
            cx.notify();
        }
    }

    pub(in crate::features) fn clear_command_suggestion_draft(&mut self, cx: &mut Context<Self>) {
        self.command_suggestion_search_gen = self.command_suggestion_search_gen.saturating_add(1);
        let mut changed = false;
        if self.command_input_tracker != TerminalInputState::new() {
            self.command_input_tracker = TerminalInputState::new();
            changed = true;
        }
        if self.command_suggestions.take().is_some() {
            changed = true;
        }
        if changed {
            cx.notify();
        }
    }

    pub(in crate::features) fn note_command_suggestion_input(
        &mut self,
        bytes: &[u8],
        cx: &mut Context<Self>,
    ) {
        if self.credential_suggestions.is_some() || self.is_credential_prompt_input_mode() {
            return;
        }
        if !self.settings.interaction_command_suggestions_enabled {
            self.clear_command_suggestion_draft(cx);
            return;
        }
        if self.active_session_id.is_none() {
            self.clear_command_suggestion_draft(cx);
            return;
        }
        let Ok(text) = std::str::from_utf8(bytes) else {
            self.clear_command_suggestion_draft(cx);
            return;
        };
        if text.is_empty() {
            return;
        }

        // Exit interactive suppression on Ctrl+C or q (Tauri resetCommandSuggestionSuppression).
        if self.command_suggestions_suppressed && (text == "\u{0003}" || text == "q") {
            self.command_suggestions_suppressed = false;
            self.command_input_tracker = TerminalInputState::new();
            self.command_suggestions = None;
            cx.notify();
            return;
        }

        // Capture submission command before tracker reset on Enter.
        if text.contains('\r') || text.contains('\n') {
            let submitted = get_tracked_submission_command(&self.command_input_tracker);
            if !submitted.is_empty() {
                self.pending_command_history_entry = Some(submitted.clone());
                if command_starts_suggestion_suppressing_program(&submitted) {
                    self.command_suggestions_suppressed = true;
                }
            }
        }

        // Tab-desync recovery: before applying non-tab input, resync from terminal line.
        if text != "\t"
            && self.command_input_tracker.desynced
            && self.command_input_tracker.desync_reason == Some("tab")
        {
            if let Some(line) = self.read_active_terminal_input_line() {
                if let Some(recovered) =
                    resync_from_terminal_line(&self.command_input_tracker, &line)
                {
                    self.command_input_tracker = recovered;
                }
            }
        }

        self.command_input_tracker = apply_terminal_input_data(&self.command_input_tracker, text);

        if self.command_suggestions_suppressed {
            self.command_suggestion_search_gen =
                self.command_suggestion_search_gen.saturating_add(1);
            if self.command_suggestions.take().is_some() {
                cx.notify();
            }
            return;
        }

        if is_pager_search_or_command_input(&get_tracked_command(&self.command_input_tracker)) {
            self.command_suggestion_search_gen =
                self.command_suggestion_search_gen.saturating_add(1);
            if self.command_suggestions.take().is_some() {
                cx.notify();
            }
            return;
        }

        if !can_suggest_from_tracker(&self.command_input_tracker) {
            self.command_suggestion_search_gen =
                self.command_suggestion_search_gen.saturating_add(1);
            if self.command_suggestions.take().is_some() {
                cx.notify();
            }
            return;
        }
        self.schedule_command_suggestion_refresh(cx);
    }

    pub(in crate::features) fn schedule_command_suggestion_refresh(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        // Tauri useCommandHistory: 80ms debounce before fuzzy search.
        self.command_suggestion_search_gen = self.command_suggestion_search_gen.saturating_add(1);
        let request_id = self.command_suggestion_search_gen;
        cx.spawn(async move |this, cx| {
            Timer::after(Duration::from_millis(80)).await;
            let _ = this.update(cx, |this, cx| {
                if this.command_suggestion_search_gen != request_id {
                    return;
                }
                this.refresh_command_suggestions(cx);
            });
        })
        .detach();
    }

    pub(in crate::features) fn read_active_terminal_input_line(&self) -> Option<String> {
        let offset = self.active_terminal_display_offset();
        let snapshot =
            self.terminal_snapshot_for_session(self.active_session_id.as_deref(), offset);
        if snapshot.cursor_row == usize::MAX {
            return None;
        }
        let line = snapshot.lines.get(snapshot.cursor_row)?;
        Some(terminal_line_prefix_for_cell_col(line, snapshot.cursor_col))
    }

    pub(in crate::features) fn refresh_command_suggestions(&mut self, cx: &mut Context<Self>) {
        let Some(session_id) = self
            .active_session_id
            .as_deref()
            .filter(|session_id| !session_id.is_empty())
            .map(ToOwned::to_owned)
        else {
            self.hide_command_suggestions_if_present(cx);
            return;
        };
        if self.credential_suggestions.is_some()
            || self.is_credential_prompt_input_mode()
            || self.command_suggestions_suppressed
        {
            self.hide_command_suggestions_if_present(cx);
            return;
        }
        if !self.settings.interaction_command_suggestions_enabled {
            self.hide_command_suggestions_if_present(cx);
            return;
        }
        let pattern = get_tracked_command(&self.command_input_tracker);
        let min_chars = self
            .settings
            .interaction_command_suggestion_min_chars
            .max(1) as usize;
        let max_chars = self
            .settings
            .interaction_command_suggestion_max_chars
            .max(min_chars as u32) as usize;
        if pattern.chars().count() < min_chars {
            self.hide_command_suggestions_if_present(cx);
            return;
        }
        // Pager/search-like prefixes: hide suggestions.
        if pattern.starts_with('/') || pattern.starts_with('?') || pattern.starts_with(':') {
            self.hide_command_suggestions_if_present(cx);
            return;
        }
        let results = search_command_sources(
            &self.command_history,
            &self.quick_commands,
            &pattern,
            12,
            Some(min_chars),
            Some(max_chars),
        );
        if results.is_empty() {
            self.hide_command_suggestions_if_present(cx);
            return;
        }
        let (cursor_row, cursor_col) = self.active_terminal_cursor_cell();
        let selected_index = self
            .command_suggestions
            .as_ref()
            .filter(|state| state.session_id == session_id)
            .map(|state| state.selected_index.min(results.len().saturating_sub(1)))
            .unwrap_or(0);
        self.command_suggestions = Some(CommandSuggestionState {
            session_id,
            draft: pattern,
            items: results
                .into_iter()
                .map(|item| CommandSuggestionItem {
                    command: item.command,
                    display: item.display,
                    source: item.source,
                    score: item.score,
                    indices: item.indices,
                })
                .collect(),
            selected_index,
            cursor_row,
            cursor_col,
        });
        cx.notify();
    }

    fn hide_command_suggestions_if_present(&mut self, cx: &mut Context<Self>) {
        if self.command_suggestions.take().is_some() {
            cx.notify();
        }
    }

    pub(in crate::features) fn active_terminal_cursor_cell(&self) -> (usize, usize) {
        let offset = self.active_terminal_display_offset();
        let snapshot =
            self.terminal_snapshot_for_session(self.active_session_id.as_deref(), offset);
        let row = if snapshot.cursor_row == usize::MAX {
            snapshot.lines.len().saturating_sub(1)
        } else {
            snapshot.cursor_row
        };
        (row, snapshot.cursor_col)
    }

    /// Handle suggestion popup keys. Returns true when the key was consumed.

    pub(in crate::features) fn handle_command_suggestion_key(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(state) = self.command_suggestions.as_ref() else {
            return false;
        };
        if state.items.is_empty() {
            return false;
        }
        let session_id = state.session_id.clone();
        if self.active_session_id.as_deref() != Some(session_id.as_str())
            || self
                .terminal_surface_bounds_for_session(Some(&session_id))
                .is_none()
        {
            self.command_suggestions = None;
            cx.notify();
            return false;
        }
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return false;
        }
        match keystroke.key.as_str() {
            "escape" => {
                self.dismiss_command_suggestions(cx);
                true
            }
            "up" => {
                if let Some(state) = self.command_suggestions.as_mut() {
                    if state.selected_index == 0 {
                        state.selected_index = state.items.len().saturating_sub(1);
                    } else {
                        state.selected_index -= 1;
                    }
                    cx.notify();
                }
                true
            }
            "down" => {
                if let Some(state) = self.command_suggestions.as_mut() {
                    state.selected_index = (state.selected_index + 1) % state.items.len().max(1);
                    cx.notify();
                }
                true
            }
            "tab" => {
                self.apply_selected_command_suggestion(false, cx);
                true
            }
            "enter" => {
                self.apply_selected_command_suggestion(true, cx);
                true
            }
            _ => false,
        }
    }

    pub(in crate::features) fn apply_selected_command_suggestion(
        &mut self,
        execute: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.command_suggestions.clone() else {
            return;
        };
        let Some(item) = state.items.get(state.selected_index).cloned() else {
            return;
        };
        let source = item.source.clone();
        let command = item.command;
        // Tauri replaceCurrentLine path: Ctrl+E (end) + Ctrl+U (kill line) + command.
        let mut payload = String::new();
        payload.push('\u{05}'); // Ctrl+E
        payload.push('\u{15}'); // Ctrl+U
        payload.push_str(&command);
        if execute {
            payload.push('\r');
        }
        self.command_input_tracker = TerminalInputState::new();
        self.command_suggestions = None;
        self.send_terminal_input_without_suggestion_track(payload.into_bytes(), cx);
        if !execute {
            // After fill, tracker becomes the filled command for continued typing.
            self.command_input_tracker =
                apply_terminal_input_data(&TerminalInputState::new(), &command);
            self.refresh_command_suggestions(cx);
        }
        self.terminal_status = if execute {
            format!("executed suggestion from {source}")
        } else {
            format!("filled suggestion from {source}")
        };
        cx.notify();
    }

    pub(in crate::features) fn delete_command_suggestion_history(
        &mut self,
        command: String,
        cx: &mut Context<Self>,
    ) {
        let command = command.trim().to_string();
        if command.is_empty() {
            return;
        }
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            Ok(store) => {
                if let Err(error) = store.delete_command_history(&command) {
                    self.terminal_status = format!("failed to delete history: {error}");
                    cx.notify();
                    return;
                }
                self.command_history = store.list_command_history(64).unwrap_or_default();
                for history in self.session_command_history.values_mut() {
                    history.retain(|entry| entry != &command);
                }
            }
            Err(error) => {
                self.terminal_status = format!("failed to open store: {error}");
                cx.notify();
                return;
            }
        }

        if let Some(state) = self.command_suggestions.as_mut() {
            state
                .items
                .retain(|item| !(item.source == "history" && item.command == command));
            if state.items.is_empty() {
                self.command_suggestions = None;
            } else {
                state.selected_index = state
                    .selected_index
                    .min(state.items.len().saturating_sub(1));
            }
        } else {
            self.refresh_command_suggestions(cx);
        }
        self.terminal_status = format!("deleted history command '{command}'");
        cx.notify();
    }

    pub(in crate::features) fn command_suggestions_overlay(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let Some(state) = self.command_suggestions.as_ref() else {
            return div().into_any_element();
        };
        if state.items.is_empty() {
            return div().into_any_element();
        }
        if self.active_session_id.as_deref() != Some(state.session_id.as_str()) {
            return div().into_any_element();
        }
        let menu_w = 380.0_f32;
        let menu_h = (state.items.len() as f32 * 28.0 + 44.0).min(320.0);
        let Some((x, y)) = self.suggestion_overlay_position_for_session(
            Some(&state.session_id),
            state.cursor_row,
            state.cursor_col,
            menu_w,
            menu_h,
        ) else {
            return div().into_any_element();
        };

        let mut list = div()
            .id(SharedString::from("command-suggestions-list"))
            .flex()
            .flex_col()
            .max_h(px(280.))
            .overflow_y_scroll();
        for (index, item) in state.items.iter().enumerate() {
            let selected = index == state.selected_index;
            let source_label = match item.source.as_str() {
                "history" => "H",
                "quickCommand" => "Q",
                other => other,
            };
            let label = if item.display.trim().is_empty() {
                item.command.clone()
            } else {
                item.display.clone()
            };
            let is_history = item.source == "history";
            let delete_command = item.command.clone();
            let mut row = div()
                .id(SharedString::from(format!("command-suggestion-{index}")))
                .h(px(28.))
                .px_2()
                .flex()
                .items_center()
                .gap_2()
                .border_l_2()
                .border_color(rgb(if selected {
                    palette.accent
                } else {
                    palette.surface
                }))
                .bg(rgb(if selected {
                    palette.hover
                } else {
                    palette.surface
                }))
                .text_size(px(11.))
                .text_color(rgb(palette.text))
                .cursor_pointer()
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(state) = this.command_suggestions.as_mut() {
                        state.selected_index = index;
                    }
                    this.apply_selected_command_suggestion(true, cx);
                }))
                .child(
                    div()
                        .w(px(16.))
                        .flex_none()
                        .text_size(px(10.))
                        .text_color(rgb(if selected {
                            palette.accent
                        } else {
                            palette.text_dimmed
                        }))
                        .child(source_label.to_string()),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .font_family(crate::features::gpui_code_font_family())
                        .flex()
                        .items_center()
                        .overflow_hidden()
                        .children(command_suggestion_highlight_parts(
                            &label,
                            &item.indices,
                            palette,
                            selected,
                        )),
                );
            if is_history {
                row = row.child(
                    div()
                        .id(SharedString::from(format!(
                            "command-suggestion-del-{index}"
                        )))
                        .flex_none()
                        .w(px(20.))
                        .h(px(20.))
                        .rounded_sm()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(11.))
                        .text_color(rgb(palette.text_dimmed))
                        .hover(|this| {
                            this.bg(rgb(palette.surface_elevated))
                                .text_color(rgb(palette.danger))
                        })
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.delete_command_suggestion_history(delete_command.clone(), cx);
                        }))
                        .child("×"),
                );
            }
            list = list.child(row);
        }

        div()
            .id(SharedString::from("command-suggestions-overlay"))
            .absolute()
            .left(px(x))
            .top(px(y))
            .w(px(menu_w))
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface_elevated))
            .shadow_lg()
            .overflow_hidden()
            .child(
                div()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(FontWeight(600.))
                            .text_color(rgb(palette.text_dimmed))
                            .child("SUGGESTIONS"),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(format!("{}", state.items.len())),
                    ),
            )
            .child(list)
            .child(
                div()
                    .px_2()
                    .py_1()
                    .border_t_1()
                    .border_color(rgb(palette.border))
                    .text_size(px(10.))
                    .text_color(rgb(palette.text_dimmed))
                    .child("↑↓ select · Enter run · Tab fill · Esc dismiss"),
            )
            .into_any_element()
    }

    pub(in crate::features) fn suggestion_overlay_position_for_session(
        &self,
        session_id: Option<&str>,
        cursor_row: usize,
        cursor_col: usize,
        menu_w: f32,
        menu_h: f32,
    ) -> Option<(f32, f32)> {
        let bounds = self.terminal_surface_bounds_for_session(session_id)?;
        Some(suggestion_overlay_position(
            bounds,
            self.terminal_cell_size(),
            self.terminal_content_padding_px(),
            self.terminal_gutter_width_px(),
            self.last_viewport_size,
            cursor_row,
            cursor_col,
            menu_w,
            menu_h,
        ))
    }
}

pub(in crate::features) fn suggestion_overlay_position(
    bounds: Bounds<Pixels>,
    cell_size: (f32, f32),
    pad: f32,
    gutter: f32,
    viewport_size: (f32, f32),
    cursor_row: usize,
    cursor_col: usize,
    menu_w: f32,
    menu_h: f32,
) -> (f32, f32) {
    let (cell_w, cell_h) = cell_size;
    let base_x = f32::from(bounds.origin.x) + pad + gutter + cursor_col as f32 * cell_w;
    let base_y = f32::from(bounds.origin.y) + pad + (cursor_row as f32 + 1.0) * cell_h;
    let (viewport_w, viewport_h) = viewport_size;
    let mut x = base_x;
    let mut y = base_y + 4.0;
    if x + menu_w + 8.0 > viewport_w {
        x = (viewport_w - menu_w - 8.0).max(8.0);
    }
    if y + menu_h + 8.0 > viewport_h {
        y = (base_y - menu_h - 4.0).max(8.0);
    }
    (x.max(8.0), y.max(8.0))
}

#[cfg(test)]
mod overlay_position_tests {
    use super::*;
    use gpui::{point, size};

    #[test]
    fn suggestion_overlay_position_anchors_below_cursor() {
        let (x, y) = suggestion_overlay_position(
            Bounds::new(point(px(10.0), px(20.0)), size(px(800.0), px(400.0))),
            (8.0, 16.0),
            8.0,
            0.0,
            (1024.0, 768.0),
            2,
            4,
            300.0,
            120.0,
        );

        assert_eq!(x, 50.0);
        assert_eq!(y, 80.0);
    }

    #[test]
    fn suggestion_overlay_position_clamps_and_flips_inside_viewport() {
        let (x, y) = suggestion_overlay_position(
            Bounds::new(point(px(700.0), px(500.0)), size(px(200.0), px(160.0))),
            (8.0, 16.0),
            8.0,
            0.0,
            (900.0, 620.0),
            4,
            30,
            300.0,
            140.0,
        );

        assert_eq!(x, 592.0);
        assert_eq!(y, 444.0);
    }
}

fn terminal_line_prefix_for_cell_col(line: &str, cell_col: usize) -> String {
    let end = terminal_byte_index_for_cell_col(line, cell_col);
    line.get(..end).unwrap_or(line).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_line_prefix_uses_terminal_cells_for_wide_chars() {
        assert_eq!(terminal_line_prefix_for_cell_col("界x", 0), "");
        assert_eq!(terminal_line_prefix_for_cell_col("界x", 1), "");
        assert_eq!(terminal_line_prefix_for_cell_col("界x", 2), "界");
        assert_eq!(terminal_line_prefix_for_cell_col("界x", 3), "界x");
    }

    #[test]
    fn terminal_line_prefix_keeps_combining_mark_with_base_char() {
        let text = "e\u{301}x";

        assert_eq!(terminal_line_prefix_for_cell_col(text, 0), "");
        assert_eq!(terminal_line_prefix_for_cell_col(text, 1), "e\u{301}");
        assert_eq!(terminal_line_prefix_for_cell_col(text, 2), "e\u{301}x");
    }
}

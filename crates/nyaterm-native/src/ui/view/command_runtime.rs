use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn insert_ai_command_card(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        self.apply_ai_command_card(index, false, cx);
    }

    pub(in crate::ui::view) fn run_ai_command_card(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        self.apply_ai_command_card(index, true, cx);
    }

    pub(in crate::ui::view) fn insert_ai_command_card_by_id(
        &mut self,
        card_id: String,
        cx: &mut Context<Self>,
    ) {
        self.apply_ai_command_card_by_id(card_id, false, cx);
    }

    pub(in crate::ui::view) fn run_ai_command_card_by_id(
        &mut self,
        card_id: String,
        cx: &mut Context<Self>,
    ) {
        self.apply_ai_command_card_by_id(card_id, true, cx);
    }

    pub(in crate::ui::view) fn find_ai_command_card(&self, card_id: &str) -> Option<AiCommandCard> {
        self.ai_command_cards
            .iter()
            .find(|card| card.id == card_id)
            .cloned()
            .or_else(|| {
                self.ai_chat_messages
                    .iter()
                    .flat_map(|message| message.command_cards.iter())
                    .find(|card| card.id == card_id)
                    .cloned()
            })
    }

    pub(in crate::ui::view) fn active_session_history_commands(&self) -> Vec<String> {
        self.active_session_id
            .as_deref()
            .and_then(|session_id| self.session_command_history.get(session_id))
            .cloned()
            .unwrap_or_default()
    }

    fn active_session_history_command(&self, index: usize) -> Option<String> {
        let session_id = self.active_session_id.as_deref()?;
        self.session_command_history
            .get(session_id)?
            .get(index)
            .cloned()
    }

    pub(in crate::ui::view) fn insert_history_command(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        self.apply_history_command(index, false, cx);
    }

    pub(in crate::ui::view) fn run_history_command(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        self.apply_history_command(index, true, cx);
    }

    pub(in crate::ui::view) fn insert_command_search_result(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        self.apply_command_search_result(index, false, cx);
    }

    pub(in crate::ui::view) fn run_command_search_result(
        &mut self,
        index: usize,
        cx: &mut Context<Self>,
    ) {
        self.apply_command_search_result(index, true, cx);
    }

    fn apply_command_search_result(&mut self, index: usize, execute: bool, cx: &mut Context<Self>) {
        if self.active_session_id.is_none() {
            self.terminal_status =
                "start a terminal session before using command search".to_string();
            cx.notify();
            return;
        }
        let Some(result) = self.command_search_results().into_iter().nth(index) else {
            self.terminal_status = "command search result is no longer available".to_string();
            cx.notify();
            return;
        };
        let mut command = result.command.trim().to_string();
        if command.is_empty() {
            self.terminal_status = "command search result is empty".to_string();
            cx.notify();
            return;
        }
        if execute && !command.ends_with('\n') {
            command.push('\n');
        }
        self.send_terminal_input(command.into_bytes(), cx);
        self.terminal_status = if execute {
            format!("ran search result '{}'", result.display)
        } else {
            format!("inserted search result '{}'", result.display)
        };
        cx.notify();
    }

    fn apply_history_command(&mut self, index: usize, execute: bool, cx: &mut Context<Self>) {
        if self.active_session_id.is_none() {
            self.terminal_status = "start a terminal session before using history".to_string();
            cx.notify();
            return;
        }
        let Some(command_text) = self.active_session_history_command(index) else {
            self.terminal_status = "history command is no longer available".to_string();
            cx.notify();
            return;
        };
        let mut command = command_text.trim().to_string();
        if command.is_empty() {
            self.terminal_status = "history command is empty".to_string();
            cx.notify();
            return;
        }
        if execute && !command.ends_with('\n') {
            command.push('\n');
        }
        self.send_terminal_input(command.into_bytes(), cx);
        self.terminal_status = if execute {
            format!("ran history command '{command_text}'")
        } else {
            format!("inserted history command '{command_text}'")
        };
        cx.notify();
    }

    fn apply_ai_command_card(&mut self, index: usize, execute: bool, cx: &mut Context<Self>) {
        let Some(card) = self.ai_command_cards.get(index).cloned() else {
            self.ai_status = "AI command card is no longer available".to_string();
            cx.notify();
            return;
        };
        self.apply_ai_command_card_value(card, execute, cx);
    }

    fn apply_ai_command_card_by_id(
        &mut self,
        card_id: String,
        execute: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(card) = self.find_ai_command_card(&card_id) else {
            self.ai_status = "AI command card is no longer available".to_string();
            cx.notify();
            return;
        };
        self.apply_ai_command_card_value(card, execute, cx);
    }

    fn apply_ai_command_card_value(
        &mut self,
        card: AiCommandCard,
        execute: bool,
        cx: &mut Context<Self>,
    ) {
        if self.active_session_id.is_none() {
            self.ai_status = "Start a terminal session before using an AI command".to_string();
            cx.notify();
            return;
        }
        let mut command = card.command.trim().to_string();
        if command.is_empty() {
            self.ai_status = "AI command card has no command".to_string();
            cx.notify();
            return;
        }
        let should_continue_agent = execute && is_agent_command_card(&card);
        if should_continue_agent && self.ai_settings.agent_background_execution_enabled {
            match self.begin_ai_agent_background_execution(&card.command, cx) {
                Ok(()) => {
                    self.record_ai_command_card_audit(&card, true, false);
                    cx.notify();
                }
                Err(error) => {
                    self.ai_status = error;
                    let step_index = self
                        .ai_agent_steps
                        .last()
                        .map(|step| step.step_index)
                        .unwrap_or(0);
                    self.upsert_ai_agent_step(
                        step_index,
                        AiAgentStepStatus::Failed,
                        "Failed",
                        self.ai_status.clone(),
                    );
                    cx.notify();
                }
            }
            return;
        }
        if execute && !command.ends_with('\n') {
            command.push('\n');
        }
        let input_bytes = if should_continue_agent {
            match self.begin_ai_agent_observation(&card.command) {
                Ok(Some(wrapped_command)) => wrapped_command.into_bytes(),
                Ok(None) => command.clone().into_bytes(),
                Err(error) => {
                    self.ai_status = error;
                    let step_index = self
                        .ai_agent_steps
                        .last()
                        .map(|step| step.step_index)
                        .unwrap_or(0);
                    self.upsert_ai_agent_step(
                        step_index,
                        AiAgentStepStatus::Failed,
                        "Failed",
                        self.ai_status.clone(),
                    );
                    cx.notify();
                    return;
                }
            }
        } else {
            command.clone().into_bytes()
        };

        self.record_ai_command_card_audit(&card, execute, true);

        self.send_terminal_input(input_bytes, cx);
        self.ai_status = if should_continue_agent {
            if let Some(state) = self.ai_agent_loop.as_ref().cloned() {
                self.upsert_ai_agent_step(
                    state.step_index,
                    AiAgentStepStatus::Running,
                    "Running",
                    truncate_preview(&state.command, 140),
                );
                format!(
                    "AI Agent observing command output for step {}/{}",
                    state.step_index + 1,
                    state.max_steps
                )
            } else {
                format!("Ran AI command card '{}'", card.title)
            }
        } else if execute {
            format!("Ran AI command card '{}'", card.title)
        } else {
            format!("Inserted AI command card '{}'", card.title)
        };
        cx.notify();
    }

    pub(in crate::ui::view) fn record_command_history_from_bytes(
        &mut self,
        session_id: Option<&str>,
        bytes: &[u8],
    ) {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return;
        };
        if !text.contains('\n') && !text.contains('\r') {
            return;
        }
        let submitted: Vec<String> = text
            .split(['\r', '\n'])
            .map(str::trim)
            .filter(|command| !command.is_empty())
            .map(ToOwned::to_owned)
            .collect();
        if submitted.is_empty() {
            return;
        }
        if let Some(session_id) = session_id {
            for command in &submitted {
                self.record_session_command_history(session_id, command);
            }
        }
        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            Ok(store) => {
                for command in submitted {
                    if let Err(error) = store.append_command_history(&command) {
                        self.store_status.message = format!("command history save failed: {error}");
                        self.store_status.ready = false;
                        return;
                    }
                }
                self.command_history = store.list_command_history(64).unwrap_or_default();
            }
            Err(error) => {
                self.store_status.message = format!("command history store failed: {error}");
                self.store_status.ready = false;
            }
        }
    }

    fn record_session_command_history(&mut self, session_id: &str, command: &str) {
        let normalized_command = command.trim();
        if normalized_command.is_empty() {
            return;
        }
        let history = self
            .session_command_history
            .entry(session_id.to_string())
            .or_default();
        history.insert(0, normalized_command.to_string());
        history.truncate(SESSION_COMMAND_HISTORY_LIMIT);
    }

    pub(in crate::ui::view) fn handle_command_search_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                self.command_search_draft.pop();
                cx.notify();
            }
            "escape" => {
                self.command_search_draft.clear();
                self.terminal_status = "command search cleared".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.command_search_draft.push_str(input);
                    cx.notify();
                }
            }
        }
    }


    pub(in crate::ui::view) fn dismiss_command_suggestions(&mut self, cx: &mut Context<Self>) {
        let mut changed = false;
        if self.command_suggestions.take().is_some() {
            changed = true;
        }
        // Keep draft for continued typing; only clear list visibility.
        if changed {
            cx.notify();
        }
    }

    pub(in crate::ui::view) fn clear_command_suggestion_draft(&mut self, cx: &mut Context<Self>) {
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

    pub(in crate::ui::view) fn note_command_suggestion_input(
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
        self.command_input_tracker = apply_terminal_input_data(&self.command_input_tracker, text);
        if !can_suggest_from_tracker(&self.command_input_tracker) {
            if self.command_suggestions.take().is_some() {
                cx.notify();
            }
            return;
        }
        self.refresh_command_suggestions(cx);
    }

    pub(in crate::ui::view) fn refresh_command_suggestions(&mut self, cx: &mut Context<Self>) {
        if self.credential_suggestions.is_some() || self.is_credential_prompt_input_mode() {
            self.command_suggestions = None;
            return;
        }
        if !self.settings.interaction_command_suggestions_enabled {
            self.command_suggestions = None;
            cx.notify();
            return;
        }
        let pattern = get_tracked_command(&self.command_input_tracker);
        let min_chars = self.settings.interaction_command_suggestion_min_chars.max(1) as usize;
        let max_chars = self
            .settings
            .interaction_command_suggestion_max_chars
            .max(min_chars as u32) as usize;
        if pattern.chars().count() < min_chars {
            self.command_suggestions = None;
            cx.notify();
            return;
        }
        // Pager/search-like prefixes: hide suggestions.
        if pattern.starts_with('/') || pattern.starts_with('?') || pattern.starts_with(':') {
            self.command_suggestions = None;
            cx.notify();
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
            self.command_suggestions = None;
            cx.notify();
            return;
        }
        let (cursor_row, cursor_col) = self.active_terminal_cursor_cell();
        let selected_index = self
            .command_suggestions
            .as_ref()
            .map(|state| state.selected_index.min(results.len().saturating_sub(1)))
            .unwrap_or(0);
        self.command_suggestions = Some(CommandSuggestionState {
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

    fn active_terminal_cursor_cell(&self) -> (usize, usize) {
        let offset = self.active_terminal_scroll_offset();
        let snapshot = self
            .active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal_views.get(session_id))
            .map(|view| view.screen.viewport_snapshot(offset))
            .unwrap_or_else(|| self.terminal_screen.viewport_snapshot(offset));
        let row = if snapshot.cursor_row == usize::MAX {
            snapshot.lines.len().saturating_sub(1)
        } else {
            snapshot.cursor_row
        };
        (row, snapshot.cursor_col)
    }

    /// Handle suggestion popup keys. Returns true when the key was consumed.
    pub(in crate::ui::view) fn handle_command_suggestion_key(
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

    pub(in crate::ui::view) fn apply_selected_command_suggestion(
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
            payload.push('\n');
        }
        self.command_input_tracker = TerminalInputState::new();
        self.command_suggestions = None;
        self.send_terminal_input_without_suggestion_track(payload.into_bytes(), cx);
        if !execute {
            // After fill, tracker becomes the filled command for continued typing.
            self.command_input_tracker = apply_terminal_input_data(&TerminalInputState::new(), &command);
            self.refresh_command_suggestions(cx);
        }
        self.terminal_status = if execute {
            format!("executed suggestion from {source}")
        } else {
            format!("filled suggestion from {source}")
        };
        cx.notify();
    }

    pub(in crate::ui::view) fn delete_command_suggestion_history(
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
            state.items.retain(|item| {
                !(item.source == "history" && item.command == command)
            });
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

    pub(in crate::ui::view) fn command_suggestions_overlay(
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
        let bounds = self.terminal_surface_bounds;
        let (cell_w, cell_h) = self.terminal_cell_size();
        let pad = self.terminal_content_padding_px();
        let gutter = self.terminal_gutter_width_px();
        let (base_x, base_y) = if let Some(bounds) = bounds {
            (
                f32::from(bounds.origin.x) + pad + gutter + state.cursor_col as f32 * cell_w,
                f32::from(bounds.origin.y) + pad + (state.cursor_row as f32 + 1.0) * cell_h,
            )
        } else {
            (24.0, 120.0)
        };
        let (viewport_w, viewport_h) = self.last_viewport_size;
        let menu_w = 360.0_f32;
        let menu_h = (state.items.len() as f32 * 28.0 + 44.0).min(320.0);
        let mut x = base_x;
        let mut y = base_y + 4.0;
        if x + menu_w + 8.0 > viewport_w {
            x = (viewport_w - menu_w - 8.0).max(8.0);
        }
        if y + menu_h + 8.0 > viewport_h {
            y = (base_y - menu_h - 4.0).max(8.0);
        }

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
                .border_color(rgb(if selected { palette.accent } else { palette.surface }))
                .bg(rgb(if selected { palette.hover } else { palette.surface }))
                .text_size(px(12.))
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
                        .font_family("JetBrains Mono")
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
                        .id(SharedString::from(format!("command-suggestion-del-{index}")))
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
            .left(px(x.max(8.0)))
            .top(px(y.max(8.0)))
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
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_muted))
                            .child("Suggestions"),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(format!("{} matches", state.items.len())),
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

    pub(in crate::ui::view) fn command_search_results(&self) -> Vec<nyaterm_domain::FuzzyResult> {
        search_command_sources(
            &self.command_history,
            &self.quick_commands,
            &self.command_search_draft,
            8,
            Some(1),
            Some(512),
        )
    }
}

fn command_suggestion_highlight_parts(
    text: &str,
    indices: &[u32],
    palette: ThemePalette,
    selected: bool,
) -> Vec<gpui::AnyElement> {
    let _selected = selected;
    let clipped = truncate_preview(text, 48);
    let index_set: HashSet<u32> = indices.iter().copied().collect();
    let chars: Vec<char> = clipped.chars().collect();
    let mut parts = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let highlighted = index_set.contains(&(i as u32));
        let start = i;
        i += 1;
        while i < chars.len() && index_set.contains(&(i as u32)) == highlighted {
            i += 1;
        }
        let chunk: String = chars[start..i].iter().collect();
        let color = if highlighted {
            palette.accent
        } else {
            palette.text
        };
        parts.push(
            div()
                .text_color(rgb(color))
                .when(highlighted, |this| this.font_weight(FontWeight::SEMIBOLD))
                .child(chunk)
                .into_any_element(),
        );
    }
    if parts.is_empty() {
        parts.push(
            div()
                .text_color(rgb(palette.text))
                .child(clipped)
                .into_any_element(),
        );
    }
    parts
}


use super::*;

impl NyaTermApp {
    pub(in crate::features) fn clear_action_link_tooltip(&mut self, cx: &mut Context<Self>) {
        let mut changed = false;
        if self.action_link_tooltip.take().is_some() {
            changed = true;
        }
        if self.action_link_hover_pending.take().is_some() {
            changed = true;
        }
        if changed {
            cx.notify();
        }
    }

    pub(in crate::features) fn poll_action_link_tooltip_delay(
        &mut self,
        _cx: &mut Context<Self>,
    ) -> bool {
        let Some((key, started, tip)) = self.action_link_hover_pending.clone() else {
            return false;
        };
        if started.elapsed() < Duration::from_millis(250) {
            return false;
        }
        self.action_link_hover_pending = None;
        // Only show if still matching the pending key (not superseded).
        if self
            .action_link_tooltip
            .as_ref()
            .is_some_and(|current| current.match_key == key)
        {
            return true;
        }
        self.action_link_tooltip = Some(tip);
        true
    }

    pub(in crate::features) fn update_action_link_hover(
        &mut self,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        if !self.settings.terminal_action_links_enabled {
            self.clear_action_link_tooltip(cx);
            return;
        }
        // Hide while menus are open or while selecting text.
        if self.action_link_menu.is_some()
            || self.terminal_context_menu.is_some()
            || self.terminal_selection_dragging
            || self.translation_dialog.is_some()
        {
            self.clear_action_link_tooltip(cx);
            return;
        }
        let Some((item, actions)) = self.action_link_at_point(event.position) else {
            self.clear_action_link_tooltip(cx);
            return;
        };
        if actions.is_empty() {
            self.clear_action_link_tooltip(cx);
            return;
        }
        let default = actions
            .iter()
            .find(|action| action.is_default)
            .cloned()
            .or_else(|| actions.first().cloned());
        let Some(default) = default else {
            self.clear_action_link_tooltip(cx);
            return;
        };
        let match_key = format!(
            "{}|{}|{}|{}",
            item.kind.label(),
            item.value,
            item.start,
            item.end
        );
        let preview = default
            .command
            .clone()
            .or_else(|| default.open_url.clone())
            .unwrap_or_else(|| default.label.clone());
        let next = ActionLinkTooltipState {
            x: event.position.x,
            y: event.position.y,
            kind_label: item.kind.label().to_string(),
            value: item.value.clone(),
            default_action_label: default.label.clone(),
            default_action_preview: preview,
            has_more_actions: actions.len() > 1,
            match_key: match_key.clone(),
        };
        // Already visible for this link: track position.
        if let Some(current) = self.action_link_tooltip.as_ref() {
            if current.match_key == match_key {
                if current.x != next.x || current.y != next.y {
                    self.action_link_tooltip = Some(next);
                    cx.notify();
                }
                return;
            }
        }
        // Pending same link: update position only.
        if let Some((key, started, _)) = self.action_link_hover_pending.clone() {
            if key == match_key {
                let ready = started.elapsed() >= Duration::from_millis(250);
                self.action_link_hover_pending = Some((match_key, started, next));
                if ready {
                    self.poll_action_link_tooltip_delay(cx);
                }
                return;
            }
        }
        // New link under cursor: start 250ms delay (Tauri ActionLinkTooltip).
        self.action_link_tooltip = None;
        self.action_link_hover_pending = Some((match_key, Instant::now(), next));
        cx.notify();
    }

    pub(in crate::features) fn action_link_at_point(
        &self,
        position: Point<Pixels>,
    ) -> Option<(ActionLinkMatch, Vec<ActionLinkAction>)> {
        // Only hit-test when the pointer is over the painted terminal content area.
        let bounds = self.terminal_surface_bounds?;
        let (cell_w, cell_h) = self.terminal_cell_size();
        let pad = self.terminal_content_padding_px();
        let gutter = self.terminal_gutter_width_px();
        let local_x = f32::from(position.x - bounds.origin.x) - pad - gutter;
        let local_y = f32::from(position.y - bounds.origin.y) - pad;
        if local_x < 0. || local_y < 0. {
            return None;
        }
        let (rows, cols) = self.active_terminal_grid_size();
        if local_y >= cell_h * rows as f32 || local_x >= cell_w * cols as f32 {
            return None;
        }
        let cell = self.point_to_terminal_cell(position)?;
        let offset = self.active_terminal_scroll_offset();
        let snapshot = self
            .active_session_id
            .as_deref()
            .and_then(|session_id| self.terminal_views.get(session_id))
            .map(|view| view.screen.viewport_snapshot(offset))
            .unwrap_or_else(|| self.terminal_screen.viewport_snapshot(offset));
        let line = snapshot.lines.get(cell.row)?;
        let chars: Vec<char> = line.chars().collect();
        if chars.is_empty() {
            return None;
        }
        let char_offset = cell.col.min(chars.len().saturating_sub(1));
        let byte_offset: usize = chars.iter().take(char_offset).map(|ch| ch.len_utf8()).sum();
        let matchers = &self.settings.terminal_action_links_matchers;
        let item = match_at_offset(line, byte_offset, matchers)?;
        let actions = actions_for_match(&item);
        Some((item, actions))
    }

    pub(in crate::features) fn close_action_link_menu(&mut self, cx: &mut Context<Self>) {
        let mut changed = false;
        if self.action_link_menu.take().is_some() {
            self.terminal_status = "action link menu closed".to_string();
            changed = true;
        }
        if self.action_link_tooltip.take().is_some() {
            changed = true;
        }
        if changed {
            cx.notify();
        }
    }

    pub(in crate::features) fn try_open_action_link_menu_at_click(
        &mut self,
        event: &ClickEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some((item, actions)) = self.action_link_at_click(event) else {
            return false;
        };
        if actions.is_empty() {
            return false;
        }
        let menu_actions = actions
            .into_iter()
            .map(|action| ActionLinkMenuAction {
                id: action.id,
                label: action.label,
                command: action.command,
                open_url: action.open_url,
                is_default: action.is_default,
            })
            .collect::<Vec<_>>();
        self.action_link_tooltip = None;
        self.command_suggestions = None;
        self.credential_suggestions = None;
        self.action_link_menu = Some(ActionLinkMenuState {
            x: event.position().x,
            y: event.position().y,
            kind_label: item.kind.label().to_string(),
            value: item.value,
            actions: menu_actions,
        });
        self.terminal_context_menu = None;
        self.terminal_status = format!("action link menu: {}", item.kind.label());
        cx.notify();
        true
    }

    pub(in crate::features) fn action_link_at_click(
        &self,
        event: &ClickEvent,
    ) -> Option<(ActionLinkMatch, Vec<ActionLinkAction>)> {
        self.action_link_at_point(event.position())
    }

    /// Ctrl/Cmd-click OSC 8 hyperlinks (uri from the terminal screen model).

    pub(in crate::features) fn try_activate_osc8_hyperlink_at_click(
        &mut self,
        event: &ClickEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(pos) = self.point_to_terminal_cell(event.position()) else {
            return false;
        };
        let session_id = self.active_session_id.clone().unwrap_or_default();
        let scroll_offset = self.active_terminal_scroll_offset();
        let snapshot = self
            .terminal_views
            .get(&session_id)
            .map(|view| view.screen.viewport_snapshot(scroll_offset))
            .unwrap_or_else(|| self.terminal_screen.viewport_snapshot(scroll_offset));
        let Some(spans) = snapshot.hyperlink_lines.get(pos.row) else {
            return false;
        };
        let col = pos.col;
        let Some(span) = spans
            .iter()
            .find(|span| col >= span.start_col && col <= span.end_col)
        else {
            return false;
        };
        let url = span.uri.clone();
        // Only open common URL schemes for safety (Tauri oscLinkHandler parity).
        let lower = url.to_ascii_lowercase();
        if !(lower.starts_with("http://")
            || lower.starts_with("https://")
            || lower.starts_with("mailto:"))
        {
            self.terminal_status = format!("blocked OSC 8 scheme: {url}");
            cx.notify();
            return true;
        }
        match open_external_url_for_action(&url) {
            Ok(()) => self.terminal_status = format!("opened OSC 8 link: {url}"),
            Err(error) => self.terminal_status = format!("open OSC 8 link failed: {error}"),
        }
        cx.notify();
        true
    }

    pub(in crate::features) fn try_activate_action_link_at_click(
        &mut self,
        event: &ClickEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.try_activate_osc8_hyperlink_at_click(event, cx) {
            return true;
        }
        let Some((item, actions)) = self.action_link_at_click(event) else {
            return false;
        };
        self.action_link_tooltip = None;
        let Some(default) = actions
            .iter()
            .find(|action| action.is_default)
            .cloned()
            .or_else(|| actions.first().cloned())
        else {
            return false;
        };
        if let Some(url) = default.open_url {
            match open_external_url_for_action(&url) {
                Ok(()) => self.terminal_status = format!("opened {}: {url}", item.kind.label()),
                Err(error) => self.terminal_status = format!("open link failed: {error}"),
            }
            cx.notify();
            return true;
        }
        if let Some(command) = default.command {
            self.execute_action_link_command(command, cx);
            return true;
        }
        false
    }
}

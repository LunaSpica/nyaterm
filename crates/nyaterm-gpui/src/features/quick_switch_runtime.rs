use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn open_quick_switch(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.quick_switch_open = true;
        self.quick_switch_query.clear();
        self.quick_switch_selected_index = 0;
        self.terminal_status = "quick switch opened".to_string();
        window.focus(&self.quick_switch_focus);
        cx.notify();
    }

    pub(in crate::ui::view) fn close_quick_switch(&mut self, cx: &mut Context<Self>) {
        self.quick_switch_open = false;
        self.quick_switch_query.clear();
        self.quick_switch_selected_index = 0;
        self.terminal_status = "quick switch closed".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn quick_switch_items(&self) -> Vec<QuickSwitchItem> {
        let mut items = Vec::new();
        if let Some(pending_name) = self.pending_session_name.clone() {
            items.push(QuickSwitchItem::Pending {
                title: format!("Connecting {pending_name}"),
                subtitle: "pending SSH session".to_string(),
            });
        }

        for session in self.ordered_sessions() {
            let title = self.session_display_name_by_info(&session);
            let active = self.active_session_id.as_deref() == Some(session.id.as_str());
            let unread = self
                .terminal_views
                .get(&session.id)
                .is_some_and(|view| view.has_unread);
            let mut subtitle = format!(
                "{} - {}",
                session_kind_label(session.kind),
                short_id(&session.id)
            );
            if let Some(path) = session.working_dir.as_ref() {
                subtitle.push_str(" - ");
                subtitle.push_str(&path.display().to_string());
            }
            items.push(QuickSwitchItem::Session {
                id: session.id,
                title,
                subtitle,
                active,
                unread,
            });
        }

        let mut connections = self.connections.clone();
        connections.sort_by(|left, right| {
            left.name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase())
                .then(left.id.cmp(&right.id))
        });
        for connection in connections {
            items.push(QuickSwitchItem::Connection {
                title: connection.name.clone(),
                subtitle: format!("{} - {}", connection.kind_label(), connection.endpoint()),
                connection,
            });
        }
        items
    }

    pub(in crate::ui::view) fn filtered_quick_switch_items(&self) -> Vec<QuickSwitchItem> {
        let items = self.quick_switch_items();
        let query = self.quick_switch_query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return items;
        }
        let mut scored = items
            .into_iter()
            .filter_map(|item| {
                let text = item.search_text().to_ascii_lowercase();
                text.find(&query).map(|index| (index, item))
            })
            .collect::<Vec<_>>();
        scored.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then(left.1.title().cmp(right.1.title()))
        });
        scored.into_iter().map(|(_, item)| item).collect()
    }

    pub(in crate::ui::view) fn select_quick_switch_item(
        &mut self,
        item: QuickSwitchItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.quick_switch_open = false;
        self.quick_switch_query.clear();
        self.quick_switch_selected_index = 0;
        match item {
            QuickSwitchItem::Session { id, .. } => {
                self.select_session(id, cx);
            }
            QuickSwitchItem::Connection { connection, .. } => {
                self.start_saved_connection(connection, window, cx);
            }
            QuickSwitchItem::Pending { .. } => {
                self.terminal_status = "session is still connecting".to_string();
                cx.notify();
            }
        }
    }

    pub(in crate::ui::view) fn handle_quick_switch_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let items = self.filtered_quick_switch_items();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "escape" => self.close_quick_switch(cx),
            "up" => {
                if !items.is_empty() {
                    self.quick_switch_selected_index =
                        (self.quick_switch_selected_index + items.len() - 1) % items.len();
                }
                cx.notify();
            }
            "down" => {
                if !items.is_empty() {
                    self.quick_switch_selected_index =
                        (self.quick_switch_selected_index + 1) % items.len();
                }
                cx.notify();
            }
            "enter" => {
                if let Some(item) = items
                    .get(
                        self.quick_switch_selected_index
                            .min(items.len().saturating_sub(1)),
                    )
                    .cloned()
                {
                    self.select_quick_switch_item(item, window, cx);
                }
            }
            "backspace" => {
                self.quick_switch_query.pop();
                self.quick_switch_selected_index = 0;
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.quick_switch_query.push_str(input);
                    self.quick_switch_selected_index = 0;
                    cx.notify();
                }
            }
        }
    }
}

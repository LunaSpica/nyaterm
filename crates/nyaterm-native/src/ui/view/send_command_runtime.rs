use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn handle_send_command_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.alt || keystroke.modifiers.function {
            return;
        }

        match keystroke.key.as_str() {
            "enter" => {
                self.send_bottom_command(true, cx);
            }
            "backspace" if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                if self.send_command_data_type == SendCommandDataType::Hex {
                    // Delete last hex digit (ignore spacing), then reformat pairs.
                    let mut cleaned: String = self
                        .send_command_draft
                        .chars()
                        .filter(|ch| ch.is_ascii_hexdigit())
                        .collect();
                    cleaned.pop();
                    self.send_command_draft = format_send_command_hex_display(&cleaned);
                    self.clamp_send_command_hex_scroll();
                } else {
                    self.send_command_draft.pop();
                }
                cx.notify();
            }
            "tab" if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                if self.send_command_data_type != SendCommandDataType::Hex {
                    self.send_command_draft.push_str("\t");
                    cx.notify();
                }
            }
            "escape" => {
                self.send_command_draft.clear();
                self.terminal_status = "command send cleared".to_string();
                cx.notify();
            }
            _ if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    if self.send_command_data_type == SendCommandDataType::Hex {
                        let filtered: String = input
                            .chars()
                            .filter(|ch| ch.is_ascii_hexdigit() || ch.is_whitespace())
                            .collect();
                        if filtered.is_empty() {
                            return;
                        }
                        self.send_command_draft.push_str(&filtered);
                        self.send_command_draft =
                            format_send_command_hex_display(&self.send_command_draft);
                        self.clamp_send_command_hex_scroll();
                    } else {
                        self.send_command_draft.push_str(input);
                    }
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    pub(in crate::ui::view) fn send_bottom_command(
        &mut self,
        append_enter: bool,
        cx: &mut Context<Self>,
    ) {
        if self.send_command_sending {
            self.stop_send_command(cx);
            return;
        }

        let session_kind = self.active_session_kind();
        let mut draft = self.send_command_draft.clone();
        if append_enter && self.send_command_data_type == SendCommandDataType::Text {
            draft.push('\n');
        }
        let units = match self.build_send_command_units(&draft, session_kind) {
            Ok(units) => units,
            Err(message) => {
                self.terminal_status = message;
                cx.notify();
                return;
            }
        };
        if units.is_empty() {
            self.terminal_status = "command send is empty".to_string();
            cx.notify();
            return;
        }
        let target_session_ids = self.send_command_target_session_ids();
        if target_session_ids.is_empty() {
            self.terminal_status = "start a session before sending".to_string();
            cx.notify();
            return;
        }

        // None => infinite rounds (Tauri SendCommandCount null / ∞).
        let infinite = self.send_command_count.is_none();
        let rounds = self.send_command_count.unwrap_or(1).max(1);
        let interval = self.send_command_interval_seconds.max(0.0);
        let units_per_round = units.len() as u32;
        let total_units = if infinite {
            0
        } else {
            units_per_round.saturating_mul(rounds)
        };
        let cancel = Arc::new(AtomicBool::new(false));
        self.send_command_cancel = Some(cancel.clone());
        self.send_command_sending = true;
        self.send_command_progress_completed = 0;
        self.send_command_progress_total = total_units;
        self.send_command_progress_round = 0;
        self.send_command_progress_rounds = if infinite { 0 } else { rounds };
        self.terminal_status = if infinite {
            format!("sending {units_per_round} unit(s) × ∞")
        } else {
            format!("sending {units_per_round} unit(s) × {rounds}")
        };
        cx.notify();

        cx.spawn(async move |this, cx| {
            let mut first = true;
            let mut aborted = false;
            let mut round = 0u32;
            'outer: loop {
                if !infinite && round >= rounds {
                    break;
                }
                if cancel.load(Ordering::SeqCst) {
                    aborted = true;
                    break;
                }
                round = round.saturating_add(1);
                let _ = this.update(cx, |this, cx| {
                    this.send_command_progress_round = round;
                    cx.notify();
                });
                for unit in &units {
                    if cancel.load(Ordering::SeqCst) {
                        aborted = true;
                        break 'outer;
                    }
                    if !first && interval > 0.0 {
                        Timer::after(Duration::from_secs_f64(interval)).await;
                        if cancel.load(Ordering::SeqCst) {
                            aborted = true;
                            break 'outer;
                        }
                    }
                    first = false;
                    let unit = unit.clone();
                    let targets = target_session_ids.clone();
                    let _ = this.update(cx, |this, cx| {
                        for session_id in &targets {
                            this.send_terminal_input_to_session(
                                session_id.clone(),
                                unit.clone(),
                                cx,
                            );
                        }
                        this.send_command_progress_completed =
                            this.send_command_progress_completed.saturating_add(1);
                        cx.notify();
                    });
                }
            }
            let _ = this.update(cx, |this, cx| {
                this.send_command_sending = false;
                this.send_command_cancel = None;
                if aborted {
                    this.terminal_status = if infinite {
                        format!(
                            "command send stopped at {} unit(s) · round {}",
                            this.send_command_progress_completed,
                            this.send_command_progress_round
                        )
                    } else {
                        format!(
                            "command send stopped at {}/{}",
                            this.send_command_progress_completed,
                            this.send_command_progress_total
                        )
                    };
                } else if infinite {
                    this.terminal_status = format!(
                        "command send completed: {} unit(s)",
                        this.send_command_progress_completed
                    );
                } else {
                    this.terminal_status =
                        format!("command send completed: {rounds} round(s)");
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(in crate::ui::view) fn stop_send_command(&mut self, cx: &mut Context<Self>) {
        if let Some(cancel) = self.send_command_cancel.as_ref() {
            cancel.store(true, Ordering::SeqCst);
            self.terminal_status = "stopping command send…".to_string();
            cx.notify();
        }
    }


    pub(in crate::ui::view) fn send_command_target_session_ids(&self) -> Vec<String> {
        let sessions = self.session_manager.list_sessions().unwrap_or_default();
        let active_kind = self.active_session_kind();
        let is_compatible = |kind: SessionKind| -> bool {
            match active_kind {
                Some(SessionKind::Serial) => matches!(kind, SessionKind::Serial),
                Some(_) => !matches!(kind, SessionKind::Serial),
                None => true,
            }
        };
        match &self.send_command_target {
            SendCommandTarget::Current => self.active_session_id.clone().into_iter().collect(),
            SendCommandTarget::AllCompatible => {
                if active_kind.is_none() {
                    return Vec::new();
                }
                sessions
                    .into_iter()
                    .filter(|session| is_compatible(session.kind))
                    .map(|session| session.id)
                    .collect()
            }
            SendCommandTarget::Group(group_id) => {
                let Some(group) = self.sync_groups.iter().find(|group| &group.id == group_id) else {
                    return Vec::new();
                };
                if !group.enabled {
                    return Vec::new();
                }
                let paused: std::collections::HashSet<&str> = group
                    .paused_session_ids
                    .iter()
                    .map(String::as_str)
                    .collect();
                let session_kind_by_id: std::collections::HashMap<&str, SessionKind> = sessions
                    .iter()
                    .map(|session| (session.id.as_str(), session.kind))
                    .collect();
                group
                    .session_ids
                    .iter()
                    .filter(|session_id| !paused.contains(session_id.as_str()))
                    .filter(|session_id| {
                        session_kind_by_id
                            .get(session_id.as_str())
                            .copied()
                            .is_some_and(is_compatible)
                    })
                    .cloned()
                    .collect()
            }
        }
    }

    pub(in crate::ui::view) fn send_command_group_target_options(
        &self,
    ) -> Vec<(String, String, usize)> {
        let sessions = self.session_manager.list_sessions().unwrap_or_default();
        let active_kind = self.active_session_kind();
        let is_compatible = |kind: SessionKind| -> bool {
            match active_kind {
                Some(SessionKind::Serial) => matches!(kind, SessionKind::Serial),
                Some(_) => !matches!(kind, SessionKind::Serial),
                None => true,
            }
        };
        let session_kind_by_id: std::collections::HashMap<&str, SessionKind> = sessions
            .iter()
            .map(|session| (session.id.as_str(), session.kind))
            .collect();
        self.sync_groups
            .iter()
            .filter(|group| group.enabled)
            .filter_map(|group| {
                let paused: std::collections::HashSet<&str> = group
                    .paused_session_ids
                    .iter()
                    .map(String::as_str)
                    .collect();
                let count = group
                    .session_ids
                    .iter()
                    .filter(|session_id| !paused.contains(session_id.as_str()))
                    .filter(|session_id| {
                        session_kind_by_id
                            .get(session_id.as_str())
                            .copied()
                            .is_some_and(is_compatible)
                    })
                    .count();
                if count == 0 {
                    None
                } else {
                    Some((group.id.clone(), group.name.clone(), count))
                }
            })
            .collect()
    }

    pub(in crate::ui::view) fn set_send_command_target(
        &mut self,
        target: SendCommandTarget,
        cx: &mut Context<Self>,
    ) {
        self.send_command_target = target;
        let label = match &self.send_command_target {
            SendCommandTarget::Current => "Current".to_string(),
            SendCommandTarget::AllCompatible => "All compatible".to_string(),
            SendCommandTarget::Group(id) => self
                .sync_groups
                .iter()
                .find(|group| &group.id == id)
                .map(|group| format!("Group: {}", group.name))
                .unwrap_or_else(|| "Group".to_string()),
        };
        self.terminal_status = format!("command send target: {label}");
        cx.notify();
    }

    pub(in crate::ui::view) fn build_send_command_units(
        &self,
        draft: &str,
        session_kind: Option<SessionKind>,
    ) -> Result<Vec<Vec<u8>>, String> {
        build_send_command_units_for(
            draft,
            self.send_command_data_type,
            self.send_command_mode,
            self.send_command_line_ending,
            session_kind,
        )
    }

    pub(in crate::ui::view) fn active_session_kind(&self) -> Option<SessionKind> {
        let active_id = self.active_session_id.as_deref()?;
        self.session_manager
            .list_sessions()
            .ok()?
            .into_iter()
            .find(|session| session.id == active_id)
            .map(|session| session.kind)
    }

    pub(in crate::ui::view) fn adjust_send_command_count(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        // Tauri: decrement from 1 -> ∞ (None); increment from ∞ -> 1.
        self.send_command_count = match (self.send_command_count, delta) {
            (None, d) if d < 0 => None,
            (None, _) => Some(1),
            (Some(1), d) if d < 0 => None,
            (Some(n), d) => Some((n as i32 + d).clamp(1, 9999) as u32),
        };
        cx.notify();
    }

    pub(in crate::ui::view) fn adjust_send_command_interval(
        &mut self,
        delta: f64,
        cx: &mut Context<Self>,
    ) {
        self.send_command_interval_seconds =
            (self.send_command_interval_seconds + delta).clamp(0.0, 60.0);
        cx.notify();
    }

    /// Tauri defaults: line=1.00s, char/byte=0.02s, packet=0.
    pub(in crate::ui::view) fn apply_send_command_default_interval(&mut self) {
        self.send_command_interval_seconds = match (self.send_command_data_type, self.send_command_mode)
        {
            (SendCommandDataType::Hex, SendCommandMode::Byte) => 0.02,
            (SendCommandDataType::Hex, _) => 0.0,
            (SendCommandDataType::Text, SendCommandMode::Line) => 1.0,
            (SendCommandDataType::Text, _) => 0.02,
        };
    }

    pub(in crate::ui::view) fn set_send_command_data_type(
        &mut self,
        data_type: SendCommandDataType,
        cx: &mut Context<Self>,
    ) {
        self.send_command_data_type = data_type;
        match data_type {
            SendCommandDataType::Hex => {
                if matches!(
                    self.send_command_mode,
                    SendCommandMode::Line | SendCommandMode::Character
                ) {
                    self.send_command_mode = SendCommandMode::Byte;
                }
            }
            SendCommandDataType::Text => {
                if matches!(
                    self.send_command_mode,
                    SendCommandMode::Packet | SendCommandMode::Byte
                ) {
                    self.send_command_mode = SendCommandMode::Line;
                }
            }
        }
        self.apply_send_command_default_interval();
        self.send_command_hex_scroll_x = 0.;
        self.send_command_hex_scroll_y = 0.;
        self.terminal_status = format!(
            "command send data: {}",
            match data_type {
                SendCommandDataType::Text => "Text",
                SendCommandDataType::Hex => "Hex",
            }
        );
        cx.notify();
    }

    pub(in crate::ui::view) fn set_send_command_mode(
        &mut self,
        mode: SendCommandMode,
        cx: &mut Context<Self>,
    ) {
        self.send_command_mode = mode;
        self.apply_send_command_default_interval();
        cx.notify();
    }

    pub(in crate::ui::view) fn clamp_send_command_hex_scroll(&mut self) {
        // Approximate viewport for guide overlay (Tauri textarea scrollTop/scrollLeft).
        const HEX_LINE_PX: f32 = 15.;
        const HEX_CHAR_PX: f32 = 7.2;
        const VIEWPORT_LINES: f32 = 5.;
        const VIEWPORT_CHARS: f32 = 48.;

        let display = format_send_command_hex_display(&self.send_command_draft);
        let lines: Vec<&str> = display.lines().collect();
        let line_count = lines.len().max(1) as f32;
        let max_line_chars = lines
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0) as f32;

        let max_scroll_y = ((line_count - VIEWPORT_LINES).max(0.)) * HEX_LINE_PX;
        let max_scroll_x = ((max_line_chars - VIEWPORT_CHARS).max(0.)) * HEX_CHAR_PX;

        self.send_command_hex_scroll_y = self.send_command_hex_scroll_y.clamp(0., max_scroll_y);
        self.send_command_hex_scroll_x = self.send_command_hex_scroll_x.clamp(0., max_scroll_x);
    }
}

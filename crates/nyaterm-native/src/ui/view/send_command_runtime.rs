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
                self.send_command_draft.pop();
                cx.notify();
            }
            "tab" if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                self.send_command_draft.push('\t');
                cx.notify();
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
                    self.send_command_draft.push_str(input);
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
        if self.active_session_id.is_none() {
            self.terminal_status = "start a session before sending".to_string();
            cx.notify();
            return;
        }

        let rounds = self.send_command_count.max(1);
        let interval = self.send_command_interval_seconds.max(0.0);
        let units_per_round = units.len() as u32;
        let total_units = units_per_round.saturating_mul(rounds);
        let cancel = Arc::new(AtomicBool::new(false));
        self.send_command_cancel = Some(cancel.clone());
        self.send_command_sending = true;
        self.send_command_progress_completed = 0;
        self.send_command_progress_total = total_units;
        self.send_command_progress_round = 0;
        self.send_command_progress_rounds = rounds;
        self.terminal_status = format!("sending {units_per_round} unit(s) × {rounds}");
        cx.notify();

        cx.spawn(async move |this, cx| {
            let mut first = true;
            let mut aborted = false;
            'outer: for round in 1..=rounds {
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
                    let _ = this.update(cx, |this, cx| {
                        this.send_terminal_input(unit, cx);
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
                    this.terminal_status = format!(
                        "command send stopped at {}/{}",
                        this.send_command_progress_completed, this.send_command_progress_total
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
        let next = (self.send_command_count as i32 + delta).clamp(1, 999);
        self.send_command_count = next as u32;
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
}

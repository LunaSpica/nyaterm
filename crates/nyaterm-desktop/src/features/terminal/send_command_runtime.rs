use super::*;

use crate::send_command::{
    SendCommandControlFocus, SendCommandDataType, SendCommandLineEnding, SendCommandMode,
    SendCommandTarget,
};

impl NyaTermApp {
    fn send_command_count_label(&self) -> String {
        self.send_command
            .options
            .count
            .map(|n| n.to_string())
            .unwrap_or_else(|| "∞".to_string())
    }

    fn sync_send_command_count_input(&mut self) {
        self.send_command.options.count_input = self.send_command_count_label();
    }

    fn sync_send_command_interval_input(&mut self) {
        self.send_command.options.sync_interval_input();
    }

    pub(in crate::features) fn close_send_command_menus(&mut self) {
        self.send_command.options.close_menus();
    }

    pub(in crate::features) fn focus_send_command_control(
        &mut self,
        control: SendCommandControlFocus,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.send_command.progress.sending {
            return;
        }
        self.close_send_command_menus();
        self.send_command.composer.control_focus = Some(control);
        match control {
            SendCommandControlFocus::Count => self.sync_send_command_count_input(),
            SendCommandControlFocus::Interval => self.sync_send_command_interval_input(),
        }
        window.focus(&self.send_command.composer.controls_focus);
        cx.notify();
    }

    pub(in crate::features) fn blur_send_command_control(&mut self, cx: &mut Context<Self>) {
        match self.send_command.composer.control_focus {
            Some(SendCommandControlFocus::Count) => {
                self.apply_send_command_count_input(false);
                self.sync_send_command_count_input();
            }
            Some(SendCommandControlFocus::Interval) => {
                self.apply_send_command_interval_input(false);
                self.sync_send_command_interval_input();
            }
            None => {}
        }
        self.send_command.composer.control_focus = None;
        cx.notify();
    }

    pub(in crate::features) fn handle_send_command_control_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let Some(control) = self.send_command.composer.control_focus else {
            return;
        };
        let keystroke = &event.keystroke;
        if keystroke.modifiers.alt || keystroke.modifiers.function {
            return;
        }

        match keystroke.key.as_str() {
            "enter" | "tab" => {
                self.blur_send_command_control(cx);
            }
            "escape" => {
                self.send_command.composer.control_focus = None;
                self.sync_send_command_count_input();
                self.sync_send_command_interval_input();
                cx.notify();
            }
            "backspace" if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                match control {
                    SendCommandControlFocus::Count => {
                        self.send_command.options.count_input.pop();
                        self.apply_send_command_count_input(true);
                    }
                    SendCommandControlFocus::Interval => {
                        self.send_command.options.interval_input.pop();
                        self.apply_send_command_interval_input(true);
                    }
                }
                cx.notify();
            }
            _ if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                else {
                    return;
                };
                match control {
                    SendCommandControlFocus::Count => {
                        let filtered: String = input
                            .chars()
                            .filter(|ch| {
                                ch.is_ascii_digit()
                                    || matches!(ch, 'i' | 'n' | 'f' | 'I' | 'N' | 'F' | '∞')
                            })
                            .collect();
                        if filtered.is_empty() {
                            return;
                        }
                        self.send_command.options.count_input.push_str(&filtered);
                        self.apply_send_command_count_input(true);
                    }
                    SendCommandControlFocus::Interval => {
                        let filtered: String = input
                            .chars()
                            .filter(|ch| ch.is_ascii_digit() || *ch == '.')
                            .collect();
                        if filtered.is_empty() {
                            return;
                        }
                        self.send_command.options.interval_input.push_str(&filtered);
                        self.apply_send_command_interval_input(true);
                    }
                }
                cx.notify();
            }
            _ => {}
        }
    }

    fn apply_send_command_count_input(&mut self, live: bool) {
        self.send_command.options.apply_count_input(live);
    }

    fn apply_send_command_interval_input(&mut self, live: bool) {
        let trimmed = self.send_command.options.interval_input.trim();
        if let Ok(value) = trimmed.parse::<f64>() {
            if value.is_finite() && value >= 0.0 {
                self.send_command.options.interval_seconds = value.clamp(0.0, 60.0);
            }
        } else if !live {
            self.apply_send_command_default_interval();
        }
    }

    pub(in crate::features) fn toggle_send_command_data_menu(&mut self, cx: &mut Context<Self>) {
        if self.send_command.progress.sending {
            return;
        }
        self.send_command.composer.control_focus = None;
        let next = !self.send_command.options.data_menu_open;
        self.close_send_command_menus();
        self.send_command.options.data_menu_open = next;
        cx.notify();
    }

    pub(in crate::features) fn toggle_send_command_mode_menu(&mut self, cx: &mut Context<Self>) {
        if self.send_command.progress.sending {
            return;
        }
        self.send_command.composer.control_focus = None;
        let next = !self.send_command.options.mode_menu_open;
        self.close_send_command_menus();
        self.send_command.options.mode_menu_open = next;
        cx.notify();
    }

    pub(in crate::features) fn toggle_send_command_target_menu(&mut self, cx: &mut Context<Self>) {
        if self.send_command.progress.sending {
            return;
        }
        self.send_command.composer.control_focus = None;
        let next = !self.send_command.options.target_menu_open;
        self.close_send_command_menus();
        self.send_command.options.target_menu_open = next;
        cx.notify();
    }

    pub(in crate::features) fn toggle_send_command_line_ending_menu(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if self.send_command.progress.sending {
            return;
        }
        self.send_command.composer.control_focus = None;
        let next = !self.send_command.options.line_ending_menu_open;
        self.close_send_command_menus();
        self.send_command.options.line_ending_menu_open = next;
        cx.notify();
    }

    pub(in crate::features) fn handle_send_command_key_down(
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
                if self.send_command.options.data_type == SendCommandDataType::Hex {
                    // Delete last hex digit (ignore spacing), then reformat pairs.
                    let mut cleaned: String = self
                        .send_command
                        .composer
                        .draft
                        .chars()
                        .filter(|ch| ch.is_ascii_hexdigit())
                        .collect();
                    cleaned.pop();
                    self.send_command.composer.draft = format_send_command_hex_display(&cleaned);
                    self.clamp_send_command_hex_scroll();
                } else {
                    self.send_command.composer.draft.pop();
                }
                cx.notify();
            }
            "tab" if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                if self.send_command.options.data_type != SendCommandDataType::Hex {
                    self.send_command.composer.draft.push_str("\t");
                    cx.notify();
                }
            }
            "escape" => {
                self.send_command.composer.draft.clear();
                self.terminal.view.status = "command send cleared".to_string();
                cx.notify();
            }
            _ if !keystroke.modifiers.platform && !keystroke.modifiers.control => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    if self.send_command.options.data_type == SendCommandDataType::Hex {
                        let filtered: String = input
                            .chars()
                            .filter(|ch| ch.is_ascii_hexdigit() || ch.is_whitespace())
                            .collect();
                        if filtered.is_empty() {
                            return;
                        }
                        self.send_command.composer.draft.push_str(&filtered);
                        self.send_command.composer.draft =
                            format_send_command_hex_display(&self.send_command.composer.draft);
                        self.clamp_send_command_hex_scroll();
                    } else {
                        self.send_command.composer.draft.push_str(input);
                    }
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    pub(in crate::features) fn send_bottom_command(
        &mut self,
        append_enter: bool,
        cx: &mut Context<Self>,
    ) {
        if self.send_command.progress.sending {
            self.stop_send_command(cx);
            return;
        }

        let session_kind = self.active_session_kind();
        let mut draft = self.send_command.composer.draft.clone();
        if append_enter && self.send_command.options.data_type == SendCommandDataType::Text {
            draft.push('\n');
        }
        let units = match self.build_send_command_units(&draft, session_kind) {
            Ok(units) => units,
            Err(message) => {
                self.terminal.view.status = message;
                cx.notify();
                return;
            }
        };
        if units.is_empty() {
            self.terminal.view.status = "command send is empty".to_string();
            cx.notify();
            return;
        }
        let target_session_ids = self.send_command_target_session_ids();
        if target_session_ids.is_empty() {
            if matches!(self.send_command.options.target, SendCommandTarget::Current)
                && self
                    .active_session_id
                    .as_deref()
                    .is_some_and(|session_id| self.is_session_disconnected(session_id))
            {
                self.terminal.view.status =
                    "session disconnected — reconnect before sending".to_string();
                cx.notify();
                return;
            }
            self.terminal.view.status = "start a session before sending".to_string();
            cx.notify();
            return;
        }

        // None => infinite rounds (Tauri SendCommandCount null / ∞).
        let infinite = self.send_command.options.count.is_none();
        let rounds = self.send_command.options.count.unwrap_or(1).max(1);
        let interval = self.send_command.options.interval_seconds.max(0.0);
        let units_per_round = units.len() as u32;
        let total_units = if infinite {
            0
        } else {
            units_per_round.saturating_mul(rounds)
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let failed_writes = Arc::new(AtomicUsize::new(0));
        let raw_units = self.send_command.options.data_type == SendCommandDataType::Hex;
        self.send_command.progress.cancel = Some(cancel.clone());
        self.send_command.progress.sending = true;
        self.send_command.progress.completed = 0;
        self.send_command.progress.total = total_units;
        self.send_command.progress.round = 0;
        self.send_command.progress.rounds = if infinite { 0 } else { rounds };
        self.terminal.view.status = if infinite {
            format!("sending {units_per_round} unit(s) × ∞")
        } else {
            format!("sending {units_per_round} unit(s) × {rounds}")
        };
        cx.notify();

        cx.spawn(async move |this, cx| {
            let mut first = true;
            let mut aborted = false;
            let mut round = 0u32;
            let failed_writes_for_send = failed_writes.clone();
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
                    this.send_command.progress.round = round;
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
                    let failed_writes = failed_writes_for_send.clone();
                    let _ = this.update(cx, |this, cx| {
                        for session_id in &targets {
                            let sent = if raw_units {
                                this.send_terminal_raw_input_to_session(
                                    session_id.clone(),
                                    unit.clone(),
                                    cx,
                                )
                            } else {
                                this.send_terminal_input_to_session(
                                    session_id.clone(),
                                    unit.clone(),
                                    cx,
                                )
                            };
                            if !sent {
                                failed_writes.fetch_add(1, Ordering::SeqCst);
                            }
                        }
                        this.send_command.progress.completed =
                            this.send_command.progress.completed.saturating_add(1);
                        cx.notify();
                    });
                }
            }
            let _ = this.update(cx, |this, cx| {
                this.send_command.progress.sending = false;
                this.send_command.progress.cancel = None;
                let failed_writes = failed_writes.load(Ordering::SeqCst);
                if aborted {
                    this.terminal.view.status = if infinite {
                        format!(
                            "command send stopped at {} unit(s) · round {}",
                            this.send_command.progress.completed, this.send_command.progress.round
                        )
                    } else {
                        format!(
                            "command send stopped at {}/{}",
                            this.send_command.progress.completed, this.send_command.progress.total
                        )
                    };
                    if failed_writes > 0 {
                        this.terminal.view.status =
                            format!("{}, {failed_writes} failed write(s)", this.terminal.view.status);
                    }
                } else if infinite {
                    this.terminal.view.status = if failed_writes == 0 {
                        format!(
                            "command send completed: {} unit(s)",
                            this.send_command.progress.completed
                        )
                    } else {
                        format!(
                            "command send completed: {} unit(s), {failed_writes} failed write(s)",
                            this.send_command.progress.completed
                        )
                    };
                } else {
                    this.terminal.view.status = if failed_writes == 0 {
                        format!("command send completed: {rounds} round(s)")
                    } else {
                        format!(
                            "command send completed: {rounds} round(s), {failed_writes} failed write(s)"
                        )
                    };
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(in crate::features) fn stop_send_command(&mut self, cx: &mut Context<Self>) {
        if let Some(cancel) = self.send_command.progress.cancel.as_ref() {
            cancel.store(true, Ordering::SeqCst);
            self.terminal.view.status = "stopping command send…".to_string();
            cx.notify();
        }
    }

    pub(in crate::features) fn send_command_target_session_ids(&self) -> Vec<String> {
        // Live sessions only from local metadata (skip disconnected tabs).
        let sessions = self
            .ordered_sessions()
            .into_iter()
            .filter(|session| !self.is_session_disconnected(&session.id))
            .collect::<Vec<_>>();
        let live_session_ids = sessions
            .iter()
            .map(|session| session.id.as_str())
            .collect::<std::collections::HashSet<_>>();
        let active_kind = self.active_session_kind();
        let is_compatible = |kind: SessionKind| -> bool {
            match active_kind {
                Some(SessionKind::Serial) => matches!(kind, SessionKind::Serial),
                Some(_) => !matches!(kind, SessionKind::Serial),
                None => true,
            }
        };
        match &self.send_command.options.target {
            SendCommandTarget::Current => self
                .active_session_id
                .as_ref()
                .filter(|session_id| live_session_ids.contains(session_id.as_str()))
                .cloned()
                .into_iter()
                .collect(),
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
                let Some(group) = self.sync_groups.iter().find(|group| &group.id == group_id)
                else {
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

    pub(in crate::features) fn send_command_group_target_options(
        &self,
    ) -> Vec<(String, String, usize)> {
        let sessions = self
            .ordered_sessions()
            .into_iter()
            .filter(|session| !self.is_session_disconnected(&session.id))
            .collect::<Vec<_>>();
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

    pub(in crate::features) fn set_send_command_target(
        &mut self,
        target: SendCommandTarget,
        cx: &mut Context<Self>,
    ) {
        self.send_command.options.target = target;
        self.close_send_command_menus();
        let label = match &self.send_command.options.target {
            SendCommandTarget::Current => "Current".to_string(),
            SendCommandTarget::AllCompatible => "All compatible".to_string(),
            SendCommandTarget::Group(id) => self
                .sync_groups
                .iter()
                .find(|group| &group.id == id)
                .map(|group| format!("Group: {}", group.name))
                .unwrap_or_else(|| "Group".to_string()),
        };
        self.terminal.view.status = format!("command send target: {label}");
        cx.notify();
    }

    pub(in crate::features) fn build_send_command_units(
        &self,
        draft: &str,
        session_kind: Option<SessionKind>,
    ) -> Result<Vec<Vec<u8>>, String> {
        build_send_command_units_for(
            draft,
            self.send_command.options.data_type,
            self.send_command.options.mode,
            self.send_command.options.line_ending,
            session_kind,
        )
    }

    pub(in crate::features) fn active_session_kind(&self) -> Option<SessionKind> {
        let active_id = self.active_session_id.as_deref()?;
        if self.is_session_disconnected(active_id) {
            return None;
        }
        self.session_info(active_id).map(|session| session.kind)
    }

    pub(in crate::features) fn adjust_send_command_count(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        if self.send_command.progress.sending {
            return;
        }
        // Tauri: decrement from 1 -> ∞ (None); increment from ∞ -> 1.
        self.send_command.options.count = match (self.send_command.options.count, delta) {
            (None, d) if d < 0 => None,
            (None, _) => Some(1),
            (Some(1), d) if d < 0 => None,
            (Some(n), d) => Some((n as i32 + d).clamp(1, 9999) as u32),
        };
        self.sync_send_command_count_input();
        cx.notify();
    }

    /// Tauri defaults: line=1.00s, char/byte=0.02s, packet=0.
    pub(in crate::features) fn apply_send_command_default_interval(&mut self) {
        self.send_command.options.interval_seconds = match (
            self.send_command.options.data_type,
            self.send_command.options.mode,
        ) {
            (SendCommandDataType::Hex, SendCommandMode::Byte) => 0.02,
            (SendCommandDataType::Hex, _) => 0.0,
            (SendCommandDataType::Text, SendCommandMode::Line) => 1.0,
            (SendCommandDataType::Text, _) => 0.02,
        };
        self.sync_send_command_interval_input();
    }

    pub(in crate::features) fn set_send_command_data_type(
        &mut self,
        data_type: SendCommandDataType,
        cx: &mut Context<Self>,
    ) {
        self.send_command.options.data_type = data_type;
        self.close_send_command_menus();
        match data_type {
            SendCommandDataType::Hex => {
                if matches!(
                    self.send_command.options.mode,
                    SendCommandMode::Line | SendCommandMode::Character
                ) {
                    self.send_command.options.mode = SendCommandMode::Byte;
                }
            }
            SendCommandDataType::Text => {
                if matches!(
                    self.send_command.options.mode,
                    SendCommandMode::Packet | SendCommandMode::Byte
                ) {
                    self.send_command.options.mode = SendCommandMode::Line;
                }
            }
        }
        self.apply_send_command_default_interval();
        self.send_command.composer.hex_scroll_x = 0.;
        self.send_command.composer.hex_scroll_y = 0.;
        self.terminal.view.status = format!(
            "command send data: {}",
            match data_type {
                SendCommandDataType::Text => "Text",
                SendCommandDataType::Hex => "Hex",
            }
        );
        cx.notify();
    }

    pub(in crate::features) fn set_send_command_mode(
        &mut self,
        mode: SendCommandMode,
        cx: &mut Context<Self>,
    ) {
        self.send_command.options.mode = mode;
        self.close_send_command_menus();
        self.apply_send_command_default_interval();
        cx.notify();
    }

    pub(in crate::features) fn set_send_command_line_ending(
        &mut self,
        line_ending: SendCommandLineEnding,
        cx: &mut Context<Self>,
    ) {
        self.send_command.options.line_ending = line_ending;
        self.close_send_command_menus();
        cx.notify();
    }

    pub(in crate::features) fn clamp_send_command_hex_scroll(&mut self) {
        self.send_command.composer.clamp_hex_scroll();
    }
}

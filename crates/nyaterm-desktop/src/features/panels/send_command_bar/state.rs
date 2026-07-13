use super::*;

pub(super) struct SendCommandBarViewState {
    pub(super) palette: crate::theme::ThemePalette,
    pub(super) group_targets: Vec<(String, String, usize)>,
    pub(super) target_scope_label: String,
    pub(super) target_kind: &'static str,
    pub(super) target_available: bool,
    pub(super) is_serial_text_line: bool,
    pub(super) validation_text: String,
    pub(super) validation_error: bool,
    pub(super) preview: String,
    pub(super) input_hint: &'static str,
    pub(super) count_label: String,
    pub(super) interval_label: String,
    pub(super) line_ending_label: &'static str,
    pub(super) is_sending: bool,
    pub(super) progress_ratio: f32,
    pub(super) progress_label: String,
}

impl NyaTermApp {
    pub(super) fn send_command_bar_view_state(&self) -> SendCommandBarViewState {
        let palette = self.theme_palette();
        let active_kind = self.active_session_kind();
        let active_target = self
            .active_session_name()
            .or_else(|| {
                self.active_session_id
                    .as_ref()
                    .map(|session_id| format!("Session {}", short_id(session_id)))
            })
            .unwrap_or_else(|| "No active session".to_string());
        let target_available = self.active_session_id.is_some();
        let group_targets = self.send_command_group_target_options();
        let target_scope_label = match &self.send_command_target {
            SendCommandTarget::Current => active_target.clone(),
            SendCommandTarget::AllCompatible => {
                let n = self.send_command_target_session_ids().len();
                if n == 0 {
                    "No compatible sessions".to_string()
                } else {
                    format!("All compatible ({n})")
                }
            }
            SendCommandTarget::Group(group_id) => {
                let n = self.send_command_target_session_ids().len();
                let name = group_targets
                    .iter()
                    .find(|(id, _, _)| id == group_id)
                    .map(|(_, name, _)| name.clone())
                    .or_else(|| {
                        self.sync_groups
                            .iter()
                            .find(|group| &group.id == group_id)
                            .map(|group| group.name.clone())
                    })
                    .unwrap_or_else(|| "Group".to_string());
                if n == 0 {
                    format!("Group: {name} (empty)")
                } else {
                    format!("Group: {name} ({n})")
                }
            }
        };
        let target_kind = match active_kind {
            Some(SessionKind::Serial) => "Serial Data",
            Some(SessionKind::RawTcp) => "Raw TCP",
            Some(SessionKind::Telnet) => "Telnet",
            Some(SessionKind::Ssh | SessionKind::LocalPty) => "Shell Command",
            None => "No session",
        };
        let is_serial_text_line = matches!(active_kind, Some(SessionKind::Serial))
            && self.send_command_data_type == SendCommandDataType::Text
            && self.send_command_mode == SendCommandMode::Line;
        let unit_result =
            self.build_send_command_units(&self.send_command_draft.clone(), active_kind);
        let (validation_text, validation_error, unit_count, byte_count) = match &unit_result {
            Ok(units) => {
                let bytes = units.iter().map(Vec::len).sum::<usize>();
                (
                    format!(
                        "{} unit(s) · {} byte(s) · count {} · interval {:.2}s",
                        units.len(),
                        bytes,
                        self.send_command_count
                            .map(|n| n.to_string())
                            .unwrap_or_else(|| "∞".to_string()),
                        self.send_command_interval_seconds
                    ),
                    false,
                    units.len(),
                    bytes,
                )
            }
            Err(error) => (error.clone(), true, 0usize, 0usize),
        };
        let preview = if self.send_command_data_type == SendCommandDataType::Hex {
            send_command_hex_preview(&self.send_command_draft)
        } else {
            truncate_preview(&self.send_command_draft.replace('\n', "\\n"), 96)
        };
        let input_hint = if self.send_command_data_type == SendCommandDataType::Hex {
            "e.g. 48 65 6C 6C 6F"
        } else {
            "Type command or payload…"
        };
        let count_label = self
            .send_command_count
            .map(|n| n.to_string())
            .unwrap_or_else(|| "∞".to_string());
        let interval_label = format!("{:.2}", self.send_command_interval_seconds);
        let line_ending_label = match self.send_command_line_ending {
            SendCommandLineEnding::None => "None",
            SendCommandLineEnding::Cr => "CR",
            SendCommandLineEnding::Lf => "LF",
            SendCommandLineEnding::Crlf => "CR+LF",
        };
        let _ = (unit_count, byte_count);
        let is_sending = self.send_command_sending;
        let infinite_progress = is_sending && self.send_command_progress_rounds == 0;
        let progress_total = self.send_command_progress_total.max(1);
        let progress_completed = if infinite_progress {
            self.send_command_progress_completed
        } else {
            self.send_command_progress_completed.min(progress_total)
        };
        let progress_ratio = if infinite_progress {
            // Indeterminate-ish pulse from completed units.
            (((progress_completed % 20) as f32) / 20.0).clamp(0.08, 0.95)
        } else {
            progress_completed as f32 / progress_total as f32
        };
        let progress_label = if is_sending {
            if infinite_progress {
                format!(
                    "Sending ∞ · round {} · {} unit(s)",
                    self.send_command_progress_round.max(1),
                    progress_completed
                )
            } else {
                format!(
                    "Sending {}/{} · round {}/{}",
                    progress_completed,
                    self.send_command_progress_total,
                    self.send_command_progress_round.max(1),
                    self.send_command_progress_rounds.max(1)
                )
            }
        } else {
            validation_text.clone()
        };

        SendCommandBarViewState {
            palette,
            group_targets,
            target_scope_label,
            target_kind,
            target_available,
            is_serial_text_line,
            validation_text,
            validation_error,
            preview,
            input_hint,
            count_label,
            interval_label,
            line_ending_label,
            is_sending,
            progress_ratio,
            progress_label,
        }
    }
}

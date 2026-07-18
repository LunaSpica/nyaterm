use super::*;

pub(super) struct SendCommandBarViewState {
    pub(super) palette: crate::theme::ThemePalette,
    pub(super) group_targets: Vec<(String, String, usize)>,
    pub(super) target_kind: &'static str,
    pub(super) is_serial_text_line: bool,
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
        let group_targets = self.send_command_group_target_options();
        let target_kind = match active_kind {
            Some(SessionKind::Serial) => self.tr("serialSend.serialData"),
            Some(SessionKind::RawTcp) => "Raw TCP",
            Some(SessionKind::Telnet) => "Telnet",
            Some(SessionKind::Ssh | SessionKind::LocalPty) => self.tr("serialSend.shellCommand"),
            None => self.tr("serialSend.unavailable"),
        };
        let is_serial_text_line = matches!(active_kind, Some(SessionKind::Serial))
            && self.send_command_data_type == SendCommandDataType::Text
            && self.send_command_mode == SendCommandMode::Line;
        let unit_result =
            self.build_send_command_units(&self.send_command_draft.clone(), active_kind);
        let (validation_error, unit_count, byte_count) = match &unit_result {
            Ok(units) => {
                let bytes = units.iter().map(Vec::len).sum::<usize>();
                (false, units.len(), bytes)
            }
            Err(_) => (true, 0usize, 0usize),
        };
        let preview = if self.send_command_data_type == SendCommandDataType::Hex {
            send_command_hex_preview(&self.send_command_draft)
        } else {
            truncate_preview(&self.send_command_draft.replace('\n', "\\n"), 96)
        };
        let input_hint = if self.send_command_data_type == SendCommandDataType::Hex {
            self.tr("serialSend.hexPlaceholder")
        } else {
            self.tr("serialSend.textPlaceholder")
        };
        let count_label = self
            .send_command_count
            .map(|n| n.to_string())
            .unwrap_or_else(|| "∞".to_string());
        let interval_label = format!("{:.2}", self.send_command_interval_seconds);
        let line_ending_label = match self.send_command_line_ending {
            SendCommandLineEnding::None => self.tr("serialSend.noLineEnding"),
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
                let round = self.tr("serialSend.shellProgressInfinite").replace(
                    "{{current}}",
                    &self.send_command_progress_round.max(1).to_string(),
                );
                let units = self
                    .tr("serialSend.shellProgressUnits")
                    .replace("{{completed}}", &progress_completed.to_string())
                    .replace("{{total}}", "∞");
                format!("{round} · {units}")
            } else {
                let units = self
                    .tr("serialSend.shellProgressUnits")
                    .replace("{{completed}}", &progress_completed.to_string())
                    .replace("{{total}}", &self.send_command_progress_total.to_string());
                let round = self
                    .tr("serialSend.shellProgressRound")
                    .replace(
                        "{{current}}",
                        &self.send_command_progress_round.max(1).to_string(),
                    )
                    .replace(
                        "{{total}}",
                        &self.send_command_progress_rounds.max(1).to_string(),
                    );
                format!("{units} · {round}")
            }
        } else {
            String::new()
        };

        SendCommandBarViewState {
            palette,
            group_targets,
            target_kind,
            is_serial_text_line,
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

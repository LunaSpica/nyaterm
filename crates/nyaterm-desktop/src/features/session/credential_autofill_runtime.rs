use super::*;

const CREDENTIAL_AUTOFILL_INPUT_TAIL_LIMIT: usize = 4096;
const RECENT_PROMPT_TTL_MS: u64 = 30_000;
const PENDING_PASSWORD_TTL_MS: u64 = 60_000;
const CREDENTIAL_PROMPT_INPUT_TTL_MS: u64 = 120_000;

impl NyaTermApp {
    pub(in crate::features) fn now_unix_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0)
    }

    pub(in crate::features) fn dismiss_credential_suggestions(&mut self, cx: &mut Context<Self>) {
        let had_panel = self.credential_suggestions.take().is_some();
        self.credential_autofill_buffer.clear();
        self.credential_autofill_recent.clear();
        self.credential_autofill_detection_pending = false;
        self.credential_autofill_pending_request = None;
        if had_panel {
            cx.notify();
        }
    }

    pub(in crate::features) fn reset_credential_autofill(&mut self, cx: &mut Context<Self>) {
        self.credential_suggestions = None;
        self.credential_autofill_buffer.clear();
        self.credential_autofill_recent.clear();
        self.credential_autofill_pending = None;
        self.credential_autofill_detection_pending = false;
        self.credential_autofill_pending_request = None;
        self.credential_autofill_sending = false;
        self.credential_prompt_input_until_ms = 0;
        cx.notify();
    }

    pub(in crate::features) fn is_credential_prompt_input_mode(&self) -> bool {
        let now = Self::now_unix_ms();
        self.credential_prompt_input_until_ms > now
    }

    fn prune_recent_credential_prompts(&mut self, now: u64) {
        self.credential_autofill_recent
            .retain(|_, ts| now.saturating_sub(*ts) <= RECENT_PROMPT_TTL_MS);
    }

    fn remember_credential_prompt(
        &mut self,
        kind: CredentialPromptKind,
        prompt_text: &str,
        now: u64,
    ) -> bool {
        self.prune_recent_credential_prompts(now);
        let key = format!("{kind:?}:{prompt_text}");
        if let Some(last) = self.credential_autofill_recent.get(&key) {
            if now.saturating_sub(*last) < RECENT_PROMPT_TTL_MS {
                return false;
            }
        }
        self.credential_autofill_recent.insert(key, now);
        true
    }

    fn show_credential_panel(
        &mut self,
        kind: CredentialPromptKind,
        matches: Vec<SavedCredential>,
        prompt_text: String,
        cx: &mut Context<Self>,
    ) {
        if matches.is_empty() {
            return;
        }
        let Some(session_id) = self.active_session_id.clone() else {
            return;
        };
        let (cursor_row, cursor_col) = self.active_terminal_cursor_cell_for_autofill();
        self.dismiss_command_suggestions(cx);
        self.credential_suggestions = Some(CredentialSuggestionState {
            session_id,
            kind,
            matches,
            prompt_text,
            selected_index: 0,
            cursor_row,
            cursor_col,
        });
        cx.notify();
    }

    fn active_terminal_cursor_cell_for_autofill(&self) -> (usize, usize) {
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

    pub(in crate::features) fn drain_pending_credential_autofill_detection(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        // Common idle path: no credentials, no detection, no match pipeline reply.
        if self.credential_autofill_pending_request.is_none()
            && !self.credential_autofill_detection_pending
            && self.connection_saved_credentials.is_empty()
            && self.credential_autofill_pending.is_none()
        {
            return false;
        }
        let mut dirty = self.drain_credential_autofill_match_events(cx);
        let detection_was_pending = self.credential_autofill_detection_pending;
        if credential_autofill_snapshot_detection_can_run(
            self.active_session_id.as_deref(),
            !self.connection_saved_credentials.is_empty()
                || self.credential_autofill_pending.is_some(),
            self.terminal_runtime.session_event_queued_output_bytes,
            self.pending_session_events.len(),
            self.pending_terminal_frame_events.len(),
            self.terminal_frame_pipeline.queued_event_count(),
            self.terminal_frame_pipeline.queued_output_bytes(),
            self.credential_autofill_pending_request.is_some(),
        ) {
            dirty |= self.sync_credential_autofill_from_active_snapshot(cx);
        }
        if !credential_autofill_detection_should_run_this_tick(
            detection_was_pending,
            credential_autofill_pending_detection_can_run(
                self.active_session_id.as_deref(),
                self.credential_autofill_detection_pending,
                self.terminal_runtime.session_event_queued_output_bytes,
                self.pending_session_events.len(),
                self.pending_terminal_frame_events.len(),
                self.terminal_frame_pipeline.queued_event_count(),
                self.terminal_frame_pipeline.queued_output_bytes(),
                self.credential_autofill_pending_request.is_some(),
            ),
        ) {
            return dirty;
        }
        self.credential_autofill_detection_pending = false;
        dirty |= self.detect_credential_prompt(cx);
        dirty
    }

    fn sync_credential_autofill_from_active_snapshot(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(session_id) = self.active_session_id.clone() else {
            return false;
        };
        let Some(prompt_text) = self
            .terminal_views
            .get(&session_id)
            .and_then(|view| view.frame_snapshot.as_deref())
            .and_then(credential_autofill_prompt_text_from_snapshot)
        else {
            return self.sync_credential_autofill_prompt_text(&session_id, String::new(), cx);
        };
        self.sync_credential_autofill_prompt_text(&session_id, prompt_text, cx)
    }

    fn sync_credential_autofill_prompt_text(
        &mut self,
        session_id: &str,
        prompt_text: String,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.active_session_id.as_deref() != Some(session_id) {
            return false;
        }
        if prompt_text.is_empty() {
            if !self.credential_autofill_buffer.is_empty() {
                self.credential_autofill_buffer.clear();
                self.credential_prompt_input_until_ms = 0;
                return true;
            }
            return false;
        }
        if self.credential_autofill_buffer == prompt_text
            && (self.credential_autofill_detection_pending
                || self.credential_autofill_pending.is_none())
        {
            return false;
        }
        let detected_prompt_kind = credential_autofill_detect_prompt_kind(&prompt_text);
        if detected_prompt_kind.is_none() && self.credential_autofill_pending.is_none() {
            if !self.credential_autofill_buffer.is_empty() {
                self.credential_autofill_buffer.clear();
                self.credential_prompt_input_until_ms = 0;
                return true;
            }
            return false;
        }

        let mut dirty = false;
        if self.credential_autofill_buffer != prompt_text {
            self.credential_autofill_buffer = prompt_text;
            dirty = true;
        }
        if detected_prompt_kind.is_some() {
            self.credential_prompt_input_until_ms =
                Self::now_unix_ms().saturating_add(CREDENTIAL_PROMPT_INPUT_TTL_MS);
            // Suppress command suggestions while a credential prompt is live.
            if self.command_suggestions.take().is_some() {
                dirty = true;
            }
            self.command_input_tracker = TerminalInputState::new();
        }

        if self.credential_suggestions.is_some() || self.credential_autofill_sending {
            return dirty;
        }
        if !self.credential_autofill_detection_pending {
            self.credential_autofill_detection_pending = true;
            dirty = true;
            cx.notify();
        }
        dirty
    }

    pub(in crate::features) fn detect_credential_prompt(
        &mut self,
        _cx: &mut Context<Self>,
    ) -> bool {
        if self.active_session_id.is_none() || self.credential_suggestions.is_some() {
            return false;
        }
        if self.connection_saved_credentials.is_empty() {
            return false;
        }

        let now = Self::now_unix_ms();
        let prompt_text =
            credential_autofill_prompt_text_from_visible(&self.credential_autofill_buffer);
        if prompt_text.is_empty() {
            return false;
        }
        let Some(prompt_kind) = credential_autofill_detect_prompt_kind(&prompt_text) else {
            return false;
        };
        let current_line = prompt_text.trim().to_string();
        let Some(active_session_id) = self.active_session_id.clone() else {
            return false;
        };
        let credentials = self.connection_saved_credentials.clone();

        if let Some(pending) = self.credential_autofill_pending.clone() {
            if pending.expires_at_ms <= now {
                self.credential_autofill_pending = None;
            }
        }

        if let Some(pending) = self.credential_autofill_pending.clone() {
            if pending.session_id != active_session_id {
                self.credential_autofill_pending = None;
            }
        }

        if self.credential_autofill_pending.is_none()
            && !self.remember_credential_prompt(prompt_kind, &prompt_text, now)
        {
            return false;
        }

        self.credential_autofill_next_request_id =
            self.credential_autofill_next_request_id.saturating_add(1);
        let key = CredentialAutofillMatchRequestKey {
            request_id: self.credential_autofill_next_request_id,
            session_id: active_session_id,
            prompt_text,
        };
        self.credential_autofill_pending_request = Some(key.clone());
        self.credential_autofill_match_pipeline
            .request(CredentialAutofillMatchRequest {
                key,
                current_line,
                prompt_kind,
                credentials,
                pending: self.credential_autofill_pending.clone(),
            });
        true
    }

    fn drain_credential_autofill_match_events(&mut self, cx: &mut Context<Self>) -> bool {
        let mut dirty = false;
        while let Some(event) = self.credential_autofill_match_pipeline.try_recv_event() {
            dirty |= self.apply_credential_autofill_match_event(event, cx);
        }
        dirty
    }

    fn apply_credential_autofill_match_event(
        &mut self,
        event: CredentialAutofillMatchEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.credential_autofill_pending_request.as_ref() != Some(&event.key) {
            return false;
        }
        self.credential_autofill_pending_request = None;
        if self.active_session_id.as_deref() != Some(event.key.session_id.as_str()) {
            return false;
        }
        if credential_autofill_prompt_text_from_visible(&self.credential_autofill_buffer)
            != event.key.prompt_text
        {
            return false;
        }

        match event.outcome {
            CredentialAutofillMatchOutcome::Suggest {
                kind,
                matches,
                clear_pending,
            } => {
                if clear_pending {
                    self.credential_autofill_pending = None;
                }
                self.show_credential_panel(kind, matches, event.key.prompt_text, cx);
                true
            }
            CredentialAutofillMatchOutcome::AutoFill { credential, kind } => {
                self.credential_autofill_pending = None;
                self.credential_autofill_buffer.clear();
                self.credential_autofill_recent.clear();
                self.send_credential_value(&credential, kind, &event.key.session_id, cx);
                true
            }
            CredentialAutofillMatchOutcome::NoMatch { clear_pending } => {
                if clear_pending {
                    self.credential_autofill_pending = None;
                }
                false
            }
        }
    }

    fn send_credential_value(
        &mut self,
        credential: &SavedCredential,
        kind: CredentialPromptKind,
        session_id: &str,
        cx: &mut Context<Self>,
    ) {
        if session_id.is_empty() {
            return;
        }
        if self.is_session_disconnected(session_id) {
            self.terminal_status =
                "session disconnected - reconnect before filling credentials".to_string();
            cx.notify();
            return;
        }
        if self.active_session_id.as_deref() != Some(session_id) {
            self.activate_session_id_with_surface_sync(session_id, cx);
        }
        match kind {
            CredentialPromptKind::Username => {
                let mut payload = credential.username.clone();
                payload.push('\r');
                self.send_terminal_input_without_suggestion_track(payload.into_bytes(), cx);
                self.terminal_status = format!("filled username from '{}'", credential.name);
            }
            CredentialPromptKind::Password => {
                let password = self.decrypt_saved_credential_password(&credential.id);
                let Some(password) = password.filter(|value| !value.is_empty()) else {
                    self.terminal_status =
                        format!("credential '{}' has no password", credential.name);
                    cx.notify();
                    return;
                };
                let mut payload = password;
                payload.push('\r');
                self.send_terminal_input_without_suggestion_track(payload.into_bytes(), cx);
                self.terminal_status = format!("filled password from '{}'", credential.name);
            }
        }
        cx.notify();
    }

    fn decrypt_saved_credential_password(&mut self, credential_id: &str) -> Option<String> {
        let store = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .ok()?;
        store
            .load_decrypted_credential_by_id(credential_id)
            .ok()
            .flatten()
            .and_then(|entry| entry.password)
    }

    pub(in crate::features) fn apply_selected_credential_suggestion(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.credential_suggestions.clone() else {
            return;
        };
        let Some(credential) = state.matches.get(state.selected_index).cloned() else {
            return;
        };
        self.select_credential_suggestion(credential, cx);
    }

    pub(in crate::features) fn select_credential_suggestion(
        &mut self,
        credential: SavedCredential,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.credential_suggestions.clone() else {
            return;
        };
        if self.credential_autofill_sending {
            return;
        }
        let was_username = state.kind == CredentialPromptKind::Username;
        self.credential_autofill_sending = true;
        self.send_credential_value(&credential, state.kind, &state.session_id, cx);
        if was_username {
            self.credential_autofill_pending = Some(PendingCredentialAutofill {
                session_id: state.session_id.clone(),
                credential_id: credential.id,
                expires_at_ms: Self::now_unix_ms().saturating_add(PENDING_PASSWORD_TTL_MS),
            });
        } else {
            self.credential_autofill_pending = None;
        }
        self.credential_autofill_sending = false;
        self.credential_suggestions = None;
        self.credential_autofill_recent.clear();

        if was_username {
            // Keep buffer so a password prompt that arrived during selection can still be detected.
            if !self.credential_autofill_buffer.is_empty() {
                self.detect_credential_prompt(cx);
            }
        } else {
            self.credential_autofill_buffer.clear();
            self.credential_autofill_recent.clear();
        }
        cx.notify();
    }

    /// Handle credential panel keys. Returns true when the key was consumed.
    pub(in crate::features) fn handle_credential_suggestion_key(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(state) = self.credential_suggestions.as_ref() else {
            return false;
        };
        if state.matches.is_empty() {
            return false;
        }
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            // Non-navigation keys dismiss the panel (Tauri: any other input dismisses).
            return false;
        }
        match keystroke.key.as_str() {
            "escape" => {
                self.dismiss_credential_suggestions(cx);
                true
            }
            "up" => {
                if let Some(state) = self.credential_suggestions.as_mut() {
                    if state.selected_index == 0 {
                        state.selected_index = state.matches.len().saturating_sub(1);
                    } else {
                        state.selected_index -= 1;
                    }
                    cx.notify();
                }
                true
            }
            "down" => {
                if let Some(state) = self.credential_suggestions.as_mut() {
                    state.selected_index = (state.selected_index + 1) % state.matches.len().max(1);
                    cx.notify();
                }
                true
            }
            "enter" => {
                self.apply_selected_credential_suggestion(cx);
                true
            }
            _ => {
                // Typing while the panel is open dismisses it (Tauri parity).
                self.dismiss_credential_suggestions(cx);
                false
            }
        }
    }

    pub(in crate::features) fn credential_suggestions_overlay(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let Some(state) = self.credential_suggestions.as_ref() else {
            return div().into_any_element();
        };
        if state.matches.is_empty() {
            return div().into_any_element();
        }

        let menu_w = 340.0_f32;
        let menu_h = (state.matches.len() as f32 * 36.0 + 52.0).min(320.0);
        let (x, y) = self
            .suggestion_overlay_position_for_session(
                Some(&state.session_id),
                state.cursor_row,
                state.cursor_col,
                menu_w,
                menu_h,
            )
            .unwrap_or((24.0, 120.0));

        let title = match state.kind {
            CredentialPromptKind::Password => "PASSWORD",
            CredentialPromptKind::Username => "USERNAME",
        };
        let kind_icon = match state.kind {
            CredentialPromptKind::Password => "🔑",
            CredentialPromptKind::Username => "👤",
        };

        let mut list = div()
            .id(SharedString::from("credential-suggestions-list"))
            .flex()
            .flex_col()
            .max_h(px(260.))
            .overflow_y_scroll();

        for (index, credential) in state.matches.iter().enumerate() {
            let selected = index == state.selected_index;
            let credential_id = credential.id.clone();
            list = list.child(
                div()
                    .id(SharedString::from(format!("credential-suggestion-{index}")))
                    .h(px(32.))
                    .px_3()
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
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if let Some(state) = this.credential_suggestions.as_mut() {
                            state.selected_index = index;
                        }
                        if let Some(credential) =
                            this.credential_suggestions.as_ref().and_then(|state| {
                                state
                                    .matches
                                    .iter()
                                    .find(|entry| entry.id == credential_id)
                                    .cloned()
                            })
                        {
                            this.select_credential_suggestion(credential, cx);
                        }
                    }))
                    .child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(if selected {
                                palette.accent
                            } else {
                                palette.text_dimmed
                            }))
                            .child(kind_icon.to_string()),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_size(px(12.))
                                    .text_color(rgb(palette.text))
                                    .child(truncate_preview(&credential.name, 36)),
                            )
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(palette.text_dimmed))
                                    .child(truncate_preview(&credential.username, 40)),
                            ),
                    ),
            );
        }

        div()
            .id(SharedString::from("credential-suggestions-overlay"))
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
                    .gap_1()
                    .child(
                        div()
                            .text_size(px(10.))
                            .font_weight(FontWeight(600.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(format!("{kind_icon} {title}")),
                    )
                    .child(
                        div()
                            .ml_auto()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(format!("{}", state.matches.len())),
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
                    .child("↑↓ select · Enter fill · Esc dismiss"),
            )
            .into_any_element()
    }
}

fn credential_autofill_snapshot_detection_can_run(
    active_session_id: Option<&str>,
    has_credentials_or_pending: bool,
    queued_output_bytes: usize,
    pending_session_events: usize,
    pending_terminal_frame_events: usize,
    queued_terminal_frame_events: usize,
    queued_terminal_frame_output_bytes: usize,
    match_request_pending: bool,
) -> bool {
    active_session_id.is_some()
        && has_credentials_or_pending
        && queued_output_bytes == 0
        && pending_session_events == 0
        && pending_terminal_frame_events == 0
        && queued_terminal_frame_events == 0
        && queued_terminal_frame_output_bytes == 0
        && !match_request_pending
}

fn credential_autofill_pending_detection_can_run(
    active_session_id: Option<&str>,
    detection_pending: bool,
    queued_output_bytes: usize,
    pending_session_events: usize,
    pending_terminal_frame_events: usize,
    queued_terminal_frame_events: usize,
    queued_terminal_frame_output_bytes: usize,
    match_request_pending: bool,
) -> bool {
    active_session_id.is_some()
        && detection_pending
        && queued_output_bytes == 0
        && pending_session_events == 0
        && pending_terminal_frame_events == 0
        && queued_terminal_frame_events == 0
        && queued_terminal_frame_output_bytes == 0
        && !match_request_pending
}

fn credential_autofill_detection_should_run_this_tick(
    detection_was_pending: bool,
    can_run: bool,
) -> bool {
    detection_was_pending && can_run
}

fn credential_autofill_prompt_text_from_snapshot(snapshot: &TerminalSnapshot) -> Option<String> {
    let line = credential_autofill_prompt_line_from_snapshot(snapshot)?;
    Some(credential_autofill_prompt_text_from_visible(line))
}

fn credential_autofill_prompt_line_from_snapshot(snapshot: &TerminalSnapshot) -> Option<&str> {
    credential_autofill_prompt_line_from_viewport(&snapshot.lines, snapshot.cursor_row)
}

fn credential_autofill_prompt_line_from_viewport(
    lines: &[String],
    cursor_row: usize,
) -> Option<&str> {
    if lines.is_empty() {
        return None;
    }
    if cursor_row != usize::MAX {
        return lines.get(cursor_row).map(String::as_str);
    }
    lines
        .iter()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map(String::as_str)
}

fn credential_autofill_visible_tail(text: &str) -> &str {
    if text.len() <= CREDENTIAL_AUTOFILL_INPUT_TAIL_LIMIT {
        return text;
    }
    let mut start = text.len() - CREDENTIAL_AUTOFILL_INPUT_TAIL_LIMIT;
    while start < text.len() && !text.is_char_boundary(start) {
        start += 1;
    }
    &text[start..]
}

fn credential_autofill_prompt_text_from_visible(output: &str) -> String {
    if output
        .chars()
        .last()
        .is_some_and(|ch| ch == '\r' || ch == '\n')
    {
        return String::new();
    }

    let tail = credential_autofill_visible_tail(output);
    let prompt_start = tail
        .rfind(|ch| ch == '\r' || ch == '\n')
        .map(|index| index + 1)
        .unwrap_or(0);
    let prompt = tail[prompt_start..].trim();
    let prompt_len = prompt.chars().count();
    if prompt_len > 500 {
        prompt.chars().skip(prompt_len - 500).collect::<String>()
    } else {
        prompt.to_string()
    }
}

fn credential_autofill_detect_prompt_kind(prompt: &str) -> Option<CredentialPromptKind> {
    let trimmed = prompt.trim();
    if trimmed.is_empty()
        || !trimmed
            .chars()
            .last()
            .is_some_and(|ch| ch == ':' || ch == '：')
    {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("password")
        || lower.contains("passphrase")
        || lower.contains("passcode")
        || lower.contains("pin")
        || lower.contains("otp")
        || lower.contains("verification code")
        || lower.contains("authentication code")
        || lower.contains("auth code")
        || lower.contains("2fa")
        || lower.contains("mfa")
        || trimmed.contains("密码")
        || trimmed.contains("口令")
        || trimmed.contains("验证码")
        || trimmed.contains("动态码")
        || trimmed.contains("动态口令")
    {
        return Some(CredentialPromptKind::Password);
    }
    if lower.contains("username")
        || lower.contains("user name")
        || lower.contains("login as")
        || lower.contains("login")
        || lower.contains("account")
        || lower.contains("user")
        || trimmed.contains("用户名")
        || trimmed.contains("用户")
        || trimmed.contains("账号")
        || trimmed.contains("账户")
        || trimmed.contains("登录名")
    {
        return Some(CredentialPromptKind::Username);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_autofill_snapshot_detection_requires_active_session_and_credentials() {
        assert!(credential_autofill_snapshot_detection_can_run(
            Some("active"),
            true,
            0,
            0,
            0,
            0,
            0,
            false
        ));
        assert!(!credential_autofill_snapshot_detection_can_run(
            None, true, 0, 0, 0, 0, 0, false
        ));
        assert!(!credential_autofill_snapshot_detection_can_run(
            Some("active"),
            false,
            0,
            0,
            0,
            0,
            0,
            false
        ));
        assert!(!credential_autofill_snapshot_detection_can_run(
            Some("active"),
            true,
            0,
            0,
            0,
            0,
            0,
            true
        ));
    }

    #[test]
    fn credential_autofill_snapshot_detection_waits_for_all_backlogs() {
        assert!(!credential_autofill_snapshot_detection_can_run(
            Some("active"),
            true,
            1,
            0,
            0,
            0,
            0,
            false
        ));
        assert!(!credential_autofill_snapshot_detection_can_run(
            Some("active"),
            true,
            0,
            1,
            0,
            0,
            0,
            false
        ));
        assert!(!credential_autofill_snapshot_detection_can_run(
            Some("active"),
            true,
            0,
            0,
            1,
            0,
            0,
            false
        ));
        assert!(!credential_autofill_snapshot_detection_can_run(
            Some("active"),
            true,
            0,
            0,
            0,
            1,
            0,
            false
        ));
        assert!(!credential_autofill_snapshot_detection_can_run(
            Some("active"),
            true,
            0,
            0,
            0,
            0,
            1,
            false
        ));
    }

    #[test]
    fn credential_autofill_pending_detection_runs_only_when_idle() {
        assert!(credential_autofill_pending_detection_can_run(
            Some("active"),
            true,
            0,
            0,
            0,
            0,
            0,
            false
        ));
        assert!(!credential_autofill_pending_detection_can_run(
            None, true, 0, 0, 0, 0, 0, false
        ));
        assert!(!credential_autofill_pending_detection_can_run(
            Some("active"),
            false,
            0,
            0,
            0,
            0,
            0,
            false
        ));
        assert!(!credential_autofill_pending_detection_can_run(
            Some("active"),
            true,
            1,
            0,
            0,
            0,
            0,
            false
        ));
        assert!(!credential_autofill_pending_detection_can_run(
            Some("active"),
            true,
            0,
            0,
            0,
            1,
            0,
            false
        ));
        assert!(!credential_autofill_pending_detection_can_run(
            Some("active"),
            true,
            0,
            0,
            0,
            0,
            1,
            false
        ));
        assert!(!credential_autofill_pending_detection_can_run(
            Some("active"),
            true,
            0,
            0,
            0,
            0,
            0,
            true
        ));
    }

    #[test]
    fn credential_autofill_detection_waits_for_next_tick_after_snapshot_sync() {
        assert!(!credential_autofill_detection_should_run_this_tick(
            false, true
        ));
        assert!(credential_autofill_detection_should_run_this_tick(
            true, true
        ));
        assert!(!credential_autofill_detection_should_run_this_tick(
            true, false
        ));
    }

    #[test]
    fn credential_autofill_prompt_line_uses_cursor_row() {
        let lines = vec![
            "Last login".to_string(),
            "Password:".to_string(),
            "ignored:".to_string(),
        ];

        assert_eq!(
            credential_autofill_prompt_line_from_viewport(&lines, 1),
            Some("Password:")
        );
    }

    #[test]
    fn credential_autofill_prompt_line_falls_back_to_last_nonempty_line() {
        let lines = vec![
            "Last login".to_string(),
            "Password:".to_string(),
            "".to_string(),
        ];

        assert_eq!(
            credential_autofill_prompt_line_from_viewport(&lines, usize::MAX),
            Some("Password:")
        );
    }

    #[test]
    fn credential_autofill_visible_tail_caps_input_on_boundary() {
        let text = format!("{}密码：", "测".repeat(3000));
        let tail = credential_autofill_visible_tail(&text);

        assert!(tail.len() <= CREDENTIAL_AUTOFILL_INPUT_TAIL_LIMIT);
        assert!(tail.is_char_boundary(0));
        assert!(tail.ends_with("密码："));
    }

    #[test]
    fn credential_autofill_prompt_text_reads_visible_last_line() {
        assert_eq!(
            credential_autofill_prompt_text_from_visible("hello\nPassword: "),
            "Password:"
        );
        assert_eq!(
            credential_autofill_prompt_text_from_visible("Password:\n"),
            ""
        );

        let long = format!("{}Password: ", "x".repeat(700));
        let prompt = credential_autofill_prompt_text_from_visible(&long);
        assert_eq!(prompt.chars().count(), 500);
        assert!(prompt.ends_with("Password:"));
    }

    #[test]
    fn credential_autofill_detect_prompt_kind_without_regex() {
        assert_eq!(
            credential_autofill_detect_prompt_kind("Password:"),
            Some(CredentialPromptKind::Password)
        );
        assert_eq!(
            credential_autofill_detect_prompt_kind("login as:"),
            Some(CredentialPromptKind::Username)
        );
        assert_eq!(
            credential_autofill_detect_prompt_kind("密码："),
            Some(CredentialPromptKind::Password)
        );
        assert_eq!(
            credential_autofill_detect_prompt_kind("Password accepted"),
            None
        );
    }
}

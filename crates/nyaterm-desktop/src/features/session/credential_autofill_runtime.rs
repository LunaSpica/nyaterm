use super::*;

const CREDENTIAL_AUTOFILL_BUFFER_LIMIT: usize = 4096;
const CREDENTIAL_AUTOFILL_INPUT_TAIL_LIMIT: usize = 4096;
const CREDENTIAL_PROMPT_REGEX_CACHE_LIMIT: usize = 512;
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
        if had_panel {
            cx.notify();
        }
    }

    pub(in crate::features) fn reset_credential_autofill(&mut self, cx: &mut Context<Self>) {
        self.credential_suggestions = None;
        self.credential_autofill_buffer.clear();
        self.credential_autofill_recent.clear();
        self.credential_autofill_pending = None;
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

    pub(in crate::features) fn feed_credential_autofill_output(
        &mut self,
        text: &str,
        cx: &mut Context<Self>,
    ) {
        if self.active_session_id.is_none() {
            return;
        }
        if text.is_empty() {
            return;
        }
        let visible =
            credential_autofill_strip_controls_fast(credential_autofill_visible_tail(text));
        if visible.is_empty() {
            return;
        }

        self.credential_autofill_buffer.push_str(&visible);
        if self.credential_autofill_buffer.len() > CREDENTIAL_AUTOFILL_BUFFER_LIMIT {
            let excess = self.credential_autofill_buffer.len() - CREDENTIAL_AUTOFILL_BUFFER_LIMIT;
            self.credential_autofill_buffer.drain(..excess);
            while !self.credential_autofill_buffer.is_empty()
                && !self.credential_autofill_buffer.is_char_boundary(0)
            {
                self.credential_autofill_buffer.remove(0);
            }
        }

        let prompt_text =
            credential_autofill_prompt_text_from_visible(&self.credential_autofill_buffer);
        let detected_prompt_kind = credential_autofill_detect_prompt_kind(&prompt_text);
        if detected_prompt_kind.is_some() {
            self.credential_prompt_input_until_ms =
                Self::now_unix_ms().saturating_add(CREDENTIAL_PROMPT_INPUT_TTL_MS);
            // Suppress command suggestions while a credential prompt is live.
            self.command_suggestions = None;
            self.command_input_tracker = TerminalInputState::new();
        } else if visible.contains('\r') || visible.contains('\n') {
            self.credential_prompt_input_until_ms = 0;
        }

        if self.credential_suggestions.is_some() || self.credential_autofill_sending {
            return;
        }
        if detected_prompt_kind.is_none() && self.credential_autofill_pending.is_none() {
            return;
        }
        self.detect_credential_prompt(cx);
    }

    pub(in crate::features) fn detect_credential_prompt(&mut self, cx: &mut Context<Self>) {
        if self.active_session_id.is_none() || self.credential_suggestions.is_some() {
            return;
        }
        if self.connection_saved_credentials.is_empty() {
            return;
        }

        let now = Self::now_unix_ms();
        let prompt_text =
            credential_autofill_prompt_text_from_visible(&self.credential_autofill_buffer);
        if prompt_text.is_empty() {
            return;
        }
        let Some(prompt_kind) = credential_autofill_detect_prompt_kind(&prompt_text) else {
            return;
        };
        let current_line = prompt_text.trim().to_string();
        let credentials = self.connection_saved_credentials.clone();

        if let Some(pending) = self.credential_autofill_pending.clone() {
            if pending.expires_at_ms <= now {
                self.credential_autofill_pending = None;
            }
        }

        if let Some(pending) = self.credential_autofill_pending.clone() {
            let Some(active_session_id) = self.active_session_id.clone() else {
                return;
            };
            if pending.session_id != active_session_id {
                self.credential_autofill_pending = None;
            } else {
                let pending_cred = credentials
                    .iter()
                    .find(|entry| entry.id == pending.credential_id)
                    .cloned();
                if let Some(pending_cred) = pending_cred {
                    if self.credential_matches_prompt_cached(
                        &pending_cred,
                        CredentialPromptKind::Password,
                        &current_line,
                    ) || self.credential_matches_prompt_cached(
                        &pending_cred,
                        CredentialPromptKind::Password,
                        &prompt_text,
                    ) {
                        self.credential_autofill_pending = None;
                        self.credential_autofill_buffer.clear();
                        self.credential_autofill_recent.clear();
                        self.send_credential_value(
                            &pending_cred,
                            CredentialPromptKind::Password,
                            &active_session_id,
                            cx,
                        );
                        return;
                    }
                }

                if credential_autofill_detect_prompt_kind(&current_line)
                    == Some(CredentialPromptKind::Password)
                {
                    self.credential_autofill_pending = None;
                } else {
                    return;
                }
            }
        }

        if !self.remember_credential_prompt(prompt_kind, &prompt_text, now) {
            return;
        }

        if prompt_kind == CredentialPromptKind::Password {
            let password_matches = self.find_matching_credentials_cached(
                &credentials,
                CredentialPromptKind::Password,
                &prompt_text,
            );
            if !password_matches.is_empty() {
                self.show_credential_panel(
                    CredentialPromptKind::Password,
                    password_matches,
                    prompt_text,
                    cx,
                );
                return;
            }

            if credential_autofill_detect_prompt_kind(&prompt_text)
                == Some(CredentialPromptKind::Password)
            {
                let fallback = find_password_only_fallback_credentials(&credentials);
                if !fallback.is_empty() {
                    self.show_credential_panel(
                        CredentialPromptKind::Password,
                        fallback,
                        prompt_text,
                        cx,
                    );
                    return;
                }
            }
            return;
        }

        let username_matches = self.find_matching_credentials_cached(
            &credentials,
            CredentialPromptKind::Username,
            &prompt_text,
        );
        if username_matches.is_empty() {
            return;
        }
        self.show_credential_panel(
            CredentialPromptKind::Username,
            username_matches,
            prompt_text,
            cx,
        );
    }

    fn find_matching_credentials_cached(
        &mut self,
        credentials: &[SavedCredential],
        kind: CredentialPromptKind,
        output: &str,
    ) -> Vec<SavedCredential> {
        credentials
            .iter()
            .filter(|credential| self.credential_matches_prompt_cached(credential, kind, output))
            .cloned()
            .collect()
    }

    fn credential_matches_prompt_cached(
        &mut self,
        credential: &SavedCredential,
        kind: CredentialPromptKind,
        output: &str,
    ) -> bool {
        if !credential.enabled {
            return false;
        }
        if kind == CredentialPromptKind::Username && credential.username.trim().is_empty() {
            return false;
        }
        if kind == CredentialPromptKind::Password && !credential.has_password {
            return false;
        }

        let pattern = get_credential_prompt_pattern(credential, kind);
        if pattern.is_empty() {
            return false;
        }
        let cache_key = format!("{}:{kind:?}:{pattern}", credential.id);
        if !self.credential_prompt_regex_cache.contains_key(&cache_key) {
            if self.credential_prompt_regex_cache.len() >= CREDENTIAL_PROMPT_REGEX_CACHE_LIMIT {
                self.credential_prompt_regex_cache.clear();
            }
            let Some(regex) = compile_prompt_regex(&pattern) else {
                return false;
            };
            self.credential_prompt_regex_cache
                .insert(cache_key.clone(), regex);
        }
        self.credential_prompt_regex_cache
            .get(&cache_key)
            .is_some_and(|regex| regex.is_match(output))
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
            self.activate_session_id(session_id);
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

        let bounds = self.terminal_surface_bounds_for_session(Some(&state.session_id));
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
        let menu_w = 340.0_f32;
        let menu_h = (state.matches.len() as f32 * 36.0 + 52.0).min(320.0);
        let mut x = base_x;
        let mut y = base_y + 4.0;
        if x + menu_w + 8.0 > viewport_w {
            x = (viewport_w - menu_w - 8.0).max(8.0);
        }
        if y + menu_h + 8.0 > viewport_h {
            y = (base_y - menu_h - 4.0).max(8.0);
        }

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

fn credential_autofill_strip_controls_fast(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut iter = text.chars().peekable();
    while let Some(ch) = iter.next() {
        if ch != '\x1b' {
            out.push(ch);
            continue;
        }
        match iter.peek().copied() {
            Some('[') => {
                iter.next();
                for next in iter.by_ref() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
            Some(']') => {
                iter.next();
                let mut saw_esc = false;
                for next in iter.by_ref() {
                    if next == '\x07' {
                        break;
                    }
                    if saw_esc && next == '\\' {
                        break;
                    }
                    saw_esc = next == '\x1b';
                }
            }
            Some('@'..='Z' | '\\' | '_' | '^') => {
                iter.next();
            }
            _ => {}
        }
    }
    out
}

fn credential_autofill_prompt_text_from_visible(output: &str) -> String {
    if output
        .chars()
        .last()
        .is_some_and(|ch| ch == '\r' || ch == '\n')
    {
        return String::new();
    }

    let normalized = output.replace('\r', "\n");
    let prompt = normalized.rsplit('\n').next().unwrap_or("").trim();
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
    fn credential_autofill_fast_strip_removes_common_controls() {
        let text = "\x1b[32mhello\x1b[0m\x1b]0;title\x07\nPassword: ";

        assert_eq!(
            credential_autofill_strip_controls_fast(text),
            "hello\nPassword: "
        );
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

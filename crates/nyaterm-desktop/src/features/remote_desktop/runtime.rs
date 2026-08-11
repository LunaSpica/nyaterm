use std::time::{Duration, Instant};

use gpui::{Bounds, ClipboardItem, Context, DevicePixels, Point, Size, Window, point, size};
use nyaterm_core::{ConnectionStore, KnownHostCheck, RdpCertificateMetadata};
use nyaterm_remote_desktop::{
    CertificateDecision, ClipboardOrigin, DirtyRect, Framebuffer, RdpCapability,
    RdpCertificatePolicy, RdpCertificateRequest, RdpCertificateResponse, RdpClipboardMode,
    RdpError, RdpErrorKind, RdpFrameEvent, RdpInputEvent, RdpRuntimeEvent, RdpSessionConfig,
    RdpSessionState,
};

use crate::features::NyaTermApp;

const RESIZE_DEBOUNCE: Duration = Duration::from_millis(150);
const CLIPBOARD_POLL_INTERVAL: Duration = Duration::from_millis(250);
const POINTER_MOVE_INTERVAL: Duration = Duration::from_millis(8);
const METRICS_REPORT_INTERVAL: Duration = Duration::from_secs(5);

impl NyaTermApp {
    pub(in crate::features) fn ensure_rdp_focus_reporting(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.remote_desktop.focus_subscriptions.is_empty() {
            return;
        }
        let subscription = cx.on_focus_out(
            &self.remote_desktop.focus,
            window,
            |this, _event, _window, _cx| {
                super::keyboard_capture::set_keyboard_capture(
                    this.remote_desktop.manager.clone(),
                    None,
                );
                if let Some(session_id) = this.session.active_id_owned() {
                    this.release_rdp_keys(&session_id);
                }
            },
        );
        self.remote_desktop.focus_subscriptions.push(subscription);
    }

    pub(in crate::features) fn create_rdp_runtime(
        &mut self,
        config: RdpSessionConfig,
    ) -> Result<String, RdpError> {
        let session_id = self.remote_desktop.manager.create_session(config)?;
        self.remote_desktop.insert_connecting(session_id.clone());
        Ok(session_id)
    }

    pub(in crate::features) fn create_failed_rdp_runtime(&mut self, error: RdpError) -> String {
        let session_id = nyaterm_core::uuid();
        self.remote_desktop.insert_connecting(session_id.clone());
        if let Some(session) = self.remote_desktop.sessions.get_mut(&session_id) {
            set_rdp_view_error(session, error.kind, error.message);
        }
        session_id
    }

    pub(in crate::features) fn retry_rdp_runtime(&mut self, session_id: &str) {
        self.restart_rdp_runtime(session_id, true);
    }

    fn restart_rdp_runtime(&mut self, session_id: &str, reset_attempts: bool) {
        let Some(metadata) = self.session.metadata(session_id).cloned() else {
            return;
        };
        let mut config = match metadata.launch_config {
            crate::models::SessionLaunchConfig::Rdp(config) => config,
            _ => return,
        };
        if config.password.is_none()
            && let Some(connection_id) = metadata.source_connection_id.as_deref()
            && let Some(connection) = self
                .connection_state
                .connections()
                .iter()
                .find(|connection| connection.id == connection_id)
        {
            config.password = load_rdp_password(self, connection.auth.as_ref());
        }
        let reconnect_attempts = if reset_attempts {
            0
        } else {
            self.remote_desktop
                .sessions
                .get(session_id)
                .map_or(0, |session| session.reconnect_attempts)
        };
        let _ = self.close_rdp_runtime(session_id);
        match self
            .remote_desktop
            .manager
            .create_session_with_id(session_id.to_string(), config)
        {
            Ok(_) => {
                self.remote_desktop
                    .insert_connecting(session_id.to_string());
                if let Some(session) = self.remote_desktop.sessions.get_mut(session_id) {
                    session.reconnect_attempts = reconnect_attempts;
                }
                if let Some(metadata) = self.session.metadata_mut(session_id) {
                    metadata.disconnected = false;
                }
                self.shell.set_status("RDP reconnecting".to_string());
            }
            Err(error) => {
                self.remote_desktop
                    .insert_connecting(session_id.to_string());
                if let Some(session) = self.remote_desktop.sessions.get_mut(session_id) {
                    set_rdp_view_error(session, error.kind, error.message);
                }
            }
        }
    }

    pub(in crate::features) fn close_rdp_runtime(
        &mut self,
        session_id: &str,
    ) -> Result<(), RdpError> {
        if self.session.active_id() == Some(session_id) {
            super::keyboard_capture::set_keyboard_capture(
                self.remote_desktop.manager.clone(),
                None,
            );
            self.release_rdp_keys(session_id);
        }
        if let Some(mut session) = self.remote_desktop.sessions.remove(session_id) {
            if let Some(texture) = session.texture.take() {
                self.remote_desktop.pending_texture_removals.push(texture);
            }
            if let Some(texture) = session.cursor_texture.take() {
                self.remote_desktop.pending_texture_removals.push(texture);
            }
        }
        self.remote_desktop.manager.close(session_id)
    }

    pub(in crate::features) fn release_rdp_keys(&mut self, session_id: &str) {
        let Some(session) = self.remote_desktop.sessions.get_mut(session_id) else {
            return;
        };
        if let Some(event) = session.keys.release_all() {
            let _ = self
                .remote_desktop
                .manager
                .send_input(session_id, vec![event]);
        }
    }

    pub(in crate::features) fn drain_rdp_runtime_events(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        for texture in self.remote_desktop.pending_texture_removals.drain(..) {
            window.remove_dynamic_texture(texture);
        }
        let ids = self
            .remote_desktop
            .sessions
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        let mut dirty = false;
        for session_id in ids {
            let drain = self.remote_desktop.manager.drain(&session_id);
            if drain.control.is_empty() && drain.frames.is_empty() {
                continue;
            }
            if self.remote_desktop.metrics_enabled {
                self.remote_desktop.metrics_control_events += drain.control.len();
                self.remote_desktop.metrics_frame_updates += drain.frames.len();
                self.remote_desktop.metrics_dropped_frames += drain.dropped_frames;
            }
            dirty = true;
            for event in drain.control {
                self.apply_rdp_control_event(&session_id, event, window, cx);
            }
            self.apply_rdp_frame_batch(&session_id, drain.frames, window);
        }
        dirty |= self.drive_rdp_pointer_flush();
        dirty |= self.drive_rdp_resize_debounce();
        dirty |= self.drive_rdp_reconnects();
        self.sync_rdp_keyboard_capture(window);
        dirty |= self.poll_active_rdp_clipboard(cx);
        self.report_rdp_metrics();
        dirty
    }

    fn sync_rdp_keyboard_capture(&self, window: &Window) {
        let target = self.session.active_id().and_then(|session_id| {
            (self.remote_desktop.focus.is_focused(window)
                && self
                    .remote_desktop
                    .sessions
                    .get(session_id)
                    .is_some_and(|session| matches!(session.state, RdpSessionState::Connected)))
            .then(|| session_id.to_string())
        });
        super::keyboard_capture::set_keyboard_capture(self.remote_desktop.manager.clone(), target);
    }

    pub(in crate::features) fn update_rdp_viewport(
        &mut self,
        session_id: &str,
        bounds: Bounds<gpui::Pixels>,
    ) {
        let Some(session) = self.remote_desktop.sessions.get_mut(session_id) else {
            return;
        };
        session.viewport = Some(bounds);
        self.queue_rdp_resize(
            session_id,
            f32::from(bounds.size.width).round().max(1.0) as u32,
            f32::from(bounds.size.height).round().max(1.0) as u32,
        );
    }

    pub(in crate::features) fn send_rdp_key_down(
        &mut self,
        session_id: &str,
        key: &str,
        key_char: Option<&str>,
        repeat: bool,
    ) -> bool {
        let Some(session) = self.remote_desktop.sessions.get_mut(session_id) else {
            return false;
        };
        let event = session.keys.key_down(key, repeat).or_else(|| {
            key_char
                .filter(|text| !text.is_empty())
                .map(|text| RdpInputEvent::Unicode {
                    text: text.to_string(),
                })
        });
        let Some(event) = event else {
            return false;
        };
        self.remote_desktop
            .manager
            .send_input(session_id, vec![event])
            .is_ok()
    }

    pub(in crate::features) fn send_rdp_key_up(&mut self, session_id: &str, key: &str) -> bool {
        let Some(event) = self
            .remote_desktop
            .sessions
            .get_mut(session_id)
            .and_then(|session| session.keys.key_up(key))
        else {
            return false;
        };
        self.remote_desktop
            .manager
            .send_input(session_id, vec![event])
            .is_ok()
    }

    pub(in crate::features) fn send_rdp_pointer(
        &mut self,
        session_id: &str,
        position: gpui::Point<gpui::Pixels>,
        button: Option<nyaterm_remote_desktop::RdpPointerButton>,
        pressed: bool,
    ) -> bool {
        let Some(session) = self.remote_desktop.sessions.get_mut(session_id) else {
            return false;
        };
        let (Some(viewport), Some(framebuffer)) = (session.viewport, session.framebuffer.as_ref())
        else {
            return false;
        };
        let x = f32::from(position.x - viewport.origin.x);
        let y = f32::from(position.y - viewport.origin.y);
        let Some(remote) = nyaterm_remote_desktop::viewport_to_remote(
            x,
            y,
            f32::from(viewport.size.width),
            f32::from(viewport.size.height),
            framebuffer.width(),
            framebuffer.height(),
        ) else {
            return false;
        };
        let now = Instant::now();
        if button.is_none() {
            if session.last_pointer == Some(remote) {
                return false;
            }
            session.last_pointer = Some(remote);
            if session.last_pointer_sent_at.is_some_and(|sent_at| {
                now.saturating_duration_since(sent_at) < POINTER_MOVE_INTERVAL
            }) {
                session.pending_pointer = Some(remote);
                return true;
            }
        }
        session.last_pointer = Some(remote);
        session.pending_pointer = None;
        session.last_pointer_sent_at = Some(now);
        self.remote_desktop
            .manager
            .send_input(
                session_id,
                vec![RdpInputEvent::Pointer {
                    x: remote.0,
                    y: remote.1,
                    button,
                    pressed,
                }],
            )
            .is_ok()
    }

    fn drive_rdp_pointer_flush(&mut self) -> bool {
        let now = Instant::now();
        let mut sent = false;
        for (session_id, session) in &mut self.remote_desktop.sessions {
            let Some(pointer) = session.pending_pointer else {
                continue;
            };
            if session.last_pointer_sent_at.is_some_and(|sent_at| {
                now.saturating_duration_since(sent_at) < POINTER_MOVE_INTERVAL
            }) {
                continue;
            }
            session.pending_pointer = None;
            session.last_pointer_sent_at = Some(now);
            sent |= self
                .remote_desktop
                .manager
                .send_input(
                    session_id,
                    vec![RdpInputEvent::Pointer {
                        x: pointer.0,
                        y: pointer.1,
                        button: None,
                        pressed: false,
                    }],
                )
                .is_ok();
        }
        sent
    }

    fn report_rdp_metrics(&mut self) {
        if !self.remote_desktop.metrics_enabled {
            return;
        }
        let now = Instant::now();
        if now.saturating_duration_since(self.remote_desktop.metrics_last_report)
            < METRICS_REPORT_INTERVAL
        {
            return;
        }
        tracing::debug!(
            active_sessions = self.remote_desktop.sessions.len(),
            control_events = self.remote_desktop.metrics_control_events,
            frame_updates = self.remote_desktop.metrics_frame_updates,
            dropped_frames = self.remote_desktop.metrics_dropped_frames,
            "RDP runtime metrics"
        );
        self.remote_desktop.metrics_last_report = now;
        self.remote_desktop.metrics_control_events = 0;
        self.remote_desktop.metrics_frame_updates = 0;
        self.remote_desktop.metrics_dropped_frames = 0;
    }

    fn apply_rdp_control_event(
        &mut self,
        session_id: &str,
        event: RdpRuntimeEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            RdpRuntimeEvent::State { state, message, .. } => {
                if let Some(session) = self.remote_desktop.sessions.get_mut(session_id) {
                    if let RdpSessionState::Failed(error) = &state {
                        session.error = Some(error.clone());
                    }
                    session.state = state;
                }
                if let Some(message) = message {
                    self.shell.set_status(message);
                }
            }
            RdpRuntimeEvent::Frame {
                event:
                    RdpFrameEvent::Reset {
                        epoch,
                        width,
                        height,
                    },
                ..
            } => {
                self.reset_rdp_framebuffer(session_id, epoch, width, height, window);
            }
            RdpRuntimeEvent::Frame { .. } => {}
            RdpRuntimeEvent::Clipboard { text, .. } => {
                if self.session.active_id() != Some(session_id) {
                    return;
                }
                let accepted = self
                    .remote_desktop
                    .sessions
                    .get_mut(session_id)
                    .and_then(|session| {
                        session
                            .clipboard
                            .accept(ClipboardOrigin::Remote, &text)
                            .ok()
                            .flatten()
                    })
                    .is_some();
                if accepted {
                    cx.write_to_clipboard(ClipboardItem::new_string(text));
                }
            }
            RdpRuntimeEvent::CertificateRequest(request) => {
                self.handle_rdp_certificate_request(session_id, request);
            }
            RdpRuntimeEvent::Capability { capability, .. } => {
                if let Some(session) = self.remote_desktop.sessions.get_mut(session_id) {
                    session.capability = Some(capability);
                }
                if capability == RdpCapability::DynamicResizeUnavailable {
                    self.shell
                        .set_status("RDP server does not support dynamic resize".to_string());
                }
            }
            RdpRuntimeEvent::Error { error, fatal, .. } => {
                let should_reconnect = fatal && self.schedule_rdp_reconnect(session_id, &error);
                if let Some(session) = self.remote_desktop.sessions.get_mut(session_id) {
                    session.error = Some(error.clone());
                    if fatal && !should_reconnect {
                        session.state = RdpSessionState::Failed(error.clone());
                    }
                }
                if !should_reconnect {
                    self.shell.set_status(format_rdp_error(&error));
                }
            }
        }
    }

    fn schedule_rdp_reconnect(&mut self, session_id: &str, error: &RdpError) -> bool {
        if !rdp_error_is_retryable(error.kind) {
            return false;
        }
        let Some(config) =
            self.session
                .metadata(session_id)
                .and_then(|metadata| match &metadata.launch_config {
                    crate::models::SessionLaunchConfig::Rdp(config) => Some(&config.reconnect),
                    _ => None,
                })
        else {
            return false;
        };
        if !config.enabled {
            return false;
        }
        let Some(session) = self.remote_desktop.sessions.get_mut(session_id) else {
            return false;
        };
        if session.reconnect_attempts >= config.max_attempts {
            return false;
        }
        session.reconnect_attempts += 1;
        let delay = rdp_reconnect_delay(session.reconnect_attempts, rand::random_range(0..250));
        session.reconnect_at = Some(Instant::now() + delay);
        session.state = RdpSessionState::Reconnecting;
        self.shell.set_status(format!(
            "RDP reconnecting in {:.1}s (attempt {}/{})",
            delay.as_secs_f32(),
            session.reconnect_attempts,
            config.max_attempts
        ));
        true
    }

    fn drive_rdp_reconnects(&mut self) -> bool {
        let now = Instant::now();
        let due = self
            .remote_desktop
            .sessions
            .iter()
            .filter(|(_, session)| session.reconnect_at.is_some_and(|deadline| now >= deadline))
            .map(|(session_id, _)| session_id.clone())
            .collect::<Vec<_>>();
        for session_id in &due {
            if let Some(session) = self.remote_desktop.sessions.get_mut(session_id) {
                session.reconnect_at = None;
            }
            self.restart_rdp_runtime(session_id, false);
        }
        !due.is_empty()
    }

    fn reset_rdp_framebuffer(
        &mut self,
        session_id: &str,
        epoch: u64,
        width: u32,
        height: u32,
        window: &mut Window,
    ) {
        let Some(session) = self.remote_desktop.sessions.get_mut(session_id) else {
            return;
        };
        if let Some(texture) = session.texture.take() {
            window.remove_dynamic_texture(texture);
        }
        if let Some(texture) = session.cursor_texture.take() {
            window.remove_dynamic_texture(texture);
        }
        match Framebuffer::new(epoch, width, height) {
            Ok(framebuffer) => {
                let texture_size = size(DevicePixels(width as i32), DevicePixels(height as i32));
                match window.create_dynamic_texture(texture_size, framebuffer.pixels(), width * 4) {
                    Ok(texture) => {
                        session.framebuffer = Some(framebuffer);
                        session.texture = Some(texture);
                        session.cursor = None;
                    }
                    Err(error) => set_rdp_view_error(
                        session,
                        RdpErrorKind::Protocol,
                        format!("failed to create RDP texture: {error}"),
                    ),
                }
            }
            Err(error) => set_rdp_view_error(
                session,
                RdpErrorKind::Protocol,
                format!("invalid RDP desktop reset: {error}"),
            ),
        }
    }

    fn apply_rdp_frame_batch(
        &mut self,
        session_id: &str,
        frames: Vec<RdpFrameEvent>,
        window: &mut Window,
    ) {
        let Some(session) = self.remote_desktop.sessions.get_mut(session_id) else {
            return;
        };
        let mut dirty_rects = Vec::new();
        for frame in frames {
            if let RdpFrameEvent::Cursor(cursor) = frame {
                if session
                    .framebuffer
                    .as_ref()
                    .is_some_and(|framebuffer| framebuffer.epoch() == cursor.epoch)
                {
                    if let Some(texture) = session.cursor_texture.take() {
                        window.remove_dynamic_texture(texture);
                    }
                    if cursor.visible
                        && cursor.width > 0
                        && cursor.height > 0
                        && let Ok(texture) = window.create_dynamic_texture(
                            size(
                                DevicePixels(cursor.width as i32),
                                DevicePixels(cursor.height as i32),
                            ),
                            &cursor.pixels,
                            cursor.width * 4,
                        )
                    {
                        session.cursor_texture = Some(texture);
                    }
                    session.cursor = Some(cursor);
                }
                continue;
            }
            let Some(framebuffer) = session.framebuffer.as_mut() else {
                continue;
            };
            match framebuffer.apply(&frame) {
                Ok(Some(rect)) => dirty_rects.push(rect),
                Ok(None) => {}
                Err(nyaterm_remote_desktop::FramebufferError::StaleEpoch { .. }) => {}
                Err(error) => {
                    set_rdp_view_error(
                        session,
                        RdpErrorKind::Protocol,
                        format!("invalid RDP frame: {error}"),
                    );
                    return;
                }
            }
        }
        if !dirty_rects.is_empty() {
            clear_rdp_reconnect_after_frame(session);
        }
        let (Some(framebuffer), Some(texture)) = (session.framebuffer.as_ref(), session.texture)
        else {
            return;
        };
        let framebuffer_area = u64::from(framebuffer.width()) * u64::from(framebuffer.height());
        let dirty_area = dirty_rects
            .iter()
            .map(|rect| u64::from(rect.width) * u64::from(rect.height))
            .sum::<u64>();
        if dirty_rects.len() > 64 || dirty_area.saturating_mul(100) >= framebuffer_area * 60 {
            let bounds = Bounds::new(
                Point::new(DevicePixels(0), DevicePixels(0)),
                Size::new(
                    DevicePixels(framebuffer.width() as i32),
                    DevicePixels(framebuffer.height() as i32),
                ),
            );
            let _ = window.update_dynamic_texture(
                texture,
                bounds,
                framebuffer.pixels(),
                framebuffer.width() * 4,
            );
            return;
        }
        for rect in nyaterm_remote_desktop::merge_dirty_rects(dirty_rects) {
            let _ = upload_rdp_rect(window, texture, framebuffer, rect);
        }
    }

    fn handle_rdp_certificate_request(&mut self, session_id: &str, request: RdpCertificateRequest) {
        let policy = self
            .session
            .metadata(session_id)
            .and_then(|metadata| match &metadata.launch_config {
                crate::models::SessionLaunchConfig::Rdp(config) => Some(config.certificate_policy),
                _ => None,
            })
            .unwrap_or(RdpCertificatePolicy::Prompt);
        let check = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| {
            store.check_rdp_known_host(&request.host, request.port, &request.sha256_fingerprint)
        })
        .unwrap_or(KnownHostCheck::UnknownHost);
        let decision = match policy {
            RdpCertificatePolicy::Insecure => CertificateDecision::Accept,
            RdpCertificatePolicy::TrustOnFirstUse => match check {
                KnownHostCheck::Match => CertificateDecision::Accept,
                KnownHostCheck::UnknownHost => CertificateDecision::AcceptAndRemember,
                KnownHostCheck::HostSeen => CertificateDecision::Reject,
            },
            RdpCertificatePolicy::Strict | RdpCertificatePolicy::RejectChanged => {
                if check == KnownHostCheck::Match {
                    CertificateDecision::Accept
                } else {
                    CertificateDecision::Reject
                }
            }
            RdpCertificatePolicy::Prompt => {
                if check == KnownHostCheck::Match {
                    CertificateDecision::Accept
                } else {
                    CertificateDecision::Prompt
                }
            }
        };
        match decision {
            CertificateDecision::Accept => {
                let _ = self
                    .remote_desktop
                    .manager
                    .respond_certificate(&request.request_id, RdpCertificateResponse::TrustOnce);
            }
            CertificateDecision::AcceptAndRemember => {
                let _ = self.remember_rdp_certificate(&request);
                let _ = self.remote_desktop.manager.respond_certificate(
                    &request.request_id,
                    RdpCertificateResponse::TrustAndRemember,
                );
            }
            CertificateDecision::Reject => {
                let _ = self
                    .remote_desktop
                    .manager
                    .respond_certificate(&request.request_id, RdpCertificateResponse::Reject);
            }
            CertificateDecision::Prompt => {
                if let Some(session) = self.remote_desktop.sessions.get_mut(session_id) {
                    session.certificate_request = Some(request);
                }
            }
        }
    }

    pub(in crate::features) fn resolve_rdp_certificate(
        &mut self,
        session_id: &str,
        response: RdpCertificateResponse,
    ) {
        let request = self
            .remote_desktop
            .sessions
            .get_mut(session_id)
            .and_then(|session| session.certificate_request.take());
        let Some(request) = request else {
            return;
        };
        if response == RdpCertificateResponse::TrustAndRemember {
            let _ = self.remember_rdp_certificate(&request);
        }
        if let Err(error) = self
            .remote_desktop
            .manager
            .respond_certificate(&request.request_id, response)
        {
            self.shell.set_status(format_rdp_error(&error));
        }
    }

    fn remember_rdp_certificate(&self, request: &RdpCertificateRequest) -> anyhow::Result<()> {
        let store = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )?;
        store.upsert_rdp_known_host(
            &request.host,
            request.port,
            &request.sha256_fingerprint,
            RdpCertificateMetadata {
                subject: request.subject.clone(),
                issuer: request.issuer.clone(),
                valid_from: request.valid_from.clone(),
                valid_to: request.valid_to.clone(),
            },
        )?;
        Ok(())
    }

    pub(in crate::features) fn queue_rdp_resize(
        &mut self,
        session_id: &str,
        width: u32,
        height: u32,
    ) {
        let width = width.clamp(200, 8192) & !1;
        let height = height.clamp(200, 8192) & !1;
        if let Some(session) = self.remote_desktop.sessions.get_mut(session_id)
            && session.last_resize != Some((width, height))
        {
            session.pending_resize = Some((width, height, Instant::now()));
        }
    }

    fn drive_rdp_resize_debounce(&mut self) -> bool {
        let now = Instant::now();
        let mut sent = false;
        for (session_id, session) in &mut self.remote_desktop.sessions {
            let Some((width, height, queued_at)) = session.pending_resize else {
                continue;
            };
            if now.saturating_duration_since(queued_at) < RESIZE_DEBOUNCE {
                continue;
            }
            session.pending_resize = None;
            if self
                .remote_desktop
                .manager
                .resize(session_id, width, height)
                .is_ok()
            {
                session.last_resize = Some((width, height));
                sent = true;
            }
        }
        sent
    }

    fn poll_active_rdp_clipboard(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(session_id) = self.session.active_id_owned() else {
            return false;
        };
        if !self.remote_desktop.is_session(&session_id) {
            return false;
        }
        if !self
            .remote_desktop
            .sessions
            .get(&session_id)
            .is_some_and(|session| matches!(session.state, RdpSessionState::Connected))
        {
            return false;
        }
        let clipboard_enabled = self.session.metadata(&session_id).is_some_and(|metadata| {
            matches!(
                &metadata.launch_config,
                crate::models::SessionLaunchConfig::Rdp(config)
                    if config.clipboard.mode == RdpClipboardMode::TextOnly
            )
        });
        if !clipboard_enabled {
            return false;
        }
        let now = Instant::now();
        if self
            .remote_desktop
            .last_clipboard_poll
            .is_some_and(|last| now.saturating_duration_since(last) < CLIPBOARD_POLL_INTERVAL)
        {
            return false;
        }
        self.remote_desktop.last_clipboard_poll = Some(now);
        if !clipboard_has_unicode_text() {
            return false;
        }
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return false;
        };
        let Some(session) = self.remote_desktop.sessions.get_mut(&session_id) else {
            return false;
        };
        let Ok(Some(_generation)) = session.clipboard.accept(ClipboardOrigin::Local, &text) else {
            return false;
        };
        if let Err(error) = self
            .remote_desktop
            .manager
            .set_clipboard_text(&session_id, text)
        {
            session.error = Some(error);
        }
        true
    }
}

#[cfg(target_os = "windows")]
fn clipboard_has_unicode_text() -> bool {
    // GPUI logs every unsupported OLE clipboard format it probes. Only enter
    // that path when Windows reports the text format this bridge accepts.
    unsafe {
        windows_sys::Win32::System::DataExchange::IsClipboardFormatAvailable(
            windows_sys::Win32::System::Ole::CF_UNICODETEXT as u32,
        ) != 0
    }
}

#[cfg(not(target_os = "windows"))]
fn clipboard_has_unicode_text() -> bool {
    true
}

fn upload_rdp_rect(
    window: &mut Window,
    texture: gpui::DynamicTexture,
    framebuffer: &Framebuffer,
    rect: DirtyRect,
) -> anyhow::Result<()> {
    let stride = framebuffer.width() * 4;
    let start = (u64::from(rect.y) * u64::from(stride) + u64::from(rect.x) * 4) as usize;
    let row_bytes = rect.width * 4;
    let len = (u64::from(rect.height - 1) * u64::from(stride) + u64::from(row_bytes)) as usize;
    let pixels = &framebuffer.pixels()[start..start + len];
    window.update_dynamic_texture(
        texture,
        Bounds::new(
            point(DevicePixels(rect.x as i32), DevicePixels(rect.y as i32)),
            size(
                DevicePixels(rect.width as i32),
                DevicePixels(rect.height as i32),
            ),
        ),
        pixels,
        stride,
    )
}

fn set_rdp_view_error(
    session: &mut super::state::RemoteDesktopSessionState,
    kind: RdpErrorKind,
    message: String,
) {
    let error = RdpError::new(kind, message);
    session.error = Some(error.clone());
    session.state = RdpSessionState::Failed(error);
}

fn rdp_error_is_retryable(kind: RdpErrorKind) -> bool {
    matches!(
        kind,
        RdpErrorKind::Timeout
            | RdpErrorKind::ConnectionRefused
            | RdpErrorKind::Tls
            | RdpErrorKind::Transport
            | RdpErrorKind::Session
    )
}

fn rdp_reconnect_delay(attempt: u32, jitter_ms: u64) -> Duration {
    const BACKOFF_SECONDS: [u64; 6] = [1, 2, 4, 8, 15, 30];
    let index = attempt.saturating_sub(1) as usize;
    Duration::from_secs(BACKOFF_SECONDS[index.min(BACKOFF_SECONDS.len() - 1)])
        + Duration::from_millis(jitter_ms.min(249))
}

fn clear_rdp_reconnect_after_frame(session: &mut super::state::RemoteDesktopSessionState) {
    session.reconnect_attempts = 0;
    session.reconnect_at = None;
    session.error = None;
}

pub(super) fn format_rdp_error(error: &RdpError) -> String {
    let category = match error.kind {
        RdpErrorKind::Authentication => "Authentication failed",
        RdpErrorKind::CertificateRejected => "Certificate rejected",
        RdpErrorKind::Timeout => "Connection timed out",
        RdpErrorKind::ConnectionRefused => "Connection refused",
        RdpErrorKind::Tls => "RDP TLS connection failed",
        RdpErrorKind::Transport => "RDP transport interrupted",
        RdpErrorKind::Session => "RDP session failed",
        RdpErrorKind::Clipboard => "RDP clipboard failed",
        RdpErrorKind::Negotiation => "RDP negotiation failed",
        RdpErrorKind::HelperMissing => "RDP helper is missing",
        RdpErrorKind::HelperCrashed => "RDP helper crashed",
        RdpErrorKind::Ipc => "RDP helper communication failed",
        RdpErrorKind::Protocol => "RDP protocol error",
        RdpErrorKind::Unsupported => "RDP feature is unsupported",
    };
    format!("{category}: {}", error.message)
}

fn load_rdp_password(
    app: &NyaTermApp,
    auth: Option<&nyaterm_core::ConnectionAuth>,
) -> Option<String> {
    let auth = auth?;
    if auth.mode == "none" {
        return None;
    }
    if let Some(password) = auth
        .password
        .as_deref()
        .filter(|password| !password.trim().is_empty())
    {
        return (!auth.has_password).then(|| password.to_string());
    }
    let password_id = auth
        .password_id
        .as_deref()
        .map(str::trim)
        .filter(|password_id| !password_id.is_empty())?;
    ConnectionStore::open_with_portable_key_path(
        app.runtime.config_dir(),
        app.runtime.portable_key_path().map(ToOwned::to_owned),
    )
    .ok()?
    .load_decrypted_password_by_id(password_id)
    .ok()??
    .password
    .filter(|password| !password.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use nyaterm_remote_desktop::{RdpError, RdpErrorKind};

    use super::{clear_rdp_reconnect_after_frame, rdp_error_is_retryable, rdp_reconnect_delay};
    use crate::features::remote_desktop::state::RemoteDesktopSessionState;

    #[test]
    fn reconnect_classification_only_accepts_transient_failures() {
        for kind in [
            RdpErrorKind::Timeout,
            RdpErrorKind::ConnectionRefused,
            RdpErrorKind::Tls,
            RdpErrorKind::Transport,
            RdpErrorKind::Session,
        ] {
            assert!(rdp_error_is_retryable(kind), "{kind:?}");
        }
        for kind in [
            RdpErrorKind::Authentication,
            RdpErrorKind::CertificateRejected,
            RdpErrorKind::Negotiation,
            RdpErrorKind::Clipboard,
            RdpErrorKind::HelperMissing,
            RdpErrorKind::HelperCrashed,
            RdpErrorKind::Ipc,
            RdpErrorKind::Protocol,
            RdpErrorKind::Unsupported,
        ] {
            assert!(!rdp_error_is_retryable(kind), "{kind:?}");
        }
    }

    #[test]
    fn reconnect_backoff_caps_and_bounds_jitter() {
        let expected = [1, 2, 4, 8, 15, 30, 30];
        for (index, seconds) in expected.into_iter().enumerate() {
            assert_eq!(
                rdp_reconnect_delay(index as u32 + 1, 0),
                Duration::from_secs(seconds)
            );
        }
        assert_eq!(rdp_reconnect_delay(1, 999), Duration::from_millis(1_249));
    }

    #[test]
    fn first_frame_clears_reconnect_attempt_and_error_state() {
        let mut session = RemoteDesktopSessionState {
            reconnect_attempts: 4,
            reconnect_at: Some(Instant::now()),
            error: Some(RdpError::new(RdpErrorKind::Transport, "interrupted")),
            ..Default::default()
        };

        clear_rdp_reconnect_after_frame(&mut session);

        assert_eq!(session.reconnect_attempts, 0);
        assert_eq!(session.reconnect_at, None);
        assert_eq!(session.error, None);
    }
}

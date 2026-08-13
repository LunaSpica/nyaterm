use std::collections::{HashMap, VecDeque};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use tokio::time::timeout;
use uuid::Uuid;
use vnc::{
    ClientKeyEvent, ClientMouseEvent, PixelFormat as VncPixelFormat, Rect, Screen, VncClient,
    VncConnector, VncEncoding, VncEvent, VncLimits, VncSecurityPolicy, X11Event,
};
use zeroize::Zeroizing;

use crate::{
    MAX_VNC_CLIPBOARD_TEXT_BYTES, MAX_VNC_FRAMEBUFFER_HEIGHT, MAX_VNC_FRAMEBUFFER_WIDTH,
    MAX_VNC_INPUT_BATCH, PixelFormat, RdpFrameEvent, VncError, VncErrorKind, VncInputEvent,
    VncRuntimeEvent, VncSecurityMode, VncSessionConfig, VncSessionDrain, VncSessionState,
};

const FRAME_QUEUE_LIMIT: usize = 64;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(30);
const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(8);
const UPDATE_REQUEST_INTERVAL: Duration = Duration::from_millis(16);

enum WorkerCommand {
    Input(VncInputEvent),
    Clipboard(String),
    FullRefresh,
    Close,
}

#[derive(Default)]
struct EventQueue {
    control: VecDeque<VncRuntimeEvent>,
    frames: VecDeque<RdpFrameEvent>,
    current_epoch: u64,
    waiting_for_full_frame: bool,
    dropped_frames: usize,
}

impl EventQueue {
    fn push_control(&mut self, event: VncRuntimeEvent) {
        self.control.push_back(event);
    }

    fn push_reset(&mut self, session_id: &str, width: u32, height: u32) -> u64 {
        self.current_epoch = self.current_epoch.saturating_add(1).max(1);
        self.frames.clear();
        self.waiting_for_full_frame = true;
        self.control.push_back(VncRuntimeEvent::Frame {
            session_id: session_id.to_string(),
            event: RdpFrameEvent::Reset {
                epoch: self.current_epoch,
                width,
                height,
            },
        });
        self.current_epoch
    }

    fn push_frame(&mut self, frame: RdpFrameEvent) {
        if self.frames.len() >= FRAME_QUEUE_LIMIT {
            self.dropped_frames += self.frames.len() + 1;
            self.frames.clear();
            self.waiting_for_full_frame = true;
        }
        if matches!(&frame, RdpFrameEvent::Bitmap { full: true, .. }) {
            self.waiting_for_full_frame = false;
        }
        self.frames.push_back(frame);
    }

    fn drain(&mut self) -> VncSessionDrain {
        VncSessionDrain {
            control: self.control.drain(..).collect(),
            frames: self.frames.drain(..).collect(),
            dropped_frames: std::mem::take(&mut self.dropped_frames),
            waiting_for_full_frame: self.waiting_for_full_frame,
        }
    }
}

struct SessionRecord {
    state: Arc<Mutex<VncSessionState>>,
    queue: Arc<Mutex<EventQueue>>,
    sender: mpsc::Sender<WorkerCommand>,
    worker: Option<JoinHandle<()>>,
    close_requested: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct VncSessionManager {
    sessions: Mutex<HashMap<String, SessionRecord>>,
}

impl VncSessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_session(&self, config: VncSessionConfig) -> Result<String, VncError> {
        self.create_session_with_id(Uuid::new_v4().to_string(), config)
    }

    pub fn create_session_with_id(
        &self,
        session_id: String,
        config: VncSessionConfig,
    ) -> Result<String, VncError> {
        validate_vnc_config(&config)?;
        let queue = Arc::new(Mutex::new(EventQueue::default()));
        let state = Arc::new(Mutex::new(VncSessionState::Connecting));
        let close_requested = Arc::new(AtomicBool::new(false));
        let (sender, receiver) = mpsc::channel();
        let worker = spawn_worker(
            session_id.clone(),
            config,
            Arc::clone(&queue),
            Arc::clone(&state),
            Arc::clone(&close_requested),
            receiver,
        );
        let mut sessions = self.sessions.lock().map_err(|_| {
            VncError::new(
                VncErrorKind::Internal,
                "VNC session registry lock is poisoned",
            )
        })?;
        if let Some(mut existing) = sessions.remove(&session_id) {
            existing.close_requested.store(true, Ordering::Release);
            let _ = existing.sender.send(WorkerCommand::Close);
            if let Some(worker) = existing.worker.take() {
                let _ = worker.join();
            }
        }
        sessions.insert(
            session_id.clone(),
            SessionRecord {
                state,
                queue,
                sender,
                worker: Some(worker),
                close_requested,
            },
        );
        Ok(session_id)
    }

    pub fn send_input(&self, session_id: &str, events: Vec<VncInputEvent>) -> Result<(), VncError> {
        if events.len() > MAX_VNC_INPUT_BATCH {
            return Err(VncError::new(
                VncErrorKind::Protocol,
                format!("VNC input batch exceeds {MAX_VNC_INPUT_BATCH} events"),
            ));
        }
        let sessions = self.sessions.lock().map_err(|_| {
            VncError::new(
                VncErrorKind::Internal,
                "VNC session registry lock is poisoned",
            )
        })?;
        let record = sessions.get(session_id).ok_or_else(|| {
            VncError::new(
                VncErrorKind::Protocol,
                format!("VNC session '{session_id}' is not running"),
            )
        })?;
        for event in events {
            record
                .sender
                .send(WorkerCommand::Input(event))
                .map_err(|_| VncError::new(VncErrorKind::Transport, "VNC worker channel closed"))?;
        }
        Ok(())
    }

    pub fn set_clipboard_text(&self, session_id: &str, text: String) -> Result<(), VncError> {
        if !is_latin1_within_limit(&text) {
            return Err(VncError::new(
                VncErrorKind::Clipboard,
                "VNC clipboard text must be Latin-1 and no larger than 1 MiB",
            ));
        }
        let sessions = self.sessions.lock().map_err(|_| {
            VncError::new(
                VncErrorKind::Internal,
                "VNC session registry lock is poisoned",
            )
        })?;
        let record = sessions.get(session_id).ok_or_else(|| {
            VncError::new(
                VncErrorKind::Protocol,
                format!("VNC session '{session_id}' is not running"),
            )
        })?;
        record
            .sender
            .send(WorkerCommand::Clipboard(text))
            .map_err(|_| VncError::new(VncErrorKind::Transport, "VNC worker channel closed"))
    }

    pub fn request_full_frame(&self, session_id: &str) -> Result<(), VncError> {
        let sessions = self.sessions.lock().map_err(|_| {
            VncError::new(
                VncErrorKind::Internal,
                "VNC session registry lock is poisoned",
            )
        })?;
        let record = sessions.get(session_id).ok_or_else(|| {
            VncError::new(
                VncErrorKind::Protocol,
                format!("VNC session '{session_id}' is not running"),
            )
        })?;
        record
            .sender
            .send(WorkerCommand::FullRefresh)
            .map_err(|_| VncError::new(VncErrorKind::Transport, "VNC worker channel closed"))
    }

    pub fn drain(&self, session_id: &str) -> VncSessionDrain {
        let Ok(sessions) = self.sessions.lock() else {
            return VncSessionDrain::default();
        };
        sessions
            .get(session_id)
            .and_then(|record| record.queue.lock().ok().map(|mut queue| queue.drain()))
            .unwrap_or_default()
    }

    pub fn close(&self, session_id: &str) -> Result<(), VncError> {
        let mut sessions = self.sessions.lock().map_err(|_| {
            VncError::new(
                VncErrorKind::Internal,
                "VNC session registry lock is poisoned",
            )
        })?;
        let Some(mut record) = sessions.remove(session_id) else {
            return Ok(());
        };
        record.close_requested.store(true, Ordering::Release);
        let _ = record.sender.send(WorkerCommand::Close);
        if let Some(worker) = record.worker.take() {
            let _ = worker.join();
        }
        Ok(())
    }

    pub fn state(&self, session_id: &str) -> Option<VncSessionState> {
        self.sessions.lock().ok().and_then(|sessions| {
            sessions
                .get(session_id)
                .and_then(|record| record.state.lock().ok().map(|state| *state))
        })
    }
}

pub fn validate_vnc_config(config: &VncSessionConfig) -> Result<(), VncError> {
    if config.host.trim().is_empty() {
        return Err(VncError::new(
            VncErrorKind::Protocol,
            "VNC host is required",
        ));
    }
    if matches!(config.security.mode, VncSecurityMode::VncAuth) && config.password.is_none() {
        return Err(VncError::new(
            VncErrorKind::Authentication,
            "VNC Authentication requires a password",
        ));
    }
    if let Some(password) = config.password.as_ref()
        && password.as_bytes().len() > 8
    {
        return Err(VncError::new(
            VncErrorKind::Authentication,
            "Classic VNC authentication passwords must be 8 bytes or fewer",
        ));
    }
    Ok(())
}

pub fn classify_vnc_error(error: vnc::VncError) -> VncError {
    let (kind, message) = match error {
        vnc::VncError::NoPassword | vnc::VncError::WrongPassword => (
            VncErrorKind::Authentication,
            "VNC authentication failed".to_string(),
        ),
        vnc::VncError::UnsupportedSecurityType
        | vnc::VncError::RequiredSecurityTypeUnavailable(_)
        | vnc::VncError::InvalidSecurityType(_) => (
            VncErrorKind::Authentication,
            format!(
                "The VNC server requires an unsupported security type. Currently supported: None and VNC Authentication. Details: {error}"
            ),
        ),
        vnc::VncError::InvalidEncoding(_) | vnc::VncError::InvalidImageData => {
            (VncErrorKind::Encoding, error.to_string())
        }
        vnc::VncError::IoError(_) => (VncErrorKind::Transport, error.to_string()),
        vnc::VncError::LimitExceeded { .. }
        | vnc::VncError::InvalidDimensions
        | vnc::VncError::IntegerOverflow(_)
        | vnc::VncError::WrongPixelFormat
        | vnc::VncError::WrongServerMessage
        | vnc::VncError::InvalidSecurityResult(_)
        | vnc::VncError::SecurityFailure(_) => (VncErrorKind::Protocol, error.to_string()),
        _ => (VncErrorKind::Internal, error.to_string()),
    };
    VncError::new(kind, message)
}

fn spawn_worker(
    session_id: String,
    config: VncSessionConfig,
    queue: Arc<Mutex<EventQueue>>,
    state: Arc<Mutex<VncSessionState>>,
    close_requested: Arc<AtomicBool>,
    receiver: mpsc::Receiver<WorkerCommand>,
) -> JoinHandle<()> {
    thread::Builder::new()
        .name(format!("nyaterm-vnc-{session_id}"))
        .spawn(move || {
            let runtime = Runtime::new();
            match runtime {
                Ok(runtime) => runtime.block_on(run_worker(
                    session_id,
                    config,
                    queue,
                    state,
                    close_requested,
                    receiver,
                )),
                Err(error) => push_error(
                    &queue,
                    &state,
                    &session_id,
                    VncError::new(
                        VncErrorKind::Internal,
                        format!("failed to start VNC runtime: {error}"),
                    ),
                    true,
                ),
            }
        })
        .expect("failed to spawn VNC worker")
}

async fn run_worker(
    session_id: String,
    config: VncSessionConfig,
    queue: Arc<Mutex<EventQueue>>,
    state: Arc<Mutex<VncSessionState>>,
    close_requested: Arc<AtomicBool>,
    receiver: mpsc::Receiver<WorkerCommand>,
) {
    let receiver = Arc::new(Mutex::new(receiver));
    let mut attempt = 0;
    loop {
        if close_requested.load(Ordering::Acquire) {
            set_state(
                &queue,
                &state,
                &session_id,
                VncSessionState::Disconnected,
                None,
            );
            return;
        }
        let connecting_state = if attempt == 0 {
            VncSessionState::Connecting
        } else {
            VncSessionState::Reconnecting
        };
        set_state(&queue, &state, &session_id, connecting_state, None);
        let result = run_generation(
            &session_id,
            &config,
            &queue,
            &state,
            Arc::clone(&receiver),
            Arc::clone(&close_requested),
        )
        .await;
        match result {
            Ok(()) => return,
            Err(error) => {
                if close_requested.load(Ordering::Acquire) {
                    set_state(
                        &queue,
                        &state,
                        &session_id,
                        VncSessionState::Disconnected,
                        None,
                    );
                    return;
                }
                let retryable =
                    matches!(error.kind, VncErrorKind::Transport | VncErrorKind::Internal);
                if !retryable
                    || !config.reconnect.enabled
                    || attempt >= config.reconnect.max_attempts
                {
                    push_error(&queue, &state, &session_id, error, true);
                    return;
                }
                attempt += 1;
                tokio::time::sleep(reconnect_delay(attempt)).await;
            }
        }
    }
}

async fn run_generation(
    session_id: &str,
    config: &VncSessionConfig,
    queue: &Arc<Mutex<EventQueue>>,
    state: &Arc<Mutex<VncSessionState>>,
    receiver: Arc<Mutex<mpsc::Receiver<WorkerCommand>>>,
    close_requested: Arc<AtomicBool>,
) -> Result<(), VncError> {
    let stream = timeout(
        CONNECT_TIMEOUT,
        TcpStream::connect((config.host.as_str(), config.port)),
    )
    .await
    .map_err(|_| VncError::new(VncErrorKind::Transport, "VNC connection timed out"))?
    .map_err(|error| {
        VncError::new(
            VncErrorKind::Transport,
            format!("Unable to connect to the VNC server: {error}"),
        )
    })?;
    set_state(
        queue,
        state,
        session_id,
        VncSessionState::Authenticating,
        None,
    );
    let password = Zeroizing::new(config.password.clone().unwrap_or_default());
    let auth_password = password.to_string();
    let connector = VncConnector::new(stream)
        .set_auth_method(async move { Ok(auth_password) })
        .set_security_policy(security_policy(
            config.security.mode,
            config.password.is_some(),
        ))
        .set_pixel_format(VncPixelFormat::rgba())
        .set_limits(vnc_limits())
        .add_encoding(VncEncoding::DesktopSizePseudo)
        .add_encoding(VncEncoding::Zrle)
        .add_encoding(VncEncoding::Tight)
        .add_encoding(VncEncoding::Raw)
        .allow_shared(config.shared)
        .build()
        .map_err(classify_vnc_error)?;
    let client = timeout(HANDSHAKE_TIMEOUT, connector.try_start())
        .await
        .map_err(|_| {
            VncError::new(
                VncErrorKind::Transport,
                "VNC protocol negotiation timed out",
            )
        })?
        .and_then(|state| state.finish())
        .map_err(classify_vnc_error)?;
    set_state(queue, state, session_id, VncSessionState::Negotiating, None);
    let mut pressed_keys = Vec::new();
    let mut poll = tokio::time::interval(EVENT_POLL_INTERVAL);
    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut refresh_due = false;
    let refresh_delay = tokio::time::sleep(Duration::from_secs(86_400));
    tokio::pin!(refresh_delay);
    loop {
        if close_requested.load(Ordering::Acquire) {
            release_pressed_keys(&client, &mut pressed_keys).await;
            let _ = client.close().await;
            return Ok(());
        }
        tokio::select! {
            _ = poll.tick() => {
                loop {
                    match client.poll_event().await {
                        Ok(Some(event)) => {
                            handle_vnc_event(session_id, queue, state, event)?;
                            refresh_due = true;
                            refresh_delay.as_mut().reset(tokio::time::Instant::now() + UPDATE_REQUEST_INTERVAL);
                        }
                        Ok(None) => break,
                        Err(error) => return Err(classify_vnc_error(error)),
                    }
                }
            }
            _ = &mut refresh_delay, if refresh_due => {
                client.input(X11Event::Refresh).await.map_err(classify_vnc_error)?;
                refresh_due = false;
            }
            command = recv_worker_command(Arc::clone(&receiver)) => {
                match command {
                    Some(WorkerCommand::Input(event)) if !config.view_only => {
                        send_vnc_input(&client, event, &mut pressed_keys).await?;
                    }
                    Some(WorkerCommand::Input(_)) => {}
                    Some(WorkerCommand::Clipboard(text)) if !config.view_only && config.clipboard.enabled => {
                        client.input(X11Event::CopyText(text)).await.map_err(classify_vnc_error)?;
                    }
                    Some(WorkerCommand::Clipboard(_)) => {}
                    Some(WorkerCommand::FullRefresh) => {
                        client.input(X11Event::FullRefresh).await.map_err(classify_vnc_error)?;
                    }
                    Some(WorkerCommand::Close) | None => {
                        release_pressed_keys(&client, &mut pressed_keys).await;
                        let _ = client.close().await;
                        return Ok(());
                    }
                }
            }
        }
    }
}

async fn recv_worker_command(
    receiver: Arc<Mutex<mpsc::Receiver<WorkerCommand>>>,
) -> Option<WorkerCommand> {
    tokio::task::spawn_blocking(move || receiver.lock().ok().and_then(|rx| rx.recv().ok()))
        .await
        .ok()
        .flatten()
}

fn handle_vnc_event(
    session_id: &str,
    queue: &Arc<Mutex<EventQueue>>,
    state: &Arc<Mutex<VncSessionState>>,
    event: VncEvent,
) -> Result<(), VncError> {
    match event {
        VncEvent::SetResolution(Screen { width, height }) => {
            validate_framebuffer_dimensions(u32::from(width), u32::from(height))?;
            let mut queue = queue.lock().map_err(|_| {
                VncError::new(VncErrorKind::Internal, "VNC event queue lock is poisoned")
            })?;
            queue.push_reset(session_id, u32::from(width), u32::from(height));
        }
        VncEvent::RawImage(rect, pixels) => {
            let epoch = {
                let mut event_queue = queue.lock().map_err(|_| {
                    VncError::new(VncErrorKind::Internal, "VNC event queue lock is poisoned")
                })?;
                if event_queue.current_epoch == 0 {
                    let width = u32::from(rect.x) + u32::from(rect.width);
                    let height = u32::from(rect.y) + u32::from(rect.height);
                    event_queue.push_reset(session_id, width, height)
                } else {
                    event_queue.current_epoch
                }
            };
            validate_rect(rect)?;
            let frame = RdpFrameEvent::Bitmap {
                epoch,
                full: rect.x == 0 && rect.y == 0,
                x: u32::from(rect.x),
                y: u32::from(rect.y),
                width: u32::from(rect.width),
                height: u32::from(rect.height),
                stride: u32::from(rect.width) * 4,
                format: PixelFormat::Rgba8,
                pixels,
            };
            let mut event_queue = queue.lock().map_err(|_| {
                VncError::new(VncErrorKind::Internal, "VNC event queue lock is poisoned")
            })?;
            event_queue.push_frame(frame);
            drop(event_queue);
            set_state(queue, state, session_id, VncSessionState::Connected, None);
        }
        VncEvent::Text(text) if is_latin1_within_limit(&text) => {
            queue
                .lock()
                .map_err(|_| {
                    VncError::new(VncErrorKind::Internal, "VNC event queue lock is poisoned")
                })?
                .push_control(VncRuntimeEvent::Clipboard {
                    session_id: session_id.to_string(),
                    text,
                });
        }
        VncEvent::Error(message) => {
            return Err(VncError::new(VncErrorKind::Protocol, message));
        }
        VncEvent::JpegImage(_, _) => {
            return Err(VncError::new(
                VncErrorKind::Encoding,
                "The server sent a Tight JPEG event instead of decoded RGBA pixels",
            ));
        }
        VncEvent::Copy(_, _) | VncEvent::SetCursor(_, _) => {
            return Err(VncError::new(
                VncErrorKind::Encoding,
                "The server sent an unrequested VNC encoding",
            ));
        }
        VncEvent::SetPixelFormat(_) | VncEvent::Bell => {}
        _ => {}
    }
    Ok(())
}

async fn send_vnc_input(
    client: &VncClient,
    event: VncInputEvent,
    pressed_keys: &mut Vec<u32>,
) -> Result<(), VncError> {
    match event {
        VncInputEvent::Key { keysym, pressed } => {
            if pressed {
                if !pressed_keys.contains(&keysym) {
                    pressed_keys.push(keysym);
                }
            } else {
                pressed_keys.retain(|key| *key != keysym);
            }
            client
                .input(X11Event::KeyEvent(ClientKeyEvent {
                    keycode: keysym,
                    down: pressed,
                }))
                .await
                .map_err(classify_vnc_error)?;
        }
        VncInputEvent::Pointer { x, y, button_mask } => {
            client
                .input(X11Event::PointerEvent(ClientMouseEvent {
                    position_x: u16::try_from(x.min(u32::from(u16::MAX))).unwrap_or(u16::MAX),
                    position_y: u16::try_from(y.min(u32::from(u16::MAX))).unwrap_or(u16::MAX),
                    bottons: button_mask,
                }))
                .await
                .map_err(classify_vnc_error)?;
        }
        VncInputEvent::ReleaseAllKeys => release_pressed_keys(client, pressed_keys).await,
    }
    Ok(())
}

async fn release_pressed_keys(client: &VncClient, pressed_keys: &mut Vec<u32>) {
    for keysym in pressed_keys.drain(..) {
        let _ = client
            .input(X11Event::KeyEvent(ClientKeyEvent {
                keycode: keysym,
                down: false,
            }))
            .await;
    }
}

fn set_state(
    queue: &Arc<Mutex<EventQueue>>,
    state: &Arc<Mutex<VncSessionState>>,
    session_id: &str,
    next: VncSessionState,
    message: Option<String>,
) {
    if let Ok(mut state) = state.lock() {
        *state = next;
    }
    if let Ok(mut queue) = queue.lock() {
        queue.push_control(VncRuntimeEvent::State {
            session_id: session_id.to_string(),
            state: next,
            message,
        });
    }
}

fn push_error(
    queue: &Arc<Mutex<EventQueue>>,
    state: &Arc<Mutex<VncSessionState>>,
    session_id: &str,
    error: VncError,
    fatal: bool,
) {
    if let Ok(mut state) = state.lock() {
        *state = VncSessionState::Failed;
    }
    if let Ok(mut queue) = queue.lock() {
        queue.push_control(VncRuntimeEvent::Error {
            session_id: session_id.to_string(),
            error,
            fatal,
        });
    }
}

fn security_policy(mode: VncSecurityMode, has_password: bool) -> VncSecurityPolicy {
    match mode {
        VncSecurityMode::None => VncSecurityPolicy::NoneOnly,
        VncSecurityMode::VncAuth => VncSecurityPolicy::VncAuthOnly,
        VncSecurityMode::Auto if has_password => VncSecurityPolicy::VncAuthOnly,
        VncSecurityMode::Auto => VncSecurityPolicy::NoneOnly,
    }
}

fn vnc_limits() -> VncLimits {
    VncLimits {
        max_framebuffer_width: u16::try_from(MAX_VNC_FRAMEBUFFER_WIDTH).unwrap_or(u16::MAX),
        max_framebuffer_height: u16::try_from(MAX_VNC_FRAMEBUFFER_HEIGHT).unwrap_or(u16::MAX),
        max_framebuffer_pixels: usize::try_from(
            MAX_VNC_FRAMEBUFFER_WIDTH * MAX_VNC_FRAMEBUFFER_HEIGHT,
        )
        .unwrap_or(usize::MAX),
        max_clipboard_bytes: MAX_VNC_CLIPBOARD_TEXT_BYTES,
        max_rectangles_per_update: 1024,
        max_encoded_payload_bytes: 64 * 1024 * 1024,
        max_decoded_payload_bytes: usize::try_from(
            MAX_VNC_FRAMEBUFFER_WIDTH * MAX_VNC_FRAMEBUFFER_HEIGHT * 4,
        )
        .unwrap_or(usize::MAX),
        channel_capacity: 32,
        ..VncLimits::default()
    }
}

fn reconnect_delay(attempt: u32) -> Duration {
    Duration::from_secs(match attempt {
        0 | 1 => 1,
        2 => 2,
        3 => 4,
        4 => 8,
        5 => 15,
        _ => 30,
    })
}

fn validate_framebuffer_dimensions(width: u32, height: u32) -> Result<(), VncError> {
    if width == 0
        || height == 0
        || width > MAX_VNC_FRAMEBUFFER_WIDTH
        || height > MAX_VNC_FRAMEBUFFER_HEIGHT
    {
        return Err(VncError::new(
            VncErrorKind::Protocol,
            format!("VNC framebuffer {width}x{height} is outside the supported range"),
        ));
    }
    Ok(())
}

fn validate_rect(rect: Rect) -> Result<(), VncError> {
    if rect.width == 0 || rect.height == 0 {
        return Err(VncError::new(
            VncErrorKind::Protocol,
            "VNC rectangle must be non-empty",
        ));
    }
    Ok(())
}

fn is_latin1_within_limit(text: &str) -> bool {
    text.len() <= MAX_VNC_CLIPBOARD_TEXT_BYTES && text.chars().all(|ch| u32::from(ch) <= 0xff)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Framebuffer, VncClipboardConfig, VncDisplayConfig, VncReconnectConfig, VncSecurityConfig,
    };

    fn config() -> VncSessionConfig {
        VncSessionConfig {
            name: "vnc".to_string(),
            host: "127.0.0.1".to_string(),
            port: 5900,
            password: None,
            security: VncSecurityConfig::default(),
            display: VncDisplayConfig::default(),
            clipboard: VncClipboardConfig::default(),
            reconnect: VncReconnectConfig::default(),
            shared: true,
            view_only: false,
        }
    }

    #[test]
    fn validates_password_length_for_classic_vnc_auth() {
        let mut config = config();
        config.security.mode = VncSecurityMode::VncAuth;
        config.password = Some("123456789".to_string());
        let error = validate_vnc_config(&config).expect_err("long password should fail");
        assert_eq!(error.kind, VncErrorKind::Authentication);
    }

    #[test]
    fn validates_clipboard_latin1_limit() {
        assert!(is_latin1_within_limit("hello"));
        assert!(!is_latin1_within_limit("hello \u{0100}"));
        assert!(!is_latin1_within_limit(
            &"a".repeat(MAX_VNC_CLIPBOARD_TEXT_BYTES + 1)
        ));
    }

    #[test]
    fn rgba_vnc_frame_reaches_shared_framebuffer_as_bgra() {
        let mut framebuffer = Framebuffer::new(1, 1, 1).expect("framebuffer");
        let frame = RdpFrameEvent::Bitmap {
            epoch: 1,
            full: true,
            x: 0,
            y: 0,
            width: 1,
            height: 1,
            stride: 4,
            format: PixelFormat::Rgba8,
            pixels: vec![10, 20, 30, 255],
        };
        framebuffer.apply(&frame).expect("apply");
        assert_eq!(framebuffer.pixels(), &[30, 20, 10, 255]);
    }
}

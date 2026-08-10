use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use uuid::Uuid;

const RDP_MIN_WIDTH: u32 = 640;
const RDP_MIN_HEIGHT: u32 = 480;
const RDP_MAX_WIDTH: u32 = 7680;
const RDP_MAX_HEIGHT: u32 = 4320;
const RDP_EVENT_QUEUE_LIMIT: usize = 128;
const RDP_RUNTIME_UNAVAILABLE: &str = "native RDP runtime is not wired yet: upstream ironrdp-client 0.1.0 currently pulls pre-release crypto dependencies that conflict with NyaTerm's credential and SSH dependency set";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RdpSessionConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub domain: String,
    pub password: Option<String>,
    pub use_nla: bool,
    pub certificate_policy: RdpCertificatePolicy,
    pub display: RdpDisplayConfig,
    pub clipboard: RdpClipboardConfig,
    pub reconnect: RdpReconnectConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RdpCertificatePolicy {
    #[default]
    Prompt,
    TrustOnFirstUse,
    RejectChanged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RdpDisplayConfig {
    pub mode: RdpDisplayMode,
    pub width: u32,
    pub height: u32,
    pub color_depth: u8,
}

impl Default for RdpDisplayConfig {
    fn default() -> Self {
        Self {
            mode: RdpDisplayMode::FitWindow,
            width: 1920,
            height: 1080,
            color_depth: 32,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RdpDisplayMode {
    #[default]
    FitWindow,
    Fixed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RdpClipboardConfig {
    pub mode: RdpClipboardMode,
}

impl Default for RdpClipboardConfig {
    fn default() -> Self {
        Self {
            mode: RdpClipboardMode::TextOnly,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RdpClipboardMode {
    Disabled,
    #[default]
    TextOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RdpReconnectConfig {
    pub enabled: bool,
    pub max_attempts: u32,
}

impl Default for RdpReconnectConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_attempts: 5,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RdpInputEvent {
    KeyDown {
        scan_code: u16,
        extended: bool,
        repeat: bool,
    },
    KeyUp {
        scan_code: u16,
        extended: bool,
        repeat: bool,
    },
    Unicode {
        text: String,
    },
    Pointer {
        x: u32,
        y: u32,
        button: Option<RdpPointerButton>,
        pressed: bool,
    },
    ReleaseAllKeys,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RdpPointerButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RdpFrameEvent {
    Resize {
        width: u32,
        height: u32,
    },
    Bitmap {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    Cursor {
        visible: bool,
        x: u32,
        y: u32,
        hotspot_x: u32,
        hotspot_y: u32,
        rgba: Vec<u8>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RdpRuntimeEvent {
    State {
        session_id: String,
        state: RdpSessionState,
        message: Option<String>,
    },
    Frame {
        session_id: String,
        event: RdpFrameEvent,
    },
    CertificateRequest(RdpCertificateRequest),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RdpSessionState {
    Connecting,
    Active,
    AwaitingCertificate(RdpCertificateRequest),
    Failed(String),
    Closed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RdpCertificateRequest {
    pub request_id: String,
    pub host: String,
    pub port: u16,
    pub sha256_fingerprint: String,
    pub subject: Option<String>,
    pub issuer: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
}

#[derive(Default)]
pub struct RdpSessionManager {
    sessions: Mutex<HashMap<String, RdpSessionRecord>>,
    pending_certificate_requests: Mutex<HashMap<String, String>>,
}

struct RdpSessionRecord {
    state: RdpSessionState,
    events: VecDeque<RdpRuntimeEvent>,
}

impl RdpSessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn create_session(&self, config: RdpSessionConfig) -> anyhow::Result<String> {
        validate_rdp_config(&config)?;
        let session_id = Uuid::new_v4().to_string();
        let state = RdpSessionState::Failed(RDP_RUNTIME_UNAVAILABLE.to_string());
        let mut record = RdpSessionRecord {
            state: state.clone(),
            events: VecDeque::new(),
        };
        push_event(
            &mut record.events,
            RdpRuntimeEvent::State {
                session_id: session_id.clone(),
                state,
                message: Some(RDP_RUNTIME_UNAVAILABLE.to_string()),
            },
        );
        self.sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(session_id.clone(), record);
        Ok(session_id)
    }

    pub fn close(&self, session_id: &str) {
        if let Some(record) = self
            .sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get_mut(session_id)
        {
            record.state = RdpSessionState::Closed;
            push_event(
                &mut record.events,
                RdpRuntimeEvent::State {
                    session_id: session_id.to_string(),
                    state: RdpSessionState::Closed,
                    message: Some("RDP session closed".to_string()),
                },
            );
        }
        self.pending_certificate_requests
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|_, pending_session_id| pending_session_id != session_id);
    }

    pub fn state(&self, session_id: &str) -> Option<RdpSessionState> {
        self.sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(session_id)
            .map(|record| record.state.clone())
    }

    pub fn drain_events(&self, session_id: &str) -> Vec<RdpRuntimeEvent> {
        let mut sessions = self
            .sessions
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(record) = sessions.get_mut(session_id) else {
            return Vec::new();
        };
        record.events.drain(..).collect()
    }

    pub fn respond_certificate(&self, request_id: &str, _accepted: bool) -> anyhow::Result<()> {
        let removed = self
            .pending_certificate_requests
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(request_id);
        if removed.is_some() {
            return Ok(());
        }
        anyhow::bail!("RDP certificate request '{request_id}' was not found")
    }

    pub fn send_input(&self, session_id: &str, _events: Vec<RdpInputEvent>) -> anyhow::Result<()> {
        ensure_session_exists(&self.sessions, session_id)?;
        anyhow::bail!("{RDP_RUNTIME_UNAVAILABLE}")
    }

    pub fn resize(&self, session_id: &str, width: u32, height: u32) -> anyhow::Result<()> {
        ensure_session_exists(&self.sessions, session_id)?;
        validate_rdp_size(width, height)?;
        anyhow::bail!("{RDP_RUNTIME_UNAVAILABLE}")
    }

    pub fn set_clipboard_text(&self, session_id: &str, _text: String) -> anyhow::Result<()> {
        ensure_session_exists(&self.sessions, session_id)?;
        anyhow::bail!("{RDP_RUNTIME_UNAVAILABLE}")
    }
}

fn ensure_session_exists(
    sessions: &Mutex<HashMap<String, RdpSessionRecord>>,
    session_id: &str,
) -> anyhow::Result<()> {
    if sessions
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .contains_key(session_id)
    {
        Ok(())
    } else {
        anyhow::bail!("RDP session '{session_id}' was not found")
    }
}

fn push_event(events: &mut VecDeque<RdpRuntimeEvent>, event: RdpRuntimeEvent) {
    if events.len() >= RDP_EVENT_QUEUE_LIMIT {
        events.pop_front();
    }
    events.push_back(event);
}

fn validate_rdp_config(config: &RdpSessionConfig) -> anyhow::Result<()> {
    if config.host.trim().is_empty() {
        anyhow::bail!("RDP host is required");
    }
    validate_rdp_size(config.display.width, config.display.height)?;
    if !matches!(config.display.color_depth, 16 | 32) {
        anyhow::bail!("RDP color depth must be 16 or 32");
    }
    Ok(())
}

fn validate_rdp_size(width: u32, height: u32) -> anyhow::Result<()> {
    if !(RDP_MIN_WIDTH..=RDP_MAX_WIDTH).contains(&width) {
        anyhow::bail!("RDP width is outside the supported range");
    }
    if !(RDP_MIN_HEIGHT..=RDP_MAX_HEIGHT).contains(&height) {
        anyhow::bail!("RDP height is outside the supported range");
    }
    Ok(())
}

pub fn parse_rdp_certificate_policy(value: &str) -> RdpCertificatePolicy {
    match value.trim() {
        "trust_on_first_use" | "tofu" => RdpCertificatePolicy::TrustOnFirstUse,
        "reject_changed" => RdpCertificatePolicy::RejectChanged,
        _ => RdpCertificatePolicy::Prompt,
    }
}

pub fn parse_rdp_display_mode(value: &str) -> RdpDisplayMode {
    match value.trim() {
        "fixed" => RdpDisplayMode::Fixed,
        _ => RdpDisplayMode::FitWindow,
    }
}

pub fn parse_rdp_clipboard_mode(value: &str) -> RdpClipboardMode {
    match value.trim() {
        "disabled" | "off" => RdpClipboardMode::Disabled,
        _ => RdpClipboardMode::TextOnly,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RDP_RUNTIME_UNAVAILABLE, RdpCertificatePolicy, RdpClipboardMode, RdpDisplayConfig,
        RdpDisplayMode, RdpRuntimeEvent, RdpSessionConfig, RdpSessionManager, RdpSessionState,
        parse_rdp_certificate_policy, parse_rdp_clipboard_mode, parse_rdp_display_mode,
    };

    fn test_config() -> RdpSessionConfig {
        RdpSessionConfig {
            name: "rdp".to_string(),
            host: "127.0.0.1".to_string(),
            port: 3389,
            username: "user".to_string(),
            domain: String::new(),
            password: Some("secret".to_string()),
            use_nla: true,
            certificate_policy: RdpCertificatePolicy::Prompt,
            display: RdpDisplayConfig::default(),
            clipboard: Default::default(),
            reconnect: Default::default(),
        }
    }

    #[test]
    fn rdp_string_options_match_saved_connection_values() {
        assert_eq!(
            parse_rdp_certificate_policy("trust_on_first_use"),
            RdpCertificatePolicy::TrustOnFirstUse
        );
        assert_eq!(
            parse_rdp_certificate_policy("reject_changed"),
            RdpCertificatePolicy::RejectChanged
        );
        assert_eq!(
            parse_rdp_certificate_policy("prompt"),
            RdpCertificatePolicy::Prompt
        );
        assert_eq!(parse_rdp_display_mode("fixed"), RdpDisplayMode::Fixed);
        assert_eq!(
            parse_rdp_display_mode("fit-window"),
            RdpDisplayMode::FitWindow
        );
        assert_eq!(
            parse_rdp_clipboard_mode("disabled"),
            RdpClipboardMode::Disabled
        );
        assert_eq!(
            parse_rdp_clipboard_mode("text-only"),
            RdpClipboardMode::TextOnly
        );
    }

    #[test]
    fn rdp_manager_rejects_invalid_display_size_before_runtime() {
        let manager = RdpSessionManager::new();
        let mut config = test_config();
        config.display.width = 10;

        let error = manager
            .create_session(config)
            .expect_err("invalid config should fail");
        assert!(error.to_string().contains("width"));
    }

    #[test]
    fn rdp_manager_records_runtime_unavailable_state() {
        let manager = RdpSessionManager::new();
        let session_id = manager
            .create_session(test_config())
            .expect("create rdp session");

        assert!(matches!(
            manager.state(&session_id),
            Some(RdpSessionState::Failed(message)) if message == RDP_RUNTIME_UNAVAILABLE
        ));
        let events = manager.drain_events(&session_id);
        assert!(events.iter().any(|event| {
            matches!(
                event,
                RdpRuntimeEvent::State {
                    state: RdpSessionState::Failed(message),
                    ..
                } if message == RDP_RUNTIME_UNAVAILABLE
            )
        }));
    }

    #[test]
    fn rdp_manager_validates_resize_before_reporting_runtime_unavailable() {
        let manager = RdpSessionManager::new();
        let session_id = manager
            .create_session(test_config())
            .expect("create rdp session");

        let invalid = manager.resize(&session_id, 10, 1080).unwrap_err();
        assert!(invalid.to_string().contains("width"));

        let unavailable = manager.resize(&session_id, 1280, 720).unwrap_err();
        assert_eq!(unavailable.to_string(), RDP_RUNTIME_UNAVAILABLE);
    }
}

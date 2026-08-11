use std::fmt;

use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

pub const PROTOCOL_VERSION: u32 = 2;

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl fmt::Debug for RdpSessionConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RdpSessionConfig")
            .field("name", &self.name)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field("domain", &self.domain)
            .field("password", &self.password.as_ref().map(|_| "[REDACTED]"))
            .field("use_nla", &self.use_nla)
            .field("certificate_policy", &self.certificate_policy)
            .field("display", &self.display)
            .field("clipboard", &self.clipboard)
            .field("reconnect", &self.reconnect)
            .finish()
    }
}

impl Drop for RdpSessionConfig {
    fn drop(&mut self) {
        if let Some(password) = self.password.as_mut() {
            password.zeroize();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RdpCertificatePolicy {
    #[default]
    Prompt,
    TrustOnFirstUse,
    Strict,
    #[serde(alias = "accept-temporarily")]
    Insecure,
    #[serde(alias = "reject_changed")]
    RejectChanged,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RdpDisplayMode {
    #[default]
    FitWindow,
    Fixed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RdpClipboardMode {
    Disabled,
    #[default]
    TextOnly,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RdpPointerButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PixelFormat {
    Bgra8,
    Rgba8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RdpFrameEvent {
    Reset {
        epoch: u64,
        width: u32,
        height: u32,
    },
    Bitmap {
        epoch: u64,
        full: bool,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        stride: u32,
        format: PixelFormat,
        pixels: Vec<u8>,
    },
    Cursor(RdpCursorEvent),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RdpCursorEvent {
    pub epoch: u64,
    pub visible: bool,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub hotspot_x: u32,
    pub hotspot_y: u32,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RdpSessionState {
    Idle,
    Connecting,
    Connected,
    Reconnecting,
    Disconnecting,
    Disconnected,
    Failed(RdpError),
    #[deprecated(note = "use Connected")]
    Active,
    #[deprecated(note = "use Disconnected")]
    Closed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RdpCertificateResponse {
    Reject,
    TrustOnce,
    TrustAndRemember,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RdpCapability {
    DynamicResizeUnavailable,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RdpErrorKind {
    Authentication,
    CertificateRejected,
    Timeout,
    ConnectionRefused,
    Tls,
    Transport,
    Session,
    Clipboard,
    Negotiation,
    HelperMissing,
    HelperCrashed,
    Ipc,
    Protocol,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
#[error("{kind:?}: {message}")]
pub struct RdpError {
    pub kind: RdpErrorKind,
    pub message: String,
}

impl RdpError {
    pub fn new(kind: RdpErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
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
    Clipboard {
        session_id: String,
        text: String,
        generation: u64,
    },
    CertificateRequest(RdpCertificateRequest),
    Capability {
        session_id: String,
        capability: RdpCapability,
    },
    Error {
        session_id: String,
        error: RdpError,
        fatal: bool,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RdpSessionDrain {
    pub control: Vec<RdpRuntimeEvent>,
    pub frames: Vec<RdpFrameEvent>,
    pub dropped_frames: usize,
    pub waiting_for_full_frame: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RdpControlMessage {
    ClientHello {
        version: u32,
    },
    ServerHello {
        version: u32,
    },
    Connect {
        session_id: String,
        config: RdpSessionConfig,
    },
    DesktopReset {
        session_id: String,
        epoch: u64,
        width: u32,
        height: u32,
    },
    State {
        session_id: String,
        state: RdpSessionState,
        message: Option<String>,
    },
    Input {
        session_id: String,
        events: Vec<RdpInputEvent>,
    },
    Resize {
        session_id: String,
        width: u32,
        height: u32,
    },
    Clipboard {
        session_id: String,
        text: String,
        generation: u64,
    },
    CertificateRequest(RdpCertificateRequest),
    CertificateResponse {
        request_id: String,
        response: RdpCertificateResponse,
    },
    Capability {
        session_id: String,
        capability: RdpCapability,
    },
    Error {
        session_id: String,
        error: RdpError,
        fatal: bool,
    },
    RequestFullFrame {
        session_id: String,
    },
    Disconnect {
        session_id: String,
    },
}

pub fn parse_rdp_certificate_policy(value: &str) -> RdpCertificatePolicy {
    match value.trim() {
        "trust_on_first_use" | "trust-on-first-use" | "tofu" => {
            RdpCertificatePolicy::TrustOnFirstUse
        }
        "strict" | "reject_changed" | "reject-changed" => RdpCertificatePolicy::Strict,
        "insecure" | "accept-temporarily" => RdpCertificatePolicy::Insecure,
        _ => RdpCertificatePolicy::Prompt,
    }
}

pub fn parse_rdp_display_mode(value: &str) -> RdpDisplayMode {
    match value.trim() {
        "fixed" | "native" => RdpDisplayMode::Fixed,
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
        RdpCertificatePolicy, RdpDisplayMode, parse_rdp_certificate_policy, parse_rdp_display_mode,
    };

    #[test]
    fn certificate_policy_accepts_public_and_legacy_names() {
        assert_eq!(
            parse_rdp_certificate_policy("accept-temporarily"),
            RdpCertificatePolicy::Insecure
        );
        assert_eq!(
            parse_rdp_certificate_policy("insecure"),
            RdpCertificatePolicy::Insecure
        );
        assert_eq!(
            parse_rdp_certificate_policy("tofu"),
            RdpCertificatePolicy::TrustOnFirstUse
        );
        assert_eq!(
            parse_rdp_certificate_policy("reject-changed"),
            RdpCertificatePolicy::Strict
        );
    }

    #[test]
    fn legacy_native_display_mode_is_fixed() {
        assert_eq!(parse_rdp_display_mode("native"), RdpDisplayMode::Fixed);
    }
}

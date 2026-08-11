mod certificate;
mod clipboard;
mod frame;
mod input;
mod ipc;
mod protocol;
mod session;

pub use certificate::{CertificateDecision, evaluate_certificate};
pub use clipboard::{ClipboardOrigin, ClipboardTracker, MAX_CLIPBOARD_TEXT_BYTES};
pub use frame::{DirtyRect, Framebuffer, FramebufferError, merge_dirty_rects};
pub use input::{KeyMapper, RemoteKey, viewport_to_remote};
pub use ipc::{
    CONTROL_PAYLOAD_LIMIT, FRAME_PAYLOAD_LIMIT, HEADER_LEN, Packet, PacketReader, PacketType,
    decode_control, decode_cursor_packet, decode_frame_packet, encode_control,
    encode_cursor_packet, encode_frame_packet, read_packet, write_packet,
};
pub use protocol::{
    PROTOCOL_VERSION, PixelFormat, RdpCapability, RdpCertificatePolicy, RdpCertificateRequest,
    RdpCertificateResponse, RdpClipboardConfig, RdpClipboardMode, RdpControlMessage,
    RdpCursorEvent, RdpDisplayConfig, RdpDisplayMode, RdpError, RdpErrorKind, RdpFrameEvent,
    RdpInputEvent, RdpPointerButton, RdpReconnectConfig, RdpRuntimeEvent, RdpSessionConfig,
    RdpSessionDrain, RdpSessionState, parse_rdp_certificate_policy, parse_rdp_clipboard_mode,
    parse_rdp_display_mode,
};
pub use session::{RdpSessionManager, resolve_helper_path};

use crate::theme::ThemePalette;
use gpui::rgb;
use nyaterm_core::{
    AppSettingsSummary, CloudSyncError, CloudSyncHistoryEntry, CloudSyncSettings, RiskLevel,
    TunnelConfig,
};
use nyaterm_transport::{
    SessionKind, SshSessionConfig, SshTunnelMode, TelnetEnterMode, safe_recording_name,
};

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::AiAgentStepStatus;


#[path = "formatting/labels.rs"]
mod labels;
pub(in crate::features) use labels::*;

#[path = "formatting/ai_history.rs"]
mod ai_history;
pub(in crate::features) use ai_history::*;

#[path = "formatting/connection_icons.rs"]
mod connection_icons;
pub(in crate::features) use connection_icons::*;

#[path = "formatting/markdown.rs"]
mod markdown;
pub(in crate::features) use markdown::*;

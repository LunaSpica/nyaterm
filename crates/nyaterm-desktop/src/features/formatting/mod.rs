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

use super::{AiAgentStepStatus, NyaTermApp};

mod labels;
pub(in crate::features) use labels::*;

mod ai_history;
pub(in crate::features) use ai_history::*;

mod markdown;
pub(in crate::features) use markdown::*;

impl NyaTermApp {
    pub(in crate::features) fn tr(&self, key: &'static str) -> &'static str {
        crate::i18n::text(&self.settings.language, key)
    }
}

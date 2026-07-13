use gpui::{
    App, ClickEvent, FontStyle, FontWeight, HighlightStyle, IntoElement, SharedString,
    StrikethroughStyle, StyledText, UnderlineStyle, Window, WindowControlArea, div, prelude::*, px,
    rgb, rgba, svg,
};
use nyaterm_core::{
    CloudSyncHistoryEntry, ConnectionType, NativeServiceStatus, SavedConnection, TunnelConfig,
    truncate_preview,
};
use nyaterm_transport::{DockerContainer, NetworkInfo, RemoteProcess};

use crate::models::WorkspaceSplitDirection;
use crate::widgets::{mode_button, small_button, status_pill};

use super::{
    ConnectionIconDef, InlineMdStyle, MarkdownBlock, ThemePalette, cloud_sync_history_summary,
    cloud_sync_kind_text_color, cloud_sync_status_dot_color, cloud_sync_status_text_color,
    compact_id, docker_state_color, docker_state_label, format_cloud_provider, format_duration_ms,
    format_history_timestamp_ms, format_rate, parse_inline_markdown, parse_markdown_blocks,
    tunnel_endpoint, tunnel_mode_label, tunnel_name,
};

#[path = "view_widgets/chrome.rs"]
mod chrome;
pub(in crate::features) use chrome::*;

#[path = "view_widgets/inspector_widgets.rs"]
mod inspector_widgets;
pub(in crate::features) use inspector_widgets::*;

#[path = "view_widgets/stats.rs"]
mod stats;
pub(in crate::features) use stats::*;
#[path = "view_widgets/rows.rs"]
mod rows;
pub(in crate::features) use rows::*;

#[path = "view_widgets/icons.rs"]
mod icons;
pub(in crate::features) use icons::*;

#[path = "view_widgets/markdown.rs"]
mod markdown;
pub(in crate::features) use markdown::*;

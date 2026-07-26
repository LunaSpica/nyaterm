use gpui::{
    App, ClickEvent, FontStyle, FontWeight, HighlightStyle, IntoElement, SharedString,
    StrikethroughStyle, StyledText, TitlebarOptions, UnderlineStyle, Window, WindowControlArea,
    div, prelude::*, px, rgb, rgba, svg,
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
    compact_id, docker_state_color, docker_state_label, format_history_timestamp_ms, format_rate,
    parse_inline_markdown, parse_markdown_blocks, tunnel_endpoint, tunnel_mode_label, tunnel_name,
};

mod chrome;
pub(in crate::features) use chrome::*;

mod inspector_widgets;
pub(in crate::features) use inspector_widgets::*;

mod stats;
pub(in crate::features) use stats::*;
mod rows;
pub(in crate::features) use rows::*;

mod icons;
pub(in crate::features) use icons::*;

mod markdown;
pub(in crate::features) use markdown::*;

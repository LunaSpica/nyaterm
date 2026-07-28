use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, mpsc};
use std::time::Instant;

use gpui::{FocusHandle, ScrollHandle, WindowHandle};
use nyaterm_core::{
    AppRuntime, AppSettingsSummary, CommandHistoryEntry, Group, KeywordHighlightConfig,
    NativeServices, OtpEntry, ProxyConfig, ProxyGroup, QuickCommand, QuickCommandCategory,
    SavedConnection, SavedCredential, SavedPassword, SshKey, TunnelConfig, TunnelGroup,
};
use nyaterm_legacy::MigrationInventory;
use nyaterm_transport::SessionEvent;

use super::ai::AiFeatureState;
use super::commands::QuickCommandFeatureState;
use super::connections::ConnectionFeatureState;
use super::panels::SendCommandFeatureState;
use super::recording::RecordingFeatureState;
use super::remote::RemoteOpsFeatureState;
use super::remote_editor_window::RemoteFileEditorWindow;
use super::runtime_jobs::{CommandPersistenceRequest, CommandPersistenceResult};
use super::session::SessionFeatureState;
use super::settings::{SecurityFeatureState, SettingsFeatureState};
use super::settings_window::SettingsWindow;
use super::sync::CloudSyncFeatureState;
use super::terminal::TerminalFeatureState;
use super::text_inputs::TextInputRegistry;
use super::transfers::TransferFeatureState;
use super::translation::TranslationFeatureState;
use super::tunnels::TunnelFeatureState;
use super::update::UpdateFeatureState;
use crate::models::{
    ActivityBarContextMenuState, ActivityBarLayoutState, BottomPanelMode, BottomPanelResizeState,
    ConfigPathPromptKind, DiagnosticsPathPromptKind, HeaderStatusState,
    KeywordHighlightPathPromptKind, MainMode, NavItem, PanelResizeState, PanelStackResizeState,
    RightFocus, SettingsTab, SnapshotPasswordPromptState, StoreStatus, SyncInputGroup, TitleMenu,
    TitleMenuSubmenu, WorkspacePaneNode, WorkspaceSplitResizeState, WorkspaceSplitState,
};

mod construct;
mod types;

pub(in crate::features) use types::{
    PendingSavedConnectionStart, SavedConnectionStartOptions, SettingsDraftSnapshot,
    TerminalRuntimeUiState,
};

pub struct NyaTermApp {
    pub(in crate::features) stores: crate::entities::UiStoreHandles,
    pub(in crate::features) runtime: AppRuntime,
    pub(in crate::features) services: NativeServices,
    pub(in crate::features) inventory: MigrationInventory,
    pub(in crate::features) connections: Vec<SavedConnection>,
    pub(in crate::features) pending_saved_connection_queue: VecDeque<PendingSavedConnectionStart>,
    pub(in crate::features) connection_groups: Vec<Group>,
    pub(in crate::features) connection_state: ConnectionFeatureState,
    /// Real text inputs for the panels that have not been given their own,
    /// keyed by an id the panel picks. See `features::text_inputs`.
    pub(in crate::features) text_inputs: TextInputRegistry,
    pub(in crate::features) connection_ssh_keys: Vec<SshKey>,
    pub(in crate::features) connection_otp_entries: Vec<OtpEntry>,
    pub(in crate::features) connection_saved_passwords: Vec<SavedPassword>,
    pub(in crate::features) connection_saved_credentials: Vec<SavedCredential>,
    pub(in crate::features) connection_serial_ports: Vec<String>,
    pub(in crate::features) tunnels: Vec<TunnelConfig>,
    pub(in crate::features) tunnel_groups: Vec<TunnelGroup>,
    pub(in crate::features) proxies: Vec<ProxyConfig>,
    pub(in crate::features) proxy_groups: Vec<ProxyGroup>,
    pub(in crate::features) quick_commands: Arc<[QuickCommand]>,
    pub(in crate::features) quick_command_categories: Vec<QuickCommandCategory>,
    pub(in crate::features) quick_command_state: QuickCommandFeatureState,
    pub(in crate::features) remote_ops: RemoteOpsFeatureState,
    pub(in crate::features) security: SecurityFeatureState,
    pub(in crate::features) settings_state: SettingsFeatureState,
    pub(in crate::features) ai: AiFeatureState,
    pub(in crate::features) terminal: TerminalFeatureState,
    pub(in crate::features) send_command: SendCommandFeatureState,
    pub(in crate::features) transfer: TransferFeatureState,
    pub(in crate::features) translation: TranslationFeatureState,
    pub(in crate::features) update: UpdateFeatureState,
    pub(in crate::features) cloud_sync: CloudSyncFeatureState,
    pub(in crate::features) command_history: Arc<[CommandHistoryEntry]>,
    pub(in crate::features) command_persistence_tx: mpsc::Sender<CommandPersistenceRequest>,
    pub(in crate::features) command_persistence_rx: mpsc::Receiver<CommandPersistenceResult>,
    pub(in crate::features) command_persistence_pending: usize,
    pub(in crate::features) session: SessionFeatureState,
    pub(in crate::features) bottom_panel: BottomPanelMode,
    pub(in crate::features) quick_cmd_height: f32,
    pub(in crate::features) serial_send_height: f32,
    pub(in crate::features) bottom_panel_resize: Option<BottomPanelResizeState>,
    pub(in crate::features) sync_groups: Vec<SyncInputGroup>,
    pub(in crate::features) sync_groups_open: bool,
    pub(in crate::features) sync_groups_focus: FocusHandle,
    pub(in crate::features) sync_groups_search_draft: String,
    pub(in crate::features) sync_groups_selected_id: Option<String>,
    pub(in crate::features) sync_groups_delete_pending: Option<String>,
    /// Broadcast keyboard input to every live session (Tauri broadcastToAll).
    pub(in crate::features) broadcast_to_all: bool,
    pub(in crate::features) keyword_highlights: KeywordHighlightConfig,
    pub(in crate::features) settings: AppSettingsSummary,
    pub(in crate::features) settings_master_password_enabled: bool,
    pub(in crate::features) settings_master_password_draft: String,
    pub(in crate::features) store_status: StoreStatus,
    pub(in crate::features) recording: RecordingFeatureState,
    pub(in crate::features) tunnel_runtime: TunnelFeatureState,
    pub(in crate::features) about_open: bool,
    pub(in crate::features) remote_editor_window: Option<WindowHandle<RemoteFileEditorWindow>>,
    pub(in crate::features) remote_editor_window_open_pending: bool,
    pub(in crate::features) config_path_prompt: Option<ConfigPathPromptKind>,
    pub(in crate::features) diagnostics_path_prompt: Option<DiagnosticsPathPromptKind>,
    pub(in crate::features) keyword_highlight_path_prompt: Option<KeywordHighlightPathPromptKind>,
    pub(in crate::features) active_snapshot_password_prompt: Option<SnapshotPasswordPromptState>,
    pub(in crate::features) lock_focus: FocusHandle,
    pub(in crate::features) lock_password_draft: String,
    pub(in crate::features) lock_status: String,
    pub(in crate::features) pending_session_events: VecDeque<SessionEvent>,
    pub(in crate::features) diagnostic_log_last_at: HashMap<&'static str, Instant>,
    pub(in crate::features) last_viewport_size: (f32, f32),
    /// Cached intrinsic dimensions for the current tiled wallpaper path.
    pub(in crate::features) wallpaper_tile_dimensions: Option<(String, u32, u32)>,
    /// When the window viewport last changed (resize/drag geometry).
    pub(in crate::features) last_viewport_change_at: Option<Instant>,
    /// Deadline for treating title-bar window dragging as active.
    pub(in crate::features) title_drag_active_until: Option<Instant>,
    pub(in crate::features) selected_nav: NavItem,
    pub(in crate::features) main_mode: MainMode,
    pub(in crate::features) settings_active_tab: SettingsTab,
    /// Expanded multi-item groups in the settings sidebar (Tauri keeps this local to the page).
    pub(in crate::features) settings_expanded_groups: HashSet<String>,
    /// Committed values captured when the in-window settings page opens.
    pub(in crate::features) settings_draft_snapshot: Option<SettingsDraftSnapshot>,
    pub(in crate::features) settings_window: Option<WindowHandle<SettingsWindow>>,
    pub(in crate::features) settings_window_open_pending: bool,
    /// Main workspace panel state to restore after leaving the in-window settings page.
    pub(in crate::features) settings_previous_left_collapsed: Option<bool>,
    pub(in crate::features) settings_previous_right_collapsed: Option<bool>,
    pub(in crate::features) active_left_panel: Option<NavItem>,
    pub(in crate::features) active_right_panel: Option<NavItem>,
    pub(in crate::features) left_open_panels: Vec<String>,
    pub(in crate::features) right_open_panels: Vec<String>,
    pub(in crate::features) panel_stack_sizes: HashMap<String, f32>,
    pub(in crate::features) panel_multi_open: bool,
    pub(in crate::features) right_focus: RightFocus,
    pub(in crate::features) left_sidebar_collapsed: bool,
    pub(in crate::features) right_inspector_collapsed: bool,
    pub(in crate::features) mobile_left_open: bool,
    pub(in crate::features) mobile_right_open: bool,
    pub(in crate::features) left_panel_width: f32,
    pub(in crate::features) right_panel_width: f32,
    pub(in crate::features) panel_resize: Option<PanelResizeState>,
    pub(in crate::features) panel_stack_resize: Option<PanelStackResizeState>,
    pub(in crate::features) activity_bar_layout: ActivityBarLayoutState,
    pub(in crate::features) activity_bar_context_menu: Option<ActivityBarContextMenuState>,
    pub(in crate::features) title_menu_open: Option<TitleMenu>,
    pub(in crate::features) title_menu_submenu: Option<TitleMenuSubmenu>,
    pub(in crate::features) header_status: HeaderStatusState,
    /// Open-tabs overflow menu (Tauri TabBar expand-more when many tabs).
    pub(in crate::features) open_tabs_menu_open: bool,
    /// New-session menu next to the tab strip + control.
    pub(in crate::features) new_session_menu_open: bool,
    /// Whether the Tauri-style "All sessions" submenu is expanded.
    pub(in crate::features) new_session_all_sessions_open: bool,
    /// Hovered group ids from the root submenu through the deepest open child menu.
    pub(in crate::features) new_session_group_menu_path: Vec<String>,
    /// Horizontal scroll handle for the global session tab strip (scroll-into-view).
    pub(in crate::features) session_tab_strip_scroll: ScrollHandle,
    /// Request scroll-into-view of the active tab on next paint (Tauri TabBar).
    pub(in crate::features) session_tab_scroll_into_view_pending: bool,
    /// Last failed connect name (shown as ephemeral failed tab chrome).
    pub(in crate::features) last_connect_failure_name: Option<String>,
    /// Last failed connect error text.
    pub(in crate::features) last_connect_failure_error: Option<String>,
    /// Legacy/global active pane tree view: mirrors the active tab's per-tab root when split.
    pub(in crate::features) workspace_split: Option<WorkspaceSplitState>,
    pub(in crate::features) workspace_split_resize: Option<WorkspaceSplitResizeState>,
    /// Per-tab pane trees keyed by tab-root session id (Tauri `Tab.root`).
    pub(in crate::features) session_pane_roots: HashMap<String, WorkspacePaneNode>,
    /// Leaf session id → owning tab-root session id (hidden from tab strip when secondary).
    pub(in crate::features) session_tab_owner: HashMap<String, String>,
    pub(in crate::features) focused_terminal_window_leaf_id: Option<String>,
    /// Whether we already attempted startup restore of global workspace pane splits.
    pub(in crate::features) workspace_pane_layout_restored: bool,
    pub(in crate::features) startup_restore_complete: bool,
    pub(in crate::features) is_locked: bool,
    pub(in crate::features) last_user_activity_at: Instant,
}

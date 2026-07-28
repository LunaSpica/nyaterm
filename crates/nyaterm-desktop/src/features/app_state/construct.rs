use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use crate::models::{
    ActivityBarLayoutState, BottomPanelMode, HeaderStatusState, MainMode, NavItem, PanelSide,
    RightFocus, SessionEventBridge, SettingsTab, StartupCommandAction, StoreStatus,
    TerminalFramePipeline,
};
use crate::terminal::initial_terminal_screen;
use gpui::{Context, ScrollHandle};
use nyaterm_core::{
    AiSettings, AppRuntime, AppSettingsSummary, CLOUD_SYNC_HISTORY_LIMIT, CloudSyncSettings,
    CloudSyncState, ConnectionStore, KeywordHighlightConfig, NativeServices, TranslationSettings,
    read_cloud_sync_history, uuid,
};
use nyaterm_terminal::TerminalOutputDecoder;
use nyaterm_transport::{SessionManager, SftpDuplicatePolicy};

use super::super::settings::{SettingsFeatureFocus, SettingsFeatureState};
use super::super::{
    AiFeatureFocus, AiFeatureState, CloudSyncFeatureState, ConnectionFeatureFocus,
    ConnectionFeatureState, CredentialPromptBroker, DEFAULT_DUPLICATE_STARTUP_DELAY_MS,
    HostKeyPromptBroker, INITIAL_TERMINAL_BANNER, LEGACY_ROOT, NativeOtpProvider,
    QuickCommandFeatureFocus, QuickCommandFeatureState, RecordingFeatureState,
    RemoteOpsFeatureFocus, RemoteOpsFeatureState, SecurityFeatureFocus, SecurityFeatureState,
    SendCommandFeatureFocus, SendCommandFeatureState, SessionFeatureState,
    SftpDuplicatePromptBroker, TerminalFeatureFocus, TerminalFeatureState, TextInputRegistry,
    TransferFeatureFocus, TransferFeatureState, TranslationFeatureState, TunnelFeatureState,
    UpdateFeatureState, ai_active_profile_drafts, ai_usage_counts, appearance_font_options,
    quick_command_sort_mode_from_setting, quick_command_view_mode_from_setting,
    spawn_command_persistence_worker,
};
use super::NyaTermApp;
use crate::models::panel_collapsed_from_persistence;
#[cfg(feature = "migration-dashboard")]
use nyaterm_legacy::LegacyProject;
#[cfg(not(feature = "migration-dashboard"))]
use nyaterm_legacy::MigrationInventory;

impl NyaTermApp {
    pub fn new(
        runtime: AppRuntime,
        stores: crate::entities::UiStoreHandles,
        cx: &mut Context<Self>,
    ) -> Self {
        nyaterm_core::warm_terminal_input_tracker();
        #[cfg(feature = "migration-dashboard")]
        let inventory = {
            let legacy = LegacyProject::new(LEGACY_ROOT);
            nyaterm_legacy::inventory(&legacy)
        };
        #[cfg(not(feature = "migration-dashboard"))]
        let inventory = MigrationInventory {
            legacy_root: std::path::PathBuf::from(LEGACY_ROOT),
            exists: false,
            rust_files: 0,
            frontend_files: 0,
            command_modules: 0,
            copied_vendor_roots: Vec::new(),
        };
        let (command_persistence_tx, command_persistence_rx) = spawn_command_persistence_worker(
            runtime.config_dir().to_path_buf(),
            runtime.portable_key_path().map(ToOwned::to_owned),
        );
        let (
            connections,
            connection_groups,
            connection_ssh_keys,
            connection_otp_entries,
            connection_saved_passwords,
            connection_saved_credentials,
            tunnels,
            tunnel_groups,
            proxies,
            proxy_groups,
            quick_commands,
            quick_command_categories,
            command_history,
            keyword_highlights,
            settings,
            store_status,
            cloud_sync_settings,
            cloud_sync_state,
            translation_settings,
            ai_settings,
            ai_session_count,
            ai_message_count,
            ai_audit_count,
        ) = match ConnectionStore::open_with_portable_key_path(
            runtime.config_dir(),
            runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            Ok(store) => {
                let path = store.db_path().display().to_string();
                match store.load_sessions() {
                    Ok(config) => {
                        let settings = store.load_app_settings_summary().unwrap_or_default();
                        let connection_groups = config.groups.clone();
                        let connection_ssh_keys = store.list_ssh_keys().unwrap_or_default();
                        let connection_otp_entries = store.list_otp_entries().unwrap_or_default();
                        let connection_saved_passwords = store.list_passwords().unwrap_or_default();
                        let connection_saved_credentials =
                            store.list_credentials().unwrap_or_default();
                        let tunnels = store.list_tunnels().unwrap_or_default();
                        let tunnel_groups = store.list_tunnel_groups().unwrap_or_default();
                        let proxies = store.list_proxies().unwrap_or_default();
                        let proxy_groups = store.list_proxy_groups().unwrap_or_default();
                        let cloud_sync_settings =
                            store.load_cloud_sync_settings().unwrap_or_default();
                        let cloud_sync_state = store.load_cloud_sync_state().unwrap_or_default();
                        let translation_settings = store
                            .load_translation_settings()
                            .unwrap_or_else(|_| TranslationSettings {
                                target_language: settings.language.clone(),
                                ..TranslationSettings::default()
                            });
                        let quick_commands = store.load_quick_commands().unwrap_or_default();
                        let command_history = store.list_command_history(64).unwrap_or_default();
                        let keyword_highlights =
                            store.load_keyword_highlights().unwrap_or_default();
                        let ai_settings = store.load_ai_settings().unwrap_or_default();
                        let (ai_session_count, ai_message_count, ai_audit_count) =
                            ai_usage_counts(&store);
                        (
                            config.connections,
                            connection_groups,
                            connection_ssh_keys,
                            connection_otp_entries,
                            connection_saved_passwords,
                            connection_saved_credentials,
                            tunnels,
                            tunnel_groups,
                            proxies,
                            proxy_groups,
                            quick_commands.commands,
                            quick_commands.categories,
                            command_history,
                            keyword_highlights,
                            settings,
                            StoreStatus {
                                path,
                                message: "redb connection store online".to_string(),
                                ready: true,
                            },
                            cloud_sync_settings,
                            cloud_sync_state,
                            translation_settings,
                            ai_settings,
                            ai_session_count,
                            ai_message_count,
                            ai_audit_count,
                        )
                    }
                    Err(error) => (
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        Vec::new(),
                        KeywordHighlightConfig::default(),
                        AppSettingsSummary::default(),
                        StoreStatus {
                            path,
                            message: format!("failed to load sessions: {error}"),
                            ready: false,
                        },
                        CloudSyncSettings::default(),
                        CloudSyncState::default(),
                        TranslationSettings::default(),
                        AiSettings::default(),
                        0,
                        0,
                        0,
                    ),
                }
            }
            Err(error) => (
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                KeywordHighlightConfig::default(),
                AppSettingsSummary::default(),
                StoreStatus {
                    path: runtime
                        .config_dir()
                        .join("nyaterm.redb")
                        .display()
                        .to_string(),
                    message: format!("failed to open store: {error}"),
                    ready: false,
                },
                CloudSyncSettings::default(),
                CloudSyncState::default(),
                TranslationSettings::default(),
                AiSettings::default(),
                0,
                0,
                0,
            ),
        };
        let (appearance_ui_font_options, appearance_terminal_font_options) =
            appearance_font_options(cx);
        let otp_provider = Arc::new(NativeOtpProvider::new(
            runtime.config_dir().to_path_buf(),
            runtime.portable_key_path().map(ToOwned::to_owned),
        ));
        let transfer_duplicate_policy =
            SftpDuplicatePolicy::from_legacy_value(&settings.transfer_duplicate_strategy);
        let recording = RecordingFeatureState::new(settings.recording_memory_limit_bytes as usize);
        let recording_writer = recording.writer();
        let cloud_sync_history = read_cloud_sync_history(
            runtime.log_dir(),
            settings.diagnostics_retention_days,
            CLOUD_SYNC_HISTORY_LIMIT,
        )
        .unwrap_or_default();
        let (ai_model_draft, ai_base_url_draft) = ai_active_profile_drafts(&ai_settings);
        let left_panel_width = settings.ui_left_panel_width as f32;
        let right_panel_width = settings.ui_right_panel_width as f32;
        let transfer_panel_height = settings.ui_transfer_height as f32;
        let quick_cmd_height = settings.ui_quick_cmd_height as f32;
        let serial_send_height = settings.ui_serial_send_height as f32;
        let activity_bar_layout = ActivityBarLayoutState {
            left_top: settings.ui_activity_bar_left_top.clone(),
            left_bottom: settings.ui_activity_bar_left_bottom.clone(),
            right_top: settings.ui_activity_bar_right_top.clone(),
            right_bottom: settings.ui_activity_bar_right_bottom.clone(),
            show_labels: settings.ui_activity_bar_show_labels,
        };
        let active_left_panel = settings
            .ui_active_left_panel
            .as_deref()
            .and_then(NavItem::from_persistence_id)
            .filter(|item| {
                activity_bar_layout.side_for_entry(item.persistence_id()) == Some(PanelSide::Left)
            });
        let active_right_panel = settings
            .ui_active_right_panel
            .as_deref()
            .and_then(NavItem::from_persistence_id)
            .filter(|item| {
                activity_bar_layout.side_for_entry(item.persistence_id()) == Some(PanelSide::Right)
            });
        let left_sidebar_collapsed = panel_collapsed_from_persistence(
            settings.ui_left_panel_collapsed,
            settings.ui_panel_multi_open,
            active_left_panel.is_some(),
            !settings.ui_left_open_panels.is_empty(),
        );
        let right_inspector_collapsed = panel_collapsed_from_persistence(
            settings.ui_right_panel_collapsed,
            settings.ui_panel_multi_open,
            active_right_panel.is_some(),
            !settings.ui_right_open_panels.is_empty(),
        );
        let security_secrets_unlocked = !settings.has_master_password;
        let left_open_panels = settings.ui_left_open_panels.clone();
        let right_open_panels = settings.ui_right_open_panels.clone();
        let panel_stack_sizes = settings
            .ui_panel_stack_sizes
            .iter()
            .filter_map(|(key, value)| (*value > 0).then(|| (key.clone(), (*value as f32) / 1000.)))
            .collect::<HashMap<_, _>>();
        let panel_multi_open = settings.ui_panel_multi_open;
        let settings_master_password_enabled = settings.has_master_password;
        let mut terminal_output_decoder = TerminalOutputDecoder::default();
        terminal_output_decoder.set_encoding(&settings.interaction_default_encoding);
        let mut terminal_screen = initial_terminal_screen();
        terminal_screen.set_encoding(&settings.interaction_default_encoding);
        let session_manager = Arc::new(SessionManager::new());
        let terminal_frame_pipeline = TerminalFramePipeline::spawn(recording_writer);
        let session_event_bridge = SessionEventBridge::spawn(
            Arc::clone(&session_manager),
            terminal_frame_pipeline.clone(),
            settings.interaction_default_encoding.clone(),
            settings.terminal_scrollback_lines.clamp(100, 100_000) as usize,
        );

        let connections_filter_placeholder =
            crate::i18n::text(&settings.language, "savedConnections.filter");

        Self {
            stores,
            runtime,
            services: NativeServices::new(),
            inventory,
            connections,
            pending_saved_connection_queue: VecDeque::new(),
            connection_state: ConnectionFeatureState::new(
                &settings,
                ConnectionFeatureFocus {
                    filter_placeholder: connections_filter_placeholder.into(),
                    import: cx.focus_handle(),
                    editor: cx.focus_handle(),
                    group_editor: cx.focus_handle(),
                    group_open_confirm: cx.focus_handle(),
                    network_tunnel_editor: cx.focus_handle(),
                    network_proxy_editor: cx.focus_handle(),
                },
                cx,
            ),
            connection_groups,
            connection_ssh_keys,
            connection_otp_entries,
            connection_saved_passwords,
            connection_saved_credentials,
            connection_serial_ports: Vec::new(),
            tunnels,
            tunnel_groups,
            proxies,
            proxy_groups,
            quick_commands: Arc::from(quick_commands),
            quick_command_categories,
            quick_command_state: QuickCommandFeatureState::new(
                quick_command_sort_mode_from_setting(&settings.ui_quick_cmd_sort_mode),
                quick_command_view_mode_from_setting(&settings.ui_quick_cmd_view_mode),
                QuickCommandFeatureFocus {
                    editor: cx.focus_handle(),
                    details: cx.focus_handle(),
                    category_rename: cx.focus_handle(),
                    variable: cx.focus_handle(),
                    import: cx.focus_handle(),
                },
            ),
            send_command: SendCommandFeatureState::new(SendCommandFeatureFocus {
                editor: cx.focus_handle(),
            }),
            terminal: TerminalFeatureState::new(
                terminal_screen,
                terminal_output_decoder,
                terminal_frame_pipeline,
                String::from(INITIAL_TERMINAL_BANNER),
                "idle".to_string(),
                1.0,
                TerminalFeatureFocus {
                    actions: cx.focus_handle(),
                    x11_display: cx.focus_handle(),
                    terminal: cx.focus_handle(),
                },
            ),
            ai: AiFeatureState::new(
                ai_settings,
                ai_model_draft,
                ai_base_url_draft,
                format!("ai-session-{}", uuid()),
                ai_session_count,
                ai_message_count,
                ai_audit_count,
                AiFeatureFocus {
                    chat: cx.focus_handle(),
                    clear_history_confirm: cx.focus_handle(),
                    auto_execution_confirm: cx.focus_handle(),
                    action: cx.focus_handle(),
                    manual_model: cx.focus_handle(),
                    credential: cx.focus_handle(),
                },
            ),
            transfer: TransferFeatureState::new(
                ".".to_string(),
                "nyaterm-download.bin".to_string(),
                transfer_duplicate_policy,
                transfer_panel_height,
                TransferFeatureFocus {
                    panel: cx.focus_handle(),
                    queue: cx.focus_handle(),
                    job_delete: cx.focus_handle(),
                    download_path: cx.focus_handle(),
                    browser: cx.focus_handle(),
                    rename: cx.focus_handle(),
                    move_to: cx.focus_handle(),
                    delete: cx.focus_handle(),
                    new_folder: cx.focus_handle(),
                    new_file: cx.focus_handle(),
                    new_symlink: cx.focus_handle(),
                    properties: cx.focus_handle(),
                    unknown_file: cx.focus_handle(),
                    editor: cx.focus_handle(),
                    default_editor: cx.focus_handle(),
                    external_sync: cx.focus_handle(),
                },
            ),
            security: SecurityFeatureState::new(
                security_secrets_unlocked,
                "security ready".to_string(),
                SecurityFeatureFocus {
                    key_editor: cx.focus_handle(),
                    otp_editor: cx.focus_handle(),
                    password_editor: cx.focus_handle(),
                    credential_editor: cx.focus_handle(),
                    unlock: cx.focus_handle(),
                },
            ),
            settings_state: SettingsFeatureState::new(
                appearance_ui_font_options,
                appearance_terminal_font_options,
                SettingsFeatureFocus {
                    search_engine: cx.focus_handle(),
                    keyword_highlight: cx.focus_handle(),
                    keybindings: cx.focus_handle(),
                },
            ),
            remote_ops: RemoteOpsFeatureState::new(RemoteOpsFeatureFocus {}),
            translation: TranslationFeatureState::new(translation_settings),
            update: UpdateFeatureState::new(),
            cloud_sync: CloudSyncFeatureState::new(
                cloud_sync_settings,
                cloud_sync_state,
                cloud_sync_history,
            ),
            command_history: Arc::from(command_history),
            command_persistence_tx,
            command_persistence_rx,
            command_persistence_pending: 0,
            session: SessionFeatureState::new(session_manager, session_event_bridge),
            text_inputs: TextInputRegistry::default(),
            action_link_menu: None,
            action_link_tooltip: None,
            action_link_hover_pending: None,
            bottom_panel: if settings.ui_serial_send_visible {
                BottomPanelMode::CommandSend
            } else if settings.ui_quick_cmd_visible {
                BottomPanelMode::QuickCommands
            } else {
                BottomPanelMode::Hidden
            },
            quick_cmd_height,
            serial_send_height,
            bottom_panel_resize: None,
            sync_groups: Vec::new(),
            sync_groups_open: false,
            sync_groups_focus: cx.focus_handle(),
            sync_groups_search_draft: String::new(),
            sync_groups_selected_id: None,
            sync_groups_delete_pending: None,
            broadcast_to_all: false,
            keyword_highlights,
            settings,
            settings_master_password_enabled,
            settings_master_password_draft: String::new(),
            store_status,
            recording,
            tunnel_runtime: TunnelFeatureState::new(),
            about_open: false,
            remote_editor_window: None,
            remote_editor_window_open_pending: false,
            config_path_prompt: None,
            diagnostics_path_prompt: None,
            keyword_highlight_path_prompt: None,
            active_snapshot_password_prompt: None,
            duplicate_prompts: Arc::new(SftpDuplicatePromptBroker::default()),
            active_duplicate_prompt: None,
            host_key_prompts: Arc::new(HostKeyPromptBroker::default()),
            active_host_key_prompt: None,
            credential_prompts: Arc::new(CredentialPromptBroker::default()),
            active_credential_prompt: None,
            active_keyboard_interactive_prompt: None,
            credential_prompt_focus_pending: false,
            credential_focus: cx.focus_handle(),
            otp_provider,
            tab_actions_session_id: None,
            tab_actions_anchor: None,
            tab_actions_submenu: None,
            tab_actions_focus: cx.focus_handle(),
            close_all_sessions_confirm_open: false,
            pending_quit_after_close_all: false,
            pending_window_quit: false,
            close_all_sessions_confirm_focus: cx.focus_handle(),
            rename_session_id: None,
            rename_draft: String::new(),
            rename_focus: cx.focus_handle(),
            color_picker_open: false,
            color_picker_focus: cx.focus_handle(),
            session_info_open: false,
            session_info_focus: cx.focus_handle(),
            startup_command_open: false,
            startup_command_action: StartupCommandAction::Duplicate,
            startup_command_draft: String::new(),
            startup_command_delay_ms: DEFAULT_DUPLICATE_STARTUP_DELAY_MS,
            startup_command_focus: cx.focus_handle(),
            temporary_ssh_link_open: false,
            temporary_ssh_link_draft: String::new(),
            temporary_ssh_link_error: None,
            temporary_ssh_link_focus: cx.focus_handle(),
            multi_line_paste: None,
            multi_line_paste_marked_text: String::new(),
            multi_line_paste_marked_range: None,
            multi_line_paste_cursor: 0,
            multi_line_paste_anchor: None,
            multi_line_paste_focus: cx.focus_handle(),
            lock_focus: cx.focus_handle(),
            lock_password_draft: String::new(),
            lock_status: String::new(),
            pending_terminal_frame_events: VecDeque::new(),
            pending_session_events: VecDeque::new(),
            diagnostic_log_last_at: HashMap::new(),
            cached_terminal_theme_palette: None,
            cached_keyword_highlight_rules: None,
            last_viewport_size: (1280., 800.),
            wallpaper_tile_dimensions: None,
            last_viewport_change_at: None,
            title_drag_active_until: None,
            selected_nav: NavItem::Workspace,
            main_mode: MainMode::Workspace,
            settings_active_tab: SettingsTab::General,
            settings_expanded_groups: HashSet::from(["workspace".to_string()]),
            settings_draft_snapshot: None,
            settings_window: None,
            settings_window_open_pending: false,
            settings_previous_left_collapsed: None,
            settings_previous_right_collapsed: None,
            active_left_panel,
            active_right_panel,
            left_open_panels,
            right_open_panels,
            panel_stack_sizes,
            panel_multi_open,
            right_focus: RightFocus::Default,
            left_sidebar_collapsed,
            right_inspector_collapsed,
            mobile_left_open: false,
            mobile_right_open: false,
            left_panel_width,
            right_panel_width,
            panel_resize: None,
            panel_stack_resize: None,
            activity_bar_layout,
            activity_bar_context_menu: None,
            title_menu_open: None,
            title_menu_submenu: None,
            header_status: HeaderStatusState::default(),
            open_tabs_menu_open: false,
            new_session_menu_open: false,
            new_session_all_sessions_open: false,
            new_session_group_menu_path: Vec::new(),
            session_tab_strip_scroll: ScrollHandle::new(),
            session_tab_scroll_into_view_pending: false,
            last_connect_failure_name: None,
            last_connect_failure_error: None,
            workspace_split: None,
            workspace_split_resize: None,
            session_pane_roots: HashMap::new(),
            session_tab_owner: HashMap::new(),
            focused_terminal_window_leaf_id: None,
            workspace_pane_layout_restored: false,
            startup_restore_complete: false,
            is_locked: false,
            last_user_activity_at: Instant::now(),
        }
    }
}

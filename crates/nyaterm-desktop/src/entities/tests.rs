use super::{
    AiSnapshot, AiStore, CloudSyncSnapshot, CloudSyncStore, ConnectionsSnapshot, ConnectionsStore,
    OverlaySnapshot, OverlayStore, RemoteOpsSnapshot, RemoteOpsStore, SessionSnapshot,
    SessionStore, SettingsSnapshot, SettingsStore, StartupRestoreStore, TransferSnapshot,
    TransferStore, WindowRuntimeStore, WorkspaceSnapshot, WorkspaceStore,
};

#[test]
fn workspace_store_tracks_active_session_and_tab_order() {
    let mut store = WorkspaceStore::default();
    store.activate_session("session-a");
    store.set_ordered_tab_roots(vec!["session-a".into(), "session-b".into()]);

    assert_eq!(store.active_session_id(), Some("session-a"));
    assert_eq!(store.ordered_tab_roots(), ["session-a", "session-b"]);
}

#[test]
fn session_store_tracks_live_sessions() {
    let mut store = SessionStore::default();
    store.mark_live("session-a");
    store.mark_live("session-b");
    store.mark_closed("session-a");

    assert!(!store.is_live("session-a"));
    assert!(store.is_live("session-b"));
    assert_eq!(store.live_session_count(), 1);
}

#[test]
fn window_runtime_store_starts_pump_once() {
    let mut store = WindowRuntimeStore::default();

    assert!(store.mark_started());
    assert!(!store.mark_started());
    assert!(store.pump_started());
}

#[test]
fn startup_restore_store_starts_after_window_open_once() {
    let mut store = StartupRestoreStore::default();

    assert!(store.mark_started_after_window_open());
    assert!(!store.mark_started_after_window_open());
    assert!(store.started_after_window_open());
}

#[test]
fn startup_restore_store_tracks_queue_and_completion() {
    let mut store = StartupRestoreStore::default();
    store.set_queue(vec![
        nyaterm_core::RestorableOpenTab::with_leaf_root("one", "Local", None, None, None),
        nyaterm_core::RestorableOpenTab::with_leaf_root("two", "Local", None, None, None),
    ]);

    assert_eq!(store.queue_len(), 2);
    assert!(store.can_pump_queue(false));
    assert!(!store.can_pump_queue(true));
    assert_eq!(
        store.pop_next_tab().as_ref().map(|tab| tab.title.as_str()),
        Some("one")
    );
    assert_eq!(store.queue_len(), 1);
    assert!(store.mark_complete());
    assert!(!store.can_pump_queue(false));
}

#[test]
fn workspace_store_notifies_only_for_changed_snapshots() {
    let mut store = WorkspaceStore::default();
    let snapshot = WorkspaceSnapshot {
        active_session_id: Some("session-a".into()),
        ordered_tab_roots: vec!["session-a".into()],
        selected_nav: "Workspace".into(),
        main_mode: "Workspace".into(),
        active_left_panel: Some("Saved Connections".into()),
        active_right_panel: None,
        left_sidebar_collapsed: false,
        right_inspector_collapsed: false,
        workspace_split_active: false,
        terminal_windows_active: false,
    };

    assert!(store.replace_snapshot(snapshot.clone()));
    assert!(!store.replace_snapshot(snapshot));
    assert_eq!(store.active_session_id(), Some("session-a"));
    assert_eq!(store.ordered_tab_roots(), ["session-a"]);
}

#[test]
fn workspace_store_mutates_navigation_and_left_panel() {
    let mut store = WorkspaceStore::default();

    assert!(store.select_nav("Settings", "Page"));
    assert!(!store.select_nav("Settings", "Page"));
    assert!(store.open_left_panel("File Explorer"));
    assert!(!store.open_left_panel("File Explorer"));
    assert!(store.close_left_panel());
    assert!(!store.close_left_panel());

    let snapshot = store.snapshot().expect("workspace snapshot");
    assert_eq!(snapshot.selected_nav, "Settings");
    assert_eq!(snapshot.main_mode, "Page");
    assert!(snapshot.left_sidebar_collapsed);
    assert_eq!(snapshot.active_left_panel, None);
}

#[test]
fn session_store_notifies_only_for_changed_snapshots() {
    let mut store = SessionStore::default();
    let snapshot = SessionSnapshot {
        active_session_id: Some("session-a".into()),
        ordered_session_ids: vec!["session-a".into(), "session-b".into()],
        live_session_ids: vec!["session-b".into()],
        metadata_count: 1,
        terminal_view_count: 1,
        pending_start_count: 0,
        host_prompt_active: false,
        credential_prompt_active: false,
        zmodem_session_count: 0,
    };

    assert!(store.replace_snapshot(snapshot.clone()));
    assert!(!store.replace_snapshot(snapshot));
    assert!(!store.is_live("session-a"));
    assert!(store.is_live("session-b"));
}

#[test]
fn session_store_mutates_active_order_and_removal() {
    let mut store = SessionStore::default();

    assert!(store.set_ordered_session_ids(vec![
        "session-a".into(),
        "session-b".into(),
        "session-c".into(),
    ]));
    assert!(store.activate("session-b"));
    assert!(!store.activate("session-b"));
    assert!(store.move_session_to_index("session-c", 0));
    assert_eq!(
        store.ordered_session_ids(),
        ["session-c", "session-a", "session-b"]
    );
    assert!(store.remove_session("session-b"));
    assert_eq!(store.active_session_id(), None);
    assert_eq!(store.ordered_session_ids(), ["session-c", "session-a"]);
}

#[test]
fn overlay_store_notifies_only_for_changed_snapshots() {
    let mut store = OverlayStore::default();
    let snapshot = OverlaySnapshot {
        quick_switch_open: true,
        tab_actions_open: false,
        rename_open: false,
        color_picker_open: false,
        session_info_open: false,
        startup_command_open: false,
        temporary_ssh_link_open: false,
        multi_line_paste_open: false,
        terminal_actions_open: false,
        terminal_context_menu_open: false,
        action_link_menu_open: false,
        action_link_tooltip_open: false,
        command_suggestions_open: false,
        credential_suggestions_open: false,
        close_all_sessions_confirm_open: false,
        locked: false,
    };

    assert!(store.replace_snapshot(snapshot.clone()));
    assert!(!store.replace_snapshot(snapshot));
    assert!(
        store
            .snapshot()
            .is_some_and(|snapshot| snapshot.quick_switch_open)
    );
}

#[test]
fn overlay_store_mutates_open_close_and_menu_exclusion() {
    let mut store = OverlayStore::default();

    assert!(store.set_quick_switch_open(true));
    assert!(!store.set_quick_switch_open(true));
    assert!(store.set_terminal_context_menu_open(true));
    assert!(store.set_action_link_menu_open(true));
    let snapshot = store.snapshot().expect("overlay snapshot");
    assert!(snapshot.action_link_menu_open);
    assert!(!snapshot.terminal_context_menu_open);
    assert!(store.set_locked(true));
    assert!(!store.set_locked(true));
}

#[test]
fn settings_store_notifies_only_for_changed_snapshots() {
    let mut store = SettingsStore::default();
    let snapshot = SettingsSnapshot {
        active_tab: "Security".into(),
        has_master_password: true,
        security_unlocked: false,
        cloud_sync_enabled: true,
        startup_restore: true,
    };
    assert!(store.replace_snapshot(snapshot.clone()));
    assert!(!store.replace_snapshot(snapshot));
}

#[test]
fn connections_store_notifies_only_for_changed_snapshots() {
    let mut store = ConnectionsStore::default();
    let snapshot = ConnectionsSnapshot {
        connection_count: 3,
        group_count: 1,
        search_active: true,
        editor_open: false,
        group_editor_open: false,
        delete_confirm_open: false,
        sort_mode: "Name".into(),
    };
    assert!(store.replace_snapshot(snapshot.clone()));
    assert!(!store.replace_snapshot(snapshot));
}

#[test]
fn transfer_ai_sync_remote_stores_replace_snapshots() {
    assert!(TransferStore::default().replace_snapshot(TransferSnapshot {
        job_count: 1,
        active_job_count: 1,
        browser_path: "/tmp".into(),
        selected_count: 0,
        browser_busy: true,
    }));
    assert!(AiStore::default().replace_snapshot(AiSnapshot {
        chat_pending: true,
        message_count: 2,
        session_id: "s".into(),
        agent_active: false,
    }));
    assert!(
        CloudSyncStore::default().replace_snapshot(CloudSyncSnapshot {
            enabled: true,
            provider: "local".into(),
            conflict_active: false,
            last_status: "ok".into(),
        })
    );
    assert!(
        RemoteOpsStore::default().replace_snapshot(RemoteOpsSnapshot {
            process_count: 4,
            docker_tab: "Containers".into(),
            stats_ready: true,
            confirm_open: false,
        })
    );
}

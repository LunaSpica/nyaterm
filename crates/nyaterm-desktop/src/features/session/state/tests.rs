use std::sync::{Arc, mpsc};
use std::time::Instant;

use gpui::TestAppContext;
use nyaterm_core::{AiExecutionProfile, ConnectionType, SavedConnection, uuid};
use nyaterm_store::{StoreConfig, StoreRuntime};
use nyaterm_transport::{
    LocalSessionConfig, SessionEvent, SessionKind, SessionManager, SshCredentialPrompt,
    SshCredentialPromptKind, SshCredentialPromptReason, SshHostKey, SshKeyboardInteractivePrompt,
    SshKeyboardInteractiveRequest, SshSessionConfig,
};

use crate::features::runtime_jobs::SessionStartResult;
use crate::features::session::HostKeyPromptIssue;
use crate::models::{
    SessionEventBridge, SessionLaunchConfig, SessionRuntimeMetadata, StartupCommandAction,
    TabActionsSubmenu, TerminalFramePipeline, WorkspaceSplitDirection,
};
use crate::temporary_ssh_link::TemporaryLinkProtocol;

use super::{
    AgentPromptBroker, CredentialPromptBroker, CredentialPromptState, FailedSessionStart,
    HostKeyPromptBroker, HostKeyPromptRequest, KeyboardInteractivePromptState,
    NativeOtpCodePreview, NativeOtpProvider, PendingSessionStart, PromptResolution,
    RenameSessionSubmission, SavedConnectionStartOptions, SessionFeatureFocus, SessionFeatureState,
    SessionPromptState, SessionStartEventRequest, SessionStartFeatureState,
    SftpDuplicatePromptBroker,
};

fn pending(name: &str) -> PendingSessionStart {
    PendingSessionStart {
        connection_name: name.to_string(),
        launch_config: None,
        requested_at: Instant::now(),
        kind: SessionKind::LocalPty,
        ai_execution_profile: AiExecutionProfile::default(),
        custom_name: None,
        tab_color: None,
        locked: false,
        after_session_id: None,
        insert_index: None,
        seed_output: None,
        startup_command: None,
        multiplex_key: None,
        source_connection_id: None,
        reconnect_session_id: None,
    }
}

fn saved_connection(id: &str) -> SavedConnection {
    SavedConnection {
        id: id.to_string(),
        name: id.to_string(),
        config: ConnectionType::LocalTerminal {
            shell_path: String::new(),
            shell_args: String::new(),
            working_dir: None,
            ai_execution_profile: AiExecutionProfile::Auto,
            encoding: String::new(),
        },
        group_id: None,
        description: None,
        sort_order: 0,
        icon: None,
        icon_auto_detect: None,
        auth: None,
        recording: None,
        ssh_algorithms: None,
        ssh_profile: Default::default(),
        terminal_type: None,
        sftp: Default::default(),
        network: None,
        post_login: None,
        created_at_ms: None,
        updated_at_ms: None,
        last_used_at_ms: None,
    }
}

fn test_otp_provider() -> Arc<NativeOtpProvider> {
    let config_dir = std::env::temp_dir().join(format!(
        "nyaterm-session-state-test-{}-{}",
        std::process::id(),
        uuid()
    ));
    let store = StoreRuntime::spawn(StoreConfig {
        config_dir,
        portable_key_path: None,
    })
    .expect("spawn test store")
    .blocking_client();
    Arc::new(NativeOtpProvider::new(store))
}

fn prompt_state(cx: &TestAppContext) -> SessionPromptState {
    SessionPromptState {
        duplicate_prompts: Arc::new(SftpDuplicatePromptBroker::default()),
        active_duplicate_prompt: None,
        host_key_prompts: Arc::new(HostKeyPromptBroker::default()),
        active_host_key_prompt: None,
        credential_prompts: Arc::new(CredentialPromptBroker::default()),
        active_credential_prompt: None,
        active_keyboard_interactive_prompt: None,
        agent_prompts: Arc::new(AgentPromptBroker::default()),
        active_agent_prompt: None,
        credential_prompt_focus_pending: false,
        credential_focus: cx.update(|cx| cx.focus_handle()),
        otp_provider: test_otp_provider(),
    }
}

fn session_state(cx: &TestAppContext) -> SessionFeatureState {
    let manager = Arc::new(SessionManager::new());
    let event_bridge = SessionEventBridge::spawn(
        Arc::clone(&manager),
        TerminalFramePipeline::default(),
        "utf-8".to_string(),
        10_000,
    );
    let focus = || cx.update(|cx| cx.focus_handle());
    SessionFeatureState::new(
        manager,
        event_bridge,
        test_otp_provider(),
        SessionFeatureFocus {
            credential: focus(),
            tab_actions: focus(),
            color_picker: focus(),
            info: focus(),
        },
    )
}

fn session_metadata(name: &str, multiplex_key: Option<&str>) -> SessionRuntimeMetadata {
    let config = LocalSessionConfig {
        name: name.to_string(),
        ..LocalSessionConfig::default()
    };
    SessionRuntimeMetadata {
        ssh_config: None,
        ssh_multiplex_key: multiplex_key.map(ToOwned::to_owned),
        source_connection_id: None,
        ai_execution_profile: AiExecutionProfile::Posix,
        launch_config: SessionLaunchConfig::Local(config),
        disconnected: false,
    }
}

fn credential_prompt_state(id: &str) -> CredentialPromptState {
    let (response_tx, _response_rx) = mpsc::channel();
    CredentialPromptState {
        id: id.to_string(),
        prompt: SshCredentialPrompt {
            host: "example.test".to_string(),
            port: 22,
            username: "nya".to_string(),
            connection_name: "example".to_string(),
            kind: SshCredentialPromptKind::Password,
            reason: SshCredentialPromptReason::MissingPassword,
            attempt: 1,
            prompt_text: None,
            echo: false,
        },
        response_tx,
        value: String::new(),
    }
}

#[test]
fn credential_prompt_owner_isolates_input_and_clears_focus_on_take() {
    let cx = TestAppContext::single();
    let mut prompts = prompt_state(&cx);

    prompts.activate_credential(credential_prompt_state("credential-1"));
    assert!(prompts.credential_focus_is_pending());
    assert!(!prompts.apply_credential_input("credential-2", "wrong".to_string()));
    assert_eq!(
        prompts
            .active_credential()
            .expect("credential prompt should remain active")
            .value,
        ""
    );

    assert!(prompts.apply_credential_input("credential-1", "secret".to_string()));
    let prompt = prompts
        .take_credential()
        .expect("matching credential prompt should be taken");
    assert_eq!(prompt.id, "credential-1");
    assert_eq!(prompt.value, "secret");
    assert!(!prompts.credential_focus_is_pending());
    assert!(prompts.active_credential().is_none());
}

#[test]
fn mismatched_host_key_resolution_preserves_active_prompt() {
    let cx = TestAppContext::single();
    let mut prompts = prompt_state(&cx);
    let (response_tx, _response_rx) = mpsc::channel();
    prompts.active_host_key_prompt = Some(HostKeyPromptRequest {
        id: "host-key-1".to_string(),
        host_key: SshHostKey {
            host: "example.test".to_string(),
            port: 22,
            host_identifier: "example.test".to_string(),
            key_type: "ssh-ed25519".to_string(),
            key_base64: "test-key".to_string(),
            fingerprint: "SHA256:test".to_string(),
        },
        issue: HostKeyPromptIssue::Unknown,
        response_tx,
    });

    assert!(matches!(
        prompts.take_host_key_resolution("host-key-2"),
        PromptResolution::Changed
    ));
    assert_eq!(
        prompts
            .active_host_key()
            .expect("mismatched resolution must restore the prompt")
            .id,
        "host-key-1"
    );
}

#[test]
fn otp_missing_entry_preserves_manual_timing_but_clears_refresh_timing() {
    let cx = TestAppContext::single();
    let mut prompts = prompt_state(&cx);
    let (response_tx, _response_rx) = mpsc::channel();
    prompts.activate_keyboard_interactive(KeyboardInteractivePromptState {
        id: "keyboard-1".to_string(),
        request: SshKeyboardInteractiveRequest {
            host: "example.test".to_string(),
            port: 22,
            username: "nya".to_string(),
            connection_name: "example".to_string(),
            name: "verification".to_string(),
            instructions: String::new(),
            round: 1,
            prompts: vec![SshKeyboardInteractivePrompt {
                prompt: "Code".to_string(),
                echo: false,
            }],
            otp_id: Some("otp-1".to_string()),
        },
        response_tx,
        responses: vec![String::new()],
        focused_index: 0,
        otp_code: Some("test-code".to_string()),
        otp_type: Some("totp".to_string()),
        otp_period: 30,
        otp_time_step: Some(7),
        otp_error: None,
    });

    assert!(!prompts.apply_keyboard_interactive_otp_result(Ok(None), false));
    assert_eq!(
        prompts
            .active_keyboard_interactive()
            .expect("keyboard prompt should remain active")
            .otp_time_step,
        Some(7)
    );

    assert!(!prompts.apply_keyboard_interactive_otp_result(Ok(None), true));
    let active = prompts
        .active_keyboard_interactive()
        .expect("keyboard prompt should remain active");
    assert!(active.otp_time_step.is_none());
    assert_eq!(active.otp_error.as_deref(), Some("OTP entry not found"));

    assert!(prompts.apply_keyboard_interactive_otp_result(
        Ok(Some(NativeOtpCodePreview {
            code: "next-code".to_string(),
            otp_type: "totp".to_string(),
            period: 30,
            time_step: Some(8),
        })),
        true,
    ));
    let active = prompts
        .active_keyboard_interactive()
        .expect("keyboard prompt should remain active");
    assert_eq!(active.otp_time_step, Some(8));
    assert!(active.otp_error.is_none());
}

#[test]
fn session_state_owns_live_runtime_and_initializes_transient_state() {
    let cx = TestAppContext::single();
    let focus = || cx.update(|cx| cx.focus_handle());
    let manager = Arc::new(SessionManager::new());
    let event_bridge = SessionEventBridge::spawn(
        Arc::clone(&manager),
        TerminalFramePipeline::default(),
        "utf-8".to_string(),
        10_000,
    );
    let otp_provider = test_otp_provider();
    let mut sessions = SessionFeatureState::new(
        Arc::clone(&manager),
        event_bridge,
        Arc::clone(&otp_provider),
        SessionFeatureFocus {
            credential: focus(),
            tab_actions: focus(),
            color_picker: focus(),
            info: focus(),
        },
    );

    assert!(Arc::ptr_eq(&sessions.manager_handle(), &manager));
    assert!(!sessions.start_has_pending());
    assert!(!sessions.start.has_failed());
    assert!(sessions.command_history_for("missing").is_none());
    assert!(sessions.active_id().is_none());
    assert!(sessions.active_ssh_config().is_none());
    assert_eq!(
        sessions.active_ai_execution_profile(),
        AiExecutionProfile::SendOnly
    );
    assert!(sessions.order.is_empty());
    assert!(sessions.metadata.is_empty());
    assert_eq!(sessions.protocol_runtime_counts(), (0, 0, 0));
    assert!(Arc::ptr_eq(&sessions.prompt_otp_provider(), &otp_provider));
    assert!(sessions.prompt_active_credential().is_none());
    assert!(sessions.prompt_active_keyboard_interactive().is_none());
    assert!(!sessions.restore_is_complete());
    assert!(sessions.mark_restore_complete());
    assert!(!sessions.mark_restore_complete());

    sessions.extend_pending_events([
        SessionEvent::Output {
            session_id: "session-a".to_string(),
            data: vec![1, 2, 3],
        },
        SessionEvent::Error {
            session_id: "session-a".to_string(),
            message: "test error".to_string(),
        },
    ]);
    assert_eq!(sessions.pending_event_count(), 2);
    assert_eq!(sessions.pending_event_output_bytes(), 3);
    assert!(matches!(
        sessions.pop_pending_event(),
        Some(SessionEvent::Output { data, .. }) if data == vec![1, 2, 3]
    ));

    sessions.record_command_history("session-a", "  git status  ");
    sessions.record_command_history("session-a", "cargo check");
    assert_eq!(
        sessions.command_history_for("session-a"),
        Some(["cargo check".to_string(), "git status".to_string()].as_slice())
    );
    sessions.migrate_command_history("session-a", "session-b");
    sessions.remove_command_from_all_history("git status");
    assert_eq!(
        sessions.command_history_for("session-b"),
        Some(["cargo check".to_string()].as_slice())
    );

    assert!(sessions.begin_reconnect_action("session-b".to_string()));
    assert_eq!(sessions.busy_action("session-b"), Some("reconnect"));
    assert!(!sessions.begin_disconnect_action("session-b".to_string()));
    sessions.finish_busy_action("session-b");
    assert!(!sessions.session_is_busy("session-b"));

    sessions
        .dialogs
        .open_startup_command(StartupCommandAction::Multiplex, 75_000);
    assert_eq!(sessions.dialogs.startup_command_delay_ms(), 60_000);
    assert!(
        sessions
            .dialogs
            .apply_text_input("startup-command", "  uptime  ".to_string())
    );
    let (action, request) = sessions
        .dialogs
        .take_startup_command()
        .expect("non-empty command should be accepted");
    assert_eq!(action, StartupCommandAction::Multiplex);
    assert_eq!(request.command, "uptime");
    assert_eq!(request.delay_ms, 60_000);
    assert!(sessions.dialogs.take_startup_command().is_none());

    sessions
        .dialogs
        .open_tab_actions("session-a".to_string(), Some((10.0, 20.0)));
    assert!(
        sessions
            .dialogs
            .select_tab_actions_submenu(TabActionsSubmenu::Ai)
    );
    sessions.dialogs.request_quit_after_close_all();
    sessions.dialogs.open_close_all_sessions_confirm();
    assert!(sessions.dialogs.tab_actions_session_id().is_none());
    assert!(sessions.dialogs.take_close_all_sessions_confirm());
    assert!(!sessions.dialogs.should_quit_after_close_all());

    sessions
        .dialogs
        .open_rename("session-a".to_string(), "original");
    assert!(
        sessions
            .dialogs
            .apply_text_input("rename", "   ".to_string())
    );
    assert!(matches!(
        sessions.dialogs.take_rename_submission(),
        RenameSessionSubmission::Empty
    ));
    assert!(
        sessions
            .dialogs
            .apply_text_input("rename", "renamed".to_string())
    );
    assert!(matches!(
        sessions.dialogs.take_rename_submission(),
        RenameSessionSubmission::Ready { session_id, name }
            if session_id == "session-a" && name == "renamed"
    ));

    sessions.dialogs.open_temporary_ssh_link();
    sessions
        .dialogs
        .apply_temporary_ssh_link("user@example.test".to_string());
    sessions
        .dialogs
        .reject_temporary_ssh_link("temporarySsh.invalid");
    assert_eq!(
        sessions.dialogs.temporary_ssh_link_error(),
        Some("temporarySsh.invalid")
    );
    sessions
        .dialogs
        .set_temporary_link_protocol(TemporaryLinkProtocol::Serial);
    assert_eq!(
        sessions.dialogs.temporary_link_protocol(),
        TemporaryLinkProtocol::Serial
    );
    assert_eq!(sessions.dialogs.temporary_ssh_link_error(), None);
    sessions
        .dialogs
        .apply_temporary_serial_port_name("COM7".to_string());
    sessions
        .dialogs
        .apply_temporary_serial_baud_rate("115200x".to_string());
    assert_eq!(sessions.dialogs.temporary_serial_port_name(), "COM7");
    assert_eq!(sessions.dialogs.temporary_serial_baud_rate(), "115200");
    sessions.dialogs.close_temporary_ssh_link();
    assert_eq!(
        sessions.dialogs.temporary_link_protocol(),
        TemporaryLinkProtocol::Ssh
    );
    assert!(sessions.dialogs.temporary_ssh_link_draft().is_empty());
    assert!(sessions.dialogs.temporary_serial_port_name().is_empty());
    assert_eq!(sessions.dialogs.temporary_serial_baud_rate(), "115200");
}

#[test]
fn session_catalog_registration_and_reordering_stay_synchronized() {
    let cx = TestAppContext::single();
    let mut sessions = session_state(&cx);

    sessions.register_session_metadata("session-a", session_metadata("first", None));
    sessions.register_session_metadata("session-b", session_metadata("second", None));
    sessions.register_session_metadata("session-c", session_metadata("third", None));
    sessions.register_session_metadata("session-a", session_metadata("updated", None));

    assert_eq!(
        sessions.session_order(),
        ["session-a", "session-b", "session-c"]
    );
    assert_eq!(sessions.ordered_sessions().len(), 3);
    assert_eq!(sessions.session_info("session-a").unwrap().name, "updated");

    assert!(sessions.move_session_after("session-a", "session-c"));
    assert_eq!(
        sessions.session_order(),
        ["session-b", "session-c", "session-a"]
    );
    assert!(sessions.move_session_to_index("session-c", 0));
    assert_eq!(
        sessions.session_order(),
        ["session-c", "session-b", "session-a"]
    );
    assert!(!sessions.move_session_after("missing", "session-a"));
}

#[test]
fn session_catalog_owns_presentation_and_active_history_queries() {
    let cx = TestAppContext::single();
    let mut sessions = session_state(&cx);
    let mut metadata = session_metadata("fallback", None);
    let ssh = SshSessionConfig {
        name: "catalog name".to_string(),
        username: "nya".to_string(),
        host: "example.test".to_string(),
        port: 2222,
        ..SshSessionConfig::default()
    };
    metadata.launch_config = SessionLaunchConfig::Ssh(Box::new(ssh));
    metadata.disconnected = true;
    sessions.register_session_metadata("session-a", metadata);

    let info = sessions
        .session_info("session-a")
        .expect("registered metadata should produce session info");
    assert_eq!(sessions.display_name_by_info(&info), "catalog name");
    sessions.set_dynamic_title("session-a", Some("dynamic title".to_string()));
    assert_eq!(
        sessions.display_name("session-a").as_deref(),
        Some("dynamic title")
    );
    sessions.set_custom_name("session-a".to_string(), "custom name".to_string());
    assert_eq!(
        sessions.display_name("session-a").as_deref(),
        Some("custom name")
    );
    assert_eq!(
        sessions.endpoint("session-a").as_deref(),
        Some("nya@example.test:2222")
    );
    assert_eq!(
        sessions.ssh_host("session-a").as_deref(),
        Some("example.test")
    );
    assert_eq!(
        sessions.ssh_address("session-a").as_deref(),
        Some("ssh -p 2222 nya@example.test")
    );
    assert!(sessions.is_disconnected("session-a"));

    sessions.update_cwd("session-a", "/srv/app".to_string());
    assert_eq!(
        sessions.tab_tooltip_lines("session-a"),
        [
            "nya@example.test:2222",
            "ssh -p 2222 nya@example.test",
            "Disconnected — press Enter to reconnect",
            "cwd /srv/app",
        ]
    );

    sessions.select_active_session("session-a");
    sessions.record_command_history("session-a", "git status");
    sessions.record_command_history("session-a", "cargo check");
    assert_eq!(
        sessions.active_command_history_snapshot(),
        ["cargo check", "git status"]
    );
    assert_eq!(
        sessions.active_command_history_entry(1).as_deref(),
        Some("git status")
    );
}

#[test]
fn active_session_selection_derives_configuration_from_the_catalog() {
    let cx = TestAppContext::single();
    let mut sessions = session_state(&cx);
    let mut metadata = session_metadata("first", None);
    metadata.ssh_config = Some(SshSessionConfig::default());
    metadata.ai_execution_profile = AiExecutionProfile::Auto;
    sessions.register_session_metadata("session-a", metadata);

    assert!(sessions.select_active_session_if_none("session-a"));
    assert_eq!(sessions.active_id(), Some("session-a"));
    assert!(sessions.active_ssh_config().is_some());
    assert_eq!(
        sessions.active_ai_execution_profile(),
        AiExecutionProfile::Auto
    );
    assert!(!sessions.select_active_session_if_none("session-b"));

    sessions.register_session_metadata("session-a", session_metadata("updated", None));
    assert_eq!(sessions.active_id(), Some("session-a"));
    assert!(sessions.active_ssh_config().is_none());
    assert_eq!(
        sessions.active_ai_execution_profile(),
        AiExecutionProfile::Posix
    );

    assert_eq!(
        sessions.select_active_session("missing").as_deref(),
        Some("session-a")
    );
    assert_eq!(sessions.active_id(), Some("missing"));
    assert!(sessions.active_ssh_config().is_none());
    assert_eq!(
        sessions.active_ai_execution_profile(),
        AiExecutionProfile::SendOnly
    );

    assert_eq!(sessions.clear_active_session().as_deref(), Some("missing"));
    assert!(sessions.active_id().is_none());
    assert!(sessions.active_ssh_config().is_none());
    assert_eq!(
        sessions.active_ai_execution_profile(),
        AiExecutionProfile::SendOnly
    );
}

#[test]
fn session_disconnect_transition_is_idempotent_and_reports_multiplex_owner() {
    let cx = TestAppContext::single();
    let mut sessions = session_state(&cx);

    assert!(sessions.mark_session_disconnected("missing").is_none());
    sessions.register_session_metadata("session-a", session_metadata("first", Some("multiplex-a")));
    sessions
        .register_session_metadata("session-b", session_metadata("second", Some("multiplex-a")));
    assert!(sessions.other_live_session_uses_multiplex_key("session-a", "multiplex-a"));

    let changed = sessions
        .mark_session_disconnected("session-a")
        .expect("registered session should transition");
    assert!(!changed.already_disconnected);
    assert_eq!(changed.multiplex_key.as_deref(), Some("multiplex-a"));
    assert!(sessions.metadata("session-a").unwrap().disconnected);
    assert!(sessions.other_live_session_uses_multiplex_key("session-a", "multiplex-a"));

    let unchanged = sessions
        .mark_session_disconnected("session-a")
        .expect("disconnected session should remain registered");
    assert!(unchanged.already_disconnected);
    assert_eq!(unchanged.multiplex_key.as_deref(), Some("multiplex-a"));

    sessions
        .mark_session_disconnected("session-b")
        .expect("shared session should transition");
    assert!(!sessions.other_live_session_uses_multiplex_key("session-a", "multiplex-a"));
}

#[test]
fn reconnect_presentation_migration_preserves_destination_overrides() {
    let cx = TestAppContext::single();
    let mut sessions = session_state(&cx);

    sessions.set_custom_name("old".to_string(), "old name".to_string());
    sessions.set_custom_name("new".to_string(), "new name".to_string());
    sessions.set_dynamic_title("old", Some("old title".to_string()));
    sessions.set_dynamic_title("new", Some("new title".to_string()));
    assert!(sessions.update_cwd("old", "/old".to_string()));
    assert!(!sessions.update_cwd("old", "/old".to_string()));
    assert!(sessions.update_cwd("new", "/new".to_string()));
    sessions.set_tab_color("old", Some(0x112233));
    sessions.set_tab_color("new", Some(0x445566));
    assert!(sessions.set_tab_locked("old", true));
    sessions.record_command_history("old", "pwd");

    sessions.migrate_session_presentation("old", "new");

    assert_eq!(sessions.custom_name("new"), Some("new name"));
    assert_eq!(sessions.dynamic_title("new"), Some("old title"));
    assert!(sessions.dynamic_title("old").is_none());
    assert_eq!(sessions.cwd("new"), Some("/old"));
    assert!(sessions.cwd("old").is_none());
    assert_eq!(sessions.tab_color("new"), Some(0x445566));
    assert!(sessions.tab_is_locked("new"));
    assert!(!sessions.tab_is_locked("old"));
    assert_eq!(
        sessions.command_history_for("new"),
        Some(["pwd".to_string()].as_slice())
    );
    assert!(sessions.command_history_for("old").is_none());
}

#[test]
fn removing_session_catalog_clears_all_session_scoped_entries() {
    let cx = TestAppContext::single();
    let mut sessions = session_state(&cx);

    let mut metadata = session_metadata("first", Some("multiplex-a"));
    metadata.ssh_config = Some(SshSessionConfig::default());
    metadata.ai_execution_profile = AiExecutionProfile::Auto;
    sessions.register_session_metadata("session-a", metadata);
    sessions.set_custom_name("session-a".to_string(), "custom".to_string());
    sessions.set_dynamic_title("session-a", Some("dynamic".to_string()));
    sessions.update_cwd("session-a", "/workspace".to_string());
    sessions.set_tab_color("session-a", Some(0x112233));
    assert!(sessions.set_tab_locked("session-a", true));
    sessions.record_command_history("session-a", "pwd");
    assert!(sessions.begin_reconnect_action("session-a".to_string()));
    sessions.zmodem_state_mut_or_default("session-a");
    sessions.trzsz_state_mut_or_default("session-a");
    assert_eq!(sessions.protocol_runtime_counts(), (1, 1, 0));
    sessions.select_active_session("session-a");
    assert!(sessions.active_ssh_config().is_some());
    assert_eq!(
        sessions.active_ai_execution_profile(),
        AiExecutionProfile::Auto
    );
    assert_eq!(
        sessions.remove_session_catalog("session-a").as_deref(),
        Some("multiplex-a")
    );
    assert!(!sessions.has_session("session-a"));
    assert!(sessions.session_order().is_empty());
    assert!(sessions.custom_name("session-a").is_none());
    assert!(sessions.dynamic_title("session-a").is_none());
    assert!(sessions.cwd("session-a").is_none());
    assert!(sessions.tab_color("session-a").is_none());
    assert!(!sessions.tab_is_locked("session-a"));
    assert!(sessions.command_history_for("session-a").is_none());
    assert!(!sessions.session_is_busy("session-a"));
    assert!(sessions.active_id().is_none());
    assert!(sessions.active_ssh_config().is_none());
    assert_eq!(
        sessions.active_ai_execution_profile(),
        AiExecutionProfile::SendOnly
    );
    assert_eq!(sessions.protocol_runtime_counts(), (0, 0, 0));
}

#[test]
fn tab_lock_and_drag_state_have_single_owner() {
    let cx = TestAppContext::single();
    let mut sessions = session_state(&cx);

    assert!(sessions.set_tab_locked("tab-a", true));
    assert!(sessions.tab_is_locked("tab-a"));
    assert!(!sessions.set_tab_locked("tab-a", true));
    assert!(sessions.set_tab_locked("tab-a", false));
    assert!(!sessions.tab_is_locked("tab-a"));

    assert!(sessions.set_tab_drag_target("tab-a".to_string(), "tab-b".to_string(), false,));
    assert!(sessions.tab_drag_source_is("tab-a"));
    assert_eq!(sessions.tab_drop_after("tab-b"), Some(false));
    assert!(!sessions.set_tab_drag_target("tab-a".to_string(), "tab-b".to_string(), false,));
    assert!(sessions.set_tab_drag_target("tab-a".to_string(), "tab-b".to_string(), true,));
    assert_eq!(sessions.tab_drop_after("tab-b"), Some(true));
    assert!(sessions.clear_tab_drag());
    assert!(!sessions.clear_tab_drag());
}

#[test]
fn closing_a_split_root_can_migrate_logical_tab_presentation() {
    let cx = TestAppContext::single();
    let mut sessions = session_state(&cx);
    sessions.set_custom_name("root".to_string(), "logical tab".to_string());
    sessions.set_custom_name("survivor".to_string(), "pane".to_string());
    sessions.set_tab_color("root", Some(0x112233));
    sessions.set_tab_locked("root", true);

    sessions.migrate_tab_root_presentation("root", "survivor");

    assert_eq!(sessions.custom_name("survivor"), Some("logical tab"));
    assert_eq!(sessions.tab_color("survivor"), Some(0x112233));
    assert!(sessions.tab_is_locked("survivor"));
    assert!(sessions.custom_name("root").is_none());
    assert!(sessions.tab_color("root").is_none());
    assert!(!sessions.tab_is_locked("root"));
}

#[test]
fn tab_group_reorder_moves_split_sessions_as_one_block() {
    let cx = TestAppContext::single();
    let mut sessions = session_state(&cx);
    for id in ["a", "a-child", "b", "c", "c-child"] {
        sessions.register_session_metadata(id, session_metadata(id, None));
    }

    assert!(sessions.move_session_group_relative(
        &["a".to_string(), "a-child".to_string()],
        &["c".to_string(), "c-child".to_string()],
        true,
    ));
    assert_eq!(
        sessions.session_order(),
        ["b", "c", "c-child", "a", "a-child"]
    );
    assert!(sessions.move_session_group_to_end(&["b".to_string()]));
    assert_eq!(
        sessions.session_order(),
        ["c", "c-child", "a", "a-child", "b"]
    );
}

#[test]
fn session_start_state_owns_channel_selection_and_cancellation() {
    let mut starts = SessionStartFeatureState::new();
    starts
        .pending
        .insert("request-1".to_string(), pending("local shell"));

    assert!(starts.select_pending("request-1"));
    assert_eq!(
        starts.pending_display_name().as_deref(),
        Some("local shell")
    );

    starts
        .sender()
        .send(SessionStartResult {
            request_id: "request-1".to_string(),
            connection_name: "local shell".to_string(),
            kind: SessionKind::LocalPty,
            worker_started_at: Instant::now(),
            worker_finished_at: Instant::now(),
            result: Err("cancelled".to_string()),
        })
        .expect("session start event channel should stay connected");
    assert_eq!(
        starts
            .try_recv()
            .expect("session start result should reach its owner")
            .request_id,
        "request-1"
    );

    let closed = starts
        .close_pending("request-1")
        .expect("selected pending start should close");
    assert_eq!(closed.connection_name, "local shell");
    assert!(starts.has_cancelled_results());
    assert!(matches!(
        starts.take_event_request("request-1"),
        SessionStartEventRequest::Cancelled
    ));
    assert!(!starts.has_cancelled_results());
    assert!(!starts.has_pending());
    assert!(!starts.has_active_pending());
}

#[test]
fn session_start_registration_owns_fresh_and_reconnect_selection() {
    let mut fresh = SessionStartFeatureState::new();
    assert!(!fresh.register_pending("request-fresh".to_string(), pending("fresh")));
    assert!(fresh.request_is_active("request-fresh"));
    assert_eq!(fresh.pending_count(), 1);

    let mut reconnect = SessionStartFeatureState::new();
    reconnect.set_reconnect_target("session-old".to_string());
    assert!(reconnect.register_pending("request-reconnect".to_string(), pending("reconnect")));
    assert!(!reconnect.has_active_pending());
    assert!(reconnect.reconnect_is_pending("session-old"));
    assert_eq!(reconnect.reconnect_target(), Some("session-old"));
}

#[test]
fn session_start_results_route_normal_and_reconnect_failures_atomically() {
    let mut fresh = SessionStartFeatureState::new();
    fresh.register_pending("request-fresh".to_string(), pending("fresh"));
    let SessionStartEventRequest::Pending {
        pending: pending_state,
        was_active,
    } = fresh.take_event_request("request-fresh")
    else {
        panic!("fresh result should retain pending metadata");
    };
    assert!(was_active);
    assert!(!fresh.record_failure(
        "request-fresh".to_string(),
        pending_state,
        "connection failed".to_string(),
        was_active,
        false,
    ));
    assert!(fresh.has_failed());
    assert!(fresh.has_active_failed());
    assert_eq!(
        fresh
            .active_failed()
            .expect("active failure should be retained")
            .error,
        "connection failed"
    );

    let mut reconnect = SessionStartFeatureState::new();
    reconnect.set_reconnect_target("session-old".to_string());
    reconnect.register_pending("request-reconnect".to_string(), pending("reconnect"));
    let SessionStartEventRequest::Pending {
        pending: pending_state,
        was_active,
    } = reconnect.take_event_request("request-reconnect")
    else {
        panic!("reconnect result should retain pending metadata");
    };
    assert!(!was_active);
    assert!(reconnect.record_failure(
        "request-reconnect".to_string(),
        pending_state,
        "reconnect failed".to_string(),
        was_active,
        true,
    ));
    assert!(!reconnect.has_failed());
    assert_eq!(
        reconnect.reconnect_failure("session-old"),
        Some("reconnect failed")
    );
    assert!(reconnect.reconnect_target().is_none());
}

#[test]
fn session_start_success_and_workspace_split_are_single_owner_transitions() {
    let mut starts = SessionStartFeatureState::new();
    starts.set_reconnect_target("session-old".to_string());
    assert!(!starts.complete_success(true, false, false));
    assert!(starts.reconnect_target().is_none());

    starts.set_pending_workspace_split(
        WorkspaceSplitDirection::Horizontal,
        "session-source".to_string(),
    );
    assert!(matches!(
        starts.take_pending_workspace_split(),
        Some((WorkspaceSplitDirection::Horizontal, source)) if source == "session-source"
    ));
    assert!(starts.take_pending_workspace_split().is_none());
}

#[test]
fn session_start_state_owns_saved_connection_queue_lifecycle() {
    let mut starts = SessionStartFeatureState::new();

    assert!(!starts.has_queued_saved_connections());
    assert_eq!(
        starts.queue_saved_connection(
            saved_connection("connection-1"),
            SavedConnectionStartOptions::default(),
        ),
        1
    );
    assert!(starts.saved_connection_is_queued("connection-1"));
    assert!(!starts.saved_connection_is_queued("connection-2"));

    let queued = starts
        .pop_saved_connection()
        .expect("queued saved connection should remain owned by session starts");
    assert_eq!(queued.connection.id, "connection-1");
    assert!(!starts.has_queued_saved_connections());
}

#[test]
fn closing_pending_starts_preserves_non_reconnect_and_failed_fallback_order() {
    let mut starts = SessionStartFeatureState::new();
    starts
        .pending
        .insert("request-active".to_string(), pending("active"));
    starts
        .pending
        .insert("request-normal".to_string(), pending("normal"));
    let mut reconnect = pending("reconnect");
    reconnect.reconnect_session_id = Some("old-session".to_string());
    starts
        .pending
        .insert("request-reconnect".to_string(), reconnect);
    starts.failed.insert(
        "request-failed".to_string(),
        FailedSessionStart {
            pending: pending("failed"),
            error: "failed".to_string(),
        },
    );

    assert!(starts.select_pending("request-active"));
    starts
        .close_pending("request-active")
        .expect("active start should close");
    assert_eq!(starts.active_pending.as_deref(), Some("request-normal"));
    assert!(starts.active_failed.is_none());

    starts
        .close_pending("request-normal")
        .expect("normal start should close");
    assert!(starts.active_pending.is_none());
    assert_eq!(starts.active_failed.as_deref(), Some("request-failed"));
    assert!(starts.pending.contains_key("request-reconnect"));
}

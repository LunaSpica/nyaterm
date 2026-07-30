use std::collections::{HashMap, VecDeque};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

use tokio::sync::mpsc as tokio_mpsc;

use super::{
    DO, ForwardedTcpIpDispatch, IAC, LocalSessionConfig, OPT_SUPPRESS_GO_AHEAD,
    QueuedTransportWriter, SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT,
    SESSION_EVENT_QUEUE_OUTPUT_LIMIT, SerialSessionConfig, SessionError, SessionEvent,
    SessionEventQueue, SessionManager, SshCommand, SshKeyAuthConfig, SshProxyConfig,
    SshPtyDimensions, SshSessionConfig, TelnetSessionConfig, WILL,
    drain_deferred_ssh_open_commands, expand_proxy_command, forwarded_tcpip_sender_for,
    is_process_list_unsupported, local_pty_size, normalize_process_signal, parse_process_output,
    remap_del_to_bs, run_local_command, ssh_client_config, ssh_host_identifier,
};

/// A push must hand the event straight to a parked consumer. Before the
/// queue carried a condvar the bridge could only poll, so every event paid
/// an arbitrary slice of the poll interval before anyone looked at it.
#[test]
fn blocking_drain_wakes_on_push_rather_than_timing_out() {
    let queue = SessionEventQueue::new();
    let producer = queue.clone();
    let (ready_tx, ready_rx) = mpsc::sync_channel(0);

    let waiter = std::thread::spawn(move || {
        let _ = ready_tx.send(());
        let started = Instant::now();
        let drain = producer.drain_blocking_with_output_budget(
            16,
            Some(64 * 1024),
            Duration::from_secs(30),
        );
        (drain, started.elapsed())
    });

    // Let the waiter reach its park before producing.
    ready_rx.recv().expect("waiter started");
    std::thread::sleep(Duration::from_millis(20));
    queue.push(SessionEvent::Output {
        session_id: "s1".to_string(),
        data: b"hi".to_vec(),
    });

    let (drain, waited) = waiter.join().expect("waiter finished");
    assert_eq!(drain.events.len(), 1);
    assert!(
        waited < Duration::from_secs(5),
        "the push should have woken the park, not the 30s timeout (waited {waited:?})"
    );
}

/// An event already queued must not cost a park at all.
#[test]
fn blocking_drain_returns_queued_events_immediately() {
    let queue = SessionEventQueue::new();
    queue.push(SessionEvent::Output {
        session_id: "s1".to_string(),
        data: b"hi".to_vec(),
    });

    let started = Instant::now();
    let drain =
        queue.drain_blocking_with_output_budget(16, Some(64 * 1024), Duration::from_secs(30));

    assert_eq!(drain.events.len(), 1);
    assert!(started.elapsed() < Duration::from_secs(5));
}

/// A fully idle queue still has to return so the consumer can re-check its
/// stop flag.
#[test]
fn blocking_drain_gives_up_at_the_timeout() {
    let queue = SessionEventQueue::new();
    let drain =
        queue.drain_blocking_with_output_budget(16, Some(64 * 1024), Duration::from_millis(20));
    assert!(drain.events.is_empty());
}

struct GatedWriter {
    started_tx: mpsc::SyncSender<()>,
    release_rx: mpsc::Receiver<()>,
    output: Arc<Mutex<Vec<u8>>>,
}

impl Write for GatedWriter {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        let _ = self.started_tx.send(());
        let _ = self.release_rx.recv();
        self.output
            .lock()
            .expect("output lock")
            .extend_from_slice(data);
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct WriteCapture(Arc<Mutex<Vec<Vec<u8>>>>);

impl Write for WriteCapture {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .expect("write capture lock")
            .push(data.to_vec());
        Ok(data.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn queued_transport_writer_returns_before_blocking_write_completes() {
    let (started_tx, started_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let output = Arc::new(Mutex::new(Vec::new()));
    let mut writer = QueuedTransportWriter::spawn(
        "queued".to_string(),
        GatedWriter {
            started_tx,
            release_rx,
            output: output.clone(),
        },
        false,
        SessionEventQueue::new(),
    );

    writer.write(b"input".to_vec()).expect("queue input");
    started_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("background write should start");
    assert!(output.lock().expect("output lock").is_empty());

    release_tx.send(()).expect("release writer");
    writer.close();
    assert_eq!(*output.lock().expect("output lock"), b"input");
}

#[test]
fn queued_transport_writer_preserves_character_at_a_time_mode() {
    let writes = Arc::new(Mutex::new(Vec::new()));
    let mut writer = QueuedTransportWriter::spawn(
        "character-mode".to_string(),
        WriteCapture(writes.clone()),
        true,
        SessionEventQueue::new(),
    );

    writer.write(b"abc".to_vec()).expect("queue input");
    writer.close();

    assert_eq!(
        *writes.lock().expect("write capture lock"),
        vec![vec![b'a'], vec![b'b'], vec![b'c']]
    );
}

#[test]
fn local_session_echoes_output() {
    if cfg!(target_os = "windows") {
        return;
    }

    let manager = SessionManager::new();
    let info = manager
        .create_local_session(LocalSessionConfig {
            name: "test".to_string(),
            shell_path: Some("/bin/sh".to_string()),
            shell_args: Vec::new(),
            working_dir: None,
            cols: 80,
            rows: 24,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("local session");

    manager
        .write(&info.id, b"printf nyaterm-transport-ready\\n\n")
        .expect("write");

    let output = collect_output(&manager, &info.id, Duration::from_secs(3));
    manager.close(&info.id).expect("close");

    assert!(
        String::from_utf8_lossy(&output).contains("nyaterm-transport-ready"),
        "output was: {}",
        String::from_utf8_lossy(&output)
    );
}

#[test]
fn local_session_info_preserves_working_dir() {
    if cfg!(target_os = "windows") {
        return;
    }

    let dir = std::env::temp_dir().join(format!("nyaterm-local-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let manager = SessionManager::new();
    let info = manager
        .create_local_session(LocalSessionConfig {
            name: "cwd-test".to_string(),
            shell_path: Some("/bin/sh".to_string()),
            shell_args: Vec::new(),
            working_dir: Some(dir.clone()),
            cols: 80,
            rows: 24,
            pixel_width: 0,
            pixel_height: 0,
        })
        .expect("local session");
    let sessions = manager.list_sessions().expect("sessions");
    manager.close(&info.id).expect("close");
    std::fs::remove_dir_all(&dir).ok();

    assert_eq!(sessions[0].working_dir.as_ref(), Some(&dir));
}

#[test]
fn local_background_command_uses_working_dir_and_exit_code() {
    if cfg!(target_os = "windows") {
        return;
    }

    let dir = std::env::temp_dir().join(format!("nyaterm-local-bg-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let output = run_local_command(
        "printf ready > marker.txt; printf output; exit 7",
        Some(dir.clone()),
        Duration::from_secs(3),
    )
    .expect("local command");
    let marker = std::fs::read_to_string(dir.join("marker.txt")).expect("marker");
    std::fs::remove_dir_all(&dir).ok();

    assert_eq!(marker, "ready");
    assert_eq!(output.stdout, "output");
    assert_eq!(output.exit_status, Some(7));
}

#[test]
fn resize_updates_session_info() {
    if cfg!(target_os = "windows") {
        return;
    }

    let manager = SessionManager::new();
    let info = manager
        .create_local_session(LocalSessionConfig {
            shell_path: Some("/bin/sh".to_string()),
            ..Default::default()
        })
        .expect("local session");
    manager.resize(&info.id, 120, 32).expect("resize");
    let sessions = manager.list_sessions().expect("sessions");
    manager.close(&info.id).expect("close");

    assert_eq!(sessions[0].cols, 120);
    assert_eq!(sessions[0].rows, 32);
}

#[test]
fn raw_tcp_session_echoes_output() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let port = listener.local_addr().expect("addr").port();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .expect("timeout");
        let mut buffer = [0_u8; 64];
        let read = stream.read(&mut buffer).expect("read");
        stream.write_all(b"echo:").expect("prefix");
        stream.write_all(&buffer[..read]).expect("echo");
    });

    let manager = SessionManager::new();
    let info = manager
        .create_telnet_session(TelnetSessionConfig {
            name: "raw".to_string(),
            host: "127.0.0.1".to_string(),
            port,
            raw_tcp: true,
            ..Default::default()
        })
        .expect("raw tcp");

    manager.write(&info.id, b"hello").expect("write");
    let output = collect_output_until(&manager, &info.id, "echo:hello", Duration::from_secs(3));
    manager.close(&info.id).expect("close");
    server.join().expect("server");

    assert!(String::from_utf8_lossy(&output).contains("echo:hello"));
}

#[test]
fn telnet_session_negotiates_and_strips_iac() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let port = listener.local_addr().expect("addr").port();
    let (tx, rx) = mpsc::channel();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .expect("timeout");
        stream
            .write_all(&[IAC, WILL, OPT_SUPPRESS_GO_AHEAD, b'o', b'k'])
            .expect("write greeting");

        let mut seen = Vec::new();
        let started = Instant::now();
        while started.elapsed() < Duration::from_secs(3) {
            let mut buffer = [0_u8; 64];
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    seen.extend_from_slice(&buffer[..read]);
                    if seen
                        .windows(3)
                        .any(|window| window == [IAC, DO, OPT_SUPPRESS_GO_AHEAD])
                    {
                        break;
                    }
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    continue;
                }
                Err(error) => panic!("server read failed: {error}"),
            }
        }
        tx.send(seen).expect("send seen");
    });

    let manager = SessionManager::new();
    let info = manager
        .create_telnet_session(TelnetSessionConfig {
            name: "telnet".to_string(),
            host: "127.0.0.1".to_string(),
            port,
            raw_tcp: false,
            send_sga: true,
            ..Default::default()
        })
        .expect("telnet");

    let output = collect_output_until(&manager, &info.id, "ok", Duration::from_secs(3));
    manager.close(&info.id).expect("close");
    server.join().expect("server");
    let seen = rx.recv().expect("seen");

    assert_eq!(String::from_utf8_lossy(&output), "ok");
    assert!(
        seen.windows(3)
            .any(|window| { window == [IAC, DO, OPT_SUPPRESS_GO_AHEAD] })
    );
}

#[test]
fn serial_invalid_port_reports_open_error() {
    let manager = SessionManager::new();
    let port_name = if cfg!(target_os = "windows") {
        r"\\.\NyaTermMissingPort".to_string()
    } else {
        "/dev/nyaterm-missing-port".to_string()
    };

    let error = manager
        .create_serial_session(SerialSessionConfig {
            port_name: port_name.clone(),
            ..Default::default()
        })
        .expect_err("invalid port should not open");

    match error {
        SessionError::OpenSerial {
            port_name: actual, ..
        } => assert_eq!(actual, port_name),
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn serial_backspace_mode_remaps_delete_to_ctrl_h() {
    assert_eq!(remap_del_to_bs(b"a\x7fb"), b"a\x08b");
}

#[test]
fn ssh_refused_connection_reports_create_error() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);

    let manager = SessionManager::new();
    let error = manager
        .create_ssh_session(SshSessionConfig {
            name: "ssh".to_string(),
            host: "127.0.0.1".to_string(),
            port,
            username: "tester".to_string(),
            password: Some("secret".to_string()),
            ..Default::default()
        })
        .expect_err("closed port should not open");

    match error {
        SessionError::CreateSsh { addr, .. } => assert_eq!(addr, format!("127.0.0.1:{port}")),
        other => panic!("unexpected error: {other}"),
    }
}

#[test]
fn ssh_host_identifier_uses_openssh_port_format() {
    assert_eq!(ssh_host_identifier("example.com", 22), "example.com");
    assert_eq!(
        ssh_host_identifier("example.com", 2222),
        "[example.com]:2222"
    );
}

#[test]
fn forwarded_tcpip_dispatch_prefers_listener_specific_sender() {
    let (fallback_tx, _fallback_rx) = tokio_mpsc::unbounded_channel();
    let (specific_tx, _specific_rx) = tokio_mpsc::unbounded_channel();
    let dispatch = ForwardedTcpIpDispatch {
        fallback: Some(fallback_tx.clone()),
        by_listener: HashMap::from([(("127.0.0.1".to_string(), 2022), specific_tx.clone())]),
    };

    let exact = forwarded_tcpip_sender_for(&dispatch, "127.0.0.1", 2022).expect("specific sender");
    assert!(exact.same_channel(&specific_tx));

    let fallback =
        forwarded_tcpip_sender_for(&dispatch, "127.0.0.1", 2200).expect("fallback sender");
    assert!(fallback.same_channel(&fallback_tx));

    let empty = ForwardedTcpIpDispatch::default();
    assert!(forwarded_tcpip_sender_for(&empty, "127.0.0.1", 2022).is_none());
}

#[test]
fn process_parser_reads_legacy_rows() {
    let rows = "PROCESS\t42\t1\troot\tSs\t0.4\t1.2\t1234\t5678\t01:02\tsshd\t/usr/sbin/sshd -D\n";

    let processes = parse_process_output(rows);

    assert_eq!(processes.len(), 1);
    assert_eq!(processes[0].pid, 42);
    assert_eq!(processes[0].ppid, 1);
    assert_eq!(processes[0].user, "root");
    assert_eq!(processes[0].cpu_percent, 0.4);
    assert_eq!(processes[0].command_line, "/usr/sbin/sshd -D");
}

#[test]
fn process_parser_preserves_command_lines_containing_tabs() {
    let rows = "PROCESS\t9\t1\troot\tS\t0\t0\t1\t2\t-\tawk\tawk\twith\ttabs\n";

    let processes = parse_process_output(rows);

    assert_eq!(processes.len(), 1);
    assert_eq!(processes[0].command_line, "awk\twith\ttabs");
}

#[test]
fn process_parser_detects_unsupported_marker() {
    assert!(is_process_list_unsupported(
        "warning\nNYATERM_PROCESS_UNSUPPORTED\n"
    ));
    assert!(!is_process_list_unsupported(
        "PROCESS\t1\t0\troot\tS\t0\t0\t0\t0\t-\tsh\tsh\n"
    ));
}

#[test]
fn process_signal_normalization_matches_legacy_allowlist() {
    assert_eq!(normalize_process_signal("sigterm").unwrap(), "TERM");
    assert_eq!(normalize_process_signal("9").unwrap(), "KILL");
    assert_eq!(normalize_process_signal("cont").unwrap(), "CONT");
    assert!(normalize_process_signal("USR1").is_err());
}

#[test]
fn ssh_config_debug_redacts_password() {
    let config = SshSessionConfig {
        password: Some("super-secret".to_string()),
        ..Default::default()
    };
    let debug = format!("{config:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("super-secret"));
}

#[test]
fn ssh_config_debug_redacts_key_material() {
    let config = SshSessionConfig {
        key_auth: Some(SshKeyAuthConfig {
            key_data: "-----BEGIN PRIVATE KEY-----secret-key".to_string(),
            cert_data: Some("ssh-ed25519-cert-v01@openssh.com secret-cert".to_string()),
            passphrase: Some("key-passphrase".to_string()),
        }),
        ..Default::default()
    };
    let debug = format!("{config:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("secret-key"));
    assert!(!debug.contains("secret-cert"));
    assert!(!debug.contains("key-passphrase"));
}

#[test]
fn ssh_key_auth_debug_redacts_material_when_formatted_directly() {
    let key_auth = SshKeyAuthConfig {
        key_data: "private-key-material".to_string(),
        cert_data: Some("certificate-material".to_string()),
        passphrase: Some("passphrase-material".to_string()),
    };

    let debug = format!("{key_auth:?}");
    assert!(debug.contains("<redacted>"));
    assert!(!debug.contains("private-key-material"));
    assert!(!debug.contains("certificate-material"));
    assert!(!debug.contains("passphrase-material"));
}

#[test]
fn ssh_config_debug_redacts_proxy_password() {
    let config = SshSessionConfig {
        proxy: Some(SshProxyConfig {
            protocol: "socks5".to_string(),
            host: "127.0.0.1".to_string(),
            port: 1080,
            command: None,
            username: Some("proxy-user".to_string()),
            password: Some("proxy-secret".to_string()),
        }),
        ..Default::default()
    };
    let debug = format!("{config:?}");
    assert!(debug.contains("<redacted>"));
    assert!(debug.contains("proxy-user"));
    assert!(!debug.contains("proxy-secret"));
}

#[test]
fn ssh_client_config_disables_idle_timeout_and_maps_keepalive() {
    let config = SshSessionConfig {
        keep_alive_interval_secs: 45,
        ..Default::default()
    };

    let client_config = ssh_client_config(&config);

    assert_eq!(client_config.inactivity_timeout, None);
    assert_eq!(
        client_config.keepalive_interval,
        Some(Duration::from_secs(45))
    );
    assert_eq!(client_config.keepalive_max, 3);

    let disabled = SshSessionConfig {
        keep_alive_interval_secs: 0,
        ..Default::default()
    };
    let disabled_client_config = ssh_client_config(&disabled);

    assert_eq!(disabled_client_config.inactivity_timeout, None);
    assert_eq!(disabled_client_config.keepalive_interval, None);
}

#[test]
fn local_config_defaults_to_unknown_pixel_dimensions() {
    let config = LocalSessionConfig::default();
    assert_eq!(config.cols, 80);
    assert_eq!(config.rows, 24);
    assert_eq!(config.pixel_width, 0);
    assert_eq!(config.pixel_height, 0);
}

#[test]
fn local_pty_size_preserves_cell_and_pixel_dimensions() {
    let size = local_pty_size(132, 43, 1056, 688);
    assert_eq!(size.cols, 132);
    assert_eq!(size.rows, 43);
    assert_eq!(size.pixel_width, 1056);
    assert_eq!(size.pixel_height, 688);
}

#[test]
fn ssh_pty_dimensions_clamp_to_positive_cells() {
    let dimensions = SshPtyDimensions::new(0, 0, 0, 0);
    assert_eq!(dimensions.cols, 1);
    assert_eq!(dimensions.rows, 1);
    assert_eq!(dimensions.pixel_width, 0);
    assert_eq!(dimensions.pixel_height, 0);

    let dimensions = SshPtyDimensions::new(132, 43, 1056, 688);
    assert_eq!(dimensions.cols, 132);
    assert_eq!(dimensions.rows, 43);
    assert_eq!(dimensions.pixel_width, 1056);
    assert_eq!(dimensions.pixel_height, 688);
}

#[test]
fn ssh_pty_dimensions_use_config_size() {
    let config = SshSessionConfig {
        cols: 101,
        rows: 37,
        pixel_width: 808,
        pixel_height: 592,
        ..Default::default()
    };
    let dimensions = SshPtyDimensions::from_config(&config);
    assert_eq!(dimensions.cols, 101);
    assert_eq!(dimensions.rows, 37);
    assert_eq!(dimensions.pixel_width, 808);
    assert_eq!(dimensions.pixel_height, 592);
}

#[test]
fn deferred_ssh_open_drain_keeps_writes_and_latest_resize() {
    let (tx, mut rx) = tokio_mpsc::unbounded_channel();
    tx.send(SshCommand::Write(b"before".to_vec())).unwrap();
    tx.send(SshCommand::Resize {
        cols: 100,
        rows: 30,
        pixel_width: 800,
        pixel_height: 600,
    })
    .unwrap();
    tx.send(SshCommand::Resize {
        cols: 132,
        rows: 43,
        pixel_width: 1056,
        pixel_height: 688,
    })
    .unwrap();
    tx.send(SshCommand::Write(b"after".to_vec())).unwrap();

    let mut dimensions = SshPtyDimensions::new(80, 24, 0, 0);
    let mut pending_writes = VecDeque::new();
    let should_close =
        drain_deferred_ssh_open_commands(&mut rx, &mut dimensions, &mut pending_writes);

    assert!(!should_close);
    assert_eq!(dimensions, SshPtyDimensions::new(132, 43, 1056, 688));
    assert_eq!(
        pending_writes.into_iter().collect::<Vec<_>>(),
        vec![b"before".to_vec(), b"after".to_vec()]
    );
}

#[test]
fn deferred_ssh_open_drain_closes_before_shell_open() {
    let (tx, mut rx) = tokio_mpsc::unbounded_channel();
    tx.send(SshCommand::Write(b"queued".to_vec())).unwrap();
    tx.send(SshCommand::Close).unwrap();

    let mut dimensions = SshPtyDimensions::new(80, 24, 0, 0);
    let mut pending_writes = VecDeque::new();
    let should_close =
        drain_deferred_ssh_open_commands(&mut rx, &mut dimensions, &mut pending_writes);

    assert!(should_close);
    assert_eq!(
        pending_writes.into_iter().collect::<Vec<_>>(),
        vec![b"queued".to_vec()]
    );
}

#[test]
fn deferred_ssh_open_drain_closes_on_disconnected_command_channel() {
    let (tx, mut rx) = tokio_mpsc::unbounded_channel();
    drop(tx);

    let mut dimensions = SshPtyDimensions::new(80, 24, 0, 0);
    let mut pending_writes = VecDeque::new();

    assert!(drain_deferred_ssh_open_commands(
        &mut rx,
        &mut dimensions,
        &mut pending_writes
    ));
    assert!(pending_writes.is_empty());
}

#[test]
fn proxy_command_expansion_replaces_ssh_tokens() {
    let expanded = expand_proxy_command(
        Some("nc %h %p --user %r --literal %%"),
        "host name",
        2222,
        "user'name",
    )
    .expect("expanded command");

    #[cfg(windows)]
    {
        assert!(expanded.contains("\"host name\""));
        assert!(expanded.contains("2222"));
        assert!(expanded.contains("\"user'name\""));
    }
    #[cfg(not(windows))]
    {
        assert!(expanded.contains("'host name'"));
        assert!(expanded.contains("'2222'"));
        assert!(expanded.contains("'user'\\''name'"));
    }
    assert!(expanded.contains("--literal %"));
}

fn collect_output(manager: &SessionManager, session_id: &str, timeout: Duration) -> Vec<u8> {
    collect_output_until(manager, session_id, "nyaterm-transport-ready", timeout)
}

fn collect_output_until(
    manager: &SessionManager,
    session_id: &str,
    needle: &str,
    timeout: Duration,
) -> Vec<u8> {
    let started = Instant::now();
    let mut output = Vec::new();
    while started.elapsed() < timeout {
        for event in manager.drain_events(16).expect("events").events {
            match event {
                SessionEvent::Output {
                    session_id: event_session_id,
                    data,
                } if event_session_id == session_id => output.extend(data),
                SessionEvent::OutputDropped { .. } => {}
                SessionEvent::Error { message, .. } => panic!("session error: {message}"),
                _ => {}
            }
        }
        if String::from_utf8_lossy(&output).contains(needle) {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    output
}

#[test]
fn session_event_queue_keeps_consecutive_output_chunks_separate() {
    let queue = SessionEventQueue::new();
    queue.push(SessionEvent::Output {
        session_id: "a".to_string(),
        data: b"hello ".to_vec(),
    });
    queue.push(SessionEvent::Output {
        session_id: "a".to_string(),
        data: b"world".to_vec(),
    });

    let drain = queue.drain(8);
    assert_eq!(drain.events.len(), 2);
    assert_eq!(drain.stats.drained_output_bytes, 11);
    assert_eq!(drain.stats.queued_output_bytes, 0);
    match &drain.events[0] {
        SessionEvent::Output { session_id, data } => {
            assert_eq!(session_id, "a");
            assert_eq!(data, b"hello ");
        }
        event => panic!("unexpected event: {event:?}"),
    }
    match &drain.events[1] {
        SessionEvent::Output { session_id, data } => {
            assert_eq!(session_id, "a");
            assert_eq!(data, b"world");
        }
        event => panic!("unexpected event: {event:?}"),
    }
}

#[test]
fn session_event_queue_keeps_sessions_separate() {
    let queue = SessionEventQueue::new();
    queue.push(SessionEvent::Output {
        session_id: "a".to_string(),
        data: b"a1".to_vec(),
    });
    queue.push(SessionEvent::Output {
        session_id: "b".to_string(),
        data: b"b1".to_vec(),
    });
    queue.push(SessionEvent::Output {
        session_id: "a".to_string(),
        data: b"a2".to_vec(),
    });

    let drain = queue.drain(8);
    assert_eq!(drain.events.len(), 3);
    assert!(matches!(
        &drain.events[0],
        SessionEvent::Output { session_id, data } if session_id == "a" && data == b"a1"
    ));
    assert!(matches!(
        &drain.events[1],
        SessionEvent::Output { session_id, data } if session_id == "b" && data == b"b1"
    ));
    assert!(matches!(
        &drain.events[2],
        SessionEvent::Output { session_id, data } if session_id == "a" && data == b"a2"
    ));
}

#[test]
fn session_event_queue_respects_output_drain_budget() {
    let queue = SessionEventQueue::new();
    queue.push(SessionEvent::Output {
        session_id: "a".to_string(),
        data: vec![b'a'; 128],
    });
    queue.push(SessionEvent::Output {
        session_id: "b".to_string(),
        data: vec![b'b'; 128],
    });

    let drain = queue.drain_with_output_budget(8, Some(200));
    assert_eq!(drain.events.len(), 2);
    assert_eq!(drain.stats.drained_output_bytes, 200);
    assert_eq!(drain.stats.queued_output_bytes, 56);
    assert!(matches!(
        &drain.events[0],
        SessionEvent::Output { session_id, data } if session_id == "a" && data.len() == 128
    ));
    assert!(matches!(
        &drain.events[1],
        SessionEvent::Output { session_id, data } if session_id == "b" && data.len() == 72
    ));

    let drain = queue.drain_with_output_budget(8, Some(200));
    assert_eq!(drain.events.len(), 1);
    assert_eq!(drain.stats.drained_output_bytes, 56);
    assert_eq!(drain.stats.queued_output_bytes, 0);
}

#[test]
fn session_event_queue_zero_output_budget_does_not_drain_output() {
    let queue = SessionEventQueue::new();
    queue.push(SessionEvent::Output {
        session_id: "a".to_string(),
        data: b"hello".to_vec(),
    });

    let drain = queue.drain_with_output_budget(8, Some(0));
    assert!(drain.events.is_empty());
    assert_eq!(drain.stats.drained_output_bytes, 0);
    assert_eq!(drain.stats.queued_output_bytes, 5);

    let drain = queue.drain_with_output_budget(8, Some(8));
    assert_eq!(drain.events.len(), 1);
    assert_eq!(drain.stats.drained_output_bytes, 5);
    assert_eq!(drain.stats.queued_output_bytes, 0);
}

#[test]
fn session_event_queue_zero_output_budget_can_drain_drop_marker() {
    let queue = SessionEventQueue::new();
    queue.push(SessionEvent::Output {
        session_id: "a".to_string(),
        data: vec![b'x'; SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT + 32],
    });

    let drain = queue.drain_with_output_budget(8, Some(0));
    assert_eq!(drain.events.len(), 1);
    assert!(matches!(
        &drain.events[0],
        SessionEvent::OutputDropped { session_id, bytes } if session_id == "a" && *bytes == 32
    ));
    assert_eq!(drain.stats.drained_output_bytes, 0);
    assert_eq!(
        drain.stats.queued_output_bytes,
        SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT
    );

    let drain = queue.drain_with_output_budget(8, Some(8));
    assert_eq!(drain.events.len(), 1);
    assert_eq!(drain.stats.drained_output_bytes, 8);
    assert_eq!(
        drain.stats.queued_output_bytes,
        SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT - 8
    );
}

#[test]
fn session_event_queue_trims_oversized_output_and_reports_drop() {
    let queue = SessionEventQueue::new();
    queue.push(SessionEvent::Output {
        session_id: "a".to_string(),
        data: vec![b'x'; SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT + 32],
    });

    let drain = queue.drain(8);
    assert_eq!(drain.events.len(), 2);
    assert!(matches!(
        &drain.events[0],
        SessionEvent::OutputDropped { session_id, bytes } if session_id == "a" && *bytes == 32
    ));
    assert!(matches!(
        &drain.events[1],
        SessionEvent::Output { data, .. } if data.len() == SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT
    ));
    assert_eq!(drain.stats.dropped_output_bytes, 32);
}

#[test]
fn session_event_queue_keeps_adjacent_output_chunks_separate() {
    let queue = SessionEventQueue::new();
    queue.push(SessionEvent::Output {
        session_id: "a".to_string(),
        data: vec![b'a'; SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT - 8],
    });
    queue.push(SessionEvent::Output {
        session_id: "a".to_string(),
        data: vec![b'b'; 16],
    });

    let drain = queue.drain(8);
    assert_eq!(drain.events.len(), 2);
    assert!(matches!(
        &drain.events[0],
        SessionEvent::Output { session_id, data } if session_id == "a"
            && data.len() == SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT - 8
            && data[0] == b'a'
    ));
    assert!(matches!(
        &drain.events[1],
        SessionEvent::Output { session_id, data } if session_id == "a"
            && data.len() == 16
            && *data.last().unwrap() == b'b'
    ));
    assert_eq!(drain.stats.dropped_output_bytes, 0);
}

#[test]
fn session_event_queue_reports_global_limit_drops_for_trimmed_session() {
    let queue = SessionEventQueue::new();
    let event_count =
        (SESSION_EVENT_QUEUE_OUTPUT_LIMIT / SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT) + 2;
    for index in 0..event_count {
        queue.push(SessionEvent::Output {
            session_id: format!("session-{index}"),
            data: vec![b'x'; SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT],
        });
    }

    let drain = queue.drain(event_count + 8);
    let dropped = drain
        .events
        .iter()
        .filter_map(|event| match event {
            SessionEvent::OutputDropped { session_id, bytes } => {
                Some((session_id.as_str(), *bytes))
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        dropped,
        vec![
            ("session-0", SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT),
            ("session-1", SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT),
        ]
    );
    assert_eq!(
        drain.stats.drained_output_bytes,
        SESSION_EVENT_QUEUE_OUTPUT_LIMIT
    );
    assert_eq!(
        drain.stats.dropped_output_bytes,
        SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT * 2
    );
}

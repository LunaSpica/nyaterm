use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use nyaterm_remote_desktop::{
    PROTOCOL_VERSION, RdpControlMessage, RdpSessionState, decode_control, encode_control,
    read_packet, write_packet,
};

fn helper_command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_nyaterm-rdp-helper"))
}

#[test]
fn helper_handshake_disconnects_and_reaps_cleanly() {
    let mut child = helper_command()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();
    write_packet(
        &mut stdin,
        &encode_control(&RdpControlMessage::ClientHello {
            version: PROTOCOL_VERSION,
        })
        .unwrap(),
    )
    .unwrap();
    let hello = decode_control(&read_packet(&mut stdout).unwrap().unwrap()).unwrap();
    assert!(matches!(
        hello,
        RdpControlMessage::ServerHello {
            version: PROTOCOL_VERSION
        }
    ));

    write_packet(
        &mut stdin,
        &encode_control(&RdpControlMessage::Disconnect {
            session_id: "test-session".to_string(),
        })
        .unwrap(),
    )
    .unwrap();
    let disconnecting = decode_control(&read_packet(&mut stdout).unwrap().unwrap()).unwrap();
    assert!(matches!(
        disconnecting,
        RdpControlMessage::State {
            state: RdpSessionState::Disconnecting,
            ..
        }
    ));
    let disconnected = decode_control(&read_packet(&mut stdout).unwrap().unwrap()).unwrap();
    assert!(matches!(
        disconnected,
        RdpControlMessage::State {
            state: RdpSessionState::Disconnected,
            ..
        }
    ));
    drop(stdin);
    assert!(child.wait().unwrap().success());
}

#[test]
fn helper_crash_and_hang_processes_can_always_be_reaped() {
    let crash = helper_command()
        .env("NYATERM_RDP_HELPER_TEST_MODE", "crash")
        .status()
        .unwrap();
    assert_eq!(crash.code(), Some(91));

    let mut hung = helper_command()
        .env("NYATERM_RDP_HELPER_TEST_MODE", "hang")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_millis(100);
    while Instant::now() < deadline {
        assert!(hung.try_wait().unwrap().is_none());
        std::thread::sleep(Duration::from_millis(10));
    }
    hung.kill().unwrap();
    let status = hung.wait().unwrap();
    assert!(!status.success());
}

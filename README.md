# NyaTerm

NyaTerm is a native desktop terminal workspace for local shells, SSH, Telnet,
Serial, RDP, VNC, SFTP, tunnels, and remote operations. It is built in Rust
with GPUI and keeps terminal, transport, persistence, and presentation logic
in separate workspace crates.

NyaTerm is designed for people who move between servers, devices, local
commands, and remote files throughout the day. It is a terminal workspace and
remote-session client, not a replacement shell: it connects to shells and
remote services that you provide.

## Highlights

- Tabbed and split-pane workspaces for saved connections and active sessions.
- Local, SSH, Telnet, and Serial terminal sessions with search, scrollback,
  recording, command history, quick commands, and configurable terminal input.
- SFTP browsing and file operations with queued uploads/downloads, recursive
  transfers, remote-file editing, and terminal working-directory integration.
- SSH authentication, known-host verification, private keys, OTP, SSH Agent,
  proxies, jump hosts, tunnels, and per-connection algorithm preferences.
- RDP and VNC remote-desktop sessions with native GPUI presentation and
  isolated transport runtimes.
- Remote host tools for processes, resources, GPU/NPU information, and Docker
  where the connection supports them.
- AI-assisted command generation and terminal-output analysis with explicit
  risk and execution controls.
- Encrypted configuration snapshots, backup and cloud-sync workflows, import
  from compatible terminal clients, and compatibility-preserving storage.

Published builds and user documentation are available from
[nyaterm.app](https://nyaterm.app) and the
[GitHub Releases](https://github.com/nyakang/nyaterm/releases) page when a
release is published. The repository is always the source of truth for the
native GPUI build and compatibility behavior.

## Supported Session Types

| Type | Typical use |
| --- | --- |
| SSH | Unix/Linux servers, SFTP, tunnels, forwarding, and remote operations |
| Local Terminal | A shell and working directory on the local machine |
| Telnet | Legacy network equipment and lab systems |
| Serial | Routers, embedded boards, and other serial devices |
| RDP | Windows and other RDP-compatible remote desktops |
| VNC | VNC-compatible remote desktops with supported authentication modes |

Availability of platform integrations and protocol features depends on the
operating system, server configuration, and credentials supplied by the user.

## Build and Run

NyaTerm is a Cargo workspace using Rust 2024. Install a stable Rust toolchain
with `rustup`, then install the native development libraries required by GPUI,
PTY, serial, audio, and your platform's window system.

```bash
git clone https://github.com/nyakang/nyaterm.git
cd nyaterm
cargo run -p nyaterm-app --bin nyaterm
```

The default Cargo member is `nyaterm-app`. For a full workspace validation,
run:

```bash
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
bash scripts/check-architecture-boundaries.sh
bash scripts/check-icon-references.sh
```

The application opens a native GPUI window, so headless Linux environments
need a display service for a graphical smoke test. Platform-specific behavior
such as windowing, PTY, serial ports, clipboard, SSH Agent, RDP/VNC input, and
icon rendering must be verified on the affected operating system.

## Project Structure

```text
crates/
  nyaterm-app/             executable entry point, logging, and bundled assets
  nyaterm-core/            UI-independent models, formats, and policies
  nyaterm-desktop/         GPUI composition, feature state, views, and jobs
  nyaterm-store/           redb persistence, encryption, and compatibility I/O
  nyaterm-transport/       PTY, SSH, Telnet, Serial, SFTP, and tunnels
  nyaterm-terminal/        terminal state machine and protocol handling
  nyaterm-terminal-gpui/   GPUI terminal layout, input, and painting
  nyaterm-ui/              shared GPUI controls and theme integration
  nyaterm-remote-desktop/  UI-independent RDP/VNC runtime coordination
  nyaterm-rdp-helper/      isolated RDP helper process
  nyaterm-otp/             bundled HOTP/TOTP compatibility implementation
docs/                      architecture, compatibility, and asset policies
scripts/                   architecture, asset, and packaging checks
vendor/                    pinned or locally modified third-party dependencies
```

## Architecture and Compatibility

The native GPUI architecture is the project baseline. Each mutable UI domain
has one authoritative owner: a focused feature-state struct or a deliberately
authoritative GPUI Entity. Terminal parsing and protocol handling remain
independent of GPUI, and transport code remains independent of desktop views.
Do not reintroduce Tauri/WebView layers, snapshot-only stores, `#[path]` module
aliases, or broad feature prelude exports.

Existing configuration, credentials, backups, cloud-sync data, known hosts,
and session data are compatibility contracts. Changes to these areas must
preserve existing field names, keys, encryption behavior, and fallback readers,
and must include new-data round-trip and representative legacy-data tests.

Read [the GPUI migration status](docs/architecture/gpui-migration-status.md),
[component migration notes](docs/architecture/gpui-component-migration.md),
[remote-file backend notes](docs/architecture/remote-file-backends.md), and
[the third-party icon policy](docs/third-party-icons.md) before changing
module boundaries, persistence, or bundled assets.

## Contributing and Security

Contribution workflow and validation requirements are in
[CONTRIBUTING.md](CONTRIBUTING.md). Please report security issues privately as
described in [SECURITY.md](SECURITY.md); do not put credentials or diagnostic
payloads containing terminal data in public issues or pull requests.

## License

NyaTerm is distributed under the [Apache License, Version 2.0](LICENSE).

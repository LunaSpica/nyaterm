---
sidebar_position: 1
---

# Architecture

NyaTerm is a native Rust desktop application built with **GPUI**. Its interface, terminal emulator, connection transports, and persistence implementation live in one Cargo workspace without a browser runtime or an IPC bridge.

## Layers

```text
nyaterm-app
  └─ starts GPUI, registers assets, creates the root window
       └─ nyaterm-desktop
            ├─ AppShell / NyaTermApp / feature state / views
            ├─ nyaterm-ui                shared GPUI controls and theme
            ├─ nyaterm-terminal-gpui     terminal layout, input, and painting
            ├─ nyaterm-terminal          terminal state machine and snapshots
            ├─ nyaterm-transport         PTY, SSH, SFTP, and other protocols
            ├─ nyaterm-store             redb, transactions, compatibility readers
            └─ nyaterm-core              pure models, formats, and policies
```

The main responsibilities are:

| Crate | Responsibility |
|------|----------------|
| `nyaterm-app` | Executable entry point, logging, embedded assets, and root-window creation |
| `nyaterm-desktop` | GPUI composition, state, views, platform adapters, and background coordination |
| `nyaterm-ui` | Shared controls, theme tokens, and the `gpui-component` integration boundary |
| `nyaterm-terminal` | UI-independent terminal state, control sequences, encodings, and graphics protocols |
| `nyaterm-terminal-gpui` | GPUI terminal input, layout, selection, highlighting, images, and painting |
| `nyaterm-transport` | PTY, SSH, Telnet, Serial, SFTP, tunnels, remote operations, and transfer protocols |
| `nyaterm-store` | redb persistence, transactions, encryption adapters, and compatibility readers |
| `nyaterm-core` | Domain models, compatibility formats, parsing, policies, and pure logic |
| `nyaterm-otp` | HOTP/TOTP compatibility implementation |

## Startup

`crates/nyaterm-app/src/main.rs` is the application entry point:

1. Resolve runtime directories and initialize logging.
2. Register embedded assets and shared components with GPUI.
3. Create the native root window and an `AppShell` Entity.
4. Let `AppShell` start `StoreRuntime` and load the bootstrap snapshot asynchronously.
5. After validation succeeds, create `NyaTermApp`, then restore the window layout and sessions.

`AppShell` also owns application-level loading, recovery, and pre-exit flushing. A storage startup failure enters a recovery view instead of constructing the main application from unvalidated data.

## State ownership

`NyaTermApp` is the GPUI composition center, while focused feature-state structs own major UI domains such as connections, sessions, terminal presentation, transfers, settings, security, AI, sync, and remote operations.

Each value has one writable owner. The remaining independent Entity stores own only state that `NyaTermApp` does not own:

- `WindowRuntimeStore` for the window runtime pump
- `StartupRestoreStore` for the startup restore queue
- `OverlayStore` for the quick-switch overlay

Views read authoritative state directly when building GPUI elements. Do not introduce same-frame publish/read-back projections or keep independently mutable copies in both a feature state and an Entity.

## Background work and events

Filesystem, database, network, SSH, SFTP, subprocess, and image-decoding work does not run in render paths.

Background jobs return typed results or events to GPUI state. For example, session runtimes use `nyaterm_transport::SessionEvent` for output, working-directory changes, accepted commands, exits, and errors. The desktop window-runtime pump consumes those events, updates feature state, and notifies GPUI when a repaint is needed.

## Terminal data flow

```text
PTY / SSH / Telnet / Serial
        │
        ▼
nyaterm-transport typed events
        │
        ▼
nyaterm-desktop event drain and session state
        │
        ▼
nyaterm-terminal state machine and snapshots
        │
        ▼
nyaterm-terminal-gpui layout, input, and painting
```

`nyaterm-terminal` uses Alacritty terminal components for its grid and control-sequence state, while also owning UI-independent search, encoding, Kitty graphics, and Sixel behavior. GPUI sizing, keyboard adaptation, selection, highlighting, images, and per-frame painting remain in `nyaterm-terminal-gpui`.

## Persistence and compatibility

`nyaterm-store` executes database work through a dedicated `StoreRuntime`; the desktop submits typed requests through its UI or blocking clients. GPUI views never access redb directly.

Schema-neutral contracts such as configuration models, backup formats, cloud-sync documents, and encryption policies live in `nyaterm-core`. Database implementation and legacy-data readers live in `nyaterm-store`. Existing table names, keys, field names, encryption prefixes, `.nya` backups, and Dragonfly fallbacks are compatibility boundaries.

## Dependency rules

- `nyaterm-core`, `nyaterm-terminal`, and `nyaterm-transport` stay independent of GPUI.
- Desktop features use `nyaterm-ui` for ordinary inputs, selects, menus, switches, and dialogs.
- Modules use normal Rust module trees and explicit imports.
- New features prefer an existing focused feature state; add an authoritative Entity only when an independent lifecycle requires one.

See [GPUI Desktop Development](./frontend) for presentation rules and [Runtime, Transport, and Storage Development](./backend) for runtime and persistence guidance.

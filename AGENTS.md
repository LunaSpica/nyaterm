# Repository Guidelines

## Project Status

NyaTerm is a native GPUI application. The migration from the legacy
Tauri/WebView implementation is still incomplete, but a large part of the
desktop module tree and UI state has already been cleaned up.

Keep dynamic migration counts and long-form history in
`docs/architecture/gpui-migration-status.md`, not in this file. Update that
document when a migration boundary, debt count, or suggested order changes.

Preserve compatibility with existing NyaTerm configuration, credential, backup,
cloud-sync, known-host, and session data unless a change explicitly includes a
tested migration path.

## Workspace Structure

This is a Rust 2024 Cargo workspace using resolver `3`.

* `crates/nyaterm-app`: executable entry point, bundled assets, logging setup,
  and root window creation.
* `crates/nyaterm-core`: UI-independent domain models, compatibility formats,
  parsing, policies, AI settings/risk/provider logic, persistence
  implementation, and shared pure logic. Do not add GPUI dependencies here.
* `crates/nyaterm-desktop`: GPUI application composition, `AppShell`,
  `NyaTermApp`, feature state, views, platform adapters, background-job
  coordination, native HTTP adapters, and GPUI Entity stores.
* `crates/nyaterm-terminal`: terminal state machine, snapshots,
  control-sequence handling, encoding, and graphics protocols. It must remain
  UI-framework independent.
* `crates/nyaterm-terminal-gpui`: GPUI-specific terminal layout, input,
  highlighting, images, and painting.
* `crates/nyaterm-transport`: local PTY, SSH, Telnet, Serial, SFTP, tunnels,
  remote operations, and transfer-protocol runtime. It must remain independent
  of GPUI and desktop presentation types.
* `crates/nyaterm-ui`: shared GPUI theme tokens and reusable presentation
  widgets such as `TextField`.
* `crates/nyaterm-store`: transitional persistence facade. Storage
  implementation currently remains in `nyaterm-core`; do not expand the
  re-export-only boundary without either moving the implementation or updating
  consumers to use this crate.
* `crates/nyaterm-legacy`: migration and legacy-source compatibility only. Do
  not add new product features here.
* `crates/nyaterm-otp`: bundled OTP implementation used for HOTP/TOTP
  compatibility.
* `crates/nyaterm-app/assets`: bundled icon assets. `icons/**` is monochrome
  and painted with `svg()`/`mono_icon()`. `color/**` is full color and painted
  with `img()`/`color_icon()`. The two are not interchangeable; see
  `docs/third-party-icons.md`.
* `vendor/`: modified or pinned third-party dependencies such as `russh`,
  `russh-sftp`, and `zmodem2`.

Icon assets are vendored by `scripts/sync-icons.sh` from
`scripts/icons.manifest` and committed. Add or change icons through the
manifest rather than by hand.

## Current Architecture

`NyaTermApp` is still the central GPUI state owner, but it is no longer just one
flat migration bucket. Major transient UI domains are grouped into focused
feature-state structs, including connections, quick commands, remote ops,
security, settings interaction, AI, terminal presentation, send-command, and
transfers.

Persisted collections and compatibility-sensitive catalogs may still live on
`NyaTermApp` until an explicit ownership migration is tested. Examples include
saved connections, connection groups, SSH keys, OTP entries, saved
passwords/credentials, tunnels, proxies, quick commands, settings, cloud sync,
session metadata, and other public compatibility surfaces.

The remaining Entity stores own state the app does not:

* `RuntimeStore`: app runtime and native services.
* `WindowRuntimeStore`: window runtime pump.
* `StartupRestoreStore`: startup-restore queue.
* `OverlayStore`: authoritative quick-switch overlay state.

Do not reintroduce snapshot-only Entity stores or same-render publish/read-back
projections. If a domain moves to an Entity, make that Entity authoritative and
delete the old writable mirror in the same change.

The desktop and terminal-GPUI module trees no longer use `#[path = "..."]`.
Every directory should remain a normal Rust module tree with real `mod.rs`
files or sibling module declarations.

## Architectural Rules

For new or substantially changed features:

* Prefer a focused feature-state struct or a deliberately authoritative GPUI
  Entity over new top-level `NyaTermApp` fields.
* Keep one authoritative owner for each piece of state.
* Do not create independently mutable state in both `NyaTermApp`/FeatureState
  and an Entity Store.
* If a method only reads and writes one feature-state struct, consider moving
  that method onto the state and keeping the `NyaTermApp` method as the
  notifier/adapter when needed.
* Keep render helpers on views when they build GPUI elements; do not move view
  construction onto pure state just to reduce an `impl NyaTermApp` count.
* Use typed events for communication from background jobs to GPUI state.
* Never perform filesystem, database, network, SSH, SFTP, subprocess,
  image-decoding, or other blocking work in a render path or long-running GPUI
  update callback.
* Keep terminal parsing, snapshots, graphics protocol handling, and wire
  protocol logic in `nyaterm-terminal`.
* Keep GPUI terminal layout, input adapters, highlighting, images, and painting
  in `nyaterm-terminal-gpui`.
* Keep transport code independent of GPUI and desktop presentation types.
* Keep persistence compatibility logic out of GPUI views.

Do not add new `#[path = "..."]` declarations. This is especially strict in
`nyaterm-desktop` and `nyaterm-terminal-gpui`, where the debt is cleared and
guarded by `scripts/check-architecture-boundaries.sh`.

Do not add `use super::*` imports. The desktop debt is cleared and guarded
crate-wide; every module and test should name its dependencies explicitly.

Avoid broad exports from crate roots. The transitional `features/prelude.rs`
has been removed; do not recreate a shared feature prelude. Import models,
services, GPUI types and helpers from their authoritative modules.

## UI and Input Rules

Ordinary form, prompt, and search inputs should use `nyaterm-ui::TextField` and
the `nyaterm-core::TextEdit` editing model, either owned directly by a feature
or via the id-keyed registry in `features/text_inputs.rs`.

Do not mechanically replace full editing surfaces with single-line registry
fields. Terminal input, paste review, and `RemoteTextEditor` have dedicated
selection, undo, IME, and command-handling paths.

When adding text fields in GPUI views, make sure the wrapper has a definite
height and width. Avoid parent click handlers that immediately steal focus back
from the field.

## Persistence and Compatibility

Changes involving redb, credentials, known hosts, OTP, cloud sync, portable
snapshots, backup files, AI/translation secrets, or settings are
compatibility-sensitive.

Before changing these areas:

* Preserve existing table names, keys, serialized field names, document keys,
  encryption prefixes, master-key wrapping, backup formats, and fallback
  decryption behavior unless a migration is included.
* Test both new-data round trips and loading representative legacy data.
* Do not silently discard unknown or unsupported fields.
* Do not overwrite existing user data until validation succeeds.
* Keep secret-bearing values masked when returning settings to the UI.

`nyaterm-core/src/storage.rs` has been split by domain under
`nyaterm-core/src/storage/`, but the public facade and compatibility contract
still matter. Treat the `.nya` backup format, master-key wrapping, legacy
Dragonfly fallback, legacy text-document fallbacks, and existing redb data as
public compatibility contracts.

## Security Rules

Never log or commit:

* passwords or decrypted saved credentials;
* private-key contents or passphrases;
* OTP secrets or generated codes;
* AI, cloud-sync, OAuth, snippet, or translation API secrets;
* unredacted diagnostic payloads containing terminal or command context.

Implement custom redacted `Debug` output for secret-bearing structs when
debugging output is required. The architecture boundary script includes a
conservative guard against obvious secret-bearing `Debug` derives.

Changes under `vendor/` must document the upstream project/version or commit,
the reason for the local modification, and the validation performed.

## Build and Development Commands

Use package-specific checks while iterating:

* `cargo check -p nyaterm-app`
* `cargo test -p <crate-name>`
* `cargo run -p nyaterm-app --bin nyaterm`

Run architecture checks when touching module boundaries, state ownership,
feature preludes, quick switch, connection/network state, terminal-GPUI exports,
or secret-bearing structs:

* `bash scripts/check-architecture-boundaries.sh`

Before review, run the relevant broader checks:

* `cargo check --workspace`
* `cargo test --workspace`
* `cargo fmt --all -- --check`
* `cargo clippy --workspace --all-targets`

Use `cargo fmt --all` only when intentionally applying formatting changes.

Platform-specific GPUI, PTY, Serial, SSH, clipboard, window, path, and icon
rendering behavior must be verified on the affected operating system.

## Coding Style

Use standard `rustfmt` formatting and Rust 2024 idioms.

* `snake_case` for modules, functions, fields, and variables.
* `PascalCase` for structs, enums, and traits.
* `SCREAMING_SNAKE_CASE` for constants.
* Explicit imports instead of shared wildcard preludes.
* Typed models and errors instead of loosely structured strings.
* Small adapters at crate boundaries instead of importing application-wide
  models into low-level crates.
* Comments should explain invariants, compatibility constraints, ownership, or
  non-obvious performance decisions rather than restating code.

When splitting a large file, preserve the existing public facade where practical
so structural changes do not require unrelated call-site churn. Prefer domain
cuts that move constants, records, helpers, tests, and dependencies together
over type-only splits that leave coupling behind.

## Testing Guidelines

Add tests beside the behavior being changed.

* Terminal parsing, snapshots, graphics, selection, input, and rendering
  changes belong in `nyaterm-terminal` or `nyaterm-terminal-gpui`.
* SSH, SFTP, Telnet, Serial, tunnel, transfer, and session lifecycle changes
  belong in `nyaterm-transport`.
* Storage changes require round-trip and legacy-compatibility tests.
* Credential and encryption changes require success, invalid-password,
  corrupted-data, and legacy-format tests.
* GPUI state changes should test state transitions separately from visual
  rendering where possible.
Use descriptive behavior-oriented test names.

## Migration-Only Code

The legacy source inventory, migration dashboard, compatibility facades, and
temporary aliases must not become permanent homes for new functionality.

`migration-dashboard` is an explicit `nyaterm-desktop` feature. Default desktop
features are empty, so release/default builds must not depend on local legacy
source-tree paths such as `./temp/nyaterm-tauri`.

When a migration boundary is completed:

1. move consumers to the new authoritative API;
2. remove mirrored state or compatibility re-exports;
3. remove unused legacy code and dependencies;
4. update `docs/architecture/gpui-migration-status.md` and this file if the
   stable guidance changed.

## Commits and Pull Requests

Use Conventional Commit-style subjects:

`type(scope): imperative summary`

Common scopes include `terminal`, `transport`, `desktop`, `storage`, `ui`,
`ai`, and `sync`.

Pull requests should include:

* a concise description of the behavior and architectural impact;
* linked issues where applicable;
* commands and platforms tested;
* explicit notes for persistence, credential, migration, or vendored-dependency
  changes.

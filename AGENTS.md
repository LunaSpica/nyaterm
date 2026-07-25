# Repository Guidelines

## Project Status

NyaTerm is being migrated from the legacy Tauri/WebView implementation to a native GPUI application. The migration is incomplete, and some current boundaries are transitional rather than final.

Preserve compatibility with existing NyaTerm configuration, credential, backup, and session data unless a change explicitly includes a tested migration path.

## Workspace Structure

This is a Rust 2024 Cargo workspace.

* `crates/nyaterm-app`: executable entry point, bundled assets, logging setup, and root window creation.
* `crates/nyaterm-core`: UI-independent domain models, compatibility formats, parsing, policies, and shared pure logic. Do not add GPUI dependencies here.
* `crates/nyaterm-desktop`: GPUI application composition, feature state, views, platform adapters, and background-job coordination.
* `crates/nyaterm-terminal`: terminal state machine, snapshots, control-sequence handling, encoding, and graphics protocols. It must remain UI-framework independent.
* `crates/nyaterm-terminal-gpui`: GPUI-specific terminal layout, input, highlighting, and painting.
* `crates/nyaterm-transport`: local PTY, SSH, Telnet, Serial, SFTP, tunnels, remote operations, and transfer-protocol runtime.
* `crates/nyaterm-ui`: shared GPUI theme tokens and reusable presentation widgets.
* `crates/nyaterm-store`: transitional persistence façade. The implementation currently remains in `nyaterm-core`; do not expand the re-export-only boundary without either moving the implementation or updating consumers to use this crate.
* `crates/nyaterm-legacy`: migration and legacy-source compatibility only. Do not add new product features here.
* `crates/nyaterm-otp`: bundled OTP implementation used for HOTP/TOTP compatibility.
* `crates/nyaterm-app/assets`: application SVG assets.
* `vendor/`: modified or pinned third-party dependencies such as `russh`, `russh-sftp`, and `zmodem2`.

## Architectural Rules

The current `NyaTermApp` type is a transitional monolithic state owner. Do not increase its scope unnecessarily.

For new or substantially changed features:

* Prefer a focused feature-state struct or GPUI Entity over new top-level `NyaTermApp` fields.
* Keep one authoritative owner for each piece of state.
* Do not create independently mutable state in both `NyaTermApp` and an Entity Store.
* Treat existing Entity snapshots as projections until the corresponding feature has explicitly migrated ownership to the Entity.
* Use typed events for communication from background jobs to GPUI state.
* Never perform filesystem, database, network, SSH, SFTP, subprocess, image-decoding, or other blocking work in a render path or long-running GPUI update callback.
* Keep terminal parsing and snapshots in `nyaterm-terminal`; keep GPUI layout and painting in `nyaterm-terminal-gpui`.
* Keep transport code independent of GPUI and desktop presentation types.
* Keep persistence compatibility logic out of GPUI views.

Do not add new `#[path = "..."]` module declarations or new `use super::*` imports. Existing occurrences are migration debt and should be replaced with normal module declarations and explicit imports when the surrounding code is substantially edited.

Avoid adding broad exports to crate roots. Expose the smallest API required by consumers.

## Persistence and Compatibility

Changes involving redb, credentials, known hosts, OTP, cloud sync, portable snapshots, or settings are compatibility-sensitive.

Before changing these areas:

* Preserve existing table names, keys, serialized field names, encryption prefixes, and fallback decryption behavior unless a migration is included.
* Test both new-data round trips and loading representative legacy data.
* Do not silently discard unknown or unsupported fields.
* Do not overwrite existing user data until validation succeeds.
* Keep secret-bearing values masked when returning settings to the UI.

The `.nya` backup format, master-key wrapping, legacy Dragonfly fallback, and existing redb data must be treated as public compatibility contracts.

## Security Rules

Never log or commit:

* passwords or decrypted saved credentials;
* private-key contents or passphrases;
* OTP secrets or generated codes;
* AI, cloud-sync, OAuth, snippet, or translation API secrets;
* unredacted diagnostic payloads containing terminal or command context.

Implement custom redacted `Debug` output for secret-bearing structs when debugging output is required.

Changes under `vendor/` must document the upstream project/version or commit, the reason for the local modification, and the validation performed.

## Build and Development Commands

Use package-specific checks while iterating:

* `cargo check -p nyaterm-app`
* `cargo test -p <crate-name>`
* `cargo run -p nyaterm-app --bin nyaterm`

Before review, run the relevant broader checks:

* `cargo check --workspace`
* `cargo test --workspace`
* `cargo fmt --all -- --check`
* `cargo clippy --workspace --all-targets`

Use `cargo fmt --all` only when intentionally applying formatting changes.

Platform-specific GPUI, PTY, Serial, SSH, clipboard, window, and path behavior must be verified on the affected operating system.

## Coding Style

Use standard `rustfmt` formatting and Rust 2024 idioms.

* `snake_case` for modules, functions, fields, and variables.
* `PascalCase` for structs, enums, and traits.
* `SCREAMING_SNAKE_CASE` for constants.
* Explicit imports instead of shared wildcard preludes.
* Typed models and errors instead of loosely structured strings.
* Small adapters at crate boundaries instead of importing application-wide models into low-level crates.
* Comments should explain invariants, compatibility constraints, ownership, or non-obvious performance decisions rather than restating code.

When splitting a large file, preserve the existing public façade where practical so structural changes do not require unrelated call-site churn.

## Testing Guidelines

Add tests beside the behavior being changed.

* Terminal parsing, snapshots, graphics, selection, input, and rendering changes belong in `nyaterm-terminal` or `nyaterm-terminal-gpui`.
* SSH, SFTP, Telnet, Serial, tunnel, transfer, and session lifecycle changes belong in `nyaterm-transport`.
* Storage changes require round-trip and legacy-compatibility tests.
* Credential and encryption changes require success, invalid-password, corrupted-data, and legacy-format tests.
* GPUI state changes should test state transitions separately from visual rendering where possible.
* Visible GPUI changes should include screenshots or recordings in the pull request.

Use descriptive behavior-oriented test names.

## Migration-Only Code

The legacy source inventory, migration dashboard, compatibility façades, and temporary aliases must not become permanent homes for new functionality.

Migration-only code should be feature-gated or excluded from production builds when it depends on local source-tree paths such as `./temp/nyaterm-tauri`.

When a migration boundary is completed:

1. move consumers to the new authoritative API;
2. remove mirrored state or compatibility re-exports;
3. remove unused legacy code and dependencies;
4. update this file and the migration documentation.

## Commits and Pull Requests

Use Conventional Commit-style subjects:

`type(scope): imperative summary`

Common scopes include `terminal`, `transport`, `desktop`, `storage`, `ui`, `ai`, and `sync`.

Pull requests should include:

* a concise description of the behavior and architectural impact;
* linked issues where applicable;
* commands and platforms tested;
* screenshots or recordings for visible GPUI changes;
* explicit notes for persistence, credential, migration, or vendored-dependency changes.

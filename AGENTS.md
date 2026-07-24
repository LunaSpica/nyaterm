# Repository Guidelines

## Project Structure & Module Organization

This is a Rust 2024 Cargo workspace. Workspace members live under `crates/`, with the runnable binary in `crates/nyaterm-app` (`cargo run -p nyaterm-app --bin nyaterm`). Core behavior is split by concern: `nyaterm-core` for models/services, `nyaterm-desktop` for GPUI desktop features, `nyaterm-terminal` for terminal state/parsing, `nyaterm-terminal-gpui` for rendering/input glue, `nyaterm-transport` for transfer and connection helpers, and `nyaterm-ui` for shared UI pieces. App assets are in `crates/nyaterm-app/assets/`; root `assets/` contains repository-level assets. Vendored dependencies are in `vendor/` and are excluded from the workspace unless referenced by path.

## Build, Test, and Development Commands

- `cargo check --workspace`: fast compile check across all workspace crates.
- `cargo build --workspace`: build every workspace member.
- `cargo run -p nyaterm-app --bin nyaterm`: launch the desktop app locally.
- `cargo test --workspace`: run unit tests and integration tests for workspace crates.
- `cargo fmt --all`: apply standard Rust formatting.
- `cargo clippy --workspace --all-targets`: run Rust lints before review.

## Coding Style & Naming Conventions

Use standard `rustfmt` output and Rust 2024 idioms. Name crates and modules in kebab-case or snake_case as Cargo/Rust expects (`nyaterm-terminal-gpui`, `terminal_input_tracker.rs`). Use `snake_case` for functions and variables, `PascalCase` for types and traits, and `SCREAMING_SNAKE_CASE` for constants. Keep modules focused around existing boundaries rather than adding broad shared helpers prematurely. Prefer typed parsing and structured data over ad hoc string handling for protocol, terminal, and storage code.

## Testing Guidelines

Most tests are inline Rust unit tests under `#[cfg(test)] mod tests`; add new tests next to the behavior they cover. Use descriptive test names such as `parses_kitty_payload_chunks` or `refreshes_highlights_after_scroll`. For package-specific work, run `cargo test -p <crate-name>` first, then `cargo test --workspace` when the change touches shared types, rendering, storage, or transport behavior.

## Commit & Pull Request Guidelines

Recent history uses Conventional Commit-style subjects, for example `feat(terminal): share immutable snapshot rows`, `fix(terminal): refresh highlights after local scrolling`, and `perf(terminal): report snapshot row reuse`. Follow `type(scope): imperative summary`, using scopes like `terminal`, `transport`, `desktop`, or `ui`.

Pull requests should include a concise description, linked issue when applicable, commands run, and screenshots or recordings for visible GPUI changes. Call out migration, storage, credential, or vendored-dependency changes explicitly.

## Security & Configuration Tips

Do not commit local secrets, credentials, generated recordings, or machine-specific config. Treat cloud sync, SSH/SFTP, OTP, and credential autofill changes as security-sensitive; include tests or a clear manual verification note.

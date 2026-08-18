# NyaTerm Vendor Notes

Upstream: <https://github.com/longbridge/gpui-component>

Vendored version: `gpui-component` `0.5.2` at commit
`b1e78a515716b232a7d731cc092bdc25f3bfd787`, from upstream `main` on
2026-08-18. The previous NyaTerm snapshot was
`e1570bdc8fd2dc17d38cab09e74b1783bdf3b24b`.

Reason: NyaTerm uses gpui-component through the stable `nyaterm-ui` facade.
The complete upstream workspace is vendored, including the newer
`crates/base` and `crates/fps` members, while all GPUI types remain shared with
the sibling `vendor/zed` snapshot.

Local modifications and integration notes:

- Repointed `gpui`, `gpui_platform`, `gpui_web`, `gpui_macros`, and
  `reqwest_client` workspace dependencies to sibling paths under
  `vendor/zed`.
- Preserved the registry `zed-reqwest` dependency and feature strategy used by
  Zed's `reqwest_client` integration.
- Reapplied NyaTerm's segmented `TabBar` layout: the bar fills the available
  width and segmented tab wrappers use equal flexible widths without changing
  the public NyaTerm tab facade.
- Adapted the upstream Input/Textarea state split inside `nyaterm-ui`, keeping
  `NyaInput`, `NyaInputState::multi_line`, `NyaNumberInput`, selection, and
  other desktop call sites stable.
- Preserved ordinary-input focus when users click prefixes, suffixes, or the
  input shell instead of the text editing surface.
- Made the base dialog backdrop event wrapper fill the viewport so backdrop
  presses close the top dialog while continuing to block lower pointer events.
- Removed upstream `.git` metadata after vendoring.

Validation performed on 2026-08-18:

- `cargo tree -i gpui` reports one active `gpui v0.2.2`, from
  `vendor/zed/crates/gpui`.
- `cargo check --workspace` passed on `x86_64-unknown-linux-gnu`.
- Package tests passed: `nyaterm-ui` (41), `nyaterm-desktop` (932, 4 ignored),
  `nyaterm-terminal-gpui` (127, 1 ignored), and `nyaterm-remote-desktop` (17).
- Input focus, dialog backdrop behavior, and segmented tab facade tests passed.
- `cargo fmt --all -- --check` and
  `cargo clippy --workspace --all-targets` passed. Clippy reported existing
  non-fatal workspace warnings.
- The installed Windows GNU Rust target could not complete because the host
  lacks `x86_64-w64-mingw32-gcc`; it stopped while building `aws-lc-sys`.
- No macOS Rust target or macOS host is installed, so macOS compilation and
  runtime behavior remain part of the platform release matrix.
- `scripts/check-architecture-boundaries.sh` is referenced by repository
  documentation but is not present in this checkout, so that command could
  not be run.

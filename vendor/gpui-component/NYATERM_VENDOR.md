# NyaTerm Vendor Notes

Upstream: <https://github.com/longbridge/gpui-component>

Vendored version: `gpui-component` `0.5.2` at commit
`e1570bdc8fd2dc17d38cab09e74b1783bdf3b24b`.

Reason: `gpui-component` `0.5.2` contains the newer `NumberInput`
implementation with default numeric masking, centered text, and internal
step/min/max handling. NyaTerm uses it through `nyaterm-ui` while preserving
the desktop boundary that feature modules do not import `gpui_component`
directly.

Current status: wired into the root workspace via
`gpui-component = { path = "vendor/gpui-component/crates/ui" }`. Its workspace
dependencies for GPUI crates point at the sibling `vendor/zed` snapshot so
NyaTerm and `gpui-component` use the same active `gpui` types.

Local modifications:

- Repointed `gpui`, `gpui_platform`, `gpui_web`, `gpui_macros`, and
  `reqwest_client` workspace dependencies to the sibling `vendor/zed`
  snapshot.
- Kept the registry `zed-reqwest` dependency and features used by the Zed
  `reqwest_client` integration.
- Adjusted segmented `TabBar` layout so indicator bounds wrappers preserve
  full-width equal tab segments used by NyaTerm forms.
- Removed upstream `.git` metadata. All other source is the fixed upstream
  snapshot.

Validation performed on 2026-08-05:

- `cargo tree -i gpui` shows one `gpui v0.2.2`, from `vendor/zed`.
- `cargo test -p nyaterm-ui` passed (30 tests).
- `cargo test -p nyaterm-desktop` passed (812 tests).
- `cargo test -p nyaterm-terminal-gpui` passed (124 tests; 1 ignored).
- `cargo check -p nyaterm-app` passed.
- `bash scripts/check-architecture-boundaries.sh` passed.
- The Linux binary started and rendered the root view without crashing. Visual
  control checks were not possible because no display service was available.

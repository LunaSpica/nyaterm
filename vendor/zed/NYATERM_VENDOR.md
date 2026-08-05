# NyaTerm Vendor Notes

Upstream: <https://github.com/zed-industries/zed>

Vendored commit: `4aad57fd1f002f9feeea2b7fb6229ccbcd576cb1`, the
revision selected by the `gpui-component` lock file for commit
`e1570bdc8fd2dc17d38cab09e74b1783bdf3b24b`.

Reason: `gpui-component` `0.5.2` targets newer GPUI APIs than the registry
`gpui`/`gpui-component` pair previously used by NyaTerm. Vendoring the whole
Zed workspace keeps `gpui`, `gpui_platform`, `gpui_web`, `gpui_macros`, and
`reqwest_client` on one coherent upstream snapshot.

Local modifications:

- Removed the upstream `.git` metadata after vendoring.
- Preserved `livekit.yaml` and `crates/collab/.env.toml` from the prior NyaTerm
  snapshot as local non-code configuration files. No Zed Rust source patch is
  carried.

Validation performed on 2026-08-05:

- `cargo tree -i gpui` shows one `gpui v0.2.2`, from this snapshot.
- `cargo test -p nyaterm-ui` passed (30 tests).
- `cargo test -p nyaterm-desktop` passed (812 tests).
- `cargo test -p nyaterm-terminal-gpui` passed (124 tests; 1 ignored).
- `cargo check -p nyaterm-app` passed.
- `bash scripts/check-architecture-boundaries.sh` passed.
- The Linux binary started and rendered the root view without crashing. Visual
  control checks were not possible because no display service was available.

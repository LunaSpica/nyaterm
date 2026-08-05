# NyaTerm Vendor Notes

Upstream: <https://github.com/longbridge/gpui-component>

Vendored version: `gpui-component` `0.5.2` source snapshot, moved from
`temp/gpui-component` during the numeric input migration investigation.

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
  `reqwest_client` workspace dependencies to `../zed/crates/...`.
- Adjusted segmented `TabBar` layout so indicator bounds wrappers preserve
  full-width equal tab segments used by NyaTerm forms.

Validation performed:

- `cargo tree -i gpui`
- `cargo tree -d`
- `cargo test -p nyaterm-ui`
- `cargo test -p nyaterm-desktop`
- `cargo test -p nyaterm-terminal-gpui`
- `cargo check -p nyaterm-app`
- `bash scripts/check-architecture-boundaries.sh`

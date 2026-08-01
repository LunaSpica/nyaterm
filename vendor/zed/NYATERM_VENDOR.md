# NyaTerm Vendor Notes

Upstream: <https://github.com/zed-industries/zed>

Vendored commit: `5e1fd392f67e27fa1da91bad43eef7db1a5dec23`

Source: moved from `temp/zed` to `vendor/zed` during the GPUI stack upgrade
for `gpui-component` `0.5.2`.

Reason: `gpui-component` `0.5.2` targets newer GPUI APIs than the registry
`gpui`/`gpui-component` pair previously used by NyaTerm. Vendoring the whole
Zed workspace keeps `gpui`, `gpui_platform`, `gpui_web`, `gpui_macros`, and
`reqwest_client` on one coherent upstream snapshot.

Local modifications:

- Removed the upstream `.git` metadata after vendoring.

Validation performed:

- `cargo tree -i gpui`
- `cargo tree -d`
- `cargo test -p nyaterm-ui`
- `cargo test -p nyaterm-desktop`
- `cargo test -p nyaterm-terminal-gpui`
- `cargo check -p nyaterm-app`
- `bash scripts/check-architecture-boundaries.sh`

# NyaTerm IronRDP Client Vendor Note

- Upstream: `ironrdp-client` 0.1.0 from <https://github.com/Devolutions/IronRDP>
- Source: crates.io package `ironrdp-client-0.1.0`
- Local reason: NyaTerm needs dirty-region framebuffer output, explicit desktop reset/full-frame events, a non-reconnecting notification when Display Control is unavailable, and a certificate decision hook between TLS certificate retrieval and CredSSP credential submission. The upstream public API currently emits a full-screen allocation for every graphics update, reconnects to resize, and does not expose that trust-decision boundary.
- Scope: the patch changes only public input/output/configuration events and their emission in the existing connection and active-session loops. It does not fork the connector, CredSSP, graphics decoder, or active-stage protocol state machines.
- Validation: `cargo check -p nyaterm-rdp-helper` with the `rustls` and `clipboard` features, `cargo test -p nyaterm-remote-desktop`, helper lifecycle/clipboard tests, and NyaTerm workspace checks. Cross-platform Windows/macOS validation is tracked separately because this repository environment is Linux.

Certificate policy and the headless text-only clipboard backend remain NyaTerm-owned helper concerns. Keep future changes minimal and rebase them onto an upstream release when equivalent hooks are available.

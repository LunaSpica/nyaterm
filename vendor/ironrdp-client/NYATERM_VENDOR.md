# NyaTerm IronRDP Client Vendor Note

- Upstream: `ironrdp-client` 0.1.0 from <https://github.com/Devolutions/IronRDP>
- Source: crates.io package `ironrdp-client-0.1.0`
- Local reason: NyaTerm needs dirty-region framebuffer output, explicit desktop reset/full-frame events, a non-reconnecting notification when Display Control is unavailable, a certificate decision hook between TLS certificate retrieval and CredSSP credential submission, and correct FastPath decompressor replacement after deactivation/reactivation. The upstream public API currently emits a full-screen allocation for every graphics update, reconnects to resize, does not expose that trust-decision boundary, and does not rebuild the negotiated bulk decompressor on reactivation.
- Scope: the patch changes public input/output/configuration events and their emission in the existing connection and active-session loops, plus the minimum negotiated-compression helper used by the reactivation loop. It does not fork the connector, CredSSP, graphics decoder, or active-stage protocol state machines.
- Validation: `cargo check --workspace`, `cargo test --workspace`, `cargo clippy --workspace --all-targets`, the architecture boundary script, and the helper lifecycle/clipboard tests. Cross-platform Windows/macOS validation is tracked separately because this repository environment is Linux.

Certificate policy and the headless text-only clipboard backend remain NyaTerm-owned helper concerns. Keep future changes minimal and rebase them onto an upstream release when equivalent hooks are available.

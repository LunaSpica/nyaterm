# NyaTerm SSPI Vendor Note

- Upstream source: `sspi` 0.21.3 from <https://github.com/Devolutions/sspi-rs>, exposed as 0.21.0 to satisfy IronRDP 0.10's locked compatibility slot.
- Source: crates.io package `sspi-0.21.3`.
- Local reason: earlier 0.21 patch releases pin mutually incompatible prerelease RSA, Curve25519, Ed25519, and picky packages. The 0.21.3 source aligns RSA, and this manifest aligns picky plus the macOS Dalek crates with the stable dependency line used by NyaTerm.
- Patch: manifest dependency/version compatibility only. No SSPI Rust source is modified.
- Validation: workspace build on Linux plus required Windows/macOS helper build and NLA manual test before release.

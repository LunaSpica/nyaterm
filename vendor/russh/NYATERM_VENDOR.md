# NyaTerm russh vendor notes

Upstream: <https://github.com/warp-tech/russh>

Vendored version: `russh` 0.62.5 (`v0.62.5`) at commit
`4882af71cf27ea5293636bf4985ef296dcf20896`.

NyaTerm uses this complete source snapshot through the root workspace path
dependency. Upstream `.git` metadata is excluded.

Local modification:

- SSH name-list decoding accepts exactly one trailing comma for compatibility
  with servers that emit it. Empty name-lists remain valid, while leading,
  middle, single-comma and multiple-empty entries remain rejected. Unit tests
  cover each accepted and rejected form.
- The vendor workspace `Cargo.lock` is retained despite the upstream library
  ignore rule so NyaTerm's vendored validation resolves reproducibly.

Validation on 2026-08-05:

```text
cargo test --manifest-path vendor/russh/Cargo.toml -p russh --lib  # 159 passed
```

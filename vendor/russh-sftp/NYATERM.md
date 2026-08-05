# NyaTerm russh-sftp vendor notes

This directory is based on `russh-sftp` 2.4.0 from
<https://github.com/AspectUnk/russh-sftp> at commit
`e145c1f7ece99f41f558949ef59731f2cd1a9dfe` and is used through the workspace
path dependency in the root `Cargo.toml`.

NyaTerm carries compatibility changes first developed in the Tauri codebase,
including raw-byte remote paths and server limit handling. The GPUI port also
includes the request lifecycle and file-handle fixes from NyaTerm Tauri commits
`20c1db67de736b1fd907f62692239476a3dc526c` and
`bd8e0f75c49a215ef515edc8994c406b49b4f37d`:

- pending requests own their timeout and remove themselves when cancelled;
- stream failures wake all pending requests instead of leaving them to time out;
- late replies do not terminate the SFTP packet handler;
- dropped file handles are closed by a tracked background request instead of
  the upstream untracked `close_nowait` behavior;
- the upstream `File::close` API waits for pending writes and the remote close,
  while file shutdown remains idempotent and clears the closed handle;
- close responses, failures and timeouts release handle accounting without
  underflow;
- high-level reads and writes explicitly close remote handles.

NyaTerm also retains the compatibility APIs used by its transport boundary:
`File::read_at`, `SftpSession::symlink_openssh`, and server-limit accessors.
The vendor tests use the sibling patched `vendor/russh` path. The workspace
`Cargo.lock` is retained despite the upstream library ignore rule so vendored
validation resolves reproducibly. The upstream `.git` metadata is excluded
from the snapshot.

These changes prevent stalled writes and leaked server handles during uploads,
downloads, remote editing, cancellation, and error cleanup.

Validation on 2026-08-05:

```text
cargo test --manifest-path vendor/russh-sftp/Cargo.toml --lib  # 12 passed
cargo test -p nyaterm-transport                               # 147 passed
```

The SFTP service E2E test was skipped because `NYATERM_TEST_SFTP_*` and a
disposable remote directory were not configured.

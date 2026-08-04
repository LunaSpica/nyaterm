# NyaTerm russh-sftp vendor notes

This directory is based on `russh-sftp` 2.3.0 from
<https://github.com/AspectUnk/russh-sftp> and is used through the workspace
path dependency in the root `Cargo.toml`.

NyaTerm carries compatibility changes first developed in the Tauri codebase,
including raw-byte remote paths and server limit handling. The GPUI port also
includes the request lifecycle and file-handle fixes from NyaTerm Tauri commits
`20c1db67de736b1fd907f62692239476a3dc526c` and
`bd8e0f75c49a215ef515edc8994c406b49b4f37d`:

- pending requests own their timeout and remove themselves when cancelled;
- stream failures wake all pending requests instead of leaving them to time out;
- late replies do not terminate the SFTP packet handler;
- dropped file handles are closed by a tracked background request;
- file shutdown is idempotent and clears the closed handle;
- high-level reads and writes explicitly close remote handles.

These changes prevent stalled writes and leaked server handles during uploads,
downloads, remote editing, cancellation, and error cleanup.

Validation:

```text
cargo test --manifest-path vendor/russh-sftp/Cargo.toml --lib
cargo test -p nyaterm-transport
```

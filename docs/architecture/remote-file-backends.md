# Remote File Backends

This document defines the native GPUI remote-file backend contract. It records
behavioral parity with the former Tauri implementation without restoring a
WebView manager, a second browser UI, or Tauri command types.

## Scope

The existing GPUI browser, editor, properties dialog, transfer queue, duplicate
prompts, retry controls, and background-job model remain authoritative. Backend
selection is an implementation detail. The UI uses the terms "remote file" and
"remote transfer" and does not show a persistent backend badge.

The transport contract exposes `RemoteFileService`, `RemoteFileBackendKind`,
`RemoteFilePath`, `RemoteBinaryFile`, `FileTransferEndpoint`, `FileCopyRequest`,
and `FileCopySummary`. `SftpService` and the existing `Sftp*` models remain a
compatibility facade; string-path methods construct a tokenless
`RemoteFilePath` and delegate to the token-aware implementation where one is
available.

## Ownership And Boundaries

- `nyaterm-transport::remote_file` owns backend selection and the unified file
  operation contract. Its `shell` child owns SSH command execution and POSIX
  quoting. It has no dependency on GPUI, desktop models, or `nyaterm-core`.
- `nyaterm-transport::sftp` owns SFTP wire paths, raw path tokens, channel
  retry, recursive SFTP operations, and the compatibility facade.
- `nyaterm-core::ConnectionStore` owns the compatible backend preference
  document and redb transaction semantics.
- `nyaterm-desktop` implements `RemoteFileBackendPreferenceStore` as a narrow
  adapter around `ConnectionStore`. `SessionProtocolRuntimeState` lazily owns
  one cloneable `RemoteFileService` per session id.
- Disconnect, reconnect teardown, and catalog removal delete the session-owned
  service. A later session incarnation probes again. File and probe work runs
  on transfer worker threads, never in a GPUI render path.

## Selection State Machine

Selection is lazy and serialized within a service instance.

1. Read the cached backend for `host:port:username` and probe it once.
2. If the cached probe fails, probe the complete sequence from the beginning:
   SFTP, enhanced SCP, normal SCP.
3. Store and reuse the first successful backend for the lifetime of the
   session service.
4. If all probes fail, return the common remote-file-manager-unavailable error.
5. After selection, an operation error is returned directly. The service does
   not switch backend and does not replay a possibly mutating operation.

This deliberately means that a stale cached enhanced-SCP result is tried twice:
once as the cache hint and once at its position in the full sequence. Exact
ordering is covered by pure selection tests. `selected_backend()` is `None`
before successful selection and returns the selected kind afterward.

SFTP channel creation retries only channel-open failures classified as
`ConnectFailed` or `ResourceShortage`, with 50 ms, 150 ms, and 300 ms delays.
Authentication, host-key, protocol, and operation failures are not retried by
that channel policy.

## Backend Probes

| Backend | Probe requirements | Path model |
| --- | --- | --- |
| SFTP | Successful SFTP channel and directory listing | Raw byte path plus lossy display path |
| Enhanced SCP | POSIX shell, GNU `find -printf`, GNU `stat -c`, `tar`, `cat`, `mkdir`, `rm`, and `mv`; NUL-output behavior is verified | POSIX shell string |
| Normal SCP | POSIX `ls`, `cat`, `mkdir`, `rm`, and `mv` | POSIX shell string parsed from `ls` |

"SCP" here names the shell fallback family retained for compatibility. File
payloads use SSH session channels and the available POSIX tools; no external
local `scp` process is spawned.

## Capability Matrix

| Capability | SFTP | Enhanced SCP | Normal SCP |
| --- | :---: | :---: | :---: |
| Resolve home and list directories | Yes | Yes | Yes |
| Properties and symlink target-directory semantics | Yes | Yes | Yes |
| Create file/directory/symlink | Yes | Yes | Yes |
| Rename and guarded recursive delete | Yes | Yes | Yes |
| Recursive mode/owner/group update | Yes | Yes | Yes |
| Text read, binary read, conflict-aware atomic text save | Yes | Yes | Yes |
| File and directory upload/download | Yes | Yes | Yes |
| Duplicate policy, progress, cancel, timestamp/default-mode options | Yes | Yes, at operation boundaries | Yes, at operation boundaries |
| Non-UTF-8 remote path identity | Yes | No | No |
| Local/local, local/remote, remote/local, remote/remote copy | Yes | Yes | Yes |

SFTP recursive navigation, properties, rename, deletion, editing, external
editing, AI reads, and downloads use `RemoteFilePath`. Directory listings expose
a URL-safe unpadded Base64 `raw_path_token`; browser selection and editor tab
identity use the token when present, so colliding lossy display names remain
distinct. Session-local browser cache and navigation rollback retain the current
directory token. A symlink remains a symlink while
`symlink_target_is_directory` independently controls navigation and recursion.

UID and GID names are resolved in batches after SFTP listing. Numeric values
remain the fallback. Symbolic permissions preserve setuid, setgid, and sticky
bits.

Endpoint copy uses direct local filesystem copy for local/local. Remote source
paths retain raw SFTP identity. Two selected SFTP backends stream directly over
independent SFTP channels, including recursive directory copies and raw byte
names. Other cross-remote combinations use a session-scoped local staging
directory whose destructor removes it on success or error. The staging path is
never persisted.

## Cache Contract

The canonical document is stored in the settings table under
`settings/doc/file-backend-cache`. When it is absent, the reader accepts the
legacy text document key `file-backend-cache`. The document shape is:

```json
{
  "entries": {
    "host:22:username": {
      "last_working_backend": "sftp",
      "sftp_unavailable": false,
      "last_failure_reason": null,
      "updated_at": 0
    }
  }
}
```

Supported backend values are `sftp`, `scp_enhanced`, and `scp_normal`.
Unknown document and entry fields are preserved during updates. A malformed
canonical document fails the read/update and is not overwritten with defaults.
Entry updates use a redb write transaction, so concurrent writers serialize.

The cache is a local performance hint. It has no TTL, is not included in
portable backups or cloud-sync snapshots, and is never required for an
operation to succeed. Read failures trigger a full probe; write failures emit a
warning after successful selection and do not fail the file operation.

## Failure And Diagnostic Semantics

Probe and selection logs contain only endpoint key, backend kind, stage, and a
typed/error summary. File commands, remote paths, stdout, stderr, credentials,
and payloads are not deliberately logged. Existing diagnostic export obtains
selection information through the normal log pipeline.

Dangerous recursive delete targets such as empty paths, root, dot, dot-dot, and
paths containing a dot-dot component are rejected before dispatch. Atomic text
writes use a remote temporary sibling, validate the expected metadata/content
size, rename on success, and attempt cleanup on failure.

## Known Limits

- Both shell fallbacks require a POSIX-style remote. Network appliances, a
  restricted shell, or a non-POSIX host may have no usable backend.
- Normal SCP depends on POSIX `ls` output. It cannot represent every filename
  containing a newline and cannot promise non-UTF-8 identity.
- Shell payload execution currently collects an individual file in memory;
  pause/cancel checks occur before and after the SSH command rather than inside
  the remote byte stream. SFTP retains chunk-level pause, cancel, resume, and
  retry behavior.
- Remote-to-remote copy streams directly between two SFTP sessions when both
  selected backends are SFTP. Copies involving a shell backend use a controlled
  local staging directory and always remove it when the request completes.
- Creation beneath a directory whose name is not representable by its display
  encoding still uses the display child path. Existing token-identified entries
  are otherwise navigable and mutable through the operations listed above.

These limits are transport limitations, not a reason to add a second browser
surface. The GPUI layout intentionally remains single-browser; former Tauri
dual-pane, drag/drop, and cross-session clipboard UI are out of scope.

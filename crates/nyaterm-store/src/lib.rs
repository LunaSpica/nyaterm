//! Persistent-store boundary and single-owner runtime.

mod runtime;

pub use runtime::{
    BootstrapSnapshot, FlushBarrier, LoadBootstrap, RequestId, StoreBlockingClient, StoreConfig,
    StoreDomain, StoreEvent, StoreFnRequest, StoreOperationError, StoreRequest, StoreRuntime,
    StoreSubmitError, StoreTask, StoreUiClient, store_request,
};

pub use nyaterm_core::{
    ConfigBackupInfo, ConnectionStore, DiagnosticsError, DiagnosticsExportInfo,
    DiagnosticsExportOptions, DiagnosticsRuntimeSnapshot, KnownHostCheck, PortableSnapshotError,
    PortableSnapshotKind, PortableSnapshotMeta, RawPortableSnapshot, StorageError,
    decode_encrypted_raw_portable_snapshot, decode_raw_portable_snapshot,
    encode_encrypted_raw_portable_snapshot, encode_raw_portable_snapshot,
    export_diagnostics_archive,
};

#[cfg(test)]
mod tests {
    use super::{
        PortableSnapshotKind, RawPortableSnapshot, decode_raw_portable_snapshot,
        encode_raw_portable_snapshot,
    };

    #[test]
    fn raw_portable_snapshot_round_trips_through_store_boundary() {
        let mut snapshot = RawPortableSnapshot::backup("test-device", "test-version");
        snapshot
            .entities
            .insert("settings/default".into(), r#"{"theme":"dark"}"#.into());
        snapshot.recalculate_hash().expect("hash snapshot");

        let encoded = encode_raw_portable_snapshot(&snapshot).expect("encode snapshot");
        let decoded = decode_raw_portable_snapshot(&encoded).expect("decode snapshot");

        assert_eq!(decoded.meta.snapshot_kind, PortableSnapshotKind::Backup);
        assert_eq!(decoded.meta.device_id, "test-device");
        assert_eq!(decoded.entities, snapshot.entities);
    }
}

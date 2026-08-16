use super::{PortableSnapshotError, RawPortableSnapshot, validate_raw_snapshot};

#[test]
fn detects_payload_hash_mismatch() {
    let mut snapshot = RawPortableSnapshot::backup("device-1", "test");
    snapshot.recalculate_hash().expect("hash");
    snapshot
        .entities
        .insert("history".to_string(), "[{\"changed\":true}]".to_string());

    assert!(matches!(
        validate_raw_snapshot(&snapshot),
        Err(PortableSnapshotError::PayloadHashMismatch)
    ));
}

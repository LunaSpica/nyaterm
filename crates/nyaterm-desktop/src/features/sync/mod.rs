//! Cloud sync provider adapters and cloud sync runtime.

mod cloud_sync_provider;
mod cloud_sync_runtime;

pub(in crate::features) use cloud_sync_provider::{
    pull_provider_snapshot, push_provider_snapshot, test_provider_connection,
};

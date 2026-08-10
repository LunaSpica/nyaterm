//! Remote host operations: Docker, processes and stats runtime.

mod remote_runtime;
mod state;

pub(in crate::features) use state::{
    GpuPresentationState, NpuPresentationState, RemoteOpsFeatureFocus, RemoteOpsFeatureState,
};

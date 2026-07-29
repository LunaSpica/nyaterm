//! AI chat, agent loop, background jobs and AI settings runtime.

mod ai_agent_runtime;
mod ai_jobs;
mod ai_runtime;
mod state;

pub(in crate::features) use ai_jobs::{
    ai_active_profile_drafts, ai_usage_counts, is_agent_command_card,
};
pub(in crate::features) use state::{AiFeatureFocus, AiFeatureState, AiSettingsMutation};

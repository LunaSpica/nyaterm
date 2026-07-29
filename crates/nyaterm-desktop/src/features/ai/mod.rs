//! AI chat, agent loop, background jobs and AI settings runtime.

use std::time::Duration;

mod ai_agent_runtime;
mod ai_jobs;
mod ai_runtime;
mod state;

pub(in crate::features) use ai_jobs::{
    ai_active_profile_drafts, ai_usage_counts, is_agent_command_card,
};
pub(in crate::features) use state::{AiFeatureFocus, AiFeatureState, AiSettingsMutation};

const AGENT_OBSERVATION_MIN_WAIT: Duration = Duration::from_millis(700);
const AGENT_OBSERVATION_QUIET: Duration = Duration::from_millis(900);
const AGENT_DEFAULT_STEP_TIMEOUT: Duration = Duration::from_millis(30_000);

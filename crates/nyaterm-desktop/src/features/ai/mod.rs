//! AI chat, agent loop, background jobs and AI settings runtime.

use super::*;

mod ai_agent_runtime;
mod ai_jobs;
mod ai_runtime;

pub(in crate::features) use ai_jobs::{
    ai_active_profile_drafts, ai_job_cancelled, ai_usage_counts, is_agent_command_card,
    observation_summary, remote_command_observation, run_ai_ask_job,
};

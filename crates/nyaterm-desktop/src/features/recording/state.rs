use std::collections::HashMap;
use std::sync::Arc;

use nyaterm_transport::RecordingManager;

use crate::models::{RecordingPathPromptKind, RecordingWriteHandle, RecordingWritePipeline};

pub(in crate::features) struct RecordingFeatureState {
    pub manager: Arc<RecordingManager>,
    pub active_count: usize,
    pub pending_auto_start: Option<(String, String)>,
    pub pipeline: RecordingWritePipeline,
    pub search_draft: String,
    pub busy_actions: HashMap<String, String>,
    pub path_prompt: Option<RecordingPathPromptKind>,
}

impl RecordingFeatureState {
    pub(in crate::features) fn new(memory_limit_bytes: usize) -> Self {
        let manager = Arc::new(RecordingManager::new());
        manager.set_memory_limit(memory_limit_bytes);
        let pipeline = RecordingWritePipeline::spawn(Arc::clone(&manager));
        Self {
            manager,
            active_count: 0,
            pending_auto_start: None,
            pipeline,
            search_draft: String::new(),
            busy_actions: HashMap::new(),
            path_prompt: None,
        }
    }

    pub(in crate::features) fn writer(&self) -> RecordingWriteHandle {
        self.pipeline.writer()
    }

    pub(in crate::features) fn refresh_active_count(&mut self) {
        self.active_count = self.manager.list_recording_sessions().len();
    }

    pub(in crate::features) fn cleanup_session(&mut self, session_id: &str) {
        if self.manager.is_recording(session_id) {
            self.active_count = self.active_count.saturating_sub(1);
        }
        self.busy_actions.remove(session_id);
        self.pipeline.cleanup_session(session_id.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::RecordingFeatureState;

    #[test]
    fn recording_state_owns_runtime_and_session_cleanup_state() {
        let mut recording = RecordingFeatureState::new(1024);
        recording
            .busy_actions
            .insert("session-1".to_string(), "record".to_string());
        recording.pending_auto_start = Some(("session-1".to_string(), "local shell".to_string()));

        let _writer = recording.writer();
        recording.cleanup_session("session-1");

        assert_eq!(recording.active_count, 0);
        assert!(!recording.busy_actions.contains_key("session-1"));
        assert_eq!(
            recording
                .pending_auto_start
                .as_ref()
                .map(|value| value.0.as_str()),
            Some("session-1")
        );
        assert!(recording.path_prompt.is_none());
    }
}

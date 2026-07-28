use crate::features::NyaTermApp;

impl NyaTermApp {
    pub(in crate::features) fn sync_session_event_bridge_config(&self) {
        self.session_event_bridge.configure(
            self.settings.interaction_default_encoding.clone(),
            self.terminal_scrollback_line_limit(),
        );
    }

    pub(in crate::features) fn sync_session_event_bridge_routing(&self) {
        let mut session_ids = self.session_metadata.keys().cloned().collect::<Vec<_>>();
        if let Some(active_session_id) = self.active_session_id.clone()
            && !session_ids.contains(&active_session_id)
        {
            session_ids.push(active_session_id);
        }
        for session_id in session_ids {
            self.sync_session_event_bridge_session_policy(&session_id);
        }
    }

    pub(in crate::features) fn sync_session_event_bridge_policy(&self) {
        self.sync_session_event_bridge_config();
        self.sync_session_event_bridge_routing();
    }

    pub(in crate::features) fn sync_session_event_bridge_session_policy(&self, session_id: &str) {
        if self.session_has_active_ai_capture(session_id)
            || !self.session_sideband_detectors_idle(session_id)
        {
            self.session_event_bridge.route_session_to_ui(session_id);
        } else {
            self.session_event_bridge
                .resume_session_direct_output(session_id);
        }
    }
}

use std::collections::{HashSet, VecDeque};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::thread;
use std::time::Duration;

use nyaterm_transport::{
    SessionDrainStats, SessionEvent, SessionManager, TrzszDetector, ZmodemDetector,
};

use super::{TerminalFrameOutputSubmission, TerminalFramePipeline};

const SESSION_EVENT_BRIDGE_DRAIN_BATCH: usize = 512;
const SESSION_EVENT_BRIDGE_OUTPUT_BUDGET: usize = 128 * 1024;
const SESSION_EVENT_BRIDGE_IDLE_SLEEP: Duration = Duration::from_millis(4);
const SESSION_EVENT_BRIDGE_BUSY_SLEEP: Duration = Duration::from_millis(1);
const SESSION_EVENT_BRIDGE_UI_OUTPUT_LIMIT: usize = 1024 * 1024;
const SESSION_EVENT_BRIDGE_UI_OUTPUT_EVENT_LIMIT: usize = 128 * 1024;
const SESSION_EVENT_BRIDGE_DIRECT_OUTPUT_BACKPRESSURE: usize = 2 * 1024 * 1024;

#[derive(Clone, Debug, Default)]
pub(crate) struct SessionEventBridgeStats {
    pub(crate) direct_output_events: u64,
    pub(crate) direct_output_bytes: u64,
    pub(crate) direct_backpressure_events: u64,
    pub(crate) direct_backpressure_bytes: u64,
    pub(crate) drained_ui_events: usize,
    pub(crate) drained_ui_output_bytes: usize,
    pub(crate) ui_queued_events: usize,
    pub(crate) ui_queued_output_bytes: usize,
    pub(crate) source_queued_events: usize,
    pub(crate) source_queued_output_bytes: usize,
    pub(crate) dropped_output_bytes: usize,
}

#[derive(Debug, Default)]
pub(crate) struct SessionEventBridgeDrain {
    pub(crate) events: Vec<SessionEvent>,
    pub(crate) stats: SessionEventBridgeStats,
}

#[derive(Clone)]
pub(crate) struct SessionEventBridge {
    state: Arc<SessionEventBridgeState>,
}

struct SessionEventBridgeState {
    control: Mutex<SessionEventBridgeControl>,
    ui_queue: SessionEventBridgeQueue,
    direct_output_events: AtomicU64,
    direct_output_bytes: AtomicU64,
    direct_backpressure_events: AtomicU64,
    direct_backpressure_bytes: AtomicU64,
    stop: AtomicBool,
}

#[derive(Clone)]
struct SessionEventBridgeControl {
    force_ui_all_output: bool,
    ui_routed_sessions: HashSet<String>,
    encoding: String,
    scrollback_limit: usize,
    source_queued_events: usize,
    source_queued_output_bytes: usize,
}

#[derive(Clone)]
struct SessionEventBridgeControlSnapshot {
    force_ui_all_output: bool,
    ui_routed_sessions: HashSet<String>,
    encoding: String,
    scrollback_limit: usize,
}

#[derive(Clone)]
struct SessionEventBridgeQueue {
    inner: Arc<Mutex<SessionEventBridgeQueueInner>>,
}

#[derive(Default)]
struct SessionEventBridgeQueueInner {
    events: VecDeque<SessionEvent>,
    queued_output_bytes: usize,
}

impl SessionEventBridge {
    pub(crate) fn spawn(
        session_manager: Arc<SessionManager>,
        frame_pipeline: TerminalFramePipeline,
        encoding: String,
        scrollback_limit: usize,
    ) -> Self {
        let state = Arc::new(SessionEventBridgeState {
            control: Mutex::new(SessionEventBridgeControl {
                force_ui_all_output: false,
                ui_routed_sessions: HashSet::new(),
                encoding,
                scrollback_limit,
                source_queued_events: 0,
                source_queued_output_bytes: 0,
            }),
            ui_queue: SessionEventBridgeQueue::new(),
            direct_output_events: AtomicU64::new(0),
            direct_output_bytes: AtomicU64::new(0),
            direct_backpressure_events: AtomicU64::new(0),
            direct_backpressure_bytes: AtomicU64::new(0),
            stop: AtomicBool::new(false),
        });
        let worker_state = state.clone();
        thread::Builder::new()
            .name("nyaterm-session-event-bridge".to_string())
            .spawn(move || run_session_event_bridge(session_manager, frame_pipeline, worker_state))
            .expect("failed to spawn session event bridge");
        Self { state }
    }

    pub(crate) fn configure(
        &self,
        encoding: String,
        scrollback_limit: usize,
        force_ui_all_output: bool,
    ) {
        let Ok(mut control) = self.state.control.lock() else {
            return;
        };
        control.encoding = encoding;
        control.scrollback_limit = scrollback_limit;
        control.force_ui_all_output = force_ui_all_output;
    }

    pub(crate) fn route_session_to_ui(&self, session_id: &str) {
        if session_id.is_empty() {
            return;
        }
        if let Ok(mut control) = self.state.control.lock() {
            control.ui_routed_sessions.insert(session_id.to_string());
        }
    }

    pub(crate) fn resume_session_direct_output(&self, session_id: &str) {
        if let Ok(mut control) = self.state.control.lock() {
            control.ui_routed_sessions.remove(session_id);
        }
    }

    pub(crate) fn clear_session(&self, session_id: &str) {
        self.resume_session_direct_output(session_id);
    }

    pub(crate) fn drain_events_with_output_budget(
        &self,
        max_events: usize,
        max_output_bytes: usize,
    ) -> SessionEventBridgeDrain {
        let mut drain = self
            .state
            .ui_queue
            .drain_with_output_budget(max_events, max_output_bytes);
        drain.stats.direct_output_events =
            self.state.direct_output_events.swap(0, Ordering::Relaxed);
        drain.stats.direct_output_bytes = self.state.direct_output_bytes.swap(0, Ordering::Relaxed);
        drain.stats.direct_backpressure_events = self
            .state
            .direct_backpressure_events
            .swap(0, Ordering::Relaxed);
        drain.stats.direct_backpressure_bytes = self
            .state
            .direct_backpressure_bytes
            .swap(0, Ordering::Relaxed);
        if let Ok(control) = self.state.control.lock() {
            drain.stats.source_queued_events = control.source_queued_events;
            drain.stats.source_queued_output_bytes = control.source_queued_output_bytes;
        }
        drain
    }

    pub(crate) fn queued_event_count(&self) -> usize {
        self.state.ui_queue.len()
    }

    pub(crate) fn queued_output_bytes(&self) -> usize {
        self.state.ui_queue.queued_output_bytes()
    }
}

impl Drop for SessionEventBridge {
    fn drop(&mut self) {
        self.state.stop.store(true, Ordering::Relaxed);
    }
}

impl SessionEventBridgeState {
    fn control_snapshot(&self) -> Option<SessionEventBridgeControlSnapshot> {
        let control = self.control.lock().ok()?;
        Some(SessionEventBridgeControlSnapshot {
            force_ui_all_output: control.force_ui_all_output,
            ui_routed_sessions: control.ui_routed_sessions.clone(),
            encoding: control.encoding.clone(),
            scrollback_limit: control.scrollback_limit,
        })
    }

    fn update_source_stats(&self, stats: &SessionDrainStats) {
        if let Ok(mut control) = self.control.lock() {
            control.source_queued_events = stats.queued_events;
            control.source_queued_output_bytes = stats.queued_output_bytes;
        }
    }

    fn route_session_to_ui(&self, session_id: &str) {
        if let Ok(mut control) = self.control.lock() {
            control.ui_routed_sessions.insert(session_id.to_string());
        }
    }
}

impl SessionEventBridgeQueue {
    fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(SessionEventBridgeQueueInner::default())),
        }
    }

    fn push(&self, event: SessionEvent) {
        let Ok(mut inner) = self.inner.lock() else {
            return;
        };
        inner.push(event);
    }

    fn drain_with_output_budget(
        &self,
        max_events: usize,
        max_output_bytes: usize,
    ) -> SessionEventBridgeDrain {
        let Ok(mut inner) = self.inner.lock() else {
            return SessionEventBridgeDrain::default();
        };
        inner.drain(max_events, max_output_bytes)
    }

    fn len(&self) -> usize {
        self.inner
            .lock()
            .map(|inner| inner.events.len())
            .unwrap_or(0)
    }

    fn queued_output_bytes(&self) -> usize {
        self.inner
            .lock()
            .map(|inner| inner.queued_output_bytes)
            .unwrap_or(0)
    }
}

impl SessionEventBridgeQueueInner {
    fn push(&mut self, event: SessionEvent) {
        match event {
            SessionEvent::Output {
                session_id,
                mut data,
            } => {
                if data.is_empty() {
                    return;
                }
                let mut leading_drop = 0usize;
                if data.len() > SESSION_EVENT_BRIDGE_UI_OUTPUT_EVENT_LIMIT {
                    leading_drop = data.len() - SESSION_EVENT_BRIDGE_UI_OUTPUT_EVENT_LIMIT;
                    data.drain(..leading_drop);
                }
                if leading_drop > 0 {
                    self.push_output_drop_event(session_id.clone(), leading_drop);
                }
                self.queued_output_bytes = self.queued_output_bytes.saturating_add(data.len());
                self.events
                    .push_back(SessionEvent::Output { session_id, data });
                self.enforce_output_limit();
            }
            other => self.events.push_back(other),
        }
    }

    fn push_output_drop_event(&mut self, session_id: String, bytes: usize) {
        if bytes == 0 {
            return;
        }
        if let Some(SessionEvent::OutputDropped {
            session_id: last_session_id,
            bytes: last_bytes,
        }) = self.events.back_mut()
            && last_session_id == &session_id
        {
            *last_bytes = last_bytes.saturating_add(bytes);
            return;
        }
        self.events
            .push_back(SessionEvent::OutputDropped { session_id, bytes });
    }

    fn insert_output_drop_event(&mut self, index: usize, session_id: String, bytes: usize) {
        if bytes == 0 {
            return;
        }
        if index > 0
            && let Some(SessionEvent::OutputDropped {
                session_id: previous_session_id,
                bytes: previous_bytes,
            }) = self.events.get_mut(index - 1)
            && previous_session_id == &session_id
        {
            *previous_bytes = previous_bytes.saturating_add(bytes);
            return;
        }
        self.events.insert(
            index.min(self.events.len()),
            SessionEvent::OutputDropped { session_id, bytes },
        );
    }

    fn enforce_output_limit(&mut self) {
        while self.queued_output_bytes > SESSION_EVENT_BRIDGE_UI_OUTPUT_LIMIT {
            let excess = self.queued_output_bytes - SESSION_EVENT_BRIDGE_UI_OUTPUT_LIMIT;
            let Some(index) = self
                .events
                .iter()
                .position(|event| matches!(event, SessionEvent::Output { .. }))
            else {
                self.queued_output_bytes = 0;
                break;
            };
            let mut remove_event = false;
            let mut dropped = None;
            if let Some(SessionEvent::Output { session_id, data }) = self.events.get_mut(index) {
                let remove = excess.min(data.len());
                let dropped_session_id = session_id.clone();
                data.drain(..remove);
                self.queued_output_bytes = self.queued_output_bytes.saturating_sub(remove);
                remove_event = data.is_empty();
                dropped = Some((dropped_session_id, remove));
            }
            if let Some((session_id, bytes)) = dropped {
                if remove_event {
                    self.events.remove(index);
                }
                self.insert_output_drop_event(index, session_id, bytes);
            }
        }
    }

    fn drain(&mut self, max_events: usize, max_output_bytes: usize) -> SessionEventBridgeDrain {
        let mut events = Vec::new();
        let mut stats = SessionEventBridgeStats::default();
        let mut drained_events = 0usize;
        let mut drained_output_bytes = 0usize;
        for _ in 0..max_events {
            if drained_output_bytes >= max_output_bytes && drained_events > 0 {
                break;
            }
            let remaining_output_budget = max_output_bytes.saturating_sub(drained_output_bytes);
            if remaining_output_budget == 0
                && drained_events > 0
                && matches!(self.events.front(), Some(SessionEvent::Output { .. }))
            {
                break;
            }
            if let Some(SessionEvent::Output { session_id, data }) = self.events.front_mut() {
                let take = data.len().min(remaining_output_budget);
                if data.len() > take {
                    let remaining = data.split_off(take);
                    let chunk = std::mem::replace(data, remaining);
                    let session_id = session_id.clone();
                    self.queued_output_bytes = self.queued_output_bytes.saturating_sub(chunk.len());
                    drained_events = drained_events.saturating_add(1);
                    drained_output_bytes = drained_output_bytes.saturating_add(chunk.len());
                    events.push(SessionEvent::Output {
                        session_id,
                        data: chunk,
                    });
                    continue;
                }
            }
            let Some(event) = self.events.pop_front() else {
                break;
            };
            drained_events = drained_events.saturating_add(1);
            match &event {
                SessionEvent::Output { data, .. } => {
                    drained_output_bytes = drained_output_bytes.saturating_add(data.len());
                    self.queued_output_bytes = self.queued_output_bytes.saturating_sub(data.len());
                }
                SessionEvent::OutputDropped { bytes, .. } => {
                    stats.dropped_output_bytes = stats.dropped_output_bytes.saturating_add(*bytes);
                }
                SessionEvent::Exited { .. } | SessionEvent::Error { .. } => {}
            }
            events.push(event);
        }
        stats.drained_ui_events = drained_events;
        stats.drained_ui_output_bytes = drained_output_bytes;
        stats.ui_queued_events = self.events.len();
        stats.ui_queued_output_bytes = self.queued_output_bytes;
        SessionEventBridgeDrain { events, stats }
    }
}

fn run_session_event_bridge(
    session_manager: Arc<SessionManager>,
    frame_pipeline: TerminalFramePipeline,
    state: Arc<SessionEventBridgeState>,
) {
    while !state.stop.load(Ordering::Relaxed) {
        let Ok(drain) = session_manager.drain_events_with_output_budget(
            SESSION_EVENT_BRIDGE_DRAIN_BATCH,
            SESSION_EVENT_BRIDGE_OUTPUT_BUDGET,
        ) else {
            thread::sleep(SESSION_EVENT_BRIDGE_IDLE_SLEEP);
            continue;
        };
        state.update_source_stats(&drain.stats);
        if drain.events.is_empty() {
            thread::sleep(
                if drain.stats.queued_events > 0 || drain.stats.queued_output_bytes > 0 {
                    SESSION_EVENT_BRIDGE_BUSY_SLEEP
                } else {
                    SESSION_EVENT_BRIDGE_IDLE_SLEEP
                },
            );
            continue;
        }
        let Some(control) = state.control_snapshot() else {
            thread::sleep(SESSION_EVENT_BRIDGE_IDLE_SLEEP);
            continue;
        };
        let mut pending_direct_outputs = Vec::new();
        for event in drain.events {
            match event {
                SessionEvent::Output { session_id, data } => {
                    let frame_queued_output_bytes = frame_pipeline.queued_output_bytes();
                    if bridge_output_can_go_direct(
                        &control,
                        frame_queued_output_bytes,
                        &session_id,
                        &data,
                    ) {
                        state.direct_output_events.fetch_add(1, Ordering::Relaxed);
                        state
                            .direct_output_bytes
                            .fetch_add(data.len() as u64, Ordering::Relaxed);
                        pending_direct_outputs.push(TerminalFrameOutputSubmission {
                            session_id,
                            data,
                            encoding: control.encoding.clone(),
                            scrollback_limit: control.scrollback_limit,
                        });
                    } else {
                        if bridge_output_is_backpressured(
                            frame_queued_output_bytes,
                            &control,
                            &session_id,
                            &data,
                        ) {
                            state
                                .direct_backpressure_events
                                .fetch_add(1, Ordering::Relaxed);
                            state
                                .direct_backpressure_bytes
                                .fetch_add(data.len() as u64, Ordering::Relaxed);
                        }
                        flush_bridge_direct_outputs(&frame_pipeline, &mut pending_direct_outputs);
                        if bridge_output_may_contain_sideband_trigger(&data) {
                            state.route_session_to_ui(&session_id);
                        }
                        state
                            .ui_queue
                            .push(SessionEvent::Output { session_id, data });
                    }
                }
                SessionEvent::OutputDropped { session_id, bytes } => {
                    flush_bridge_direct_outputs(&frame_pipeline, &mut pending_direct_outputs);
                    state.route_session_to_ui(&session_id);
                    state
                        .ui_queue
                        .push(SessionEvent::OutputDropped { session_id, bytes });
                }
                SessionEvent::Exited { session_id } => {
                    flush_bridge_direct_outputs(&frame_pipeline, &mut pending_direct_outputs);
                    state.ui_queue.push(SessionEvent::Exited { session_id });
                }
                SessionEvent::Error {
                    session_id,
                    message,
                } => {
                    flush_bridge_direct_outputs(&frame_pipeline, &mut pending_direct_outputs);
                    state.ui_queue.push(SessionEvent::Error {
                        session_id,
                        message,
                    });
                }
            }
        }
        flush_bridge_direct_outputs(&frame_pipeline, &mut pending_direct_outputs);
    }
}

fn flush_bridge_direct_outputs(
    frame_pipeline: &TerminalFramePipeline,
    pending_outputs: &mut Vec<TerminalFrameOutputSubmission>,
) {
    if !pending_outputs.is_empty() {
        frame_pipeline.submit_outputs(std::mem::take(pending_outputs));
    }
}

fn bridge_output_can_go_direct(
    control: &SessionEventBridgeControlSnapshot,
    frame_pipeline_queued_output_bytes: usize,
    session_id: &str,
    data: &[u8],
) -> bool {
    !control.force_ui_all_output
        && !control.ui_routed_sessions.contains(session_id)
        && frame_pipeline_queued_output_bytes < SESSION_EVENT_BRIDGE_DIRECT_OUTPUT_BACKPRESSURE
        && !bridge_output_may_contain_sideband_trigger(data)
}

fn bridge_output_is_backpressured(
    frame_pipeline_queued_output_bytes: usize,
    control: &SessionEventBridgeControlSnapshot,
    session_id: &str,
    data: &[u8],
) -> bool {
    !control.force_ui_all_output
        && !control.ui_routed_sessions.contains(session_id)
        && frame_pipeline_queued_output_bytes >= SESSION_EVENT_BRIDGE_DIRECT_OUTPUT_BACKPRESSURE
        && !bridge_output_may_contain_sideband_trigger(data)
}

fn bridge_output_may_contain_sideband_trigger(data: &[u8]) -> bool {
    ZmodemDetector::output_may_contain_trigger(data)
        || TrzszDetector::output_may_contain_trigger(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_direct_policy_rejects_sideband_triggers() {
        let control = SessionEventBridgeControlSnapshot {
            force_ui_all_output: false,
            ui_routed_sessions: HashSet::new(),
            encoding: "UTF-8".to_string(),
            scrollback_limit: 1000,
        };
        assert!(bridge_output_can_go_direct(&control, 0, "s1", b"hello\n"));
        assert!(!bridge_output_can_go_direct(&control, 0, "s1", b"**\x18B"));
        assert!(!bridge_output_can_go_direct(&control, 0, "s1", b"::TRZSZ:"));
    }

    #[test]
    fn bridge_direct_policy_honors_ui_routed_sessions() {
        let mut routed = HashSet::new();
        routed.insert("s1".to_string());
        let control = SessionEventBridgeControlSnapshot {
            force_ui_all_output: false,
            ui_routed_sessions: routed,
            encoding: "UTF-8".to_string(),
            scrollback_limit: 1000,
        };
        assert!(!bridge_output_can_go_direct(&control, 0, "s1", b"hello\n"));
        assert!(bridge_output_can_go_direct(&control, 0, "s2", b"hello\n"));
    }

    #[test]
    fn bridge_direct_policy_yields_under_frame_backpressure() {
        let control = SessionEventBridgeControlSnapshot {
            force_ui_all_output: false,
            ui_routed_sessions: HashSet::new(),
            encoding: "UTF-8".to_string(),
            scrollback_limit: 1000,
        };

        assert!(bridge_output_can_go_direct(
            &control,
            SESSION_EVENT_BRIDGE_DIRECT_OUTPUT_BACKPRESSURE - 1,
            "s1",
            b"hello\n"
        ));
        assert!(!bridge_output_can_go_direct(
            &control,
            SESSION_EVENT_BRIDGE_DIRECT_OUTPUT_BACKPRESSURE,
            "s1",
            b"hello\n"
        ));
        assert!(bridge_output_is_backpressured(
            SESSION_EVENT_BRIDGE_DIRECT_OUTPUT_BACKPRESSURE,
            &control,
            "s1",
            b"hello\n"
        ));
    }
}

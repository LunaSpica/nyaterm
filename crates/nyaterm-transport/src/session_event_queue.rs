use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use super::{SessionDrain, SessionDrainStats, SessionEvent};

pub(super) const SESSION_EVENT_QUEUE_OUTPUT_LIMIT: usize = 8 * 1024 * 1024;
pub(super) const SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT: usize = 256 * 1024;

#[derive(Clone)]
pub(super) struct SessionEventQueue {
    shared: Arc<SessionEventQueueShared>,
}

struct SessionEventQueueShared {
    inner: Mutex<SessionEventQueueInner>,
    /// Signalled on every push so a consumer can park instead of polling.
    ready: Condvar,
}

#[derive(Default)]
struct SessionEventQueueInner {
    events: VecDeque<SessionEvent>,
    queued_output_bytes: usize,
}

impl SessionEventQueue {
    pub(super) fn new() -> Self {
        Self {
            shared: Arc::new(SessionEventQueueShared {
                inner: Mutex::new(SessionEventQueueInner::default()),
                ready: Condvar::new(),
            }),
        }
    }

    pub(super) fn push(&self, event: SessionEvent) {
        let Ok(mut inner) = self.shared.inner.lock() else {
            return;
        };
        inner.push(event);
        drop(inner);
        self.shared.ready.notify_one();
    }

    pub(super) fn drain(&self, max_events: usize) -> SessionDrain {
        self.drain_with_output_budget(max_events, None)
    }

    pub(super) fn drain_with_output_budget(
        &self,
        max_events: usize,
        max_output_bytes: Option<usize>,
    ) -> SessionDrain {
        let Ok(mut inner) = self.shared.inner.lock() else {
            return SessionDrain::default();
        };
        inner.drain(max_events, max_output_bytes)
    }

    /// Drain, parking up to `timeout` for the first event rather than returning
    /// empty. Lets a dedicated consumer thread wake on the producer's push
    /// instead of sleeping on a fixed interval and eating the latency.
    ///
    /// The timeout still bounds the park so a caller keeps its shutdown flag
    /// and periodic bookkeeping on schedule.
    pub(super) fn drain_blocking_with_output_budget(
        &self,
        max_events: usize,
        max_output_bytes: Option<usize>,
        timeout: Duration,
    ) -> SessionDrain {
        let Ok(mut inner) = self.shared.inner.lock() else {
            return SessionDrain::default();
        };
        if inner.events.is_empty() {
            let Ok((waited, _)) = self.shared.ready.wait_timeout(inner, timeout) else {
                return SessionDrain::default();
            };
            inner = waited;
        }
        inner.drain(max_events, max_output_bytes)
    }
}

impl SessionEventQueueInner {
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
                if data.len() > SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT {
                    leading_drop = data.len() - SESSION_EVENT_QUEUE_OUTPUT_EVENT_LIMIT;
                    data.drain(..leading_drop);
                }
                if leading_drop > 0 {
                    self.push_output_drop_event(session_id.clone(), leading_drop);
                }
                self.queued_output_bytes = self.queued_output_bytes.saturating_add(data.len());
                self.events.push_back(SessionEvent::Output {
                    session_id: session_id.clone(),
                    data,
                });
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
        let index = index.min(self.events.len());
        self.events
            .insert(index, SessionEvent::OutputDropped { session_id, bytes });
    }

    fn enforce_output_limit(&mut self) {
        while self.queued_output_bytes > SESSION_EVENT_QUEUE_OUTPUT_LIMIT {
            let excess = self.queued_output_bytes - SESSION_EVENT_QUEUE_OUTPUT_LIMIT;
            let Some(index) = self
                .events
                .iter()
                .position(|event| matches!(event, SessionEvent::Output { .. }))
            else {
                self.queued_output_bytes = 0;
                break;
            };
            let mut remove_event = false;
            let mut dropped: Option<(String, usize)> = None;
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
                    self.insert_output_drop_event(index, session_id, bytes);
                } else {
                    self.insert_output_drop_event(index, session_id, bytes);
                }
            } else if remove_event {
                self.events.remove(index);
            }
        }
    }

    fn drain(&mut self, max_events: usize, max_output_bytes: Option<usize>) -> SessionDrain {
        let mut events = Vec::new();
        let mut stats = SessionDrainStats::default();
        for _ in 0..max_events {
            if let Some(max_output_bytes) = max_output_bytes {
                if stats.drained_output_bytes >= max_output_bytes
                    && stats.drained_events > 0
                    && matches!(self.events.front(), Some(SessionEvent::Output { .. }))
                {
                    break;
                }
                let remaining_output_budget =
                    max_output_bytes.saturating_sub(stats.drained_output_bytes);
                if remaining_output_budget == 0
                    && stats.drained_events > 0
                    && matches!(self.events.front(), Some(SessionEvent::Output { .. }))
                {
                    break;
                }
                if remaining_output_budget == 0
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
                        stats.drained_events = stats.drained_events.saturating_add(1);
                        stats.drained_output_bytes =
                            stats.drained_output_bytes.saturating_add(chunk.len());
                        self.queued_output_bytes =
                            self.queued_output_bytes.saturating_sub(chunk.len());
                        events.push(SessionEvent::Output {
                            session_id,
                            data: chunk,
                        });
                        continue;
                    }
                }
            }
            let Some(event) = self.events.pop_front() else {
                break;
            };
            stats.drained_events = stats.drained_events.saturating_add(1);
            match &event {
                SessionEvent::Output { data, .. } => {
                    stats.drained_output_bytes =
                        stats.drained_output_bytes.saturating_add(data.len());
                    self.queued_output_bytes = self.queued_output_bytes.saturating_sub(data.len());
                }
                SessionEvent::OutputDropped { bytes, .. } => {
                    stats.dropped_output_bytes = stats.dropped_output_bytes.saturating_add(*bytes);
                }
                SessionEvent::CwdChanged { .. }
                | SessionEvent::CommandAccepted { .. }
                | SessionEvent::Exited { .. }
                | SessionEvent::Error { .. } => {}
            }
            events.push(event);
        }
        stats.queued_events = self.events.len();
        stats.queued_output_bytes = self.queued_output_bytes;
        SessionDrain { events, stats }
    }
}

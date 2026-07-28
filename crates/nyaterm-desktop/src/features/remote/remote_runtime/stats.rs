use gpui::{Context, Window};
use nyaterm_transport::RemoteStatsService;

use crate::features::NyaTermApp;
use crate::features::runtime_jobs::StatsJobResult;

const STATS_EVENT_DRAIN_LIMIT: usize = 8;

impl NyaTermApp {
    pub(in crate::features) fn refresh_stats(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.session.active_ssh_config.clone() else {
            self.remote_ops.stats.status =
                "start an SSH session before inspecting stats".to_string();
            self.terminal.view.status = self.remote_ops.stats.status.clone();
            cx.notify();
            return;
        };
        let Some(job_session_id) = self.session.active_id.clone() else {
            self.remote_ops.stats.status =
                "start an SSH session before inspecting stats".to_string();
            cx.notify();
            return;
        };
        if self.remote_ops.stats.is_pending_for(&job_session_id) {
            self.remote_ops.stats.status = "stats refresh already running".to_string();
            cx.notify();
            return;
        }

        let ticket = self.remote_ops.stats.begin_job(job_session_id.clone());
        self.remote_ops.stats.mark_refresh_started();
        self.remote_ops.stats.status = "loading remote system stats".to_string();
        std::thread::spawn(move || {
            let result = RemoteStatsService::new(config)
                .snapshot()
                .map_err(|error| error.to_string());
            let _ = ticket.tx.send(StatsJobResult {
                job_id: ticket.job_id,
                session_id: job_session_id,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn toggle_stats_cpu_expanded(&mut self, cx: &mut Context<Self>) {
        self.remote_ops.stats.toggle_cpu_expanded();
        cx.notify();
    }

    pub(in crate::features) fn drain_stats_events(&mut self) -> bool {
        let mut dirty = false;
        for _ in 0..STATS_EVENT_DRAIN_LIMIT {
            let Some(event) = self.remote_ops.stats.next_event() else {
                break;
            };
            if !self
                .remote_ops
                .stats
                .complete_event(event.job_id, &event.session_id)
            {
                continue;
            }
            dirty = true;
            if self.session.active_id.as_deref() != Some(event.session_id.as_str()) {
                continue;
            }
            match event.result {
                Ok(stats) => {
                    self.remote_ops.stats.reset_refresh_failures();
                    self.remote_ops.stats.status = format!(
                        "loaded stats for {} · load {:.2}/{:.2}/{:.2}",
                        if stats.system.hostname.trim().is_empty() {
                            "remote host"
                        } else {
                            stats.system.hostname.as_str()
                        },
                        stats.load.load1,
                        stats.load.load5,
                        stats.load.load15
                    );
                    self.terminal.view.status = self.remote_ops.stats.status.clone();
                    // The snapshot is the only place the remote OS is reported,
                    // so this is where a connection's icon can be filled in.
                    self.apply_auto_detected_connection_icon(&event.session_id, &stats.system);
                    self.remote_ops.stats.data = Some(stats);
                }
                Err(error) => {
                    if self.remote_ops.stats.record_refresh_failure() >= 3 {
                        self.remote_ops.stats.data = None;
                    }
                    self.remote_ops.stats.status = format!("stats refresh failed: {error}");
                    self.terminal.view.status = self.remote_ops.stats.status.clone();
                }
            }
        }
        dirty
    }
}

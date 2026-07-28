use std::time::Instant;

use gpui::{Context, Window};
use nyaterm_transport::RemoteStatsService;

use crate::features::NyaTermApp;
use crate::features::runtime_jobs::{StatsJobResult, remote_job_event_matches};

const STATS_EVENT_DRAIN_LIMIT: usize = 8;

impl NyaTermApp {
    pub(in crate::features) fn refresh_stats(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.remote_ops.stats.status =
                "start an SSH session before inspecting stats".to_string();
            self.terminal.view.status = self.remote_ops.stats.status.clone();
            cx.notify();
            return;
        };
        let Some(job_session_id) = self.active_session_id.clone() else {
            self.remote_ops.stats.status =
                "start an SSH session before inspecting stats".to_string();
            cx.notify();
            return;
        };
        if self.remote_ops.stats.pending
            && self.remote_ops.stats.job_session_id.as_deref() == Some(job_session_id.as_str())
        {
            self.remote_ops.stats.status = "stats refresh already running".to_string();
            cx.notify();
            return;
        }

        let job_id = self.begin_stats_job(job_session_id.clone());
        self.remote_ops.stats.last_refresh_at = Some(Instant::now());
        self.remote_ops.stats.status = "loading remote system stats".to_string();
        let tx = self.remote_ops.stats.tx.clone();
        std::thread::spawn(move || {
            let result = RemoteStatsService::new(config)
                .snapshot()
                .map_err(|error| error.to_string());
            let _ = tx.send(StatsJobResult {
                job_id,
                session_id: job_session_id,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn toggle_stats_cpu_expanded(&mut self, cx: &mut Context<Self>) {
        self.remote_ops.stats.cpu_expanded = !self.remote_ops.stats.cpu_expanded;
        self.remote_ops.stats.status = if self.remote_ops.stats.cpu_expanded {
            "showing per-core CPU usage".to_string()
        } else {
            "collapsed per-core CPU usage".to_string()
        };
        cx.notify();
    }

    pub(in crate::features) fn drain_stats_events(&mut self) -> bool {
        let mut dirty = false;
        for _ in 0..STATS_EVENT_DRAIN_LIMIT {
            let Ok(event) = self.remote_ops.stats.rx.try_recv() else {
                break;
            };
            if !remote_job_event_matches(
                self.remote_ops.stats.job_id,
                self.remote_ops.stats.job_session_id.as_deref(),
                event.job_id,
                &event.session_id,
            ) {
                continue;
            }
            dirty = true;
            self.remote_ops.stats.pending = false;
            self.remote_ops.stats.job_session_id = None;
            if self.active_session_id.as_deref() != Some(event.session_id.as_str()) {
                continue;
            }
            match event.result {
                Ok(stats) => {
                    self.remote_ops.stats.consecutive_refresh_failures = 0;
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
                    self.remote_ops.stats.consecutive_refresh_failures = self
                        .remote_ops
                        .stats
                        .consecutive_refresh_failures
                        .saturating_add(1);
                    if self.remote_ops.stats.consecutive_refresh_failures >= 3 {
                        self.remote_ops.stats.data = None;
                    }
                    self.remote_ops.stats.status = format!("stats refresh failed: {error}");
                    self.terminal.view.status = self.remote_ops.stats.status.clone();
                }
            }
        }
        dirty
    }

    fn begin_stats_job(&mut self, session_id: String) -> u64 {
        self.remote_ops.stats.job_id = self.remote_ops.stats.job_id.wrapping_add(1).max(1);
        self.remote_ops.stats.job_session_id = Some(session_id);
        self.remote_ops.stats.pending = true;
        self.remote_ops.stats.job_id
    }
}

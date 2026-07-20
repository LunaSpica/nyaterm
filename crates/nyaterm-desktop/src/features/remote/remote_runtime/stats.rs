use super::*;

const STATS_EVENT_DRAIN_LIMIT: usize = 8;

impl NyaTermApp {
    pub(in crate::features) fn refresh_stats(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.active_ssh_config.clone() else {
            self.stats_status = "start an SSH session before inspecting stats".to_string();
            self.terminal_status = self.stats_status.clone();
            cx.notify();
            return;
        };
        if self.stats_pending {
            self.stats_status = "stats refresh already running".to_string();
            cx.notify();
            return;
        }

        self.stats_pending = true;
        self.stats_last_refresh_at = Some(Instant::now());
        self.stats_status = "loading remote system stats".to_string();
        let tx = self.stats_tx.clone();
        std::thread::spawn(move || {
            let result = RemoteStatsService::new(config)
                .snapshot()
                .map_err(|error| error.to_string());
            let _ = tx.send(StatsJobResult { result });
        });
        cx.notify();
    }

    pub(in crate::features) fn toggle_stats_cpu_expanded(&mut self, cx: &mut Context<Self>) {
        self.stats_cpu_expanded = !self.stats_cpu_expanded;
        self.stats_status = if self.stats_cpu_expanded {
            "showing per-core CPU usage".to_string()
        } else {
            "collapsed per-core CPU usage".to_string()
        };
        cx.notify();
    }

    pub(in crate::features) fn drain_stats_events(&mut self) -> bool {
        if !self.stats_pending {
            return false;
        }
        let mut dirty = false;
        for _ in 0..STATS_EVENT_DRAIN_LIMIT {
            let Ok(event) = self.stats_rx.try_recv() else {
                break;
            };
            dirty = true;
            self.stats_pending = false;
            match event.result {
                Ok(stats) => {
                    self.stats_consecutive_refresh_failures = 0;
                    self.stats_status = format!(
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
                    self.terminal_status = self.stats_status.clone();
                    self.remote_stats = Some(stats);
                }
                Err(error) => {
                    self.stats_consecutive_refresh_failures =
                        self.stats_consecutive_refresh_failures.saturating_add(1);
                    if self.stats_consecutive_refresh_failures >= 3 {
                        self.remote_stats = None;
                    }
                    self.stats_status = format!("stats refresh failed: {error}");
                    self.terminal_status = self.stats_status.clone();
                }
            }
        }
        dirty
    }
}

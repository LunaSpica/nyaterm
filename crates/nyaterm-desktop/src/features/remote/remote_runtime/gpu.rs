use gpui::{Context, Window};
use nyaterm_transport::{RemoteGpuOverview, RemoteGpuService, RemoteNpuOverview, RemoteNpuService};

use crate::features::{GpuJobResult, NpuJobResult, NyaTermApp};

const ACCELERATOR_EVENT_DRAIN_LIMIT: usize = 8;

impl NyaTermApp {
    pub(in crate::features) fn refresh_gpu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_gpu_with_mode(false, window, cx);
    }

    pub(in crate::features) fn refresh_gpu_auto(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.refresh_gpu_with_mode(true, window, cx);
    }

    fn refresh_gpu_with_mode(
        &mut self,
        skip_unavailable: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.session.active_ssh_config_owned() else {
            self.remote_ops
                .set_gpu_status("start an SSH session before inspecting GPU");
            self.shell
                .set_status(self.remote_ops.gpu_status().to_string());
            cx.notify();
            return;
        };
        let Some(job_session_id) = self.session.active_id_owned() else {
            self.remote_ops
                .set_gpu_status("start an SSH session before inspecting GPU");
            cx.notify();
            return;
        };
        if skip_unavailable && self.remote_ops.gpu_unavailable_for(&job_session_id) {
            return;
        }
        if self.remote_ops.gpu_is_pending_for(&job_session_id) {
            self.remote_ops
                .set_gpu_status("GPU refresh already running");
            cx.notify();
            return;
        }

        let ticket = self.remote_ops.begin_gpu_job(job_session_id.clone());
        self.remote_ops.mark_gpu_refresh_started();
        self.remote_ops
            .set_gpu_status("loading NVIDIA GPU overview");
        std::thread::spawn(move || {
            let result = RemoteGpuService::new(config)
                .overview()
                .map_err(|error| error.to_string());
            let _ = ticket.tx.send(GpuJobResult {
                job_id: ticket.job_id,
                session_id: job_session_id,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn drain_gpu_events(&mut self) -> bool {
        let mut dirty = false;
        for _ in 0..ACCELERATOR_EVENT_DRAIN_LIMIT {
            let Some(event) = self.remote_ops.next_gpu_event() else {
                break;
            };
            if !self
                .remote_ops
                .complete_gpu_event(event.job_id, &event.session_id)
            {
                continue;
            }
            dirty = true;
            if self.session.active_id() != Some(event.session_id.as_str()) {
                continue;
            }
            match event.result {
                Ok(overview) => {
                    self.remote_ops.reset_gpu_refresh_failures();
                    self.remote_ops
                        .set_gpu_status(gpu_overview_status(&overview));
                    self.shell
                        .set_status(self.remote_ops.gpu_status().to_string());
                    self.remote_ops.apply_gpu(&event.session_id, overview);
                }
                Err(error) => {
                    self.remote_ops.record_gpu_refresh_failure();
                    self.remote_ops
                        .set_gpu_status(format!("GPU refresh failed: {error}"));
                    self.shell
                        .set_status(self.remote_ops.gpu_status().to_string());
                }
            }
        }
        dirty
    }

    pub(in crate::features) fn refresh_npu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_npu_with_mode(false, window, cx);
    }

    pub(in crate::features) fn refresh_npu_auto(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.refresh_npu_with_mode(true, window, cx);
    }

    fn refresh_npu_with_mode(
        &mut self,
        skip_unavailable: bool,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.session.active_ssh_config_owned() else {
            self.remote_ops
                .set_npu_status("start an SSH session before inspecting NPU");
            self.shell
                .set_status(self.remote_ops.npu_status().to_string());
            cx.notify();
            return;
        };
        let Some(job_session_id) = self.session.active_id_owned() else {
            self.remote_ops
                .set_npu_status("start an SSH session before inspecting NPU");
            cx.notify();
            return;
        };
        if skip_unavailable && self.remote_ops.npu_unavailable_for(&job_session_id) {
            return;
        }
        if self.remote_ops.npu_is_pending_for(&job_session_id) {
            self.remote_ops
                .set_npu_status("NPU refresh already running");
            cx.notify();
            return;
        }

        let ticket = self.remote_ops.begin_npu_job(job_session_id.clone());
        self.remote_ops.mark_npu_refresh_started();
        self.remote_ops
            .set_npu_status("loading Ascend NPU overview");
        std::thread::spawn(move || {
            let result = RemoteNpuService::new(config)
                .overview()
                .map_err(|error| error.to_string());
            let _ = ticket.tx.send(NpuJobResult {
                job_id: ticket.job_id,
                session_id: job_session_id,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn drain_npu_events(&mut self) -> bool {
        let mut dirty = false;
        for _ in 0..ACCELERATOR_EVENT_DRAIN_LIMIT {
            let Some(event) = self.remote_ops.next_npu_event() else {
                break;
            };
            if !self
                .remote_ops
                .complete_npu_event(event.job_id, &event.session_id)
            {
                continue;
            }
            dirty = true;
            if self.session.active_id() != Some(event.session_id.as_str()) {
                continue;
            }
            match event.result {
                Ok(overview) => {
                    self.remote_ops.reset_npu_refresh_failures();
                    self.remote_ops
                        .set_npu_status(npu_overview_status(&overview));
                    self.shell
                        .set_status(self.remote_ops.npu_status().to_string());
                    self.remote_ops.apply_npu(&event.session_id, overview);
                }
                Err(error) => {
                    self.remote_ops.record_npu_refresh_failure();
                    self.remote_ops
                        .set_npu_status(format!("NPU refresh failed: {error}"));
                    self.shell
                        .set_status(self.remote_ops.npu_status().to_string());
                }
            }
        }
        dirty
    }
}

fn gpu_overview_status(overview: &RemoteGpuOverview) -> String {
    if !overview.available {
        return "NVIDIA GPU is not available on this SSH host".to_string();
    }
    let used = overview
        .gpus
        .iter()
        .map(|gpu| gpu.memory_used_mb)
        .sum::<u64>();
    let total = overview
        .gpus
        .iter()
        .map(|gpu| gpu.memory_total_mb)
        .sum::<u64>();
    format!(
        "NVIDIA GPU · {} device(s) · {used}/{total} MiB",
        overview.gpus.len()
    )
}

fn npu_overview_status(overview: &RemoteNpuOverview) -> String {
    if !overview.available {
        return "Ascend NPU is not available on this SSH host".to_string();
    }
    let used = overview
        .npus
        .iter()
        .map(|npu| npu.memory_used_mb)
        .sum::<u64>();
    let total = overview
        .npus
        .iter()
        .map(|npu| npu.memory_total_mb)
        .sum::<u64>();
    format!(
        "Ascend NPU · {} device(s) · {used}/{total} MiB",
        overview.npus.len()
    )
}

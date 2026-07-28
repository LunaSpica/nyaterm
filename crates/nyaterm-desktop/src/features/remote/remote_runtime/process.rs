use gpui::{ClipboardItem, Context, Window};
use nyaterm_transport::SshProcessService;

use crate::features::NyaTermApp;
use crate::features::runtime_jobs::{ProcessJobOutput, ProcessJobResult};
use crate::models::{DockerTab, RemoteProcessSortKey};

const PROCESS_EVENT_DRAIN_LIMIT: usize = 8;

impl NyaTermApp {
    pub(in crate::features) fn set_docker_tab(&mut self, tab: DockerTab, cx: &mut Context<Self>) {
        self.remote_ops.docker.set_tab(tab);
        cx.notify();
    }

    pub(in crate::features) fn toggle_docker_tab_menu(&mut self, cx: &mut Context<Self>) {
        self.remote_ops.docker.toggle_tab_menu();
        cx.notify();
    }

    /// Apply an edit from the Docker filter box.
    pub(in crate::features) fn apply_docker_search(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.remote_ops.docker.apply_search(text);
        cx.notify();
    }

    /// Apply an edit from the process filter box.
    pub(in crate::features) fn apply_process_search(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.remote_ops.process.apply_search(text);
        cx.notify();
    }

    pub(in crate::features) fn toggle_process_sort(
        &mut self,
        key: RemoteProcessSortKey,
        cx: &mut Context<Self>,
    ) {
        self.remote_ops.process.toggle_sort(key);
        cx.notify();
    }

    pub(in crate::features) fn toggle_process_selection(
        &mut self,
        pid: u32,
        cx: &mut Context<Self>,
    ) {
        self.remote_ops.process.toggle_selection(pid);
        cx.notify();
    }

    /// Apply an edit from the nice value box.
    ///
    /// A nice value is a small signed number, so the draft keeps a leading
    /// minus and up to three digits and drops everything else.
    pub(in crate::features) fn apply_process_nice_input(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.remote_ops.process.apply_nice_input(text);
        cx.notify();
    }

    pub(in crate::features) fn apply_process_nice_draft(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((pid, nice)) = self.remote_ops.process.validated_nice_draft() else {
            cx.notify();
            return;
        };
        self.renice_process(pid, nice, window, cx);
    }

    pub(in crate::features) fn copy_process_text(
        &mut self,
        value: String,
        label: &'static str,
        cx: &mut Context<Self>,
    ) {
        cx.write_to_clipboard(ClipboardItem::new_string(value));
        self.remote_ops.process.status = format!("copied process {label}");
        self.terminal.view.status = self.remote_ops.process.status.clone();
        cx.notify();
    }

    pub(in crate::features) fn copy_docker_text(
        &mut self,
        value: String,
        label: &'static str,
        cx: &mut Context<Self>,
    ) {
        cx.write_to_clipboard(ClipboardItem::new_string(value));
        self.remote_ops.docker.status = format!("copied Docker {label}");
        self.terminal.view.status = self.remote_ops.docker.status.clone();
        cx.notify();
    }

    pub(in crate::features) fn request_process_signal(
        &mut self,
        pid: u32,
        signal: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.remote_ops.process.request_signal(pid, signal) {
            cx.notify();
        } else {
            self.signal_process(pid, signal, window, cx);
        }
    }

    pub(in crate::features) fn cancel_process_signal_confirm(&mut self, cx: &mut Context<Self>) {
        self.remote_ops.process.cancel_signal_confirm();
        cx.notify();
    }

    pub(in crate::features) fn confirm_process_signal(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(confirm) = self.remote_ops.process.take_signal_confirm() else {
            cx.notify();
            return;
        };
        self.signal_process(confirm.pid, confirm.signal, window, cx);
    }

    pub(in crate::features) fn refresh_processes(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.session.active_ssh_config.clone() else {
            self.remote_ops.process.status =
                "start an SSH session before listing processes".to_string();
            self.terminal.view.status = self.remote_ops.process.status.clone();
            cx.notify();
            return;
        };
        let Some(job_session_id) = self.session.active_id.clone() else {
            self.remote_ops.process.status =
                "start an SSH session before listing processes".to_string();
            cx.notify();
            return;
        };
        if self.remote_ops.process.is_pending_for(&job_session_id) {
            self.remote_ops.process.status = "process operation already running".to_string();
            cx.notify();
            return;
        }

        let ticket = self.remote_ops.process.begin_job(job_session_id.clone());
        self.remote_ops.process.menu_pid = None;
        self.remote_ops.process.mark_refresh_started();
        self.remote_ops.process.status = "listing remote processes".to_string();
        std::thread::spawn(move || {
            let result = SshProcessService::new(config)
                .list_processes()
                .map(ProcessJobOutput::Listed)
                .map_err(|error| error.to_string());
            let _ = ticket.tx.send(ProcessJobResult {
                job_id: ticket.job_id,
                session_id: job_session_id,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn signal_process(
        &mut self,
        pid: u32,
        signal: &'static str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.session.active_ssh_config.clone() else {
            self.remote_ops.process.status =
                "start an SSH session before signalling processes".to_string();
            self.terminal.view.status = self.remote_ops.process.status.clone();
            cx.notify();
            return;
        };
        let Some(job_session_id) = self.session.active_id.clone() else {
            self.remote_ops.process.status =
                "start an SSH session before signalling processes".to_string();
            cx.notify();
            return;
        };
        if self.remote_ops.process.is_pending_for(&job_session_id) {
            self.remote_ops.process.status = "process operation already running".to_string();
            cx.notify();
            return;
        }

        let ticket = self.remote_ops.process.begin_job(job_session_id.clone());
        self.remote_ops.process.status = format!("sending {signal} to pid {pid}");
        std::thread::spawn(move || {
            let result = (|| {
                let service = SshProcessService::new(config);
                service.signal_process(pid, signal)?;
                let processes = service.list_processes()?;
                Ok(ProcessJobOutput::Signalled {
                    pid,
                    signal: signal.to_string(),
                    processes,
                })
            })()
            .map_err(|error: anyhow::Error| error.to_string());
            let _ = ticket.tx.send(ProcessJobResult {
                job_id: ticket.job_id,
                session_id: job_session_id,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn renice_process(
        &mut self,
        pid: u32,
        nice: i32,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(config) = self.session.active_ssh_config.clone() else {
            self.remote_ops.process.status =
                "start an SSH session before renicing processes".to_string();
            self.terminal.view.status = self.remote_ops.process.status.clone();
            cx.notify();
            return;
        };
        let Some(job_session_id) = self.session.active_id.clone() else {
            self.remote_ops.process.status =
                "start an SSH session before renicing processes".to_string();
            cx.notify();
            return;
        };
        if self.remote_ops.process.is_pending_for(&job_session_id) {
            self.remote_ops.process.status = "process operation already running".to_string();
            cx.notify();
            return;
        }

        let ticket = self.remote_ops.process.begin_job(job_session_id.clone());
        self.remote_ops.process.status = format!("renicing pid {pid} to {nice}");
        std::thread::spawn(move || {
            let result = (|| {
                let service = SshProcessService::new(config);
                service.renice_process(pid, nice)?;
                let processes = service.list_processes()?;
                Ok(ProcessJobOutput::Reniced {
                    pid,
                    nice,
                    processes,
                })
            })()
            .map_err(|error: anyhow::Error| error.to_string());
            let _ = ticket.tx.send(ProcessJobResult {
                job_id: ticket.job_id,
                session_id: job_session_id,
                result,
            });
        });
        cx.notify();
    }

    pub(in crate::features) fn drain_process_events(&mut self) -> bool {
        let mut dirty = false;
        for _ in 0..PROCESS_EVENT_DRAIN_LIMIT {
            let Some(event) = self.remote_ops.process.next_event() else {
                break;
            };
            if !self
                .remote_ops
                .process
                .complete_event(event.job_id, &event.session_id)
            {
                continue;
            }
            dirty = true;
            if self.session.active_id.as_deref() != Some(event.session_id.as_str()) {
                continue;
            }
            let was_list_refresh = self.remote_ops.process.status == "listing remote processes";
            match event.result {
                Ok(ProcessJobOutput::Listed(processes)) => {
                    self.remote_ops.process.reset_refresh_failures();
                    self.remote_ops.process.status =
                        format!("loaded {} remote process(es)", processes.len());
                    self.terminal.view.status = self.remote_ops.process.status.clone();
                    self.remote_ops.process.apply_processes(processes);
                }
                Ok(ProcessJobOutput::Signalled {
                    pid,
                    signal,
                    processes,
                }) => {
                    self.remote_ops.process.status = format!("sent {signal} to pid {pid}");
                    self.terminal.view.status = self.remote_ops.process.status.clone();
                    self.remote_ops.process.signal_confirm = None;
                    self.remote_ops.process.apply_processes(processes);
                }
                Ok(ProcessJobOutput::Reniced {
                    pid,
                    nice,
                    processes,
                }) => {
                    self.remote_ops.process.status = format!("reniced pid {pid} to {nice}");
                    self.terminal.view.status = self.remote_ops.process.status.clone();
                    self.remote_ops.process.apply_processes(processes);
                }
                Err(error) => {
                    if was_list_refresh {
                        let terminal =
                            error.contains(nyaterm_transport::PROCESS_LIST_UNSUPPORTED_ERROR);
                        if self.remote_ops.process.record_refresh_failure(terminal) >= 3 {
                            self.remote_ops.process.items.clear();
                            self.remote_ops.process.snapshot_loaded = false;
                        }
                    }
                    self.remote_ops.process.status = format!("process operation failed: {error}");
                    self.terminal.view.status = self.remote_ops.process.status.clone();
                }
            }
        }
        dirty
    }
}

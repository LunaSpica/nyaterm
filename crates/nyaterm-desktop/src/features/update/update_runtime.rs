use gpui::Context;

use crate::features::NyaTermApp;
use crate::http::update::check_native_update;

use super::state::UpdateJobResult;

const UPDATE_EVENT_DRAIN_LIMIT: usize = 4;

impl NyaTermApp {
    pub(in crate::features) fn open_update_dialog(&mut self, cx: &mut Context<Self>) {
        self.update.dialog_open = true;
        self.start_update_check(cx);
    }

    pub(in crate::features) fn close_update_dialog(&mut self, cx: &mut Context<Self>) {
        self.update.dialog_open = false;
        cx.notify();
    }

    pub(in crate::features) fn start_update_check(&mut self, cx: &mut Context<Self>) {
        if self.update.pending {
            self.update.status = "update check already running".to_string();
            cx.notify();
            return;
        }
        self.update.pending = true;
        self.update.status = "checking GitHub releases...".to_string();
        self.update.info = None;
        let tx = self.update.tx.clone();
        std::thread::spawn(move || {
            let result = check_native_update();
            let _ = tx.send(UpdateJobResult { result });
        });
        cx.notify();
    }

    pub(in crate::features) fn drain_update_events(&mut self) -> bool {
        if !self.update.pending {
            return false;
        }
        let mut dirty = false;
        for _ in 0..UPDATE_EVENT_DRAIN_LIMIT {
            let Ok(event) = self.update.rx.try_recv() else {
                break;
            };
            dirty = true;
            self.update.pending = false;
            match event.result {
                Ok(info) => {
                    self.update.status = if info.available {
                        format!(
                            "update available: {} -> {}",
                            info.current_version, info.latest_version
                        )
                    } else {
                        format!("NyaTerm is up to date ({})", info.current_version)
                    };
                    self.terminal.view.status = self.update.status.clone();
                    self.update.info = Some(info);
                }
                Err(error) => {
                    self.update.status = format!("update check failed: {error}");
                    self.terminal.view.status = self.update.status.clone();
                    self.update.info = None;
                }
            }
        }
        dirty
    }
}

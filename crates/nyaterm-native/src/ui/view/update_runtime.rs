use super::*;

use crate::update_http::check_native_update;

impl NyaTermApp {
    pub(in crate::ui::view) fn start_update_check(&mut self, cx: &mut Context<Self>) {
        if self.update_pending {
            self.update_status = "update check already running".to_string();
            cx.notify();
            return;
        }
        self.update_pending = true;
        self.update_status = "checking GitHub releases...".to_string();
        self.update_info = None;
        let tx = self.update_tx.clone();
        std::thread::spawn(move || {
            let result = check_native_update();
            let _ = tx.send(UpdateJobResult { result });
        });
        cx.notify();
    }

    pub(super) fn drain_update_events(&mut self) -> bool {
        let mut dirty = false;
        while let Ok(event) = self.update_rx.try_recv() {
            dirty = true;
            self.update_pending = false;
            match event.result {
                Ok(info) => {
                    self.update_status = if info.available {
                        format!(
                            "update available: {} -> {}",
                            info.current_version, info.latest_version
                        )
                    } else {
                        format!("NyaTerm is up to date ({})", info.current_version)
                    };
                    self.terminal_status = self.update_status.clone();
                    self.update_info = Some(info);
                }
                Err(error) => {
                    self.update_status = format!("update check failed: {error}");
                    self.terminal_status = self.update_status.clone();
                    self.update_info = None;
                }
            }
        }
        dirty
    }
}

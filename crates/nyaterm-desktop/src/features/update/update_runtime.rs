use gpui::Context;

use crate::features::NyaTermApp;
use crate::http::update::check_native_update;

use super::state::UpdateJobResult;

impl NyaTermApp {
    pub(in crate::features) fn open_update_dialog(&mut self, cx: &mut Context<Self>) {
        self.update.open_dialog();
        self.start_update_check(cx);
    }

    pub(in crate::features) fn close_update_dialog(&mut self, cx: &mut Context<Self>) {
        self.update.close_dialog();
        cx.notify();
    }

    pub(in crate::features) fn start_update_check(&mut self, cx: &mut Context<Self>) {
        let Some(tx) = self.update.begin_check() else {
            cx.notify();
            return;
        };
        std::thread::spawn(move || {
            let result = check_native_update();
            let _ = tx.send(UpdateJobResult::new(result));
        });
        cx.notify();
    }

    pub(in crate::features) fn drain_update_events(&mut self) -> bool {
        let dirty = self.update.drain_events();
        if dirty {
            self.terminal.view.status = self.update.status().to_string();
        }
        dirty
    }
}

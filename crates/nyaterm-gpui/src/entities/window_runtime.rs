use std::time::Duration;

use gpui::{Context, Entity, Timer, Window};

use crate::features::NyaTermApp;

#[derive(Debug, Default)]
pub struct WindowRuntimeStore {
    pump_started: bool,
}

impl WindowRuntimeStore {
    pub fn ensure_started(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        app: Entity<NyaTermApp>,
    ) -> bool {
        if !self.mark_started() {
            return false;
        }
        app.update(cx, |app, _| app.mark_window_runtime_started());
        window
            .spawn(cx, async move |cx| {
                loop {
                    Timer::after(Duration::from_millis(50)).await;
                    let keep_running = cx
                        .update(|window, cx| {
                            app.update(cx, |app, cx| app.drive_window_runtime_tick(window, cx))
                        })
                        .unwrap_or(false);
                    if !keep_running {
                        break;
                    }
                }
            })
            .detach();
        true
    }

    pub fn mark_started(&mut self) -> bool {
        if self.pump_started {
            return false;
        }
        self.pump_started = true;
        true
    }

    pub fn pump_started(&self) -> bool {
        self.pump_started
    }
}

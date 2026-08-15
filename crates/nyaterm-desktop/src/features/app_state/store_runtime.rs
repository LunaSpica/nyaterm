use gpui::Context;
use nyaterm_store::{StoreEvent, StoreRequest};

use super::NyaTermApp;

impl NyaTermApp {
    pub(in crate::features) fn submit_store_request<R>(
        &mut self,
        generation: u64,
        request: R,
        apply: impl FnOnce(&mut Self, StoreEvent<R::Response>, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) where
        R: StoreRequest,
    {
        match self.store_ui.try_submit(generation, request) {
            Ok(task) => {
                cx.spawn(async move |this, cx| {
                    let event = task.await;
                    let _ = this.update(cx, |this, cx| apply(this, event, cx));
                })
                .detach();
            }
            Err(error) => {
                let message = format!("storage request was not queued: {error}");
                self.settings.update_store_status(message.clone(), false);
                self.shell.set_status(message);
                cx.notify();
            }
        }
    }

    pub(in crate::features) fn store_blocking_client(&self) -> nyaterm_store::StoreBlockingClient {
        self.store_blocking.clone()
    }
}

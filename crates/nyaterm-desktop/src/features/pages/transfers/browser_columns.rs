use gpui::{Context, MouseDownEvent, MouseMoveEvent, MouseUpEvent};

use crate::features::NyaTermApp;
use crate::models::TransferBrowserSortColumn;

impl NyaTermApp {
    pub(in crate::features::pages::transfers) fn start_transfer_browser_column_resize(
        &mut self,
        column: TransferBrowserSortColumn,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.transfer
            .browser
            .start_column_resize(column, event.position.x);
        cx.notify();
    }

    pub(in crate::features) fn update_transfer_browser_column_resize(
        &mut self,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        if self.transfer.browser.update_column_resize(event.position.x) {
            cx.notify();
        }
    }

    pub(in crate::features) fn finish_transfer_browser_column_resize(
        &mut self,
        _event: &MouseUpEvent,
        cx: &mut Context<Self>,
    ) {
        if self.transfer.browser.finish_column_resize() {
            cx.notify();
        }
    }
}

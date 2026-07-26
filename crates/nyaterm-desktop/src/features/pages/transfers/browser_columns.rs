use super::*;

impl NyaTermApp {
    pub(in crate::features::pages::transfers) fn start_transfer_browser_column_resize(
        &mut self,
        column: TransferBrowserSortColumn,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.transfer.browser.column_resize = Some(TransferBrowserColumnResizeState {
            column,
            start_x: event.position.x,
            start_width: self.transfer.browser.column_widths.get(column),
        });
        self.transfer.browser.status = format!("resizing {} column", column.label().to_lowercase());
        cx.notify();
    }

    pub(in crate::features) fn update_transfer_browser_column_resize(
        &mut self,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer.browser.column_resize else {
            return;
        };
        let next_width = state.start_width + (event.position.x - state.start_x);
        self.transfer
            .browser
            .column_widths
            .set(state.column, next_width);
        let width = f32::from(self.transfer.browser.column_widths.get(state.column)).round();
        self.transfer.browser.status =
            format!("{} column: {width}px", state.column.label().to_lowercase());
        cx.notify();
    }

    pub(in crate::features) fn finish_transfer_browser_column_resize(
        &mut self,
        _event: &MouseUpEvent,
        cx: &mut Context<Self>,
    ) {
        if self.transfer.browser.column_resize.take().is_some() {
            self.transfer.browser.status = "file column width updated".to_string();
            cx.notify();
        }
    }
}

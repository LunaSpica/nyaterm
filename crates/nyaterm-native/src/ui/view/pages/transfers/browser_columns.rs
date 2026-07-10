use super::*;

impl NyaTermApp {
    pub(in crate::ui::view::pages::transfers) fn start_transfer_browser_column_resize(
        &mut self,
        column: TransferBrowserSortColumn,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.transfer_browser_column_resize = Some(TransferBrowserColumnResizeState {
            column,
            start_x: event.position.x,
            start_width: self.transfer_browser_column_widths.get(column),
        });
        self.transfer_browser_status = format!("resizing {} column", column.label().to_lowercase());
        cx.notify();
    }

    pub(in crate::ui::view) fn update_transfer_browser_column_resize(
        &mut self,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer_browser_column_resize else {
            return;
        };
        let next_width = state.start_width + (event.position.x - state.start_x);
        self.transfer_browser_column_widths
            .set(state.column, next_width);
        let width = f32::from(self.transfer_browser_column_widths.get(state.column)).round();
        self.transfer_browser_status =
            format!("{} column: {width}px", state.column.label().to_lowercase());
        cx.notify();
    }

    pub(in crate::ui::view) fn finish_transfer_browser_column_resize(
        &mut self,
        _event: &MouseUpEvent,
        cx: &mut Context<Self>,
    ) {
        if self.transfer_browser_column_resize.take().is_some() {
            self.transfer_browser_status = "file column width updated".to_string();
            cx.notify();
        }
    }
}

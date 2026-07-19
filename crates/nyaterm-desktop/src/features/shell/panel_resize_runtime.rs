use super::*;
use gpui::{MouseDownEvent, MouseMoveEvent, MouseUpEvent};

const LEFT_PANEL_MIN: f32 = 160.;
const LEFT_PANEL_MAX: f32 = 720.;
const RIGHT_PANEL_MIN: f32 = 200.;
const RIGHT_PANEL_MAX: f32 = 720.;
const QUICK_CMD_HEIGHT_MIN: f32 = 36.;
const SERIAL_SEND_HEIGHT_MIN: f32 = 60.;
const BOTTOM_PANEL_HEIGHT_MAX: f32 = 520.;

impl NyaTermApp {
    pub(in crate::features) fn set_bottom_panel_mode(&mut self, mode: BottomPanelMode) {
        self.bottom_panel = mode;
        self.settings.ui_quick_cmd_visible = mode == BottomPanelMode::QuickCommands;
        self.settings.ui_serial_send_visible = mode == BottomPanelMode::CommandSend;
        self.persist_ui_layout();
    }

    pub(in crate::features) fn start_panel_resize(
        &mut self,
        side: PanelResizeSide,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        let start_width = match side {
            PanelResizeSide::Left => px(self.left_panel_width),
            PanelResizeSide::Right => px(self.right_panel_width),
        };
        self.panel_resize = Some(PanelResizeState {
            side,
            start_x: event.position.x,
            start_width,
        });
        self.terminal_status = match side {
            PanelResizeSide::Left => "resizing left panel".to_string(),
            PanelResizeSide::Right => "resizing right panel".to_string(),
        };
        cx.notify();
    }

    pub(in crate::features) fn update_panel_resize(
        &mut self,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.panel_resize else {
            return;
        };
        let delta = f32::from(event.position.x - state.start_x);
        let start = f32::from(state.start_width);
        match state.side {
            PanelResizeSide::Left => {
                self.left_panel_width = (start + delta).clamp(LEFT_PANEL_MIN, LEFT_PANEL_MAX);
                self.terminal_status =
                    format!("left panel: {:.0}px", self.left_panel_width.round());
            }
            PanelResizeSide::Right => {
                // Right handle sits on the left edge of the right panel: drag left grows width.
                self.right_panel_width = (start - delta).clamp(RIGHT_PANEL_MIN, RIGHT_PANEL_MAX);
                self.terminal_status =
                    format!("right panel: {:.0}px", self.right_panel_width.round());
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn finish_panel_resize(
        &mut self,
        _event: &MouseUpEvent,
        cx: &mut Context<Self>,
    ) {
        if self.panel_resize.take().is_some() {
            self.persist_panel_widths();
            self.terminal_status = format!(
                "panel sizes L{:.0}/R{:.0}",
                self.left_panel_width.round(),
                self.right_panel_width.round()
            );
            cx.notify();
        }
    }

    fn persist_panel_widths(&mut self) {
        self.persist_ui_layout();
    }

    pub(in crate::features) fn apply_ui_layout_from_settings(&mut self) {
        self.left_panel_width = self.settings.ui_left_panel_width as f32;
        self.right_panel_width = self.settings.ui_right_panel_width as f32;
        self.transfer_panel_height = self.settings.ui_transfer_height as f32;
        self.quick_cmd_height = self.settings.ui_quick_cmd_height as f32;
        self.serial_send_height = self.settings.ui_serial_send_height as f32;
        self.apply_activity_layout_from_settings();
        self.active_left_panel = self
            .settings
            .ui_active_left_panel
            .as_deref()
            .and_then(NavItem::from_persistence_id)
            .filter(|item| self.panel_side_for_item(*item) == Some(PanelSide::Left));
        self.active_right_panel = self
            .settings
            .ui_active_right_panel
            .as_deref()
            .and_then(NavItem::from_persistence_id)
            .filter(|item| self.panel_side_for_item(*item) == Some(PanelSide::Right));
        self.left_sidebar_collapsed = self.settings.ui_left_panel_collapsed;
        self.right_inspector_collapsed = self.settings.ui_right_panel_collapsed;
        self.apply_panel_stack_from_settings();
        if !self.settings.has_master_password {
            self.security_secrets_unlocked = true;
        }
    }

    pub(in crate::features) fn persist_ui_layout(&mut self) {
        self.settings.ui_left_panel_width = self.left_panel_width.round().clamp(160., 720.) as u32;
        self.settings.ui_right_panel_width =
            self.right_panel_width.round().clamp(200., 720.) as u32;
        self.settings.ui_transfer_height =
            self.transfer_panel_height.round().clamp(60., 600.) as u32;
        self.settings.ui_quick_cmd_height =
            self.quick_cmd_height
                .round()
                .clamp(QUICK_CMD_HEIGHT_MIN, BOTTOM_PANEL_HEIGHT_MAX) as u32;
        self.settings.ui_serial_send_height =
            self.serial_send_height
                .round()
                .clamp(SERIAL_SEND_HEIGHT_MIN, BOTTOM_PANEL_HEIGHT_MAX) as u32;
        self.settings.ui_active_left_panel = self
            .active_left_panel
            .map(|item| item.persistence_id().to_string());
        self.settings.ui_active_right_panel = self
            .active_right_panel
            .map(|item| item.persistence_id().to_string());
        self.settings.ui_left_panel_collapsed = self.left_sidebar_collapsed;
        self.settings.ui_right_panel_collapsed = self.right_inspector_collapsed;
        self.settings.ui_saved_connections_sort_mode =
            self.connection_sort_mode.persistence_id().to_string();
        self.sync_activity_layout_to_settings();
        self.sync_panel_stack_to_settings();
        if let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            match store.save_ui_layout_settings(&self.settings) {
                Ok(summary) => {
                    self.apply_gpui_settings(summary);
                    self.store_status.ready = true;
                    self.store_status.message = "panel layout saved".to_string();
                }
                Err(error) => {
                    self.store_status.ready = false;
                    self.store_status.message = format!("failed to save panel layout: {error}");
                }
            }
        }
    }

    pub(in crate::features) fn panel_resize_handle(
        &self,
        side: PanelResizeSide,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        div()
            .id(SharedString::from(format!(
                "panel-resize-{}",
                match side {
                    PanelResizeSide::Left => "left",
                    PanelResizeSide::Right => "right",
                }
            )))
            .w(px(3.))
            .flex_none()
            .h_full()
            .bg(rgb(palette.border))
            .cursor_col_resize()
            .hover(|this| this.bg(rgb(0x58a6ff)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _, cx| {
                    this.start_panel_resize(side, event, cx);
                }),
            )
    }
}

impl NyaTermApp {
    pub(in crate::features) fn start_transfer_height_resize(
        &mut self,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.transfer_height_resize = Some(TransferHeightResizeState {
            start_y: event.position.y,
            start_height: px(self.transfer_panel_height),
        });
        self.terminal_status = "resizing transfer queue".to_string();
        cx.notify();
    }

    pub(in crate::features) fn update_transfer_height_resize(
        &mut self,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer_height_resize else {
            return;
        };
        // Handle sits above the queue: drag down grows height (Tauri subtracts delta from height
        // because its handle is above and delta is positive downward; we invert equivalently).
        let delta = f32::from(event.position.y - state.start_y);
        let start = f32::from(state.start_height);
        // Tauri: transfer_height = clamp(prev - delta). With handle above queue, positive mouse-down
        // should increase queue height when dragging the handle downward from FE into queue...
        // In App.tsx: onTransferResize uses Math.max(60, Math.min(600, prev - delta)).
        // Vertical ResizeHandle delta = currentY - startY (positive down). Dragging handle down
        // shrinks FE / grows transfer? Handle is between FE and transfer; drag down => FE larger,
        // transfer smaller => height decreases with positive delta. So: start - delta.
        self.transfer_panel_height = (start - delta).clamp(60., 600.);
        self.terminal_status = format!(
            "transfer queue: {:.0}px",
            self.transfer_panel_height.round()
        );
        cx.notify();
    }

    pub(in crate::features) fn finish_transfer_height_resize(
        &mut self,
        _event: &MouseUpEvent,
        cx: &mut Context<Self>,
    ) {
        if self.transfer_height_resize.take().is_some() {
            self.persist_ui_layout();
            self.terminal_status =
                format!("transfer queue {:.0}px", self.transfer_panel_height.round());
            cx.notify();
        }
    }

    pub(in crate::features) fn transfer_height_resize_handle(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        div()
            .id(SharedString::from("transfer-height-resize"))
            .h(px(3.))
            .w_full()
            .flex_none()
            .bg(rgb(palette.border))
            .cursor_row_resize()
            .hover(|this| this.bg(rgb(0x58a6ff)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _, cx| {
                    this.start_transfer_height_resize(event, cx);
                }),
            )
    }
}

impl NyaTermApp {
    pub(in crate::features) fn start_bottom_panel_resize(
        &mut self,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        let mode = self.bottom_panel;
        let start_height = match mode {
            BottomPanelMode::QuickCommands => self.quick_cmd_height,
            BottomPanelMode::CommandSend => self.serial_send_height,
            BottomPanelMode::Hidden => return,
        };
        self.bottom_panel_resize = Some(BottomPanelResizeState {
            mode,
            start_y: event.position.y,
            start_height: px(start_height),
        });
        self.terminal_status = "resizing bottom panel".to_string();
        cx.notify();
    }

    pub(in crate::features) fn update_bottom_panel_resize(
        &mut self,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.bottom_panel_resize else {
            return;
        };
        let delta = f32::from(event.position.y - state.start_y);
        let next = (f32::from(state.start_height) - delta).clamp(
            match state.mode {
                BottomPanelMode::QuickCommands => QUICK_CMD_HEIGHT_MIN,
                BottomPanelMode::CommandSend => SERIAL_SEND_HEIGHT_MIN,
                BottomPanelMode::Hidden => return,
            },
            BOTTOM_PANEL_HEIGHT_MAX,
        );
        match state.mode {
            BottomPanelMode::QuickCommands => self.quick_cmd_height = next,
            BottomPanelMode::CommandSend => self.serial_send_height = next,
            BottomPanelMode::Hidden => return,
        }
        self.terminal_status = format!("bottom panel: {:.0}px", next.round());
        cx.notify();
    }

    pub(in crate::features) fn finish_bottom_panel_resize(
        &mut self,
        _event: &MouseUpEvent,
        cx: &mut Context<Self>,
    ) {
        if self.bottom_panel_resize.take().is_some() {
            self.persist_ui_layout();
            self.terminal_status = "bottom panel size saved".to_string();
            cx.notify();
        }
    }

    pub(in crate::features) fn bottom_panel_resize_handle(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        div()
            .id("bottom-panel-resize")
            .h(px(3.))
            .w_full()
            .flex_none()
            .bg(rgb(palette.border))
            .cursor_row_resize()
            .hover(|this| this.bg(rgb(0x58a6ff)))
            .when(self.bottom_panel == BottomPanelMode::Hidden, |this| {
                this.h_0()
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _, cx| {
                    this.start_bottom_panel_resize(event, cx);
                }),
            )
    }
}

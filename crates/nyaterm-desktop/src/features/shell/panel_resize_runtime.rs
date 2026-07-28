use gpui::{
    Context, InteractiveElement as _, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, SharedString, Styled as _, div, prelude::FluentBuilder as _, px, rgb,
};
use nyaterm_core::ConnectionStore;

use crate::features::NyaTermApp;
use crate::models::{
    BottomPanelMode, NavItem, PanelResizeSide, PanelSide, TransferHeightResizeState,
    panel_collapsed_from_persistence,
};

const QUICK_CMD_HEIGHT_MIN: f32 = 36.;
const SERIAL_SEND_HEIGHT_MIN: f32 = 60.;
const BOTTOM_PANEL_HEIGHT_MAX: f32 = 520.;

impl NyaTermApp {
    pub(in crate::features) fn set_bottom_panel_mode(&mut self, mode: BottomPanelMode) {
        self.shell.bottom_panel.mode = mode;
        self.settings.summary.ui_quick_cmd_visible = mode == BottomPanelMode::QuickCommands;
        self.settings.summary.ui_serial_send_visible = mode == BottomPanelMode::CommandSend;
        self.persist_ui_layout();
    }

    pub(in crate::features) fn start_panel_resize(
        &mut self,
        side: PanelResizeSide,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.shell.panels.start_resize(side, event.position.x);
        self.terminal.view.status = match side {
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
        let Some((side, width)) = self.shell.panels.update_resize(event.position.x) else {
            return;
        };
        match side {
            PanelResizeSide::Left => {
                self.terminal.view.status = format!("left panel: {:.0}px", width.round());
            }
            PanelResizeSide::Right => {
                // Right handle sits on the left edge of the right panel: drag left grows width.
                self.terminal.view.status = format!("right panel: {:.0}px", width.round());
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn finish_panel_resize(
        &mut self,
        _event: &MouseUpEvent,
        cx: &mut Context<Self>,
    ) {
        if self.shell.panels.finish_resize() {
            self.persist_panel_widths();
            self.terminal.view.status = format!(
                "panel sizes L{:.0}/R{:.0}",
                self.shell.panels.left_width.round(),
                self.shell.panels.right_width.round()
            );
            cx.notify();
        }
    }

    fn persist_panel_widths(&mut self) {
        self.persist_ui_layout();
    }

    pub(in crate::features) fn apply_ui_layout_from_settings(&mut self) {
        self.shell.panels.left_width = self.settings.summary.ui_left_panel_width as f32;
        self.shell.panels.right_width = self.settings.summary.ui_right_panel_width as f32;
        self.transfer.panel.height = self.settings.summary.ui_transfer_height as f32;
        self.shell.bottom_panel.quick_commands_height =
            self.settings.summary.ui_quick_cmd_height as f32;
        self.shell.bottom_panel.command_send_height =
            self.settings.summary.ui_serial_send_height as f32;
        self.apply_activity_layout_from_settings();
        self.shell.panels.active_left = self
            .settings
            .summary
            .ui_active_left_panel
            .as_deref()
            .and_then(NavItem::from_persistence_id)
            .filter(|item| self.panel_side_for_item(*item) == Some(PanelSide::Left));
        self.shell.panels.active_right = self
            .settings
            .summary
            .ui_active_right_panel
            .as_deref()
            .and_then(NavItem::from_persistence_id)
            .filter(|item| self.panel_side_for_item(*item) == Some(PanelSide::Right));
        self.shell.panels.left_collapsed = panel_collapsed_from_persistence(
            self.settings.summary.ui_left_panel_collapsed,
            self.settings.summary.ui_panel_multi_open,
            self.shell.panels.active_left.is_some(),
            !self.settings.summary.ui_left_open_panels.is_empty(),
        );
        self.shell.panels.right_collapsed = panel_collapsed_from_persistence(
            self.settings.summary.ui_right_panel_collapsed,
            self.settings.summary.ui_panel_multi_open,
            self.shell.panels.active_right.is_some(),
            !self.settings.summary.ui_right_open_panels.is_empty(),
        );
        self.apply_panel_stack_from_settings();
        if !self.settings.summary.has_master_password {
            self.security.unlock.secrets_unlocked = true;
        }
    }

    pub(in crate::features) fn persist_ui_layout(&mut self) {
        self.settings.summary.ui_left_panel_width =
            self.shell.panels.left_width.round().clamp(160., 720.) as u32;
        self.settings.summary.ui_right_panel_width =
            self.shell.panels.right_width.round().clamp(200., 720.) as u32;
        self.settings.summary.ui_transfer_height =
            self.transfer.panel.height.round().clamp(60., 600.) as u32;
        self.settings.summary.ui_quick_cmd_height =
            self.shell
                .bottom_panel
                .quick_commands_height
                .round()
                .clamp(QUICK_CMD_HEIGHT_MIN, BOTTOM_PANEL_HEIGHT_MAX) as u32;
        self.settings.summary.ui_serial_send_height =
            self.shell
                .bottom_panel
                .command_send_height
                .round()
                .clamp(SERIAL_SEND_HEIGHT_MIN, BOTTOM_PANEL_HEIGHT_MAX) as u32;
        self.settings.summary.ui_active_left_panel = self
            .shell
            .panels
            .active_left
            .map(|item| item.persistence_id().to_string());
        self.settings.summary.ui_active_right_panel = self
            .shell
            .panels
            .active_right
            .map(|item| item.persistence_id().to_string());
        self.settings.summary.ui_left_panel_collapsed = self.shell.panels.left_collapsed;
        self.settings.summary.ui_right_panel_collapsed = self.shell.panels.right_collapsed;
        self.settings.summary.ui_saved_connections_sort_mode = self
            .connection_state
            .list_sort_mode()
            .persistence_id()
            .to_string();
        self.sync_activity_layout_to_settings();
        self.sync_panel_stack_to_settings();
        if let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            match store.save_ui_layout_settings(&self.settings.summary) {
                Ok(summary) => {
                    self.apply_gpui_settings(summary);
                    self.settings.store_status.ready = true;
                    self.settings.store_status.message = "panel layout saved".to_string();
                }
                Err(error) => {
                    self.settings.store_status.ready = false;
                    self.settings.store_status.message =
                        format!("failed to save panel layout: {error}");
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
        self.transfer.panel.height_resize = Some(TransferHeightResizeState {
            start_y: event.position.y,
            start_height: px(self.transfer.panel.height),
        });
        self.terminal.view.status = "resizing transfer queue".to_string();
        cx.notify();
    }

    pub(in crate::features) fn update_transfer_height_resize(
        &mut self,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.transfer.panel.height_resize else {
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
        self.transfer.panel.height = (start - delta).clamp(60., 600.);
        self.terminal.view.status = format!(
            "transfer queue: {:.0}px",
            self.transfer.panel.height.round()
        );
        cx.notify();
    }

    pub(in crate::features) fn finish_transfer_height_resize(
        &mut self,
        _event: &MouseUpEvent,
        cx: &mut Context<Self>,
    ) {
        if self.transfer.panel.height_resize.take().is_some() {
            self.persist_ui_layout();
            self.terminal.view.status =
                format!("transfer queue {:.0}px", self.transfer.panel.height.round());
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
        if !self.shell.bottom_panel.start_resize(event.position.y) {
            return;
        }
        self.terminal.view.status = "resizing bottom panel".to_string();
        cx.notify();
    }

    pub(in crate::features) fn update_bottom_panel_resize(
        &mut self,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        let Some(next) = self.shell.bottom_panel.update_resize(event.position.y) else {
            return;
        };
        self.terminal.view.status = format!("bottom panel: {:.0}px", next.round());
        cx.notify();
    }

    pub(in crate::features) fn finish_bottom_panel_resize(
        &mut self,
        _event: &MouseUpEvent,
        cx: &mut Context<Self>,
    ) {
        if self.shell.bottom_panel.finish_resize() {
            self.persist_ui_layout();
            self.terminal.view.status = "bottom panel size saved".to_string();
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
            .when(
                self.shell.bottom_panel.mode == BottomPanelMode::Hidden,
                |this| this.h_0(),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _, cx| {
                    this.start_bottom_panel_resize(event, cx);
                }),
            )
    }
}

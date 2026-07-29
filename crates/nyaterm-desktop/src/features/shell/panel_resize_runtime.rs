use gpui::{
    Context, InteractiveElement as _, IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent,
    MouseUpEvent, SharedString, Styled as _, div, prelude::FluentBuilder as _, px, rgb,
};
use nyaterm_core::ConnectionStore;

use crate::features::{NyaTermApp, settings::UiLayoutSettingsUpdate};
use crate::models::{
    BottomPanelMode, NavItem, PanelResizeSide, PanelSide, panel_collapsed_from_persistence,
};

const QUICK_CMD_HEIGHT_MIN: f32 = 36.;
const SERIAL_SEND_HEIGHT_MIN: f32 = 60.;
const BOTTOM_PANEL_HEIGHT_MAX: f32 = 520.;

impl NyaTermApp {
    pub(in crate::features) fn set_bottom_panel_mode(&mut self, mode: BottomPanelMode) {
        self.shell.bottom_panel.mode = mode;
        self.persist_ui_layout();
    }

    pub(in crate::features) fn start_panel_resize(
        &mut self,
        side: PanelResizeSide,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.shell.panels.start_resize(side, event.position.x);
        self.shell.status = match side {
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
                self.shell.status = format!("left panel: {:.0}px", width.round());
            }
            PanelResizeSide::Right => {
                // Right handle sits on the left edge of the right panel: drag left grows width.
                self.shell.status = format!("right panel: {:.0}px", width.round());
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
            self.shell.status = format!(
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
        self.shell.panels.left_width = self.settings.summary().ui_left_panel_width as f32;
        self.shell.panels.right_width = self.settings.summary().ui_right_panel_width as f32;
        self.transfer
            .set_panel_height(self.settings.summary().ui_transfer_height as f32);
        self.shell.bottom_panel.quick_commands_height =
            self.settings.summary().ui_quick_cmd_height as f32;
        self.shell.bottom_panel.command_send_height =
            self.settings.summary().ui_serial_send_height as f32;
        self.apply_activity_layout_from_settings();
        self.shell.panels.active_left = self
            .settings
            .summary()
            .ui_active_left_panel
            .as_deref()
            .and_then(NavItem::from_persistence_id)
            .filter(|item| self.panel_side_for_item(*item) == Some(PanelSide::Left));
        self.shell.panels.active_right = self
            .settings
            .summary()
            .ui_active_right_panel
            .as_deref()
            .and_then(NavItem::from_persistence_id)
            .filter(|item| self.panel_side_for_item(*item) == Some(PanelSide::Right));
        self.shell.panels.left_collapsed = panel_collapsed_from_persistence(
            self.settings.summary().ui_left_panel_collapsed,
            self.settings.summary().ui_panel_multi_open,
            self.shell.panels.active_left.is_some(),
            !self.settings.summary().ui_left_open_panels.is_empty(),
        );
        self.shell.panels.right_collapsed = panel_collapsed_from_persistence(
            self.settings.summary().ui_right_panel_collapsed,
            self.settings.summary().ui_panel_multi_open,
            self.shell.panels.active_right.is_some(),
            !self.settings.summary().ui_right_open_panels.is_empty(),
        );
        self.apply_panel_stack_from_settings();
        if !self.settings.summary().has_master_password {
            self.security.unlock_without_master_password();
        }
    }

    pub(in crate::features) fn persist_ui_layout(&mut self) {
        let update = UiLayoutSettingsUpdate {
            left_panel_width: self.shell.panels.left_width.round().clamp(160., 720.) as u32,
            right_panel_width: self.shell.panels.right_width.round().clamp(200., 720.) as u32,
            transfer_height: self.transfer.panel_height().round().clamp(60., 600.) as u32,
            quick_command_height: self
                .shell
                .bottom_panel
                .quick_commands_height
                .round()
                .clamp(QUICK_CMD_HEIGHT_MIN, BOTTOM_PANEL_HEIGHT_MAX)
                as u32,
            quick_command_visible: self.shell.bottom_panel.mode == BottomPanelMode::QuickCommands,
            serial_send_height: self
                .shell
                .bottom_panel
                .command_send_height
                .round()
                .clamp(SERIAL_SEND_HEIGHT_MIN, BOTTOM_PANEL_HEIGHT_MAX)
                as u32,
            serial_send_visible: self.shell.bottom_panel.mode == BottomPanelMode::CommandSend,
            active_left_panel: self
                .shell
                .panels
                .active_left
                .map(|item| item.persistence_id().to_string()),
            active_right_panel: self
                .shell
                .panels
                .active_right
                .map(|item| item.persistence_id().to_string()),
            left_panel_collapsed: self.shell.panels.left_collapsed,
            right_panel_collapsed: self.shell.panels.right_collapsed,
            saved_connections_sort_mode: self
                .connection_state
                .list_sort_mode()
                .persistence_id()
                .to_string(),
            activity_bar_left_top: self.shell.chrome.activity_bar_layout.left_top.clone(),
            activity_bar_left_bottom: self.shell.chrome.activity_bar_layout.left_bottom.clone(),
            activity_bar_right_top: self.shell.chrome.activity_bar_layout.right_top.clone(),
            activity_bar_right_bottom: self.shell.chrome.activity_bar_layout.right_bottom.clone(),
            activity_bar_show_labels: self.shell.chrome.activity_bar_layout.show_labels,
            panel_multi_open: self.shell.panels.multi_open,
            left_open_panels: self.shell.panels.left_open.clone(),
            right_open_panels: self.shell.panels.right_open.clone(),
            panel_stack_sizes: self
                .shell
                .panels
                .stack_sizes
                .iter()
                .filter_map(|(key, value)| {
                    let scaled = (*value * 1000.).round();
                    (scaled.is_finite() && scaled > 0.).then(|| (key.clone(), scaled as u32))
                })
                .collect(),
        };
        self.settings.apply_ui_layout(update);
        if let Ok(store) = ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        ) {
            match store.save_ui_layout_settings(self.settings.summary()) {
                Ok(summary) => {
                    self.apply_gpui_settings(summary);
                    self.settings.set_store_ready(true);
                    self.settings
                        .set_store_message("panel layout saved".to_string());
                }
                Err(error) => {
                    self.settings.set_store_ready(false);
                    self.settings
                        .set_store_message(format!("failed to save panel layout: {error}"));
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
        self.transfer.start_panel_height_resize(event.position.y);
        self.shell.status = "resizing transfer queue".to_string();
        cx.notify();
    }

    pub(in crate::features) fn update_transfer_height_resize(
        &mut self,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        let Some(height) = self.transfer.update_panel_height_resize(event.position.y) else {
            return;
        };
        self.shell.status = format!("transfer queue: {:.0}px", height.round());
        cx.notify();
    }

    pub(in crate::features) fn finish_transfer_height_resize(
        &mut self,
        _event: &MouseUpEvent,
        cx: &mut Context<Self>,
    ) {
        if self.transfer.finish_panel_height_resize() {
            self.persist_ui_layout();
            self.shell.status = format!(
                "transfer queue {:.0}px",
                self.transfer.panel_height().round()
            );
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
        self.shell.status = "resizing bottom panel".to_string();
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
        self.shell.status = format!("bottom panel: {:.0}px", next.round());
        cx.notify();
    }

    pub(in crate::features) fn finish_bottom_panel_resize(
        &mut self,
        _event: &MouseUpEvent,
        cx: &mut Context<Self>,
    ) {
        if self.shell.bottom_panel.finish_resize() {
            self.persist_ui_layout();
            self.shell.status = "bottom panel size saved".to_string();
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

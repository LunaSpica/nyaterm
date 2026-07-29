use gpui::{Context, IntoElement, MouseButton, SharedString, div, prelude::*, px, rgb, svg};

use crate::features::NyaTermApp;

impl NyaTermApp {
    pub(in crate::features) fn active_session_menu_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let Some(menu) = self.session.active_menu().cloned() else {
            return div().into_any_element();
        };
        let session_id = menu.session_id.clone();
        let reconnect_session_id = session_id.clone();
        let close_session_id = session_id.clone();
        let busy_action = self.session.busy_action(&session_id).map(str::to_string);
        let is_busy = busy_action.is_some();
        let is_disconnected = self.is_session_disconnected(&session_id);
        let can_reconnect = !is_busy && !self.has_pending_session_start();
        let can_disconnect = !is_busy && !is_disconnected;
        let reconnect_label = if busy_action.as_deref() == Some("reconnect") {
            self.tr("tabCtx.reconnecting").to_string()
        } else {
            self.tr("tabCtx.reconnect").to_string()
        };
        let disconnect_label = if busy_action.as_deref() == Some("disconnect") {
            self.tr("tabCtx.disconnecting").to_string()
        } else {
            self.tr("tabCtx.disconnect").to_string()
        };
        let (viewport_w, viewport_h) = self.shell.viewport.size;
        let (menu_x, menu_y) = active_session_clamped_menu_position(
            f32::from(menu.x) - 132.,
            f32::from(menu.y) + 8.,
            160.,
            68.,
            viewport_w,
            viewport_h,
        );

        div()
            .id(SharedString::from("active-session-menu-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .on_click(cx.listener(|this, _, _, cx| {
                this.session.close_active_menu();
                cx.notify();
            }))
            .child(
                div()
                    .id(SharedString::from(format!(
                        "active-session-menu-{session_id}"
                    )))
                    .absolute()
                    .top(px(menu_y))
                    .left(px(menu_x))
                    .w(px(160.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(self.shell_surface_color(palette.surface))
                    .shadow_lg()
                    .py_1()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .on_click(|_, _, cx| {
                        cx.stop_propagation();
                    })
                    .child(active_session_overlay_menu_item(
                        palette,
                        format!("active-session-reconnect-{session_id}"),
                        reconnect_label,
                        "icons/session/reconnect.svg",
                        can_reconnect,
                        busy_action.as_deref() == Some("reconnect"),
                        false,
                        cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.session.close_active_menu();
                            if this.session.session_is_busy(&reconnect_session_id)
                                || this.has_pending_session_start()
                            {
                                cx.notify();
                                return;
                            }
                            this.select_session(reconnect_session_id.clone(), cx);
                            this.reconnect_session(reconnect_session_id.clone(), window, cx);
                        }),
                    ))
                    .child(active_session_overlay_menu_item(
                        palette,
                        format!("active-session-disconnect-{session_id}"),
                        disconnect_label,
                        "icons/session/disconnect.svg",
                        can_disconnect,
                        busy_action.as_deref() == Some("disconnect"),
                        true,
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.session.close_active_menu();
                            if this.session.session_is_busy(&close_session_id)
                                || this.is_session_disconnected(&close_session_id)
                            {
                                cx.notify();
                                return;
                            }
                            this.disconnect_session(close_session_id.clone(), cx);
                        }),
                    )),
            )
            .into_any_element()
    }
}

fn active_session_overlay_menu_item(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    label: impl Into<String>,
    icon_path: &'static str,
    enabled: bool,
    busy: bool,
    destructive: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let label = label.into();
    let text_color = if !enabled {
        palette.text_dimmed
    } else if destructive {
        palette.danger
    } else {
        palette.text
    };
    div()
        .id(SharedString::from(id.into()))
        .px_3()
        .h(px(30.))
        .flex()
        .items_center()
        .gap_2()
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(|this| this.bg(rgb(palette.surface_elevated)))
        })
        .when(!enabled, |this| this.opacity(0.5))
        .child(
            svg()
                .size(px(14.))
                .flex_none()
                .path(icon_path)
                .text_color(rgb(text_color)),
        )
        .child(
            div()
                .text_size(px(12.))
                .text_color(rgb(text_color))
                .child(if busy { format!("{label}") } else { label }),
        )
        .on_click(move |event, window, cx| {
            if enabled {
                on_click(event, window, cx);
            }
        })
}

fn active_session_clamped_menu_position(
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    viewport_w: f32,
    viewport_h: f32,
) -> (f32, f32) {
    let max_x = (viewport_w - width - 8.).max(8.);
    let max_y = (viewport_h - height - 8.).max(8.);
    (x.clamp(8., max_x), y.clamp(8., max_y))
}

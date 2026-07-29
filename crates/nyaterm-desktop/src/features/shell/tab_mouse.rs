use gpui::{
    ClickEvent, ClipboardItem, Context, FontWeight, InteractiveElement as _, IntoElement,
    MouseButton, ParentElement as _, Render, SharedString, StatefulInteractiveElement as _,
    Styled as _, Window, div, prelude::FluentBuilder as _, px, rgb, rgba,
};
use nyaterm_core::truncate_preview;

use crate::features::{NyaTermApp, mono_icon, short_id};

#[derive(Clone, Debug)]
pub(in crate::features) struct SessionTabDragPayload {
    pub session_id: String,
    pub display_name: String,
    pub kind_label: &'static str,
}

pub(in crate::features) struct SessionTabDragPreview {
    payload: SessionTabDragPayload,
    position: gpui::Point<gpui::Pixels>,
}

impl SessionTabDragPreview {
    pub(in crate::features) fn new(
        payload: SessionTabDragPayload,
        position: gpui::Point<gpui::Pixels>,
    ) -> Self {
        Self { payload, position }
    }
}

impl Render for SessionTabDragPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .pl(self.position.x - px(94.))
            .pt(self.position.y - px(18.))
            .child(
                div()
                    .w(px(188.))
                    .h(px(36.))
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(0x334155))
                    .bg(rgba(0x151b24dd))
                    .shadow_lg()
                    .child(div().size(px(8.)).rounded_full().bg(rgb(0x6ee7b7)))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight(700.))
                                    .text_color(rgb(0xe5edf7))
                                    .overflow_hidden()
                                    .child(truncate_preview(&self.payload.display_name, 28)),
                            )
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .text_color(rgb(0x8f98aa))
                                    .child(self.payload.kind_label),
                            ),
                    ),
            )
    }
}

/// Hover tooltip for session tabs (Tauri TabBar host / SSH address).
pub(in crate::features) struct SessionTabTooltip {
    pub title: String,
    pub lines: Vec<String>,
}

impl SessionTabTooltip {
    pub(in crate::features) fn new(title: String, lines: Vec<String>) -> Self {
        Self { title, lines }
    }
}

impl Render for SessionTabTooltip {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut body = div().flex().flex_col().gap_1().child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight(700.))
                .text_color(rgb(0xe5edf7))
                .child(self.title.clone()),
        );
        for line in &self.lines {
            let copyable = line_looks_copyable(line);
            let copy_value = line.clone();
            let row = div()
                .flex()
                .items_center()
                .gap_2()
                .min_w_0()
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .text_size(px(11.))
                        .text_color(rgb(0x8f98aa))
                        .overflow_hidden()
                        .child(line.clone()),
                )
                .when(copyable, |this| {
                    this.child(
                        div()
                            .id(SharedString::from(format!(
                                "tab-tooltip-copy-{}",
                                copy_value.chars().take(24).collect::<String>()
                            )))
                            .size(px(18.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .text_size(px(10.))
                            .text_color(rgb(0x8f98aa))
                            .hover(|style| style.bg(rgb(0x334155)).text_color(rgb(0xe5edf7)))
                            .cursor_pointer()
                            .child(mono_icon("icons/copy.svg", rgb(0x8f98aa).into(), 11.))
                            .on_click(cx.listener(move |_, _, _, cx| {
                                cx.stop_propagation();
                                cx.write_to_clipboard(ClipboardItem::new_string(
                                    copy_value.clone(),
                                ));
                            })),
                    )
                });
            body = body.child(row);
        }
        div()
            .px_3()
            .py_2()
            .max_w(px(280.))
            .rounded_md()
            .border_1()
            .border_color(rgb(0x334155))
            .bg(rgba(0x151b24f2))
            .shadow_lg()
            .child(body)
    }
}

/// Compact chrome tooltip (activity bar / bottom panel / settings).
pub(in crate::features) struct ChromeTooltip {
    pub title: String,
    pub detail: Option<String>,
}

impl ChromeTooltip {
    pub(in crate::features) fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            detail: None,
        }
    }
}

impl Render for ChromeTooltip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let mut body = div().flex().flex_col().gap_1().child(
            div()
                .text_size(px(12.))
                .font_weight(FontWeight(700.))
                .text_color(rgb(0xe5edf7))
                .child(self.title.clone()),
        );
        if let Some(detail) = &self.detail {
            body = body.child(
                div()
                    .text_size(px(11.))
                    .text_color(rgb(0x8f98aa))
                    .child(detail.clone()),
            );
        }
        div()
            .px_3()
            .py_2()
            .max_w(px(240.))
            .rounded_md()
            .border_1()
            .border_color(rgb(0x334155))
            .bg(rgba(0x151b24f2))
            .shadow_lg()
            .child(body)
    }
}

fn line_looks_copyable(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    if trimmed.starts_with("cwd ") || trimmed.starts_with("Disconnected") {
        return false;
    }
    // host, endpoint labels, ssh -p user@host
    true
}

#[derive(Debug, Clone, Copy)]
pub(in crate::features) enum TabMouseActionTarget {
    Double,
    Middle,
    Right,
}

impl NyaTermApp {
    pub(in crate::features) fn set_tab_mouse_action(
        &mut self,
        target: TabMouseActionTarget,
        action: &str,
        cx: &mut Context<Self>,
    ) {
        let action = normalize_tab_mouse_action(action);
        match target {
            TabMouseActionTarget::Double => {
                if self.settings.summary().interaction_tab_double_click_action == action {
                    return;
                }
                self.settings
                    .set_tab_double_click_action(action.to_string());
            }
            TabMouseActionTarget::Middle => {
                if self.settings.summary().interaction_tab_middle_click_action == action {
                    return;
                }
                self.settings
                    .set_tab_middle_click_action(action.to_string());
            }
            TabMouseActionTarget::Right => {
                if self.settings.summary().interaction_tab_right_click_action == action {
                    return;
                }
                self.settings.set_tab_right_click_action(action.to_string());
            }
        }
        self.save_interaction_settings(cx);
    }

    pub(in crate::features) fn handle_session_tab_click(
        &mut self,
        session_id: String,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let ClickEvent::Mouse(mouse) = event {
            let is_middle_click =
                mouse.down.button == MouseButton::Middle && mouse.up.button == MouseButton::Middle;
            if is_middle_click {
                cx.stop_propagation();
                let action = self
                    .settings
                    .summary()
                    .interaction_tab_middle_click_action
                    .clone();
                if action == "none" {
                    self.shell
                        .set_status("middle-click tab action is disabled".to_string());
                    cx.notify();
                } else {
                    self.run_tab_mouse_action(session_id, action, window, cx);
                }
                return;
            }

            if event.is_right_click() {
                cx.stop_propagation();
                let action = self
                    .settings
                    .summary()
                    .interaction_tab_right_click_action
                    .clone();
                if action == "none" {
                    let anchor = if let ClickEvent::Mouse(mouse) = event {
                        Some((
                            f32::from(mouse.up.position.x),
                            f32::from(mouse.up.position.y),
                        ))
                    } else {
                        None
                    };
                    self.open_tab_actions_at(session_id, anchor, window, cx);
                } else {
                    self.run_tab_mouse_action(session_id, action, window, cx);
                }
                return;
            }

            let is_left_double_click = mouse.down.button == MouseButton::Left
                && mouse.up.button == MouseButton::Left
                && event.click_count() >= 2;
            if is_left_double_click {
                let action = self
                    .settings
                    .summary()
                    .interaction_tab_double_click_action
                    .clone();
                if action != "none" {
                    cx.stop_propagation();
                    self.run_tab_mouse_action(session_id, action, window, cx);
                    return;
                }
            }
        }

        self.select_session(session_id, cx);
    }

    pub(in crate::features) fn reorder_session_before(
        &mut self,
        dragged_session_id: String,
        target_session_id: String,
        cx: &mut Context<Self>,
    ) {
        if dragged_session_id == target_session_id {
            return;
        }
        let sessions = self.ordered_sessions();
        let Some(target_index) = sessions
            .iter()
            .position(|session| session.id == target_session_id)
        else {
            self.shell
                .set_status("drop target session no longer exists".to_string());
            cx.notify();
            return;
        };
        let Some(source_index) = sessions
            .iter()
            .position(|session| session.id == dragged_session_id)
        else {
            self.shell
                .set_status("dragged session no longer exists".to_string());
            cx.notify();
            return;
        };
        let next_index = if source_index < target_index {
            target_index.saturating_sub(1)
        } else {
            target_index
        };
        self.move_session_to_index(&dragged_session_id, next_index);
        self.shell
            .set_status(format!("moved tab before {}", short_id(&target_session_id)));
        if self.session.restore_is_complete() {
            self.persist_open_tabs();
        } else if self.terminal_windows_is_multi_leaf() {
            self.persist_terminal_window_layout();
        }
        cx.notify();
    }

    pub(in crate::features) fn reorder_session_to_end(
        &mut self,
        dragged_session_id: String,
        cx: &mut Context<Self>,
    ) {
        let sessions = self.ordered_sessions();
        if !sessions
            .iter()
            .any(|session| session.id == dragged_session_id)
        {
            self.shell
                .set_status("dragged session no longer exists".to_string());
            cx.notify();
            return;
        }
        let last_index = sessions.len().saturating_sub(1);
        self.move_session_to_index(&dragged_session_id, last_index);
        self.shell.set_status("moved tab to end".to_string());
        if self.session.restore_is_complete() {
            self.persist_open_tabs();
        } else if self.terminal_windows_is_multi_leaf() {
            self.persist_terminal_window_layout();
        }
        cx.notify();
    }

    fn run_tab_mouse_action(
        &mut self,
        session_id: String,
        action: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if action == "close_tab" {
            self.close_session(session_id, cx);
            return;
        }

        self.select_session(session_id.clone(), cx);
        if self.session.active_id() != Some(session_id.as_str()) {
            return;
        }

        match action.as_str() {
            "none" => {}
            "rename_tab" => self.open_rename_session(session_id, window, cx),
            "copy_tab_name" => self.copy_active_session_name(cx),
            "copy_server_ip" => self.copy_active_session_ssh_host(cx),
            "duplicate_session" => self.duplicate_active_session(window, cx),
            "multiplex_ssh" => self.multiplex_active_ssh_session(window, cx),
            "reconnect_session" => self.reconnect_active_session(window, cx),
            "disconnect_session" => self.disconnect_session(session_id, cx),
            _ => {
                self.shell
                    .set_status(format!("unknown tab action '{action}'"));
                cx.notify();
            }
        }
    }
}

pub(in crate::features) const TAB_MOUSE_ACTIONS: [&str; 9] = [
    "none",
    "rename_tab",
    "copy_tab_name",
    "copy_server_ip",
    "duplicate_session",
    "multiplex_ssh",
    "reconnect_session",
    "disconnect_session",
    "close_tab",
];

fn normalize_tab_mouse_action(action: &str) -> &'static str {
    TAB_MOUSE_ACTIONS
        .iter()
        .copied()
        .find(|item| *item == action)
        .unwrap_or("none")
}

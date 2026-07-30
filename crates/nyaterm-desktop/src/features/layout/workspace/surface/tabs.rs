use std::time::{Duration, Instant};

use gpui::{
    AnimationExt, ClickEvent, Context, FontWeight, IntoElement, MouseButton, SharedString, div,
    prelude::*, px, rgb, rgba, svg,
};
use nyaterm_core::truncate_preview;

use crate::features::NyaTermApp;
use crate::features::formatting::session_kind_label;
use crate::features::shell::{SessionTabDragPayload, SessionTabDragPreview, SessionTabTooltip};

use super::super::super::view_helpers::session_kind_icon_path;

fn pending_tab_insert_index(
    session_count: usize,
    after_position: Option<usize>,
    insert_index: Option<usize>,
) -> usize {
    insert_index
        .or_else(|| after_position.map(|index| index + 1))
        .unwrap_or(session_count)
        .min(session_count)
}

type TransientSessionTab = (usize, Instant, String, String, String, Option<String>);

impl NyaTermApp {
    fn pending_session_tab(
        &mut self,
        request_id: String,
        pending_name: String,
        tab_number: usize,
        active: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let hover_bg = self.shell_surface_color(palette.hover);
        let close_request_id = request_id.clone();
        let spinner_id = SharedString::from(format!("pending-session-spinner-{request_id}"));
        div()
            .id(SharedString::from(format!(
                "pending-session-tab-{request_id}"
            )))
            .h_full()
            .min_w(px(118.))
            .max_w(px(236.))
            .px_3()
            .flex()
            .items_center()
            .gap_2()
            .relative()
            .border_r_1()
            .border_color(rgb(palette.border))
            .bg(if active {
                self.shell_surface_color(palette.hover)
            } else {
                self.shell_surface_color(palette.bg)
            })
            .cursor_pointer()
            .hover(move |this| this.bg(hover_bg))
            .when(active, |this| {
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .h(px(2.))
                        .bg(rgb(palette.primary)),
                )
            })
            .child(
                div()
                    .size(px(14.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        svg()
                            .size(px(12.))
                            .path("icons/conn/connect.svg")
                            .text_color(rgb(palette.primary))
                            .with_animation(
                                spinner_id,
                                gpui::Animation::new(Duration::from_millis(900)).repeat(),
                                |svg, delta| {
                                    svg.with_transformation(gpui::Transformation::rotate(
                                        gpui::percentage(delta),
                                    ))
                                },
                            ),
                    ),
            )
            .child(
                div()
                    .min_w(px(12.))
                    .text_size(px(11.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(palette.text_muted))
                    .child(format!("{tab_number}")),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .text_size(px(12.))
                    .font_weight(if active {
                        FontWeight(600.)
                    } else {
                        FontWeight(500.)
                    })
                    .text_color(if active {
                        rgb(palette.text)
                    } else {
                        rgb(palette.text_muted)
                    })
                    .overflow_hidden()
                    .child(truncate_preview(&pending_name, 28)),
            )
            .child(
                div()
                    .id(SharedString::from(format!(
                        "pending-session-tab-close-{close_request_id}"
                    )))
                    .group(SharedString::from(format!(
                        "pending-session-tab-close-group-{close_request_id}"
                    )))
                    .size(px(18.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_sm()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .hover(|this| this.bg(rgb(palette.border)).text_color(rgb(palette.danger)))
                    .child(
                        svg()
                            .size(px(13.))
                            .path("icons/window/close.svg")
                            .text_color(rgb(palette.text_muted))
                            .group_hover(
                                SharedString::from(format!(
                                    "pending-session-tab-close-group-{close_request_id}"
                                )),
                                |this| this.text_color(rgb(palette.danger)),
                            ),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.close_pending_session_start(close_request_id.clone(), cx);
                    })),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.select_pending_session_start(request_id.clone(), cx);
            }))
    }

    fn failed_session_tab(
        &mut self,
        request_id: String,
        failed_name: String,
        failed_error: String,
        tab_number: usize,
        active: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let close_request_id = request_id.clone();
        let select_request_id = request_id.clone();
        div()
            .id(SharedString::from(format!(
                "failed-session-tab-{request_id}"
            )))
            .h_full()
            .min_w(px(118.))
            .max_w(px(236.))
            .px_3()
            .flex()
            .items_center()
            .gap_2()
            .relative()
            .border_r_1()
            .border_color(rgb(palette.border))
            .bg(if active {
                rgba((palette.danger << 8) | 0x24)
            } else {
                rgba((palette.danger << 8) | 0x12)
            })
            .cursor_pointer()
            .hover(|this| this.bg(rgba((palette.danger << 8) | 0x24)))
            .when(active, |this| {
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .h(px(2.))
                        .bg(rgb(palette.danger)),
                )
            })
            .child(
                div()
                    .size(px(14.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        svg()
                            .size(px(12.))
                            .path("icons/session/disconnect.svg")
                            .text_color(rgb(palette.danger)),
                    ),
            )
            .child(
                div()
                    .min_w(px(12.))
                    .text_size(px(11.))
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(palette.text_muted))
                    .child(format!("{tab_number}")),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .text_size(px(12.))
                    .font_weight(if active {
                        FontWeight(600.)
                    } else {
                        FontWeight(500.)
                    })
                    .text_color(rgb(palette.danger))
                    .overflow_hidden()
                    .child(truncate_preview(&failed_name, 28)),
            )
            .child(
                div()
                    .id(SharedString::from(format!(
                        "failed-session-tab-close-{close_request_id}"
                    )))
                    .group(SharedString::from(format!(
                        "failed-session-tab-close-group-{close_request_id}"
                    )))
                    .size(px(18.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_sm()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .hover(|this| this.bg(rgb(palette.border)).text_color(rgb(palette.danger)))
                    .child(
                        svg()
                            .size(px(13.))
                            .path("icons/window/close.svg")
                            .text_color(rgb(palette.text_muted))
                            .group_hover(
                                SharedString::from(format!(
                                    "failed-session-tab-close-group-{close_request_id}"
                                )),
                                |this| this.text_color(rgb(palette.danger)),
                            ),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.close_failed_session_start(close_request_id.clone(), cx);
                    })),
            )
            .tooltip(move |_, cx| {
                cx.new(|_| SessionTabTooltip::new(failed_name.clone(), vec![failed_error.clone()]))
                    .into()
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.select_failed_session_start(select_request_id.clone(), cx);
            }))
    }

    pub(in crate::features) fn main_surface(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        // The main surface always hosts the terminal workspace. Side panels are
        // rendered by the shell around this surface to match the Tauri layout.
        let palette = self.theme_palette();
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .bg(self.shell_transparent_color(palette.bg))
            .child(self.workspace_view(cx))
    }

    pub(in crate::features) fn session_tab_strip(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let shell_hover_bg = self.shell_surface_color(palette.hover);
        let sessions = self.ordered_tab_sessions();
        let session_count = sessions.len();
        let mut transient_tabs: Vec<TransientSessionTab> = self
            .session
            .start_pending_entries()
            .filter_map(|(request_id, pending)| {
                if pending.reconnect_session_id.is_some() {
                    return None;
                }
                let name = pending
                    .custom_name
                    .as_deref()
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .unwrap_or(&pending.connection_name)
                    .to_string();
                let after_position = pending.after_session_id.as_ref().and_then(|after_id| {
                    sessions.iter().position(|session| session.id == *after_id)
                });
                let index =
                    pending_tab_insert_index(session_count, after_position, pending.insert_index);
                Some((
                    index,
                    pending.requested_at,
                    pending.connection_name.clone(),
                    request_id.clone(),
                    name,
                    None,
                ))
            })
            .collect::<Vec<_>>();
        transient_tabs.extend(
            self.session
                .start_failed_entries()
                .map(|(request_id, failed)| {
                    let pending = &failed.pending;
                    let name = pending
                        .custom_name
                        .as_deref()
                        .map(str::trim)
                        .filter(|name| !name.is_empty())
                        .unwrap_or(&pending.connection_name)
                        .to_string();
                    let after_position = pending.after_session_id.as_ref().and_then(|after_id| {
                        sessions.iter().position(|session| session.id == *after_id)
                    });
                    let index = pending_tab_insert_index(
                        session_count,
                        after_position,
                        pending.insert_index,
                    );
                    (
                        index,
                        pending.requested_at,
                        pending.connection_name.clone(),
                        request_id.clone(),
                        name,
                        Some(failed.error.clone()),
                    )
                }),
        );
        transient_tabs.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| left.2.cmp(&right.2))
                .then_with(|| left.3.cmp(&right.3))
        });
        if self.shell.session_tab_scroll_into_view_pending() {
            if let Some(active_id) = self.session.active_id()
                && let Some(index) = sessions.iter().position(|session| session.id == active_id)
            {
                let pending_count = transient_tabs
                    .iter()
                    .filter(|(pending_index, _, _, _, _, _)| *pending_index <= index)
                    .count();
                let child_index = index + pending_count;
                self.shell
                    .session_tab_strip_scroll()
                    .scroll_to_item(child_index);
            }
            self.shell.consume_session_tab_scroll_into_view();
        }
        let mut tabs = div()
            .id("session-tab-strip-scroll")
            .h_full()
            .min_w_0()
            .flex_1()
            .flex()
            .items_center()
            // Tauri tab-strip-scroll: horizontal overflow instead of clipping tabs.
            .overflow_x_scroll()
            .overflow_y_hidden()
            .track_scroll(self.shell.session_tab_strip_scroll());

        let mut transient_cursor = 0usize;
        for (tab_index, session) in sessions.into_iter().enumerate() {
            while transient_cursor < transient_tabs.len()
                && transient_tabs[transient_cursor].0 == tab_index
            {
                let (_, _, _, request_id, name, error) = transient_tabs[transient_cursor].clone();
                let tab_number = tab_index + transient_cursor + 1;
                let active = self.session.start_request_is_active(&request_id);
                tabs = tabs.child(match error {
                    Some(error) => self
                        .failed_session_tab(request_id, name, error, tab_number, active, cx)
                        .into_any_element(),
                    None => self
                        .pending_session_tab(request_id, name, tab_number, active, cx)
                        .into_any_element(),
                });
                transient_cursor += 1;
            }
            let display_name = self.session.display_name_by_info(&session);
            let session_id = session.id.clone();
            let close_session_id = session.id.clone();
            let tab_group_name = SharedString::from(format!("session-tab-group-{session_id}"));
            let tab_number = tab_index + transient_cursor + 1;
            let kind_icon = session_kind_icon_path(session.kind);
            let tooltip_title = display_name.clone();
            let tooltip_lines = self.session.tab_tooltip_lines(&session.id);
            let drag_payload = SessionTabDragPayload {
                session_id: session.id.clone(),
                display_name: display_name.clone(),
                kind_label: session_kind_label(session.kind),
            };
            let drop_target_session_id = session.id.clone();
            let custom_color = self.session.tab_color(&session.id);
            // Active when any leaf under this tab root is focused.
            let is_active = self
                .session
                .active_id()
                .is_some_and(|id| self.tab_root_for_session(id) == session.id);
            let leaf_ids = self
                .shell
                .workspace_pane_root(&session.id)
                .map(|root| root.session_ids())
                .unwrap_or_else(|| vec![session.id.clone()]);
            let is_disconnected = leaf_ids.iter().any(|id| self.session.is_disconnected(id));
            let tab_title = truncate_preview(&display_name, 28);
            let has_unread = leaf_ids
                .iter()
                .any(|id| self.terminal.session_has_unread(id));
            let sync_group = leaf_ids
                .iter()
                .find_map(|id| self.sync_input.active_group_for_session(id));
            let sync_paused = leaf_ids
                .iter()
                .any(|id| self.sync_input.session_is_paused_in_active_group(id));
            let show_sync_indicator = self.sync_input.broadcast_to_all() || sync_group.is_some();
            let sync_indicator_color = sync_group
                .map(|group| group.color)
                .unwrap_or(palette.primary);
            let accent = if let Some(custom_color) = custom_color {
                rgb(custom_color)
            } else if is_disconnected {
                rgb(palette.danger)
            } else if is_active {
                rgb(palette.primary)
            } else if has_unread {
                rgb(palette.warning)
            } else {
                rgb(palette.text_dimmed)
            };
            let bg = if let Some(custom_color) = custom_color {
                rgba((custom_color << 8) | if is_active { 0x24 } else { 0x14 })
            } else if is_active {
                self.shell_surface_color(palette.hover)
            } else {
                self.shell_surface_color(palette.bg)
            };
            let hover_bg = if let Some(custom_color) = custom_color {
                rgba((custom_color << 8) | if is_active { 0x32 } else { 0x22 })
            } else {
                self.shell_surface_color(palette.hover)
            };
            tabs = tabs.child(
                div()
                    .id(SharedString::from(format!("session-tab-{session_id}")))
                    .group(tab_group_name.clone())
                    .h_full()
                    .min_w(px(118.))
                    .max_w(px(236.))
                    .px_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .relative()
                    .when(is_active, |this| {
                        this.child(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .h(px(2.))
                                .w_full()
                                .bg(accent),
                        )
                    })
                    .border_r_1()
                    .border_color(if is_active {
                        custom_color.map(rgb).unwrap_or_else(|| rgb(palette.border))
                    } else {
                        rgb(palette.border)
                    })
                    .bg(bg)
                    .when(is_disconnected, |this| this.opacity(0.78))
                    .cursor_pointer()
                    .hover(move |this| this.bg(hover_bg))
                    .cursor_move()
                    .on_drag(drag_payload, |payload, position, _, cx| {
                        cx.new(|_| SessionTabDragPreview::new(payload.clone(), position))
                    })
                    .on_drop(
                        cx.listener(move |this, payload: &SessionTabDragPayload, _, cx| {
                            this.reorder_session_before(
                                payload.session_id.clone(),
                                drop_target_session_id.clone(),
                                cx,
                            );
                        }),
                    )
                    .when(custom_color.is_some(), move |this| {
                        this.child(
                            div()
                                .absolute()
                                .top_0()
                                .bottom_0()
                                .left_0()
                                .w(px(3.))
                                .bg(accent),
                        )
                    })
                    // Tauri tab: top accent when active, icon + name + close.
                    .when(is_active, |this| {
                        this.child(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .right_0()
                                .h(px(2.))
                                .bg(accent),
                        )
                        .child(
                            // Cover tab strip bottom border so the active tab blends into the terminal.
                            div()
                                .absolute()
                                .bottom_0()
                                .left_0()
                                .right_0()
                                .h(px(1.))
                                .bg(self.shell_surface_color(palette.bg)),
                        )
                    })
                    .child(
                        div()
                            .size(px(14.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(svg().size(px(12.)).path(kind_icon).text_color(accent)),
                    )
                    .child(
                        div()
                            .min_w(px(12.))
                            .text_size(px(11.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(format!("{tab_number}")),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(12.))
                            .font_weight(if is_active {
                                FontWeight(600.)
                            } else {
                                FontWeight(500.)
                            })
                            .text_color(if is_disconnected {
                                rgb(palette.text_dimmed)
                            } else if is_active {
                                rgb(palette.text)
                            } else {
                                rgb(palette.text_muted)
                            })
                            .overflow_hidden()
                            // Without this the title wraps inside the tab and the
                            // strip shows whichever line happens to land on the
                            // one row of height it has — "ste", out of "System32".
                            .whitespace_nowrap()
                            .text_ellipsis()
                            .child(tab_title.clone()),
                    )
                    .when(show_sync_indicator, |this| {
                        this.child(
                            div()
                                .size(px(14.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .opacity(if sync_paused { 0.4 } else { 1. })
                                .child(
                                    svg()
                                        .size(px(11.))
                                        .path("icons/sync.svg")
                                        .text_color(rgb(sync_indicator_color)),
                                ),
                        )
                    })
                    .when(has_unread && !is_active, |this| {
                        this.child(div().size(px(8.)).rounded_full().bg(rgb(palette.success)))
                    })
                    .child(
                        div()
                            .id(SharedString::from(format!(
                                "session-tab-close-{close_session_id}"
                            )))
                            .size(px(18.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .when(!is_active, |this| {
                                this.opacity(0.)
                                    .group_hover(tab_group_name.clone(), |style| style.opacity(1.))
                            })
                            .hover(|this| {
                                this.bg(rgb(palette.border)).text_color(rgb(palette.danger))
                            })
                            .child(
                                svg()
                                    .size(px(13.))
                                    .path("icons/window/close.svg")
                                    .text_color(rgb(palette.text_muted)),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.close_session(close_session_id.clone(), cx);
                            })),
                    )
                    .tooltip(move |_, cx| {
                        cx.new(|_| {
                            SessionTabTooltip::new(tooltip_title.clone(), tooltip_lines.clone())
                        })
                        .into()
                    })
                    .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                        this.handle_session_tab_click(session_id.clone(), event, window, cx);
                    })),
            );
        }
        while transient_cursor < transient_tabs.len() {
            let (_, _, _, request_id, name, error) = transient_tabs[transient_cursor].clone();
            let tab_number = session_count + transient_cursor + 1;
            let active = self.session.start_request_is_active(&request_id);
            tabs = tabs.child(match error {
                Some(error) => self
                    .failed_session_tab(request_id, name, error, tab_number, active, cx)
                    .into_any_element(),
                None => self
                    .pending_session_tab(request_id, name, tab_number, active, cx)
                    .into_any_element(),
            });
            transient_cursor += 1;
        }

        if session_count > 1 {
            tabs = tabs.child(
                div()
                    .id("session-tab-drop-end")
                    .h_full()
                    .min_w(px(28.))
                    .flex_none()
                    .border_l_1()
                    .border_color(rgb(palette.border))
                    .hover(move |this| this.bg(shell_hover_bg))
                    .on_drop(cx.listener(|this, payload: &SessionTabDragPayload, _, cx| {
                        this.reorder_session_to_end(payload.session_id.clone(), cx);
                    })),
            );
        }

        // Tauri TabBar trailing chrome: optional open-tabs overflow menu + new session menu.
        let open_tabs_menu = self.shell.open_tabs_menu_is_open();
        let new_session_menu = self.shell.new_session_menu_is_open();
        let open_tabs_label = self.tr("terminal.openTabs").to_string();
        let new_session_label = self.tr("terminal.newSession").to_string();
        let tab_strip_has_overflow =
            self.shell.session_tab_strip_scroll().max_offset().width > px(0.);
        // Tauri shows Open Tabs only when the strip actually overflows.
        let show_open_tabs_menu = tab_strip_has_overflow || open_tabs_menu;

        let mut session_actions = div()
            .h_full()
            .flex()
            .items_center()
            .gap_0()
            .border_l_1()
            .border_color(rgb(palette.border));

        if show_open_tabs_menu {
            session_actions = session_actions.child(
                div()
                    .relative()
                    .h_full()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_, _, _, cx| cx.stop_propagation()),
                    )
                    .child(
                        div()
                            .id("workspace-open-tabs-menu")
                            .h_full()
                            .w(px(32.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .border_r_1()
                            .border_color(rgb(palette.border))
                            .bg(if open_tabs_menu {
                                self.shell_surface_color(palette.hover)
                            } else {
                                rgba(0x00000000)
                            })
                            .text_color(rgb(palette.text_muted))
                            .cursor_pointer()
                            .hover(move |this| {
                                this.bg(shell_hover_bg).text_color(rgb(palette.text))
                            })
                            .child(
                                svg()
                                    .size(px(16.))
                                    .flex_none()
                                    .path("icons/chevron-down.svg")
                                    .text_color(rgb(palette.text_muted)),
                            )
                            .tooltip(move |_, cx| {
                                cx.new(|_| {
                                    crate::features::ChromeTooltip::new(open_tabs_label.clone())
                                })
                                .into()
                            })
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.toggle_open_tabs_menu(cx);
                            })),
                    )
                    .when(open_tabs_menu, |this| {
                        this.child(self.render_open_tabs_menu(cx))
                    }),
            );
        }

        session_actions = session_actions.child(
            div()
                .relative()
                .h_full()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|_, _, _, cx| cx.stop_propagation()),
                )
                .child(
                    div()
                        .id("workspace-new-session-menu")
                        .h_full()
                        .w(px(36.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .border_r_1()
                        .border_color(rgb(palette.border))
                        .bg(if new_session_menu {
                            self.shell_surface_color(palette.hover)
                        } else {
                            rgba(0x00000000)
                        })
                        .text_color(rgb(palette.text_muted))
                        .cursor_pointer()
                        .hover(move |this| this.bg(shell_hover_bg).text_color(rgb(palette.text)))
                        .child(
                            svg()
                                .size(px(16.))
                                .flex_none()
                                .path("icons/conn/add.svg")
                                .text_color(rgb(palette.text_muted)),
                        )
                        .tooltip(move |_, cx| {
                            cx.new(|_| {
                                crate::features::ChromeTooltip::new(new_session_label.clone())
                            })
                            .into()
                        })
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.toggle_new_session_menu(cx);
                        })),
                )
                .when(new_session_menu, |this| {
                    this.child(self.render_new_session_menu(cx))
                }),
        );

        div()
            .h(px(36.)) // Tauri TabBar: h-9
            .flex()
            .items_center()
            .border_b_1()
            .border_color(rgb(palette.border))
            .bg(self.shell_surface_color(palette.surface))
            .child(tabs)
            .child(session_actions)
    }
}

#[cfg(test)]
mod tests {
    use super::pending_tab_insert_index;

    #[test]
    fn pending_tab_position_matches_tauri_insertion_rules() {
        assert_eq!(pending_tab_insert_index(3, None, None), 3);
        assert_eq!(pending_tab_insert_index(3, Some(0), None), 1);
        assert_eq!(pending_tab_insert_index(3, Some(0), Some(2)), 2);
        assert_eq!(pending_tab_insert_index(3, None, Some(99)), 3);
    }
}

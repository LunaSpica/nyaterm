use super::*;

impl NyaTermApp {
    pub(in crate::features) fn sync_groups_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let selected_group = self.selected_sync_group().cloned();
        let selected_group_id = selected_group.as_ref().map(|group| group.id.clone());
        let mut group_list = div().flex().flex_col().gap_2();
        if self.sync_groups.is_empty() {
            group_list = group_list.child(
                div()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.input))
                    .p_3()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .child("No sync groups."),
            );
        }
        for group in self.sync_groups.clone() {
            let selected = selected_group_id.as_deref() == Some(group.id.as_str());
            let session_count = group.session_ids.len();
            group_list = group_list.child(
                div()
                    .id(SharedString::from(format!("sync-group-{}", group.id)))
                    .rounded_sm()
                    .border_1()
                    .border_color(if selected {
                        rgb(0x3b82f6)
                    } else {
                        rgb(palette.border)
                    })
                    .bg(if selected {
                        rgb(palette.hover)
                    } else {
                        rgb(palette.input)
                    })
                    .p_3()
                    .cursor_pointer()
                    .hover(|this| this.bg(rgb(0x151b24)))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .min_w_0()
                                    .child(div().size(px(10.)).rounded_full().bg(rgb(group.color)))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .text_sm()
                                            .font_weight(FontWeight(800.))
                                            .text_color(rgb(palette.text))
                                            .child(truncate_preview(&group.name, 26)),
                                    ),
                            )
                            .child(status_pill(
                                if group.enabled { "on" } else { "off" },
                                if group.enabled {
                                    rgb(palette.success)
                                } else {
                                    rgb(palette.text_muted)
                                },
                                if group.enabled {
                                    rgb(palette.hover)
                                } else {
                                    rgb(palette.border)
                                },
                            )),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(format!("{session_count} session(s)")),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select_sync_group(group.id.clone(), cx);
                    })),
            );
        }

        let selected_members = selected_group
            .as_ref()
            .map(|group| group.session_ids.iter().cloned().collect::<HashSet<_>>())
            .unwrap_or_default();
        let selected_paused = selected_group
            .as_ref()
            .map(|group| {
                group
                    .paused_session_ids
                    .iter()
                    .cloned()
                    .collect::<HashSet<_>>()
            })
            .unwrap_or_default();
        let mut session_rows = div().flex().flex_col().gap_2();
        let sessions = self.ordered_sessions();
        if sessions.is_empty() {
            session_rows = session_rows.child(
                div()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.input))
                    .p_3()
                    .text_xs()
                    .text_color(rgb(palette.text_muted))
                    .child("Start sessions before building a sync group."),
            );
        }
        for session in sessions {
            let session_id = session.id.clone();
            let in_group = selected_members.contains(&session_id);
            let paused = selected_paused.contains(&session_id);
            let active = self.active_session_id.as_deref() == Some(session_id.as_str());
            let title = self.session_display_name_by_info(&session);
            session_rows = session_rows.child(
                div()
                    .id(SharedString::from(format!("sync-session-{session_id}")))
                    .rounded_sm()
                    .border_1()
                    .border_color(if in_group {
                        rgb(palette.border)
                    } else {
                        rgb(palette.border)
                    })
                    .bg(if in_group {
                        rgb(0x111827)
                    } else {
                        rgb(palette.input)
                    })
                    .p_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight(800.))
                                            .text_color(if paused {
                                                rgb(palette.text_muted)
                                            } else {
                                                rgb(palette.text)
                                            })
                                            .child(truncate_preview(&title, 42)),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(palette.text_muted))
                                            .child(format!(
                                                "{}{}",
                                                session_kind_label(session.kind),
                                                if active { " · active" } else { "" }
                                            )),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(status_pill(
                                        if paused {
                                            "paused"
                                        } else if in_group {
                                            "sync"
                                        } else {
                                            "out"
                                        },
                                        if paused {
                                            rgb(0xfacc15)
                                        } else if in_group {
                                            rgb(palette.success)
                                        } else {
                                            rgb(palette.text_muted)
                                        },
                                        if paused {
                                            rgb(0x3a2f14)
                                        } else if in_group {
                                            rgb(palette.hover)
                                        } else {
                                            rgb(palette.border)
                                        },
                                    ))
                                    .child(small_button(palette,
                                        format!("sync-session-toggle-{session_id}"),
                                        if in_group { "Remove" } else { "Add" },
                                        cx.listener({
                                            let session_id = session_id.clone();
                                            move |this, _, _, cx| {
                                                this.toggle_session_in_selected_sync_group(
                                                    session_id.clone(),
                                                    cx,
                                                );
                                            }
                                        }),
                                    ))
                                    .when(in_group, |this| {
                                        this.child(small_button(palette,
                                            format!("sync-session-pause-{session_id}"),
                                            if paused { "Resume" } else { "Pause" },
                                            cx.listener({
                                                let session_id = session_id.clone();
                                                move |this, _, _, cx| {
                                                    this.toggle_session_paused_in_selected_sync_group(
                                                        session_id.clone(),
                                                        cx,
                                                    );
                                                }
                                            }),
                                        ))
                                    }),
                            ),
                    ),
            );
        }

        div()
            .id(SharedString::from("sync-groups-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x030508e6))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.sync_groups_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.sync_groups_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                match event.keystroke.key.as_str() {
                    "escape" => this.close_sync_groups(cx),
                    "n" | "N" => this.create_sync_group(cx),
                    "delete" | "backspace" => this.delete_selected_sync_group(cx),
                    _ => {}
                }
            }))
            .child(
                div()
                    .id(SharedString::from("sync-groups-dialog"))
                    .w(px(760.))
                    .max_w_full()
                    .mx_4()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .p_4()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .flex()
                            .items_start()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight(800.))
                                            .text_color(rgb(palette.text))
                                            .child("Sync Input Groups"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(palette.text_muted))
                                            .child("Keyboard input and sent commands broadcast to active peers."),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(small_button(palette,
                                        "sync-group-new",
                                        "New",
                                        cx.listener(|this, _, _, cx| {
                                            this.create_sync_group(cx);
                                        }),
                                    ))
                                    .child(small_button(palette,
                                        "sync-group-close",
                                        "Close",
                                        cx.listener(|this, _, _, cx| {
                                            this.close_sync_groups(cx);
                                        }),
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .mt_4()
                            .grid()
                            .grid_cols(3)
                            .gap_3()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight(800.))
                                            .text_color(rgb(palette.text_muted))
                                            .child("Groups"),
                                    )
                                    .child(group_list),
                            )
                            .child(
                                div()
                                    .col_span(2)
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight(800.))
                                                    .text_color(rgb(palette.text_muted))
                                                    .child("Sessions"),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .items_center()
                                                    .gap_2()
                                                    .child(small_button(palette,
                                                        "sync-group-toggle",
                                                        if selected_group
                                                            .as_ref()
                                                            .is_some_and(|group| group.enabled)
                                                        {
                                                            "Disable"
                                                        } else {
                                                            "Enable"
                                                        },
                                                        cx.listener(|this, _, _, cx| {
                                                            this.toggle_selected_sync_group_enabled(cx);
                                                        }),
                                                    ))
                                                    .child(small_button(palette,
                                                        "sync-group-delete",
                                                        "Delete",
                                                        cx.listener(|this, _, _, cx| {
                                                            this.delete_selected_sync_group(cx);
                                                        }),
                                                    )),
                                            ),
                                    )
                                    .child(session_rows),
                            ),
                    ),
            )
    }
}

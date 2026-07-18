use super::*;

impl NyaTermApp {
    pub(in crate::features) fn sync_groups_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let search_input_entity = cx.entity();
        let name_input_entity = search_input_entity.clone();
        let selected_group = self.selected_sync_group().cloned();
        let selected_group_id = selected_group.as_ref().map(|group| group.id.clone());
        let selected_group_name = selected_group
            .as_ref()
            .map(|group| format!("{}{}", group.name, self.sync_groups_name_marked_text))
            .unwrap_or_default();
        let search_display = if self.sync_groups_search_draft.is_empty()
            && self.sync_groups_search_marked_text.is_empty()
        {
            self.tr("syncGroup.searchPlaceholder").to_string()
        } else {
            format!(
                "{}{}",
                self.sync_groups_search_draft, self.sync_groups_search_marked_text
            )
        };
        let pending_delete_name = self
            .sync_groups_delete_pending
            .as_deref()
            .and_then(|id| self.sync_groups.iter().find(|group| group.id == id))
            .map(|group| group.name.clone());
        let pending_delete_message = pending_delete_name.as_ref().map(|name| {
            self.tr("syncGroup.deleteGroupConfirm")
                .replace("{{name}}", name)
        });
        let mut group_list = div()
            .id(SharedString::from("sync-groups-list"))
            .max_h(px(350.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2();
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
                    .child(self.tr("syncGroup.noGroups")),
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
                                    .child(
                                        div()
                                            .w(px(6.))
                                            .h(px(36.))
                                            .rounded_full()
                                            .bg(rgb(group.color)),
                                    )
                                    .child(
                                        div()
                                            .min_w_0()
                                            .text_sm()
                                            .font_weight(FontWeight(800.))
                                            .text_color(rgb(palette.text))
                                            .child(truncate_preview(&group.name, 26)),
                                    ),
                            )
                            .child(div().size(px(8.)).rounded_full().bg(rgb(if group.enabled {
                                palette.success
                            } else {
                                palette.text_muted
                            }))),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(
                                self.tr("syncGroup.sessionCount")
                                    .replace("{{count}}", &session_count.to_string()),
                            ),
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
        let mut session_rows = div()
            .id(SharedString::from("sync-sessions-list"))
            .max_h(px(290.))
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_2();
        let query = self.sync_groups_search_draft.trim().to_ascii_lowercase();
        let all_sessions = self.ordered_sessions();
        let has_sessions = !all_sessions.is_empty();
        let sessions = all_sessions
            .into_iter()
            .filter(|session| self.sync_group_session_matches_search(session, &query))
            .collect::<Vec<_>>();
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
                    .child(self.tr(if has_sessions {
                        "syncGroup.noSessionMatches"
                    } else {
                        "syncGroup.noSessions"
                    })),
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
                                                if active {
                                                    format!(
                                                        " · {}",
                                                        self.tr("sessionQuickSwitcher.active")
                                                    )
                                                } else {
                                                    String::new()
                                                }
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
                                            self.tr("syncGroup.paused")
                                        } else if in_group {
                                            self.tr("syncGroup.activeMembers")
                                        } else {
                                            self.tr("syncGroup.filterAvailable")
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
                                    .child(small_button(
                                        palette,
                                        format!("sync-session-toggle-{session_id}"),
                                        if in_group {
                                            self.tr("common.remove")
                                        } else {
                                            self.tr("common.add")
                                        },
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
                                        this.child(small_button(
                                            palette,
                                            format!("sync-session-pause-{session_id}"),
                                            if paused {
                                                self.tr("syncGroup.resumeSync")
                                            } else {
                                                self.tr("syncGroup.pauseSync")
                                            },
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
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                if this.handle_sync_groups_key_down(event, window, cx) {
                    return;
                }
                match event.keystroke.key.as_str() {
                    "escape" => this.close_sync_groups(cx),
                    "n" | "N" => this.create_sync_group(cx),
                    "delete" => this.request_delete_selected_sync_group(cx),
                    _ => {}
                }
            }))
            .child(
                div()
                    .id(SharedString::from("sync-groups-dialog"))
                    .w(px(900.))
                    .max_w_full()
                    .mx_4()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .h(px(500.))
                    .max_h_full()
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
                                            .child(self.tr("syncGroup.title")),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(rgb(palette.text_muted))
                                            .child(self.tr("syncGroup.description")),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(small_button(palette,
                                        "sync-group-new",
                                        self.tr("syncGroup.newGroup"),
                                        cx.listener(|this, _, _, cx| {
                                            this.create_sync_group(cx);
                                        }),
                                    ))
                                    .child(small_button(palette,
                                        "sync-group-close",
                                        self.tr("common.close"),
                                        cx.listener(|this, _, _, cx| {
                                            this.close_sync_groups(cx);
                                        }),
                                    )),
                            ),
                    )
                    .child(
                        div()
                            .mt_4()
                            .flex()
                            .min_h_0()
                            .gap_3()
                            .child(
                                div()
                                    .w(px(208.))
                                    .flex_none()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight(800.))
                                            .text_color(rgb(palette.text_muted))
                                            .child(self.tr("syncGroup.groups")),
                                    )
                                    .child(group_list),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .w(px(6.))
                                                    .h(px(34.))
                                                    .rounded_full()
                                                    .bg(rgb(
                                                        selected_group
                                                            .as_ref()
                                                            .map(|group| group.color)
                                                            .unwrap_or(palette.border),
                                                    )),
                                            )
                                            .child(
                                                div()
                                                    .id(SharedString::from("sync-group-name-input"))
                                                    .relative()
                                                    .h(px(34.))
                                                    .flex_1()
                                                    .min_w_0()
                                                    .px_2()
                                                    .flex()
                                                    .items_center()
                                                    .rounded_sm()
                                                    .border_1()
                                                    .border_color(rgb(palette.border))
                                                    .bg(rgb(palette.input))
                                                    .text_sm()
                                                    .text_color(rgb(palette.text))
                                                    .track_focus(&self.sync_groups_name_focus)
                                                    .on_click(cx.listener(|this, _, window, cx| {
                                                        window.focus(&this.sync_groups_name_focus);
                                                        cx.notify();
                                                    }))
                                                    .child(selected_group_name)
                                                    .child(
                                                        gpui::canvas(
                                                            |_bounds, _window, _cx| {},
                                                            move |bounds, _state, window, cx| {
                                                                window.handle_input(
                                                                    &name_input_entity
                                                                        .read(cx)
                                                                        .sync_groups_name_focus,
                                                                    gpui::ElementInputHandler::new(
                                                                        bounds,
                                                                        name_input_entity.clone(),
                                                                    ),
                                                                    cx,
                                                                );
                                                            },
                                                        )
                                                        .absolute()
                                                        .inset_0(),
                                                    ),
                                            ),
                                    )
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
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .font_weight(FontWeight(800.))
                                                            .text_color(rgb(palette.text_muted))
                                                            .child(self.tr("syncGroup.sessions")),
                                                    )
                                                    .child(
                                                        div()
                                                            .id(SharedString::from(
                                                                "sync-group-search-input",
                                                            ))
                                                            .relative()
                                                            .w(px(260.))
                                                            .h(px(30.))
                                                            .px_2()
                                                            .flex()
                                                            .items_center()
                                                            .rounded_sm()
                                                            .border_1()
                                                            .border_color(rgb(palette.border))
                                                            .bg(rgb(palette.input))
                                                            .text_xs()
                                                            .text_color(if self
                                                                .sync_groups_search_draft
                                                                .is_empty()
                                                            {
                                                                rgb(palette.text_muted)
                                                            } else {
                                                                rgb(palette.text)
                                                            })
                                                            .track_focus(
                                                                &self.sync_groups_search_focus,
                                                            )
                                                            .on_click(cx.listener(
                                                                |this, _, window, cx| {
                                                                    window.focus(
                                                                        &this.sync_groups_search_focus,
                                                                    );
                                                                    cx.notify();
                                                                },
                                                            ))
                                                            .child(search_display)
                                                            .child(
                                                                gpui::canvas(
                                                                    |_bounds, _window, _cx| {},
                                                                    move |bounds, _state, window, cx| {
                                                                        window.handle_input(
                                                                            &search_input_entity
                                                                                .read(cx)
                                                                                .sync_groups_search_focus,
                                                                            gpui::ElementInputHandler::new(
                                                                                bounds,
                                                                                search_input_entity
                                                                                    .clone(),
                                                                            ),
                                                                            cx,
                                                                        );
                                                                    },
                                                                )
                                                                .absolute()
                                                                .inset_0(),
                                                            ),
                                                    ),
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
                                                            self.tr("syncGroup.disable")
                                                        } else {
                                                            self.tr("syncGroup.enable")
                                                        },
                                                        cx.listener(|this, _, _, cx| {
                                                            this.toggle_selected_sync_group_enabled(cx);
                                                        }),
                                                    ))
                                                    .child(small_button(palette,
                                                        "sync-group-delete",
                                                        self.tr("syncGroup.deleteGroup"),
                                                        cx.listener(|this, _, _, cx| {
                                                            this.request_delete_selected_sync_group(cx);
                                                        }),
                                                    )),
                                            ),
                                    )
                                    .child(session_rows)
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .flex_wrap()
                                            .gap_2()
                                            .pt_2()
                                            .border_t_1()
                                            .border_color(rgb(palette.border))
                                            .child(small_button(
                                                palette,
                                                "sync-group-select-all",
                                                self.tr("syncGroup.selectAll"),
                                                cx.listener(|this, _, _, cx| {
                                                    this.select_all_sync_group_sessions(cx);
                                                }),
                                            ))
                                            .child(small_button(
                                                palette,
                                                "sync-group-add-filtered",
                                                self.tr("syncGroup.addFiltered"),
                                                cx.listener(|this, _, _, cx| {
                                                    this.add_filtered_sync_group_sessions(cx);
                                                }),
                                            ))
                                            .child(small_button(
                                                palette,
                                                "sync-group-remove-filtered",
                                                self.tr("syncGroup.removeFiltered"),
                                                cx.listener(|this, _, _, cx| {
                                                    this.remove_filtered_sync_group_sessions(cx);
                                                }),
                                            ))
                                            .child(small_button(
                                                palette,
                                                "sync-group-same-host",
                                                self.tr("syncGroup.selectSameHost"),
                                                cx.listener(|this, _, _, cx| {
                                                    this.select_same_host_sync_group_sessions(cx);
                                                }),
                                            ))
                                            .child(small_button(
                                                palette,
                                                "sync-group-clear-all",
                                                self.tr("syncGroup.deselectAll"),
                                                cx.listener(|this, _, _, cx| {
                                                    this.clear_sync_group_sessions(cx);
                                                }),
                                            )),
                                    ),
                            ),
                    ),
            )
            .when_some(pending_delete_message, |this, message| {
                this.child(
                    div()
                        .id(SharedString::from("sync-group-delete-backdrop"))
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(rgba(0x00000099))
                        .on_click(|_, _, cx| cx.stop_propagation())
                        .child(
                            div()
                                .id(SharedString::from("sync-group-delete-dialog"))
                                .w(px(360.))
                                .max_w_full()
                                .mx_4()
                                .rounded_md()
                                .border_1()
                                .border_color(rgb(palette.border))
                                .bg(rgb(palette.surface_elevated))
                                .shadow_lg()
                                .p_4()
                                .on_click(|_, _, cx| cx.stop_propagation())
                                .child(
                                    div()
                                        .text_sm()
                                        .font_weight(FontWeight(800.))
                                        .text_color(rgb(palette.text))
                                        .child(self.tr("syncGroup.deleteGroup")),
                                )
                                .child(
                                    div()
                                        .mt_2()
                                        .text_xs()
                                        .text_color(rgb(palette.text_muted))
                                        .child(message),
                                )
                                .child(
                                    div()
                                        .mt_4()
                                        .flex()
                                        .justify_end()
                                        .gap_2()
                                        .child(small_button(
                                            palette,
                                            "sync-group-delete-cancel",
                                            self.tr("common.cancel"),
                                            cx.listener(|this, _, _, cx| {
                                                this.cancel_delete_sync_group(cx);
                                            }),
                                        ))
                                        .child(small_button(
                                            palette,
                                            "sync-group-delete-confirm",
                                            self.tr("syncGroup.deleteGroup"),
                                            cx.listener(|this, _, _, cx| {
                                                this.confirm_delete_sync_group(cx);
                                            }),
                                        )),
                                ),
                        ),
                )
            })
    }
}

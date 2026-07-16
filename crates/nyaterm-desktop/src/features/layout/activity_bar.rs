use super::*;

impl NyaTermApp {
    pub(in crate::features) fn activity_bar_context_menu_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let Some(menu) = self.activity_bar_context_menu.clone() else {
            return div().into_any_element();
        };
        let entry_id = menu.entry_id.clone();
        let move_up_id = entry_id.clone();
        let move_down_id = entry_id.clone();
        let entry_label = ActivityBarEntry::from_persistence_id(&menu.entry_id)
            .map(|entry| entry.label())
            .unwrap_or("Item");

        let mut zone_buttons = div().flex().flex_col().gap_1();
        for zone in ActivityBarZone::all() {
            let target = zone;
            let id = entry_id.clone();
            let selected = zone == menu.zone;
            zone_buttons = zone_buttons.child(
                div()
                    .id(SharedString::from(format!(
                        "activity-move-{}",
                        zone.persistence_key()
                    )))
                    .h(px(28.))
                    .px_2()
                    .flex()
                    .items_center()
                    .rounded_sm()
                    .cursor_pointer()
                    .text_xs()
                    .text_color(if selected {
                        rgb(palette.accent)
                    } else {
                        rgb(palette.text)
                    })
                    .bg(if selected {
                        rgb(palette.hover)
                    } else {
                        rgb(palette.bg)
                    })
                    .hover(|this| this.bg(rgb(palette.surface_elevated)))
                    .child(zone.label())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.move_activity_entry(id.clone(), target, None, cx);
                    })),
            );
        }

        div()
            .id(SharedString::from("activity-context-backdrop"))
            .absolute()
            .inset_0()
            .flex()
            .items_start()
            .justify_center()
            .pt(px(72.))
            .bg(rgba(0x0d111788))
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_activity_bar_context_menu(cx);
            }))
            .child(
                div()
                    .id(SharedString::from("activity-context-menu"))
                    .w(px(240.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface))
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .on_mouse_down(MouseButton::Left, |_, _, _| {})
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight(800.))
                            .text_color(rgb(palette.text))
                            .child(format!("Activity · {entry_label}")),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "activity-move-up",
                                "Up",
                                cx.listener(move |this, _, _, cx| {
                                    this.reorder_activity_entry(move_up_id.clone(), -1, cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "activity-move-down",
                                "Down",
                                cx.listener(move |this, _, _, cx| {
                                    this.reorder_activity_entry(move_down_id.clone(), 1, cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "activity-toggle-labels",
                                if self.activity_bar_layout.show_labels {
                                    "Hide Labels"
                                } else {
                                    "Show Labels"
                                },
                                cx.listener(|this, _, _, cx| {
                                    this.toggle_activity_bar_labels(cx);
                                }),
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_muted))
                            .child("Move to zone"),
                    )
                    .child(zone_buttons)
                    .child(small_button(
                        palette,
                        "activity-menu-close",
                        "Close",
                        cx.listener(|this, _, _, cx| {
                            this.close_activity_bar_context_menu(cx);
                        }),
                    )),
            )
            .into_any_element()
    }

    pub(in crate::features) fn activity_bar(
        &mut self,
        side: ActivitySide,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (top_zone, bottom_zone) = match side {
            ActivitySide::Left => (ActivityBarZone::LeftTop, ActivityBarZone::LeftBottom),
            ActivitySide::Right => (ActivityBarZone::RightTop, ActivityBarZone::RightBottom),
        };
        let top_entries = self.activity_entries_for_zone(top_zone);
        let bottom_entries = self.activity_entries_for_zone(bottom_zone);
        let top_len = top_entries.len();
        let bottom_len = bottom_entries.len();
        let show_labels = self.activity_bar_layout.show_labels;
        let palette = self.theme_palette();

        // Tauri DropZone: gap-0.5 pt-1
        let mut top = div().flex().flex_col().items_center().gap(px(2.)).pt_1();
        for (index, entry) in top_entries.into_iter().enumerate() {
            top = top.child(self.activity_entry_button(
                entry,
                side,
                top_zone,
                index,
                show_labels,
                cx,
            ));
        }
        // End-of-zone drop target (append).
        top = top.child(self.activity_zone_end_drop_target(top_zone, top_len, cx));

        let mut bottom = div()
            .mt_auto()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(2.))
            .pb_1();
        for (index, entry) in bottom_entries.into_iter().enumerate() {
            bottom = bottom.child(self.activity_entry_button(
                entry,
                side,
                bottom_zone,
                index,
                show_labels,
                cx,
            ));
        }
        bottom = bottom.child(self.activity_zone_end_drop_target(bottom_zone, bottom_len, cx));

        div()
            .w(if show_labels { px(52.) } else { px(40.) })
            .flex_none()
            .flex()
            .flex_col()
            .border_color(rgb(palette.border))
            .when(side == ActivitySide::Left, |this| this.border_r_1())
            .when(side == ActivitySide::Right, |this| this.border_l_1())
            .bg(rgb(palette.bg))
            .child(top)
            .child(bottom)
    }

    pub(in crate::features) fn activity_zone_end_drop_target(
        &self,
        zone: ActivityBarZone,
        end_index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let zone_key = zone.persistence_key();
        div()
            .id(SharedString::from(format!("activity-zone-end-{zone_key}")))
            .w_full()
            .h(px(8.))
            .flex_none()
            .on_drop(
                cx.listener(move |this, payload: &ActivityBarDragPayload, _, cx| {
                    if payload.entry_id.is_empty() {
                        return;
                    }
                    this.move_activity_entry(payload.entry_id.clone(), zone, Some(end_index), cx);
                }),
            )
    }

    pub(in crate::features) fn activity_entry_button(
        &self,
        entry: ActivityBarEntry,
        side: ActivitySide,
        zone: ActivityBarZone,
        index: usize,
        show_labels: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.activity_entry_selected(entry);
        let icon_path = entry.icon_path();
        let glyph = entry.glyph();
        let tooltip = entry.label();
        let short_label = entry.short_label();
        let palette = self.theme_palette();
        let active_color = match side {
            ActivitySide::Left => rgb(palette.accent),
            ActivitySide::Right => rgb(palette.success),
        };
        let icon_color = if selected {
            active_color
        } else {
            rgb(palette.text_muted)
        };
        let entry_id = entry.persistence_id().to_string();
        let context_entry_id = entry_id.clone();
        let recording_active =
            matches!(entry, ActivityBarEntry::Recording) && self.recording_active_count > 0;
        let indicator = if recording_active {
            rgb(palette.danger)
        } else if selected {
            active_color
        } else {
            rgb(palette.bg)
        };
        let bg = if recording_active {
            // Keep a subdued danger wash while recording.
            rgb(0x3d1418)
        } else if selected {
            rgb(palette.hover)
        } else {
            rgb(palette.bg)
        };

        div()
            .id(SharedString::from(format!("activity-{entry_id}")))
            .relative()
            .when(show_labels, |this| {
                this.w_full()
                    .px_1()
                    .py_1()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap_1()
            })
            .when(!show_labels, |this| {
                this.w_full()
                    .h(px(36.))
                    .flex()
                    .items_center()
                    .justify_center()
            })
            .rounded_sm()
            .cursor_pointer()
            .text_sm()
            .font_weight(FontWeight(700.))
            .text_color(if selected {
                // Tauri ActivityBarButton uses primary color when active.
                active_color
            } else {
                rgb(palette.text_muted)
            })
            .bg(bg)
            .hover(move |hover| hover.bg(rgb(palette.hover)).text_color(active_color))
            .child(
                div()
                    .absolute()
                    .top(px(8.))
                    .bottom(px(8.))
                    .w(px(2.))
                    .rounded_full()
                    .bg(indicator)
                    .when(side == ActivitySide::Left, |this| this.left_0())
                    .when(side == ActivitySide::Right, |this| this.right_0()),
            )
            .child(activity_icon(
                icon_path,
                glyph,
                icon_color.into(),
                if show_labels { 18. } else { 20. },
            ))
            .when(show_labels, |this| {
                this.child(
                    div()
                        .text_size(px(8.))
                        .font_weight(FontWeight(500.))
                        .text_color(if selected {
                            active_color
                        } else {
                            rgb(palette.text_muted)
                        })
                        .child(short_label),
                )
            })
            .cursor_move()
            .on_drag(
                ActivityBarDragPayload {
                    entry_id: entry_id.clone(),
                    zone,
                    index,
                    label: tooltip.to_string(),
                },
                |payload, position, _, cx| {
                    cx.new(|_| ActivityBarDragPreview::new(payload.clone(), position))
                },
            )
            .on_drop({
                let drop_zone = zone;
                let drop_index = index;
                cx.listener(move |this, payload: &ActivityBarDragPayload, _, cx| {
                    if payload.entry_id.is_empty() {
                        return;
                    }
                    // Drop onto this button inserts before it (Tauri dropIndex == idx).
                    this.move_activity_entry(
                        payload.entry_id.clone(),
                        drop_zone,
                        Some(drop_index),
                        cx,
                    );
                })
            })
            .tooltip({
                let title = tooltip.to_string();
                let detail = if show_labels {
                    None
                } else {
                    Some(short_label.to_string())
                };
                move |_, cx| {
                    let mut tip = ChromeTooltip::new(title.clone());
                    if let Some(detail) = detail.clone() {
                        tip = tip.with_detail(detail);
                    }
                    cx.new(|_| tip).into()
                }
            })
            .on_click(cx.listener(move |this, _, window, cx| {
                this.activate_activity_entry(entry, window, cx);
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, _, _, cx| {
                    this.open_activity_bar_context_menu(context_entry_id.clone(), zone, index, cx);
                }),
            )
    }

    pub(in crate::features) fn bottom_panel_button(
        &self,
        mode: BottomPanelMode,
        icon: &'static str,
        tooltip: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let selected = self.bottom_panel == mode;
        div()
            .id(SharedString::from(format!("bottom-panel-{tooltip}")))
            .relative()
            .size(px(36.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .cursor_pointer()
            .text_sm()
            .font_weight(FontWeight(700.))
            .text_color(if selected {
                rgb(0xffffff)
            } else {
                rgb(palette.text_muted)
            })
            .bg(if selected {
                rgb(palette.hover)
            } else {
                rgb(palette.bg)
            })
            .hover(|hover| hover.bg(rgb(palette.hover)).text_color(rgb(0xffffff)))
            .child(
                div()
                    .absolute()
                    .top(px(7.))
                    .bottom(px(7.))
                    .right_0()
                    .w(px(2.))
                    .rounded_full()
                    .bg(if selected {
                        rgb(palette.success)
                    } else {
                        rgb(palette.bg)
                    }),
            )
            .child(icon)
            .tooltip({
                let title = tooltip.to_string();
                move |_, cx| cx.new(|_| ChromeTooltip::new(title.clone())).into()
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.bottom_panel = if this.bottom_panel == mode {
                    BottomPanelMode::Hidden
                } else {
                    mode
                };
                cx.notify();
            }))
    }

    pub(in crate::features) fn recording_activity_button(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let recording_count = self.recording_active_count;
        let selected = self.right_focus == RightFocus::Recording || recording_count > 0;
        div()
            .id(SharedString::from("activity-recording"))
            .relative()
            .size(px(36.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .cursor_pointer()
            .text_sm()
            .font_weight(FontWeight(700.))
            .text_color(if selected {
                rgb(0xffffff)
            } else {
                rgb(palette.text_muted)
            })
            .bg(if selected {
                if recording_count > 0 {
                    rgb(0x3d1418)
                } else {
                    rgb(palette.hover)
                }
            } else {
                rgb(palette.bg)
            })
            .hover(|hover| hover.bg(rgb(palette.hover)).text_color(rgb(0xffffff)))
            .child(
                div()
                    .absolute()
                    .top(px(7.))
                    .bottom(px(7.))
                    .right_0()
                    .w(px(2.))
                    .rounded_full()
                    .bg(if recording_count > 0 {
                        rgb(palette.danger)
                    } else if selected {
                        rgb(palette.success)
                    } else {
                        rgb(palette.bg)
                    }),
            )
            .child(activity_icon(
                Some("icons/record.svg"),
                "●",
                if recording_count > 0 {
                    rgb(palette.danger).into()
                } else if selected {
                    rgb(0xffffff).into()
                } else {
                    rgb(palette.text_muted).into()
                },
                18.,
            ))
            .on_click(cx.listener(|this, _, _, cx| {
                this.open_panel(NavItem::Recording, cx);
            }))
    }

    pub(in crate::features) fn lock_activity_button(
        &self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        div()
            .id(SharedString::from("activity-lock"))
            .relative()
            .size(px(36.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .cursor_pointer()
            .text_sm()
            .font_weight(FontWeight(700.))
            .text_color(if self.is_locked {
                rgb(0xffffff)
            } else {
                rgb(palette.text_muted)
            })
            .bg(if self.is_locked {
                rgb(palette.hover)
            } else {
                rgb(palette.bg)
            })
            .hover(|hover| hover.bg(rgb(palette.hover)).text_color(rgb(0xffffff)))
            .child(
                div()
                    .absolute()
                    .top(px(7.))
                    .bottom(px(7.))
                    .right_0()
                    .w(px(2.))
                    .rounded_full()
                    .bg(if self.is_locked {
                        rgb(palette.success)
                    } else {
                        rgb(palette.bg)
                    }),
            )
            .child(activity_icon(
                Some("icons/lock.svg"),
                "L",
                if self.is_locked {
                    rgb(0xffffff).into()
                } else {
                    rgb(palette.text_muted).into()
                },
                18.,
            ))
            .on_click(cx.listener(|this, _, window, cx| {
                this.lock_app(window, cx);
            }))
    }
}

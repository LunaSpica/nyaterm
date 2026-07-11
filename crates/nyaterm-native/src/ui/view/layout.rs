use super::*;

#[path = "layout/prompts.rs"]
mod prompts;
#[path = "layout/sidebar.rs"]
mod sidebar;
#[path = "layout/workspace.rs"]
mod workspace;

impl NyaTermApp {
    pub(in crate::ui::view) fn status_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let tab_count = self.ordered_tab_sessions().len();
        let pane_count = self.ordered_sessions().len();
        let session_status = if let Some(pending) = self.pending_session_name.as_ref() {
            format!("connecting {pending}")
        } else if let Some(session_id) = self.active_session_id.as_deref() {
            let tab_root = self.tab_root_for_session(session_id);
            let leaf_name = self
                .session_display_name(session_id)
                .unwrap_or_else(|| short_id(session_id).to_string());
            let mut status = if tab_root != session_id {
                let tab_name = self
                    .session_display_name(&tab_root)
                    .unwrap_or_else(|| short_id(&tab_root).to_string());
                if tab_name == leaf_name {
                    leaf_name
                } else {
                    format!("{tab_name} › {leaf_name}")
                }
            } else {
                leaf_name
            };
            if let Some(endpoint) = self.session_endpoint(session_id) {
                status = format!("{status} · {endpoint}");
            }
            if self.is_session_disconnected(session_id) {
                status = format!("{status} · disconnected");
            } else if self
                .session_pane_roots
                .get(&tab_root)
                .is_some_and(|root| root.is_split())
            {
                let count = self
                    .session_pane_roots
                    .get(&tab_root)
                    .map(|root| root.session_ids().len())
                    .unwrap_or(1);
                status = format!("{status} · {count}p");
            }
            status
        } else if self.last_connect_failure_name.is_some() {
            "failed".to_string()
        } else {
            "idle".to_string()
        };
        let recording_count = self.recording_manager.list_recording_sessions().len();
        let recording_status = if recording_count == 0 {
            "off".to_string()
        } else {
            format!("{recording_count} active")
        };
        let running_transfers = self
            .transfer_jobs
            .iter()
            .filter(|job| {
                matches!(
                    job.status,
                    TransferJobStatus::Running | TransferJobStatus::Cancelling
                )
            })
            .count();
        let failed_transfers = self
            .transfer_jobs
            .iter()
            .filter(|job| job.status == TransferJobStatus::Failed)
            .count();
        let transfer_status = if running_transfers > 0 {
            format!("{running_transfers} running")
        } else if failed_transfers > 0 {
            format!("{failed_transfers} failed")
        } else if self.transfer_jobs.is_empty() {
            "idle".to_string()
        } else {
            format!("{} job(s)", self.transfer_jobs.len())
        };
        let bottom_panel = match self.bottom_panel {
            BottomPanelMode::QuickCommands => "quick commands",
            BottomPanelMode::CommandSend => "command send",
            BottomPanelMode::Hidden => "hidden",
        };
        let ai_status = if self.ai_agent_loop.is_some() {
            "agent".to_string()
        } else if self.ai_chat_pending {
            "chat".to_string()
        } else if self
            .ai_prepared_request
            .as_ref()
            .is_some_and(|request| request.action == AiAction::CustomFileAction) {
            "file".to_string()
        } else if self.ai_settings.enabled {
            match self.ai_settings.default_mode {
                AiMode::Ask => "ask",
                AiMode::Agent => "agent",
            }
            .to_string()
        } else {
            "off".to_string()
        };
        let (cpu_status, mem_status) = self
            .remote_stats
            .as_ref()
            .map(|stats| {
                let memory_total = stats.memory.used.saturating_add(stats.memory.available);
                let memory_percent = if memory_total > 0 {
                    stats.memory.used as f64 / memory_total as f64 * 100.
                } else {
                    0.
                };
                (
                    format!("{:.0}%", stats.cpu.usage),
                    format!("{memory_percent:.0}%"),
                )
            })
            .unwrap_or_else(|| ("n/a".to_string(), "n/a".to_string()));
        let lock_status = if self.is_locked {
            "locked".to_string()
        } else if self.settings.enable_screen_lock && self.settings.idle_lock_minutes > 0 {
            format!("auto {}m", self.settings.idle_lock_minutes)
        } else {
            "manual".to_string()
        };

        let palette = self.theme_palette();
        div()
            .h(px(22.))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .gap_1()
            .px_2()
            .border_t_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.bg))
            .text_size(px(10.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .min_w_0()
                    .child(status_bar_button(palette, 
                        "status-session",
                        "Session",
                        session_status,
                        if self.pending_session_name.is_some() {
                            rgb(palette.warning)
                        } else if self
                            .active_session_id
                            .as_deref()
                            .is_some_and(|id| self.is_session_disconnected(id))
                        {
                            rgb(palette.danger)
                        } else if self.active_session_id.is_some() {
                            rgb(palette.success)
                        } else if self.last_connect_failure_name.is_some() {
                            rgb(palette.danger)
                        } else {
                            rgb(palette.text_muted)
                        },
                        cx.listener(|this, _, _, cx| {
                            this.main_mode = MainMode::Workspace;
                            this.terminal_status = "workspace focused".to_string();
                            cx.notify();
                        }),
                    ))
                    .child(status_bar_label(palette, 
                        "Tabs",
                        if pane_count > tab_count {
                            format!("{tab_count} ({pane_count}p)")
                        } else {
                            tab_count.to_string()
                        },
                        rgb(palette.accent),
                    ))
                    .child(status_bar_button(palette, 
                        "status-recording",
                        "Recording",
                        recording_status,
                        if recording_count > 0 {
                            rgb(palette.danger)
                        } else {
                            rgb(palette.text_muted)
                        },
                        cx.listener(|this, _, _, cx| {
                            this.ensure_panel_open(NavItem::Recording);
                            cx.notify();
                        }),
                    ))
                    .child(status_bar_button(palette, 
                        "status-transfer",
                        "Transfer",
                        transfer_status,
                        if running_transfers > 0 {
                            rgb(palette.warning)
                        } else if failed_transfers > 0 {
                            rgb(palette.danger)
                        } else {
                            rgb(palette.text_muted)
                        },
                        cx.listener(|this, _, _, cx| {
                            this.ensure_panel_open(NavItem::Transfers);
                            cx.notify();
                        }),
                    ))
                    .child(status_bar_label(palette, "Panel", bottom_panel, rgb(palette.accent)))
                    .child(status_bar_button(palette, 
                        "status-broadcast",
                        "Broadcast",
                        if self.broadcast_to_all {
                            "on".to_string()
                        } else {
                            "off".to_string()
                        },
                        if self.broadcast_to_all {
                            rgb(palette.warning)
                        } else {
                            rgb(palette.text_muted)
                        },
                        cx.listener(|this, _, _, cx| {
                            this.toggle_broadcast_to_all(cx);
                        }),
                    )),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_1()
                    .min_w_0()
                    .child(status_bar_button(palette, 
                        "status-ai",
                        "AI",
                        ai_status,
                        if self.ai_settings.enabled {
                            rgb(palette.accent)
                        } else {
                            rgb(palette.text_muted)
                        },
                        cx.listener(|this, _, _, cx| {
                            this.ensure_panel_open(NavItem::AiAssistant);
                            cx.notify();
                        }),
                    ))
                    .child(status_bar_button(palette, 
                        "status-cpu",
                        "CPU",
                        cpu_status,
                        rgb(palette.accent),
                        cx.listener(|this, _, _, cx| {
                            this.ensure_panel_open(NavItem::Stats);
                            cx.notify();
                        }),
                    ))
                    .child(status_bar_button(palette, 
                        "status-memory",
                        "MEM",
                        mem_status,
                        rgb(palette.accent),
                        cx.listener(|this, _, _, cx| {
                            this.ensure_panel_open(NavItem::Stats);
                            cx.notify();
                        }),
                    ))
                    .child(status_bar_label(palette, 
                        "Store",
                        if self.store_status.ready {
                            "online"
                        } else {
                            "offline"
                        },
                        if self.store_status.ready {
                            rgb(palette.success)
                        } else {
                            rgb(palette.danger)
                        },
                    ))
                    .child(status_bar_button(palette, 
                        "status-lock",
                        "Lock",
                        lock_status,
                        if self.is_locked {
                            rgb(palette.danger)
                        } else {
                            rgb(palette.text_muted)
                        },
                        cx.listener(|this, _, window, cx| {
                            this.lock_app(window, cx);
                        }),
                    )),
            )
    }

    pub(in crate::ui::view) fn activity_bar_context_menu_overlay(
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
                            .child(small_button(palette, 
                                "activity-move-up",
                                "Up",
                                cx.listener(move |this, _, _, cx| {
                                    this.reorder_activity_entry(move_up_id.clone(), -1, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "activity-move-down",
                                "Down",
                                cx.listener(move |this, _, _, cx| {
                                    this.reorder_activity_entry(move_down_id.clone(), 1, cx);
                                }),
                            ))
                            .child(small_button(palette, 
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
                    .child(small_button(palette, 
                        "activity-menu-close",
                        "Close",
                        cx.listener(|this, _, _, cx| {
                            this.close_activity_bar_context_menu(cx);
                        }),
                    )),
            )
            .into_any_element()
    }

    pub(in crate::ui::view) fn title_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {

        let palette = self.theme_palette();
        // Compact chrome closer to Tauri title/menu strip density.
        div()
            .h(px(36.))
            .flex()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface))
            .child(
                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .window_control_area(WindowControlArea::Drag)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .mr_2()
                            .child(logo_mark(palette))
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(palette.text))
                                    .child("NyaTerm"),
                            ),
                    )
                    .child(self.title_menu_trigger(TitleMenu::File, cx))
                    .child(self.title_menu_trigger(TitleMenu::View, cx))
                    .child(self.title_menu_trigger(TitleMenu::Terminal, cx))
                    .child(self.title_menu_trigger(TitleMenu::Help, cx)),
            )
            .child(
                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .flex_1()
                    .window_control_area(WindowControlArea::Drag)
                    .child(
                        div()
                            .max_w(px(520.))
                            .overflow_hidden()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child(self.title_context_label()),
                    ),
            )
            .child(
                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w(px(10.))
                            .h_full()
                            .window_control_area(WindowControlArea::Drag),
                    )
                    .child(window_control_button(
                        palette,
                        "window-min",
                        "–",
                        WindowControlArea::Min,
                        |_, window, _| window.minimize_window(),
                    ))
                    .child(window_control_button(
                        palette,
                        "window-max",
                        "□",
                        WindowControlArea::Max,
                        |_, window, _| window.zoom_window(),
                    ))
                    .child(window_control_button(
                        palette,
                        "window-close",
                        "×",
                        WindowControlArea::Close,
                        |_, window, _| window.remove_window(),
                    )),
            )
    }

    fn title_context_label(&self) -> String {
        if let Some(session_id) = self.active_session_id.as_deref() {
            let tab_root = self.tab_root_for_session(session_id);
            let leaf_name = self
                .session_display_name(session_id)
                .unwrap_or_else(|| short_id(session_id).to_string());
            let name = if tab_root != session_id {
                let tab_name = self
                    .session_display_name(&tab_root)
                    .unwrap_or_else(|| short_id(&tab_root).to_string());
                if tab_name == leaf_name {
                    leaf_name
                } else {
                    format!("{tab_name} › {leaf_name}")
                }
            } else {
                leaf_name
            };
            let mut parts = vec![name];
            if let Some(endpoint) = self.session_endpoint(session_id) {
                parts.push(endpoint);
            }
            if self.is_session_disconnected(session_id) {
                parts.push("disconnected".to_string());
            } else if self
                .session_pane_roots
                .get(&tab_root)
                .is_some_and(|root| root.is_split())
            {
                let count = self
                    .session_pane_roots
                    .get(&tab_root)
                    .map(|root| root.session_ids().len())
                    .unwrap_or(1);
                parts.push(format!("{count} panes"));
            }
            return parts.join(" · ");
        }
        if let Some(pending) = self.pending_session_name.as_ref() {
            return format!("Connecting {pending}");
        }
        if let (Some(failed), Some(error)) = (
            self.last_connect_failure_name.as_ref(),
            self.last_connect_failure_error.as_ref(),
        ) {
            return format!("Failed {failed} · {}", truncate_preview(error, 40));
        }
        "NyaTerm".to_string()
    }

    fn left_panel_meta(&self) -> &'static str {
        match self.current_left_panel().unwrap_or(NavItem::Transfers) {
            NavItem::Transfers => "file explorer",
            NavItem::Tunnels => "network",
            NavItem::SecurityAuth => "security / auth",
            NavItem::SyncBackupHistory => "sync / backup",
            NavItem::Migration => "migration",
            other => other.label(),
        }
    }

    pub(in crate::ui::view) fn activity_bar(
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
            top = top.child(self.activity_entry_button(entry, side, top_zone, index, show_labels, cx));
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
            bottom =
                bottom.child(self.activity_entry_button(entry, side, bottom_zone, index, show_labels, cx));
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

    fn activity_zone_end_drop_target(
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
            .on_drop(cx.listener(move |this, payload: &ActivityBarDragPayload, _, cx| {
                if payload.entry_id.is_empty() {
                    return;
                }
                this.move_activity_entry(
                    payload.entry_id.clone(),
                    zone,
                    Some(end_index),
                    cx,
                );
            }))
    }

    fn activity_entry_button(
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
        let recording_active = matches!(entry, ActivityBarEntry::Recording)
            && !self.recording_manager.list_recording_sessions().is_empty();
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
            .hover(move |hover| {
                hover
                    .bg(rgb(palette.hover))
                    .text_color(active_color)
            })
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
            .child(activity_icon(icon_path, glyph, icon_color.into(), if show_labels { 18. } else { 20. }))
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
                    this.open_activity_bar_context_menu(
                        context_entry_id.clone(),
                        zone,
                        index,
                        cx,
                    );
                }),
            )
    }

    fn bottom_panel_button(
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

    fn recording_activity_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let recording_count = self.recording_manager.list_recording_sessions().len();
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

    fn lock_activity_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
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

    fn title_menu_trigger(
        &self,
        menu: TitleMenu,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let open = self.title_menu_open == Some(menu);
        let label = menu.label();
        let palette = self.theme_palette();
        div()
            .relative()
            .child(
                div()
                    .id(SharedString::from(format!("title-menu-trigger-{label}")))
                    .h(px(28.))
                    .px_2()
                    .flex()
                    .items_center()
                    .rounded_sm()
                    .text_xs()
                    .text_color(if open {
                        rgb(palette.text)
                    } else {
                        rgb(palette.text_muted)
                    })
                    .bg(if open {
                        rgb(palette.hover)
                    } else {
                        rgb(palette.surface)
                    })
                    .cursor_pointer()
                    .hover(move |this| this.bg(rgb(palette.hover)).text_color(rgb(palette.accent)))
                    .child(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.toggle_title_menu(menu, cx);
                    })),
            )
            .when(open, |this| {
                this.child(self.title_menu_dropdown(menu, cx))
            })
    }

    fn title_menu_dropdown(
        &self,
        menu: TitleMenu,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let shortcut = |id: &str, fallback: &str| self.display_shortcut_for(id, fallback);
        let palette = self.theme_palette();
        let mut items = div()
            .id(SharedString::from(format!("title-menu-{}", menu.label())))
            .absolute()
            .top(px(30.))
            .left_0()
                        .w(px(260.))
            .max_h(px(480.))
            .overflow_y_scroll()
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface))
            .shadow_lg()
            .py_1()
            .flex()
            .flex_col();

        match menu {
            TitleMenu::File => {
                items = items
                    .child(title_menu_item(
                        palette,
                        "title-file-new-session",
                        "New Session",
                        Some(shortcut("tab.newSession", "Ctrl+Shift+N")),
                        cx.listener(|this, _, window, cx| {
                            this.close_title_menu(cx);
                            this.start_local_session(window, cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-file-temp-ssh",
                        "Temporary SSH Link",
                        Some(shortcut("tab.temporarySshLink", "Ctrl+Alt+N")),
                        cx.listener(|this, _, window, cx| {
                            this.close_title_menu(cx);
                            this.open_temporary_ssh_link_dialog(window, cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-file-import",
                        "Import Config",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.prompt_config_import(cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-file-export",
                        "Export Config",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.prompt_config_export(cx);
                        }),
                    ));
            }
            TitleMenu::View => {
                items = items
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .text_size(px(10.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text_dimmed))
                            .child("Theme"),
                    )
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-github-dark",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "github-dark"
                                || (current == "catppuccin" && "github-dark" == "catppuccin-mocha");
                            let label = crate::ui::theme::appearance_theme_label("github-dark");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("github-dark", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-dracula",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "dracula"
                                || (current == "catppuccin" && "dracula" == "catppuccin-mocha");
                            let label = crate::ui::theme::appearance_theme_label("dracula");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("dracula", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-nord",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "nord"
                                || (current == "catppuccin" && "nord" == "catppuccin-mocha");
                            let label = crate::ui::theme::appearance_theme_label("nord");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("nord", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-monokai-pro",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "monokai-pro"
                                || (current == "catppuccin" && "monokai-pro" == "catppuccin-mocha");
                            let label = crate::ui::theme::appearance_theme_label("monokai-pro");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("monokai-pro", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-solarized-light",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "solarized-light"
                                || (current == "catppuccin" && "solarized-light" == "catppuccin-mocha");
                            let label = crate::ui::theme::appearance_theme_label("solarized-light");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("solarized-light", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-catppuccin-mocha",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "catppuccin-mocha"
                                || (current == "catppuccin" && "catppuccin-mocha" == "catppuccin-mocha");
                            let label = crate::ui::theme::appearance_theme_label("catppuccin-mocha");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("catppuccin-mocha", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-tokyo-night",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "tokyo-night"
                                || (current == "catppuccin" && "tokyo-night" == "catppuccin-mocha");
                            let label = crate::ui::theme::appearance_theme_label("tokyo-night");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("tokyo-night", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-one-dark-pro",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "one-dark-pro"
                                || (current == "catppuccin" && "one-dark-pro" == "catppuccin-mocha");
                            let label = crate::ui::theme::appearance_theme_label("one-dark-pro");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("one-dark-pro", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-rose-pine",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "rose-pine"
                                || (current == "catppuccin" && "rose-pine" == "catppuccin-mocha");
                            let label = crate::ui::theme::appearance_theme_label("rose-pine");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("rose-pine", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-gruvbox-dark",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "gruvbox-dark"
                                || (current == "catppuccin" && "gruvbox-dark" == "catppuccin-mocha");
                            let label = crate::ui::theme::appearance_theme_label("gruvbox-dark");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("gruvbox-dark", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-github-light",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "github-light"
                                || (current == "catppuccin" && "github-light" == "catppuccin-mocha");
                            let label = crate::ui::theme::appearance_theme_label("github-light");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("github-light", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-catppuccin-latte",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "catppuccin-latte"
                                || (current == "catppuccin" && "catppuccin-latte" == "catppuccin-mocha");
                            let label = crate::ui::theme::appearance_theme_label("catppuccin-latte");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("catppuccin-latte", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-one-light",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "one-light"
                                || (current == "catppuccin" && "one-light" == "catppuccin-mocha");
                            let label = crate::ui::theme::appearance_theme_label("one-light");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("one-light", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-theme-nya-high-contrast",
                        {
                            let current = self.settings.theme.as_str();
                            let selected = current == "nya-high-contrast"
                                || (current == "catppuccin" && "nya-high-contrast" == "catppuccin-mocha");
                            let label = crate::ui::theme::appearance_theme_label("nya-high-contrast");
                            if selected {
                                format!("✓ {label}")
                            } else {
                                label.to_string()
                            }
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_appearance_theme("nya-high-contrast", cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(
                        div()
                            .px_3()
                            .py_1()
                            .text_size(px(10.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text_dimmed))
                            .child("Language"),
                    )
                    .child(title_menu_item(
                        palette,
                        "title-view-lang-en",
                        if matches!(self.settings.language.as_str(), "en" | "en-US") {
                            "✓ English"
                        } else {
                            "English"
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_ui_language("en", cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-lang-zh",
                        if matches!(self.settings.language.as_str(), "zh-CN" | "zh") {
                            "✓ 中文"
                        } else {
                            "中文"
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.update_ui_language("zh-CN", cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-view-zoom-in",
                        "Zoom In",
                        Some(shortcut("view.zoomIn", "Ctrl+=")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.zoom_terminal_in(cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-zoom-out",
                        "Zoom Out",
                        Some(shortcut("view.zoomOut", "Ctrl+-")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.zoom_terminal_out(cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-reset-zoom",
                        "Reset Zoom",
                        Some(shortcut("view.resetZoom", "Ctrl+0")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.reset_terminal_font_size(cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-view-toggle-left",
                        "Toggle Left Sidebar",
                        Some(shortcut("view.toggleLeftSidebar", "Ctrl+Shift+E")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.toggle_left_sidebar(cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-toggle-right",
                        "Toggle Right Sidebar",
                        Some(shortcut("view.toggleRightSidebar", "Ctrl+Shift+B")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.toggle_right_inspector(cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-view-smart-split",
                        "Smart Split",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.apply_smart_split(SmartSplitMode::Auto, cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-tile-h",
                        "Tile Horizontally",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.apply_smart_split(SmartSplitMode::Horizontal, cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-tile-v",
                        "Tile Vertically",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.apply_smart_split(SmartSplitMode::Vertical, cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-view-merge-windows",
                        "Merge Windows",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.close_terminal_window_layout(cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-view-settings",
                        "Settings",
                        Some(shortcut("view.openSettings", "Ctrl+,")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.open_page(NavItem::Settings, cx);
                        }),
                    ));
            }
            TitleMenu::Terminal => {
                items = items
                    .child(title_menu_item(
                        palette,
                        "title-term-copy",
                        "Copy",
                        Some(shortcut("terminal.copy", "Ctrl+Shift+C")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.copy_terminal_selection_or_visible(cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-term-paste",
                        "Paste",
                        Some(shortcut("terminal.paste", "Ctrl+Shift+V")),
                        cx.listener(|this, _, window, cx| {
                            this.close_title_menu(cx);
                            this.paste_from_clipboard(window, cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-term-select-all",
                        "Select All",
                        Some(shortcut("terminal.selectAll", "Ctrl+Shift+A")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.select_all_terminal_visible(cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-term-find",
                        "Find",
                        Some(shortcut("terminal.find", "Ctrl+Shift+F")),
                        cx.listener(|this, _, window, cx| {
                            this.close_title_menu(cx);
                            this.open_terminal_search(window, cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-term-quick-switch",
                        "Command Palette",
                        Some(shortcut("tab.quickSwitch", "Ctrl+Shift+S")),
                        cx.listener(|this, _, window, cx| {
                            this.close_title_menu(cx);
                            this.open_quick_switch(window, cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-term-split-h",
                        "Split Horizontal",
                        None,
                        cx.listener(|this, _, window, cx| {
                            this.close_title_menu(cx);
                            this.split_workspace_with_duplicate(
                                WorkspaceSplitDirection::Horizontal,
                                window,
                                cx,
                            );
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-term-split-v",
                        "Split Vertical",
                        None,
                        cx.listener(|this, _, window, cx| {
                            this.close_title_menu(cx);
                            this.split_workspace_with_duplicate(
                                WorkspaceSplitDirection::Vertical,
                                window,
                                cx,
                            );
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-term-unsplit",
                        "Unsplit",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.unsplit_workspace(cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-term-sync-groups",
                        "Manage Sync Groups",
                        Some(shortcut("terminal.manageSyncGroups", "Ctrl+Shift+G")),
                        cx.listener(|this, _, window, cx| {
                            this.close_title_menu(cx);
                            this.open_sync_groups(window, cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-term-broadcast",
                        if self.broadcast_to_all {
                            "Broadcast to All ✓"
                        } else {
                            "Broadcast to All"
                        },
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.toggle_broadcast_to_all(cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-term-smart-split",
                        "Smart Split",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.apply_smart_split(SmartSplitMode::Auto, cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-term-tile-h",
                        "Tile Horizontally",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.apply_smart_split(SmartSplitMode::Horizontal, cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-term-tile-v",
                        "Tile Vertically",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.apply_smart_split(SmartSplitMode::Vertical, cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-term-clear",
                        "Clear Terminal",
                        Some(shortcut("terminal.clear", "Ctrl+L")),
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.clear_terminal(cx);
                        }),
                    ));
            }
            TitleMenu::Help => {
                let update_label = if self.update_pending {
                    "Checking Updates…"
                } else if self.update_info.as_ref().is_some_and(|info| info.available) {
                    "Update Available"
                } else {
                    "Check for Updates"
                };
                items = items
                    .child(title_menu_item(
                        palette,
                        "title-help-docs",
                        "Documentation",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.terminal_status =
                                "docs: https://github.com/nyaterm/nyaterm".to_string();
                            cx.notify();
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-help-update",
                        update_label,
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.start_update_check(cx);
                        }),
                    ))
                    .child(title_menu_item(
                        palette,
                        "title-help-migration",
                        "Migration Status",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.open_page(NavItem::Migration, cx);
                        }),
                    ))
                    .child(title_menu_separator(palette))
                    .child(title_menu_item(
                        palette,
                        "title-help-about",
                        "About NyaTerm",
                        None,
                        cx.listener(|this, _, _, cx| {
                            this.close_title_menu(cx);
                            this.terminal_status = format!(
                                "NyaTerm native {}",
                                env!("CARGO_PKG_VERSION")
                            );
                            cx.notify();
                        }),
                    ));
            }
        }

        items
    }

    pub(in crate::ui::view) fn toggle_title_menu(
        &mut self,
        menu: TitleMenu,
        cx: &mut Context<Self>,
    ) {
        self.title_menu_open = if self.title_menu_open == Some(menu) {
            None
        } else {
            Some(menu)
        };
        if self.title_menu_open.is_some() {
            self.open_tabs_menu_open = false;
            self.new_session_menu_open = false;
        }
        cx.notify();
    }

    pub(in crate::ui::view) fn close_title_menu(&mut self, cx: &mut Context<Self>) {
        if self.title_menu_open.take().is_some() {
            cx.notify();
        }
    }
}

fn title_menu_item(palette: crate::ui::theme::ThemePalette,
    id: impl Into<String>,
    label: impl Into<String>,
    shortcut: Option<String>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,) -> impl IntoElement {    let label = label.into();
    let mut row = div()
        .id(SharedString::from(id.into()))
        .h(px(30.))
        .px_3()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .text_size(px(12.))
        .text_color(rgb(palette.text))
        .cursor_pointer()
        .hover(|this| this.bg(rgb(palette.surface_elevated)))
        .on_click(on_click)
        .child(div().min_w_0().flex_1().child(label));
    if let Some(shortcut) = shortcut {
        row = row.child(
            div()
                .text_size(px(10.))
                .text_color(rgb(palette.text_dimmed))
                .child(shortcut),
        );
    }
    row
}

fn title_menu_separator(palette: crate::ui::theme::ThemePalette) -> impl IntoElement {    div()
        .h(px(1.))
        .mx_2()
        .my_1()
        .bg(rgb(palette.border))
}

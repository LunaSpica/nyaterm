use super::*;

#[path = "layout/prompts.rs"]
mod prompts;
#[path = "layout/sidebar.rs"]
mod sidebar;
#[path = "layout/workspace.rs"]
mod workspace;

impl NyaTermApp {
    pub(in crate::ui::view) fn status_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let sessions = self.session_manager.list_sessions().unwrap_or_default();
        let active_session = self
            .pending_session_name
            .clone()
            .or_else(|| self.active_session_name())
            .or_else(|| {
                self.active_session_id
                    .as_ref()
                    .map(|session_id| format!("Session {}", short_id(session_id)))
            })
            .unwrap_or_else(|| "no session".to_string());
        let session_status = if self.pending_session_name.is_some() {
            "connecting".to_string()
        } else if self.active_session_id.is_some() {
            active_session
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
            .is_some_and(|request| request.action == AiAction::CustomFileAction)
        {
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

        div()
            .h(px(24.))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .px_2()
            .border_t_1()
            .border_color(rgb(0x242a35))
            .bg(rgb(0x0f141c))
            .text_xs()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .min_w_0()
                    .child(status_bar_button(
                        "status-session",
                        "Session",
                        session_status,
                        if self.active_session_id.is_some() {
                            rgb(0x6ee7b7)
                        } else {
                            rgb(0x98a3b8)
                        },
                        cx.listener(|this, _, _, cx| {
                            this.select(NavItem::Workspace, cx);
                        }),
                    ))
                    .child(status_bar_label(
                        "Tabs",
                        sessions.len().to_string(),
                        rgb(0x93c5fd),
                    ))
                    .child(status_bar_button(
                        "status-recording",
                        "Recording",
                        recording_status,
                        if recording_count > 0 {
                            rgb(0xfca5a5)
                        } else {
                            rgb(0x98a3b8)
                        },
                        cx.listener(|this, _, _, cx| {
                            this.selected_nav = NavItem::Workspace;
                            this.main_mode = MainMode::Workspace;
                            this.right_focus = RightFocus::Recording;
                            cx.notify();
                        }),
                    ))
                    .child(status_bar_button(
                        "status-transfer",
                        "Transfer",
                        transfer_status,
                        if running_transfers > 0 {
                            rgb(0xfacc15)
                        } else if failed_transfers > 0 {
                            rgb(0xfca5a5)
                        } else {
                            rgb(0x98a3b8)
                        },
                        cx.listener(|this, _, _, cx| {
                            this.select(NavItem::Transfers, cx);
                        }),
                    ))
                    .child(status_bar_label("Panel", bottom_panel, rgb(0xc4b5fd))),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap_1()
                    .min_w_0()
                    .child(status_bar_button(
                        "status-ai",
                        "AI",
                        ai_status,
                        if self.ai_settings.enabled {
                            rgb(0x93c5fd)
                        } else {
                            rgb(0x98a3b8)
                        },
                        cx.listener(|this, _, _, cx| {
                            this.selected_nav = NavItem::Workspace;
                            this.main_mode = MainMode::Workspace;
                            this.right_focus = RightFocus::Default;
                            cx.notify();
                        }),
                    ))
                    .child(status_bar_button(
                        "status-cpu",
                        "CPU",
                        cpu_status,
                        rgb(0x93c5fd),
                        cx.listener(|this, _, _, cx| {
                            this.select(NavItem::Stats, cx);
                        }),
                    ))
                    .child(status_bar_button(
                        "status-memory",
                        "MEM",
                        mem_status,
                        rgb(0x93c5fd),
                        cx.listener(|this, _, _, cx| {
                            this.select(NavItem::Stats, cx);
                        }),
                    ))
                    .child(status_bar_label(
                        "Store",
                        if self.store_status.ready {
                            "online"
                        } else {
                            "offline"
                        },
                        if self.store_status.ready {
                            rgb(0x6ee7b7)
                        } else {
                            rgb(0xfca5a5)
                        },
                    ))
                    .child(status_bar_button(
                        "status-lock",
                        "Lock",
                        lock_status,
                        if self.is_locked {
                            rgb(0xfca5a5)
                        } else {
                            rgb(0x98a3b8)
                        },
                        cx.listener(|this, _, window, cx| {
                            this.lock_app(window, cx);
                        }),
                    )),
            )
    }

    pub(in crate::ui::view) fn title_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .h(px(40.))
            .flex()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(rgb(0x202630))
            .bg(rgb(0x11151c))
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
                            .child(logo_mark())
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(800.))
                                    .child("NyaTerm"),
                            ),
                    )
                    .child(menu_bar_button(
                        "File",
                        cx.listener(|this, _, _, cx| this.select(NavItem::Connections, cx)),
                    ))
                    .child(menu_bar_button(
                        "View",
                        cx.listener(|this, _, _, cx| this.select(NavItem::Workspace, cx)),
                    ))
                    .child(menu_bar_button(
                        "Terminal",
                        cx.listener(|this, _, window, cx| {
                            this.start_local_session(window, cx);
                        }),
                    ))
                    .child(menu_bar_button(
                        "Help",
                        cx.listener(|this, _, _, cx| this.open_page(NavItem::Migration, cx)),
                    )),
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
                            .text_color(rgb(0x8f98aa))
                            .child(self.title_context_label()),
                    ),
            )
            .child(
                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .child(status_pill("native", rgb(0x6ee7b7), rgb(0x14352b)))
                    .child(
                        div()
                            .w(px(10.))
                            .h_full()
                            .window_control_area(WindowControlArea::Drag),
                    )
                    .child(window_control_button(
                        "window-min",
                        "-",
                        WindowControlArea::Min,
                        |_, window, _| window.minimize_window(),
                    ))
                    .child(window_control_button(
                        "window-max",
                        "[]",
                        WindowControlArea::Max,
                        |_, window, _| window.zoom_window(),
                    ))
                    .child(window_control_button(
                        "window-close",
                        "x",
                        WindowControlArea::Close,
                        |_, window, _| window.remove_window(),
                    )),
            )
    }

    fn title_context_label(&self) -> String {
        let session = self
            .active_session_id
            .as_deref()
            .map(|id| format!("session {}", short_id(id)))
            .unwrap_or_else(|| "no active session".to_string());
        format!(
            "{} / {} / {}",
            self.selected_nav.label(),
            session,
            status_label(&self.terminal_status)
        )
    }

    fn left_panel_meta(&self) -> &'static str {
        match self.selected_nav {
            NavItem::Workspace => "workspace",
            NavItem::Connections => "connections",
            NavItem::Tunnels => "network",
            NavItem::Stats => "resources",
            NavItem::Processes => "processes",
            NavItem::Docker => "containers",
            NavItem::Translation => "translation",
            NavItem::Transfers => "sftp",
            NavItem::Settings => "settings",
            NavItem::Migration => "migration",
        }
    }

    pub(in crate::ui::view) fn activity_bar(
        &mut self,
        side: ActivitySide,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let top_items: &[(NavItem, &str, &str)] = match side {
            ActivitySide::Left => &[
                (NavItem::Workspace, "WS", "Workspace"),
                (NavItem::Connections, "CN", "Saved Connections"),
                (NavItem::Transfers, "FE", "File Explorer"),
                (NavItem::Tunnels, "NW", "Network"),
            ],
            ActivitySide::Right => &[
                (NavItem::Stats, "RS", "Resource Monitor"),
                (NavItem::Processes, "PS", "Process Manager"),
                (NavItem::Docker, "DK", "Docker"),
                (NavItem::Translation, "TR", "Translation"),
            ],
        };
        let bottom_items: &[(NavItem, &str, &str)] = match side {
            ActivitySide::Left => &[(NavItem::Migration, "MG", "Migration")],
            ActivitySide::Right => &[(NavItem::Settings, "ST", "Settings")],
        };

        let mut top = div().flex().flex_col().items_center().gap_1().pt_1();
        for (item, icon, tooltip) in top_items {
            top = top.child(self.activity_button(*item, icon, tooltip, side, cx));
        }

        let mut bottom = div()
            .mt_auto()
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            .pb_1();
        if side == ActivitySide::Right {
            bottom = bottom
                .child(self.bottom_panel_button(
                    BottomPanelMode::QuickCommands,
                    "QC",
                    "Quick Commands",
                    cx,
                ))
                .child(self.bottom_panel_button(
                    BottomPanelMode::CommandSend,
                    "SD",
                    "Command Send",
                    cx,
                ))
                .child(self.recording_activity_button(cx))
                .child(self.lock_activity_button(cx));
        }
        for (item, icon, tooltip) in bottom_items {
            bottom = bottom.child(self.activity_button(*item, icon, tooltip, side, cx));
        }

        div()
            .w(px(40.))
            .flex_none()
            .flex()
            .flex_col()
            .border_color(rgb(0x242a35))
            .when(side == ActivitySide::Left, |this| this.border_r_1())
            .when(side == ActivitySide::Right, |this| this.border_l_1())
            .bg(rgb(0x10141b))
            .child(top)
            .child(bottom)
    }

    fn activity_button(
        &self,
        item: NavItem,
        icon: &'static str,
        tooltip: &'static str,
        side: ActivitySide,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.selected_nav == item;
        div()
            .id(SharedString::from(format!("activity-{tooltip}")))
            .relative()
            .size(px(32.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .cursor_pointer()
            .text_xs()
            .font_weight(FontWeight(800.))
            .text_color(if selected {
                rgb(0xffffff)
            } else {
                rgb(0x98a3b8)
            })
            .bg(if selected {
                rgb(0x243044)
            } else {
                rgb(0x10141b)
            })
            .hover(|hover| hover.bg(rgb(0x202632)).text_color(rgb(0xffffff)))
            .child(
                div()
                    .absolute()
                    .top(px(7.))
                    .bottom(px(7.))
                    .w(px(2.))
                    .rounded_full()
                    .bg(if selected {
                        rgb(0x6ee7b7)
                    } else {
                        rgb(0x10141b)
                    })
                    .when(side == ActivitySide::Left, |this| this.left_0())
                    .when(side == ActivitySide::Right, |this| this.right_0()),
            )
            .child(icon)
            .on_click(cx.listener(move |this, _, _, cx| {
                match side {
                    ActivitySide::Left => {
                        this.left_sidebar_collapsed = false;
                    }
                    ActivitySide::Right
                        if matches!(
                            item,
                            NavItem::Stats
                                | NavItem::Processes
                                | NavItem::Docker
                                | NavItem::Translation
                        ) =>
                    {
                        this.right_inspector_collapsed = false;
                    }
                    ActivitySide::Right => {}
                }
                this.select(item, cx);
            }))
    }

    fn bottom_panel_button(
        &self,
        mode: BottomPanelMode,
        icon: &'static str,
        tooltip: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.bottom_panel == mode;
        div()
            .id(SharedString::from(format!("bottom-panel-{tooltip}")))
            .relative()
            .size(px(32.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .cursor_pointer()
            .text_xs()
            .font_weight(FontWeight(800.))
            .text_color(if selected {
                rgb(0xffffff)
            } else {
                rgb(0x98a3b8)
            })
            .bg(if selected {
                rgb(0x243044)
            } else {
                rgb(0x10141b)
            })
            .hover(|hover| hover.bg(rgb(0x202632)).text_color(rgb(0xffffff)))
            .child(
                div()
                    .absolute()
                    .top(px(7.))
                    .bottom(px(7.))
                    .right_0()
                    .w(px(2.))
                    .rounded_full()
                    .bg(if selected {
                        rgb(0x6ee7b7)
                    } else {
                        rgb(0x10141b)
                    }),
            )
            .child(icon)
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
        let recording_count = self.recording_manager.list_recording_sessions().len();
        let selected = self.right_focus == RightFocus::Recording || recording_count > 0;
        div()
            .id(SharedString::from("activity-recording"))
            .relative()
            .size(px(32.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .cursor_pointer()
            .text_xs()
            .font_weight(FontWeight(800.))
            .text_color(if selected {
                rgb(0xffffff)
            } else {
                rgb(0x98a3b8)
            })
            .bg(if selected {
                if recording_count > 0 {
                    rgb(0x3a1717)
                } else {
                    rgb(0x243044)
                }
            } else {
                rgb(0x10141b)
            })
            .hover(|hover| hover.bg(rgb(0x202632)).text_color(rgb(0xffffff)))
            .child(
                div()
                    .absolute()
                    .top(px(7.))
                    .bottom(px(7.))
                    .right_0()
                    .w(px(2.))
                    .rounded_full()
                    .bg(if recording_count > 0 {
                        rgb(0xfca5a5)
                    } else if selected {
                        rgb(0x6ee7b7)
                    } else {
                        rgb(0x10141b)
                    }),
            )
            .child("RC")
            .on_click(cx.listener(|this, _, _, cx| {
                this.selected_nav = NavItem::Workspace;
                this.main_mode = MainMode::Workspace;
                this.right_inspector_collapsed = false;
                this.right_focus = if this.right_focus == RightFocus::Recording {
                    RightFocus::Default
                } else {
                    RightFocus::Recording
                };
                this.terminal_status = if this.right_focus == RightFocus::Recording {
                    "recording panel opened".to_string()
                } else {
                    "inspector restored".to_string()
                };
                cx.notify();
            }))
    }

    fn lock_activity_button(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id(SharedString::from("activity-lock"))
            .relative()
            .size(px(32.))
            .flex()
            .items_center()
            .justify_center()
            .rounded_sm()
            .cursor_pointer()
            .text_xs()
            .font_weight(FontWeight(800.))
            .text_color(if self.is_locked {
                rgb(0xffffff)
            } else {
                rgb(0x98a3b8)
            })
            .bg(if self.is_locked {
                rgb(0x243044)
            } else {
                rgb(0x10141b)
            })
            .hover(|hover| hover.bg(rgb(0x202632)).text_color(rgb(0xffffff)))
            .child(
                div()
                    .absolute()
                    .top(px(7.))
                    .bottom(px(7.))
                    .right_0()
                    .w(px(2.))
                    .rounded_full()
                    .bg(if self.is_locked {
                        rgb(0x6ee7b7)
                    } else {
                        rgb(0x10141b)
                    }),
            )
            .child("LK")
            .on_click(cx.listener(|this, _, window, cx| {
                this.lock_app(window, cx);
            }))
    }
}

use super::*;

impl NyaTermApp {
    pub(in crate::features) fn title_bar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let compact_layout = !cfg!(target_os = "macos");
        let narrow_left = compact_layout && self.last_viewport_size.0 < 1024.;
        let narrow_right = compact_layout && self.last_viewport_size.0 < 768.;
        // Match Tauri Header: h-10.
        div()
            .h(px(40.))
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
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.mark_title_drag_activity();
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .mr_2()
                            .child(logo_mark(palette)),
                    )
                    .when(narrow_left, |this| {
                        this.child(
                            div()
                                .id("title-mobile-left")
                                .size(px(28.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_sm()
                                .text_size(px(16.))
                                .text_color(rgb(palette.text_muted))
                                .cursor_pointer()
                                .hover(|this| {
                                    this.bg(rgb(palette.hover)).text_color(rgb(palette.text))
                                })
                                .child("☰")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toggle_mobile_left_drawer(cx);
                                })),
                        )
                    })
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
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.mark_title_drag_activity();
                            cx.notify();
                        }),
                    )
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
                    .when(narrow_right, |this| {
                        this.child(
                            div()
                                .id("title-mobile-right")
                                .size(px(28.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_sm()
                                .text_size(px(15.))
                                .text_color(rgb(palette.text_muted))
                                .cursor_pointer()
                                .hover(|this| {
                                    this.bg(rgb(palette.hover)).text_color(rgb(palette.text))
                                })
                                .child("◧")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.toggle_mobile_right_drawer(cx);
                                })),
                        )
                    })
                    .child(
                        div()
                            .w(px(10.))
                            .h_full()
                            .window_control_area(WindowControlArea::Drag)
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _, cx| {
                                    this.mark_title_drag_activity();
                                    cx.notify();
                                }),
                            ),
                    )
                    .child(window_control_button(
                        palette,
                        "window-min",
                        "–",
                        WindowControlArea::Min,
                        cx.listener(|this, _, window, cx| {
                            this.handle_window_minimize(window, cx);
                        }),
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
                        cx.listener(|this, _, window, cx| {
                            this.handle_window_close_request(window, cx);
                        }),
                    )),
            )
    }

    pub(in crate::features) fn title_context_label(&self) -> String {
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
        if let Some(pending) = self.pending_session_status_label() {
            return pending;
        }
        if let (Some(failed), Some(error)) = (
            self.last_connect_failure_name.as_ref(),
            self.last_connect_failure_error.as_ref(),
        ) {
            return format!("Failed {failed} · {}", truncate_preview(error, 40));
        }
        "NyaTerm".to_string()
    }

    pub(in crate::features) fn left_panel_meta(&self) -> &'static str {
        match self.current_left_panel().unwrap_or(NavItem::Transfers) {
            NavItem::Transfers => "file explorer",
            NavItem::Tunnels => "network",
            NavItem::SecurityAuth => "security / auth",
            NavItem::SyncBackupHistory => "sync / backup",
            NavItem::Migration => "migration",
            other => other.label(),
        }
    }

    pub(in crate::features) fn title_menu_trigger(
        &self,
        menu: TitleMenu,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let open = self.title_menu_open == Some(menu);
        let id_label = menu.label();
        let label = self.tr(menu.i18n_key());
        let palette = self.theme_palette();
        div()
            .relative()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, _, cx| cx.stop_propagation()),
            )
            .child(
                div()
                    .id(SharedString::from(format!("title-menu-trigger-{id_label}")))
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
                    .hover(move |this| this.bg(rgb(palette.hover)).text_color(rgb(palette.primary)))
                    .child(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.toggle_title_menu(menu, cx);
                    })),
            )
            .when(open, |this| this.child(self.title_menu_dropdown(menu, cx)))
    }

    pub(in crate::features) fn toggle_title_menu(
        &mut self,
        menu: TitleMenu,
        cx: &mut Context<Self>,
    ) {
        self.title_menu_open = if self.title_menu_open == Some(menu) {
            None
        } else {
            Some(menu)
        };
        self.title_menu_submenu = None;
        if self.title_menu_open.is_some() {
            self.open_tabs_menu_open = false;
            self.new_session_menu_open = false;
            self.new_session_all_sessions_open = false;
            self.new_session_group_menu_path.clear();
        }
        cx.notify();
    }

    pub(in crate::features) fn close_title_menu(&mut self, cx: &mut Context<Self>) {
        self.title_menu_submenu = None;
        if self.title_menu_open.take().is_some() {
            cx.notify();
        }
    }

    pub(in crate::features) fn open_title_submenu(
        &mut self,
        submenu: TitleMenuSubmenu,
        cx: &mut Context<Self>,
    ) {
        if self.title_menu_submenu != Some(submenu) {
            self.title_menu_submenu = Some(submenu);
            cx.notify();
        }
    }
}

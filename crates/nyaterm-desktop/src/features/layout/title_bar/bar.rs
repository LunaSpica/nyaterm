use gpui::{
    Context, IntoElement, MouseButton, SharedString, Window, WindowControlArea, div, prelude::*,
    px, rgb, rgba, svg,
};
use nyaterm_transport::SessionKind;
use time::{OffsetDateTime, UtcOffset, Weekday, macros::format_description};

use crate::features::{
    ChromeTooltip, NyaTermApp, format_file_size, format_rate, format_uptime, logo_mark, short_id,
    window_control_button,
};
use crate::models::{HeaderStatusMode, TitleMenu, TitleMenuSubmenu};

use super::super::title_menu_helpers::{title_menu_item, title_menu_separator};
use super::super::view_helpers::session_kind_icon_path;

struct HeaderStatusContent {
    icon_path: &'static str,
    label: String,
}

impl NyaTermApp {
    pub(in crate::features) fn title_bar(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let macos = cfg!(target_os = "macos");
        let compact_layout = !cfg!(target_os = "macos");
        let narrow_left = compact_layout && self.last_viewport_size.0 < 1024.;
        let narrow_right = compact_layout && self.last_viewport_size.0 < 768.;
        let header_status_visible = self.settings.ui_header_status_visible;
        let header_status = self.header_status_content();
        // Match Tauri Header: h-10.
        div()
            .h(px(40.))
            .flex()
            .items_center()
            .justify_between()
            .border_b_1()
            .border_color(rgb(palette.border))
            .bg(self.shell_surface_color(palette.surface))
            .child(
                div()
                    .h_full()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .when(macos, |this| this.pl(px(70.)))
                    .window_control_area(WindowControlArea::Drag)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.mark_title_drag_activity();
                            cx.notify();
                        }),
                    )
                    .when(!macos, |this| {
                        this.child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .mr_2()
                                .child(logo_mark(palette)),
                        )
                    })
                    .when(narrow_left, |this| {
                        this.child(
                            div()
                                .id("title-mobile-left")
                                .group("title-mobile-left")
                                .size(px(28.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_sm()
                                .text_color(rgb(palette.text_muted))
                                .cursor_pointer()
                                .hover(|this| {
                                    this.bg(rgb(palette.hover)).text_color(rgb(palette.text))
                                })
                                .child(
                                    svg()
                                        .size(px(16.))
                                        .path("icons/menu/menu.svg")
                                        .text_color(rgb(palette.text_muted))
                                        .group_hover("title-mobile-left", |this| {
                                            this.text_color(rgb(palette.text))
                                        }),
                                )
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
                    .when(header_status_visible, |this| {
                        this.child(self.header_status_control(header_status, cx))
                    }),
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
                                .group("title-mobile-right")
                                .size(px(28.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded_sm()
                                .text_color(rgb(palette.text_muted))
                                .cursor_pointer()
                                .hover(|this| {
                                    this.bg(rgb(palette.hover)).text_color(rgb(palette.text))
                                })
                                .child(
                                    svg()
                                        .size(px(16.))
                                        .path("icons/menu/sidebar.svg")
                                        .text_color(rgb(palette.text_muted))
                                        .group_hover("title-mobile-right", |this| {
                                            this.text_color(rgb(palette.text))
                                        }),
                                )
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
                    .when(!macos, |this| {
                        this.child(window_control_button(
                            palette,
                            "window-min",
                            "icons/window/minimize.svg",
                            WindowControlArea::Min,
                            cx.listener(|this, _, window, cx| {
                                this.handle_window_minimize(window, cx);
                            }),
                        ))
                        .child(window_control_button(
                            palette,
                            "window-max",
                            if window.is_maximized() {
                                "icons/window/restore.svg"
                            } else {
                                "icons/window/maximize.svg"
                            },
                            WindowControlArea::Max,
                            |_, window, _| window.zoom_window(),
                        ))
                        .child(window_control_button(
                            palette,
                            "window-close",
                            "icons/window/close.svg",
                            WindowControlArea::Close,
                            cx.listener(|this, _, window, cx| {
                                this.handle_window_close_request(window, cx);
                            }),
                        ))
                    }),
            )
    }

    fn header_status_control(
        &self,
        content: HeaderStatusContent,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let menu_open = self.header_status.menu_open;
        let select_label = self.tr("headerStatus.select");

        div()
            .relative()
            .max_w(px(520.))
            .flex()
            .items_center()
            .gap_0()
            .rounded_sm()
            .text_xs()
            .text_color(rgb(palette.text_muted))
            .when(menu_open, |this| this.bg(rgb(palette.hover)))
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap_2()
                    .overflow_hidden()
                    .px_2()
                    .py_1()
                    .window_control_area(WindowControlArea::Drag)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _, cx| {
                            this.mark_title_drag_activity();
                            cx.notify();
                        }),
                    )
                    .child(
                        svg()
                            .size(px(14.))
                            .flex_none()
                            .path(content.icon_path)
                            .text_color(rgb(palette.text_muted)),
                    )
                    .child(div().min_w_0().overflow_hidden().child(content.label)),
            )
            .child(
                div()
                    .id("header-status-menu-trigger")
                    .size(px(24.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_sm()
                    .cursor_pointer()
                    .hover(move |this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_, _, _, cx| cx.stop_propagation()),
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.toggle_header_status_menu(cx);
                    }))
                    .tooltip(move |_, cx| cx.new(|_| ChromeTooltip::new(select_label)).into())
                    .child(
                        svg()
                            .size(px(14.))
                            .path("icons/chevron-down.svg")
                            .text_color(rgb(palette.text_muted)),
                    ),
            )
            .when(menu_open, |this| {
                this.child(self.header_status_dropdown(cx))
            })
    }

    fn header_status_dropdown(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let selected = HeaderStatusMode::from_setting(&self.settings.ui_header_status_mode);
        let mut menu = div()
            .id("header-status-menu")
            .absolute()
            .top(px(30.))
            .right_0()
            .w(px(196.))
            .rounded_md()
            .border_1()
            .border_color(rgb(palette.border))
            .bg(self.shell_surface_color(palette.surface))
            .shadow_lg()
            .py_1()
            .flex()
            .flex_col()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, _, cx| cx.stop_propagation()),
            );

        for mode in HeaderStatusMode::ALL {
            menu = menu.child(title_menu_item(
                palette,
                format!("header-status-mode-{}", mode.persistence_id()),
                Some(mode.icon_path()),
                selected == mode,
                self.tr(mode.i18n_key()),
                None,
                cx.listener(move |this, _, _, cx| {
                    this.set_header_status_mode(mode, cx);
                }),
            ));
        }

        menu.child(title_menu_separator(palette))
            .child(title_menu_item(
                palette,
                "header-status-hide",
                Some("icons/close.svg"),
                false,
                self.tr("headerStatus.hide"),
                None,
                cx.listener(|this, _, _, cx| {
                    this.set_header_status_visible(false, cx);
                }),
            ))
    }

    fn header_status_content(&self) -> HeaderStatusContent {
        let mode = HeaderStatusMode::from_setting(&self.settings.ui_header_status_mode);
        match mode {
            HeaderStatusMode::Session => HeaderStatusContent {
                icon_path: self.title_context_icon().unwrap_or(mode.icon_path()),
                label: self.title_context_label(),
            },
            HeaderStatusMode::DateTime => HeaderStatusContent {
                icon_path: mode.icon_path(),
                label: format_header_datetime(local_now(), &self.settings.language),
            },
            HeaderStatusMode::Resources | HeaderStatusMode::Host => {
                let label = self
                    .remote_stats_header_label(mode)
                    .unwrap_or_else(|| self.remote_stats_header_fallback());
                HeaderStatusContent {
                    icon_path: mode.icon_path(),
                    label,
                }
            }
        }
    }

    fn remote_stats_header_label(&self, mode: HeaderStatusMode) -> Option<String> {
        if self.active_ssh_config.is_none() || !self.settings.ui_show_remote_stats {
            return None;
        }
        let stats = self.remote_ops.stats.data.as_ref()?;
        if mode == HeaderStatusMode::Host {
            let hostname = stats.system.hostname.trim();
            return Some(format!(
                "{} - {}/{} - {}",
                if hostname.is_empty() {
                    "remote host"
                } else {
                    hostname
                },
                stats.system.os,
                stats.system.arch,
                format_uptime(stats.system.uptime_sec),
            ));
        }

        let memory_total = stats.memory.used.saturating_add(stats.memory.available);
        let tx = stats
            .networks
            .iter()
            .map(|network| network.tx_bytes_per_sec.max(0.))
            .sum::<f64>();
        let rx = stats
            .networks
            .iter()
            .map(|network| network.rx_bytes_per_sec.max(0.))
            .sum::<f64>();
        Some(format!(
            "CPU {:.0}% - RAM {}/{} - TX {} - RX {}",
            stats.cpu.usage.clamp(0., 100.),
            format_file_size(Some(stats.memory.used)),
            format_file_size(Some(memory_total)),
            format_rate(tx),
            format_rate(rx),
        ))
    }

    fn remote_stats_header_fallback(&self) -> String {
        if self.active_ssh_config.is_none() {
            self.tr("panel.resourceMonitorNoSession").to_string()
        } else if !self.settings.ui_show_remote_stats {
            self.tr("panel.resourceMonitorDisabled").to_string()
        } else if self.remote_ops.stats.consecutive_refresh_failures > 0
            && self.remote_ops.stats.data.is_none()
        {
            self.tr("panel.resourceMonitorError").to_string()
        } else {
            self.tr("common.loading").to_string()
        }
    }

    pub(in crate::features) fn toggle_header_status_menu(&mut self, cx: &mut Context<Self>) {
        self.header_status.menu_open = !self.header_status.menu_open;
        if self.header_status.menu_open {
            self.title_menu_open = None;
            self.title_menu_submenu = None;
            self.open_tabs_menu_open = false;
            self.new_session_menu_open = false;
            self.new_session_all_sessions_open = false;
            self.new_session_group_menu_path.clear();
        }
        cx.notify();
    }

    pub(in crate::features) fn set_header_status_mode(
        &mut self,
        mode: HeaderStatusMode,
        cx: &mut Context<Self>,
    ) {
        self.settings.ui_header_status_mode = mode.persistence_id().to_string();
        self.settings.ui_header_status_visible = true;
        self.header_status.menu_open = false;
        self.header_status.rendered_minute = current_unix_minute();
        self.persist_header_status_settings();
        cx.notify();
    }

    pub(in crate::features) fn set_header_status_visible(
        &mut self,
        visible: bool,
        cx: &mut Context<Self>,
    ) {
        self.settings.ui_header_status_visible = visible;
        self.header_status.menu_open = false;
        self.persist_header_status_settings();
        cx.notify();
    }

    fn persist_header_status_settings(&mut self) {
        if self.settings_draft_snapshot.is_some() {
            self.terminal.view.status =
                "header status changed; apply settings to persist".to_string();
        } else {
            self.persist_ui_layout();
        }
    }

    pub(in crate::features) fn header_status_needs_remote_stats(&self) -> bool {
        self.settings.ui_header_status_visible
            && HeaderStatusMode::from_setting(&self.settings.ui_header_status_mode)
                .needs_remote_stats()
    }

    pub(in crate::features) fn header_status_clock_refresh_due(&self) -> bool {
        self.settings.ui_header_status_visible
            && HeaderStatusMode::from_setting(&self.settings.ui_header_status_mode)
                == HeaderStatusMode::DateTime
            && self.header_status.rendered_minute != current_unix_minute()
    }

    pub(in crate::features) fn refresh_header_status_clock(&mut self) -> bool {
        if !self.header_status_clock_refresh_due() {
            return false;
        }
        self.header_status.rendered_minute = current_unix_minute();
        true
    }

    pub(in crate::features) fn title_context_label(&self) -> String {
        if self.active_pending_session_start.is_some()
            && let Some(pending) = self.pending_session_display_name()
        {
            return pending;
        }
        if self.active_failed_session_start.is_some()
            && let Some(failed) = self.failed_session_display_name()
        {
            return failed;
        }
        if let Some(session_id) = self.active_session_id.as_deref() {
            let tab_root = self.tab_root_for_session(session_id);
            let name = self
                .session_display_name(&tab_root)
                .unwrap_or_else(|| short_id(&tab_root).to_string());
            let has_custom_name = self
                .session_custom_names
                .get(&tab_root)
                .is_some_and(|value| !value.trim().is_empty());
            if !has_custom_name
                && self
                    .session_info(session_id)
                    .is_some_and(|session| session.kind == SessionKind::Ssh)
                && let Some(endpoint) = self.session_endpoint(session_id)
            {
                return format!("{name} — {endpoint}");
            }
            return name;
        }
        if let Some(pending) = self.pending_session_display_name() {
            return pending;
        }
        if let Some(failed) = self.failed_session_display_name() {
            return failed;
        }
        if let Some(failed) = self.last_connect_failure_name.as_ref() {
            return failed.clone();
        }
        "NyaTerm".to_string()
    }

    fn title_context_icon(&self) -> Option<&'static str> {
        if self.active_pending_session_start.is_some() {
            return Some("icons/conn/connect.svg");
        }
        if self.active_failed_session_start.is_some() {
            return Some("icons/session/disconnect.svg");
        }
        if let Some(session_id) = self.active_session_id.as_deref() {
            return self
                .session_info(session_id)
                .map(|session| session_kind_icon_path(session.kind));
        }
        if self.has_pending_session_start() {
            return Some("icons/conn/connect.svg");
        }
        if self.has_failed_session_start() || self.last_connect_failure_name.is_some() {
            return Some("icons/session/disconnect.svg");
        }
        None
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
                        rgba(0x00000000)
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
            self.header_status.menu_open = false;
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

fn current_unix_minute() -> i64 {
    OffsetDateTime::now_utc().unix_timestamp().div_euclid(60)
}

fn local_now() -> OffsetDateTime {
    let now = OffsetDateTime::now_utc();
    UtcOffset::current_local_offset().map_or(now, |offset| now.to_offset(offset))
}

fn format_header_datetime(datetime: OffsetDateTime, language: &str) -> String {
    let date_time = datetime
        .format(format_description!("[year]-[month]-[day] [hour]:[minute]"))
        .unwrap_or_default();
    let weekday = localized_weekday(datetime.weekday(), language);
    if language.trim().to_ascii_lowercase().starts_with("zh") {
        format!("{date_time} {weekday}")
    } else {
        format!("{weekday}, {date_time}")
    }
}

fn localized_weekday(weekday: Weekday, language: &str) -> &'static str {
    let chinese = language.trim().to_ascii_lowercase().starts_with("zh");
    match (chinese, weekday) {
        (true, Weekday::Monday) => "周一",
        (true, Weekday::Tuesday) => "周二",
        (true, Weekday::Wednesday) => "周三",
        (true, Weekday::Thursday) => "周四",
        (true, Weekday::Friday) => "周五",
        (true, Weekday::Saturday) => "周六",
        (true, Weekday::Sunday) => "周日",
        (false, Weekday::Monday) => "Mon",
        (false, Weekday::Tuesday) => "Tue",
        (false, Weekday::Wednesday) => "Wed",
        (false, Weekday::Thursday) => "Thu",
        (false, Weekday::Friday) => "Fri",
        (false, Weekday::Saturday) => "Sat",
        (false, Weekday::Sunday) => "Sun",
    }
}

#[cfg(test)]
mod tests {
    use time::{Date, Month, Time, UtcOffset};

    use super::{format_header_datetime, localized_weekday};

    #[test]
    fn formats_header_datetime_for_supported_languages() {
        let datetime = Date::from_calendar_date(2026, Month::July, 27)
            .expect("date")
            .with_time(Time::from_hms(9, 5, 0).expect("time"))
            .assume_offset(UtcOffset::from_hms(8, 0, 0).expect("offset"));

        assert_eq!(
            format_header_datetime(datetime, "en"),
            "Mon, 2026-07-27 09:05"
        );
        assert_eq!(
            format_header_datetime(datetime, "zh-CN"),
            "2026-07-27 09:05 周一"
        );
    }

    #[test]
    fn localizes_every_weekday_without_falling_back() {
        for weekday in [
            time::Weekday::Monday,
            time::Weekday::Tuesday,
            time::Weekday::Wednesday,
            time::Weekday::Thursday,
            time::Weekday::Friday,
            time::Weekday::Saturday,
            time::Weekday::Sunday,
        ] {
            assert!(!localized_weekday(weekday, "en").is_empty());
            assert!(!localized_weekday(weekday, "zh-CN").is_empty());
        }
    }
}

use gpui::{
    Context, IntoElement, MouseButton, Window, WindowControlArea, div, prelude::*, px, rgb, svg,
};
use nyaterm_transport::SessionKind;
use nyaterm_ui::{NyaDropdownMenu, NyaMenuAnchor};
use time::{OffsetDateTime, UtcOffset, Weekday, macros::format_description};

use crate::features::{
    NyaTermApp, format_file_size, format_rate, format_uptime, logo_mark, short_id,
    window_control_button,
};
use crate::models::{HeaderStatusMode, TitleMenu};

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
        let narrow_left = compact_layout && self.shell.viewport_size().0 < 1024.;
        let narrow_right = compact_layout && self.shell.viewport_size().0 < 768.;
        let header_status_visible = self.settings.summary().ui_header_status_visible;
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
        let select_label = self.tr("headerStatus.select");
        let selected =
            HeaderStatusMode::from_setting(&self.settings.summary().ui_header_status_mode);
        let mut items = HeaderStatusMode::ALL
            .into_iter()
            .map(|mode| {
                nyaterm_ui::NyaMenuItem::action(self.tr(mode.i18n_key()))
                    .icon(mode.icon_path())
                    .checked(selected == mode)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_header_status_mode(mode, cx);
                    }))
            })
            .collect::<Vec<_>>();
        items.extend([
            nyaterm_ui::NyaMenuItem::separator(),
            nyaterm_ui::NyaMenuItem::action(self.tr("headerStatus.hide"))
                .icon("icons/close.svg")
                .on_click(cx.listener(|this, _, _, cx| {
                    this.set_header_status_visible(false, cx);
                })),
        ]);

        div()
            .max_w(px(520.))
            .flex()
            .items_center()
            .gap_0()
            .rounded_sm()
            .text_xs()
            .text_color(rgb(palette.text_muted))
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
                    .flex_none()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_, _, _, cx| cx.stop_propagation()),
                    )
                    .child(
                        NyaDropdownMenu::new("header-status-menu-trigger")
                            .icon("icons/chevron-down.svg")
                            .icon_size(px(14.))
                            .tooltip(select_label)
                            .min_width(px(196.))
                            .items(items)
                            .on_trigger(cx.listener(|this, _, _, cx| {
                                this.shell.close_open_tabs_menu();
                                this.shell.close_new_session_menu();
                                cx.notify();
                            })),
                    ),
            )
    }

    fn header_status_content(&self) -> HeaderStatusContent {
        let mode = HeaderStatusMode::from_setting(&self.settings.summary().ui_header_status_mode);
        match mode {
            HeaderStatusMode::Session => HeaderStatusContent {
                icon_path: self.title_context_icon().unwrap_or(mode.icon_path()),
                label: self.title_context_label(),
            },
            HeaderStatusMode::DateTime => HeaderStatusContent {
                icon_path: mode.icon_path(),
                label: format_header_datetime(local_now(), &self.settings.summary().language),
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
        if self.session.active_ssh_config().is_none()
            || !self.settings.summary().ui_show_remote_stats
        {
            return None;
        }
        let stats_state = self.remote_ops.stats_presentation();
        let stats = stats_state.data.as_ref()?;
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
        if self.session.active_ssh_config().is_none() {
            self.tr("panel.resourceMonitorNoSession").to_string()
        } else if !self.settings.summary().ui_show_remote_stats {
            self.tr("panel.resourceMonitorDisabled").to_string()
        } else {
            let stats = self.remote_ops.stats_presentation();
            if stats.consecutive_refresh_failures > 0 && stats.data.is_none() {
                self.tr("panel.resourceMonitorError").to_string()
            } else {
                self.tr("common.loading").to_string()
            }
        }
    }

    pub(in crate::features) fn set_header_status_mode(
        &mut self,
        mode: HeaderStatusMode,
        cx: &mut Context<Self>,
    ) {
        self.settings
            .set_header_status_mode(mode.persistence_id().to_string());
        self.shell
            .set_header_status_rendered_minute(current_unix_minute());
        self.persist_header_status_settings();
        cx.notify();
    }

    pub(in crate::features) fn set_header_status_visible(
        &mut self,
        visible: bool,
        cx: &mut Context<Self>,
    ) {
        self.settings.set_header_status_visible(visible);
        self.persist_header_status_settings();
        cx.notify();
    }

    fn persist_header_status_settings(&mut self) {
        if self.shell.has_settings_draft() {
            self.shell
                .set_status("header status changed; apply settings to persist".to_string());
        } else {
            self.persist_ui_layout();
        }
    }

    pub(in crate::features) fn header_status_needs_remote_stats(&self) -> bool {
        self.settings.summary().ui_header_status_visible
            && HeaderStatusMode::from_setting(&self.settings.summary().ui_header_status_mode)
                .needs_remote_stats()
    }

    pub(in crate::features) fn header_status_clock_refresh_due(&self) -> bool {
        self.settings.summary().ui_header_status_visible
            && HeaderStatusMode::from_setting(&self.settings.summary().ui_header_status_mode)
                == HeaderStatusMode::DateTime
            && self.shell.header_status_rendered_minute() != current_unix_minute()
    }

    pub(in crate::features) fn refresh_header_status_clock(&mut self) -> bool {
        if !self.header_status_clock_refresh_due() {
            return false;
        }
        self.shell
            .set_header_status_rendered_minute(current_unix_minute());
        true
    }

    pub(in crate::features) fn title_context_label(&self) -> String {
        if self.session.start_has_active_pending()
            && let Some(pending) = self.session.start_pending_display_name()
        {
            return pending;
        }
        if self.session.start_has_active_failed()
            && let Some(failed) = self.session.start_failed_display_name()
        {
            return failed;
        }
        if let Some(session_id) = self.session.active_id() {
            let tab_root = self.tab_root_for_session(session_id);
            let name = self
                .session
                .display_name(&tab_root)
                .unwrap_or_else(|| short_id(&tab_root).to_string());
            let has_custom_name = self
                .session
                .custom_name(&tab_root)
                .is_some_and(|value| !value.trim().is_empty());
            if !has_custom_name
                && self
                    .session
                    .session_info(session_id)
                    .is_some_and(|session| session.kind == SessionKind::Ssh)
                && let Some(endpoint) = self.session.endpoint(session_id)
            {
                return format!("{name} — {endpoint}");
            }
            return name;
        }
        if let Some(pending) = self.session.start_pending_display_name() {
            return pending;
        }
        if let Some(failed) = self.session.start_failed_display_name() {
            return failed;
        }
        if let Some(failed) = self.shell.last_connect_failure_name() {
            return failed.to_string();
        }
        "NyaTerm".to_string()
    }

    fn title_context_icon(&self) -> Option<&'static str> {
        if self.session.start_has_active_pending() {
            return Some("icons/conn/connect.svg");
        }
        if self.session.start_has_active_failed() {
            return Some("icons/session/disconnect.svg");
        }
        if let Some(session_id) = self.session.active_id() {
            return self
                .session
                .session_info(session_id)
                .map(|session| session_kind_icon_path(session.kind));
        }
        if self.session.start_has_pending() {
            return Some("icons/conn/connect.svg");
        }
        if self.session.start_has_failed() || self.shell.last_connect_failure_name().is_some() {
            return Some("icons/session/disconnect.svg");
        }
        None
    }

    pub(in crate::features) fn title_menu_trigger(
        &self,
        menu: TitleMenu,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id_label = menu.label();
        let label = self.tr(menu.i18n_key());
        let items = self.title_menu_items(menu, cx);
        div()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_, _, _, cx| cx.stop_propagation()),
            )
            .child(
                NyaDropdownMenu::new(format!("title-menu-trigger-{id_label}"))
                    .label(label)
                    .anchor(NyaMenuAnchor::TopLeft)
                    .min_width(px(220.))
                    .items(items)
                    .on_trigger(cx.listener(|this, _, _, cx| {
                        this.shell.close_open_tabs_menu();
                        this.shell.close_new_session_menu();
                        cx.notify();
                    })),
            )
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

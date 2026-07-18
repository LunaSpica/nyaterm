use gpui::{
    AppContext, Bounds, Context, Entity, IntoElement, Render, Subscription, TitlebarOptions,
    Window, WindowBounds, WindowHandle, WindowKind, WindowOptions, div, prelude::*, px, rgb, size,
};

use super::NyaTermApp;

pub(in crate::features) struct SettingsWindow {
    app: Entity<NyaTermApp>,
    _app_subscription: Subscription,
}

impl SettingsWindow {
    fn new(app: Entity<NyaTermApp>, cx: &mut Context<Self>) -> Self {
        let app_subscription = cx.observe(&app, |_, _, cx| cx.notify());
        Self {
            app,
            _app_subscription: app_subscription,
        }
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.app.read(cx).settings_draft_snapshot.is_none() {
            self.app.update(cx, |app, cx| {
                app.settings_window = None;
                cx.notify();
            });
            window.defer(cx, |window, _| window.remove_window());
            return div().size_full().into_any_element();
        }

        let viewport = window.viewport_size();
        let viewport_width = f32::from(viewport.width);
        let (palette, font_family, font_size, title) = self.app.read_with(cx, |app, _| {
            (
                app.theme_palette(),
                app.gpui_ui_font_family(),
                app.settings.ui_font_size.clamp(12, 24) as f32,
                app.tr("settings.title").to_string(),
            )
        });
        window.set_window_title(&title);
        let content = self
            .app
            .update(cx, |app, cx| app.settings_window_view(viewport_width, cx));

        div()
            .size_full()
            .overflow_hidden()
            .bg(rgb(palette.bg))
            .text_color(rgb(palette.text))
            .font_family(font_family)
            .text_size(px(font_size))
            .child(content)
            .into_any_element()
    }
}

impl NyaTermApp {
    pub(in crate::features) fn activate_settings_window(&mut self, cx: &mut Context<Self>) -> bool {
        let Some(handle) = self.settings_window else {
            return false;
        };
        if handle
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
        {
            true
        } else {
            self.settings_window = None;
            false
        }
    }

    pub(in crate::features) fn open_settings_window(&mut self, cx: &mut Context<Self>) -> bool {
        if self.activate_settings_window(cx) {
            return true;
        }

        let app = cx.entity();
        let title = self.tr("settings.title").to_string();
        let bounds = Bounds::centered(None, size(px(800.), px(560.)), cx);
        let close_app = app.clone();
        let view_app = app.clone();
        let result: anyhow::Result<WindowHandle<SettingsWindow>> = cx.open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some(title.into()),
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(640.), px(480.))),
                kind: WindowKind::Floating,
                is_minimizable: false,
                ..Default::default()
            },
            move |window, cx| {
                window.on_window_should_close(cx, move |_, cx| {
                    close_app.update(cx, |app, cx| {
                        app.cancel_settings(cx);
                        app.settings_window = None;
                    });
                    true
                });
                cx.new(|cx| SettingsWindow::new(view_app, cx))
            },
        );

        match result {
            Ok(handle) => {
                self.settings_window = Some(handle);
                cx.notify();
                true
            }
            Err(error) => {
                self.settings_window = None;
                self.terminal_status = format!("failed to open settings window: {error}");
                cx.notify();
                false
            }
        }
    }
}

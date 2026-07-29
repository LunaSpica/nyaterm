use gpui::{
    App, AppContext, Bounds, Context, Entity, IntoElement, Render, Subscription, Window,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, div, prelude::*, px, rgb, size,
};

use crate::features::{NyaTermApp, child_window_header, child_window_titlebar};

pub(in crate::features::commands) struct QuickCommandWindow {
    app: Entity<NyaTermApp>,
    _app_subscription: Subscription,
}

impl QuickCommandWindow {
    fn new(app: Entity<NyaTermApp>, cx: &mut Context<Self>) -> Self {
        let app_subscription = cx.observe(&app, |_, _, cx| cx.notify());
        Self {
            app,
            _app_subscription: app_subscription,
        }
    }
}

impl Render for QuickCommandWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.app.read(cx).commands.quick_editor().is_none() {
            self.app.update(cx, |app, cx| {
                app.commands.close_quick_editor();
                cx.notify();
            });
            window.defer(cx, |window, _| window.remove_window());
            return div().size_full().into_any_element();
        }

        let viewport_width = f32::from(window.viewport_size().width);
        let (palette, font_family, font_size, title) = self.app.read_with(cx, |app, _| {
            (
                app.theme_palette(),
                app.gpui_ui_font_family(),
                app.settings.summary().ui_font_size.clamp(12, 24) as f32,
                app.quick_command_editor_title().to_string(),
            )
        });
        window.set_window_title(&title);
        let content = self.app.update(cx, |app, cx| {
            app.quick_command_editor_window_view(viewport_width, cx)
        });
        let close_app = self.app.clone();

        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(palette.bg))
            .text_color(rgb(palette.text))
            .font_family(font_family)
            .text_size(px(font_size))
            .child(child_window_header(
                palette,
                title,
                None,
                false,
                window.is_maximized(),
                move |_, window, cx| {
                    close_app.update(cx, |app, cx| app.close_quick_command_editor(cx));
                    window.remove_window();
                },
            ))
            .child(div().flex_1().min_h_0().overflow_hidden().child(content))
            .into_any_element()
    }
}

impl NyaTermApp {
    pub(in crate::features) fn quick_command_editor_title(&self) -> &'static str {
        if self
            .commands
            .quick_editor()
            .is_some_and(|editor| editor.original.is_some())
        {
            self.tr("quickCommands.editCommand")
        } else {
            self.tr("quickCommands.addCommand")
        }
    }

    pub(in crate::features) fn activate_quick_command_window(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(handle) = self.commands.quick_editor_window() else {
            return false;
        };
        let app = cx.entity();
        cx.defer(move |cx| {
            if handle
                .update(cx, |_, window, _| window.activate_window())
                .is_err()
            {
                let _ = app.update(cx, |app, cx| {
                    if app.commands.clear_quick_editor_window_if(handle) {
                        cx.notify();
                    }
                });
            }
        });
        true
    }

    pub(in crate::features) fn open_quick_command_window(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.activate_quick_command_window(cx) {
            return true;
        }
        if self.commands.quick_editor_window_is_pending() {
            return true;
        }

        if !self.commands.request_quick_editor_window() {
            return false;
        }
        cx.notify();
        let app = cx.entity();
        cx.defer(move |cx| {
            let should_open = app.read(cx).commands.quick_editor_window_is_pending();
            if should_open {
                open_quick_command_window_now_from_app(app, cx);
            }
        });
        true
    }
}

fn open_quick_command_window_now_from_app(app: Entity<NyaTermApp>, cx: &mut App) {
    if app.read(cx).commands.quick_editor_window().is_some() {
        let _ = app.update(cx, |app, cx| {
            app.commands.cancel_quick_editor_window_request();
            app.activate_quick_command_window(cx);
            cx.notify();
        });
        return;
    }
    if app.read(cx).commands.quick_editor().is_none() {
        let _ = app.update(cx, |app, cx| {
            app.commands.cancel_quick_editor_window_request();
            cx.notify();
        });
        return;
    }

    let title = app.read(cx).quick_command_editor_title().to_string();
    let bounds = Bounds::centered(None, size(px(540.), px(640.)), cx);
    let close_app = app.clone();
    let view_app = app.clone();
    let result: anyhow::Result<WindowHandle<QuickCommandWindow>> = cx.open_window(
        WindowOptions {
            titlebar: child_window_titlebar(title),
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(420.), px(480.))),
            kind: WindowKind::Floating,
            is_minimizable: false,
            ..Default::default()
        },
        move |window, cx| {
            window.on_window_should_close(cx, move |_, cx| {
                close_app.update(cx, |app, cx| {
                    app.commands.close_quick_editor();
                    app.shell
                        .set_status("quick command editor closed".to_string());
                    cx.notify();
                });
                true
            });
            let editor_focus = view_app.read(cx).commands.quick_editor_focus().clone();
            window.focus(&editor_focus);
            cx.new(|cx| QuickCommandWindow::new(view_app, cx))
        },
    );

    let _ = app.update(cx, |app, cx| match result {
        Ok(handle) => {
            app.commands.finish_quick_editor_window_open(Some(handle));
            cx.notify();
        }
        Err(error) => {
            app.commands.finish_quick_editor_window_open(None);
            app.shell
                .set_status(format!("failed to open quick command window: {error}"));
            cx.notify();
        }
    });
}

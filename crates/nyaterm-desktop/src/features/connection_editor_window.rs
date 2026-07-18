use gpui::{
    AppContext, Bounds, Context, Entity, IntoElement, Render, Subscription, Window, WindowBounds,
    WindowHandle, WindowKind, WindowOptions, div, prelude::*, px, rgb, size,
};

use super::{NyaTermApp, child_window_header, child_window_titlebar};

pub(in crate::features) struct ConnectionEditorWindow {
    app: Entity<NyaTermApp>,
    _app_subscription: Subscription,
}

impl ConnectionEditorWindow {
    fn new(app: Entity<NyaTermApp>, cx: &mut Context<Self>) -> Self {
        let app_subscription = cx.observe(&app, |_, _, cx| cx.notify());
        Self {
            app,
            _app_subscription: app_subscription,
        }
    }
}

impl Render for ConnectionEditorWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(editor) = self.app.read(cx).connection_editor.clone() else {
            self.app.update(cx, |app, cx| {
                app.connection_editor_window = None;
                cx.notify();
            });
            window.defer(cx, |window, _| window.remove_window());
            return div().size_full().into_any_element();
        };

        let (palette, font_family, font_size, title) = self.app.read_with(cx, |app, _| {
            (
                app.theme_palette(),
                app.gpui_ui_font_family(),
                app.settings.ui_font_size.clamp(12, 24) as f32,
                app.connection_editor_title().to_string(),
            )
        });
        window.set_window_title(&title);
        let content = self
            .app
            .update(cx, |app, cx| app.connection_editor_window_view(editor, cx));
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
                move |_, window, cx| {
                    close_app.update(cx, |app, cx| app.close_connection_editor(cx));
                    window.remove_window();
                },
            ))
            .child(div().flex_1().min_h_0().overflow_hidden().child(content))
            .into_any_element()
    }
}

impl NyaTermApp {
    pub(in crate::features) fn connection_editor_title(&self) -> &'static str {
        if self
            .connection_editor
            .as_ref()
            .is_some_and(|editor| editor.id.is_some())
        {
            self.tr("dialog.editConnection")
        } else {
            self.tr("dialog.newConnection")
        }
    }

    pub(in crate::features) fn activate_connection_editor_window(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(handle) = self.connection_editor_window else {
            return false;
        };
        if handle
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
        {
            true
        } else {
            self.connection_editor_window = None;
            false
        }
    }

    pub(in crate::features) fn open_connection_editor_window(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.activate_connection_editor_window(cx) {
            return true;
        }

        let app = cx.entity();
        let title = self.connection_editor_title().to_string();
        let bounds = Bounds::centered(None, size(px(520.), px(620.)), cx);
        let close_app = app.clone();
        let view_app = app.clone();
        let result: anyhow::Result<WindowHandle<ConnectionEditorWindow>> = cx.open_window(
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
                        app.connection_icon_picker_open = false;
                        app.connection_editor_menu = None;
                        app.connection_editor = None;
                        app.connection_editor_window = None;
                        app.terminal_status = "connection editor closed".to_string();
                        cx.notify();
                    });
                    true
                });
                let editor_focus = view_app.read(cx).connection_editor_focus.clone();
                window.focus(&editor_focus);
                cx.new(|cx| ConnectionEditorWindow::new(view_app, cx))
            },
        );

        match result {
            Ok(handle) => {
                self.connection_editor_window = Some(handle);
                cx.notify();
                true
            }
            Err(error) => {
                self.connection_editor_window = None;
                self.terminal_status = format!("failed to open connection editor window: {error}");
                cx.notify();
                false
            }
        }
    }
}

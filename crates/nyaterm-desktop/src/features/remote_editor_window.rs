use gpui::{
    AppContext, Bounds, Context, Entity, IntoElement, Render, Subscription, TitlebarOptions,
    Window, WindowBounds, WindowHandle, WindowOptions, div, prelude::*, px, rgb, size,
};

use super::NyaTermApp;

pub(in crate::features) struct RemoteFileEditorWindow {
    app: Entity<NyaTermApp>,
    _app_subscription: Subscription,
}

impl RemoteFileEditorWindow {
    fn new(app: Entity<NyaTermApp>, cx: &mut Context<Self>) -> Self {
        let app_subscription = cx.observe(&app, |_, _, cx| cx.notify());
        Self {
            app,
            _app_subscription: app_subscription,
        }
    }
}

impl Render for RemoteFileEditorWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.app.read(cx).transfer_editor.is_none() {
            self.app.update(cx, |app, cx| {
                app.remote_editor_window = None;
                cx.notify();
            });
            window.defer(cx, |window, _| window.remove_window());
            return div().size_full().into_any_element();
        }

        let (palette, font_family, font_size, title) = self.app.read_with(cx, |app, _| {
            let workspace = app.transfer_editor.as_ref().expect("editor checked above");
            let editor = workspace
                .active_tab()
                .expect("open editor workspace has an active tab");
            let name = if editor.name.trim().is_empty() {
                &editor.remote_path
            } else {
                &editor.name
            };
            (
                app.theme_palette(),
                app.gpui_ui_font_family(),
                app.settings.ui_font_size.clamp(12, 24) as f32,
                format!(
                    "{}{}",
                    if workspace.tabs.iter().any(|tab| tab.dirty) {
                        "* "
                    } else {
                        ""
                    },
                    name
                ),
            )
        });
        window.set_window_title(&title);
        let content = self
            .app
            .update(cx, |app, cx| app.transfer_editor_window_view(cx));

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
    pub(in crate::features) fn open_remote_file_editor_window(&mut self, cx: &mut Context<Self>) {
        if let Some(handle) = self.remote_editor_window {
            if handle
                .update(cx, |_, window, _| window.activate_window())
                .is_ok()
            {
                return;
            }
            self.remote_editor_window = None;
        }

        let app = cx.entity();
        let title = self.tr("fileEditor.title").to_string();
        let bounds = Bounds::centered(None, size(px(980.), px(720.)), cx);
        let close_app = app.clone();
        let view_app = app.clone();
        let result: anyhow::Result<WindowHandle<RemoteFileEditorWindow>> = cx.open_window(
            WindowOptions {
                titlebar: Some(TitlebarOptions {
                    title: Some(title.into()),
                    ..Default::default()
                }),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(640.), px(480.))),
                ..Default::default()
            },
            move |window, cx| {
                window.on_window_should_close(cx, move |_, cx| {
                    close_app.update(cx, |app, cx| {
                        app.close_transfer_editor(cx);
                        let should_close = app.transfer_editor.is_none();
                        if should_close {
                            app.remote_editor_window = None;
                        }
                        should_close
                    })
                });
                let editor_focus = view_app.read(cx).transfer_editor_focus.clone();
                window.focus(&editor_focus);
                cx.new(|cx| RemoteFileEditorWindow::new(view_app, cx))
            },
        );

        match result {
            Ok(handle) => {
                self.remote_editor_window = Some(handle);
            }
            Err(error) => {
                self.remote_editor_window = None;
                self.terminal_status = format!("failed to open remote editor window: {error}");
            }
        }
        cx.notify();
    }
}

use gpui::{
    AppContext, Bounds, Context, Entity, IntoElement, Render, Subscription, Window, WindowBounds,
    WindowHandle, WindowKind, WindowOptions, div, prelude::*, px, rgb, size,
};

use super::{NyaTermApp, child_window_header, child_window_titlebar};

pub(in crate::features) struct TransferExternalSyncWindow {
    app: Entity<NyaTermApp>,
    prompt_id: String,
    _app_subscription: Subscription,
}

impl TransferExternalSyncWindow {
    fn new(app: Entity<NyaTermApp>, prompt_id: String, cx: &mut Context<Self>) -> Self {
        let app_subscription = cx.observe(&app, |_, _, cx| cx.notify());
        Self {
            app,
            prompt_id,
            _app_subscription: app_subscription,
        }
    }
}

impl Render for TransferExternalSyncWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(prompt) = self
            .app
            .read(cx)
            .transfer_external_sync_prompts
            .get(&self.prompt_id)
            .cloned()
        else {
            let prompt_id = self.prompt_id.clone();
            self.app.update(cx, |app, cx| {
                app.transfer_external_sync_windows.remove(&prompt_id);
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
                app.tr("fileExplorer.fileModified").to_string(),
            )
        });
        window.set_window_title(&title);
        let prompt_id = self.prompt_id.clone();
        let content = self.app.update(cx, |app, cx| {
            app.transfer_external_sync_window_view(prompt_id, prompt, cx)
        });
        let close_app = self.app.clone();
        let close_prompt_id = self.prompt_id.clone();

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
                Some("icons/sync.svg"),
                false,
                window.is_maximized(),
                move |_, window, cx| {
                    close_app.update(cx, |app, cx| {
                        app.ignore_external_editor_sync_prompt(&close_prompt_id, cx);
                    });
                    window.remove_window();
                },
            ))
            .child(div().flex_1().min_h_0().overflow_hidden().child(content))
            .into_any_element()
    }
}

impl NyaTermApp {
    pub(in crate::features) fn activate_transfer_external_sync_window(
        &mut self,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some((prompt_id, handle)) = self
            .transfer_external_sync_windows
            .iter()
            .next()
            .map(|(prompt_id, handle)| (prompt_id.clone(), *handle))
        else {
            return false;
        };
        if handle
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
        {
            true
        } else {
            self.transfer_external_sync_windows.remove(&prompt_id);
            false
        }
    }

    pub(in crate::features) fn open_transfer_external_sync_window(
        &mut self,
        prompt_id: String,
        cx: &mut Context<Self>,
    ) -> bool {
        if let Some(handle) = self.transfer_external_sync_windows.get(&prompt_id).copied() {
            if handle
                .update(cx, |_, window, _| window.activate_window())
                .is_ok()
            {
                return true;
            }
            self.transfer_external_sync_windows.remove(&prompt_id);
        }
        if !self.transfer_external_sync_prompts.contains_key(&prompt_id) {
            return false;
        }

        let app = cx.entity();
        let title = self.tr("fileExplorer.fileModified").to_string();
        let bounds = Bounds::centered(None, size(px(440.), px(240.)), cx);
        let close_app = app.clone();
        let close_prompt_id = prompt_id.clone();
        let view_app = app.clone();
        let view_prompt_id = prompt_id.clone();
        let result: anyhow::Result<WindowHandle<TransferExternalSyncWindow>> = cx.open_window(
            WindowOptions {
                titlebar: child_window_titlebar(title),
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                kind: WindowKind::Floating,
                is_resizable: false,
                is_minimizable: false,
                ..Default::default()
            },
            move |window, cx| {
                window.on_window_should_close(cx, move |_, cx| {
                    close_app.update(cx, |app, cx| {
                        app.ignore_external_editor_sync_prompt(&close_prompt_id, cx);
                    });
                    true
                });
                let prompt_focus = view_app.read(cx).transfer_external_sync_focus.clone();
                window.focus(&prompt_focus);
                cx.new(|cx| TransferExternalSyncWindow::new(view_app, view_prompt_id, cx))
            },
        );

        match result {
            Ok(handle) => {
                self.transfer_external_sync_windows
                    .insert(prompt_id, handle);
                cx.notify();
                true
            }
            Err(error) => {
                self.transfer_external_sync_windows.remove(&prompt_id);
                self.terminal_status = format!("failed to open auto-upload window: {error}");
                cx.notify();
                false
            }
        }
    }
}

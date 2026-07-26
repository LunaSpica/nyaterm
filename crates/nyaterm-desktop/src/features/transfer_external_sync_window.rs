use gpui::{
    App, AppContext, Bounds, Context, Entity, IntoElement, Render, Subscription, Window,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, div, prelude::*, px, rgb, size,
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
            .transfer
            .external_sync
            .prompts
            .get(&self.prompt_id)
            .cloned()
        else {
            let prompt_id = self.prompt_id.clone();
            self.app.update(cx, |app, cx| {
                app.transfer.external_sync.windows.remove(&prompt_id);
                app.transfer
                    .external_sync
                    .window_open_pending
                    .remove(&prompt_id);
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
            .transfer
            .external_sync
            .windows
            .iter()
            .next()
            .map(|(prompt_id, handle)| (prompt_id.clone(), *handle))
        else {
            return false;
        };
        let app = cx.entity();
        cx.defer(move |cx| {
            if handle
                .update(cx, |_, window, _| window.activate_window())
                .is_err()
            {
                let _ = app.update(cx, |app, cx| {
                    app.transfer.external_sync.windows.remove(&prompt_id);
                    cx.notify();
                });
            }
        });
        true
    }

    pub(in crate::features) fn open_transfer_external_sync_window(
        &mut self,
        prompt_id: String,
        cx: &mut Context<Self>,
    ) -> bool {
        if let Some(handle) = self.transfer.external_sync.windows.get(&prompt_id).copied() {
            let app = cx.entity();
            cx.defer(move |cx| {
                if handle
                    .update(cx, |_, window, _| window.activate_window())
                    .is_err()
                {
                    let _ = app.update(cx, |app, cx| {
                        app.transfer.external_sync.windows.remove(&prompt_id);
                        cx.notify();
                    });
                }
            });
            return true;
        }
        if self
            .transfer
            .external_sync
            .window_open_pending
            .contains(&prompt_id)
        {
            return true;
        }
        if !self.transfer.external_sync.prompts.contains_key(&prompt_id) {
            return false;
        }

        self.transfer
            .external_sync
            .window_open_pending
            .insert(prompt_id.clone());
        cx.notify();
        let app = cx.entity();
        cx.defer(move |cx| {
            let should_open = app
                .read(cx)
                .transfer
                .external_sync
                .window_open_pending
                .contains(&prompt_id);
            if should_open {
                open_transfer_external_sync_window_now_from_app(app, prompt_id, cx);
            }
        });
        true
    }
}

fn open_transfer_external_sync_window_now_from_app(
    app: Entity<NyaTermApp>,
    prompt_id: String,
    cx: &mut App,
) {
    if let Some(handle) = app
        .read(cx)
        .transfer
        .external_sync
        .windows
        .get(&prompt_id)
        .copied()
    {
        let _ = handle.update(cx, |_, window, _| window.activate_window());
        let _ = app.update(cx, |app, cx| {
            app.transfer
                .external_sync
                .window_open_pending
                .remove(&prompt_id);
            cx.notify();
        });
        return;
    }
    if !app
        .read(cx)
        .transfer
        .external_sync
        .prompts
        .contains_key(&prompt_id)
    {
        let _ = app.update(cx, |app, cx| {
            app.transfer
                .external_sync
                .window_open_pending
                .remove(&prompt_id);
            cx.notify();
        });
        return;
    }

    let title = app.read(cx).tr("fileExplorer.fileModified").to_string();
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
            let prompt_focus = view_app.read(cx).transfer.external_sync.focus.clone();
            window.focus(&prompt_focus);
            cx.new(|cx| TransferExternalSyncWindow::new(view_app, view_prompt_id, cx))
        },
    );

    let _ = app.update(cx, |app, cx| match result {
        Ok(handle) => {
            app.transfer
                .external_sync
                .windows
                .insert(prompt_id.clone(), handle);
            app.transfer
                .external_sync
                .window_open_pending
                .remove(&prompt_id);
            cx.notify();
        }
        Err(error) => {
            app.transfer.external_sync.windows.remove(&prompt_id);
            app.transfer
                .external_sync
                .window_open_pending
                .remove(&prompt_id);
            app.terminal.view.status = format!("failed to open auto-upload window: {error}");
            cx.notify();
        }
    });
}

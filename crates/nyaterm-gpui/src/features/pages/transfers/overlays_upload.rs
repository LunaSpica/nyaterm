use super::*;

impl NyaTermApp {
    pub(in crate::features::pages::transfers) fn open_transfer_browser_upload_menu(
        &mut self,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.transfer_browser_favorites_menu = None;
        self.transfer_browser_context_menu = None;
        self.transfer_browser_upload_menu = Some(TransferBrowserUploadMenuState {
            x: event.position.x,
            y: event.position.y + px(22.),
        });
        self.transfer_browser_status = "upload menu opened".to_string();
        cx.notify();
    }

    pub(in crate::features) fn close_transfer_browser_upload_menu(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.transfer_browser_upload_menu = None;
        cx.notify();
    }

    pub(in crate::features) fn transfer_browser_upload_menu_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let state = self
            .transfer_browser_upload_menu
            .unwrap_or(TransferBrowserUploadMenuState {
                x: px(24.),
                y: px(24.),
            });

        div()
            .id(SharedString::from("transfer-browser-upload-menu-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_transfer_browser_upload_menu(cx);
            }))
            .child(
                div()
                    .id(SharedString::from("transfer-browser-upload-menu"))
                    .absolute()
                    .top(state.y)
                    .left(state.x)
                    .w(px(176.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface))
                    .shadow_lg()
                    .py_1()
                    .flex()
                    .flex_col()
                    .on_click(|_, _, cx| {
                        cx.stop_propagation();
                    })
                    .child(upload_menu_item(
                        palette,
                        "transfer-browser-upload-menu-files",
                        "icons/fe/upload.svg",
                        "Upload Files",
                        cx.listener(|this, _, _, cx| {
                            this.close_transfer_browser_upload_menu(cx);
                            this.prompt_transfer_browser_upload_path(
                                TransferPathPromptKind::UploadFile,
                                cx,
                            );
                        }),
                    ))
                    .child(upload_menu_item(
                        palette,
                        "transfer-browser-upload-menu-folder",
                        "icons/fe/upload-folder.svg",
                        "Upload Folder",
                        cx.listener(|this, _, _, cx| {
                            this.close_transfer_browser_upload_menu(cx);
                            this.prompt_transfer_browser_upload_path(
                                TransferPathPromptKind::UploadDirectory,
                                cx,
                            );
                        }),
                    )),
            )
    }
}

fn upload_menu_item(
    palette: crate::theme::ThemePalette,
    id: impl Into<String>,
    icon_path: &'static str,
    label: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(30.))
        .px_2()
        .mx_1()
        .rounded_sm()
        .flex()
        .items_center()
        .gap_2()
        .cursor_pointer()
        .text_size(px(12.))
        .text_color(rgb(palette.text))
        .hover(|this| {
            this.bg(rgb(palette.surface_elevated))
                .text_color(rgb(0xffffff))
        })
        .child(
            svg()
                .size(px(16.))
                .flex_none()
                .path(icon_path)
                .text_color(rgb(palette.text_muted)),
        )
        .child(label)
        .on_click(on_click)
}

use super::*;

impl NyaTermApp {
    pub(in crate::ui::view::pages::transfers) fn open_transfer_browser_favorites_menu(
        &mut self,
        event: &MouseDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.transfer_browser_favorites_menu = Some(TransferBrowserFavoritesMenuState {
            x: event.position.x,
            y: event.position.y + px(18.),
        });
        self.transfer_browser_status = "favorite directories opened".to_string();
        cx.notify();
    }

    pub(in crate::ui::view) fn close_transfer_browser_favorites_menu(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.transfer_browser_favorites_menu = None;
        cx.notify();
    }

    pub(in crate::ui::view) fn transfer_browser_favorites_menu_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let state =
            self.transfer_browser_favorites_menu
                .unwrap_or(TransferBrowserFavoritesMenuState {
                    x: px(24.),
                    y: px(24.),
                });
        let current_path = normalized_transfer_browser_path(&self.transfer_browser_path);
        let is_current_favorite = self
            .transfer_browser_favorites
            .iter()
            .any(|path| path == &current_path);
        let favorite_paths = self
            .transfer_browser_favorites
            .iter()
            .cloned()
            .collect::<Vec<_>>();

        let mut list = div().flex().flex_col().gap_1();
        for path in favorite_paths {
            let is_current = path == current_path;
            let open_path = path.clone();
            let remove_path = path.clone();
            list = list.child(
                div()
                    .id(SharedString::from(format!(
                        "transfer-browser-favorite-menu-item-{path}"
                    )))
                    .min_h(px(30.))
                    .rounded_sm()
                    .border_1()
                    .border_color(if is_current {
                        rgb(0x256d3f)
                    } else {
                        rgb(0x303848)
                    })
                    .bg(if is_current {
                        rgb(0x17253b)
                    } else {
                        rgb(0x151b27)
                    })
                    .px_2()
                    .flex()
                    .items_center()
                    .gap_2()
                    .cursor_pointer()
                    .hover(|this| this.bg(rgb(0x223047)))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.close_transfer_browser_favorites_menu(cx);
                        this.open_transfer_browser_directory(open_path.clone(), window, cx);
                    }))
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .font_family("JetBrains Mono")
                            .text_size(px(10.))
                            .text_color(if is_current {
                                rgb(0x93c5fd)
                            } else {
                                rgb(0xdbeafe)
                            })
                            .child(truncate_preview(&path, 46)),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!(
                                "transfer-browser-favorite-menu-remove-{remove_path}"
                            )))
                            .size(px(20.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .text_color(rgb(0x86efac))
                            .hover(|this| this.bg(rgb(0x263142)).text_color(rgb(0xfca5a5)))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                                cx.stop_propagation();
                                this.remove_transfer_browser_favorite_path(remove_path.clone(), cx);
                            }))
                            .child("x"),
                    ),
            );
        }

        div()
            .id(SharedString::from(
                "transfer-browser-favorites-menu-overlay",
            ))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .on_click(cx.listener(|this, _, _, cx| {
                this.close_transfer_browser_favorites_menu(cx);
            }))
            .child(
                div()
                    .id(SharedString::from("transfer-browser-favorites-menu"))
                    .absolute()
                    .top(state.y)
                    .left(state.x)
                    .w(px(300.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(0x303848))
                    .bg(rgb(0x0b0f16))
                    .shadow_lg()
                    .p_3()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .on_click(|_, _, cx| {
                        cx.stop_propagation();
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_2()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight(800.))
                                    .text_color(rgb(0xe5edf7))
                                    .child("Favorite Directories"),
                            )
                            .child(status_pill(
                                if is_current_favorite {
                                    "saved"
                                } else {
                                    "browse"
                                },
                                if is_current_favorite {
                                    rgb(0x86efac)
                                } else {
                                    rgb(0x93c5fd)
                                },
                                rgb(0x17253b),
                            )),
                    )
                    .child(favorite_menu_button(
                        "transfer-browser-favorite-menu-add-current",
                        if is_current_favorite {
                            "Move Current To Top"
                        } else {
                            "Add Current Directory"
                        },
                        cx.listener(|this, _, _, cx| {
                            this.add_current_transfer_browser_favorite(cx);
                        }),
                    ))
                    .child(
                        div()
                            .border_t_1()
                            .border_color(rgb(0x202633))
                            .pt_2()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .when(self.transfer_browser_favorites.is_empty(), |this| {
                                this.child(
                                    div()
                                        .rounded_sm()
                                        .border_1()
                                        .border_color(rgb(0x202633))
                                        .bg(rgb(0x10151e))
                                        .px_2()
                                        .py_2()
                                        .text_xs()
                                        .text_color(rgb(0x64748b))
                                        .child("No favorite directories yet."),
                                )
                            })
                            .when(!self.transfer_browser_favorites.is_empty(), |this| {
                                this.child(list)
                            }),
                    ),
            )
    }
}

fn favorite_menu_button(
    id: impl Into<String>,
    label: impl Into<SharedString>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(SharedString::from(id.into()))
        .h(px(30.))
        .px_2()
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x303848))
        .bg(rgb(0x151b27))
        .text_color(rgb(0xdbeafe))
        .text_xs()
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x223047)))
        .child(label.into())
        .on_click(on_click)
}

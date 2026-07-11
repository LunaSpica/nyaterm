use super::*;

impl NyaTermApp {
    pub(super) fn transfer_browser_path_row(
        &mut self,
        current_browser_path: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let display_browser_path = display_transfer_browser_home_path(
            &current_browser_path,
            &self.transfer_browser_home_dir,
        );
        let is_current_favorite = self
            .transfer_browser_favorites
            .iter()
            .any(|path| path == &current_browser_path);
        let history_paths = self
            .transfer_browser_visited_history
            .iter()
            .cloned()
            .take(5)
            .collect::<Vec<_>>();
        let favorite_paths = self
            .transfer_browser_favorites
            .iter()
            .cloned()
            .take(8)
            .collect::<Vec<_>>();
        let path_draft_value = if self.transfer_browser_path_draft.is_empty() {
            "Type remote path".to_string()
        } else {
            format!("{}|", self.transfer_browser_path_draft)
        };

        // Tauri FileExplorerPathBar: minHeight ~26px, mono path, favorites on the right.
        let palette = self.theme_palette();
        div()
            .flex()
            .flex_col()
            .gap_0()
            .min_h(px(26.))
            .border_b_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface))
            .px_2()
            .py(px(2.))
            .justify_center()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(px(10.))
                            .font_weight(FontWeight(600.))
                            .text_color(rgb(palette.text_dimmed))
                            .child("PATH"),
                    )
                    .when(self.transfer_browser_path_editing, |this| {
                        this.child(
                            div()
                                .id(SharedString::from("transfer-browser-path-input"))
                                .h(px(22.))
                                .flex_1()
                                .min_w_0()
                                .rounded_sm()
                                .border_1()
                                .border_color(rgb(0x256d3f))
                                .bg(rgb(0x0d1320))
                                .px_2()
                                .flex()
                                .items_center()
                                .font_family("JetBrains Mono")
                                .text_xs()
                                .text_color(if self.transfer_browser_path_draft.is_empty() {
                                    rgb(0x64748b)
                                } else {
                                    rgb(0xdbeafe)
                                })
                                .track_focus(&self.transfer_browser_path_focus)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    window.focus(&this.transfer_browser_path_focus);
                                    cx.notify();
                                }))
                                .on_key_down(cx.listener(
                                    |this, event: &KeyDownEvent, window, cx| {
                                        cx.stop_propagation();
                                        this.handle_transfer_browser_path_key_down(
                                            event, window, cx,
                                        );
                                    },
                                ))
                                .child(truncate_preview(&path_draft_value, 120)),
                        )
                    })
                    .when(!self.transfer_browser_path_editing, |this| {
                        this.child(transfer_browser_path_breadcrumbs(
                            current_browser_path.clone(),
                            self.transfer_browser_path.clone(),
                            cx,
                        ))
                        .child(
                            div()
                                .id(SharedString::from("transfer-browser-path-display"))
                                .max_w(px(280.))
                                .rounded_sm()
                                .px_2()
                                .py_1()
                                .font_family("JetBrains Mono")
                                .text_size(px(10.))
                                .text_color(rgb(0x94a3b8))
                                .cursor_pointer()
                                .hover(|this| this.bg(rgb(0x18202b)).text_color(rgb(0xdbeafe)))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.begin_transfer_browser_path_edit(window, cx);
                                }))
                                .child(truncate_preview(&display_browser_path, 44)),
                        )
                    })
                    .child(
                        div()
                            .id(SharedString::from("transfer-browser-path-favorite"))
                            .size(px(22.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .border_1()
                            .border_color(if is_current_favorite {
                                rgb(0x2f7d4f)
                            } else {
                                rgb(0x303848)
                            })
                            .bg(if is_current_favorite {
                                rgb(0x123024)
                            } else {
                                rgb(0x10151e)
                            })
                            .text_sm()
                            .text_color(if is_current_favorite {
                                rgb(0x86efac)
                            } else {
                                rgb(0x64748b)
                            })
                            .cursor_pointer()
                            .hover(|this| this.bg(rgb(0x18202b)).text_color(rgb(0xd1fae5)))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, event: &MouseDownEvent, _window, cx| {
                                    cx.stop_propagation();
                                    this.open_transfer_browser_favorites_menu(event, cx);
                                }),
                            )
                            .child(
                                svg()
                                    .size(px(14.))
                                    .flex_none()
                                    .path(if is_current_favorite {
                                        "icons/fe/star.svg"
                                    } else {
                                        "icons/fe/star-outline.svg"
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .ml_auto()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(palette, 
                                "transfer-browser-copy-current-path",
                                "Copy",
                                cx.listener(|this, _, _, cx| {
                                    this.copy_current_transfer_browser_path(cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "transfer-browser-send-current-path",
                                "Send",
                                cx.listener(|this, _, _, cx| {
                                    this.send_current_transfer_browser_path_to_terminal(cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "transfer-browser-current-properties",
                                "Props",
                                cx.listener(|this, _, window, cx| {
                                    this.open_current_transfer_browser_properties(window, cx);
                                }),
                            )),
                    ),
            )
            .when(
                self.transfer_browser_path_editing
                    && (!history_paths.is_empty() || !favorite_paths.is_empty()),
                |this| {
                    this.child(transfer_browser_path_quick_lists(
                        current_browser_path,
                        self.transfer_browser_home_dir.clone(),
                        history_paths,
                        favorite_paths,
                        cx,
                    ))
                },
            )
    }

    pub(super) fn copy_current_transfer_browser_path(&mut self, cx: &mut Context<Self>) {
        let path = normalized_transfer_browser_path(&self.transfer_browser_path);
        cx.write_to_clipboard(ClipboardItem::new_string(path.clone()));
        self.terminal_status = "copied current remote directory".to_string();
        self.transfer_browser_status = truncate_preview(&path, 92);
        cx.notify();
    }

    pub(super) fn send_current_transfer_browser_path_to_terminal(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if self.active_session_id.is_none() {
            self.terminal_status = "start a session before sending remote path".to_string();
            cx.notify();
            return;
        }
        let path = normalized_transfer_browser_path(&self.transfer_browser_path);
        self.send_terminal_input(path.clone().into_bytes(), cx);
        self.terminal_status = "sent current remote directory to terminal".to_string();
        self.transfer_browser_status = truncate_preview(&path, 92);
        cx.notify();
    }

    pub(super) fn begin_transfer_browser_path_edit(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.transfer_browser_path_draft =
            normalized_transfer_browser_path(&self.transfer_browser_path);
        self.transfer_browser_path_editing = true;
        self.transfer_browser_status = "editing remote directory path".to_string();
        self.start_transfer_browser_home_dir_job(cx);
        window.focus(&self.transfer_browser_path_focus);
        cx.notify();
    }

    pub(super) fn cancel_transfer_browser_path_edit(&mut self, cx: &mut Context<Self>) {
        self.transfer_browser_path_draft.clear();
        self.transfer_browser_path_editing = false;
        self.transfer_browser_status = "remote directory path edit cancelled".to_string();
        cx.notify();
    }

    pub(super) fn submit_transfer_browser_path_edit(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = expand_transfer_browser_home_path(
            &self.transfer_browser_path_draft,
            &self.transfer_browser_home_dir,
        );
        if path.is_empty() {
            self.transfer_browser_status = "enter a remote directory path".to_string();
            cx.notify();
            return;
        }
        if path == "~" || path.starts_with("~/") {
            self.transfer_browser_status = if self.transfer_browser_home_dir_pending {
                "remote home is still resolving".to_string()
            } else {
                "remote home is unavailable for this session".to_string()
            };
            cx.notify();
            return;
        }
        self.transfer_browser_path_draft.clear();
        self.transfer_browser_path_editing = false;
        self.open_transfer_browser_directory(path, window, cx);
    }

    pub(super) fn handle_transfer_browser_path_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "enter" => {
                self.submit_transfer_browser_path_edit(window, cx);
            }
            "escape" => {
                self.cancel_transfer_browser_path_edit(cx);
            }
            "backspace" => {
                self.transfer_browser_path_draft.pop();
                self.transfer_browser_status = "editing remote directory path".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.transfer_browser_path_draft.push_str(input);
                    self.transfer_browser_status = "editing remote directory path".to_string();
                    cx.notify();
                }
            }
        }
    }
}

fn transfer_browser_path_quick_lists(
    current_browser_path: String,
    home_dir: String,
    history_paths: Vec<String>,
    favorite_paths: Vec<String>,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    div()
        .ml(px(40.))
        .flex()
        .flex_col()
        .gap_2()
        .rounded_sm()
        .border_1()
        .border_color(rgb(0x263142))
        .bg(rgb(0x0d1320))
        .p_2()
        .when(!history_paths.is_empty(), |this| {
            this.child(transfer_browser_path_history_list(
                current_browser_path.clone(),
                home_dir.clone(),
                history_paths,
                cx,
            ))
        })
        .when(!favorite_paths.is_empty(), |this| {
            this.child(transfer_browser_path_quick_list(
                "Favorites",
                current_browser_path,
                home_dir,
                favorite_paths,
                true,
                cx,
            ))
        })
}

fn transfer_browser_path_history_list(
    current_browser_path: String,
    home_dir: String,
    paths: Vec<String>,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let mut list = div().flex().flex_col().gap_1().child(
        div()
            .text_size(px(10.))
            .font_weight(FontWeight(800.))
            .text_color(rgb(0x64748b))
            .child("History"),
    );

    for path in paths {
        let is_current = path == current_browser_path;
        let display_path = display_transfer_browser_home_path(&path, &home_dir);
        let open_path = path.clone();
        list = list.child(
            div()
                .id(SharedString::from(format!(
                    "transfer-browser-path-history-{path}"
                )))
                .h(px(24.))
                .w_full()
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
                    rgb(0x10151e)
                })
                .px_2()
                .flex()
                .items_center()
                .font_family("JetBrains Mono")
                .text_size(px(10.))
                .text_color(if is_current {
                    rgb(0x93c5fd)
                } else {
                    rgb(0xdbeafe)
                })
                .cursor_pointer()
                .hover(|this| this.bg(rgb(0x18202b)))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.transfer_browser_path_editing = false;
                    this.open_transfer_browser_directory(open_path.clone(), window, cx);
                }))
                .child(truncate_preview(&display_path, 72)),
        );
    }

    list
}

fn transfer_browser_path_quick_list(
    label: &'static str,
    current_browser_path: String,
    home_dir: String,
    paths: Vec<String>,
    allow_remove: bool,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let mut row = div().flex().items_center().gap_1().min_w_0().child(
        div()
            .w(px(58.))
            .flex_none()
            .text_size(px(10.))
            .font_weight(FontWeight(800.))
            .text_color(rgb(0x64748b))
            .child(label),
    );

    for path in paths {
        let is_current = path == current_browser_path;
        let display_path = display_transfer_browser_home_path(&path, &home_dir);
        let open_path = path.clone();
        let remove_path = path.clone();
        row = row.child(
            div()
                .id(SharedString::from(format!(
                    "transfer-browser-path-quick-{label}-{path}"
                )))
                .h(px(24.))
                .max_w(px(if is_current { 220. } else { 148. }))
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
                    rgb(0x10151e)
                })
                .px_1()
                .flex()
                .items_center()
                .gap_1()
                .font_family("JetBrains Mono")
                .text_size(px(10.))
                .text_color(rgb(0xdbeafe))
                .cursor_pointer()
                .hover(|this| this.bg(rgb(0x18202b)))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.transfer_browser_path_editing = false;
                    this.open_transfer_browser_directory(open_path.clone(), window, cx);
                }))
                .child(div().min_w_0().flex_1().px_1().child(truncate_preview(
                    &display_path,
                    if is_current { 38 } else { 22 },
                )))
                .when(allow_remove, |this| {
                    this.child(
                        div()
                            .id(SharedString::from(format!(
                                "transfer-browser-path-remove-favorite-{remove_path}"
                            )))
                            .size(px(16.))
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
                    )
                }),
        );
    }

    row
}

fn display_transfer_browser_home_path(path: &str, home_dir: &str) -> String {
    let path = normalized_transfer_browser_path(path);
    let home_dir = normalized_transfer_browser_path(home_dir);
    if home_dir.is_empty() || home_dir == "." {
        return path;
    }
    if path == home_dir {
        return "~".to_string();
    }
    let home_prefix = format!("{home_dir}/");
    if let Some(suffix) = path.strip_prefix(&home_prefix) {
        return format!("~/{suffix}");
    }
    path
}

fn expand_transfer_browser_home_path(path: &str, home_dir: &str) -> String {
    let trimmed = path.trim();
    let home_dir = normalized_transfer_browser_path(home_dir);
    if home_dir.is_empty() || home_dir == "." {
        return normalized_transfer_browser_path(trimmed);
    }
    if trimmed == "~" {
        return home_dir;
    }
    if let Some(suffix) = trimmed.strip_prefix("~/") {
        return normalized_transfer_browser_path(&remote_child_path(&home_dir, suffix));
    }
    normalized_transfer_browser_path(trimmed)
}

fn transfer_browser_path_breadcrumbs(
    current_browser_path: String,
    raw_browser_path: String,
    cx: &mut Context<NyaTermApp>,
) -> gpui::AnyElement {
    let mut row = div()
        .min_w_0()
        .flex_1()
        .flex()
        .items_center()
        .gap_1()
        .overflow_hidden();
    let breadcrumbs = transfer_browser_breadcrumbs(&current_browser_path);
    if breadcrumbs.is_empty() {
        return row
            .child(
                div()
                    .font_family("JetBrains Mono")
                    .text_xs()
                    .text_color(rgb(0x64748b))
                    .child(truncate_preview(&raw_browser_path, 72)),
            )
            .into_any_element();
    }

    let last_index = breadcrumbs.len().saturating_sub(1);
    for (index, (label, path)) in breadcrumbs.into_iter().enumerate() {
        if index > 0 {
            row = row.child(
                div()
                    .flex_shrink_0()
                    .font_family("JetBrains Mono")
                    .text_xs()
                    .text_color(rgb(0x475569))
                    .child("/"),
            );
        }
        let is_current = index == last_index;
        let button_path = path.clone();
        row = row.child(
            div()
                .id(SharedString::from(format!(
                    "transfer-browser-breadcrumb-{index}"
                )))
                .h(px(24.))
                .max_w(px(if is_current { 240. } else { 150. }))
                .rounded_sm()
                .border_1()
                .border_color(if is_current {
                    rgb(0x2f7d4f)
                } else {
                    rgb(0x303848)
                })
                .bg(if is_current {
                    rgb(0x123024)
                } else {
                    rgb(0x10151e)
                })
                .px_2()
                .flex()
                .items_center()
                .font_family("JetBrains Mono")
                .text_xs()
                .text_color(if is_current {
                    rgb(0xd1fae5)
                } else {
                    rgb(0xdbeafe)
                })
                .cursor_pointer()
                .hover(|this| this.bg(rgb(0x18202b)))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_transfer_browser_directory(button_path.clone(), window, cx);
                }))
                .child(truncate_preview(&label, if is_current { 40 } else { 24 })),
        );
    }

    row.into_any_element()
}

fn transfer_browser_breadcrumbs(path: &str) -> Vec<(String, String)> {
    let path = normalized_transfer_browser_path(path);
    if path.is_empty() {
        return Vec::new();
    }
    if path == "/" {
        return vec![("/".to_string(), "/".to_string())];
    }

    if path.starts_with('/') {
        let mut output = vec![("/".to_string(), "/".to_string())];
        let mut current = String::new();
        for segment in path.split('/').filter(|segment| !segment.is_empty()) {
            current.push('/');
            current.push_str(segment);
            output.push((segment.to_string(), current.clone()));
        }
        return output;
    }

    let mut output = Vec::new();
    let mut current = String::new();
    for segment in path.split('/').filter(|segment| !segment.is_empty()) {
        if current.is_empty() {
            current.push_str(segment);
        } else {
            current.push('/');
            current.push_str(segment);
        }
        output.push((segment.to_string(), current.clone()));
    }
    output
}

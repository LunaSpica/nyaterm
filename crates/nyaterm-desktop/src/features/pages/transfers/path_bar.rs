use super::*;

impl NyaTermApp {
    pub(super) fn transfer_browser_path_row(
        &mut self,
        current_browser_path: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let display_browser_path = display_transfer_browser_home_path(
            &current_browser_path,
            &self.transfer.browser.home_dir,
        );
        let is_current_favorite = self
            .transfer
            .browser
            .favorites
            .iter()
            .any(|path| path == &current_browser_path);
        let history_paths = self
            .transfer
            .browser
            .visited_history
            .iter()
            .cloned()
            .take(5)
            .collect::<Vec<_>>();
        let path_draft_value = if self.transfer.browser.path_draft.is_empty() {
            self.tr("fileExplorer.editPath").to_string()
        } else {
            format!("{}|", self.transfer.browser.path_draft)
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
            .bg(self.shell_transparent_color(palette.surface))
            .px_2()
            .py(px(2.))
            .justify_center()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .when(self.transfer.browser.path_editing, |this| {
                        this.child(
                            div()
                                .id(SharedString::from("transfer-browser-path-input"))
                                .h_full()
                                .flex_1()
                                .min_w_0()
                                .px_0()
                                .py_0()
                                .flex()
                                .items_center()
                                .font_family(crate::features::gpui_code_font_family())
                                .text_size(px(10.))
                                .text_color(if self.transfer.browser.path_draft.is_empty() {
                                    rgb(palette.text_muted)
                                } else {
                                    rgb(palette.text)
                                })
                                .track_focus(&self.transfer.browser.path_focus)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    window.focus(&this.transfer.browser.path_focus);
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
                    .when(!self.transfer.browser.path_editing, |this| {
                        this.child(
                            div()
                                .id(SharedString::from("transfer-browser-path-display"))
                                .min_w_0()
                                .flex_1()
                                .rounded_sm()
                                .px_0()
                                .py_0()
                                .font_family(crate::features::gpui_code_font_family())
                                .text_size(px(10.))
                                .text_color(rgb(palette.text_muted))
                                .overflow_hidden()
                                .cursor_pointer()
                                .hover(|this| this.text_color(rgb(palette.text)))
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.begin_transfer_browser_path_edit(window, cx);
                                }))
                                .child(truncate_preview(&display_browser_path, 120)),
                        )
                    })
                    .child(
                        div()
                            .id(SharedString::from("transfer-browser-path-favorite"))
                            .ml_1()
                            .size(px(22.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .text_sm()
                            .text_color(if is_current_favorite {
                                rgb(palette.link)
                            } else {
                                rgb(palette.text_muted)
                            })
                            .cursor_pointer()
                            .hover(|this| {
                                this.bg(rgb(palette.surface_elevated))
                                    .text_color(rgb(palette.text))
                            })
                            .tooltip({
                                let label = self.tr("fileExplorer.favorites").to_string();
                                move |_, cx| {
                                    cx.new(|_| crate::features::ChromeTooltip::new(label.clone()))
                                        .into()
                                }
                            })
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
                                    })
                                    .text_color(if is_current_favorite {
                                        rgb(palette.link)
                                    } else {
                                        rgb(palette.text_muted)
                                    }),
                            ),
                    ),
            )
            .when(
                self.transfer.browser.path_editing && !history_paths.is_empty(),
                |this| {
                    this.child(transfer_browser_path_history_list(
                        palette,
                        self.shell_surface_color(palette.surface),
                        current_browser_path,
                        self.transfer.browser.home_dir.clone(),
                        history_paths,
                        cx,
                    ))
                },
            )
    }

    pub(super) fn copy_current_transfer_browser_path(&mut self, cx: &mut Context<Self>) {
        let path = normalized_transfer_browser_path(&self.transfer.browser.path);
        cx.write_to_clipboard(ClipboardItem::new_string(path.clone()));
        self.terminal.view.status = "copied current remote directory".to_string();
        self.transfer.browser.status = truncate_preview(&path, 92);
        cx.notify();
    }

    pub(super) fn send_current_transfer_browser_path_to_terminal(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if self.active_session_id.is_none() {
            self.terminal.view.status = "start a session before sending remote path".to_string();
            cx.notify();
            return;
        }
        let path = normalized_transfer_browser_path(&self.transfer.browser.path);
        if self.send_terminal_input(path.clone().into_bytes(), cx) {
            self.terminal.view.status = "sent current remote directory to terminal".to_string();
            self.transfer.browser.status = truncate_preview(&path, 92);
            cx.notify();
        }
    }

    pub(super) fn begin_transfer_browser_path_edit(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.transfer.browser.path_draft =
            normalized_transfer_browser_path(&self.transfer.browser.path);
        self.transfer.browser.path_editing = true;
        self.transfer.browser.status = "editing remote directory path".to_string();
        self.start_transfer_browser_home_dir_job(cx);
        window.focus(&self.transfer.browser.path_focus);
        cx.notify();
    }

    pub(super) fn cancel_transfer_browser_path_edit(&mut self, cx: &mut Context<Self>) {
        self.transfer.browser.cancel_path_edit();
        cx.notify();
    }

    pub(super) fn submit_transfer_browser_path_edit(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = expand_transfer_browser_home_path(
            &self.transfer.browser.path_draft,
            &self.transfer.browser.home_dir,
        );
        if path.is_empty() {
            self.transfer.browser.status = "enter a remote directory path".to_string();
            cx.notify();
            return;
        }
        if path == "~" || path.starts_with("~/") {
            self.transfer.browser.status = if self.transfer.browser.home_dir_pending {
                "remote home is still resolving".to_string()
            } else {
                "remote home is unavailable for this session".to_string()
            };
            cx.notify();
            return;
        }
        self.transfer.browser.path_draft.clear();
        self.transfer.browser.path_editing = false;
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
                self.transfer.browser.path_draft.pop();
                self.transfer.browser.status = "editing remote directory path".to_string();
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.transfer.browser.path_draft.push_str(input);
                    self.transfer.browser.status = "editing remote directory path".to_string();
                    cx.notify();
                }
            }
        }
    }
}

fn transfer_browser_path_history_list(
    palette: crate::theme::ThemePalette,
    popup_bg: gpui::Rgba,
    current_browser_path: String,
    home_dir: String,
    paths: Vec<String>,
    cx: &mut Context<NyaTermApp>,
) -> impl IntoElement {
    let mut list = div()
        .id(SharedString::from("transfer-browser-path-history-list"))
        .mt(px(1.))
        .max_h(px(120.))
        .overflow_scroll()
        .scrollbar_width(px(6.))
        .rounded_b_md()
        .border_1()
        .border_color(rgb(palette.border))
        .bg(popup_bg)
        .shadow_lg()
        .flex()
        .flex_col();

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
                .px_2()
                .flex()
                .items_center()
                .font_family(crate::features::gpui_code_font_family())
                .text_size(px(10.))
                .text_color(if is_current {
                    rgb(palette.link)
                } else {
                    rgb(palette.text)
                })
                .cursor_pointer()
                .hover(|this| this.bg(rgb(palette.hover)))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.transfer.browser.path_editing = false;
                    this.open_transfer_browser_directory(open_path.clone(), window, cx);
                }))
                .child(truncate_preview(&display_path, 72)),
        );
    }

    list
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

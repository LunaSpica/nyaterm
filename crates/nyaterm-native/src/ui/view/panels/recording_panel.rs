use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn recording_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active_session_id = self.active_session_id.clone();
        let sessions = self.ordered_sessions();
        let session_count = sessions.len();
        let recording_count = sessions
            .iter()
            .filter(|session| self.recording_manager.is_recording(&session.id))
            .count();
        let query = self.recording_session_filter_query();

        let mut session_rows = div().flex().flex_col().gap_1().p_2();
        let mut visible_count = 0usize;
        if sessions.is_empty() {
            session_rows = session_rows.child(
                div()
                    .py_4()
                    .text_center()
                    .text_size(px(11.))
                    .text_color(rgb(0x6e7681))
                    .child("No active sessions"),
            );
        } else {
            for session in sessions {
                let session_name = self.session_display_name_by_info(&session);
                let haystack = format!(
                    "{} {} {} {}",
                    session_name,
                    session.name,
                    session_kind_label(session.kind),
                    session.id
                )
                .to_ascii_lowercase();
                if !query.is_empty() && !haystack.contains(&query) {
                    continue;
                }
                visible_count += 1;

                let session_id = session.id.clone();
                let start_session_id = session.id.clone();
                let save_session_id = session.id.clone();
                let select_session_id = session.id.clone();
                let start_session_name = session_name.clone();
                let save_session_name = session_name.clone();
                let is_current = active_session_id.as_deref() == Some(session.id.as_str());
                let session_is_recording = self.recording_manager.is_recording(&session.id);
                let kind = session_kind_label(session.kind);
                let short = short_id(&session.id).to_string();

                session_rows = session_rows.child(
                    div()
                        .id(SharedString::from(format!(
                            "recording-session-row-{session_id}"
                        )))
                        .rounded_md()
                        .px_2()
                        .py_1()
                        .bg(if is_current {
                            rgb(0x122033)
                        } else {
                            rgb(0x161b22)
                        })
                        .border_1()
                        .border_color(if is_current {
                            rgb(0x1f6feb)
                        } else {
                            rgb(0x161b22)
                        })
                        .flex()
                        .items_center()
                        .gap_2()
                        .cursor_pointer()
                        .hover(|this| this.bg(rgb(0x1c2128)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.select_session(select_session_id.clone(), cx);
                        }))
                        .child(
                            div()
                                .size(px(8.))
                                .rounded_full()
                                .flex_none()
                                .bg(if session_is_recording {
                                    rgb(0xef4444)
                                } else {
                                    rgb(0x22c55e)
                                }),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .min_w_0()
                                                .flex_1()
                                                .text_xs()
                                                .font_weight(FontWeight(600.))
                                                .text_color(rgb(0xc9d1d9))
                                                .overflow_hidden()
                                                .child(truncate_preview(&session_name, 34)),
                                        )
                                        .child(
                                            div()
                                                .px_1()
                                                .rounded_sm()
                                                .bg(rgb(0x21262d))
                                                .text_size(px(10.))
                                                .font_weight(FontWeight(700.))
                                                .text_color(rgb(0x8b949e))
                                                .child(kind),
                                        ),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(
                                            div()
                                                .font_family("JetBrains Mono")
                                                .text_size(px(10.))
                                                .text_color(rgb(0x6e7681))
                                                .child(short),
                                        )
                                        .when(session_is_recording, |this| {
                                            this.child(
                                                div()
                                                    .px_1()
                                                    .rounded_sm()
                                                    .bg(rgb(0x3d1418))
                                                    .text_size(px(10.))
                                                    .text_color(rgb(0xf85149))
                                                    .child("● REC"),
                                            )
                                        }),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_0()
                                .child(recording_action_svg_button(
                                    format!("recording-session-toggle-{session_id}"),
                                    if session_is_recording {
                                        "icons/session/stop.svg"
                                    } else {
                                        "icons/session/record.svg"
                                    },
                                    if session_is_recording {
                                        rgb(0xf85149)
                                    } else {
                                        rgb(0x8b949e)
                                    },
                                    cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        if this.recording_manager.is_recording(&start_session_id) {
                                            this.stop_recording_for_session(&start_session_id, cx);
                                        } else {
                                            this.prompt_recording_path_for_session(
                                                RecordingPathPromptKind::Start,
                                                start_session_id.clone(),
                                                start_session_name.clone(),
                                                cx,
                                            );
                                        }
                                    }),
                                ))
                                .child(recording_action_svg_button(
                                    format!("recording-session-save-{session_id}"),
                                    "icons/session/save.svg",
                                    rgb(0x8b949e),
                                    cx.listener(move |this, _, _, cx| {
                                        cx.stop_propagation();
                                        this.prompt_recording_path_for_session(
                                            RecordingPathPromptKind::SaveTranscript,
                                            save_session_id.clone(),
                                            save_session_name.clone(),
                                            cx,
                                        );
                                    }),
                                )),
                        ),
                );
            }
            if visible_count == 0 {
                session_rows = session_rows.child(
                    div()
                        .py_4()
                        .text_center()
                        .text_size(px(11.))
                        .text_color(rgb(0x6e7681))
                        .child("No matching sessions"),
                );
            }
        }

        let count_label = if query.is_empty() {
            session_count.to_string()
        } else {
            format!("{visible_count}/{session_count}")
        };
        let _ = recording_count;

        // Tauri RecordingPanel: PanelHeader(meta count) + search strip + dense session rows.
        // Shared stack already renders PanelHeader; body is search + list only.
        div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(rgb(0x161b22))
            .child(
                div()
                    .h(px(36.))
                    .flex_none()
                    .px_2()
                    .border_b_1()
                    .border_color(rgb(0x30363d))
                    .bg(rgb(0x161b22))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .relative()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .id(SharedString::from("recording-session-search"))
                                    .h(px(28.))
                                    .rounded_md()
                                    .bg(rgb(0x21262d))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .cursor_text()
                                    .track_focus(&self.recording_search_focus)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        window.focus(&this.recording_search_focus);
                                        cx.notify();
                                    }))
                                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                        cx.stop_propagation();
                                        this.handle_recording_search_key_down(event, cx);
                                    }))
                                    .child(
                                        svg()
                                            .size(px(14.))
                                            .flex_none()
                                            .path("icons/fe/search.svg")
                                            .text_color(rgb(0x8b949e)),
                                    )
                                    .child(
                                        div()
                                            .min_w_0()
                                            .flex_1()
                                            .text_size(px(12.))
                                            .text_color(if self.recording_search_draft.is_empty() {
                                                rgb(0x6e7681)
                                            } else {
                                                rgb(0xc9d1d9)
                                            })
                                            .child(if self.recording_search_draft.is_empty() {
                                                "Search sessions".to_string()
                                            } else {
                                                self.recording_search_draft.clone()
                                            }),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(0x6e7681))
                            .child(count_label),
                    ),
            )
            .child(
                div()
                    .id(SharedString::from("recording-session-list"))
                    .flex_1()
                    .min_h_0()
                    .overflow_scroll()
                    .scrollbar_width(px(6.))
                    .child(session_rows),
            )
    }

    fn recording_session_filter_query(&self) -> String {
        self.recording_search_draft.trim().to_ascii_lowercase()
    }
}

fn recording_action_svg_button(
    id: impl Into<String>,
    icon_path: &'static str,
    color: impl Into<gpui::Hsla>,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let color = color.into();
    div()
        .id(SharedString::from(id.into()))
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .text_color(color)
        .cursor_pointer()
        .hover(|this| this.bg(rgb(0x21262d)).text_color(rgb(0xc9d1d9)))
        .child(
            svg()
                .size(px(16.))
                .flex_none()
                .path(icon_path),
        )
        .on_click(on_click)
}

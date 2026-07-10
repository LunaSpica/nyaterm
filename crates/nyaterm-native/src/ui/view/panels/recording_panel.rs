use super::*;

impl NyaTermApp {
    pub(in crate::ui::view) fn recording_panel(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active_session_id = self.active_session_id.clone();
        let sessions = self.ordered_sessions();
        let recording_count = sessions
            .iter()
            .filter(|session| self.recording_manager.is_recording(&session.id))
            .count();
        let mut session_rows = div().mt_3().flex().flex_col().gap_1();
        if sessions.is_empty() {
            session_rows = session_rows.child(
                div()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(0x263142))
                    .bg(rgb(0x0d1320))
                    .p_3()
                    .text_xs()
                    .text_color(rgb(0x98a3b8))
                    .child("No active sessions."),
            );
        } else {
            for session in sessions.into_iter().take(8) {
                let session_id = session.id.clone();
                let start_session_id = session.id.clone();
                let save_session_id = session.id.clone();
                let select_session_id = session.id.clone();
                let session_name = self.session_display_name_by_info(&session);
                let start_session_name = session_name.clone();
                let save_session_name = session_name.clone();
                let is_current = active_session_id.as_deref() == Some(session.id.as_str());
                let session_is_recording = self.recording_manager.is_recording(&session.id);
                session_rows = session_rows.child(
                    div()
                        .id(SharedString::from(format!(
                            "recording-session-row-{session_id}"
                        )))
                        .rounded_sm()
                        .border_1()
                        .border_color(if is_current {
                            rgb(0x3b82f6)
                        } else {
                            rgb(0x263142)
                        })
                        .bg(if is_current {
                            rgb(0x10223b)
                        } else {
                            rgb(0x0d1320)
                        })
                        .p_2()
                        .flex()
                        .items_center()
                        .gap_2()
                        .cursor_pointer()
                        .hover(|this| this.bg(rgb(0x182235)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.select_session(select_session_id.clone(), cx);
                        }))
                        .child(
                            div()
                                .size(px(7.))
                                .rounded_full()
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
                                .gap_1()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .min_w_0()
                                                .text_xs()
                                                .font_weight(FontWeight(700.))
                                                .text_color(rgb(0xe5edf7))
                                                .child(truncate_preview(&session_name, 34)),
                                        )
                                        .child(status_pill(
                                            session_kind_label(session.kind),
                                            rgb(0x93c5fd),
                                            rgb(0x17233a),
                                        )),
                                )
                                .child(
                                    div()
                                        .font_family("JetBrains Mono")
                                        .text_size(px(10.))
                                        .text_color(rgb(0x8f98aa))
                                        .child(short_id(&session_id).to_string()),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(small_button(
                                    format!("recording-session-toggle-{session_id}"),
                                    if session_is_recording {
                                        "Stop"
                                    } else {
                                        "Start"
                                    },
                                    cx.listener(move |this, _, _, cx| {
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
                                .child(small_button(
                                    format!("recording-session-save-{session_id}"),
                                    "Save",
                                    cx.listener(move |this, _, _, cx| {
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
        }
        let search = self.recording_search_results();
        let mut rows = div().mt_3().flex().flex_col().gap_2();
        match search {
            Ok(response) if response.results.is_empty() => {
                rows = rows.child(
                    div()
                        .text_xs()
                        .text_color(rgb(0x98a3b8))
                        .line_height(px(18.))
                        .child(if self.recording_search_draft.trim().is_empty() {
                            "Type to search captured terminal history."
                        } else {
                            "No transcript matches."
                        }),
                );
            }
            Ok(response) => {
                rows = rows.child(div().text_xs().text_color(rgb(0x64748b)).child(format!(
                    "{} match(es) · {} ms",
                    response.total, response.elapsed_ms
                )));
                for result in response.results.into_iter().take(4) {
                    let meta = format!("{} · line {}", result.source, result.line_number);
                    rows = rows.child(
                        div()
                            .border_t_1()
                            .border_color(rgb(0x2a3140))
                            .pt_2()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .gap_2()
                                    .child(
                                        div()
                                            .min_w_0()
                                            .text_xs()
                                            .font_family("JetBrains Mono")
                                            .text_color(rgb(0xe5edf7))
                                            .child(truncate_preview(&result.preview, 120)),
                                    )
                                    .child(div().text_xs().text_color(rgb(0x64748b)).child(meta)),
                            )
                            .child(div().text_xs().text_color(rgb(0x98a3b8)).child(format!(
                                "context {} before / {} after",
                                result.before.len(),
                                result.after.len()
                            ))),
                    );
                }
            }
            Err(error) => {
                rows = rows.child(
                    div()
                        .text_xs()
                        .text_color(rgb(0xfca5a5))
                        .line_height(px(18.))
                        .child(format!("Search failed: {error}")),
                );
            }
        }

        div()
            .rounded_md()
            .border_1()
            .border_color(rgb(0x2a3140))
            .bg(rgb(0x151923))
            .p_4()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(FontWeight(700.))
                            .child("Recording"),
                    )
                    .child(status_pill(
                        if recording_count > 0 {
                            "recording"
                        } else {
                            "idle"
                        },
                        if recording_count > 0 {
                            rgb(0xfca5a5)
                        } else {
                            rgb(0x93c5fd)
                        },
                        if recording_count > 0 {
                            rgb(0x3a1717)
                        } else {
                            rgb(0x17233a)
                        },
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(small_button(
                        "recording-start",
                        "Start",
                        cx.listener(|this, _, _, cx| {
                            this.prompt_recording_path(RecordingPathPromptKind::Start, cx);
                        }),
                    ))
                    .child(small_button(
                        "recording-stop",
                        "Stop",
                        cx.listener(|this, _, _, cx| {
                            this.stop_active_recording(cx);
                        }),
                    ))
                    .child(small_button(
                        "recording-save-transcript",
                        "Save",
                        cx.listener(|this, _, _, cx| {
                            this.prompt_recording_path(RecordingPathPromptKind::SaveTranscript, cx);
                        }),
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .grid()
                    .grid_cols(3)
                    .gap_2()
                    .child(metric(
                        "Sessions",
                        self.session_manager
                            .list_sessions()
                            .map(|sessions| sessions.len().to_string())
                            .unwrap_or_else(|_| "0".to_string()),
                    ))
                    .child(metric("Recording", recording_count.to_string()))
                    .child(metric(
                        "Active",
                        active_session_id
                            .as_deref()
                            .map(short_id)
                            .unwrap_or("none")
                            .to_string(),
                    )),
            )
            .child(
                div()
                    .mt_3()
                    .text_xs()
                    .font_weight(FontWeight(700.))
                    .text_color(rgb(0x8f98aa))
                    .child("Session Recording"),
            )
            .child(session_rows)
            .child(
                transfer_input(
                    "recording-search-input",
                    "Transcript Search",
                    self.recording_search_draft.clone(),
                    true,
                )
                .mt_3()
                .track_focus(&self.recording_search_focus)
                .on_click(cx.listener(|this, _, window, cx| {
                    window.focus(&this.recording_search_focus);
                    cx.notify();
                }))
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                    cx.stop_propagation();
                    this.handle_recording_search_key_down(event, cx);
                })),
            )
            .child(rows)
    }
}

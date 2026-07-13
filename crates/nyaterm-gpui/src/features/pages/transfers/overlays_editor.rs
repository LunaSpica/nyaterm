use super::*;
use crate::features::{TransferExternalSyncPromptState, metric};
use gpui::rgba;
use std::path::PathBuf;

impl NyaTermApp {
    pub(in crate::features) fn transfer_external_sync_prompt_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let prompt =
            self.transfer_external_sync_prompt
                .clone()
                .unwrap_or(TransferExternalSyncPromptState {
                    job_id: String::new(),
                    remote_path: String::new(),
                    local_path: PathBuf::new(),
                });

        div()
            .id(SharedString::from("transfer-external-sync-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgba(0x020617dd))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.transfer_external_sync_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.transfer_external_sync_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                if event.keystroke.key.as_str() == "escape" {
                    this.ignore_pending_external_editor_sync(cx);
                }
            }))
            .child(
                div()
                    .w(px(520.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight(800.))
                                    .child("External File Modified"),
                            )
                            .child(status_pill(
                                "sync pending",
                                rgb(palette.warning),
                                rgb(0x3a2d10),
                            )),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(palette.text_muted))
                            .child("The externally opened file changed locally."),
                    )
                    .child(metric(
                        palette,
                        "Remote",
                        truncate_preview(&prompt.remote_path, 58),
                    ))
                    .child(metric(
                        palette,
                        "Local",
                        truncate_preview(&prompt.local_path.display().to_string(), 58),
                    ))
                    .child(
                        div()
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(small_button(
                                palette,
                                "transfer-external-sync-ignore",
                                "Ignore",
                                cx.listener(|this, _, _, cx| {
                                    this.ignore_pending_external_editor_sync(cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "transfer-external-sync-upload",
                                "Upload",
                                cx.listener(|this, _, _, cx| {
                                    this.upload_pending_external_editor_sync(false, cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "transfer-external-sync-always",
                                "Always Upload",
                                cx.listener(|this, _, _, cx| {
                                    this.upload_pending_external_editor_sync(true, cx);
                                }),
                            )),
                    ),
            )
    }

    pub(in crate::features) fn transfer_editor_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let state = self.transfer_editor.clone().unwrap_or(TransferEditorState {
            remote_path: String::new(),
            name: String::new(),
            content: String::new(),
            search_query: String::new(),
            active_match: 0,
            base_size: None,
            base_modified_at: None,
            loading: false,
            saving: false,
            dirty: false,
            conflict: false,
            close_confirm: false,
            close_after_save: false,
            reload_confirm: false,
            error: None,
            focused_field: TransferEditorField::Content,
        });
        let status = if state.loading {
            "loading"
        } else if state.saving {
            "saving"
        } else if state.conflict {
            "conflict"
        } else if state.dirty {
            "dirty"
        } else {
            "saved"
        };
        let line_count = state.content.lines().count().max(1);
        let byte_count = state.content.len();
        let search_matches = editor_search_matches(&state.content, &state.search_query);
        let active_match = state
            .active_match
            .min(search_matches.len().saturating_sub(1));
        let content_preview =
            editor_content_preview(&state.content, &state.search_query, active_match);
        let search_label = if state.search_query.is_empty() {
            "Search text".to_string()
        } else {
            state.search_query.clone()
        };

        div()
            .id(SharedString::from("transfer-editor-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(rgb(0x030508))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.transfer_editor_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.transfer_editor_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                cx.stop_propagation();
                this.handle_transfer_editor_key_down(event, window, cx);
            }))
            .child(
                div()
                    .id(SharedString::from("transfer-editor-dialog"))
                    .w(px(780.))
                    .h(px(620.))
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.bg))
                    .shadow_lg()
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .min_w_0()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(FontWeight(800.))
                                            .text_color(rgb(palette.text))
                                            .child(format!(
                                                "Remote Editor: {}",
                                                truncate_preview(&state.name, 52)
                                            )),
                                    )
                                    .child(
                                        div()
                                            .font_family("JetBrains Mono")
                                            .text_xs()
                                            .text_color(rgb(palette.text_muted))
                                            .child(truncate_preview(&state.remote_path, 96)),
                                    ),
                            )
                            .child(status_pill(status, rgb(palette.accent), rgb(palette.hover))),
                    )
                    .when_some(state.error.clone(), |this, error| {
                        this.child(
                            div()
                                .rounded_sm()
                                .bg(rgb(0x351216))
                                .px_3()
                                .py_2()
                                .text_xs()
                                .text_color(rgb(palette.danger))
                                .child(error),
                        )
                    })
                    .when(state.conflict, |this| {
                        this.child(
                            div()
                                .rounded_sm()
                                .bg(rgb(0x352912))
                                .px_3()
                                .py_2()
                                .text_xs()
                                .text_color(rgb(palette.warning))
                                .child("Remote file changed since it was loaded. Use Force Save to overwrite it."),
                        )
                    })
                    .when(state.close_confirm, |this| {
                        this.child(
                            div()
                                .rounded_sm()
                                .bg(rgb(0x352912))
                                .px_3()
                                .py_2()
                                .text_xs()
                                .text_color(rgb(palette.warning))
                                .child("Unsaved changes are still in memory. Save or discard them."),
                        )
                    })
                    .when(state.reload_confirm, |this| {
                        this.child(
                            div()
                                .rounded_sm()
                                .bg(rgb(0x352912))
                                .px_3()
                                .py_2()
                                .text_xs()
                                .text_color(rgb(palette.warning))
                                .child("Reload will discard unsaved changes. Press Reload again to continue."),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .id(SharedString::from("transfer-editor-search-input"))
                                    .h(px(32.))
                                    .flex_1()
                                    .min_w_0()
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(if state.focused_field == TransferEditorField::Search {
                                        rgb(0x256d3f)
                                    } else {
                                        rgb(palette.border)
                                    })
                                    .bg(rgb(palette.input))
                                    .px_3()
                                    .flex()
                                    .items_center()
                                    .font_family("JetBrains Mono")
                                    .text_xs()
                                    .text_color(if state.search_query.is_empty() {
                                        rgb(palette.text_muted)
                                    } else {
                                        rgb(palette.text)
                                    })
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        if let Some(state) = this.transfer_editor.as_mut() {
                                            state.focused_field = TransferEditorField::Search;
                                        }
                                        window.focus(&this.transfer_editor_focus);
                                        cx.notify();
                                    }))
                                    .child(truncate_preview(&search_label, 96)),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(palette.text_muted))
                                    .child(if state.search_query.is_empty() {
                                        "0 / 0".to_string()
                                    } else if search_matches.is_empty() {
                                        "no match".to_string()
                                    } else {
                                        format!("{} / {}", active_match + 1, search_matches.len())
                                    }),
                            )
                            .child(small_button(palette, 
                                "transfer-editor-prev-match",
                                "Prev",
                                cx.listener(|this, _, _, cx| {
                                    this.advance_transfer_editor_search(-1, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "transfer-editor-next-match",
                                "Next",
                                cx.listener(|this, _, _, cx| {
                                    this.advance_transfer_editor_search(1, cx);
                                }),
                            ))
                            .child(small_button(palette, 
                                "transfer-editor-clear-search",
                                "Clear",
                                cx.listener(|this, _, _, cx| {
                                    if let Some(state) = this.transfer_editor.as_mut() {
                                        state.search_query.clear();
                                        state.active_match = 0;
                                    }
                                    cx.notify();
                                }),
                            )),
                    )
                    .child(
                        div()
                            .id(SharedString::from("transfer-editor-content"))
                            .flex_1()
                            .min_h_0()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(palette.input))
                            .p_3()
                            .font_family("JetBrains Mono")
                            .text_xs()
                            .text_color(if state.loading {
                                rgb(palette.text_muted)
                            } else {
                                rgb(palette.text)
                            })
                            .overflow_hidden()
                            .child(if state.loading {
                                SharedString::from("Loading remote text file...")
                            } else if content_preview.is_empty() {
                                SharedString::from("")
                            } else {
                                SharedString::from(content_preview)
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(rgb(palette.text_muted))
                                    .child(format!(
                                        "{line_count} line(s) · {byte_count} byte(s) · Ctrl/Cmd+F search · Ctrl/Cmd+S saves"
                                    )),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(small_button(palette, 
                                        "transfer-editor-reload",
                                        if state.reload_confirm {
                                            "Confirm Reload"
                                        } else {
                                            "Reload"
                                        },
                                        cx.listener(|this, _, window, cx| {
                                            if let Some(state) = this.transfer_editor.as_mut() {
                                                if state.dirty && !state.reload_confirm {
                                                    state.reload_confirm = true;
                                                    state.close_confirm = false;
                                                    state.error = Some(
                                                        "Reload will discard unsaved changes."
                                                            .to_string(),
                                                    );
                                                    this.terminal_status = "confirm remote editor reload".to_string();
                                                    cx.notify();
                                                    return;
                                                }
                                                state.loading = true;
                                                state.error = None;
                                                state.conflict = false;
                                                state.close_confirm = false;
                                                state.reload_confirm = false;
                                                let remote_path = state.remote_path.clone();
                                                this.start_sftp_editor_load_job(remote_path, window, cx);
                                            }
                                        }),
                                    ))
                                    .when(state.reload_confirm, |this| {
                                        this.child(small_button(palette, 
                                            "transfer-editor-cancel-reload",
                                            "Cancel Reload",
                                            cx.listener(|this, _, _, cx| {
                                                this.cancel_transfer_editor_reload_confirm(cx);
                                            }),
                                        ))
                                    })
                                    .child(small_button(palette, 
                                        "transfer-editor-save",
                                        if state.saving { "Saving" } else { "Save" },
                                        cx.listener(|this, _, window, cx| {
                                            this.save_transfer_editor(false, window, cx);
                                        }),
                                    ))
                                    .child(small_button(palette, 
                                        "transfer-editor-force-save",
                                        "Force Save",
                                        cx.listener(|this, _, window, cx| {
                                            this.save_transfer_editor(true, window, cx);
                                        }),
                                    ))
                                    .child(small_button(palette, 
                                        "transfer-editor-close",
                                        "Close",
                                        cx.listener(|this, _, _, cx| {
                                            this.close_transfer_editor(cx);
                                        }),
                                    ))
                                    .when(state.close_confirm, |this| {
                                        this.child(small_button(palette, 
                                            "transfer-editor-save-close",
                                            if state.saving { "Saving" } else { "Save Close" },
                                            cx.listener(|this, _, window, cx| {
                                                this.save_transfer_editor_and_close(window, cx);
                                            }),
                                        ))
                                        .child(small_button(palette, 
                                            "transfer-editor-cancel-close",
                                            "Cancel Close",
                                            cx.listener(|this, _, _, cx| {
                                                this.cancel_transfer_editor_close_confirm(cx);
                                            }),
                                        ))
                                        .child(small_button(palette, 
                                            "transfer-editor-discard",
                                            "Discard",
                                            cx.listener(|this, _, _, cx| {
                                                this.discard_transfer_editor(cx);
                                            }),
                                        ))
                                    }),
                            ),
                    ),
            )
    }
}

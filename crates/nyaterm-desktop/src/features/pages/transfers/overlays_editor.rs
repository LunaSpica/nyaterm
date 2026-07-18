use super::*;
use gpui::rgba;

#[derive(Clone, Copy)]
enum ExternalSyncButtonStyle {
    Ghost,
    Outline,
    Primary,
}

fn external_sync_button(
    palette: crate::theme::ThemePalette,
    id: &'static str,
    label: &'static str,
    style: ExternalSyncButtonStyle,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let transparent = rgba(0x00000000);
    let (background, border, text) = match style {
        ExternalSyncButtonStyle::Ghost => (transparent, transparent, rgb(palette.text_muted)),
        ExternalSyncButtonStyle::Outline => (
            rgb(palette.bg).into(),
            rgb(palette.border).into(),
            rgb(palette.text),
        ),
        ExternalSyncButtonStyle::Primary => (
            rgb(palette.accent).into(),
            rgb(palette.accent).into(),
            rgb(palette.bg),
        ),
    };
    div()
        .id(SharedString::from(id))
        .h(px(28.))
        .px_3()
        .flex()
        .items_center()
        .rounded_sm()
        .border_1()
        .border_color(border)
        .bg(background)
        .text_color(text)
        .text_xs()
        .cursor_pointer()
        .hover(move |this| match style {
            ExternalSyncButtonStyle::Primary => this.bg(rgba((palette.accent << 8) | 0xe8)),
            ExternalSyncButtonStyle::Ghost | ExternalSyncButtonStyle::Outline => {
                this.bg(rgb(palette.hover)).text_color(rgb(palette.text))
            }
        })
        .child(label)
        .on_click(on_click)
}

impl NyaTermApp {
    pub(in crate::features) fn transfer_external_sync_prompt_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some((prompt_id, prompt)) = self.active_external_editor_sync_prompt() else {
            return div().into_any_element();
        };
        self.transfer_external_sync_prompt_surface(prompt_id, prompt, false, cx)
    }

    pub(in crate::features) fn transfer_external_sync_window_view(
        &mut self,
        prompt_id: String,
        prompt: TransferExternalSyncPromptState,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.transfer_external_sync_prompt_surface(prompt_id, prompt, true, cx)
    }

    fn transfer_external_sync_prompt_surface(
        &mut self,
        prompt_id: String,
        prompt: TransferExternalSyncPromptState,
        standalone: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let palette = self.theme_palette();
        let title = self.tr("fileExplorer.fileModified");
        let prompt_label = self.tr("fileExplorer.uploadPrompt");
        let cancel_label = self.tr("common.cancel");
        let always_label = self.tr("fileExplorer.alwaysUpload");
        let upload_once_label = self.tr("fileExplorer.uploadOnce");
        let ignore_prompt_id = prompt_id.clone();
        let always_prompt_id = prompt_id.clone();
        let upload_prompt_id = prompt_id.clone();

        div()
            .id(SharedString::from("transfer-external-sync-overlay"))
            .when(!standalone, |this| {
                this.absolute()
                    .top_0()
                    .bottom_0()
                    .left_0()
                    .right_0()
                    .bg(rgba(0x020617dd))
                    .p_3()
            })
            .when(standalone, |this| this.size_full().bg(rgb(palette.bg)))
            .flex()
            .items_center()
            .justify_center()
            .track_focus(&self.transfer_external_sync_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.transfer_external_sync_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                cx.stop_propagation();
                if event.keystroke.key.as_str() == "escape" {
                    this.ignore_external_editor_sync_prompt(&ignore_prompt_id, cx);
                }
            }))
            .child(
                div()
                    .w_full()
                    .when(standalone, |this| this.size_full())
                    .when(!standalone, |this| {
                        this.max_w(px(440.))
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .shadow_lg()
                    })
                    .bg(rgb(palette.bg))
                    .p_4()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(div().text_sm().font_weight(FontWeight(700.)).child(title))
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                div()
                                    .text_size(px(13.))
                                    .text_color(rgb(palette.text))
                                    .child(prompt_label),
                            )
                            .child(
                                div()
                                    .rounded_md()
                                    .border_1()
                                    .border_color(rgb(palette.border))
                                    .bg(rgb(palette.input))
                                    .px_3()
                                    .py_2()
                                    .font_family(crate::features::gpui_code_font_family())
                                    .text_xs()
                                    .text_color(rgb(palette.text_muted))
                                    .child(truncate_preview(&prompt.remote_path, 120)),
                            ),
                    )
                    .child(
                        div()
                            .pt_3()
                            .border_t_1()
                            .border_color(rgb(palette.border))
                            .flex()
                            .justify_end()
                            .gap_2()
                            .child(external_sync_button(
                                palette,
                                "transfer-external-sync-ignore",
                                cancel_label,
                                ExternalSyncButtonStyle::Ghost,
                                cx.listener(move |this, _, _, cx| {
                                    this.ignore_external_editor_sync_prompt(&prompt_id, cx);
                                }),
                            ))
                            .child(external_sync_button(
                                palette,
                                "transfer-external-sync-always",
                                always_label,
                                ExternalSyncButtonStyle::Outline,
                                cx.listener(move |this, _, _, cx| {
                                    this.upload_external_editor_sync_prompt(
                                        &always_prompt_id,
                                        true,
                                        cx,
                                    );
                                }),
                            ))
                            .child(external_sync_button(
                                palette,
                                "transfer-external-sync-upload",
                                upload_once_label,
                                ExternalSyncButtonStyle::Primary,
                                cx.listener(move |this, _, _, cx| {
                                    this.upload_external_editor_sync_prompt(
                                        &upload_prompt_id,
                                        false,
                                        cx,
                                    );
                                }),
                            )),
                    ),
            )
            .into_any_element()
    }

    pub(in crate::features) fn transfer_editor_overlay(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        self.transfer_editor_surface(false, cx)
    }

    pub(in crate::features) fn transfer_editor_window_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.transfer_editor_surface(true, cx).into_any_element()
    }

    fn transfer_editor_surface(
        &mut self,
        standalone: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let editor_title_label = self.tr("fileEditor.title");
        let loading_label = self.tr("common.loading");
        let saving_label = self.tr("common.saving");
        let save_label = self.tr("common.save");
        let cancel_label = self.tr("common.cancel");
        let saved_label = self.tr("fileEditor.saved");
        let unsaved_label = self.tr("fileEditor.unsaved");
        let conflict_label = self.tr("fileEditor.conflictTitle");
        let conflict_desc = self.tr("fileEditor.conflictDesc");
        let unsaved_desc = self.tr("fileEditor.unsavedDesc");
        let reload_dirty_desc = self.tr("fileEditor.reloadDirtyDesc");
        let reload_label = self.tr("fileEditor.reload");
        let confirm_reload_label = self.tr("fileEditor.discardAndReload");
        let open_external_label = self.tr("fileEditor.openExternal");
        let force_save_label = self.tr("fileEditor.forceSave");
        let save_all_label = self.tr("fileEditor.saveAll");
        let save_close_label = self.tr("fileEditor.saveAndClose");
        let discard_label = self.tr("fileEditor.discard");
        let search_placeholder = self.tr("fileEditor.searchPlaceholder");
        let previous_match_label = self.tr("fileEditor.previousMatch");
        let next_match_label = self.tr("fileEditor.nextMatch");
        let clear_search_label = self.tr("fileEditor.clearSearch");
        let no_match_label = self.tr("fileEditor.noMatch");
        let lines_label = self.tr("fileEditor.lines");
        let bytes_label = self.tr("fileEditor.bytes");
        let workspace = self.transfer_editor.clone();
        let state = workspace
            .as_ref()
            .and_then(TransferEditorWorkspaceState::active_tab)
            .cloned()
            .unwrap_or(TransferEditorState {
                id: String::new(),
                session_id: None,
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
                close_after_save: false,
                reload_confirm: false,
                error: None,
                focused_field: TransferEditorField::Content,
            });
        let close_confirm = workspace
            .as_ref()
            .is_some_and(|workspace| workspace.close_confirm);
        let tabs = workspace
            .as_ref()
            .map(|workspace| workspace.tabs.clone())
            .unwrap_or_default();
        let status = if state.loading {
            loading_label
        } else if state.saving {
            saving_label
        } else if state.conflict {
            conflict_label
        } else if state.dirty {
            unsaved_label
        } else {
            saved_label
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
            search_placeholder.to_string()
        } else {
            state.search_query.clone()
        };
        let active_tab_id = state.id.clone();
        let mut tab_strip = div()
            .id("transfer-editor-tabs")
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .h(px(40.))
            .flex_none()
            .flex()
            .overflow_x_scroll()
            .border_b_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface));
        for (index, tab) in tabs.iter().enumerate() {
            let tab_id = tab.id.clone();
            let close_tab_id = tab.id.clone();
            let active = tab.id == active_tab_id;
            let base_label = if tab.name.trim().is_empty() {
                tab.remote_path
                    .rsplit('/')
                    .next()
                    .filter(|name| !name.is_empty())
                    .unwrap_or(tab.remote_path.as_str())
            } else {
                tab.name.as_str()
            };
            let duplicate_name = tabs.iter().filter(|other| other.name == tab.name).count() > 1;
            let label = if duplicate_name {
                let parent = tab
                    .remote_path
                    .rsplit_once('/')
                    .map(|(parent, _)| parent.rsplit('/').next().unwrap_or(parent))
                    .filter(|parent| !parent.is_empty())
                    .unwrap_or("/");
                format!("{base_label} · {parent}")
            } else {
                base_label.to_string()
            };
            tab_strip = tab_strip.child(
                div()
                    .id(SharedString::from(format!("transfer-editor-tab-{index}")))
                    .h_full()
                    .min_w(px(96.))
                    .max_w(px(240.))
                    .px_3()
                    .relative()
                    .flex()
                    .items_center()
                    .gap_2()
                    .border_r_1()
                    .border_color(rgb(palette.border))
                    .bg(if active {
                        rgb(palette.bg)
                    } else {
                        rgb(palette.surface)
                    })
                    .text_color(if active {
                        rgb(palette.text)
                    } else {
                        rgb(palette.text_muted)
                    })
                    .cursor_pointer()
                    .hover(|this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.activate_transfer_editor_tab(&tab_id, cx);
                    }))
                    .when(active, |this| {
                        this.child(
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .right_0()
                                .h(px(1.))
                                .bg(rgb(palette.accent)),
                        )
                    })
                    .when(tab.dirty, |this| {
                        this.child(
                            div()
                                .size(px(6.))
                                .flex_none()
                                .rounded_full()
                                .bg(rgb(palette.accent)),
                        )
                    })
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .overflow_hidden()
                            .font_family(crate::features::gpui_code_font_family())
                            .text_xs()
                            .child(truncate_preview(&label, 28)),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!(
                                "transfer-editor-tab-close-{index}"
                            )))
                            .size(px(20.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded_sm()
                            .text_size(px(14.))
                            .text_color(rgb(palette.text_muted))
                            .hover(|this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.close_transfer_editor_tab(&close_tab_id, cx);
                            }))
                            .child("×"),
                    ),
            );
        }

        div()
            .id(SharedString::from("transfer-editor-overlay"))
            .absolute()
            .top_0()
            .bottom_0()
            .left_0()
            .right_0()
            .bg(if standalone {
                rgb(palette.bg)
            } else {
                rgb(0x030508)
            })
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
                    .when(standalone, |this| this.size_full())
                    .when(!standalone, |this| {
                        this.w(px(780.))
                            .h(px(620.))
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .shadow_lg()
                    })
                    .bg(rgb(palette.bg))
                    .relative()
                    .p_4()
                    .pt(px(56.))
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(tab_strip)
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
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
                                                "{editor_title_label}: {}",
                                                truncate_preview(&state.name, 52)
                                            )),
                                    )
                                    .child(
                                        div()
                                            .font_family(crate::features::gpui_code_font_family())
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
                                .child(conflict_desc),
                        )
                    })
                    .when(close_confirm, |this| {
                        this.child(
                            div()
                                .rounded_sm()
                                .bg(rgb(0x352912))
                                .px_3()
                                .py_2()
                                .text_xs()
                                .text_color(rgb(palette.warning))
                                .child(unsaved_desc),
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
                                .child(reload_dirty_desc),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .id(SharedString::from("transfer-editor-search-input"))
                                    .h(px(32.))
                                    .flex_1()
                                    .min_w(px(180.))
                                    .rounded_sm()
                                    .border_1()
                                    .border_color(
                                        if state.focused_field == TransferEditorField::Search {
                                            rgb(0x256d3f)
                                        } else {
                                            rgb(palette.border)
                                        },
                                    )
                                    .bg(rgb(palette.input))
                                    .px_3()
                                    .flex()
                                    .items_center()
                                    .font_family(crate::features::gpui_code_font_family())
                                    .text_xs()
                                    .text_color(if state.search_query.is_empty() {
                                        rgb(palette.text_muted)
                                    } else {
                                        rgb(palette.text)
                                    })
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        if let Some(state) = this.active_transfer_editor_tab_mut() {
                                            state.focused_field = TransferEditorField::Search;
                                        }
                                        window.focus(&this.transfer_editor_focus);
                                        cx.notify();
                                    }))
                                    .child(truncate_preview(&search_label, 96)),
                            )
                            .child(div().text_xs().text_color(rgb(palette.text_muted)).child(
                                if state.search_query.is_empty() {
                                    "0 / 0".to_string()
                                } else if search_matches.is_empty() {
                                    no_match_label.to_string()
                                } else {
                                    format!("{} / {}", active_match + 1, search_matches.len())
                                },
                            ))
                            .child(small_button(
                                palette,
                                "transfer-editor-prev-match",
                                previous_match_label,
                                cx.listener(|this, _, _, cx| {
                                    this.advance_transfer_editor_search(-1, cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "transfer-editor-next-match",
                                next_match_label,
                                cx.listener(|this, _, _, cx| {
                                    this.advance_transfer_editor_search(1, cx);
                                }),
                            ))
                            .child(small_button(
                                palette,
                                "transfer-editor-clear-search",
                                clear_search_label,
                                cx.listener(|this, _, _, cx| {
                                    if let Some(state) = this.active_transfer_editor_tab_mut() {
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
                            .font_family(crate::features::gpui_code_font_family())
                            .text_xs()
                            .text_color(if state.loading {
                                rgb(palette.text_muted)
                            } else {
                                rgb(palette.text)
                            })
                            .overflow_hidden()
                            .child(if state.loading {
                                SharedString::from(loading_label)
                            } else if content_preview.is_empty() {
                                SharedString::from("")
                            } else {
                                SharedString::from(content_preview)
                            }),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .child(div().text_xs().text_color(rgb(palette.text_muted)).child(
                                format!("{line_count} {lines_label} · {byte_count} {bytes_label}"),
                            ))
                            .child(
                                div()
                                    .flex()
                                    .flex_1()
                                    .min_w(px(300.))
                                    .flex_wrap()
                                    .items_center()
                                    .justify_end()
                                    .gap_2()
                                    .child(small_button(
                                        palette,
                                        "transfer-editor-reload",
                                        if state.reload_confirm {
                                            confirm_reload_label
                                        } else {
                                            reload_label
                                        },
                                        cx.listener(|this, _, window, cx| {
                                            if let Some(state) =
                                                this.active_transfer_editor_tab_mut()
                                            {
                                                if state.dirty && !state.reload_confirm {
                                                    state.reload_confirm = true;
                                                    state.error =
                                                        Some(reload_dirty_desc.to_string());
                                                    this.terminal_status =
                                                        "confirm remote editor reload".to_string();
                                                    cx.notify();
                                                    return;
                                                }
                                                state.loading = true;
                                                state.error = None;
                                                state.conflict = false;
                                                state.reload_confirm = false;
                                                let session_id = state.session_id.clone();
                                                let remote_path = state.remote_path.clone();
                                                this.start_sftp_editor_load_job(
                                                    session_id,
                                                    remote_path,
                                                    window,
                                                    cx,
                                                );
                                            }
                                        }),
                                    ))
                                    .when(state.reload_confirm, |this| {
                                        this.child(small_button(
                                            palette,
                                            "transfer-editor-cancel-reload",
                                            cancel_label,
                                            cx.listener(|this, _, _, cx| {
                                                this.cancel_transfer_editor_reload_confirm(cx);
                                            }),
                                        ))
                                    })
                                    .child(small_button(
                                        palette,
                                        "transfer-editor-open-external",
                                        open_external_label,
                                        cx.listener(|this, _, _, cx| {
                                            this.open_active_transfer_editor_external(cx);
                                        }),
                                    ))
                                    .child(small_button(
                                        palette,
                                        "transfer-editor-save",
                                        if state.saving {
                                            saving_label
                                        } else {
                                            save_label
                                        },
                                        cx.listener(|this, _, window, cx| {
                                            this.save_transfer_editor(false, window, cx);
                                        }),
                                    ))
                                    .when(tabs.len() > 1, |this| {
                                        this.child(small_button(
                                            palette,
                                            "transfer-editor-save-all",
                                            save_all_label,
                                            cx.listener(|this, _, window, cx| {
                                                this.save_all_transfer_editor_tabs(window, cx);
                                            }),
                                        ))
                                    })
                                    .when(state.conflict, |this| {
                                        this.child(small_button(
                                            palette,
                                            "transfer-editor-force-save",
                                            force_save_label,
                                            cx.listener(|this, _, window, cx| {
                                                this.save_transfer_editor(true, window, cx);
                                            }),
                                        ))
                                    })
                                    .when(close_confirm, |this| {
                                        this.child(small_button(
                                            palette,
                                            "transfer-editor-save-close",
                                            if state.saving {
                                                saving_label
                                            } else {
                                                save_close_label
                                            },
                                            cx.listener(|this, _, window, cx| {
                                                this.save_transfer_editor_and_close(window, cx);
                                            }),
                                        ))
                                        .child(small_button(
                                            palette,
                                            "transfer-editor-cancel-close",
                                            cancel_label,
                                            cx.listener(|this, _, _, cx| {
                                                this.cancel_transfer_editor_close_confirm(cx);
                                            }),
                                        ))
                                        .child(
                                            small_button(
                                                palette,
                                                "transfer-editor-discard",
                                                discard_label,
                                                cx.listener(|this, _, _, cx| {
                                                    this.discard_transfer_editor(cx);
                                                }),
                                            ),
                                        )
                                    }),
                            ),
                    ),
            )
    }
}

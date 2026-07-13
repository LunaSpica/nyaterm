use super::*;

impl NyaTermApp {
    pub(in crate::features) fn keybindings_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        // Tauri KeyboardShortcutsTab: search + section per category + dense shortcut rows.
        let supported = SHORTCUT_REGISTRY
            .iter()
            .filter(|shortcut| shortcut.native_status == ShortcutNativeStatus::Supported)
            .count();
        let pending = SHORTCUT_REGISTRY
            .iter()
            .filter(|shortcut| shortcut.native_status == ShortcutNativeStatus::Pending)
            .count();
        let overrides = self.settings.keybindings.len();
        let search = self.keybinding_search_draft.clone();
        let search_display = if search.is_empty() {
            "Search shortcuts…".to_string()
        } else {
            search.clone()
        };
        let mut groups = div().flex().flex_col().gap_3();
        for category in SHORTCUT_CATEGORIES {
            groups = groups.child(self.shortcut_category_group(category, &search, cx));
        }

        div()
            .id("settings-keybindings-panel")
            .flex()
            .flex_col()
            .gap_3()
            .track_focus(&self.keybindings_focus)
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.keybindings_focus);
                cx.notify();
            }))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                this.handle_keybinding_key_down(event, cx);
            }))
            .child(settings_form_section(palette,
                Some("Keyboard shortcuts"),
                Some("Record overrides stored in the same keybindings map as the Tauri app."),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(palette,
                        "Registry",
                        Some(SharedString::from(format!(
                            "{} total · {supported} native · {pending} pending · {overrides} overrides",
                            SHORTCUT_REGISTRY.len()
                        ))),
                        if overrides > 0 {
                            small_button(palette,
                                "keybindings-reset-all",
                                "Reset All",
                                cx.listener(|this, _, _, cx| {
                                    this.reset_all_keybindings(cx);
                                }),
                            )
                            .into_any_element()
                        } else {
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette.text_muted))
                                .child("Defaults")
                                .into_any_element()
                        },
                    ))
                    .child(settings_form_row(
                        palette,
                        "Search",
                        Some(SharedString::from(
                            "Filter by action label, shortcut id, or key chords.",
                        )),
                        div()
                            .id("settings-keybindings-search")
                            .min_w(px(220.))
                            .h(px(28.))
                            .px_2()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .bg(rgb(palette.input))
                            .flex()
                            .items_center()
                            .text_size(px(12.))
                            .text_color(if search.is_empty() {
                                rgb(palette.text_dimmed)
                            } else {
                                rgb(palette.text)
                            })
                            .track_focus(&self.keybinding_search_focus)
                            .cursor_pointer()
                            .child(search_display)
                            .on_click(cx.listener(|this, _, window, cx| {
                                window.focus(&this.keybinding_search_focus);
                                cx.notify();
                            }))
                            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                                this.handle_keybinding_search_key_down(event, cx);
                            })),
                    ))
                    .child(
                        div()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(palette.surface_elevated))
                            .bg(rgb(palette.bg))
                            .px_3()
                            .py_2()
                            .text_size(px(11.))
                            .line_height(px(16.))
                            .text_color(rgb(palette.text_muted))
                            .child(
                                "Press Record, type a shortcut, then Save or Enter. Esc cancels recording. Conflicts block save.",
                            ),
                    ),
            ))
            .child(groups)
    }

    pub(in crate::features) fn shortcut_category_group(
        &mut self,
        category: ShortcutCategory,
        search: &str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let needle = search.trim().to_ascii_lowercase();
        let shortcuts = SHORTCUT_REGISTRY
            .iter()
            .filter(|shortcut| shortcut.category == category)
            .filter(|shortcut| {
                if needle.is_empty() {
                    return true;
                }
                let keys = shortcut_keys_for(shortcut.id, &self.settings.keybindings)
                    .unwrap_or_else(|| shortcut.default_keys.to_string());
                let display = format_hotkey_for_display(&keys).to_ascii_lowercase();
                shortcut.label.to_ascii_lowercase().contains(&needle)
                    || shortcut.id.to_ascii_lowercase().contains(&needle)
                    || display.contains(&needle)
                    || keys.to_ascii_lowercase().contains(&needle)
            })
            .collect::<Vec<_>>();
        let count = shortcuts.len();
        if !needle.is_empty() && count == 0 {
            return div().into_any_element();
        }
        let mut rows = div().flex().flex_col().gap_1();
        for shortcut in shortcuts {
            rows = rows.child(self.shortcut_registry_row(shortcut, cx));
        }

        settings_form_section(
            palette,
            Some(category.label()),
            None,
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(settings_form_row(
                    palette,
                    "Shortcuts",
                    Some(SharedString::from(format!("{count} in category"))),
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette.text_muted))
                        .child("Native"),
                ))
                .child(rows),
        )
        .into_any_element()
    }

    pub(in crate::features) fn shortcut_registry_row(
        &mut self,
        shortcut: &'static ShortcutDefinition,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let (badge_fg, badge_bg) = match shortcut.native_status {
            ShortcutNativeStatus::Supported => (rgb(palette.success), rgb(0x12261c)),
            ShortcutNativeStatus::Partial => (rgb(palette.accent), rgb(palette.hover)),
            ShortcutNativeStatus::Pending => (rgb(palette.danger), rgb(0x2d1215)),
            ShortcutNativeStatus::Contextual => (rgb(palette.warning), rgb(0x2a2111)),
        };
        let is_custom = self.settings.keybindings.contains_key(shortcut.id);
        let is_recording = self.keybinding_recording_id.as_deref() == Some(shortcut.id);
        let effective_keys = shortcut_keys_for(shortcut.id, &self.settings.keybindings)
            .unwrap_or_else(|| shortcut.default_keys.to_string());
        let conflict = if is_recording {
            self.keybinding_pending_keys
                .as_deref()
                .and_then(|keys| self.keybinding_conflict_label(keys, shortcut.id))
        } else {
            None
        };
        let key_display = if is_recording {
            self.keybinding_pending_keys
                .as_deref()
                .map(format_hotkey_for_display)
                .unwrap_or_else(|| "Press keys...".to_string())
        } else {
            format_hotkey_for_display(&effective_keys)
        };
        let shortcut_id = shortcut.id.to_string();
        let reset_shortcut_id = shortcut.id.to_string();
        let is_switch_to = shortcut.id == "tab.switchTo";

        div()
            .rounded_md()
            .px_2()
            .py_1()
            .border_1()
            .border_color(if is_recording {
                rgb(0x1f6feb)
            } else {
                rgb(palette.surface_elevated)
            })
            .bg(if is_recording {
                rgb(palette.hover)
            } else {
                rgb(palette.bg)
            })
            .flex()
            .flex_wrap()
            .items_center()
            .gap_2()
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
                                    .text_size(px(12.))
                                    .font_weight(FontWeight(600.))
                                    .text_color(rgb(palette.text))
                                    .overflow_hidden()
                                    .child(shortcut.label),
                            )
                            .when(is_custom, |this| {
                                this.child(
                                    div()
                                        .text_size(px(10.))
                                        .font_weight(FontWeight(600.))
                                        .text_color(rgb(0xbc8cff))
                                        .child("custom"),
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(10.))
                            .text_color(rgb(palette.text_dimmed))
                            .overflow_hidden()
                            .child(format!("{} · {}", shortcut.id, shortcut.note)),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .rounded_md()
                            .border_1()
                            .border_color(if conflict.is_some() {
                                rgb(palette.danger)
                            } else if is_recording {
                                rgb(0x388bfd)
                            } else {
                                rgb(palette.border)
                            })
                            .bg(rgb(palette.surface))
                            .px_2()
                            .py_0()
                            .h(px(24.))
                            .flex()
                            .items_center()
                            .font_family("JetBrains Mono")
                            .text_size(px(10.))
                            .font_weight(FontWeight(700.))
                            .text_color(if conflict.is_some() {
                                rgb(palette.danger)
                            } else if is_recording {
                                rgb(palette.accent)
                            } else {
                                rgb(palette.text)
                            })
                            .child(key_display),
                    )
                    .when_some(conflict.clone(), |this, name| {
                        this.child(
                            div()
                                .text_size(px(10.))
                                .text_color(rgb(palette.danger))
                                .child(format!("conflicts: {name}")),
                        )
                    }),
            )
            .child(status_pill(
                shortcut.native_status.label(),
                badge_fg,
                badge_bg,
            ))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .when(is_recording, |this| {
                        this.child(small_button(palette,
                            format!("keybinding-save-{}", shortcut.id),
                            "Save",
                            cx.listener(|this, _, _, cx| {
                                this.confirm_keybinding_recording(cx);
                            }),
                        ))
                        .child(small_button(palette,
                            format!("keybinding-cancel-{}", shortcut.id),
                            "Cancel",
                            cx.listener(|this, _, _, cx| {
                                this.cancel_keybinding_recording(cx);
                            }),
                        ))
                    })
                    .when(!is_recording, |this| {
                        this.child(small_button(palette,
                            format!("keybinding-record-{}", shortcut.id),
                            "Record",
                            cx.listener(move |this, _, window, cx| {
                                this.start_keybinding_recording(shortcut_id.clone(), window, cx);
                            }),
                        ))
                    })
                    .when(is_custom && !is_recording, |this| {
                        this.child(small_button(palette,
                            format!("keybinding-reset-{}", shortcut.id),
                            "Reset",
                            cx.listener(move |this, _, _, cx| {
                                this.reset_keybinding(reset_shortcut_id.clone(), cx);
                            }),
                        ))
                    }),
            )
            .when(is_recording && is_switch_to, |this| {
                this.child(
                    div()
                        .w_full()
                        .text_size(px(10.))
                        .text_color(rgb(palette.text_muted))
                        .child(
                            "Tab switch template must end with number 1 (e.g. ctrl+1). Other digits fill 2–9.",
                        ),
                )
            })
    }
}

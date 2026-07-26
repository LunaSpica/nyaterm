use gpui::{
    AnyElement, App, ClickEvent, Context, FontWeight, IntoElement, KeyDownEvent, SharedString,
    Window, div, prelude::*, px, rgb, rgba, svg,
};
use nyaterm_core::{CloudSyncSettings, RiskLevel};
use nyaterm_transport::SftpDuplicatePolicy;

use crate::models::{
    CloudSyncConflictState, CloudSyncInputField, SearchEngineEditorField, SettingsTab,
    SnapshotPasswordPromptKind, SnapshotPasswordPromptState, TranslateInputField,
};
use crate::theme::ThemePalette;
use crate::widgets::{small_button, status_pill};

use super::super::{
    ChromeTooltip, NyaTermApp, TAB_MOUSE_ACTIONS, TabMouseActionTarget, cloud_secret_display,
    compact_id, configured_cloud_sync_provider, format_history_timestamp_ms, none_if_blank,
    transfer_input, truncate_preview,
};

mod ai;
mod security;
mod sync_backup;
mod terminal;
mod transfer;
mod translation;
mod workspace;

impl NyaTermApp {
    pub(in crate::features) fn settings_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        self.settings_surface(self.last_viewport_size.0, false, cx)
    }

    pub(in crate::features) fn settings_window_view(
        &mut self,
        viewport_width: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        self.settings_surface(viewport_width, true, cx)
            .into_any_element()
    }

    fn settings_surface(
        &mut self,
        viewport_width: f32,
        native_window: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let backup_snapshot_prompt =
            self.active_snapshot_password_prompt
                .clone()
                .filter(|prompt| {
                    matches!(
                        prompt.kind,
                        SnapshotPasswordPromptKind::Export | SnapshotPasswordPromptKind::Import
                    )
                });
        self.settings_shell(backup_snapshot_prompt, viewport_width, native_window, cx)
    }

    pub(in crate::features) fn settings_shell(
        &mut self,
        backup_snapshot_prompt: Option<SnapshotPasswordPromptState>,
        viewport_width: f32,
        native_window: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Tauri SettingsPage shell: compact header + narrow nav + scroll content.
        let palette = self.theme_palette();
        let settings_title = self.tr("settings.title");
        let active_group = self.tr(self.settings_active_tab.group_i18n_key());
        let active_label = self.tr(self.settings_active_tab.i18n_key());
        let back_label = self.tr("common.close");
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(palette.bg))
            .when(!native_window, |this| {
                this.child(
                    div()
                        .h(px(36.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_3()
                        .border_b_1()
                        .border_color(rgb(palette.border))
                        .bg(rgb(palette.section_header))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_size(px(12.))
                                        .font_weight(FontWeight(700.))
                                        .text_color(rgb(palette.text))
                                        .child(settings_title),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text_dimmed))
                                        .child("·"),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text_muted))
                                        .child(active_group),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .text_color(rgb(palette.text_dimmed))
                                        .child("/"),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.))
                                        .font_weight(FontWeight(600.))
                                        .text_color(rgb(palette.text))
                                        .child(active_label),
                                ),
                        )
                        .child(
                            div()
                                .id(SharedString::from("settings-close"))
                                .h(px(26.))
                                .px_2()
                                .flex()
                                .items_center()
                                .rounded_md()
                                .text_size(px(11.))
                                .text_color(rgb(palette.text_muted))
                                .cursor_pointer()
                                .hover(move |this| {
                                    this.bg(rgb(palette.surface_elevated))
                                        .text_color(rgb(palette.text))
                                })
                                .child(back_label)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.close_settings(cx);
                                })),
                        ),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h(px(0.))
                    .size_full()
                    .child(self.settings_sidebar(viewport_width, cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .overflow_hidden()
                            .bg(rgb(palette.bg))
                            .child(self.settings_active_panel(viewport_width, cx)),
                    ),
            )
            .when_some(backup_snapshot_prompt, |this, prompt| {
                this.child(
                    div()
                        .flex_none()
                        .px_4()
                        .child(self.snapshot_password_prompt_banner(prompt, cx)),
                )
            })
            .child(self.settings_action_footer(cx))
    }

    fn settings_action_footer(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        let dirty = self.settings_draft_dirty();
        let validation_error = dirty.then(|| self.pending_settings_cloud_error()).flatten();
        let apply_disabled = !dirty || validation_error.is_some();
        let confirm_disabled = validation_error.is_some();
        let status = validation_error.clone().unwrap_or_else(|| {
            self.tr(if dirty {
                "fileEditor.unsavedDesc"
            } else {
                "updater.noUpdate"
            })
            .to_string()
        });
        let cancel_label = self.tr("common.cancel");
        let apply_label = self.tr("common.apply");
        let confirm_label = self.tr("common.confirm");

        div()
            .h(px(48.))
            .flex_none()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .px_3()
            .border_t_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.section_header))
            .child(
                div()
                    .min_w_0()
                    .text_size(px(11.))
                    .text_color(if validation_error.is_some() {
                        rgb(palette.warning)
                    } else {
                        rgb(palette.text_muted)
                    })
                    .child(status),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("settings-cancel")
                            .h(px(28.))
                            .px_3()
                            .flex()
                            .items_center()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .text_size(px(11.))
                            .text_color(rgb(palette.text))
                            .cursor_pointer()
                            .hover(move |this| this.bg(rgb(palette.hover)))
                            .child(cancel_label)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.cancel_settings(cx);
                            })),
                    )
                    .child(
                        div()
                            .id("settings-apply")
                            .h(px(28.))
                            .px_3()
                            .flex()
                            .items_center()
                            .rounded_md()
                            .border_1()
                            .border_color(rgb(palette.border))
                            .text_size(px(11.))
                            .text_color(if apply_disabled {
                                rgb(palette.text_dimmed)
                            } else {
                                rgb(palette.text)
                            })
                            .when(!apply_disabled, |this| {
                                this.cursor_pointer()
                                    .hover(move |this| this.bg(rgb(palette.hover)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.apply_settings_draft(false, cx);
                                    }))
                            })
                            .child(apply_label),
                    )
                    .child(
                        div()
                            .id("settings-confirm")
                            .h(px(28.))
                            .px_3()
                            .flex()
                            .items_center()
                            .rounded_md()
                            .bg(if confirm_disabled {
                                rgb(palette.surface_elevated)
                            } else {
                                rgb(palette.link)
                            })
                            .text_size(px(11.))
                            .font_weight(FontWeight(600.))
                            .text_color(if confirm_disabled {
                                rgb(palette.text_dimmed)
                            } else {
                                rgb(0xffffff)
                            })
                            .when(!confirm_disabled, |this| {
                                this.cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.confirm_settings_draft(cx);
                                    }))
                            })
                            .child(confirm_label),
                    ),
            )
    }

    fn settings_sidebar(
        &mut self,
        viewport_width: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let compact = viewport_width < 640.;
        let sidebar_width = if compact {
            56.
        } else if viewport_width < 1024. {
            192.
        } else {
            224.
        };
        let workspace_expanded = self.settings_expanded_groups.contains("workspace");
        let terminal_expanded = self.settings_expanded_groups.contains("terminal_session");
        let ai_expanded = self.settings_expanded_groups.contains("ai_group");

        let mut sidebar_nav = div()
            .id(SharedString::from("settings-sidebar-scroll"))
            .flex_1()
            .min_h_0()
            .when(compact, |this| this.px_2().py_3())
            .when(!compact, |this| this.px_3().py_3())
            .overflow_scroll();

        sidebar_nav = sidebar_nav
            .child(self.settings_group_header(
                "workspace",
                self.tr("settings.groupWorkspace"),
                "icons/settings.svg",
                workspace_expanded,
                palette.link,
                compact,
                cx,
            ))
            .when(workspace_expanded, |this| {
                this.child(self.settings_tab_button(
                    SettingsTab::General,
                    "settings-tab-general",
                    compact,
                    true,
                    cx,
                ))
                .child(self.settings_tab_button(
                    SettingsTab::Appearance,
                    "settings-tab-appearance",
                    compact,
                    true,
                    cx,
                ))
                .child(self.settings_tab_button(
                    SettingsTab::Interaction,
                    "settings-tab-interaction",
                    compact,
                    true,
                    cx,
                ))
                .child(self.settings_tab_button(
                    SettingsTab::Keybindings,
                    "settings-tab-keybindings",
                    compact,
                    true,
                    cx,
                ))
            })
            .child(self.settings_group_header(
                "terminal_session",
                self.tr("settings.groupTerminalSession"),
                "icons/conn/terminal.svg",
                terminal_expanded,
                palette.success,
                compact,
                cx,
            ))
            .when(terminal_expanded, |this| {
                this.child(self.settings_tab_button(
                    SettingsTab::TerminalGeneral,
                    "settings-tab-terminal-general",
                    compact,
                    true,
                    cx,
                ))
                .child(self.settings_tab_button(
                    SettingsTab::Search,
                    "settings-tab-search",
                    compact,
                    true,
                    cx,
                ))
                .child(self.settings_tab_button(
                    SettingsTab::Translation,
                    "settings-tab-translation",
                    compact,
                    true,
                    cx,
                ))
            })
            .child(self.settings_group_header(
                "ai_group",
                self.tr("ai.title"),
                "icons/ai.svg",
                ai_expanded,
                0xbc8cff,
                compact,
                cx,
            ))
            .when(ai_expanded, |this| {
                this.child(self.settings_tab_button(
                    SettingsTab::AiGeneral,
                    "settings-tab-ai-general",
                    compact,
                    true,
                    cx,
                ))
                .child(self.settings_tab_button(
                    SettingsTab::AiModels,
                    "settings-tab-ai-models",
                    compact,
                    true,
                    cx,
                ))
                .child(self.settings_tab_button(
                    SettingsTab::AiRules,
                    "settings-tab-ai-rules",
                    compact,
                    true,
                    cx,
                ))
            })
            .child(self.settings_tab_button(
                SettingsTab::Transfer,
                "settings-tab-transfer",
                compact,
                false,
                cx,
            ))
            .child(self.settings_tab_button(
                SettingsTab::Security,
                "settings-tab-security",
                compact,
                false,
                cx,
            ))
            .child(self.settings_tab_button(
                SettingsTab::SyncBackup,
                "settings-tab-sync-backup",
                compact,
                false,
                cx,
            ));

        div()
            .w(px(sidebar_width))
            .flex_none()
            .h_full()
            .flex()
            .flex_col()
            .border_r_1()
            .border_color(rgba((palette.border << 8) | 0xb3))
            .bg(rgba((palette.surface_elevated << 8) | 0x33))
            .child(
                div()
                    .h(px(64.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap_3()
                    .when(compact, |this| this.justify_center())
                    .when(!compact, |this| this.px_3())
                    .border_b_1()
                    .border_color(rgba((palette.border << 8) | 0xb3))
                    .child(
                        svg()
                            .size(px(if compact { 22. } else { 24. }))
                            .flex_none()
                            .path("icons/settings.svg")
                            .text_color(rgb(palette.primary)),
                    )
                    .when(!compact, |this| {
                        this.child(
                            div()
                                .min_w_0()
                                .overflow_hidden()
                                .text_size(px(16.))
                                .font_weight(FontWeight(700.))
                                .text_color(rgb(palette.text))
                                .child(self.tr("settings.title")),
                        )
                    }),
            )
            .child(sidebar_nav)
    }

    fn settings_group_header(
        &self,
        group: &'static str,
        title: &'static str,
        icon_path: &'static str,
        expanded: bool,
        accent: u32,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        div()
            .id(SharedString::from(format!("settings-group-{group}")))
            .mt_1()
            .mb_1()
            .h(px(40.))
            .when(!compact, |this| this.px_3())
            .flex()
            .items_center()
            .when(compact, |this| this.justify_center())
            .when(!compact, |this| this.justify_between())
            .rounded_lg()
            .cursor_pointer()
            .hover(|this| this.bg(rgb(palette.hover)))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_size(px(14.))
                    .font_weight(FontWeight(600.))
                    .text_color(rgb(palette.text_muted))
                    .child(svg().size(px(18.)).path(icon_path).text_color(rgb(accent)))
                    .when(!compact, |this| this.child(title)),
            )
            .when(!compact, |this| {
                this.child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette.text_dimmed))
                        .child(
                            svg()
                                .size(px(18.))
                                .flex_none()
                                .path(if expanded {
                                    "icons/chevron-down.svg"
                                } else {
                                    "icons/fe/forward.svg"
                                })
                                .text_color(rgb(palette.text_dimmed)),
                        ),
                )
            })
            .when(compact, |this| {
                this.tooltip(move |_, cx| cx.new(|_| ChromeTooltip::new(title)).into())
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                if !this.settings_expanded_groups.insert(group.to_string()) {
                    this.settings_expanded_groups.remove(group);
                }
                cx.notify();
            }))
    }

    fn settings_tab_button(
        &self,
        tab: SettingsTab,
        id: &'static str,
        compact: bool,
        nested: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let selected = self.settings_active_tab == tab;
        let label = self.tr(tab.i18n_key());

        // Tauri settings nav item: soft primary fill, no permanent green border.
        div()
            .id(id)
            .mt(if nested { px(4.) } else { px(8.) })
            .h(px(if nested { 34. } else { 40. }))
            .when(!compact, |this| this.px_3())
            .flex()
            .items_center()
            .when(compact, |this| this.justify_center())
            .gap_2()
            .rounded_lg()
            .border_1()
            .border_color(if selected {
                rgba((palette.primary << 8) | 0x33)
            } else {
                rgb(0x00000000)
            })
            .bg(if selected {
                rgba((palette.primary << 8) | 0x1f)
            } else {
                rgb(0x00000000)
            })
            .text_color(if selected {
                rgb(palette.text)
            } else {
                rgb(palette.text_muted)
            })
            .text_size(px(if nested { 13. } else { 14. }))
            .font_weight(if selected {
                FontWeight(600.)
            } else {
                FontWeight(500.)
            })
            .cursor_pointer()
            .hover(move |this| {
                this.bg(if selected {
                    rgba((palette.primary << 8) | 0x29)
                } else {
                    rgb(palette.hover)
                })
                .text_color(rgb(palette.text))
            })
            .child(
                svg()
                    .size(px(if nested { 16. } else { 18. }))
                    .flex_none()
                    .path(tab.icon_path())
                    .text_color(if selected {
                        rgb(palette.primary)
                    } else {
                        rgb(palette.text_muted)
                    }),
            )
            .when(!compact, |this| {
                this.child(div().min_w_0().flex_1().overflow_hidden().child(label))
            })
            .when(compact, |this| {
                this.tooltip(move |_, cx| cx.new(|_| ChromeTooltip::new(label)).into())
            })
            .on_click(cx.listener(move |this, _, _, cx| {
                this.settings_active_tab = tab;
                this.appearance_menu_open = None;
                cx.notify();
            }))
    }

    fn settings_active_panel(
        &mut self,
        viewport_width: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let active_tab = self.settings_active_tab;
        let active_label = self.tr(active_tab.i18n_key());
        let content = self.settings_tab_content(active_tab, cx);
        let compact = viewport_width < 640.;
        let wide = viewport_width >= 1024.;

        // Match the responsive SettingsPage title and centered content column.
        div()
            .size_full()
            .min_w_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .flex_none()
                    .when(compact, |this| this.px_4().py_4())
                    .when(!compact, |this| this.px_6().py_5())
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .min_w_0()
                            .text_size(px(if compact { 18. } else { 24. }))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(palette.text))
                            .child(active_label),
                    ),
            )
            .child(
                div()
                    .id(SharedString::from("settings-content-scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_scroll()
                    .when(compact, |this| this.px_4().py_4())
                    .when(!compact && !wide, |this| this.px_6().py_6())
                    .when(wide, |this| this.px_8().py_8())
                    .child(
                        div()
                            .w_full()
                            .max_w(px(1024.))
                            .mx_auto()
                            .flex()
                            .flex_col()
                            .gap(if compact { px(20.) } else { px(24.) })
                            .child(content),
                    ),
            )
    }

    fn settings_tab_content(
        &mut self,
        active_tab: SettingsTab,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match active_tab {
            SettingsTab::General => self.general_settings_section(cx).into_any_element(),
            SettingsTab::Appearance => self.appearance_settings_section(cx).into_any_element(),
            SettingsTab::Interaction => self.interaction_settings_section(cx).into_any_element(),
            SettingsTab::Keybindings => self.keybindings_settings_section(cx).into_any_element(),
            SettingsTab::TerminalGeneral => self
                .terminal_general_settings_section(cx)
                .into_any_element(),
            SettingsTab::Search => self.terminal_search_settings_section(cx).into_any_element(),
            SettingsTab::Translation => self.translation_settings_section(cx).into_any_element(),
            SettingsTab::AiGeneral => self.ai_settings_section(cx).into_any_element(),
            SettingsTab::AiModels => self.ai_models_settings_section(cx).into_any_element(),
            SettingsTab::AiRules => self.ai_rules_settings_section(cx).into_any_element(),
            SettingsTab::Transfer => self.transfer_settings_section(cx).into_any_element(),
            SettingsTab::Security => self.security_settings_section(cx).into_any_element(),
            SettingsTab::SyncBackup => self.cloud_sync_settings_section(cx).into_any_element(),
        }
    }
}

/// Tauri SettingSection: rounded card with optional title/desc and body.
pub(super) fn settings_form_section(
    palette: ThemePalette,
    title: Option<&'static str>,
    desc: Option<&'static str>,
    content: impl IntoElement,
) -> impl IntoElement {
    div()
        .rounded_lg()
        .border_1()
        .border_color(rgba((palette.border << 8) | 0xb3))
        .bg(rgba((palette.surface << 8) | 0x99))
        .overflow_hidden()
        .when(title.is_some() || desc.is_some(), |this| {
            this.child(
                div()
                    .px_4()
                    .py_4()
                    .border_b_1()
                    .border_color(rgba((palette.surface_elevated << 8) | 0x99))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .when_some(title, |this, title| {
                        this.child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight(600.))
                                .text_color(rgb(palette.text))
                                .child(title),
                        )
                    })
                    .when_some(desc, |this, desc| {
                        this.child(
                            div()
                                .text_size(px(12.))
                                .text_color(rgb(palette.text_dimmed))
                                .child(desc),
                        )
                    }),
            )
        })
        .child(div().px_4().py_4().flex().flex_col().gap_4().child(content))
}

/// Tauri SettingRow: label/desc left, control right.
pub(super) fn settings_form_row(
    palette: ThemePalette,
    label: impl Into<SharedString>,
    desc: Option<SharedString>,
    control: impl IntoElement,
) -> impl IntoElement {
    let label = label.into();
    div()
        .flex()
        .flex_wrap()
        .items_start()
        .justify_between()
        .gap_4()
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_size(px(14.))
                        .font_weight(FontWeight(500.))
                        .text_color(rgb(palette.text))
                        .child(label),
                )
                .when_some(desc, |this, desc| {
                    this.child(
                        div()
                            .text_size(px(12.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(desc),
                    )
                }),
        )
        .child(
            div()
                .flex_none()
                .min_w_0()
                .max_w_full()
                .flex()
                .items_center()
                .justify_end()
                .gap_2()
                .child(control),
        )
}

/// Compact on/off switch control (Tauri SettingSwitch look).
pub(in crate::features::pages) fn settings_switch(
    palette: ThemePalette,
    id: impl Into<String>,
    checked: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    settings_switch_with_enabled(palette, id, checked, true, on_click)
}

pub(super) fn settings_switch_with_enabled(
    palette: ThemePalette,
    id: impl Into<String>,
    checked: bool,
    enabled: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let on_bg = palette.primary;
    let off_bg = palette.border;
    let on_hover = palette.primary_hover;
    let off_hover = palette.hover;
    div()
        .id(SharedString::from(id.into()))
        .h(px(22.))
        .w(px(40.))
        .flex()
        .items_center()
        .rounded_full()
        .px(px(2.))
        .bg(if checked { rgb(on_bg) } else { rgb(off_bg) })
        .opacity(if enabled { 1.0 } else { 0.45 })
        .when(enabled, |this| {
            this.cursor_pointer()
                .hover(move |this| {
                    this.bg(if checked {
                        rgb(on_hover)
                    } else {
                        rgb(off_hover)
                    })
                })
                .on_click(on_click)
        })
        .child(
            div()
                .size(px(18.))
                .rounded_full()
                .bg(rgb(0xffffff))
                .when(checked, |this| this.ml_auto()),
        )
}

/// Compact choice chips for enum-like settings.
pub(super) fn settings_choice_chip(
    palette: ThemePalette,
    id: impl Into<String>,
    label: impl Into<SharedString>,
    selected: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let label = label.into();
    let selected_border = palette.primary;
    let idle_border = palette.border;
    // Selected chip bg: light themes tint lightly; dark themes use hover/elevated surface.
    let selected_bg = {
        let luminance = {
            let r = ((palette.bg >> 16) & 0xff) as f32;
            let g = ((palette.bg >> 8) & 0xff) as f32;
            let b = (palette.bg & 0xff) as f32;
            (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0
        };
        if luminance > 0.6 {
            // light: soft hover-ish fill
            palette.hover
        } else {
            palette.hover
        }
    };
    let idle_bg = palette.input;
    let selected_text = palette.primary;
    let idle_text = palette.text_muted;
    let hover_bg = palette.surface_elevated;
    let hover_text = palette.text;
    div()
        .id(SharedString::from(id.into()))
        .h(px(28.))
        .px_2()
        .flex()
        .items_center()
        .rounded_md()
        .border_1()
        .border_color(if selected {
            rgb(selected_border)
        } else {
            rgb(idle_border)
        })
        .bg(if selected {
            rgb(selected_bg)
        } else {
            rgb(idle_bg)
        })
        .text_size(px(11.))
        .font_weight(if selected {
            FontWeight(600.)
        } else {
            FontWeight(500.)
        })
        .text_color(if selected {
            rgb(selected_text)
        } else {
            rgb(idle_text)
        })
        .cursor_pointer()
        .hover(move |this| this.bg(rgb(hover_bg)).text_color(rgb(hover_text)))
        .child(label)
        .on_click(on_click)
}

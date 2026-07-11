use gpui::{
    AnyElement, App, ClickEvent, Context, FontWeight, IntoElement, KeyDownEvent, SharedString,
    Window, div, prelude::*, px, rgb,
};
use nyaterm_domain::{
    AgentCommandExecutionMode, AiCustomActionConfig, AiMode, AiModelSource, AiProviderKind,
    RiskLevel,
};
use nyaterm_session::SftpDuplicatePolicy;

use crate::ui::components::{section_header, small_button, status_pill};
use crate::ui::theme::ThemePalette;
use crate::ui::models::{
    AiInputField, CloudSyncConflictState, CloudSyncInputField, ConfigPathPromptKind,
    DiagnosticsPathPromptKind, KeywordHighlightEditorField, KeywordHighlightPathPromptKind, SettingsTab,
    SearchEngineEditorField, SnapshotPasswordPromptKind, SnapshotPasswordPromptState, TerminalSearchMode, TransferJobStatus,
    TransferPathPromptKind, TranslateInputField,
};

use super::super::{
    NyaTermApp, TAB_MOUSE_ACTIONS, TabMouseActionTarget, ai_active_profile_api_key, cloud_secret_display,
    cloud_sync_history_row, compact_id, compact_setting_state, configured_cloud_sync_provider,
    metric, none_if_blank, policy_button, setting_state, tab_mouse_action_label, transfer_input,
    truncate_preview,
};

mod ai;
mod security;
mod sync_backup;
mod terminal;
mod transfer;
mod translation;
mod workspace;

impl NyaTermApp {
    pub(in crate::ui::view) fn settings_view(
        &mut self,
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
        self.settings_shell(backup_snapshot_prompt, cx)
    }

    pub(in crate::ui::view) fn settings_shell(
        &mut self,
        backup_snapshot_prompt: Option<SnapshotPasswordPromptState>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        // Tauri SettingsPage shell: compact header + narrow nav + scroll content.
        let palette = self.theme_palette();
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(palette.bg))
            .child(
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
                                    .child("Settings"),
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
                                    .child(self.settings_active_tab.group_label()),
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
                                    .child(self.settings_active_tab.label()),
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
                            .child("Back")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close_settings(cx);
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h(px(0.))
                    .size_full()
                    .child(self.settings_sidebar(cx))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .min_h_0()
                            .overflow_hidden()
                            .bg(rgb(palette.bg))
                            .child(self.settings_active_panel(backup_snapshot_prompt, cx)),
                    ),
            )
    }

    fn settings_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = self.theme_palette();
        div()
            .id(SharedString::from("settings-sidebar-scroll"))
            .w(px(220.))
            .flex_none()
            .h_full()
            .border_r_1()
            .border_color(rgb(palette.border))
            .bg(rgb(palette.surface))
            .px_2()
            .py_2()
            .overflow_scroll()
            .child(settings_category_header(palette, "Workspace", "WS", rgb(palette.accent)))
            .child(self.settings_tab_button(SettingsTab::General, "settings-tab-general", cx))
            .child(self.settings_tab_button(SettingsTab::Appearance, "settings-tab-appearance", cx))
            .child(self.settings_tab_button(
                SettingsTab::Interaction,
                "settings-tab-interaction",
                cx,
            ))
            .child(self.settings_tab_button(
                SettingsTab::Keybindings,
                "settings-tab-keybindings",
                cx,
            ))
            .child(settings_category_header(palette, 
                "Terminal Session",
                "TM",
                rgb(palette.success),
            ))
            .child(self.settings_tab_button(
                SettingsTab::TerminalGeneral,
                "settings-tab-terminal-general",
                cx,
            ))
            .child(self.settings_tab_button(SettingsTab::Search, "settings-tab-search", cx))
            .child(self.settings_tab_button(
                SettingsTab::Translation,
                "settings-tab-translation",
                cx,
            ))
            .child(settings_category_header(palette, "AI", "AI", rgb(0xbc8cff)))
            .child(self.settings_tab_button(SettingsTab::AiGeneral, "settings-tab-ai-general", cx))
            .child(self.settings_tab_button(SettingsTab::AiModels, "settings-tab-ai-models", cx))
            .child(self.settings_tab_button(SettingsTab::AiRules, "settings-tab-ai-rules", cx))
            .child(settings_category_header(palette, "Transfer", "TF", rgb(palette.accent)))
            .child(self.settings_tab_button(SettingsTab::Transfer, "settings-tab-transfer", cx))
            .child(settings_category_header(palette, "Security", "SC", rgb(palette.warning)))
            .child(self.settings_tab_button(SettingsTab::Security, "settings-tab-security", cx))
            .child(settings_category_header(palette, "Sync Backup", "BK", rgb(palette.success)))
            .child(self.settings_tab_button(
                SettingsTab::SyncBackup,
                "settings-tab-sync-backup",
                cx,
            ))
    }

    fn settings_tab_button(
        &self,
        tab: SettingsTab,
        id: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let selected = self.settings_active_tab == tab;

        // Tauri settings nav item: soft primary fill, no permanent green border.
        div()
            .id(id)
            .mt_0()
            .h(px(30.))
            .px_2()
            .flex()
            .items_center()
            .gap_2()
            .rounded_md()
            .border_1()
            .border_color(if selected {
                rgb(0x1f6feb)
            } else {
                rgb(0x00000000)
            })
            .bg(if selected {
                rgb(palette.hover)
            } else {
                rgb(0x00000000)
            })
            .text_color(if selected {
                rgb(palette.text)
            } else {
                rgb(palette.text_muted)
            })
            .text_size(px(12.))
            .font_weight(if selected {
                FontWeight(600.)
            } else {
                FontWeight(500.)
            })
            .cursor_pointer()
            .hover(move |this| this.bg(rgb(palette.hover)).text_color(rgb(palette.text)))
            .child(
                div()
                    .size(px(6.))
                    .rounded_full()
                    .flex_none()
                    .bg(if selected {
                        rgb(palette.accent)
                    } else {
                        rgb(palette.border)
                    }),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .overflow_hidden()
                    .child(tab.label()),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.settings_active_tab = tab;
                cx.notify();
            }))
    }

    fn settings_active_panel(
        &mut self,
        backup_snapshot_prompt: Option<SnapshotPasswordPromptState>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let active_tab = self.settings_active_tab;
        let content = self.settings_tab_content(active_tab, backup_snapshot_prompt, cx);

        // Tauri content pane: no heavy outer card; compact title strip + scroll body.
        div()
            .size_full()
            .min_w_0()
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(
                div()
                    .flex_none()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(palette.border))
                    .flex()
                    .items_end()
                    .justify_between()
                    .gap_3()
                    .child(
                        div()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap_0()
                            .child(
                                div()
                                    .text_size(px(10.))
                                    .font_weight(FontWeight(600.))
                                    .text_color(rgb(palette.text_dimmed))
                                    .child(active_tab.group_label()),
                            )
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .font_weight(FontWeight(700.))
                                    .text_color(rgb(palette.text))
                                    .child(active_tab.label()),
                            ),
                    ),
            )
            .child(
                div()
                    .id(SharedString::from("settings-content-scroll"))
                    .flex_1()
                    .min_h_0()
                    .overflow_scroll()
                    .px_4()
                    .py_3()
                    .child(div().flex().flex_col().gap_3().child(content)),
            )
    }

    fn settings_tab_content(
        &mut self,
        active_tab: SettingsTab,
        backup_snapshot_prompt: Option<SnapshotPasswordPromptState>,
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
            SettingsTab::SyncBackup => div()
                .flex()
                .flex_col()
                .gap_4()
                .child(self.config_backup_settings_section(backup_snapshot_prompt, cx))
                .child(self.cloud_sync_settings_section(cx))
                .child(self.diagnostics_settings_section(cx))
                .into_any_element(),
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
        .border_color(rgb(palette.border))
        .bg(rgb(palette.surface))
        .overflow_hidden()
        .when(title.is_some() || desc.is_some(), |this| {
            this.child(
                div()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(rgb(palette.surface_elevated))
                    .flex()
                    .flex_col()
                    .gap_1()
                    .when_some(title, |this, title| {
                        this.child(
                            div()
                                .text_size(px(13.))
                                .font_weight(FontWeight(600.))
                                .text_color(rgb(palette.text))
                                .child(title),
                        )
                    })
                    .when_some(desc, |this, desc| {
                        this.child(
                            div()
                                .text_size(px(11.))
                                .text_color(rgb(palette.text_dimmed))
                                .child(desc),
                        )
                    }),
            )
        })
        .child(div().px_4().py_3().flex().flex_col().gap_3().child(content))
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
                        .text_size(px(13.))
                        .font_weight(FontWeight(500.))
                        .text_color(rgb(palette.text))
                        .child(label),
                )
                .when_some(desc, |this, desc| {
                    this.child(
                        div()
                            .text_size(px(11.))
                            .text_color(rgb(palette.text_dimmed))
                            .child(desc),
                    )
                }),
        )
        .child(
            div()
                .flex_none()
                .flex()
                .items_center()
                .justify_end()
                .gap_2()
                .child(control),
        )
}

/// Compact on/off switch control (Tauri SettingSwitch look).
pub(super) fn settings_switch(
    palette: ThemePalette,
    id: impl Into<String>,
    checked: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let on_bg = palette.accent;
    let off_bg = palette.border;
    let on_hover = if palette.accent == 0x58a6ff {
        0x388bfd
    } else if palette.accent == 0x0969da {
        0x218bff
    } else {
        // lighten-ish fallback for catppuccin accent
        0xd4b8fa
    };
    let off_hover = palette.hover;
    div()
        .id(SharedString::from(id.into()))
        .h(px(22.))
        .w(px(40.))
        .flex()
        .items_center()
        .rounded_full()
        .px(px(2.))
        .bg(if checked {
            rgb(on_bg)
        } else {
            rgb(off_bg)
        })
        .cursor_pointer()
        .hover(move |this| {
            this.bg(if checked {
                rgb(on_hover)
            } else {
                rgb(off_hover)
            })
        })
        .child(
            div()
                .size(px(18.))
                .rounded_full()
                .bg(rgb(0xffffff))
                .when(checked, |this| this.ml_auto()),
        )
        .on_click(on_click)
}

/// Compact choice chips for enum-like settings.
pub(super) fn settings_choice_chip(
    palette: ThemePalette,
    id: impl Into<String>,
    label: &'static str,
    selected: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let selected_border = palette.accent;
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
    let selected_text = palette.accent;
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

fn settings_category_header(
    palette: ThemePalette,
    title: &'static str,
    _badge: &'static str,
    _accent: impl Into<gpui::Hsla>,
) -> impl IntoElement {
    div()
        .mt_2()
        .mb_1()
        .px_2()
        .py_1()
        .flex()
        .items_center()
        .text_size(px(10.))
        .font_weight(FontWeight(700.))
        .text_color(rgb(palette.text_dimmed))
        .child(title.to_uppercase())
}

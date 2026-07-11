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
use crate::ui::models::{
    AiInputField, CloudSyncConflictState, CloudSyncInputField, ConfigPathPromptKind,
    DiagnosticsPathPromptKind, KeywordHighlightPathPromptKind, SettingsTab,
    SnapshotPasswordPromptKind, SnapshotPasswordPromptState, TerminalSearchMode, TransferJobStatus,
    TransferPathPromptKind, TranslateInputField,
};

use super::super::{
    NyaTermApp, TabMouseActionTarget, ai_active_profile_api_key, cloud_secret_display,
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
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x0d1117))
            .child(
                div()
                    .h(px(36.))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_3()
                    .border_b_1()
                    .border_color(rgb(0x30363d))
                    .bg(rgb(0x12171f))
                    .child(
                        div()
                            .text_size(px(12.))
                            .font_weight(FontWeight(700.))
                            .text_color(rgb(0xc9d1d9))
                            .child("Settings"),
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
                            .text_color(rgb(0x8b949e))
                            .cursor_pointer()
                            .hover(|this| this.bg(rgb(0x21262d)).text_color(rgb(0xc9d1d9)))
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
                            .bg(rgb(0x0d1117))
                            .child(self.settings_active_panel(backup_snapshot_prompt, cx)),
                    ),
            )
    }

    fn settings_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id(SharedString::from("settings-sidebar-scroll"))
            .w(px(220.))
            .flex_none()
            .h_full()
            .border_r_1()
            .border_color(rgb(0x30363d))
            .bg(rgb(0x161b22))
            .px_2()
            .py_2()
            .overflow_scroll()
            .child(settings_category_header("Workspace", "WS", rgb(0x58a6ff)))
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
            .child(settings_category_header(
                "Terminal Session",
                "TM",
                rgb(0x3fb950),
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
            .child(settings_category_header("AI", "AI", rgb(0xbc8cff)))
            .child(self.settings_tab_button(SettingsTab::AiGeneral, "settings-tab-ai-general", cx))
            .child(self.settings_tab_button(SettingsTab::AiModels, "settings-tab-ai-models", cx))
            .child(self.settings_tab_button(SettingsTab::AiRules, "settings-tab-ai-rules", cx))
            .child(settings_category_header("Transfer", "TF", rgb(0x58a6ff)))
            .child(self.settings_tab_button(SettingsTab::Transfer, "settings-tab-transfer", cx))
            .child(settings_category_header("Security", "SC", rgb(0xd29922)))
            .child(self.settings_tab_button(SettingsTab::Security, "settings-tab-security", cx))
            .child(settings_category_header("Sync Backup", "BK", rgb(0x3fb950)))
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
                rgb(0x122033)
            } else {
                rgb(0x00000000)
            })
            .text_color(if selected {
                rgb(0xc9d1d9)
            } else {
                rgb(0x8b949e)
            })
            .text_size(px(12.))
            .font_weight(if selected {
                FontWeight(600.)
            } else {
                FontWeight(500.)
            })
            .cursor_pointer()
            .hover(|this| this.bg(rgb(0x1c2128)).text_color(rgb(0xc9d1d9)))
            .child(
                div()
                    .size(px(6.))
                    .rounded_full()
                    .flex_none()
                    .bg(if selected {
                        rgb(0x58a6ff)
                    } else {
                        rgb(0x30363d)
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
                    .border_color(rgb(0x21262d))
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
                                    .text_color(rgb(0x6e7681))
                                    .child(active_tab.group_label()),
                            )
                            .child(
                                div()
                                    .text_size(px(16.))
                                    .font_weight(FontWeight(700.))
                                    .text_color(rgb(0xc9d1d9))
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

fn settings_category_header(
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
        .text_color(rgb(0x6e7681))
        .child(title.to_uppercase())
}

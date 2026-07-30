use gpui::{
    App, ClickEvent, Context, FontWeight, IntoElement, SharedString, Window, div, prelude::*, px,
    rgb,
};

use crate::features::{
    NyaTermApp, TAB_MOUSE_ACTIONS, TabMouseActionTarget, TextInputSetup, gpui_code_font_family,
};
use crate::theme::ThemePalette;
use crate::widgets::small_button;

use super::super::{
    settings_choice_chip, settings_form_row, settings_form_section, settings_switch,
};

impl NyaTermApp {
    pub(in crate::features) fn interaction_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let encoding = self.settings.summary().interaction_default_encoding.clone();
        // Built before the form, which reads `self` throughout: creating the
        // box needs it mutably.
        let word_separators_input = self
            .text_input_box(
                "settings.interaction.word-separators",
                &self.settings.summary().interaction_word_separators.clone(),
                TextInputSetup::default(),
                cx,
            )
            .into_any_element();
        let double_action = self
            .settings
            .summary()
            .interaction_tab_double_click_action
            .clone();
        let middle_action = self
            .settings
            .summary()
            .interaction_tab_middle_click_action
            .clone();
        let right_action = self
            .settings
            .summary()
            .interaction_tab_right_click_action
            .clone();
        let delay_ms = self
            .settings
            .summary()
            .interaction_duplicate_session_command_delay_ms;
        let min_chars = self
            .settings
            .summary()
            .interaction_command_suggestion_min_chars;
        let max_chars = self
            .settings
            .summary()
            .interaction_command_suggestion_max_chars;
        let suggestions_enabled = self
            .settings
            .summary()
            .interaction_command_suggestions_enabled;

        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(settings_form_section(
                palette,
                Some(self.tr("settings.interactionClipboardMouse")),
                Some(self.tr("settings.interactionClipboardMouseDesc")),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.copyOnSelect"),
                        Some(SharedString::from(self.tr("settings.copyOnSelectDesc"))),
                        settings_switch(
                            palette,
                            "interaction-copy-select",
                            self.settings.summary().interaction_copy_on_select,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_interaction_copy_on_select(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.rightClickPaste"),
                        Some(SharedString::from(self.tr("settings.rightClickPasteDesc"))),
                        settings_switch(
                            palette,
                            "interaction-right-paste",
                            self.settings.summary().interaction_right_click_paste,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_interaction_right_click_paste(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some(self.tr("settings.interactionCommandInput")),
                Some(self.tr("settings.interactionCommandInputDesc")),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.commandSuggestions"),
                        Some(SharedString::from(
                            self.tr("settings.commandSuggestionsDesc"),
                        )),
                        settings_switch(
                            palette,
                            "interaction-cmd-suggestions",
                            suggestions_enabled,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_command_suggestions(cx);
                            }),
                        ),
                    ))
                    .when(suggestions_enabled, |this| {
                        this.child(settings_form_row(
                            palette,
                            self.tr("settings.commandSuggestionsMinChars"),
                            Some(SharedString::from(
                                self.tr("settings.commandSuggestionsMinCharsDesc"),
                            )),
                            interaction_number_stepper(
                                palette,
                                "interaction-suggest-min-minus",
                                "interaction-suggest-min-plus",
                                min_chars,
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_command_suggestion_min_chars(-1, cx);
                                }),
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_command_suggestion_min_chars(1, cx);
                                }),
                            ),
                        ))
                        .child(settings_form_row(
                            palette,
                            self.tr("settings.commandSuggestionsMaxChars"),
                            Some(SharedString::from(
                                self.tr("settings.commandSuggestionsMaxCharsDesc"),
                            )),
                            interaction_number_stepper(
                                palette,
                                "interaction-suggest-max-minus",
                                "interaction-suggest-max-plus",
                                max_chars,
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_command_suggestion_max_chars(-1, cx);
                                }),
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_command_suggestion_max_chars(1, cx);
                                }),
                            ),
                        ))
                    })
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(settings_field_meta(
                                palette,
                                self.tr("settings.wordSeparators"),
                                self.tr("settings.wordSeparatorsDesc"),
                            ))
                            .child(div().w_full().max_w(px(640.)).child(word_separators_input)),
                    )
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.duplicateSessionCommandDelay"),
                        Some(SharedString::from(
                            self.tr("settings.duplicateSessionCommandDelayDesc"),
                        )),
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(small_button(
                                palette,
                                "interaction-dup-delay-minus",
                                "-",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_duplicate_session_command_delay(-100, cx);
                                }),
                            ))
                            .child(
                                div()
                                    .min_w(px(64.))
                                    .text_center()
                                    .font_family(gpui_code_font_family())
                                    .text_size(px(11.))
                                    .text_color(rgb(palette.text))
                                    .child(format!("{delay_ms} ms")),
                            )
                            .child(small_button(
                                palette,
                                "interaction-dup-delay-plus",
                                "+",
                                cx.listener(|this, _, _, cx| {
                                    this.adjust_duplicate_session_command_delay(100, cx);
                                }),
                            )),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.altAsMeta"),
                        Some(SharedString::from(self.tr("settings.altAsMetaDesc"))),
                        settings_switch(
                            palette,
                            "interaction-alt-meta",
                            self.settings.summary().interaction_alt_as_meta,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_alt_as_meta(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("settings.macImeCompatibility"),
                        Some(SharedString::from(
                            self.tr("settings.macImeCompatibilityDesc"),
                        )),
                        settings_switch(
                            palette,
                            "interaction-mac-ime",
                            self.settings.summary().interaction_mac_ime_compatibility,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_mac_ime_compatibility(cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some(self.tr("settings.tabMouseActions")),
                Some(self.tr("settings.tabMouseActionsDesc")),
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(self.tab_mouse_action_settings_field(
                        palette,
                        TabMouseActionPresentation {
                            label: self.tr("settings.tabDoubleClickAction"),
                            description: self.tr("settings.tabDoubleClickActionDesc"),
                            id_prefix: "interaction-tab-double",
                            target: TabMouseActionTarget::Double,
                            current: &double_action,
                        },
                        cx,
                    ))
                    .child(self.tab_mouse_action_settings_field(
                        palette,
                        TabMouseActionPresentation {
                            label: self.tr("settings.tabMiddleClickAction"),
                            description: self.tr("settings.tabMiddleClickActionDesc"),
                            id_prefix: "interaction-tab-middle",
                            target: TabMouseActionTarget::Middle,
                            current: &middle_action,
                        },
                        cx,
                    ))
                    .child(self.tab_mouse_action_settings_field(
                        palette,
                        TabMouseActionPresentation {
                            label: self.tr("settings.tabRightClickAction"),
                            description: self.tr("settings.tabRightClickActionDesc"),
                            id_prefix: "interaction-tab-right",
                            target: TabMouseActionTarget::Right,
                            current: &right_action,
                        },
                        cx,
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some(self.tr("settings.interactionEncoding")),
                Some(self.tr("settings.interactionEncodingDesc")),
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(settings_field_meta(
                        palette,
                        self.tr("settings.defaultEncoding"),
                        self.tr("settings.defaultEncodingDesc"),
                    ))
                    .child(
                        div()
                            .flex()
                            .gap_1()
                            .child(settings_choice_chip(
                                palette,
                                "interaction-encoding-utf8",
                                "UTF-8",
                                encoding == "UTF-8",
                                cx.listener(|this, _, _, cx| {
                                    this.set_interaction_encoding("UTF-8", cx);
                                }),
                            ))
                            .child(settings_choice_chip(
                                palette,
                                "interaction-encoding-gbk",
                                "GBK",
                                encoding == "GBK",
                                cx.listener(|this, _, _, cx| {
                                    this.set_interaction_encoding("GBK", cx);
                                }),
                            )),
                    ),
            ))
    }

    fn tab_mouse_action_settings_field(
        &mut self,
        palette: ThemePalette,
        presentation: TabMouseActionPresentation<'_>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let TabMouseActionPresentation {
            label,
            description,
            id_prefix,
            target,
            current,
        } = presentation;
        let chips = TAB_MOUSE_ACTIONS.iter().fold(
            div().flex().flex_wrap().gap_1().max_w(px(640.)),
            |row, action| {
                let action_id = (*action).to_string();
                let selected = current == *action;
                let chip_id = format!("{id_prefix}-{action}");
                let action_label = self.tr(tab_mouse_action_i18n_key(action));
                row.child(settings_choice_chip(
                    palette,
                    chip_id,
                    action_label,
                    selected,
                    cx.listener(move |this, _, _, cx| {
                        this.set_tab_mouse_action(target, &action_id, cx);
                    }),
                ))
            },
        );

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(settings_field_meta(palette, label, description))
            .child(chips)
    }
}

struct TabMouseActionPresentation<'a> {
    label: &'static str,
    description: &'static str,
    id_prefix: &'static str,
    target: TabMouseActionTarget,
    current: &'a str,
}

fn interaction_number_stepper(
    palette: ThemePalette,
    minus_id: &'static str,
    plus_id: &'static str,
    value: u32,
    on_minus: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_plus: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_1()
        .child(small_button(palette, minus_id, "-", on_minus))
        .child(
            div()
                .min_w(px(42.))
                .text_center()
                .font_family(gpui_code_font_family())
                .text_size(px(11.))
                .text_color(rgb(palette.text))
                .child(value.to_string()),
        )
        .child(small_button(palette, plus_id, "+", on_plus))
}

fn settings_field_meta(
    palette: ThemePalette,
    label: &'static str,
    desc: &'static str,
) -> impl IntoElement {
    div()
        .min_w_0()
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight(500.))
                .text_color(rgb(palette.text))
                .child(label),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(11.))
                .text_color(rgb(palette.text_dimmed))
                .child(desc),
        )
}

fn tab_mouse_action_i18n_key(action: &str) -> &'static str {
    match action {
        "rename_tab" => "tabCtx.rename",
        "copy_tab_name" => "tabCtx.copyName",
        "copy_server_ip" => "tabCtx.copyIp",
        "duplicate_session" => "tabCtx.duplicate",
        "multiplex_ssh" => "tabCtx.multiplexSsh",
        "reconnect_session" => "tabCtx.reconnect",
        "disconnect_session" => "tabCtx.disconnect",
        "close_tab" => "tabCtx.close",
        _ => "settings.tabMouseActionNone",
    }
}

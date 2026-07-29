use gpui::{
    AnyElement, App, ClickEvent, Context, IntoElement, SharedString, Window, div, prelude::*, px,
    rgb,
};
use nyaterm_core::RiskLevel;

use crate::features::{NyaTermApp, TextInputSetup};
use crate::models::AiInputField;
use crate::theme::ThemePalette;
use crate::widgets::small_button;

use super::super::{settings_form_row, settings_form_section, settings_switch};

impl NyaTermApp {
    pub(in crate::features) fn ai_input(
        &mut self,
        _id: &'static str,
        label: &'static str,
        value: String,
        field: AiInputField,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let setup = if field == AiInputField::ApiKey {
            TextInputSetup::masked()
        } else {
            TextInputSetup::default()
        };
        self.text_input_field(
            format!("ai.input.{}", field.input_key()),
            label,
            &value,
            setup,
            cx,
        )
        .into_any_element()
    }

    pub(in crate::features) fn ai_settings_section(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let risk_menu_id = "ai-smart-risk";
        let risk_menu_open = self.settings.appearance_menu_open(risk_menu_id);
        let risk_label = self.tr(ai_risk_i18n_key(
            &self.ai.settings.config.agent_smart_auto_execute_max_risk,
        ));
        let risk_low = self.tr("ai.riskLow");
        let risk_medium = self.tr("ai.riskMedium");
        let risk_high = self.tr("ai.riskHigh");
        let risk_critical = self.tr("ai.riskCritical");

        div()
            .flex()
            .flex_col()
            .gap_5()
            .child(settings_form_section(
                palette,
                Some(self.tr("ai.general")),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        self.tr("ai.enabled"),
                        None,
                        settings_switch(
                            palette,
                            "ai-enabled",
                            self.ai.settings.config.enabled,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_ai_enabled(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("ai.redaction"),
                        None,
                        settings_switch(
                            palette,
                            "ai-redaction-toggle",
                            self.ai.settings.config.redaction_enabled,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_ai_redaction(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("ai.allowSave"),
                        None,
                        settings_switch(
                            palette,
                            "ai-save-command-toggle",
                            self.ai.settings.config.allow_save_command,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_ai_allow_save_command(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("ai.recordHistory"),
                        None,
                        settings_switch(
                            palette,
                            "ai-history-toggle",
                            self.ai.settings.config.record_history,
                            cx.listener(|this, _, _, cx| {
                                this.toggle_ai_record_history(cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("ai.requestUserAgent"),
                        Some(SharedString::from(self.tr("ai.requestUserAgentDesc"))),
                        div().w_full().max_w(px(300.)).child(self.ai_input(
                            "ai-request-user-agent",
                            self.tr("ai.requestUserAgent"),
                            self.ai.settings.config.request_user_agent.clone(),
                            AiInputField::RequestUserAgent,
                            cx,
                        )),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("ai.contextLineLimit"),
                        None,
                        ai_number_stepper(
                            palette,
                            "ai-context-minus",
                            "ai-context-plus",
                            self.ai.settings.config.context_line_limit.to_string(),
                            cx.listener(|this, _, _, cx| {
                                this.adjust_ai_context_line_limit(-50, cx);
                            }),
                            cx.listener(|this, _, _, cx| {
                                this.adjust_ai_context_line_limit(50, cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("ai.timeoutMs"),
                        None,
                        ai_number_stepper(
                            palette,
                            "ai-timeout-minus",
                            "ai-timeout-plus",
                            self.ai.settings.config.timeout_ms.to_string(),
                            cx.listener(|this, _, _, cx| {
                                this.adjust_ai_timeout_ms(-1_000, cx);
                            }),
                            cx.listener(|this, _, _, cx| {
                                this.adjust_ai_timeout_ms(1_000, cx);
                            }),
                        ),
                    )),
            ))
            .child(settings_form_section(
                palette,
                Some(self.tr("ai.agentSettings")),
                None,
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(settings_form_row(
                        palette,
                        self.tr("ai.smartAutoExecuteMaxRisk"),
                        Some(SharedString::from(
                            self.tr("ai.smartAutoExecuteMaxRiskDesc"),
                        )),
                        ai_risk_select(
                            palette,
                            risk_menu_open,
                            risk_label,
                            cx.listener(move |this, _, _, cx| {
                                this.settings.toggle_appearance_menu(risk_menu_id);
                                cx.notify();
                            }),
                            [
                                (
                                    risk_low,
                                    RiskLevel::Low,
                                    self.ai.settings.config.agent_smart_auto_execute_max_risk
                                        == RiskLevel::Low,
                                ),
                                (
                                    risk_medium,
                                    RiskLevel::Medium,
                                    self.ai.settings.config.agent_smart_auto_execute_max_risk
                                        == RiskLevel::Medium,
                                ),
                                (
                                    risk_high,
                                    RiskLevel::High,
                                    self.ai.settings.config.agent_smart_auto_execute_max_risk
                                        == RiskLevel::High,
                                ),
                                (
                                    risk_critical,
                                    RiskLevel::Critical,
                                    self.ai.settings.config.agent_smart_auto_execute_max_risk
                                        == RiskLevel::Critical,
                                ),
                            ]
                            .into_iter()
                            .enumerate()
                            .map(|(index, (label, risk, selected))| {
                                let hover = palette.hover;
                                div()
                                    .id(SharedString::from(format!("ai-smart-risk-option-{index}")))
                                    .h(px(30.))
                                    .px_2()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .rounded_sm()
                                    .text_size(px(12.))
                                    .text_color(rgb(if selected {
                                        palette.primary
                                    } else {
                                        palette.text
                                    }))
                                    .cursor_pointer()
                                    .hover(move |this| this.bg(rgb(hover)))
                                    .child(label)
                                    .when(selected, |this| {
                                        this.child(crate::features::mono_icon(
                                            "icons/check.svg",
                                            rgb(palette.primary).into(),
                                            12.,
                                        ))
                                    })
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.settings.close_appearance_menu();
                                        this.update_ai_smart_auto_execute_max_risk(
                                            risk.clone(),
                                            cx,
                                        );
                                    }))
                            })
                            .collect(),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("ai.agentMaxSteps"),
                        None,
                        ai_number_stepper(
                            palette,
                            "ai-agent-steps-minus",
                            "ai-agent-steps-plus",
                            self.ai
                                .settings
                                .config
                                .max_agent_steps
                                .unwrap_or(10)
                                .to_string(),
                            cx.listener(|this, _, _, cx| {
                                this.adjust_ai_agent_steps(-1, cx);
                            }),
                            cx.listener(|this, _, _, cx| {
                                this.adjust_ai_agent_steps(1, cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("ai.agentStepTimeout"),
                        None,
                        ai_number_stepper(
                            palette,
                            "ai-agent-step-timeout-minus",
                            "ai-agent-step-timeout-plus",
                            self.ai
                                .settings
                                .config
                                .agent_step_timeout_ms
                                .unwrap_or(30_000)
                                .to_string(),
                            cx.listener(|this, _, _, cx| {
                                this.adjust_ai_agent_step_timeout_ms(-1_000, cx);
                            }),
                            cx.listener(|this, _, _, cx| {
                                this.adjust_ai_agent_step_timeout_ms(1_000, cx);
                            }),
                        ),
                    ))
                    .child(settings_form_row(
                        palette,
                        self.tr("ai.terminalOutputLines"),
                        None,
                        ai_number_stepper(
                            palette,
                            "ai-output-lines-minus",
                            "ai-output-lines-plus",
                            self.ai.settings.config.terminal_output_lines.to_string(),
                            cx.listener(|this, _, _, cx| {
                                this.adjust_ai_terminal_output_lines(-1, cx);
                            }),
                            cx.listener(|this, _, _, cx| {
                                this.adjust_ai_terminal_output_lines(1, cx);
                            }),
                        ),
                    ))
                    .child(ai_help_text(palette, self.tr("ai.agentMaxStepsDesc")))
                    .child(ai_help_text(palette, self.tr("ai.terminalOutputLinesDesc"))),
            ))
    }
}

fn ai_number_stepper(
    palette: ThemePalette,
    minus_id: &'static str,
    plus_id: &'static str,
    value: String,
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
                .min_w(px(72.))
                .text_center()
                .font_family(crate::features::gpui_code_font_family())
                .text_size(px(11.))
                .text_color(rgb(palette.text))
                .child(value),
        )
        .child(small_button(palette, plus_id, "+", on_plus))
}

fn ai_risk_select(
    palette: ThemePalette,
    open: bool,
    value: &'static str,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    options: Vec<gpui::Stateful<gpui::Div>>,
) -> impl IntoElement {
    let hover = palette.hover;
    div()
        .flex()
        .flex_col()
        .w(px(180.))
        .child(
            div()
                .id("ai-smart-risk-trigger")
                .h(px(34.))
                .w_full()
                .px_3()
                .rounded_sm()
                .border_1()
                .border_color(rgb(if open { palette.link } else { palette.border }))
                .bg(rgb(palette.input))
                .flex()
                .items_center()
                .justify_between()
                .gap_2()
                .cursor_pointer()
                .hover(move |this| this.bg(rgb(hover)))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(palette.text))
                        .child(value),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(palette.text_dimmed))
                        .child(if open { "^" } else { "v" }),
                )
                .on_click(on_toggle),
        )
        .when(open, |this| {
            this.child(
                div()
                    .mt_1()
                    .rounded_sm()
                    .border_1()
                    .border_color(rgb(palette.border))
                    .bg(rgb(palette.surface_elevated))
                    .p_1()
                    .children(options),
            )
        })
}

fn ai_help_text(palette: ThemePalette, text: &'static str) -> impl IntoElement {
    div()
        .text_size(px(11.))
        .text_color(rgb(palette.text_muted))
        .child(text)
}

fn ai_risk_i18n_key(risk: &RiskLevel) -> &'static str {
    match risk {
        RiskLevel::Low => "ai.riskLow",
        RiskLevel::Medium => "ai.riskMedium",
        RiskLevel::High => "ai.riskHigh",
        RiskLevel::Critical => "ai.riskCritical",
    }
}

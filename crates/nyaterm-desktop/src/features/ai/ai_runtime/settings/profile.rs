use super::*;

impl NyaTermApp {
    pub(in crate::features) fn toggle_ai_enabled(&mut self, cx: &mut Context<Self>) {
        self.ai.settings.config.enabled = !self.ai.settings.config.enabled;
        self.ai.panel.status = if self.ai.settings.config.enabled {
            "AI enabled"
        } else {
            "AI disabled"
        }
        .to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn set_ai_mode(&mut self, mode: AiMode, cx: &mut Context<Self>) {
        self.ai.settings.config.default_mode = mode;
        self.ai.panel.status = "AI mode updated".to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn set_ai_command_mode(
        &mut self,
        mode: AgentCommandExecutionMode,
        cx: &mut Context<Self>,
    ) {
        self.ai.settings.config.agent_command_execution_mode = mode;
        self.ai.panel.status = "Agent command policy updated".to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn toggle_ai_background_execution(&mut self, cx: &mut Context<Self>) {
        self.ai.settings.config.agent_background_execution_enabled =
            !self.ai.settings.config.agent_background_execution_enabled;
        self.ai.panel.status = if self.ai.settings.config.agent_background_execution_enabled {
            "Agent background execution enabled"
        } else {
            "Agent background execution disabled"
        }
        .to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn toggle_ai_redaction(&mut self, cx: &mut Context<Self>) {
        self.ai.settings.config.redaction_enabled = !self.ai.settings.config.redaction_enabled;
        self.ai.panel.status = "AI redaction updated".to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn toggle_ai_allow_save_command(&mut self, cx: &mut Context<Self>) {
        self.ai.settings.config.allow_save_command = !self.ai.settings.config.allow_save_command;
        self.ai.panel.status = "AI command saving updated".to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn toggle_ai_record_history(&mut self, cx: &mut Context<Self>) {
        self.ai.settings.config.record_history = !self.ai.settings.config.record_history;
        self.ai.panel.status = "AI history recording updated".to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn adjust_ai_context_line_limit(
        &mut self,
        delta: i32,
        cx: &mut Context<Self>,
    ) {
        let current = self.ai.settings.config.context_line_limit as i32;
        self.ai.settings.config.context_line_limit = (current + delta).clamp(50, 500) as u32;
        self.ai.panel.status = "AI context line limit updated".to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn adjust_ai_timeout_ms(&mut self, delta: i64, cx: &mut Context<Self>) {
        let current = self.ai.settings.config.timeout_ms as i64;
        self.ai.settings.config.timeout_ms = (current + delta).clamp(5_000, 300_000) as u64;
        self.ai.panel.status = "AI timeout updated".to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn adjust_ai_agent_steps(
        &mut self,
        delta: i16,
        cx: &mut Context<Self>,
    ) {
        let current = self.ai.settings.config.max_agent_steps.unwrap_or(10) as i16;
        self.ai.settings.config.max_agent_steps = Some((current + delta).clamp(1, 50) as u16);
        self.ai.panel.status = "AI Agent max steps updated".to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn adjust_ai_agent_step_timeout_ms(
        &mut self,
        delta: i64,
        cx: &mut Context<Self>,
    ) {
        let current = self
            .ai
            .settings
            .config
            .agent_step_timeout_ms
            .unwrap_or(30_000) as i64;
        self.ai.settings.config.agent_step_timeout_ms =
            Some((current + delta).clamp(5_000, 120_000) as u64);
        self.ai.panel.status = "AI Agent step timeout updated".to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn adjust_ai_terminal_output_lines(
        &mut self,
        delta: i16,
        cx: &mut Context<Self>,
    ) {
        let current = self.ai.settings.config.terminal_output_lines as i16;
        self.ai.settings.config.terminal_output_lines = (current + delta).clamp(0, 100) as u16;
        self.ai.panel.status = "AI terminal output lines updated".to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn adjust_ai_file_size_mb(
        &mut self,
        delta: i64,
        cx: &mut Context<Self>,
    ) {
        let mb = 1024 * 1024;
        let current = (self.ai.settings.config.max_ai_file_size_bytes / mb).max(1) as i64;
        self.ai.settings.config.max_ai_file_size_bytes =
            (current + delta).clamp(1, 256) as u64 * mb;
        self.ai.panel.status = "AI file size limit updated".to_string();
        self.persist_ai_settings_now(cx);
    }

    pub(in crate::features) fn update_ai_smart_auto_execute_max_risk(
        &mut self,
        risk: RiskLevel,
        cx: &mut Context<Self>,
    ) {
        self.ai.settings.config.agent_smart_auto_execute_max_risk = risk;
        self.ai.panel.status = "AI smart auto-execute risk updated".to_string();
        self.persist_ai_settings_now(cx);
    }
}

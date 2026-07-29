use gpui::{Context, KeyDownEvent, Window};

use crate::features::{NyaTermApp, TextInputSetup};
use crate::models::{SessionLaunchConfig, StartupCommandAction};

use super::state::RenameSessionSubmission;

impl NyaTermApp {
    pub(in crate::features) fn open_rename_session(
        &mut self,
        session_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(current_name) = self.session_display_name(&session_id) else {
            self.terminal.view.status = "session no longer exists".to_string();
            cx.notify();
            return;
        };
        self.session.dialogs.open_rename(session_id, &current_name);
        self.forget_text_inputs("session.rename");
        let rename_draft = self.session.dialogs.rename_draft().to_string();
        let field = self.text_input(
            "session.rename",
            &rename_draft,
            TextInputSetup::placeholder(self.tr("tabCtx.renamePlaceholder")),
            cx,
        );
        self.terminal.view.status = "rename tab opened".to_string();
        window.focus(&field.read(cx).focus_handle());
        field.update(cx, |field, cx| field.select_all(window, cx));
        cx.notify();
    }

    pub(in crate::features) fn close_rename_session(&mut self, cx: &mut Context<Self>) {
        self.session.dialogs.cancel_rename();
        self.forget_text_inputs("session.rename");
        self.terminal.view.status = "rename tab cancelled".to_string();
        cx.notify();
    }

    pub(in crate::features) fn submit_rename_session(&mut self, cx: &mut Context<Self>) {
        let (session_id, trimmed) = match self.session.dialogs.take_rename_submission() {
            RenameSessionSubmission::Inactive => {
                self.terminal.view.status = "no tab rename is active".to_string();
                cx.notify();
                return;
            }
            RenameSessionSubmission::Empty => {
                self.terminal.view.status = "tab name cannot be empty".to_string();
                cx.notify();
                return;
            }
            RenameSessionSubmission::Ready { session_id, name } => (session_id, name),
        };
        self.forget_text_inputs("session.rename");
        self.session
            .set_custom_name(session_id.clone(), trimmed.clone());
        self.terminal.view.status = format!("renamed tab to {trimmed}");
        cx.notify();
    }

    pub(in crate::features) fn handle_rename_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }
        match keystroke.key.as_str() {
            "escape" => self.close_rename_session(cx),
            "enter" => self.submit_rename_session(cx),
            _ => {}
        }
    }

    pub(in crate::features) fn open_startup_command_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.open_startup_command_dialog_for(StartupCommandAction::Duplicate, window, cx);
    }

    pub(in crate::features) fn open_startup_command_dialog_for(
        &mut self,
        action: StartupCommandAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.session.active_id().is_none() {
            self.terminal.view.status = match action {
                StartupCommandAction::Duplicate => {
                    "select a session before duplicating with a command"
                }
                StartupCommandAction::Multiplex => {
                    "select an SSH session before multiplexing with a command"
                }
            }
            .to_string();
            cx.notify();
            return;
        }
        if action == StartupCommandAction::Multiplex
            && self
                .session
                .active_id()
                .and_then(|session_id| self.session.metadata(session_id))
                .is_none_or(|metadata| {
                    !matches!(metadata.launch_config, SessionLaunchConfig::Ssh(_))
                })
        {
            self.terminal.view.status = "active session is not SSH".to_string();
            cx.notify();
            return;
        }
        let delay_ms = u64::from(
            self.settings
                .summary
                .interaction_duplicate_session_command_delay_ms,
        );
        self.session.dialogs.open_startup_command(action, delay_ms);
        self.forget_text_inputs("session.startup-command");
        let field = self.text_input(
            "session.startup-command",
            "",
            TextInputSetup::placeholder(self.tr("tabCtx.commandRequired")),
            cx,
        );
        self.terminal.view.status = action.status_opened().to_string();
        window.focus(&field.read(cx).focus_handle());
        cx.notify();
    }

    pub(in crate::features) fn close_startup_command_dialog(&mut self, cx: &mut Context<Self>) {
        let action = self.session.dialogs.cancel_startup_command();
        self.forget_text_inputs("session.startup-command");
        self.terminal.view.status = action.status_cancelled().to_string();
        cx.notify();
    }

    pub(in crate::features) fn adjust_startup_command_delay(
        &mut self,
        delta_ms: i64,
        cx: &mut Context<Self>,
    ) {
        self.session.dialogs.adjust_startup_command_delay(delta_ms);
        cx.notify();
    }

    pub(in crate::features) fn submit_startup_command_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((action, startup_command)) = self.session.dialogs.take_startup_command() else {
            self.terminal.view.status = "startup command cannot be empty".to_string();
            cx.notify();
            return;
        };
        self.forget_text_inputs("session.startup-command");
        match action {
            StartupCommandAction::Duplicate => {
                self.duplicate_active_session_with_startup(Some(startup_command), window, cx);
            }
            StartupCommandAction::Multiplex => {
                self.multiplex_active_ssh_session_with_startup(Some(startup_command), window, cx);
            }
        }
    }

    pub(in crate::features) fn handle_startup_command_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }
        match keystroke.key.as_str() {
            "escape" => self.close_startup_command_dialog(cx),
            "enter" => self.submit_startup_command_dialog(window, cx),
            "up" => self.adjust_startup_command_delay(100, cx),
            "down" => self.adjust_startup_command_delay(-100, cx),
            _ => {}
        }
    }

    pub(in crate::features) fn apply_session_text_input(
        &mut self,
        field: &str,
        text: String,
        cx: &mut Context<Self>,
    ) {
        if !self.session.dialogs.apply_text_input(field, text) {
            return;
        }
        cx.notify();
    }

    pub(in crate::features) fn remove_session_state(&mut self, session_id: &str) {
        self.clear_terminal_mouse_report_for_session(session_id);
        self.session.start.clear_reconnect_failure(session_id);
        // If this leaf was a tab root, drop its pane tree (prune will rekey survivors).
        self.shell.workspace.remove_session(session_id);
        let multiplex_key = self.session.remove_session_catalog(session_id);
        self.session.clear_event_bridge_session(session_id);
        self.terminal.view.views.remove(session_id);
        self.remove_terminal_surface(session_id);
        self.terminal
            .view
            .frame_pipeline
            .remove_session(session_id.to_string());
        self.terminal
            .layout
            .session_surface_bounds
            .remove(session_id);
        self.transfer.browser.session_cache.remove(session_id);
        self.transfer
            .external_sync
            .prompts
            .retain(|_, prompt| prompt.session_id.as_deref() != Some(session_id));
        self.transfer
            .external_sync
            .windows
            .retain(|prompt_id, _| self.transfer.external_sync.prompts.contains_key(prompt_id));
        self.transfer
            .external_sync
            .window_open_pending
            .retain(|prompt_id| self.transfer.external_sync.prompts.contains_key(prompt_id));
        if self
            .transfer
            .file_ops
            .properties
            .as_ref()
            .is_some_and(|state| state.session_id.as_deref() == Some(session_id))
        {
            self.transfer.file_ops.properties = None;
        }
        if let Some(workspace) = self.transfer.editor.workspace.as_mut() {
            let active_removed = workspace
                .active_tab()
                .is_some_and(|tab| tab.session_id.as_deref() == Some(session_id));
            workspace
                .tabs
                .retain(|tab| tab.session_id.as_deref() != Some(session_id));
            if active_removed {
                workspace.active_tab_id = workspace
                    .tabs
                    .first()
                    .map(|tab| tab.id.clone())
                    .unwrap_or_default();
            }
            if workspace.tabs.is_empty() {
                self.transfer.editor.workspace = None;
            }
        }
        self.purge_session_from_sync_groups(session_id);
        self.reconcile_terminal_windows();
        if self.session.restore_is_complete() {
            self.persist_open_tabs();
        }
        if let Some(multiplex_key) = multiplex_key {
            if let Some(handle) = self
                .session
                .take_multiplex_handle_if_unreferenced(&multiplex_key)
            {
                super::disconnect_multiplex_handle(handle);
            }
        }
    }

    pub(in crate::features) fn next_session_after(&self, session_id: &str) -> Option<String> {
        // Local metadata includes live + disconnected tabs; no transport lock.
        self.session.next_session_after(session_id)
    }
}

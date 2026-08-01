use gpui::{Context, KeyDownEvent, Window};
use nyaterm_core::{ConnectionStore, QuickCommand, QuickCommandCategory, uuid};

use crate::features::{NyaTermApp, non_empty_string};
use crate::models::QuickCommandEditorField;

use super::helpers::unix_millis_now;

impl NyaTermApp {
    pub(in crate::features) fn focus_quick_command_editor_field(
        &mut self,
        field: QuickCommandEditorField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.commands.focus_quick_editor_field(field) {
            window.focus(self.commands.quick_editor_focus(), cx);
            cx.notify();
        }
    }

    pub(in crate::features) fn set_quick_command_editor_category(
        &mut self,
        category_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if self
            .commands
            .set_quick_editor_category(category_id, String::new())
        {
            cx.notify();
        }
    }

    pub(in crate::features) fn confirm_quick_command_editor_category_draft(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let Some(draft) = self
            .commands
            .quick_editor()
            .map(|editor| editor.category_draft.trim().to_string())
        else {
            return;
        };
        if draft.is_empty() {
            return;
        }
        let category_id = self
            .commands
            .quick_command_categories()
            .iter()
            .find(|category| category.name.eq_ignore_ascii_case(&draft))
            .map(|category| category.id.clone());
        if let Some(category_id) = category_id {
            self.commands
                .set_quick_editor_category(Some(category_id), String::new());
        } else {
            self.commands.set_quick_editor_category(None, draft);
        }
        cx.notify();
    }

    pub(in crate::features) fn set_quick_command_editor_color(
        &mut self,
        color_tag: Option<&'static str>,
        cx: &mut Context<Self>,
    ) {
        if self
            .commands
            .set_quick_editor_color(color_tag.map(ToOwned::to_owned))
        {
            cx.notify();
        }
    }

    pub(in crate::features) fn set_quick_command_editor_icon(
        &mut self,
        icon_tag: Option<&'static str>,
        cx: &mut Context<Self>,
    ) {
        if self
            .commands
            .set_quick_editor_icon(icon_tag.map(ToOwned::to_owned))
        {
            cx.notify();
        }
    }

    pub(in crate::features) fn toggle_quick_command_editor_pinned(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if self.commands.toggle_quick_editor_pinned() {
            cx.notify();
        }
    }

    pub(in crate::features) fn set_quick_command_editor_execution_mode(
        &mut self,
        mode: &'static str,
        cx: &mut Context<Self>,
    ) {
        if self.commands.set_quick_editor_execution_mode(mode) {
            cx.notify();
        }
    }

    pub(in crate::features) fn save_quick_command_editor(&mut self, cx: &mut Context<Self>) {
        let label_required = self.tr("quickCommands.errorLabelRequired").to_string();
        let command_required = self.tr("quickCommands.errorCommandRequired").to_string();
        let categories = self.commands.quick_command_categories().to_vec();
        let Some(editor) = self.commands.quick_editor_snapshot() else {
            return;
        };
        let label = editor.label.trim().to_string();
        let command_text = editor.command.trim().to_string();
        if label.is_empty() {
            self.commands
                .set_quick_editor_error(label_required, Some(QuickCommandEditorField::Label));
            cx.notify();
            return;
        }
        if command_text.is_empty() {
            self.commands
                .set_quick_editor_error(command_required, Some(QuickCommandEditorField::Command));
            cx.notify();
            return;
        }

        let now = unix_millis_now();
        let original = editor.original.clone();
        let category_draft = editor.category_draft.trim().to_string();
        let (category_id, new_category) = if category_draft.is_empty() {
            (editor.category_id.clone(), None)
        } else if let Some(existing) = categories
            .iter()
            .find(|category| category.name.eq_ignore_ascii_case(&category_draft))
        {
            (Some(existing.id.clone()), None)
        } else {
            let category = QuickCommandCategory {
                id: format!("quick-category-{}", uuid()),
                name: category_draft,
            };
            (Some(category.id.clone()), Some(category))
        };
        let command = QuickCommand {
            id: original
                .as_ref()
                .map(|command| command.id.clone())
                .unwrap_or_else(|| format!("qc-{}", uuid())),
            label,
            command: command_text,
            category_id,
            description: non_empty_string(editor.description.clone()),
            color_tag: editor.color_tag.clone(),
            icon_tag: editor.icon_tag.clone(),
            pinned: editor.pinned.then_some(true),
            execution_mode: Some(editor.execution_mode.clone()),
            source: original.as_ref().and_then(|command| command.source.clone()),
            risk_level: original
                .as_ref()
                .and_then(|command| command.risk_level.clone()),
            updated_at: Some(now),
            created_at: original
                .as_ref()
                .and_then(|command| command.created_at)
                .or(Some(now)),
            use_count: original.as_ref().and_then(|command| command.use_count),
        };

        match ConnectionStore::open_with_portable_key_path(
            self.runtime.config_dir(),
            self.runtime.portable_key_path().map(ToOwned::to_owned),
        )
        .and_then(|store| store.upsert_quick_command(command.clone(), new_category))
        {
            Ok(config) => {
                self.commands
                    .replace_quick_command_catalog(config.commands, config.categories);
                self.commands.close_quick_editor();
                self.settings
                    .update_store_status(format!("quick command '{}' saved", command.label), true);
                self.shell
                    .set_status(self.settings.store_status().message.to_string());
            }
            Err(error) => {
                self.commands
                    .set_quick_editor_error(error.to_string(), None);
                self.settings
                    .update_store_status(format!("quick command save failed: {error}"), false);
                self.shell
                    .set_status(self.settings.store_status().message.to_string());
            }
        }
        cx.notify();
    }

    pub(in crate::features) fn handle_quick_command_editor_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        let primary = keystroke.modifiers.platform || keystroke.modifiers.control;
        if primary && !keystroke.modifiers.alt && matches!(keystroke.key.as_str(), "s" | "S") {
            self.save_quick_command_editor(cx);
            return;
        }
        if primary || keystroke.modifiers.alt || keystroke.modifiers.function {
            return;
        }

        // The boxes own the text and the clipboard; the dialog owns the keys
        // that close or save it, which the boxes leave unconsumed. The script
        // box takes Enter itself, so an Enter arriving here is a save.
        match keystroke.key.as_str() {
            "escape" => self.close_quick_command_editor(cx),
            "enter" => self.save_quick_command_editor(cx),
            _ => {}
        }
    }

    /// Apply an edit from one of the quick command editor's inputs.
    pub(in crate::features) fn apply_quick_command_editor_input(
        &mut self,
        field: &str,
        text: String,
        cx: &mut Context<Self>,
    ) {
        let field = match field {
            "label" => QuickCommandEditorField::Label,
            "command" => QuickCommandEditorField::Command,
            "category" => QuickCommandEditorField::Category,
            "description" => QuickCommandEditorField::Description,
            _ => return,
        };
        if self.commands.apply_quick_editor_input(field, text) {
            cx.notify();
        }
    }
}

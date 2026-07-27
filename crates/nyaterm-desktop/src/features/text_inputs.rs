//! A registry of real text inputs, keyed by an id the caller picks.
//!
//! Most panels still draw their "inputs" as a label div over a draft string,
//! with a focus handle per box and a hand-written key handler. That pattern has
//! no caret, no selection, no IME and no clipboard, and every panel reimplements
//! a little of it.
//!
//! The connection editor solved this by owning one [`TextField`] entity per
//! field. Doing the same everywhere would mean threading a map through every
//! panel's state. Instead the fields live here, keyed by a string id, and are
//! created the first time a panel renders one. A panel needs no state of its
//! own beyond the value it already keeps, and edits arrive as one event with the
//! id attached.

use std::collections::HashMap;

use gpui::{
    App, AppContext as _, Context, Entity, InteractiveElement as _, IntoElement, MouseButton,
    ParentElement as _, SharedString, Styled as _, Subscription, div, prelude::FluentBuilder as _,
    px, rgb,
};
use nyaterm_ui::{TextField, TextFieldEvent};

use super::NyaTermApp;

/// How a field should behave, for the one call that creates it.
///
/// Only read when the field is first seen; later renders reuse the entity, so
/// changing this for an existing id has no effect until the id is forgotten.
#[derive(Default, Clone)]
pub(in crate::features) struct TextInputSetup {
    pub placeholder: SharedString,
    pub masked: bool,
    pub multi_line: bool,
}

impl TextInputSetup {
    pub fn placeholder(placeholder: impl Into<SharedString>) -> Self {
        Self {
            placeholder: placeholder.into(),
            ..Default::default()
        }
    }

    pub fn masked() -> Self {
        Self {
            masked: true,
            ..Default::default()
        }
    }

    pub fn multi_line(placeholder: impl Into<SharedString>) -> Self {
        Self {
            placeholder: placeholder.into(),
            masked: false,
            multi_line: true,
        }
    }
}

/// The setup for a settings field, masked when it holds a secret.
///
/// A stored secret is never read back into its box: the box holds the draft
/// that replaces it, and the panel badges whether one is stored at all.
pub(in crate::features) fn secret_input_setup(secret: bool) -> TextInputSetup {
    if secret {
        TextInputSetup::masked()
    } else {
        TextInputSetup::default()
    }
}

#[derive(Default)]
pub(in crate::features) struct TextInputRegistry {
    fields: HashMap<SharedString, Entity<TextField>>,
    /// Kept alive alongside its field, so edits keep arriving.
    subscriptions: HashMap<SharedString, Subscription>,
}

impl NyaTermApp {
    /// The input for `id`, created on first use and seeded with `seed`.
    ///
    /// After that the field owns its own text: `seed` is ignored, because the
    /// field is the source of truth for what is being typed. Use
    /// [`Self::reset_text_input`] to push a value back down, and
    /// [`Self::forget_text_inputs`] when the thing being edited goes away.
    pub(in crate::features) fn text_input(
        &mut self,
        id: impl Into<SharedString>,
        seed: &str,
        setup: TextInputSetup,
        cx: &mut Context<Self>,
    ) -> Entity<TextField> {
        let id = id.into();
        if let Some(field) = self.text_inputs.fields.get(&id) {
            return field.clone();
        }

        let entity = cx.new(|cx| {
            TextField::new(cx, seed)
                .placeholder(setup.placeholder)
                .masked(setup.masked)
                .multi_line(setup.multi_line)
        });
        let subscription_id = id.clone();
        let subscription = cx.subscribe(&entity, move |app: &mut NyaTermApp, _, event, cx| {
            let TextFieldEvent::Changed(text) = event;
            app.on_text_input_changed(subscription_id.clone(), text.clone(), cx);
        });
        self.text_inputs.fields.insert(id.clone(), entity.clone());
        self.text_inputs.subscriptions.insert(id, subscription);
        entity
    }

    /// A bordered box hosting the input for `id`.
    ///
    /// The box is the hit target, so clicking anywhere in it takes the caret —
    /// the text itself is only one line tall inside it.
    pub(in crate::features) fn text_input_box(
        &mut self,
        id: impl Into<SharedString>,
        seed: &str,
        setup: TextInputSetup,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let id = id.into();
        let palette = self.theme_palette();
        let multi_line = setup.multi_line;
        let field = self.text_input(id.clone(), seed, setup, cx);
        let handle = field.read(cx).focus_handle();
        let focused = field.read(cx).has_focus();
        div()
            .id(id)
            // A wrapped field asks for its parent's height, so the box needs a
            // definite one and has to stretch the row that holds it: against an
            // indefinite height the percentage resolves to zero and the field
            // disappears, hit-testing and all.
            .when_else(
                multi_line,
                |this| this.h(px(88.)).py_2(),
                |this| this.h(px(30.)).items_center(),
            )
            .min_w_0()
            .px_2()
            .flex()
            .rounded_sm()
            .border_1()
            .border_color(rgb(if focused {
                palette.primary
            } else {
                palette.border
            }))
            .bg(rgb(palette.input))
            .cursor_text()
            .on_mouse_down(MouseButton::Left, move |_, window, _| {
                window.focus(&handle);
            })
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .text_xs()
                    .text_color(rgb(palette.text))
                    .child(field),
            )
    }

    /// A caption above the input for `id`.
    ///
    /// The caption goes above rather than inside the box, so the whole width is
    /// what was typed — the same shape the connection editor settled on.
    pub(in crate::features) fn text_input_field(
        &mut self,
        id: impl Into<SharedString>,
        caption: impl Into<SharedString>,
        seed: &str,
        setup: TextInputSetup,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        let caption = caption.into();
        let input = self.text_input_box(id, seed, setup, cx);
        div()
            .min_w_0()
            .flex()
            .flex_col()
            .gap_1()
            .when(!caption.is_empty(), |this| {
                this.child(
                    div()
                        .text_xs()
                        .text_color(rgb(palette.text_muted))
                        .child(caption),
                )
            })
            .child(input)
    }

    /// Push a value the runtime changed back into its input.
    pub(in crate::features) fn reset_text_input(&mut self, id: &str, text: &str, cx: &mut App) {
        if let Some(field) = self.text_inputs.fields.get(id) {
            field.update(cx, |field, cx| field.set_content(text, cx));
        }
    }

    /// Route an edit to the panel that owns the id.
    ///
    /// Ids are dotted and start with the panel, so a panel claims a whole
    /// prefix: `settings.search-engine.<index>.name`. Anything unclaimed is
    /// ignored rather than panicking — a field can outlive one frame of the
    /// panel that made it.
    fn on_text_input_changed(&mut self, id: SharedString, text: String, cx: &mut Context<Self>) {
        if let Some(rest) = id.strip_prefix("settings.search-engine.") {
            self.apply_search_engine_input(rest, text, cx);
        } else if let Some(field) = id.strip_prefix("network.tunnel-editor.") {
            self.apply_network_tunnel_editor_input(field, text, cx);
        } else if let Some(field) = id.strip_prefix("network.proxy-editor.") {
            self.apply_network_proxy_editor_input(field, text, cx);
        } else if id.as_ref() == "network.group-editor.name" {
            self.apply_network_group_editor_name(text, cx);
        } else if id.as_ref() == "transfer.new-folder.name" {
            self.apply_transfer_new_folder_name(text, cx);
        } else if id.as_ref() == "transfer.new-file.name" {
            self.apply_transfer_new_file_name(text, cx);
        } else if id.as_ref() == "transfer.new-symlink.name" {
            self.apply_transfer_new_symlink_input(
                crate::models::TransferSymlinkField::Name,
                text,
                cx,
            );
        } else if id.as_ref() == "transfer.new-symlink.target" {
            self.apply_transfer_new_symlink_input(
                crate::models::TransferSymlinkField::Target,
                text,
                cx,
            );
        } else if id.as_ref() == "transfer.move.path" {
            self.apply_transfer_move_path(text, cx);
        } else if id.as_ref() == "transfer.browser.path" {
            self.apply_transfer_browser_path_input(text, cx);
        } else if id.starts_with("transfer.rename.") {
            self.apply_transfer_rename_input(text, cx);
        } else if let Some(field) = id.strip_prefix("quick-command.editor.") {
            self.apply_quick_command_editor_input(field, text, cx);
        } else if let Some(index) = id
            .strip_prefix("quick-command.variable.")
            .and_then(|index| index.parse::<usize>().ok())
        {
            self.apply_quick_command_variable(index, text, cx);
        } else if id.as_ref() == "quick-command.category-rename" {
            self.apply_quick_command_category_rename(text, cx);
        } else if id.as_ref() == "send-command.draft" {
            self.apply_send_command_draft(text, cx);
        } else if let Some(field) = id.strip_prefix("security.editor.") {
            self.apply_security_editor_input(field, text, cx);
        } else if id.as_ref() == "ai.chat.prompt" {
            self.apply_ai_prompt(text, cx);
        } else if id.as_ref() == "ai.model-search" {
            self.apply_ai_model_search(text, cx);
        } else if let Some(rest) = id.strip_prefix("ai.credential.") {
            self.apply_ai_credential_input(rest, text, cx);
        } else if let Some(field) = id
            .strip_prefix("ai.input.")
            .and_then(crate::models::AiInputField::from_input_key)
        {
            self.apply_ai_input(field, text, cx);
        } else if let Some(field) = id
            .strip_prefix("translation.input.")
            .and_then(crate::models::TranslateInputField::from_input_key)
        {
            self.apply_translate_input(field, text, cx);
        } else if let Some(field) = id
            .strip_prefix("cloud-sync.input.")
            .and_then(crate::models::CloudSyncInputField::from_input_key)
        {
            self.apply_cloud_sync_input(field, text, cx);
        } else if id.as_ref() == "sessions.filter" {
            self.apply_active_sessions_search(text, cx);
        } else if id.as_ref() == "remote.docker.filter" {
            self.apply_docker_search(text, cx);
        } else if id.as_ref() == "remote.process.filter" {
            self.apply_process_search(text, cx);
        } else if id.starts_with("remote.process.") && id.ends_with(".nice") {
            self.apply_process_nice_input(text, cx);
        } else if id.as_ref() == "settings.interaction.word-separators" {
            self.apply_interaction_word_separators(text, cx);
        } else if id.as_ref() == "settings.terminal.x11-display" {
            self.apply_terminal_x11_display(text, cx);
        } else if id.as_ref() == "settings.security.master-password" {
            self.apply_settings_master_password(text, cx);
        } else if id.as_ref() == "settings.recording.path" {
            self.apply_recording_path(text, cx);
        } else if id.as_ref() == "settings.transfer.download-path" {
            self.apply_transfer_download_path(text, cx);
        } else if id.as_ref() == "settings.transfer.default-editor" {
            self.apply_transfer_default_editor(text, cx);
        }
    }

    /// Drop every input whose id starts with `prefix`.
    ///
    /// Called when the thing being edited closes, so reopening it seeds fresh
    /// values rather than showing what was typed into the previous one.
    pub(in crate::features) fn forget_text_inputs(&mut self, prefix: &str) {
        self.text_inputs
            .fields
            .retain(|id, _| !id.starts_with(prefix));
        self.text_inputs
            .subscriptions
            .retain(|id, _| !id.starts_with(prefix));
    }
}

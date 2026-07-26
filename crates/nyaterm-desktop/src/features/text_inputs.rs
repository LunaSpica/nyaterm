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
            .when_else(
                multi_line,
                |this| this.min_h(px(72.)).py_2().items_start(),
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

    /// What the input for `id` currently holds, if it exists.
    pub(in crate::features) fn text_input_value(&self, id: &str, cx: &App) -> Option<String> {
        self.text_inputs
            .fields
            .get(id)
            .map(|field| field.read(cx).content().to_string())
    }

    /// Whether the input for `id` has the caret, as of the last frame.
    pub(in crate::features) fn text_input_focused(&self, id: &str, cx: &App) -> bool {
        self.text_inputs
            .fields
            .get(id)
            .is_some_and(|field| field.read(cx).has_focus())
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

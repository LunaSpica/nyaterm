use gpui::{
    Action as _, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, Render, RenderOnce, SharedString, Subscription, Window,
};
use gpui_component::Sizable;
use gpui_component::input::SelectAll;
use gpui_component::input::{Input, InputEvent, InputState};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NyaInputEvent {
    Changed(String),
    Submitted(String),
}

pub struct NyaInputState {
    state: Option<Entity<InputState>>,
    seed: SharedString,
    pending_value: Option<SharedString>,
    placeholder: SharedString,
    masked: bool,
    multi_line: bool,
    rows: Option<usize>,
    disabled: bool,
    readonly: bool,
    error: bool,
    max_chars: Option<usize>,
    focus: FocusHandle,
    focused: bool,
    subscription: Option<Subscription>,
}

impl NyaInputState {
    pub fn new(cx: &mut Context<Self>, seed: impl Into<SharedString>) -> Self {
        Self {
            state: None,
            seed: seed.into(),
            pending_value: None,
            placeholder: SharedString::default(),
            masked: false,
            multi_line: false,
            rows: None,
            disabled: false,
            readonly: false,
            error: false,
            max_chars: None,
            focus: cx.focus_handle(),
            focused: false,
            subscription: None,
        }
    }

    pub fn single_line(cx: &mut Context<Self>, seed: impl Into<SharedString>) -> Self {
        Self::new(cx, seed)
    }

    pub fn multi_line(mut self, rows: Option<usize>) -> Self {
        self.multi_line = true;
        self.rows = rows;
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn readonly(mut self, readonly: bool) -> Self {
        self.readonly = readonly;
        self
    }

    pub fn error(mut self, error: bool) -> Self {
        self.error = error;
        self
    }

    pub fn max_chars(mut self, max_chars: Option<usize>) -> Self {
        self.max_chars = max_chars;
        self
    }

    pub fn value(&self, cx: &App) -> String {
        if let Some(state) = &self.state {
            state.read(cx).value().to_string()
        } else if let Some(value) = &self.pending_value {
            value.to_string()
        } else {
            self.seed.to_string()
        }
    }

    pub fn set_content(&mut self, text: &str, cx: &mut Context<Self>) {
        self.pending_value = Some(SharedString::from(text.to_string()));
        cx.notify();
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.set_content("", cx);
    }

    pub fn select_all(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let state = self.ensure_component(window, cx);
        state.update(cx, |state, cx| state.focus(window, cx));
        window.dispatch_action(SelectAll.boxed_clone(), cx);
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    pub fn component_focus_handle(&self, cx: &App) -> FocusHandle {
        self.state
            .as_ref()
            .map(|state| state.read(cx).focus_handle(cx))
            .unwrap_or_else(|| self.focus.clone())
    }

    pub fn has_focus(&self) -> bool {
        self.focused
    }

    pub fn component_state(&self) -> Option<Entity<InputState>> {
        self.state.clone()
    }

    fn ensure_component(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        if let Some(state) = self.state.clone() {
            if let Some(value) = self.pending_value.take() {
                state.update(cx, |state, cx| state.set_value(value, window, cx));
            }
            return state;
        }

        let value = self
            .pending_value
            .take()
            .unwrap_or_else(|| self.seed.clone());
        let masked = self.masked;
        let multi_line = self.multi_line;
        let placeholder = component_placeholder(self.placeholder.clone(), multi_line);
        let rows = self.rows;
        let state = cx.new(|cx| {
            let mut input = InputState::new(window, cx)
                .default_value(value)
                .placeholder(placeholder)
                .multi_line(multi_line);
            if let Some(rows) = rows {
                input = input.rows(rows);
            }
            if masked && !multi_line {
                input = input.masked(true);
            }
            input
        });
        let subscription = cx.subscribe(&state, |_, input, event: &InputEvent, cx| match event {
            InputEvent::Change => {
                cx.emit(NyaInputEvent::Changed(input.read(cx).value().to_string()))
            }
            InputEvent::PressEnter { .. } => {
                cx.emit(NyaInputEvent::Submitted(input.read(cx).value().to_string()))
            }
            InputEvent::Focus | InputEvent::Blur => {}
        });
        self.subscription = Some(subscription);
        self.state = Some(state.clone());
        state
    }
}

fn component_placeholder(placeholder: SharedString, multi_line: bool) -> SharedString {
    if !multi_line || (!placeholder.contains('\r') && !placeholder.contains('\n')) {
        return placeholder;
    }

    let mut normalized = String::with_capacity(placeholder.len());
    let mut in_line_break = false;
    for ch in placeholder.chars() {
        if matches!(ch, '\r' | '\n') {
            if !in_line_break {
                normalized.push(' ');
                in_line_break = true;
            }
        } else {
            normalized.push(ch);
            in_line_break = false;
        }
    }
    SharedString::from(normalized)
}

impl EventEmitter<NyaInputEvent> for NyaInputState {}

impl Focusable for NyaInputState {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.component_focus_handle(cx)
    }
}

impl Render for NyaInputState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.ensure_component(window, cx);
        let component_focus = state.read(cx).focus_handle(cx);
        if self.focus.is_focused(window) && !component_focus.is_focused(window) {
            state.update(cx, |state, cx| state.focus(window, cx));
        }
        self.focused = self.focus.is_focused(window) || component_focus.is_focused(window);
        let input = Input::new(&state).small().disabled(self.disabled);
        if self.multi_line {
            input.h_full()
        } else {
            input
        }
    }
}

#[derive(IntoElement)]
pub struct NyaInput {
    state: Entity<NyaInputState>,
}

impl NyaInput {
    pub fn new(state: &Entity<NyaInputState>) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl RenderOnce for NyaInput {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let (state, disabled, multi_line, focused) = self.state.update(cx, |state, cx| {
            (
                state.ensure_component(window, cx),
                state.disabled,
                state.multi_line,
                state.focus.is_focused(window),
            )
        });
        if focused {
            state.update(cx, |state, cx| state.focus(window, cx));
        }
        let input = Input::new(&state)
            .small()
            .appearance(false)
            .bordered(false)
            .focus_bordered(false)
            .disabled(disabled);
        if multi_line { input.h_full() } else { input }
    }
}

pub type NyaTextArea = NyaInput;

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};

    use super::{NyaInputState, component_placeholder};

    #[test]
    fn value_tracks_seed_reset_and_clear_before_component_renders() {
        let mut cx = TestAppContext::single();
        let field = cx.new(|cx| NyaInputState::new(cx, "seed").placeholder("Name"));

        assert_eq!(cx.read_entity(&field, |field, cx| field.value(cx)), "seed");

        field.update(&mut cx, |field, cx| field.set_content("reset", cx));
        assert_eq!(cx.read_entity(&field, |field, cx| field.value(cx)), "reset");

        field.update(&mut cx, |field, cx| field.clear(cx));
        assert_eq!(cx.read_entity(&field, |field, cx| field.value(cx)), "");
    }

    #[test]
    fn multiline_placeholder_collapses_line_breaks() {
        let placeholder = component_placeholder("first\nsecond".into(), true);

        assert_eq!(placeholder.as_ref(), "first second");
    }

    #[test]
    fn multiline_placeholder_collapses_crlf_and_blank_lines() {
        let placeholder = component_placeholder("first\r\n\r\nsecond\n\nthird".into(), true);

        assert_eq!(placeholder.as_ref(), "first second third");
        assert!(!placeholder.contains('\r'));
        assert!(!placeholder.contains('\n'));
    }

    #[test]
    fn single_line_placeholder_is_preserved() {
        let placeholder = component_placeholder("first\nsecond".into(), false);

        assert_eq!(placeholder.as_ref(), "first\nsecond");
    }

    #[test]
    fn multiline_placeholder_preserves_multibyte_text() {
        let placeholder =
            component_placeholder("例如：ls -la\n使用 {{变量名}} 注入动态参数。".into(), true);

        assert_eq!(
            placeholder.as_ref(),
            "例如：ls -la 使用 {{变量名}} 注入动态参数。"
        );
    }
}

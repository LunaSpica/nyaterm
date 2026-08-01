use gpui::{
    AnyElement, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    IntoElement, ParentElement as _, Render, RenderOnce, SharedString, Styled as _, Subscription,
    Window, div, prelude::FluentBuilder as _,
};
use gpui_component::{
    Disableable, IndexPath, Sizable,
    checkbox::Checkbox,
    radio::RadioGroup,
    select::{SearchableVec, Select, SelectEvent, SelectState},
    switch::Switch,
};

use crate::sizing::{form_control_height, form_control_size};

type NyaToggleHandler = Box<dyn Fn(&bool, &mut Window, &mut App)>;
type NyaIndexSelectHandler = Box<dyn Fn(&usize, &mut Window, &mut App)>;

#[derive(IntoElement)]
pub struct NyaSwitch {
    id: SharedString,
    checked: bool,
    disabled: bool,
    tooltip: Option<SharedString>,
    on_click: Option<NyaToggleHandler>,
}

impl NyaSwitch {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            checked: false,
            disabled: false,
            tooltip: None,
            on_click: None,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for NyaSwitch {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut switch = Switch::new(self.id).checked(self.checked).small();
        if let Some(tooltip) = self.tooltip {
            switch = switch.tooltip(tooltip);
        }
        if let Some(on_click) = self.on_click {
            switch = switch.on_click(on_click);
        }
        switch.disabled(self.disabled)
    }
}

#[derive(IntoElement)]
pub struct NyaCheckbox {
    id: SharedString,
    label: Option<SharedString>,
    checked: bool,
    disabled: bool,
    on_click: Option<NyaToggleHandler>,
}

impl NyaCheckbox {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: None,
            checked: false,
            disabled: false,
            on_click: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for NyaCheckbox {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut checkbox = Checkbox::new(self.id).checked(self.checked).small();
        if let Some(label) = self.label {
            checkbox = checkbox.label(label);
        }
        if let Some(on_click) = self.on_click {
            checkbox = checkbox.on_click(on_click);
        }
        checkbox.disabled(self.disabled)
    }
}

#[derive(IntoElement)]
pub struct NyaRadioGroup {
    id: SharedString,
    items: Vec<SharedString>,
    selected_index: Option<usize>,
    horizontal: bool,
    disabled: bool,
    on_select: Option<NyaIndexSelectHandler>,
}

impl NyaRadioGroup {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            selected_index: None,
            horizontal: false,
            disabled: false,
            on_select: None,
        }
    }

    pub fn items(mut self, items: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.items = items.into_iter().map(Into::into).collect();
        self
    }

    pub fn selected_index(mut self, selected_index: Option<usize>) -> Self {
        self.selected_index = selected_index;
        self
    }

    pub fn horizontal(mut self) -> Self {
        self.horizontal = true;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_select(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for NyaRadioGroup {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut group = if self.horizontal {
            RadioGroup::horizontal(self.id)
        } else {
            RadioGroup::vertical(self.id)
        }
        .selected_index(self.selected_index)
        .disabled(self.disabled)
        .children(self.items);
        if let Some(on_select) = self.on_select {
            group = group.on_click(on_select);
        }
        group
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NyaSelectOption {
    value: String,
    label: SharedString,
    font_family: Option<SharedString>,
}

impl NyaSelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<SharedString>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            font_family: None,
        }
    }

    pub fn font_family(mut self, font_family: impl Into<SharedString>) -> Self {
        self.font_family = Some(font_family.into());
        self
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn label(&self) -> &SharedString {
        &self.label
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NyaSelectItem {
    value: String,
    label: SharedString,
    font_family: Option<SharedString>,
}

impl gpui_component::select::SelectItem for NyaSelectItem {
    type Value = String;

    fn title(&self) -> SharedString {
        self.label.clone()
    }

    fn display_title(&self) -> Option<AnyElement> {
        self.font_family.as_ref().map(|font_family| {
            div()
                .font_family(font_family.clone())
                .child(self.label.clone())
                .into_any_element()
        })
    }

    fn render(&self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .when_some(self.font_family.clone(), |this, font_family| {
                this.font_family(font_family)
            })
            .child(self.label.clone())
    }

    fn value(&self) -> &Self::Value {
        &self.value
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NyaSelectEvent {
    Changed(Option<String>),
}

pub struct NyaSelectState {
    state: Option<Entity<SelectState<SearchableVec<NyaSelectItem>>>>,
    options: Vec<NyaSelectOption>,
    selected_value: Option<String>,
    placeholder: SharedString,
    disabled: bool,
    searchable: bool,
    options_dirty: bool,
    focus: FocusHandle,
    subscription: Option<Subscription>,
}

impl NyaSelectState {
    pub fn new(
        cx: &mut Context<Self>,
        options: impl Into<Vec<NyaSelectOption>>,
        selected_value: Option<String>,
    ) -> Self {
        Self {
            state: None,
            options: options.into(),
            selected_value,
            placeholder: SharedString::default(),
            disabled: false,
            searchable: false,
            options_dirty: false,
            focus: cx.focus_handle(),
            subscription: None,
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self
    }

    pub fn selected_value(&self) -> Option<&str> {
        self.selected_value.as_deref()
    }

    pub fn set_options(
        &mut self,
        options: impl Into<Vec<NyaSelectOption>>,
        cx: &mut Context<Self>,
    ) {
        let options = options.into();
        if self.options != options {
            self.options = options;
            self.options_dirty = self.state.is_some();
            cx.notify();
        }
    }

    pub fn set_placeholder(
        &mut self,
        placeholder: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let placeholder = placeholder.into();
        if self.placeholder != placeholder {
            self.placeholder = placeholder;
            cx.notify();
        }
    }

    pub fn set_selected_value(&mut self, selected_value: Option<String>, cx: &mut Context<Self>) {
        if self.selected_value != selected_value {
            self.selected_value = selected_value;
            cx.notify();
        }
    }

    pub fn set_disabled(&mut self, disabled: bool, cx: &mut Context<Self>) {
        if self.disabled != disabled {
            self.disabled = disabled;
            cx.notify();
        }
    }

    pub fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.state
            .as_ref()
            .map(|state| state.read(cx).focus_handle(cx))
            .unwrap_or_else(|| self.focus.clone())
    }

    fn items(&self) -> Vec<NyaSelectItem> {
        self.options
            .iter()
            .map(|option| NyaSelectItem {
                value: option.value.clone(),
                label: option.label.clone(),
                font_family: option.font_family.clone(),
            })
            .collect()
    }

    fn selected_index(&self) -> Option<IndexPath> {
        let selected = self.selected_value.as_ref()?;
        self.options
            .iter()
            .position(|option| &option.value == selected)
            .map(|row| IndexPath::default().row(row))
    }

    fn sync_selected_value(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(state) = self.state.clone() {
            if let Some(value) = self.selected_value.clone() {
                state.update(cx, |state, cx| state.set_selected_value(&value, window, cx));
            } else {
                state.update(cx, |state, cx| state.set_selected_index(None, window, cx));
            }
        }
    }

    fn sync_component(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.options_dirty {
            if let Some(state) = self.state.clone() {
                let items = SearchableVec::new(self.items());
                state.update(cx, |state, cx| state.set_items(items, window, cx));
            }
            self.options_dirty = false;
        }
        self.sync_selected_value(window, cx);
    }

    fn ensure_component(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<SelectState<SearchableVec<NyaSelectItem>>> {
        if let Some(state) = self.state.clone() {
            return state;
        }

        let items = SearchableVec::new(self.items());
        let selected_index = self.selected_index();
        let searchable = self.searchable;
        let state =
            cx.new(|cx| SelectState::new(items, selected_index, window, cx).searchable(searchable));
        let subscription = cx.subscribe(
            &state,
            |this: &mut Self, _, event: &SelectEvent<SearchableVec<NyaSelectItem>>, cx| match event
            {
                SelectEvent::Confirm(value) => {
                    this.selected_value = value.clone();
                    cx.emit(NyaSelectEvent::Changed(value.clone()));
                }
            },
        );
        self.subscription = Some(subscription);
        self.state = Some(state.clone());
        self.options_dirty = false;
        state
    }
}

impl EventEmitter<NyaSelectEvent> for NyaSelectState {}

impl Focusable for NyaSelectState {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        NyaSelectState::focus_handle(self, cx)
    }
}

impl Render for NyaSelectState {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.ensure_component(window, cx);
        self.sync_component(window, cx);
        Select::new(&state)
            .with_size(form_control_size())
            .h(form_control_height())
            .placeholder(self.placeholder.clone())
            .disabled(self.disabled)
    }
}

#[derive(IntoElement)]
pub struct NyaSelect {
    state: Entity<NyaSelectState>,
    appearance: bool,
}

impl NyaSelect {
    pub fn new(state: &Entity<NyaSelectState>) -> Self {
        Self {
            state: state.clone(),
            appearance: true,
        }
    }

    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }
}

impl RenderOnce for NyaSelect {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self { state, appearance } = self;
        let (state, placeholder, disabled) = state.update(cx, |state, cx| {
            let component = state.ensure_component(window, cx);
            state.sync_component(window, cx);
            (component, state.placeholder.clone(), state.disabled)
        });
        Select::new(&state)
            .with_size(form_control_size())
            .h(form_control_height())
            .appearance(appearance)
            .placeholder(placeholder)
            .disabled(disabled)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};

    use super::{NyaSelectOption, NyaSelectState};
    use crate::sizing::{NYA_FORM_CONTROL_HEIGHT_PX, form_control_size};

    #[test]
    fn selected_value_tracks_pre_render_updates() {
        let mut cx = TestAppContext::single();
        let select = cx.new(|cx| {
            NyaSelectState::new(
                cx,
                vec![
                    NyaSelectOption::new("light", "Light"),
                    NyaSelectOption::new("dark", "Dark"),
                ],
                Some("light".to_string()),
            )
        });

        assert_eq!(
            cx.read_entity(&select, |select, _| select
                .selected_value()
                .map(str::to_string)),
            Some("light".to_string())
        );

        select.update(&mut cx, |select, cx| {
            select.set_selected_value(Some("dark".to_string()), cx);
        });
        assert_eq!(
            cx.read_entity(&select, |select, _| select
                .selected_value()
                .map(str::to_string)),
            Some("dark".to_string())
        );
    }

    #[test]
    fn select_uses_standard_form_control_size() {
        assert_eq!(NYA_FORM_CONTROL_HEIGHT_PX, 32.);
        assert_eq!(form_control_size(), gpui_component::Size::Medium);
    }
}

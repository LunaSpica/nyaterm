use std::rc::Rc;

use gpui::{AnyElement, App, IntoElement, ParentElement, RenderOnce, SharedString, Window};
use gpui_component::{Selectable, popover::Popover};

type NyaPopoverOpenHandler = Rc<dyn Fn(&bool, &mut Window, &mut App)>;

#[derive(IntoElement)]
struct NyaPopoverTrigger {
    element: AnyElement,
    selected: bool,
}

impl Selectable for NyaPopoverTrigger {
    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.selected
    }
}

impl RenderOnce for NyaPopoverTrigger {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        self.element
    }
}

#[derive(IntoElement)]
pub struct NyaPopover {
    id: SharedString,
    trigger: AnyElement,
    content: AnyElement,
    open: Option<bool>,
    appearance: bool,
    overlay_closable: bool,
    on_open_change: Option<NyaPopoverOpenHandler>,
}

impl NyaPopover {
    pub fn new(
        id: impl Into<SharedString>,
        trigger: impl IntoElement,
        content: impl IntoElement,
    ) -> Self {
        Self {
            id: id.into(),
            trigger: trigger.into_any_element(),
            content: content.into_any_element(),
            open: None,
            appearance: true,
            overlay_closable: true,
            on_open_change: None,
        }
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }

    pub fn appearance(mut self, appearance: bool) -> Self {
        self.appearance = appearance;
        self
    }

    pub fn overlay_closable(mut self, closable: bool) -> Self {
        self.overlay_closable = closable;
        self
    }

    pub fn on_open_change(
        mut self,
        handler: impl Fn(&bool, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for NyaPopover {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let trigger = NyaPopoverTrigger {
            element: self.trigger,
            selected: false,
        };
        let mut popover = Popover::new(self.id)
            .trigger(trigger)
            .appearance(self.appearance)
            .overlay_closable(self.overlay_closable);
        if let Some(open) = self.open {
            popover = popover.open(open);
        }
        if let Some(on_open_change) = self.on_open_change {
            popover =
                popover.on_open_change(move |open, window, cx| on_open_change(open, window, cx));
        }
        popover.child(self.content)
    }
}

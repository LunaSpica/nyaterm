use gpui::{App, IntoElement, RenderOnce, SharedString, Window, div, prelude::*};
use gpui_component::tab::{Tab, TabBar};

type NyaTabSelectHandler = Box<dyn Fn(&usize, &mut Window, &mut App)>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NyaTabsVariant {
    Segmented,
    Pill,
    Underline,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NyaTabItem {
    label: SharedString,
    disabled: bool,
}

impl NyaTabItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        Self {
            label: label.into(),
            disabled: false,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

#[derive(IntoElement)]
pub struct NyaTabs {
    id: SharedString,
    items: Vec<NyaTabItem>,
    selected_index: usize,
    variant: NyaTabsVariant,
    full_width: bool,
    on_select: Option<NyaTabSelectHandler>,
}

impl NyaTabs {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            items: Vec::new(),
            selected_index: 0,
            variant: NyaTabsVariant::Segmented,
            full_width: true,
            on_select: None,
        }
    }

    pub fn item(mut self, item: NyaTabItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = NyaTabItem>) -> Self {
        self.items.extend(items);
        self
    }

    pub fn selected_index(mut self, selected_index: usize) -> Self {
        self.selected_index = selected_index;
        self
    }

    pub fn variant(mut self, variant: NyaTabsVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn full_width(mut self, full_width: bool) -> Self {
        self.full_width = full_width;
        self
    }

    pub fn on_select(mut self, handler: impl Fn(&usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Box::new(handler));
        self
    }
}

impl RenderOnce for NyaTabs {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut tabs = TabBar::new(self.id).selected_index(self.selected_index);
        tabs = match self.variant {
            NyaTabsVariant::Segmented => tabs.segmented(),
            NyaTabsVariant::Pill => tabs.pill(),
            NyaTabsVariant::Underline => tabs.underline(),
        };
        if self.full_width {
            tabs = tabs.w_full();
        }
        if let Some(on_select) = self.on_select {
            tabs = tabs.on_click(move |index, window, cx| on_select(index, window, cx));
        }
        tabs.children(self.items.into_iter().map(|item| {
            Tab::new()
                .label(item.label)
                .disabled(item.disabled)
                .flex_1()
                .min_w_0()
        }))
        .last_empty_space(div())
    }
}

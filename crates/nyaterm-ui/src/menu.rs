use std::rc::Rc;

use gpui::{
    Anchor, App, ClickEvent, InteractiveElement, IntoElement, ParentElement, Pixels, RenderOnce,
    SharedString, Styled, Window, div, prelude::FluentBuilder as _,
};
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::menu::{ContextMenuExt as _, DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_component::{ActiveTheme as _, Disableable as _, Icon, Selectable as _, Sizable as _};

type MenuClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NyaMenuAnchor {
    TopLeft,
    #[default]
    TopRight,
    BottomLeft,
    BottomRight,
}

impl NyaMenuAnchor {
    fn component_anchor(self) -> Anchor {
        match self {
            Self::TopLeft => Anchor::TopLeft,
            Self::TopRight => Anchor::TopRight,
            Self::BottomLeft => Anchor::BottomLeft,
            Self::BottomRight => Anchor::BottomRight,
        }
    }
}

#[derive(Clone)]
enum NyaMenuItemKind {
    Action,
    Label,
    Separator,
    Submenu(Vec<NyaMenuItem>),
}

#[derive(Clone)]
pub struct NyaMenuItem {
    kind: NyaMenuItemKind,
    label: SharedString,
    icon_path: Option<SharedString>,
    shortcut: Option<SharedString>,
    disabled: bool,
    checked: bool,
    danger: bool,
    on_click: Option<MenuClickHandler>,
}

impl NyaMenuItem {
    pub fn action(label: impl Into<SharedString>) -> Self {
        Self {
            kind: NyaMenuItemKind::Action,
            label: label.into(),
            icon_path: None,
            shortcut: None,
            disabled: false,
            checked: false,
            danger: false,
            on_click: None,
        }
    }

    pub fn label(label: impl Into<SharedString>) -> Self {
        Self {
            kind: NyaMenuItemKind::Label,
            ..Self::action(label)
        }
    }

    pub fn separator() -> Self {
        Self {
            kind: NyaMenuItemKind::Separator,
            ..Self::action("")
        }
    }

    pub fn submenu(label: impl Into<SharedString>, items: Vec<Self>) -> Self {
        Self {
            kind: NyaMenuItemKind::Submenu(items),
            ..Self::action(label)
        }
    }

    pub fn icon(mut self, icon_path: impl Into<SharedString>) -> Self {
        self.icon_path = Some(icon_path.into());
        self
    }

    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    fn append_to(
        &self,
        menu: PopupMenu,
        window: &mut Window,
        cx: &mut gpui::Context<PopupMenu>,
    ) -> PopupMenu {
        match &self.kind {
            NyaMenuItemKind::Separator => menu.separator(),
            NyaMenuItemKind::Label => menu.label(self.label.clone()),
            NyaMenuItemKind::Action => menu.item(self.popup_item()),
            NyaMenuItemKind::Submenu(items) => {
                let items = items.clone();
                menu.submenu_with_icon(
                    self.component_icon(),
                    self.label.clone(),
                    window,
                    cx,
                    move |menu, window, cx| {
                        items
                            .iter()
                            .fold(menu, |menu, item| item.append_to(menu, window, cx))
                    },
                )
            }
        }
    }

    fn popup_item(&self) -> PopupMenuItem {
        let mut item = if self.danger || self.shortcut.is_some() {
            let label = self.label.clone();
            let shortcut = self.shortcut.clone();
            let danger = self.danger;
            PopupMenuItem::element(move |_, cx| {
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .text_color(if danger {
                        cx.theme().danger
                    } else {
                        cx.theme().foreground
                    })
                    .child(div().min_w_0().flex_1().child(label.clone()))
                    .when_some(shortcut.clone(), |this, shortcut| {
                        this.child(
                            div()
                                .flex_none()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(shortcut),
                        )
                    })
            })
        } else {
            PopupMenuItem::new(self.label.clone())
        };

        item = item
            .when_some(self.component_icon(), |item, icon| item.icon(icon))
            .disabled(self.disabled)
            .checked(self.checked);
        if let Some(on_click) = self.on_click.clone() {
            item = item.on_click(move |event, window, cx| on_click(event, window, cx));
        }
        item
    }

    fn component_icon(&self) -> Option<Icon> {
        self.icon_path
            .clone()
            .map(|path| Icon::default().path(path))
    }
}

#[derive(IntoElement)]
pub struct NyaDropdownMenu {
    id: SharedString,
    label: Option<SharedString>,
    icon_path: Option<SharedString>,
    icon_size: Option<Pixels>,
    tooltip: Option<SharedString>,
    selected: bool,
    disabled: bool,
    anchor: NyaMenuAnchor,
    min_width: Option<Pixels>,
    max_width: Option<Pixels>,
    max_height: Option<Pixels>,
    scrollable: bool,
    items: Vec<NyaMenuItem>,
    on_trigger: Option<MenuClickHandler>,
}

impl NyaDropdownMenu {
    pub fn new(id: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: None,
            icon_path: None,
            icon_size: None,
            tooltip: None,
            selected: false,
            disabled: false,
            anchor: NyaMenuAnchor::default(),
            min_width: None,
            max_width: None,
            max_height: None,
            scrollable: false,
            items: Vec::new(),
            on_trigger: None,
        }
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn icon(mut self, icon_path: impl Into<SharedString>) -> Self {
        self.icon_path = Some(icon_path.into());
        self
    }

    pub fn icon_size(mut self, size: Pixels) -> Self {
        self.icon_size = Some(size);
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn anchor(mut self, anchor: NyaMenuAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    pub fn min_width(mut self, width: Pixels) -> Self {
        self.min_width = Some(width);
        self
    }

    pub fn max_width(mut self, width: Pixels) -> Self {
        self.max_width = Some(width);
        self
    }

    pub fn max_height(mut self, height: Pixels) -> Self {
        self.max_height = Some(height);
        self
    }

    pub fn scrollable(mut self, scrollable: bool) -> Self {
        self.scrollable = scrollable;
        self
    }

    pub fn items(mut self, items: impl IntoIterator<Item = NyaMenuItem>) -> Self {
        self.items = items.into_iter().collect();
        self
    }

    pub fn on_trigger(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_trigger = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for NyaDropdownMenu {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let mut trigger = Button::new(self.id).ghost().small();
        if let Some(label) = self.label {
            trigger = trigger.label(label);
        }
        if let Some(icon_path) = self.icon_path {
            let icon = Icon::default()
                .path(icon_path)
                .when_some(self.icon_size, |icon, size| icon.with_size(size));
            trigger = trigger.icon(icon);
        }
        if let Some(tooltip) = self.tooltip {
            trigger = trigger.tooltip(tooltip);
        }
        if let Some(on_trigger) = self.on_trigger {
            trigger = trigger.on_click(move |event, window, cx| on_trigger(event, window, cx));
        }
        trigger = trigger.selected(self.selected);

        let items = self.items;
        let min_width = self.min_width;
        let max_width = self.max_width;
        let max_height = self.max_height;
        let scrollable = self.scrollable;
        trigger.disabled(self.disabled).dropdown_menu_with_anchor(
            self.anchor.component_anchor(),
            move |menu, window, cx| {
                let menu = menu
                    .when_some(min_width, |menu, width| menu.min_w(width))
                    .when_some(max_width, |menu, width| menu.max_w(width))
                    .when_some(max_height, |menu, height| menu.max_h(height))
                    .scrollable(scrollable);
                items
                    .iter()
                    .fold(menu, |menu, item| item.append_to(menu, window, cx))
            },
        )
    }
}

#[derive(IntoElement)]
pub struct NyaContextMenu<E>
where
    E: InteractiveElement + ParentElement + Styled + IntoElement + 'static,
{
    element: E,
    items: Vec<NyaMenuItem>,
    enabled: bool,
}

impl<E> NyaContextMenu<E>
where
    E: InteractiveElement + ParentElement + Styled + IntoElement + 'static,
{
    pub fn new(element: E, items: impl IntoIterator<Item = NyaMenuItem>) -> Self {
        Self {
            element,
            items: items.into_iter().collect(),
            enabled: true,
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

impl<E> RenderOnce for NyaContextMenu<E>
where
    E: InteractiveElement + ParentElement + Styled + IntoElement + 'static,
{
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        if !self.enabled {
            return self.element.into_any_element();
        }
        let items = self.items;
        self.element
            .context_menu(move |menu, window, cx| {
                items
                    .iter()
                    .fold(menu, |menu, item| item.append_to(menu, window, cx))
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{NyaMenuItem, NyaMenuItemKind};

    #[test]
    fn menu_item_builders_preserve_behavior_flags() {
        let item = NyaMenuItem::action("Delete")
            .icon("icons/net/delete.svg")
            .disabled(true)
            .checked(true)
            .danger();

        assert!(matches!(item.kind, NyaMenuItemKind::Action));
        assert_eq!(item.label.as_ref(), "Delete");
        assert_eq!(
            item.icon_path.as_ref().map(|path| path.as_ref()),
            Some("icons/net/delete.svg")
        );
        assert_eq!(item.shortcut, None);
        assert!(item.disabled);
        assert!(item.checked);
        assert!(item.danger);
    }

    #[test]
    fn submenu_retains_nested_items() {
        let item = NyaMenuItem::submenu(
            "Move",
            vec![NyaMenuItem::action("Group A"), NyaMenuItem::separator()],
        );

        let NyaMenuItemKind::Submenu(items) = item.kind else {
            panic!("expected submenu");
        };
        assert_eq!(items.len(), 2);
    }
}

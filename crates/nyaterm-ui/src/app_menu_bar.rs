use std::rc::Rc;

use gpui::{
    Anchor, App, AppContext, ClickEvent, Context, DismissEvent, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, MouseButton, ParentElement, Pixels, Render,
    Role, SharedString, StatefulInteractiveElement, Styled, Subscription, Window, anchored,
    deferred, div, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    Selectable as _, Sizable as _,
    button::{Button, ButtonVariants as _},
    menu::PopupMenu,
};

use crate::NyaMenuItem;

const KEY_CONTEXT: &str = "NyaAppMenuBar";

type MenuLabelBuilder = Rc<dyn Fn(&App) -> SharedString>;
type MenuItemsBuilder = Rc<dyn Fn(&mut Window, &mut App) -> Vec<NyaMenuItem>>;
type MenuOpenHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// A top-level application menu with lazily-built menu contents.
///
/// The label is evaluated while the bar renders, while items are built only
/// when the menu is opened. This keeps translated labels and checked state in
/// sync without publishing menu snapshots during a render pass.
#[derive(Clone)]
pub struct NyaAppMenu {
    id: SharedString,
    label: MenuLabelBuilder,
    items: MenuItemsBuilder,
    on_open: Option<MenuOpenHandler>,
    min_width: Option<Pixels>,
}

impl NyaAppMenu {
    pub fn new(
        id: impl Into<SharedString>,
        label: impl Fn(&App) -> SharedString + 'static,
        items: impl Fn(&mut Window, &mut App) -> Vec<NyaMenuItem> + 'static,
    ) -> Self {
        Self {
            id: id.into(),
            label: Rc::new(label),
            items: Rc::new(items),
            on_open: None,
            min_width: None,
        }
    }

    pub fn on_open(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }

    pub fn min_width(mut self, width: Pixels) -> Self {
        self.min_width = Some(width);
        self
    }
}

/// Coordinated menubar behavior for NyaTerm's custom title bar.
pub struct NyaAppMenuBar {
    menus: Vec<Entity<NyaAppMenuEntry>>,
    selected_index: Option<usize>,
    action_context: Option<FocusHandle>,
}

impl NyaAppMenuBar {
    pub fn new(menus: impl IntoIterator<Item = NyaAppMenu>, cx: &mut App) -> Entity<Self> {
        let bar = cx.new(|_| Self {
            menus: Vec::new(),
            selected_index: None,
            action_context: None,
        });
        let entries = menus
            .into_iter()
            .enumerate()
            .map(|(index, menu)| {
                let bar = bar.clone();
                cx.new(|_| NyaAppMenuEntry::new(index, menu, bar))
            })
            .collect();
        bar.update(cx, |bar, cx| {
            bar.menus = entries;
            cx.notify();
        });
        bar
    }

    fn set_selected_index(
        &mut self,
        index: Option<usize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_index == index {
            return;
        }

        if self.selected_index.is_none() && index.is_some() {
            self.action_context = window.focused(cx);
        }

        if index.is_some()
            && let Some(previous) = self.selected_index
            && let Some(menu) = self.menus.get(previous)
        {
            menu.update(cx, |menu, _| menu.close_popup());
        }

        if index.is_none()
            && let Some(action_context) = self.action_context.take()
        {
            action_context.focus(window, cx);
        }

        self.selected_index = index;
        cx.notify();
    }

    fn activate_menu(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(menu) = self.menus.get(index) {
            menu.update(cx, |menu, cx| menu.prepare_open(window, cx));
        }
        self.set_selected_index(Some(index), window, cx);
    }

    fn move_left(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.selected_index else {
            return;
        };
        let next = if index == 0 {
            self.menus.len().saturating_sub(1)
        } else {
            index - 1
        };
        self.activate_menu(next, window, cx);
    }

    fn move_right(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(index) = self.selected_index else {
            return;
        };
        let next = if index + 1 >= self.menus.len() {
            0
        } else {
            index + 1
        };
        self.activate_menu(next, window, cx);
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.selected_index.is_none() {
            return;
        }
        match event.keystroke.key.as_str() {
            "left" => self.move_left(window, cx),
            "right" => self.move_right(window, cx),
            "escape" => {
                let entry = self
                    .selected_index
                    .and_then(|index| self.menus.get(index).cloned());
                if let Some(entry) = entry {
                    entry.update(cx, |entry, _| entry.close_popup());
                }
                self.set_selected_index(None, window, cx);
            }
            _ => return,
        }
        cx.stop_propagation();
    }

    fn handle_dismiss(
        &mut self,
        index: usize,
        _: &DismissEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.selected_index == Some(index) {
            self.set_selected_index(None, window, cx);
        }
    }
}

impl Render for NyaAppMenuBar {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("nya-app-menu-bar")
            .h_full()
            .min_w_0()
            .flex()
            .items_center()
            .gap_x_1()
            .overflow_x_scroll()
            .restrict_scroll_to_axis()
            .role(Role::MenuBar)
            .key_context(KEY_CONTEXT)
            // PopupMenu consumes arrows used by submenus. An unhandled top-level
            // arrow continues as a key event and is coordinated here.
            .on_key_down(cx.listener(Self::on_key_down))
            .children(self.menus.clone())
    }
}

struct NyaAppMenuEntry {
    bar: Entity<NyaAppMenuBar>,
    index: usize,
    menu: NyaAppMenu,
    items: Vec<NyaMenuItem>,
    popup_menu: Option<Entity<PopupMenu>>,
    subscription: Option<Subscription>,
}

impl NyaAppMenuEntry {
    fn new(index: usize, menu: NyaAppMenu, bar: Entity<NyaAppMenuBar>) -> Self {
        Self {
            bar,
            index,
            menu,
            items: Vec::new(),
            popup_menu: None,
            subscription: None,
        }
    }

    fn is_selected(&self, cx: &App) -> bool {
        self.bar.read(cx).selected_index == Some(self.index)
    }

    fn close_popup(&mut self) {
        self.subscription.take();
        self.popup_menu.take();
    }

    fn prepare_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.close_popup();
        if let Some(on_open) = self.menu.on_open.clone() {
            on_open(window, cx);
        }
        self.items = (self.menu.items)(window, cx);
    }

    fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let selected = self.is_selected(cx);
        if !selected {
            self.prepare_open(window, cx);
        } else {
            self.close_popup();
        }
        self.bar.update(cx, |bar, cx| {
            bar.set_selected_index((!selected).then_some(self.index), window, cx);
        });
    }

    fn handle_trigger_click(
        &mut self,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(event, ClickEvent::Mouse(_)) {
            return;
        }
        self.toggle(window, cx);
    }

    fn handle_hover(&mut self, hovered: &bool, window: &mut Window, cx: &mut Context<Self>) {
        if !*hovered || !self.bar.read(cx).selected_index.is_some() {
            return;
        }
        if !self.is_selected(cx) {
            self.prepare_open(window, cx);
        }
        self.bar.update(cx, |bar, cx| {
            if bar.selected_index != Some(self.index) {
                bar.set_selected_index(Some(self.index), window, cx);
            }
        });
    }

    fn build_popup_menu(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<PopupMenu> {
        if let Some(popup) = self.popup_menu.as_ref() {
            return popup.clone();
        }
        let items = self.items.clone();
        let bar = self.bar.clone();
        let index = self.index;
        let min_width = self.menu.min_width;
        let popup = PopupMenu::build(window, cx, |menu, window, cx| {
            let menu = menu.when_some(min_width, |menu, width| menu.min_w(width));
            items
                .iter()
                .fold(menu, |menu, item| item.append_to(menu, window, cx))
        });
        self.subscription = Some(cx.subscribe_in(
            &popup,
            window,
            move |entry, _, event: &DismissEvent, window, cx| {
                entry.close_popup();
                bar.update(cx, |bar, cx| bar.handle_dismiss(index, event, window, cx));
            },
        ));
        let focus_handle = popup.read(cx).focus_handle(cx);
        focus_handle.focus(window, cx);
        self.popup_menu = Some(popup.clone());
        popup
    }
}

impl Render for NyaAppMenuEntry {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_selected = self.is_selected(cx);
        let label = (self.menu.label)(cx);
        let popup = is_selected.then(|| self.build_popup_menu(window, cx));

        div()
            .id(self.menu.id.clone())
            .relative()
            .child(
                Button::new("menu")
                    .small()
                    .py_0p5()
                    .compact()
                    .ghost()
                    .label(label)
                    .selected(is_selected)
                    .on_mouse_down(
                        MouseButton::Left,
                        window.listener_for(&cx.entity(), |this, _, window, cx| {
                            window.prevent_default();
                            cx.stop_propagation();
                            this.toggle(window, cx);
                        }),
                    )
                    .on_click(cx.listener(Self::handle_trigger_click)),
            )
            .on_hover(cx.listener(Self::handle_hover))
            .when_some(popup, |this, popup| {
                this.child(deferred(
                    anchored()
                        .anchor(Anchor::TopLeft)
                        .snap_to_window_with_margin(px(8.))
                        .child(div().size_full().occlude().top_1().child(popup)),
                ))
            })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use gpui::{
        Context, DismissEvent, Entity, FocusHandle, InteractiveElement as _, IntoElement,
        ParentElement as _, Render, Styled as _, TestAppContext, VisualTestContext, Window, div,
    };

    use super::{NyaAppMenu, NyaAppMenuBar};
    use crate::NyaMenuItem;

    struct MenuBarFixture {
        bar: Entity<NyaAppMenuBar>,
        original_focus: FocusHandle,
    }

    impl Render for MenuBarFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .child(div().id("original-focus").track_focus(&self.original_focus))
                .child(self.bar.clone())
        }
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
    }

    fn menu(id: &'static str, items_built: Rc<Cell<usize>>, with_submenu: bool) -> NyaAppMenu {
        NyaAppMenu::new(
            id,
            move |_| id.into(),
            move |_, _| {
                items_built.set(items_built.get() + 1);
                if with_submenu {
                    vec![NyaMenuItem::submenu(
                        "Nested",
                        vec![NyaMenuItem::action("Child")],
                    )]
                } else {
                    vec![NyaMenuItem::action("Action")]
                }
            },
        )
    }

    #[gpui::test]
    fn click_toggle_and_hover_switch_build_only_the_active_menu(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let first_builds = Rc::new(Cell::new(0));
        let second_builds = Rc::new(Cell::new(0));
        let (root, cx) = cx.add_window_view({
            let first_builds = first_builds.clone();
            let second_builds = second_builds.clone();
            move |window, cx| {
                let original_focus = cx.focus_handle();
                original_focus.focus(window, cx);
                MenuBarFixture {
                    bar: NyaAppMenuBar::new(
                        [
                            menu("file", first_builds, false),
                            menu("view", second_builds, false),
                        ],
                        cx,
                    ),
                    original_focus,
                }
            }
        });
        let bar = root.read_with(cx, |root, _| root.bar.clone());
        let first = bar.read_with(cx, |bar, _| bar.menus[0].clone());
        let second = bar.read_with(cx, |bar, _| bar.menus[1].clone());

        first.update_in(cx, |entry, window, cx| entry.toggle(window, cx));
        assert_eq!(bar.read_with(cx, |bar, _| bar.selected_index), Some(0));
        assert_eq!(first_builds.get(), 1);
        assert_eq!(second_builds.get(), 0);

        second.update_in(cx, |entry, window, cx| {
            entry.handle_hover(&true, window, cx)
        });
        assert_eq!(bar.read_with(cx, |bar, _| bar.selected_index), Some(1));
        assert_eq!(first_builds.get(), 1);
        assert_eq!(second_builds.get(), 1);
        assert!(first.read_with(cx, |entry, _| entry.popup_menu.is_none()));

        second.update_in(cx, |entry, window, cx| entry.toggle(window, cx));
        assert_eq!(bar.read_with(cx, |bar, _| bar.selected_index), None);
        second.update_in(cx, |entry, window, cx| entry.toggle(window, cx));
        assert_eq!(second_builds.get(), 2);
    }

    #[gpui::test]
    fn top_level_arrows_wrap_escape_restores_focus_and_submenu_arrows_stay_local(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (root, cx) = cx.add_window_view(move |window, cx| {
            let original_focus = cx.focus_handle();
            original_focus.focus(window, cx);
            MenuBarFixture {
                bar: NyaAppMenuBar::new(
                    [
                        menu("file", Rc::new(Cell::new(0)), true),
                        menu("view", Rc::new(Cell::new(0)), false),
                    ],
                    cx,
                ),
                original_focus,
            }
        });
        let (bar, original_focus) = root.read_with(cx, |root, _| {
            (root.bar.clone(), root.original_focus.clone())
        });
        bar.update_in(cx, |bar, window, cx| bar.activate_menu(0, window, cx));

        let cx: &mut VisualTestContext = cx;
        draw(cx);
        cx.simulate_keystrokes("right");
        draw(cx);
        assert_eq!(bar.read_with(cx, |bar, _| bar.selected_index), Some(1));

        cx.simulate_keystrokes("right");
        draw(cx);
        assert_eq!(bar.read_with(cx, |bar, _| bar.selected_index), Some(0));

        cx.simulate_keystrokes("down right");
        draw(cx);
        assert_eq!(bar.read_with(cx, |bar, _| bar.selected_index), Some(0));

        cx.simulate_keystrokes("escape escape");
        draw(cx);
        assert_eq!(bar.read_with(cx, |bar, _| bar.selected_index), None);
        cx.update(|window, cx| {
            assert_eq!(window.focused(cx).as_ref(), Some(&original_focus));
        });
    }

    #[gpui::test]
    fn labels_and_items_are_resolved_from_current_state(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let label = Rc::new(RefCell::new("File"));
        let item_label = Rc::new(RefCell::new("Open"));
        let (root, cx) = cx.add_window_view({
            let label = label.clone();
            let item_label = item_label.clone();
            move |window, cx| {
                let original_focus = cx.focus_handle();
                original_focus.focus(window, cx);
                let label_for_render = label.clone();
                let item_label_for_open = item_label.clone();
                MenuBarFixture {
                    bar: NyaAppMenuBar::new(
                        [NyaAppMenu::new(
                            "file",
                            move |_| (*label_for_render.borrow()).into(),
                            move |_, _| {
                                vec![NyaMenuItem::action(
                                    (*item_label_for_open.borrow()).to_string(),
                                )]
                            },
                        )],
                        cx,
                    ),
                    original_focus,
                }
            }
        });
        let bar = root.read_with(cx, |root, _| root.bar.clone());
        let entry = bar.read_with(cx, |bar, _| bar.menus[0].clone());

        *label.borrow_mut() = "文件";
        *item_label.borrow_mut() = "打开";
        assert_eq!(
            entry.read_with(cx, |entry, app| (entry.menu.label)(app)),
            "文件"
        );
        entry.update_in(cx, |entry, window, cx| entry.toggle(window, cx));
        assert_eq!(
            entry.read_with(cx, |entry, _| entry.items[0].test_label().to_string()),
            "打开"
        );
    }

    #[gpui::test]
    fn reopening_refreshes_item_presentation_state(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let enabled = Rc::new(Cell::new(false));
        let (root, cx) = cx.add_window_view({
            let enabled = enabled.clone();
            move |window, cx| {
                let original_focus = cx.focus_handle();
                original_focus.focus(window, cx);
                let enabled = enabled.clone();
                MenuBarFixture {
                    bar: NyaAppMenuBar::new(
                        [NyaAppMenu::new(
                            "view",
                            |_| "View".into(),
                            move |_, _| {
                                let enabled = enabled.get();
                                vec![
                                    NyaMenuItem::action(if enabled { "On" } else { "Off" })
                                        .icon(if enabled {
                                            "icons/state-on.svg"
                                        } else {
                                            "icons/state-off.svg"
                                        })
                                        .shortcut(if enabled { "Ctrl+1" } else { "Ctrl+0" })
                                        .disabled(enabled)
                                        .checked(enabled),
                                ]
                            },
                        )],
                        cx,
                    ),
                    original_focus,
                }
            }
        });
        let bar = root.read_with(cx, |root, _| root.bar.clone());
        let entry = bar.read_with(cx, |bar, _| bar.menus[0].clone());

        entry.update_in(cx, |entry, window, cx| entry.toggle(window, cx));
        assert_eq!(
            entry.read_with(cx, |entry, _| entry.items[0].test_presentation()),
            (
                "Off".to_string(),
                Some("icons/state-off.svg".to_string()),
                Some("Ctrl+0".to_string()),
                false,
                false,
                false,
            )
        );

        entry.update_in(cx, |entry, window, cx| entry.toggle(window, cx));
        enabled.set(true);
        entry.update_in(cx, |entry, window, cx| entry.toggle(window, cx));
        assert_eq!(
            entry.read_with(cx, |entry, _| entry.items[0].test_presentation()),
            (
                "On".to_string(),
                Some("icons/state-on.svg".to_string()),
                Some("Ctrl+1".to_string()),
                true,
                true,
                false,
            )
        );
    }

    #[gpui::test]
    fn dismiss_and_item_execution_close_the_bar_and_restore_focus(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let invoked = Rc::new(Cell::new(false));
        let (root, cx) = cx.add_window_view({
            let invoked = invoked.clone();
            move |window, cx| {
                let original_focus = cx.focus_handle();
                original_focus.focus(window, cx);
                let invoked = invoked.clone();
                MenuBarFixture {
                    bar: NyaAppMenuBar::new(
                        [NyaAppMenu::new(
                            "file",
                            |_| "File".into(),
                            move |_, _| {
                                let invoked = invoked.clone();
                                vec![NyaMenuItem::action("Run").on_click(move |_, _, _| {
                                    invoked.set(true);
                                })]
                            },
                        )],
                        cx,
                    ),
                    original_focus,
                }
            }
        });
        let (bar, original_focus) = root.read_with(cx, |root, _| {
            (root.bar.clone(), root.original_focus.clone())
        });
        let entry = bar.read_with(cx, |bar, _| bar.menus[0].clone());

        entry.update_in(cx, |entry, window, cx| entry.toggle(window, cx));
        let cx: &mut VisualTestContext = cx;
        draw(cx);
        let popup = entry.read_with(cx, |entry, _| entry.popup_menu.clone().unwrap());
        popup.update(cx, |_, cx| cx.emit(DismissEvent));
        cx.run_until_parked();
        assert_eq!(bar.read_with(cx, |bar, _| bar.selected_index), None);
        cx.update(|window, cx| {
            assert_eq!(window.focused(cx).as_ref(), Some(&original_focus));
        });

        entry.update_in(cx, |entry, window, cx| entry.toggle(window, cx));
        draw(cx);
        cx.simulate_keystrokes("down enter");
        draw(cx);
        assert!(invoked.get());
        assert_eq!(bar.read_with(cx, |bar, _| bar.selected_index), None);
        cx.update(|window, cx| {
            assert_eq!(window.focused(cx).as_ref(), Some(&original_focus));
        });
    }
}

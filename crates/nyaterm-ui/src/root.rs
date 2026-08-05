//! Window-root adapter for gpui-component.

use gpui::{
    AnyView, AppContext as _, Context, IntoElement, ParentElement as _, Render, Styled as _,
    Window, WindowHandle, div,
};

/// NyaTerm's component root type.
///
/// Keep this alias inside `nyaterm-ui` so feature modules do not depend on the
/// third-party root type directly. Windows that render component-backed
/// dialogs, popovers, menus, tooltips, or inputs should use this as their first
/// view layer.
pub type NyaRoot = gpui_component::Root;

pub type NyaWindowHandle = WindowHandle<NyaRoot>;

struct NyaRootContent {
    view: AnyView,
}

impl NyaRootContent {
    fn new(view: impl Into<AnyView>) -> Self {
        Self { view: view.into() }
    }
}

impl Render for NyaRootContent {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(self.view.clone())
            .children(gpui_component::Root::render_sheet_layer(window, cx))
            .children(gpui_component::Root::render_dialog_layer(window, cx))
            .children(gpui_component::Root::render_notification_layer(window, cx))
    }
}

pub fn nya_root(
    view: impl Into<AnyView>,
    window: &mut Window,
    cx: &mut Context<NyaRoot>,
) -> NyaRoot {
    let content = cx.new(|_| NyaRootContent::new(view));
    gpui_component::Root::new(content, window, cx)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use gpui::{
        AppContext as _, Context, InteractiveElement as _, IntoElement, Modifiers, MouseButton,
        ParentElement as _, Render, StatefulInteractiveElement as _, Styled as _, TestAppContext,
        VisualTestContext, Window, div, point, px,
    };

    use crate::{NyaDialogFooter, NyaDialogWindowExt as _, nya_root};

    struct RootContentFixture;

    impl Render for RootContentFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .size_full()
                .debug_selector(|| "nya-root-content".to_string())
        }
    }

    struct PointerContentFixture {
        down: Arc<AtomicUsize>,
        movement: Arc<AtomicUsize>,
        up: Arc<AtomicUsize>,
    }

    impl Render for PointerContentFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let down = self.down.clone();
            let movement = self.movement.clone();
            let up = self.up.clone();
            div()
                .size_full()
                .on_any_mouse_down(move |_, _, _| {
                    down.fetch_add(1, Ordering::SeqCst);
                })
                .on_mouse_move(move |_, _, _| {
                    movement.fetch_add(1, Ordering::SeqCst);
                })
                .on_mouse_up(MouseButton::Left, move |_, _, _| {
                    up.fetch_add(1, Ordering::SeqCst);
                })
        }
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.run_until_parked();
        cx.update(|window, cx| {
            _ = window.draw(cx);
        });
    }

    #[gpui::test]
    fn nya_root_renders_component_dialog_layer(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let (_, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| RootContentFixture);
            nya_root(view, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        draw(cx);
        assert!(cx.debug_bounds("nya-root-content").is_some());

        cx.update(|window, cx| {
            window.open_nya_dialog(cx, |dialog, _, _| {
                dialog.content(
                    div()
                        .debug_selector(|| "nya-dialog-content".to_string())
                        .child("Dialog is visible"),
                )
            });
        });
        draw(cx);

        assert!(cx.debug_bounds("nya-dialog-content").is_some());
    }

    #[gpui::test]
    fn nya_dialog_blocks_lower_pointer_events_while_open_and_preserves_clicks(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let lower_down = Arc::new(AtomicUsize::new(0));
        let lower_movement = Arc::new(AtomicUsize::new(0));
        let lower_up = Arc::new(AtomicUsize::new(0));
        let dialog_clicks = Arc::new(AtomicUsize::new(0));
        let fixture_down = lower_down.clone();
        let fixture_movement = lower_movement.clone();
        let fixture_up = lower_up.clone();
        let (_, cx) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|_| PointerContentFixture {
                down: fixture_down,
                movement: fixture_movement,
                up: fixture_up,
            });
            nya_root(view, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        let clicks = dialog_clicks.clone();
        cx.update(|window, cx| {
            window.open_nya_dialog(cx, move |dialog, _, _| {
                let clicks = clicks.clone();
                dialog.title("Dialog").content(
                    div()
                        .id("nya-dialog-test-action")
                        .debug_selector(|| "nya-dialog-test-action".to_string())
                        .size(px(40.))
                        .on_click(move |_, _, _| {
                            clicks.fetch_add(1, Ordering::SeqCst);
                        }),
                )
            });
        });
        draw(cx);

        cx.simulate_mouse_move(point(px(12.), px(80.)), None, Modifiers::default());
        cx.simulate_mouse_up(
            point(px(12.), px(80.)),
            MouseButton::Left,
            Modifiers::default(),
        );
        assert_eq!(lower_movement.load(Ordering::SeqCst), 0);
        assert_eq!(lower_up.load(Ordering::SeqCst), 0);

        let action = cx
            .debug_bounds("nya-dialog-test-action")
            .expect("dialog action should be rendered");
        cx.simulate_click(action.center(), Modifiers::default());
        assert_eq!(dialog_clicks.load(Ordering::SeqCst), 1);
        assert_eq!(lower_down.load(Ordering::SeqCst), 0);
        assert_eq!(lower_movement.load(Ordering::SeqCst), 0);
        assert_eq!(lower_up.load(Ordering::SeqCst), 0);

        cx.simulate_click(point(px(12.), px(80.)), Modifiers::default());
        cx.run_until_parked();
        assert_eq!(lower_down.load(Ordering::SeqCst), 0);
        assert_eq!(lower_movement.load(Ordering::SeqCst), 0);
        cx.update(|window, cx| {
            assert!(!window.has_active_nya_dialog(cx));
        });
    }

    #[gpui::test]
    fn nya_confirm_dialog_renders_footer_actions(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let (_, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| RootContentFixture);
            nya_root(view, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            window.open_nya_dialog(cx, |dialog, _, _| {
                dialog
                    .title("Delete Item")
                    .confirm(NyaDialogFooter::new("Cancel", "Delete"))
                    .content(
                        div()
                            .debug_selector(|| "nya-confirm-dialog-content".to_string())
                            .child("Delete this item?"),
                    )
            });
        });
        draw(cx);

        assert!(cx.debug_bounds("nya-confirm-dialog-content").is_some());
        assert!(cx.debug_bounds("nya-dialog-cancel-button").is_some());
        assert!(cx.debug_bounds("nya-dialog-action-button").is_some());
    }

    #[gpui::test]
    fn nya_danger_confirm_dialog_renders_footer_actions(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let (_, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| RootContentFixture);
            nya_root(view, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            window.open_nya_dialog(cx, |dialog, _, _| {
                dialog
                    .title("Delete Folder")
                    .confirm(NyaDialogFooter::new("Cancel", "Delete").danger())
                    .content(
                        div()
                            .debug_selector(|| "nya-danger-dialog-content".to_string())
                            .child("Delete this folder?"),
                    )
            });
        });
        draw(cx);

        assert!(cx.debug_bounds("nya-danger-dialog-content").is_some());
        assert!(cx.debug_bounds("nya-dialog-cancel-button").is_some());
        assert!(cx.debug_bounds("nya-dialog-action-button").is_some());
    }

    #[gpui::test]
    fn nya_alert_dialog_renders_action_footer(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);

        let (_, cx) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| RootContentFixture);
            nya_root(view, window, cx)
        });
        let cx: &mut VisualTestContext = cx;

        cx.update(|window, cx| {
            window.open_nya_dialog(cx, |dialog, _, _| {
                dialog.title("Notice").alert("OK").content(
                    div()
                        .debug_selector(|| "nya-alert-dialog-content".to_string())
                        .child("Something happened."),
                )
            });
        });
        draw(cx);

        assert!(cx.debug_bounds("nya-alert-dialog-content").is_some());
        assert!(cx.debug_bounds("nya-dialog-action-button").is_some());
        assert!(cx.debug_bounds("nya-dialog-cancel-button").is_none());
    }
}

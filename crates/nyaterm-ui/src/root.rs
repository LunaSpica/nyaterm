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
    use gpui::{
        AppContext as _, Context, InteractiveElement as _, IntoElement, ParentElement as _, Render,
        Styled as _, TestAppContext, VisualTestContext, Window, div,
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

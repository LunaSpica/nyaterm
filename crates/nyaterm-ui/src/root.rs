//! Window-root adapter for gpui-component.

use gpui::{AnyView, Context, Window, WindowHandle};

/// NyaTerm's component root type.
///
/// Keep this alias inside `nyaterm-ui` so feature modules do not depend on the
/// third-party root type directly. Windows that render component-backed
/// dialogs, popovers, menus, tooltips, or inputs should use this as their first
/// view layer.
pub type NyaRoot = gpui_component::Root;

pub type NyaWindowHandle = WindowHandle<NyaRoot>;

pub fn nya_root(
    view: impl Into<AnyView>,
    window: &mut Window,
    cx: &mut Context<NyaRoot>,
) -> NyaRoot {
    gpui_component::Root::new(view, window, cx)
}

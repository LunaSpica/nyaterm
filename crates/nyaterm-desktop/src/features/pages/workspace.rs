use gpui::{
    Context, FontWeight, IntoElement, SharedString, div, prelude::*, px, relative, rgb, rgba, svg,
};

use super::super::{
    NyaTermApp, SessionTabDragPayload, SessionTabDragPreview, SessionTabTooltip, ThemePalette,
    session_kind_label, short_id, truncate_preview,
};
use crate::models::{
    TabDockEdge, TabDockZone, TerminalWindowNode, WorkspacePaneNode, WorkspaceSplitDirection,
};

#[path = "workspace/panes.rs"]
mod panes;
#[path = "workspace/terminal_windows.rs"]
mod terminal_windows;

impl NyaTermApp {
    pub(in crate::features) fn workspace_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let palette = self.theme_palette();
        // Match the Tauri shell: tab strip sits directly above the terminal surface.
        // Do not reconcile/prune layout here — paint must stay pure. Session
        // register/close/idle already keep terminal_windows coherent.
        let multi_leaf = self.terminal_windows_is_multi_leaf();
        let mut workspace = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .bg(rgb(palette.bg));
        if !multi_leaf {
            workspace = workspace.child(self.session_tab_strip(cx));
        }

        workspace
            .child(self.workspace_terminal_area(cx))
            .child(self.bottom_panel_view(cx))
    }

    fn workspace_terminal_area(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let palette = self.theme_palette();
        if self.active_session_id.is_none() {
            return self.empty_workspace_state(cx).into_any_element();
        }
        // Multi-leaf tab windows (Tauri TabWindowsWorkspace) take precedence over
        // single-tree pane splits when active.
        if let Some(window_root) = self.terminal_windows.clone() {
            if matches!(window_root, TerminalWindowNode::Split { .. }) {
                return div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .p_1()
                    .bg(rgb(palette.bg))
                    .child(self.render_terminal_window_node(window_root, cx))
                    .into_any_element();
            }
        }
        let Some(root) = self.workspace_split.clone() else {
            return self.terminal_canvas(cx).into_any_element();
        };

        div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .p_1()
            .bg(rgb(palette.bg))
            .child(self.render_workspace_pane_node(root, true, cx))
            .into_any_element()
    }
}

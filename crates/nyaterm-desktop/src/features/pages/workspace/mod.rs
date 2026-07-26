use gpui::{
    Context, FontWeight, IntoElement, SharedString, div, prelude::*, px, relative, rgb, rgba, svg,
};

use super::super::{
    NyaTermApp, SessionTabDragPayload, SessionTabDragPreview, SessionTabTooltip, ThemePalette,
    session_kind_label, short_id, small_button, truncate_preview,
};
use crate::models::{
    TabDockEdge, TabDockZone, TerminalWindowNode, WorkspacePaneNode, WorkspaceSplitDirection,
};

mod panes;
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
        let has_connect_failure = self.has_failed_session_start()
            || (self.last_connect_failure_name.is_some()
                && self.last_connect_failure_error.is_some());
        let show_tab_strip = !multi_leaf
            && (self.ordered_tab_session_count() > 0
                || self.has_pending_session_start()
                || has_connect_failure);
        let mut workspace = div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .bg(self.shell_transparent_color(palette.bg));
        if show_tab_strip {
            workspace = workspace.child(self.session_tab_strip(cx));
        }

        workspace
            .child(self.workspace_terminal_area(cx))
            .child(self.bottom_panel_resize_handle(cx))
            .child(self.bottom_panel_view(cx))
    }

    fn workspace_terminal_area(&mut self, cx: &mut Context<Self>) -> gpui::AnyElement {
        let palette = self.theme_palette();
        if self.active_pending_session_start.is_some() {
            return self.pending_workspace_state().into_any_element();
        }
        if self.active_failed_session_start.is_some() {
            return self.failed_workspace_state().into_any_element();
        }
        if self.active_session_id.is_none() {
            if self.has_pending_session_start() {
                return self.pending_workspace_state().into_any_element();
            }
            if self.has_failed_session_start()
                || (self.last_connect_failure_name.is_some()
                    && self.last_connect_failure_error.is_some())
            {
                return self.failed_workspace_state().into_any_element();
            }
            return self.empty_workspace_state(cx).into_any_element();
        }
        // Multi-leaf tab windows (Tauri TabWindowsWorkspace) take precedence over
        // single-tree pane splits when active.
        if let Some(window_root) = self.terminal.windows.tree.clone() {
            if matches!(window_root, TerminalWindowNode::Split { .. }) {
                return div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .bg(self.shell_transparent_color(palette.bg))
                    .child(self.render_terminal_window_node(window_root, cx))
                    .into_any_element();
            }
        }
        let root = self
            .workspace_split
            .clone()
            .unwrap_or_else(|| WorkspacePaneNode::leaf(self.active_session_id.clone().unwrap()));

        let show_chrome = root.is_split();
        div()
            .flex_1()
            .min_h_0()
            .min_w_0()
            .bg(self.shell_transparent_color(palette.bg))
            .child(self.render_workspace_pane_node(root, show_chrome, cx))
            .into_any_element()
    }
}

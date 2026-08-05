use gpui::{Context, IntoElement, div, prelude::*, px};

use super::super::NyaTermApp;

mod browser;
mod browser_columns;
mod browser_filter;
mod browser_keys;
mod browser_navigation;
mod browser_selection;
mod editor;
mod entry_row;
mod file_ops;
mod forms;
mod helpers;
mod overlays;
mod overlays_context;
mod overlays_delete_move;
mod overlays_editor;
mod overlays_favorites;
mod overlays_unknown;
mod overlays_upload;
mod path_bar;
mod properties;
mod properties_dialog;
mod queue;

use entry_row::{
    TransferBrowserEntryRowPresentation, transfer_browser_entry_row,
    transfer_browser_parent_entry_row,
};
use helpers::*;

const NATIVE_EDITOR_MAX_BYTES: u64 = 512 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::features::pages::transfers) enum TransferBrowserAvailability {
    NoSession,
    UnsupportedSession,
    DisconnectedSsh,
    Browsable,
}

fn transfer_browser_availability(
    has_active_session: bool,
    has_ssh_config: bool,
    is_disconnected: bool,
) -> TransferBrowserAvailability {
    if !has_active_session {
        TransferBrowserAvailability::NoSession
    } else if !has_ssh_config {
        TransferBrowserAvailability::UnsupportedSession
    } else if is_disconnected {
        TransferBrowserAvailability::DisconnectedSsh
    } else {
        TransferBrowserAvailability::Browsable
    }
}

fn transfer_dialog_width(viewport_width: f32, preferred_width: f32) -> f32 {
    preferred_width.min((viewport_width - 32.).max(240.))
}

fn transfer_menu_position(
    x: f32,
    y: f32,
    menu_width: f32,
    preferred_height: f32,
    viewport_width: f32,
    viewport_height: f32,
) -> (f32, f32, f32) {
    let margin = 8.;
    let max_height = (viewport_height - margin * 2.).max(80.);
    let height = preferred_height.min(max_height);
    let max_x = (viewport_width - menu_width - margin).max(margin);
    let max_y = (viewport_height - height - margin).max(margin);
    (x.clamp(margin, max_x), y.clamp(margin, max_y), max_height)
}

impl NyaTermApp {
    pub(in crate::features) fn transfers_view(
        &mut self,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let active_session_id = self.session.active_id();
        let browser_availability = transfer_browser_availability(
            active_session_id.is_some(),
            self.session.active_ssh_config().is_some(),
            active_session_id.is_some_and(|session_id| self.session.is_disconnected(session_id)),
        );
        let transfer_height = self.transfer.panel_height().clamp(60., 600.);
        let duplicate_prompt = self.session.prompt_active_duplicate().cloned();

        // Tauri AppPanelContent: FileExplorer (flex-1) + vertical resize + FileTransfer fixed height.
        let palette = self.theme_palette();
        let view = div()
            .size_full()
            .flex()
            .flex_col()
            .overflow_hidden()
            .bg(self.shell_transparent_color(palette.surface));

        view.child(
            div()
                .flex_1()
                .min_h_0()
                .overflow_hidden()
                .child(self.transfer_browser_view(browser_availability, cx)),
        )
        .child(self.transfer_height_resize_handle(cx))
        .child(
            div()
                .h(px(transfer_height))
                .flex_none()
                .overflow_hidden()
                .child(self.transfer_queue_view(cx)),
        )
        .when_some(duplicate_prompt, |this, prompt| {
            this.child(self.duplicate_prompt_banner(prompt, cx))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        TransferBrowserAvailability, transfer_browser_availability, transfer_dialog_width,
        transfer_menu_position,
    };

    #[test]
    fn browser_availability_distinguishes_session_states() {
        assert_eq!(
            transfer_browser_availability(false, false, false),
            TransferBrowserAvailability::NoSession
        );
        assert_eq!(
            transfer_browser_availability(true, false, false),
            TransferBrowserAvailability::UnsupportedSession
        );
        assert_eq!(
            transfer_browser_availability(true, true, true),
            TransferBrowserAvailability::DisconnectedSsh
        );
        assert_eq!(
            transfer_browser_availability(true, true, false),
            TransferBrowserAvailability::Browsable
        );
    }

    #[test]
    fn dialog_width_uses_preferred_size_with_narrow_viewport_fallback() {
        assert_eq!(transfer_dialog_width(1280., 500.), 500.);
        assert_eq!(transfer_dialog_width(420., 500.), 388.);
        assert_eq!(transfer_dialog_width(200., 500.), 240.);
    }

    #[test]
    fn menu_position_stays_inside_viewport() {
        assert_eq!(
            transfer_menu_position(1200., 760., 268., 360., 1280., 800.),
            (1004., 432., 784.)
        );
        assert_eq!(
            transfer_menu_position(500., 500., 268., 560., 300., 240.),
            (24., 8., 224.)
        );
    }
}

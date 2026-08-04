use gpui::{
    ClickEvent, ClipboardItem, Context, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Window,
};
use nyaterm_core::truncate_preview;
use nyaterm_transport::{SftpFileEntry, SftpFileType};

use std::collections::HashSet;
use std::time::Duration;

use crate::features::NyaTermApp;
use crate::models::TransferBrowserDragSelectionState;

use super::{TransferPathPart, remote_file_name, transfer_path_part_value};

impl NyaTermApp {
    pub(in crate::features::pages::transfers) fn select_transfer_browser_entry(
        &mut self,
        path: String,
        cx: &mut Context<Self>,
    ) {
        self.transfer.select_browser_entry(path.clone());
        self.transfer.set_remote_path(path.clone());
        self.shell.set_status(format!("selected remote {path}"));
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn select_transfer_browser_entry_from_click(
        &mut self,
        path: String,
        event: &ClickEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(self.transfer.browser_view().focus, cx);
        let modifiers = event.modifiers();
        if event.click_count() >= 2 && !modifiers.modified() {
            self.cancel_transfer_browser_pending_rename(cx);
            let entry = self
                .transfer
                .browser_view()
                .entries
                .iter()
                .find(|entry| entry.path == path)
                .cloned();
            self.select_transfer_browser_entry(path, cx);
            if let Some(entry) = entry {
                if entry.file_type == SftpFileType::Directory {
                    self.open_transfer_browser_directory(entry.path, window, cx);
                } else {
                    self.open_transfer_default(entry, window, cx);
                }
            }
        }
    }

    pub(in crate::features::pages::transfers) fn handle_transfer_browser_entry_mouse_down(
        &mut self,
        path: String,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(self.transfer.browser_view().focus, cx);
        if self
            .transfer
            .browser_view()
            .pending_rename
            .as_ref()
            .is_some_and(|pending| pending.path != path)
        {
            self.cancel_transfer_browser_pending_rename(cx);
        }

        let additive = event.modifiers.platform || event.modifiers.control;
        let range_anchor = event
            .modifiers
            .shift
            .then(|| self.transfer.browser_view().selected_remote_path.clone())
            .flatten()
            .or_else(|| {
                event
                    .modifiers
                    .shift
                    .then(|| {
                        self.transfer
                            .browser_view()
                            .selected_remote_paths
                            .iter()
                            .next()
                            .cloned()
                    })
                    .flatten()
            });
        let anchor_path = range_anchor.clone().unwrap_or_else(|| path.clone());
        let base_selection = if additive {
            self.transfer.browser_view().selected_remote_paths.clone()
        } else {
            HashSet::new()
        };

        if let Some(anchor) = range_anchor {
            self.apply_transfer_browser_range(
                anchor,
                path.clone(),
                base_selection.clone(),
                additive,
                cx,
            );
        } else if additive {
            self.toggle_transfer_browser_entry_marked(path.clone(), cx);
        } else {
            self.select_transfer_browser_entry(path.clone(), cx);
        }

        self.transfer
            .set_browser_drag_selection(TransferBrowserDragSelectionState {
                anchor_path,
                base_selection,
                additive,
            });
    }

    pub(in crate::features::pages::transfers) fn schedule_transfer_browser_name_rename(
        &mut self,
        path: String,
        was_single_selected_on_mouse_down: bool,
        event: &ClickEvent,
        cx: &mut Context<Self>,
    ) {
        let modifiers = event.modifiers();
        if !was_single_selected_on_mouse_down
            || event.click_count() != 1
            || modifiers.modified()
            || self.transfer.rename_dialog_is_open()
        {
            if event.click_count() >= 2 || modifiers.modified() {
                self.cancel_transfer_browser_pending_rename(cx);
            }
            return;
        }

        let Some(token) = self.transfer.schedule_browser_pending_rename(&path) else {
            return;
        };
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(220))
                .await;
            let _ = this.update(cx, |this, cx| {
                let rename_dialog_open = this.transfer.rename_dialog_is_open();
                let should_rename =
                    this.transfer
                        .resolve_browser_pending_rename(&path, token, rename_dialog_open);
                if should_rename {
                    this.open_transfer_rename_for_path_after_delay(path, cx);
                } else {
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn cancel_transfer_browser_pending_rename(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        if self.transfer.cancel_browser_pending_rename() {
            cx.notify();
        }
    }

    pub(in crate::features::pages::transfers) fn handle_transfer_browser_entry_mouse_move(
        &mut self,
        path: String,
        event: &MouseMoveEvent,
        cx: &mut Context<Self>,
    ) {
        if !event.dragging() {
            self.transfer.clear_browser_drag_selection();
            return;
        }

        let Some(drag_selection) = self.transfer.browser_view().drag_selection.clone() else {
            return;
        };

        self.apply_transfer_browser_range(
            drag_selection.anchor_path,
            path,
            drag_selection.base_selection,
            drag_selection.additive,
            cx,
        );
    }

    pub(in crate::features::pages::transfers) fn finish_transfer_browser_selection_drag(
        &mut self,
        _event: &MouseUpEvent,
        cx: &mut Context<Self>,
    ) {
        if self.transfer.finish_browser_drag_selection() {
            cx.notify();
        }
    }

    pub(in crate::features::pages::transfers) fn select_transfer_browser_entry_from_context(
        &mut self,
        path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(self.transfer.browser_view().focus, cx);
        if let Some(selected_count) = self.transfer.activate_marked_browser_path(&path) {
            self.transfer.set_remote_path(path);
            self.shell
                .set_status(format!("{} remote item(s) marked", selected_count));
            cx.notify();
            return;
        }

        self.select_transfer_browser_entry(path, cx);
    }

    pub(in crate::features::pages::transfers) fn prepare_transfer_browser_entry_context_menu(
        &mut self,
        path: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.transfer.close_browser_path_menu();
        self.select_transfer_browser_entry_from_context(path, window, cx);
    }

    pub(in crate::features::pages::transfers) fn prepare_transfer_browser_parent_context_menu(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(self.transfer.browser_view().focus, cx);
        self.transfer.clear_browser_selection();
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn prepare_transfer_browser_current_context_menu(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        window.focus(self.transfer.browser_view().focus, cx);
        self.transfer.clear_browser_selection();
        cx.notify();
    }

    fn apply_transfer_browser_range(
        &mut self,
        anchor_path: String,
        target_path: String,
        base_selection: HashSet<String>,
        additive: bool,
        cx: &mut Context<Self>,
    ) {
        let entries = self.visible_transfer_browser_entries();
        let anchor_index = entries.iter().position(|entry| entry.path == anchor_path);
        let target_index = entries.iter().position(|entry| entry.path == target_path);

        let (Some(anchor_index), Some(target_index)) = (anchor_index, target_index) else {
            if additive {
                let active_path = self.transfer.browser_view().selected_remote_path.clone();
                self.transfer
                    .replace_browser_selection(base_selection, active_path);
                cx.notify();
            } else {
                self.select_transfer_browser_entry(target_path, cx);
            }
            return;
        };

        let mut next_selection = if additive {
            base_selection
        } else {
            HashSet::new()
        };
        let start = anchor_index.min(target_index);
        let end = anchor_index.max(target_index);
        for entry in &entries[start..=end] {
            next_selection.insert(entry.path.clone());
        }

        let selected_count = self
            .transfer
            .replace_browser_selection(next_selection, Some(target_path.clone()));
        self.transfer.set_remote_path(target_path);
        self.shell
            .set_status(format!("{} remote item(s) marked", selected_count));
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn toggle_transfer_browser_entry_marked(
        &mut self,
        path: String,
        cx: &mut Context<Self>,
    ) {
        let selected_count = self.transfer.toggle_browser_path_mark(path.clone());
        self.transfer.set_remote_path(path.clone());
        self.shell
            .set_status(format!("{} remote item(s) marked", selected_count));
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn select_all_visible_transfer_entries(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        let entries = self.visible_transfer_browser_entries();
        let active_path = entries.first().map(|entry| entry.path.clone());
        let selected_paths = entries.iter().map(|entry| entry.path.clone()).collect();
        let selected_count = self
            .transfer
            .replace_browser_selection(selected_paths, active_path.clone());
        if let Some(path) = active_path {
            self.transfer.set_remote_path(path);
        }
        self.shell
            .set_status(format!("{} remote item(s) marked", selected_count));
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn selected_transfer_path_part(
        &self,
        part: TransferPathPart,
    ) -> Option<String> {
        let path = self
            .transfer
            .browser_view()
            .selected_remote_path
            .as_deref()?;
        Some(transfer_path_part_value(path, part))
    }

    pub(in crate::features::pages::transfers) fn copy_selected_transfer_path(
        &mut self,
        part: TransferPathPart,
        cx: &mut Context<Self>,
    ) {
        let Some(value) = self.selected_transfer_path_part(part) else {
            self.shell
                .set_status("select a remote item first".to_string());
            cx.notify();
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(value.clone()));
        self.shell
            .set_status(format!("copied remote {}", part.label()));
        self.transfer
            .set_browser_status(truncate_preview(&value, 92));
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn send_selected_transfer_path_to_terminal(
        &mut self,
        part: TransferPathPart,
        cx: &mut Context<Self>,
    ) {
        let Some(value) = self.selected_transfer_path_part(part) else {
            self.shell
                .set_status("select a remote item first".to_string());
            cx.notify();
            return;
        };
        if self.session.active_id().is_none() {
            self.shell
                .set_status("start a session before sending remote path".to_string());
            cx.notify();
            return;
        }
        if self.send_terminal_input(value.clone().into_bytes(), cx) {
            self.shell
                .set_status(format!("sent remote {} to terminal", part.label()));
            self.transfer
                .set_browser_status(truncate_preview(&value, 92));
            cx.notify();
        }
    }

    pub(in crate::features::pages::transfers) fn selected_transfer_entry(
        &self,
    ) -> Option<SftpFileEntry> {
        let selected = self
            .transfer
            .browser_view()
            .selected_remote_path
            .as_deref()?;
        self.transfer
            .browser_view()
            .entries
            .iter()
            .find(|entry| entry.path == selected)
            .cloned()
    }

    pub(in crate::features::pages::transfers) fn selected_transfer_entries(
        &self,
    ) -> Vec<SftpFileEntry> {
        if self
            .transfer
            .browser_view()
            .selected_remote_paths
            .is_empty()
        {
            return self.selected_transfer_entry().into_iter().collect();
        }
        self.visible_transfer_browser_entries()
            .into_iter()
            .filter(|entry| {
                self.transfer
                    .browser_view()
                    .selected_remote_paths
                    .contains(&entry.path)
            })
            .collect()
    }

    pub(in crate::features::pages::transfers) fn start_selected_sftp_download_jobs(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let entries = self.selected_transfer_entries();
        if entries.is_empty() {
            self.shell
                .set_status("mark remote items before downloading".to_string());
            cx.notify();
            return;
        }
        if self.settings.summary().transfer_ask_save_location {
            let remote_paths = entries
                .into_iter()
                .map(|entry| entry.path)
                .collect::<Vec<_>>();
            self.prompt_transfer_download_directory_and_start(remote_paths, window, cx);
            return;
        }
        let base_local_path = self.normalized_transfer_local_path();
        let total = entries.len();
        for entry in entries {
            let local_path = if total == 1 {
                base_local_path.clone()
            } else {
                base_local_path.join(remote_file_name(&entry.path))
            };
            self.start_sftp_download_job_for_target(entry.path, local_path, window, cx);
        }
        self.shell
            .set_status(format!("{total} SFTP download job(s) started"));
        cx.notify();
    }
}

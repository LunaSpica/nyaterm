use super::*;

impl NyaTermApp {
    pub(in crate::features::pages::transfers) fn handle_transfer_browser_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let keystroke = &event.keystroke;
        let modified_for_select_all = (keystroke.modifiers.platform || keystroke.modifiers.control)
            && !keystroke.modifiers.alt
            && !keystroke.modifiers.shift;

        if modified_for_select_all && keystroke.key.eq_ignore_ascii_case("a") {
            cx.stop_propagation();
            self.select_all_visible_transfer_entries(cx);
            return;
        }

        let unmodified = !keystroke.modifiers.alt
            && !keystroke.modifiers.control
            && !keystroke.modifiers.platform
            && !keystroke.modifiers.shift;

        if unmodified && keystroke.key.eq_ignore_ascii_case("enter") {
            cx.stop_propagation();
            if let Some(entry) = self.selected_transfer_entry() {
                if entry.file_type == SftpFileType::Directory {
                    self.open_transfer_browser_directory(entry.path, window, cx);
                } else {
                    self.open_transfer_default(entry, window, cx);
                }
            } else {
                self.terminal.view.status = "select a remote item before opening".to_string();
                cx.notify();
            }
            return;
        }

        if unmodified && keystroke.key.eq_ignore_ascii_case("backspace") {
            cx.stop_propagation();
            self.open_transfer_parent_directory(window, cx);
            return;
        }

        if unmodified && keystroke.key.eq_ignore_ascii_case("f5") {
            cx.stop_propagation();
            self.refresh_transfer_browser(window, cx);
            return;
        }

        if crate::shortcuts::shortcut_matches(
            event,
            "fileExplorer.rename",
            &self.settings.keybindings,
        ) && self.selected_transfer_entries().len() == 1
            && self.active_ssh_config.is_some()
            && self.transfer.file_ops.rename.is_none()
        {
            cx.stop_propagation();
            self.open_transfer_rename_dialog(window, cx);
            return;
        }

        if keystroke.key == "delete"
            && unmodified
            && !self.selected_transfer_entries().is_empty()
            && self.transfer.file_ops.delete.is_none()
        {
            cx.stop_propagation();
            self.open_selected_transfer_delete_dialog(window, cx);
        }
    }
}

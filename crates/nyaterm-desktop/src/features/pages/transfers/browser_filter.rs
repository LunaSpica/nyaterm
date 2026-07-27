use super::*;

impl NyaTermApp {
    pub(in crate::features::pages::transfers) fn visible_transfer_browser_entries(
        &self,
    ) -> Vec<SftpFileEntry> {
        let query = self.transfer.browser.search.trim().to_lowercase();
        let mut entries = self
            .transfer
            .browser
            .entries
            .iter()
            .filter(|entry| {
                transfer_browser_entry_is_visible(
                    entry,
                    &query,
                    self.settings.ui_file_explorer_show_hidden_files,
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            compare_transfer_browser_entries(
                left,
                right,
                self.transfer.browser.sort_column,
                self.transfer.browser.sort_direction,
            )
        });
        entries
    }

    pub(in crate::features::pages::transfers) fn toggle_transfer_browser_sort(
        &mut self,
        column: TransferBrowserSortColumn,
        cx: &mut Context<Self>,
    ) {
        if self.transfer.browser.sort_column == column {
            self.transfer.browser.sort_direction = self.transfer.browser.sort_direction.toggled();
        } else {
            self.transfer.browser.sort_column = column;
            self.transfer.browser.sort_direction = column.default_direction();
        }
        self.transfer.browser.list_offset = 0;
        self.transfer.browser.status = format!(
            "sorted by {} {}",
            self.transfer.browser.sort_column.label().to_lowercase(),
            self.transfer.browser.sort_direction.marker()
        );
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn handle_transfer_browser_search_key_down(
        &mut self,
        event: &KeyDownEvent,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.alt || keystroke.modifiers.control {
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => {
                self.transfer.browser.search.pop();
                self.transfer.browser.list_offset = 0;
                self.transfer.browser.status = transfer_browser_search_status(
                    self.transfer.browser.search.as_str(),
                    self.visible_transfer_browser_entries().len(),
                    self.transfer.browser.entries.len(),
                );
                cx.notify();
            }
            "escape" => {
                if self.transfer.browser.search.is_empty() {
                    self.transfer.browser.search_expanded = false;
                    self.transfer.browser.status = "file search closed".to_string();
                } else {
                    self.transfer.browser.search.clear();
                    self.transfer.browser.list_offset = 0;
                    self.transfer.browser.status = "file search cleared".to_string();
                }
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.transfer.browser.search.push_str(input);
                    self.transfer.browser.list_offset = 0;
                    self.transfer.browser.status = transfer_browser_search_status(
                        self.transfer.browser.search.as_str(),
                        self.visible_transfer_browser_entries().len(),
                        self.transfer.browser.entries.len(),
                    );
                    cx.notify();
                }
            }
        }
    }
}

fn transfer_browser_entry_is_visible(
    entry: &SftpFileEntry,
    normalized_query: &str,
    show_hidden_files: bool,
) -> bool {
    (show_hidden_files || !entry.name.starts_with('.'))
        && (normalized_query.is_empty() || entry.name.to_lowercase().contains(normalized_query))
}

#[cfg(test)]
mod tests {
    use super::transfer_browser_entry_is_visible;
    use nyaterm_transport::{SftpFileEntry, SftpFileType};

    fn entry(name: &str) -> SftpFileEntry {
        SftpFileEntry {
            name: name.to_string(),
            path: format!("/tmp/{name}"),
            file_type: SftpFileType::File,
            size: Some(0),
            permissions: None,
            owner: String::new(),
            group: String::new(),
            modified_at: None,
        }
    }

    #[test]
    fn hidden_entries_follow_visibility_setting_before_search() {
        let hidden = entry(".env");

        assert!(!transfer_browser_entry_is_visible(&hidden, "", false));
        assert!(!transfer_browser_entry_is_visible(&hidden, "env", false));
        assert!(transfer_browser_entry_is_visible(&hidden, "env", true));
    }

    #[test]
    fn visible_entries_still_follow_case_insensitive_search() {
        let visible = entry("ReleaseNotes.txt");

        assert!(transfer_browser_entry_is_visible(&visible, "notes", false));
        assert!(!transfer_browser_entry_is_visible(
            &visible, "archive", true
        ));
    }
}

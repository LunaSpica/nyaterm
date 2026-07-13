use super::*;

impl NyaTermApp {
    pub(in crate::ui::view::pages::transfers) fn visible_transfer_browser_entries(
        &self,
    ) -> Vec<SftpFileEntry> {
        let query = self.transfer_browser_search.trim().to_lowercase();
        let mut entries = self
            .transfer_browser_entries
            .iter()
            .filter(|entry| query.is_empty() || entry.name.to_lowercase().contains(&query))
            .cloned()
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            compare_transfer_browser_entries(
                left,
                right,
                self.transfer_browser_sort_column,
                self.transfer_browser_sort_direction,
            )
        });
        entries
    }

    pub(in crate::ui::view::pages::transfers) fn toggle_transfer_browser_sort(
        &mut self,
        column: TransferBrowserSortColumn,
        cx: &mut Context<Self>,
    ) {
        if self.transfer_browser_sort_column == column {
            self.transfer_browser_sort_direction = self.transfer_browser_sort_direction.toggled();
        } else {
            self.transfer_browser_sort_column = column;
            self.transfer_browser_sort_direction = column.default_direction();
        }
        self.transfer_browser_list_offset = 0;
        self.transfer_browser_status = format!(
            "sorted by {} {}",
            self.transfer_browser_sort_column.label().to_lowercase(),
            self.transfer_browser_sort_direction.marker()
        );
        cx.notify();
    }

    pub(in crate::ui::view::pages::transfers) fn handle_transfer_browser_search_key_down(
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
                self.transfer_browser_search.pop();
                self.transfer_browser_list_offset = 0;
                self.transfer_browser_status = transfer_browser_search_status(
                    self.transfer_browser_search.as_str(),
                    self.visible_transfer_browser_entries().len(),
                    self.transfer_browser_entries.len(),
                );
                cx.notify();
            }
            "escape" => {
                if self.transfer_browser_search.is_empty() {
                    self.transfer_browser_search_expanded = false;
                    self.transfer_browser_status = "file search closed".to_string();
                } else {
                    self.transfer_browser_search.clear();
                    self.transfer_browser_list_offset = 0;
                    self.transfer_browser_status = "file search cleared".to_string();
                }
                cx.notify();
            }
            _ => {
                if let Some(input) = keystroke
                    .key_char
                    .as_deref()
                    .filter(|input| !input.is_empty())
                {
                    self.transfer_browser_search.push_str(input);
                    self.transfer_browser_list_offset = 0;
                    self.transfer_browser_status = transfer_browser_search_status(
                        self.transfer_browser_search.as_str(),
                        self.visible_transfer_browser_entries().len(),
                        self.transfer_browser_entries.len(),
                    );
                    cx.notify();
                }
            }
        }
    }
}

use gpui::{Context, KeyDownEvent, Window};
use nyaterm_transport::SftpFileEntry;

use crate::features::{NyaTermApp, TextInputSetup};
use crate::models::TransferBrowserSortColumn;

use super::helpers::{compare_transfer_browser_entries, transfer_browser_search_status};

impl NyaTermApp {
    pub(in crate::features::pages::transfers) fn visible_transfer_browser_entries(
        &self,
    ) -> Vec<SftpFileEntry> {
        let browser = self.transfer.browser_view();
        let query = browser.search.trim().to_lowercase();
        let mut entries = browser
            .entries
            .iter()
            .filter(|entry| {
                transfer_browser_entry_is_visible(
                    entry,
                    &query,
                    self.settings.summary().ui_file_explorer_show_hidden_files,
                )
            })
            .cloned()
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            compare_transfer_browser_entries(
                left,
                right,
                browser.sort_column,
                browser.sort_direction,
            )
        });
        entries
    }

    pub(in crate::features::pages::transfers) fn toggle_transfer_browser_sort(
        &mut self,
        column: TransferBrowserSortColumn,
        cx: &mut Context<Self>,
    ) {
        self.transfer.toggle_browser_sort(column);
        cx.notify();
    }

    pub(in crate::features) fn apply_transfer_browser_search_input(
        &mut self,
        text: String,
        cx: &mut Context<Self>,
    ) {
        self.mark_user_activity();
        self.transfer.set_browser_search(text);
        let status = transfer_browser_search_status(
            self.transfer.browser_view().search.as_str(),
            self.visible_transfer_browser_entries().len(),
            self.transfer.browser_view().entries.len(),
        );
        self.transfer.set_browser_status(status);
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn focus_transfer_browser_search(
        &mut self,
        initial_text: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(text) = initial_text {
            self.transfer.set_browser_search(text);
            self.forget_text_inputs("transfer.browser.search");
        }
        self.transfer.expand_browser_search();
        let field = self.text_input(
            "transfer.browser.search",
            &self.transfer.browser_view().search.clone(),
            TextInputSetup::placeholder(self.tr("fileExplorer.searchPlaceholder")),
            cx,
        );
        window.focus(&field.read(cx).focus_handle(), cx);
        let status = transfer_browser_search_status(
            self.transfer.browser_view().search.as_str(),
            self.visible_transfer_browser_entries().len(),
            self.transfer.browser_view().entries.len(),
        );
        self.transfer.set_browser_status(status);
        cx.notify();
    }

    pub(in crate::features::pages::transfers) fn clear_or_close_transfer_browser_search(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.transfer.browser_view().search.is_empty() {
            self.transfer.close_browser_search();
            self.forget_text_inputs("transfer.browser.search");
            self.transfer.set_browser_status("file search closed");
            window.focus(self.transfer.browser_view().focus, cx);
        } else {
            self.transfer.clear_browser_search();
            self.reset_text_input("transfer.browser.search", "", cx);
            self.transfer.set_browser_status("file search cleared");
        }
        cx.notify();
    }
}

pub(super) fn transfer_browser_search_text_for_key(event: &KeyDownEvent) -> Option<String> {
    let keystroke = &event.keystroke;
    if keystroke.modifiers.alt
        || keystroke.modifiers.control
        || keystroke.modifiers.platform
        || keystroke.modifiers.function
    {
        return None;
    }
    keystroke
        .key_char
        .as_deref()
        .filter(|text| !text.is_empty() && !text.chars().any(char::is_control))
        .map(str::to_string)
        .or_else(|| (keystroke.key == "space").then(|| " ".to_string()))
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
    use super::{transfer_browser_entry_is_visible, transfer_browser_search_text_for_key};
    use gpui::{KeyDownEvent, Keystroke, Modifiers};
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

    fn key_event(key: &str, key_char: Option<&str>, modifiers: Modifiers) -> KeyDownEvent {
        KeyDownEvent {
            keystroke: Keystroke {
                modifiers,
                key: key.to_string(),
                key_char: key_char.map(str::to_string),
            },
            is_held: false,
            prefer_character_input: false,
        }
    }

    #[test]
    fn plain_text_keys_can_start_file_search() {
        assert_eq!(
            transfer_browser_search_text_for_key(&key_event("a", Some("A"), Modifiers::default())),
            Some("A".to_string())
        );
        assert_eq!(
            transfer_browser_search_text_for_key(&key_event("space", None, Modifiers::default())),
            Some(" ".to_string())
        );
    }

    #[test]
    fn shortcuts_and_control_characters_do_not_start_file_search() {
        let control = Modifiers {
            control: true,
            ..Modifiers::default()
        };
        assert_eq!(
            transfer_browser_search_text_for_key(&key_event("l", Some("l"), control)),
            None
        );
        assert_eq!(
            transfer_browser_search_text_for_key(&key_event(
                "tab",
                Some("\t"),
                Modifiers::default()
            )),
            None
        );
    }
}

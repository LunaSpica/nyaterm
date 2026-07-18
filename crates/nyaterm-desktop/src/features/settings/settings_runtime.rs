use super::*;

#[path = "settings_runtime/helpers.rs"]
mod helpers;
use helpers::*;

#[path = "settings_runtime/general_interaction.rs"]
mod general_interaction;
#[path = "settings_runtime/recording_transfer.rs"]
mod recording_transfer;
#[path = "settings_runtime/search_engines.rs"]
mod search_engines;
#[path = "settings_runtime/terminal_remote.rs"]
mod terminal_remote;

impl NyaTermApp {
    pub(in crate::features) fn open_external_url_for_ui(
        &mut self,
        url: &str,
        cx: &mut Context<Self>,
    ) {
        match open_external_url_simple(url) {
            Ok(()) => self.terminal_status = format!("opened URL: {url}"),
            Err(error) => self.terminal_status = format!("failed to open URL: {error}"),
        }
        cx.notify();
    }

    pub(in crate::features) fn open_documentation(&mut self, cx: &mut Context<Self>) {
        const DOCS_URL: &str = "https://nyaterm.app/docs/";
        self.open_external_url_for_ui(DOCS_URL, cx);
    }
}

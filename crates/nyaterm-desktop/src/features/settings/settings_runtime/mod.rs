use super::*;

mod helpers;
use helpers::*;

mod draft;
mod general_interaction;
mod recording_transfer;
mod search_engines;
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

mod ai;
mod app_state;
mod commands;
mod connections;
mod formatting;
mod icons;
mod inspector;
mod layout;
mod pages;
mod panels;
mod perf;
mod recording;
mod remote;
mod remote_desktop;
mod root;
mod runtime_jobs;
mod selects;
mod session;
mod settings;
mod shell;
mod sync;
mod sync_input;
mod terminal;
mod text_inputs;
mod transfers;
mod translation;
mod tunnels;
mod update;
mod view_widgets;

pub(crate) fn init(cx: &mut gpui::App) {
    terminal::init_key_bindings(cx);
}

pub use app_state::NyaTermApp;

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

/// Bundled SVG assets for the native shell (activity icons, logo, connection icons).
pub struct NyaTermAssets;

const ICON_FILES: &[&str] = &[
    "icons/logo.svg",
    "icons/files.svg",
    "icons/network.svg",
    "icons/auth.svg",
    "icons/sync.svg",
    "icons/settings.svg",
    "icons/connections.svg",
    "icons/ai.svg",
    "icons/sessions.svg",
    "icons/history.svg",
    "icons/resources.svg",
    "icons/processes.svg",
    "icons/docker.svg",
    "icons/commands.svg",
    "icons/send.svg",
    "icons/record.svg",
    "icons/lock.svg",
    "icons/chevron-down.svg",
    "icons/session/more.svg",
    "icons/session/save.svg",
    "icons/session/stop.svg",
    "icons/session/record.svg",
    "icons/session/disconnect.svg",
    "icons/session/reconnect.svg",
    "icons/session/rename.svg",
    "icons/translation.svg",
    "icons/migration.svg",
    "icons/conn/server.svg",
    "icons/conn/terminal.svg",
    "icons/conn/telnet.svg",
    "icons/conn/serial.svg",
    "icons/conn/linux.svg",
    "icons/conn/ubuntu.svg",
    "icons/conn/debian.svg",
    "icons/conn/apple.svg",
    "icons/conn/windows.svg",
    "icons/conn/docker.svg",
    "icons/conn/python.svg",
    "icons/conn/github.svg",
    "icons/conn/folder.svg",
    "icons/conn/file.svg",
    "icons/conn/connect.svg",
    "icons/net/move.svg",
    "icons/net/delete.svg",
    "icons/net/edit.svg",
    "icons/conn/sort.svg",
    "icons/conn/more.svg",
    "icons/conn/flash.svg",
    "icons/conn/add.svg",
    "icons/fe/new-file.svg",
    "icons/fe/new-folder.svg",
    "icons/fe/upload.svg",
    "icons/fe/upload-folder.svg",
    "icons/fe/download.svg",
    "icons/fe/delete.svg",
    "icons/fe/up.svg",
    "icons/fe/refresh.svg",
    "icons/fe/search.svg",
    "icons/fe/back.svg",
    "icons/fe/forward.svg",
    "icons/fe/star.svg",
    "icons/fe/star-outline.svg",
    "icons/fe/sync.svg",
    "icons/fe/paste.svg",
    "icons/ai/history.svg",
    "icons/ai/settings.svg",
    "icons/ai/new.svg",
    "icons/ai/send.svg",
    "icons/ai/stop.svg",
    "icons/ai/search.svg",
    "icons/ai/exec-confirm.svg",
    "icons/ai/exec-smart.svg",
    "icons/ai/exec-auto.svg",
];

fn icon_bytes(path: &str) -> Option<&'static [u8]> {
    match path {
        "icons/logo.svg" => Some(include_bytes!("../assets/icons/logo.svg")),
        "icons/files.svg" => Some(include_bytes!("../assets/icons/files.svg")),
        "icons/network.svg" => Some(include_bytes!("../assets/icons/network.svg")),
        "icons/auth.svg" => Some(include_bytes!("../assets/icons/auth.svg")),
        "icons/sync.svg" => Some(include_bytes!("../assets/icons/sync.svg")),
        "icons/settings.svg" => Some(include_bytes!("../assets/icons/settings.svg")),
        "icons/connections.svg" => Some(include_bytes!("../assets/icons/connections.svg")),
        "icons/ai.svg" => Some(include_bytes!("../assets/icons/ai.svg")),
        "icons/sessions.svg" => Some(include_bytes!("../assets/icons/sessions.svg")),
        "icons/history.svg" => Some(include_bytes!("../assets/icons/history.svg")),
        "icons/resources.svg" => Some(include_bytes!("../assets/icons/resources.svg")),
        "icons/processes.svg" => Some(include_bytes!("../assets/icons/processes.svg")),
        "icons/docker.svg" => Some(include_bytes!("../assets/icons/docker.svg")),
        "icons/commands.svg" => Some(include_bytes!("../assets/icons/commands.svg")),
        "icons/send.svg" => Some(include_bytes!("../assets/icons/send.svg")),
        "icons/record.svg" => Some(include_bytes!("../assets/icons/record.svg")),
        "icons/lock.svg" => Some(include_bytes!("../assets/icons/lock.svg")),
        "icons/chevron-down.svg" => Some(include_bytes!("../assets/icons/chevron-down.svg")),
        "icons/session/more.svg" => Some(include_bytes!("../assets/icons/session/more.svg")),
        "icons/session/save.svg" => Some(include_bytes!("../assets/icons/session/save.svg")),
        "icons/session/stop.svg" => Some(include_bytes!("../assets/icons/session/stop.svg")),
        "icons/session/record.svg" => Some(include_bytes!("../assets/icons/session/record.svg")),
        "icons/session/disconnect.svg" => {
            Some(include_bytes!("../assets/icons/session/disconnect.svg"))
        }
        "icons/session/reconnect.svg" => {
            Some(include_bytes!("../assets/icons/session/reconnect.svg"))
        }
        "icons/session/rename.svg" => Some(include_bytes!("../assets/icons/session/rename.svg")),
        "icons/translation.svg" => Some(include_bytes!("../assets/icons/translation.svg")),
        "icons/migration.svg" => Some(include_bytes!("../assets/icons/migration.svg")),
        "icons/conn/server.svg" => Some(include_bytes!("../assets/icons/conn/server.svg")),
        "icons/conn/terminal.svg" => Some(include_bytes!("../assets/icons/conn/terminal.svg")),
        "icons/conn/telnet.svg" => Some(include_bytes!("../assets/icons/conn/telnet.svg")),
        "icons/conn/serial.svg" => Some(include_bytes!("../assets/icons/conn/serial.svg")),
        "icons/conn/linux.svg" => Some(include_bytes!("../assets/icons/conn/linux.svg")),
        "icons/conn/ubuntu.svg" => Some(include_bytes!("../assets/icons/conn/ubuntu.svg")),
        "icons/conn/debian.svg" => Some(include_bytes!("../assets/icons/conn/debian.svg")),
        "icons/conn/apple.svg" => Some(include_bytes!("../assets/icons/conn/apple.svg")),
        "icons/conn/windows.svg" => Some(include_bytes!("../assets/icons/conn/windows.svg")),
        "icons/conn/docker.svg" => Some(include_bytes!("../assets/icons/conn/docker.svg")),
        "icons/conn/python.svg" => Some(include_bytes!("../assets/icons/conn/python.svg")),
        "icons/conn/github.svg" => Some(include_bytes!("../assets/icons/conn/github.svg")),
        "icons/conn/folder.svg" => Some(include_bytes!("../assets/icons/conn/folder.svg")),
        "icons/conn/file.svg" => Some(include_bytes!("../assets/icons/conn/file.svg")),
        "icons/conn/connect.svg" => Some(include_bytes!("../assets/icons/conn/connect.svg")),
        "icons/net/move.svg" => Some(include_bytes!("../assets/icons/net/move.svg")),
        "icons/net/delete.svg" => Some(include_bytes!("../assets/icons/net/delete.svg")),
        "icons/net/edit.svg" => Some(include_bytes!("../assets/icons/net/edit.svg")),
        "icons/conn/sort.svg" => Some(include_bytes!("../assets/icons/conn/sort.svg")),
        "icons/conn/more.svg" => Some(include_bytes!("../assets/icons/conn/more.svg")),
        "icons/conn/flash.svg" => Some(include_bytes!("../assets/icons/conn/flash.svg")),
        "icons/conn/add.svg" => Some(include_bytes!("../assets/icons/conn/add.svg")),
        "icons/fe/new-file.svg" => Some(include_bytes!("../assets/icons/fe/new-file.svg")),
        "icons/fe/new-folder.svg" => Some(include_bytes!("../assets/icons/fe/new-folder.svg")),
        "icons/fe/upload.svg" => Some(include_bytes!("../assets/icons/fe/upload.svg")),
        "icons/fe/upload-folder.svg" => {
            Some(include_bytes!("../assets/icons/fe/upload-folder.svg"))
        }
        "icons/fe/download.svg" => Some(include_bytes!("../assets/icons/fe/download.svg")),
        "icons/fe/delete.svg" => Some(include_bytes!("../assets/icons/fe/delete.svg")),
        "icons/fe/up.svg" => Some(include_bytes!("../assets/icons/fe/up.svg")),
        "icons/fe/refresh.svg" => Some(include_bytes!("../assets/icons/fe/refresh.svg")),
        "icons/fe/search.svg" => Some(include_bytes!("../assets/icons/fe/search.svg")),
        "icons/fe/back.svg" => Some(include_bytes!("../assets/icons/fe/back.svg")),
        "icons/fe/forward.svg" => Some(include_bytes!("../assets/icons/fe/forward.svg")),
        "icons/fe/star.svg" => Some(include_bytes!("../assets/icons/fe/star.svg")),
        "icons/fe/star-outline.svg" => Some(include_bytes!("../assets/icons/fe/star-outline.svg")),
        "icons/fe/sync.svg" => Some(include_bytes!("../assets/icons/fe/sync.svg")),
        "icons/fe/paste.svg" => Some(include_bytes!("../assets/icons/fe/paste.svg")),
        "icons/ai/history.svg" => Some(include_bytes!("../assets/icons/ai/history.svg")),
        "icons/ai/settings.svg" => Some(include_bytes!("../assets/icons/ai/settings.svg")),
        "icons/ai/new.svg" => Some(include_bytes!("../assets/icons/ai/new.svg")),
        "icons/ai/send.svg" => Some(include_bytes!("../assets/icons/ai/send.svg")),
        "icons/ai/stop.svg" => Some(include_bytes!("../assets/icons/ai/stop.svg")),
        "icons/ai/search.svg" => Some(include_bytes!("../assets/icons/ai/search.svg")),
        "icons/ai/exec-confirm.svg" => Some(include_bytes!("../assets/icons/ai/exec-confirm.svg")),
        "icons/ai/exec-smart.svg" => Some(include_bytes!("../assets/icons/ai/exec-smart.svg")),
        "icons/ai/exec-auto.svg" => Some(include_bytes!("../assets/icons/ai/exec-auto.svg")),
        _ => None,
    }
}

impl AssetSource for NyaTermAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let normalized = path.trim_start_matches('/');
        Ok(icon_bytes(normalized).map(|bytes| Cow::Borrowed(bytes)))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let normalized = path.trim_start_matches('/');
        if normalized.is_empty()
            || normalized == "icons"
            || normalized == "icons/conn"
            || normalized == "icons/fe"
            || normalized == "icons/ai"
        {
            Ok(ICON_FILES.iter().map(|item| (*item).into()).collect())
        } else {
            Ok(Vec::new())
        }
    }
}

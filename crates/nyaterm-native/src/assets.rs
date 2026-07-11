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
        if normalized.is_empty() || normalized == "icons" || normalized == "icons/conn" {
            Ok(ICON_FILES.iter().map(|item| (*item).into()).collect())
        } else {
            Ok(Vec::new())
        }
    }
}

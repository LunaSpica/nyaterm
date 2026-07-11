use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

/// Bundled SVG assets for the native shell (activity icons + logo).
pub struct NyaTermAssets;

impl AssetSource for NyaTermAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let normalized = path.trim_start_matches('/');
        let bytes = match normalized {
            "icons/logo.svg" => Some(&include_bytes!("../assets/icons/logo.svg")[..]),
            "icons/files.svg" => Some(&include_bytes!("../assets/icons/files.svg")[..]),
            "icons/network.svg" => Some(&include_bytes!("../assets/icons/network.svg")[..]),
            "icons/auth.svg" => Some(&include_bytes!("../assets/icons/auth.svg")[..]),
            "icons/sync.svg" => Some(&include_bytes!("../assets/icons/sync.svg")[..]),
            "icons/settings.svg" => Some(&include_bytes!("../assets/icons/settings.svg")[..]),
            "icons/connections.svg" => Some(&include_bytes!("../assets/icons/connections.svg")[..]),
            "icons/ai.svg" => Some(&include_bytes!("../assets/icons/ai.svg")[..]),
            "icons/sessions.svg" => Some(&include_bytes!("../assets/icons/sessions.svg")[..]),
            "icons/history.svg" => Some(&include_bytes!("../assets/icons/history.svg")[..]),
            "icons/resources.svg" => Some(&include_bytes!("../assets/icons/resources.svg")[..]),
            "icons/processes.svg" => Some(&include_bytes!("../assets/icons/processes.svg")[..]),
            "icons/docker.svg" => Some(&include_bytes!("../assets/icons/docker.svg")[..]),
            "icons/commands.svg" => Some(&include_bytes!("../assets/icons/commands.svg")[..]),
            "icons/send.svg" => Some(&include_bytes!("../assets/icons/send.svg")[..]),
            "icons/record.svg" => Some(&include_bytes!("../assets/icons/record.svg")[..]),
            "icons/lock.svg" => Some(&include_bytes!("../assets/icons/lock.svg")[..]),
            "icons/translation.svg" => Some(&include_bytes!("../assets/icons/translation.svg")[..]),
            "icons/migration.svg" => Some(&include_bytes!("../assets/icons/migration.svg")[..]),
            _ => None,
        };
        Ok(bytes.map(Cow::Borrowed))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let normalized = path.trim_start_matches('/');
        if normalized.is_empty() || normalized == "icons" {
            Ok(vec![
                "icons/logo.svg".into(),
                "icons/files.svg".into(),
                "icons/network.svg".into(),
                "icons/auth.svg".into(),
                "icons/sync.svg".into(),
                "icons/settings.svg".into(),
                "icons/connections.svg".into(),
                "icons/ai.svg".into(),
                "icons/sessions.svg".into(),
                "icons/history.svg".into(),
                "icons/resources.svg".into(),
                "icons/processes.svg".into(),
                "icons/docker.svg".into(),
                "icons/commands.svg".into(),
                "icons/send.svg".into(),
                "icons/record.svg".into(),
                "icons/lock.svg".into(),
                "icons/translation.svg".into(),
                "icons/migration.svg".into(),
            ])
        } else {
            Ok(Vec::new())
        }
    }
}

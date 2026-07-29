//! Fill in a saved connection's icon from what the remote host reports.
//!
//! Runs off the remote-stats snapshot the resource panel already collects, so no
//! extra round trip is made. Only ever fills in a blank or refreshes a previous
//! auto-detection — a hand-picked icon clears the flag and is never touched.

use nyaterm_core::ConnectionType;
use nyaterm_transport::SystemInfo;

use crate::features::{NyaTermApp, infer_connection_icon_key_from_remote_system};

impl NyaTermApp {
    pub(in crate::features) fn apply_auto_detected_connection_icon(
        &mut self,
        session_id: &str,
        system: &SystemInfo,
    ) {
        let Some(connection_id) = self
            .session
            .metadata(session_id)
            .and_then(|metadata| metadata.source_connection_id.as_deref())
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(ToOwned::to_owned)
        else {
            return;
        };

        let Some(connection) = self
            .connection_catalog
            .connections()
            .iter()
            .find(|connection| connection.id == connection_id)
        else {
            return;
        };

        // Only SSH sessions report a remote system worth reading.
        if !matches!(connection.config, ConnectionType::Ssh { .. })
            || !connection.icon_auto_detect_enabled()
        {
            return;
        }

        let Some(icon_key) = infer_connection_icon_key_from_remote_system(&system.os, &system.arch)
        else {
            return;
        };
        if connection.icon.as_deref() == Some(icon_key) {
            return;
        }

        let mut updated = connection.clone();
        updated.icon = Some(icon_key.to_string());
        updated.icon_auto_detect = Some(true);

        match self.persist_saved_connection_with_group(updated, None) {
            Ok(connection) => {
                self.terminal.view.status =
                    format!("detected {icon_key} icon for {}", connection.name);
            }
            Err(error) => {
                // A failed icon refresh must not disturb the session, so this is
                // reported as a log line rather than surfaced to the user.
                tracing::warn!(
                    target: "nyaterm::connections",
                    connection_id = %connection_id,
                    %error,
                    "failed to persist auto-detected connection icon"
                );
            }
        }
    }
}

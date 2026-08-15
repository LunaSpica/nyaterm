use std::path::{Path, PathBuf};

use nyaterm_core::{OtpEntry, SavedCredential, SavedPassword, SshKey};
use nyaterm_store::ConnectionStore;

use crate::features::SecurityCatalogState;

pub(super) struct SecurityStoreLocation {
    config_dir: PathBuf,
    portable_key_path: Option<PathBuf>,
}

impl SecurityStoreLocation {
    pub(super) fn new(config_dir: &Path, portable_key_path: Option<PathBuf>) -> Self {
        Self {
            config_dir: config_dir.to_path_buf(),
            portable_key_path,
        }
    }

    pub(super) fn open(&self) -> Result<ConnectionStore, String> {
        ConnectionStore::open_with_portable_key_path(
            &self.config_dir,
            self.portable_key_path.clone(),
        )
        .map_err(|error| error.to_string())
    }
}

pub(super) fn load_security_catalog(
    store: &ConnectionStore,
) -> Result<SecurityCatalogState, String> {
    let ssh_keys: Vec<SshKey> = store.list_ssh_keys().map_err(|error| error.to_string())?;
    let otp_entries: Vec<OtpEntry> = store
        .list_otp_entries()
        .map_err(|error| error.to_string())?;
    let passwords: Vec<SavedPassword> =
        store.list_passwords().map_err(|error| error.to_string())?;
    let credentials: Vec<SavedCredential> = store
        .list_credentials()
        .map_err(|error| error.to_string())?;
    Ok(SecurityCatalogState::new(
        ssh_keys,
        otp_entries,
        passwords,
        credentials,
    ))
}

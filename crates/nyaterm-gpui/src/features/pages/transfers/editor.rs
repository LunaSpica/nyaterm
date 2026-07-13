use super::*;
use crate::models::NavItem;
use nyaterm_transport::{SftpTransferControl, SftpTransferOptions, SshSessionConfig};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const EXTERNAL_EDITOR_WATCH_INTERVAL: Duration = Duration::from_millis(1000);
const EXTERNAL_EDITOR_UPLOAD_SETTLE: Duration = Duration::from_millis(450);
const EXTERNAL_EDITOR_STARTUP_SUPPRESSION: Duration = Duration::from_secs(2);


#[path = "editor/helpers.rs"]
mod helpers;
use helpers::*;

#[path = "editor/open.rs"]
mod open;
#[path = "editor/lifecycle.rs"]
mod lifecycle;
#[path = "editor/input_sync.rs"]
mod input_sync;

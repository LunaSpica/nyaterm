use std::time::Duration;

const EXTERNAL_EDITOR_WATCH_INTERVAL: Duration = Duration::from_millis(1000);
const EXTERNAL_EDITOR_UPLOAD_SETTLE: Duration = Duration::from_millis(450);
const EXTERNAL_EDITOR_STARTUP_SUPPRESSION: Duration = Duration::from_secs(2);

mod helpers;
mod input_sync;
mod lifecycle;
mod open;

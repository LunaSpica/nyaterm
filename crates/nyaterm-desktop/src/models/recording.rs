use std::sync::{Arc, mpsc};
use std::thread;

use nyaterm_transport::RecordingManager;

#[derive(Clone)]
pub(crate) struct RecordingWritePipeline {
    command_tx: mpsc::Sender<RecordingWriteCommand>,
}

impl RecordingWritePipeline {
    pub(crate) fn spawn(recording_manager: Arc<RecordingManager>) -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        thread::Builder::new()
            .name("nyaterm-recording-writer".to_string())
            .spawn(move || run_recording_writer(recording_manager, command_rx))
            .expect("failed to spawn recording writer");
        Self { command_tx }
    }

    pub(crate) fn write_output(&self, session_id: impl Into<String>, text: impl Into<String>) {
        let text = text.into();
        if text.is_empty() {
            return;
        }
        let _ = self.command_tx.send(RecordingWriteCommand::WriteOutput {
            session_id: session_id.into(),
            text,
        });
    }

    pub(crate) fn write_input(&self, session_id: impl Into<String>, data: impl Into<Vec<u8>>) {
        let data = data.into();
        if data.is_empty() {
            return;
        }
        let _ = self.command_tx.send(RecordingWriteCommand::WriteInput {
            session_id: session_id.into(),
            data,
        });
    }

    pub(crate) fn write_raw_input(&self, session_id: impl Into<String>, data: impl Into<Vec<u8>>) {
        let data = data.into();
        if data.is_empty() {
            return;
        }
        let _ = self.command_tx.send(RecordingWriteCommand::WriteRawInput {
            session_id: session_id.into(),
            data,
        });
    }

    pub(crate) fn cleanup_session(&self, session_id: impl Into<String>) {
        let _ = self.command_tx.send(RecordingWriteCommand::CleanupSession {
            session_id: session_id.into(),
        });
    }

    pub(crate) fn flush(&self) {
        let (ack_tx, ack_rx) = mpsc::sync_channel(0);
        if self
            .command_tx
            .send(RecordingWriteCommand::Flush { ack_tx })
            .is_ok()
        {
            let _ = ack_rx.recv();
        }
    }
}

#[derive(Debug)]
enum RecordingWriteCommand {
    WriteOutput { session_id: String, text: String },
    WriteInput { session_id: String, data: Vec<u8> },
    WriteRawInput { session_id: String, data: Vec<u8> },
    CleanupSession { session_id: String },
    Flush { ack_tx: mpsc::SyncSender<()> },
}

fn run_recording_writer(
    recording_manager: Arc<RecordingManager>,
    command_rx: mpsc::Receiver<RecordingWriteCommand>,
) {
    while let Ok(command) = command_rx.recv() {
        match command {
            RecordingWriteCommand::WriteOutput { session_id, text } => {
                recording_manager.write_output(&session_id, &text);
            }
            RecordingWriteCommand::WriteInput { session_id, data } => {
                recording_manager.write_input(&session_id, &data);
            }
            RecordingWriteCommand::WriteRawInput { session_id, data } => {
                recording_manager.write_raw_input(&session_id, &data);
            }
            RecordingWriteCommand::CleanupSession { session_id } => {
                recording_manager.cleanup_session(&session_id);
            }
            RecordingWriteCommand::Flush { ack_tx } => {
                let _ = ack_tx.send(());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn recording_pipeline_preserves_write_order_before_flush() {
        let manager = Arc::new(RecordingManager::new());
        let pipeline = RecordingWritePipeline::spawn(Arc::clone(&manager));
        let session_id = "session-a";
        pipeline.write_input(session_id, b"echo hello\r".to_vec());
        pipeline.write_output(session_id, "hello\n");
        pipeline.flush();

        let results = manager
            .search_history(nyaterm_transport::TerminalHistorySearchRequest {
                session_id: session_id.to_string(),
                query: "hello".to_string(),
                case_sensitive: false,
                regex: false,
                whole_word: false,
                limit: Some(10),
                context_before: Some(0),
                context_after: Some(0),
                max_lines: None,
            })
            .expect("search should succeed");
        assert_eq!(results.total, 2);
        assert_eq!(results.results[0].source, "input");
        assert_eq!(results.results[1].source, "output");
    }

    #[test]
    fn recording_pipeline_cleanup_runs_after_queued_writes() {
        let manager = Arc::new(RecordingManager::new());
        let pipeline = RecordingWritePipeline::spawn(Arc::clone(&manager));
        let session_id = "session-b";
        let path = unique_recording_path("pipeline-cleanup");
        manager
            .start(session_id, &path, true, false)
            .expect("recording should start");

        pipeline.write_output(session_id, "before cleanup\n");
        pipeline.cleanup_session(session_id);
        pipeline.flush();

        let text = fs::read_to_string(&path).expect("recording file should exist");
        assert!(text.contains("before cleanup"));
        let _ = fs::remove_file(path);
    }

    fn unique_recording_path(name: &str) -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("nyaterm-{name}-{nanos}.log"))
            .to_string_lossy()
            .to_string()
    }
}

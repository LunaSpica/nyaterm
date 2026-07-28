//! Background runtime ownership shared by command history and quick commands.

use std::path::PathBuf;
use std::sync::mpsc;

use crate::features::{
    CommandPersistenceRequest, CommandPersistenceResult, spawn_command_persistence_worker,
};

pub(in crate::features) struct CommandRuntimeState {
    tx: mpsc::Sender<CommandPersistenceRequest>,
    rx: mpsc::Receiver<CommandPersistenceResult>,
    pending: usize,
}

pub(in crate::features) enum CommandPersistencePoll {
    Event(CommandPersistenceResult),
    Empty,
    Disconnected { had_pending: bool },
}

impl CommandRuntimeState {
    pub(in crate::features) fn new(
        config_dir: PathBuf,
        portable_key_path: Option<PathBuf>,
    ) -> Self {
        let (tx, rx) = spawn_command_persistence_worker(config_dir, portable_key_path);
        Self { tx, rx, pending: 0 }
    }

    pub(in crate::features) fn queue(&mut self, request: CommandPersistenceRequest) -> bool {
        if self.tx.send(request).is_err() {
            return false;
        }
        self.pending = self.pending.saturating_add(1);
        true
    }

    pub(in crate::features) fn poll(&mut self) -> CommandPersistencePoll {
        match self.rx.try_recv() {
            Ok(event) => {
                self.pending = self.pending.saturating_sub(1);
                CommandPersistencePoll::Event(event)
            }
            Err(mpsc::TryRecvError::Empty) => CommandPersistencePoll::Empty,
            Err(mpsc::TryRecvError::Disconnected) => {
                let had_pending = std::mem::take(&mut self.pending) > 0;
                CommandPersistencePoll::Disconnected { had_pending }
            }
        }
    }

    pub(in crate::features) fn is_idle(&self) -> bool {
        self.pending == 0
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::{CommandPersistencePoll, CommandRuntimeState};
    use crate::features::{CommandPersistenceRequest, CommandPersistenceResult};

    #[test]
    fn command_runtime_owns_pending_request_lifecycle() {
        let (request_tx, request_rx) = mpsc::channel();
        let (result_tx, result_rx) = mpsc::channel();
        let mut runtime = CommandRuntimeState {
            tx: request_tx,
            rx: result_rx,
            pending: 0,
        };

        assert!(runtime.is_idle());
        assert!(runtime.queue(CommandPersistenceRequest::AppendHistory(vec![
            "pwd".to_string(),
        ])));
        assert!(!runtime.is_idle());
        assert!(matches!(
            request_rx.recv().expect("request should reach worker"),
            CommandPersistenceRequest::AppendHistory(commands) if commands == ["pwd"]
        ));

        result_tx
            .send(CommandPersistenceResult::History(Ok(Vec::new())))
            .expect("result channel should stay connected");
        assert!(matches!(
            runtime.poll(),
            CommandPersistencePoll::Event(CommandPersistenceResult::History(Ok(history)))
                if history.is_empty()
        ));
        assert!(runtime.is_idle());

        assert!(runtime.queue(CommandPersistenceRequest::AppendHistory(Vec::new())));
        drop(result_tx);
        assert!(matches!(
            runtime.poll(),
            CommandPersistencePoll::Disconnected { had_pending: true }
        ));
        assert!(runtime.is_idle());
    }
}

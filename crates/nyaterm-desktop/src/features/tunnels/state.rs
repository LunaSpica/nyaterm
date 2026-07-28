use std::sync::{Arc, mpsc};

use nyaterm_transport::SshTunnelManager;

use crate::features::TunnelJobResult;

pub(in crate::features) struct TunnelFeatureState {
    pub manager: Arc<SshTunnelManager>,
    tx: mpsc::Sender<TunnelJobResult>,
    rx: mpsc::Receiver<TunnelJobResult>,
    pending: Vec<String>,
}

impl TunnelFeatureState {
    pub(in crate::features) fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            manager: Arc::new(SshTunnelManager::new()),
            tx,
            rx,
            pending: Vec::new(),
        }
    }

    pub(in crate::features) fn sender(&self) -> mpsc::Sender<TunnelJobResult> {
        self.tx.clone()
    }

    pub(in crate::features) fn is_pending(&self, tunnel_id: &str) -> bool {
        self.pending.iter().any(|id| id == tunnel_id)
    }

    pub(in crate::features) fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    pub(in crate::features) fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub(in crate::features) fn mark_pending(&mut self, tunnel_id: String) {
        self.pending.push(tunnel_id);
    }

    pub(in crate::features) fn finish(&mut self, tunnel_id: &str) {
        self.pending.retain(|id| id != tunnel_id);
    }

    pub(in crate::features) fn try_recv(&self) -> Result<TunnelJobResult, mpsc::TryRecvError> {
        self.rx.try_recv()
    }
}

#[cfg(test)]
mod tests {
    use crate::features::TunnelJobResult;

    use super::TunnelFeatureState;

    #[test]
    fn tunnel_state_owns_job_channel_and_pending_lifecycle() {
        let mut tunnels = TunnelFeatureState::new();
        tunnels.mark_pending("tunnel-1".to_string());

        assert!(tunnels.has_pending());
        assert!(tunnels.is_pending("tunnel-1"));
        assert_eq!(tunnels.pending_count(), 1);

        tunnels
            .sender()
            .send(TunnelJobResult {
                tunnel_id: "tunnel-1".to_string(),
                result: Err("failed".to_string()),
            })
            .expect("tunnel event channel should stay connected");
        let event = tunnels
            .try_recv()
            .expect("tunnel event should be owned by the state");
        tunnels.finish(&event.tunnel_id);

        assert!(!tunnels.has_pending());
        assert!(!tunnels.is_pending("tunnel-1"));
    }
}

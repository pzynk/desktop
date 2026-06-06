//! Mediates pending pair requests between TCP sessions and the Tauri UI.

use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::Mutex;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairRequestEvent {
    pub device_id: String,
    pub name: String,
    pub code: String,
}

#[derive(Debug, Clone, Copy)]
pub enum PairDecision {
    Accept,
    Reject,
}

struct PendingPairRequest {
    event: PairRequestEvent,
    decision: Sender<PairDecision>,
}

#[derive(Default)]
pub struct PairingManager {
    pending: Mutex<HashMap<String, PendingPairRequest>>,
}

impl PairingManager {
    pub fn insert(&self, event: PairRequestEvent, decision: Sender<PairDecision>) {
        let mut pending = self.pending.lock().expect("pairing mutex poisoned");
        pending.insert(
            event.device_id.clone(),
            PendingPairRequest { event, decision },
        );
    }

    pub fn resolve(&self, device_id: &str, decision: PairDecision) -> Result<(), String> {
        let pending = self
            .pending
            .lock()
            .expect("pairing mutex poisoned")
            .remove(device_id)
            .ok_or_else(|| "No pending pair request for that device".to_string())?;
        pending
            .decision
            .send(decision)
            .map_err(|_| "Pairing session is no longer active".to_string())
    }

    pub fn list(&self) -> Vec<PairRequestEvent> {
        self.pending
            .lock()
            .expect("pairing mutex poisoned")
            .values()
            .map(|p| p.event.clone())
            .collect()
    }

    pub fn remove(&self, device_id: &str) {
        self.pending
            .lock()
            .expect("pairing mutex poisoned")
            .remove(device_id);
    }
}

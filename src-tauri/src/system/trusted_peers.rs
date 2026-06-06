//! JSON-backed store for paired devices.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedPeer {
    pub device_id: String,
    pub name: String,
    pub token: String,
    pub last_seen: i64,
    #[serde(default = "default_clipboard_sync")]
    pub clipboard_sync_enabled: bool,
    #[serde(default = "default_media_controls")]
    pub media_controls_enabled: bool,
    #[serde(default = "default_volume_sync")]
    pub volume_sync_enabled: bool,
    #[serde(default = "default_incoming_files")]
    pub incoming_files_enabled: bool,
    #[serde(default = "default_terminal_access")]
    pub terminal_access_enabled: bool,
}

fn default_clipboard_sync() -> bool {
    true
}

fn default_media_controls() -> bool {
    true
}

fn default_volume_sync() -> bool {
    true
}

fn default_incoming_files() -> bool {
    true
}

fn default_terminal_access() -> bool {
    false
}


#[derive(Debug)]
pub struct TrustedPeers {
    path: PathBuf,
    peers: HashMap<String, TrustedPeer>,
}

impl TrustedPeers {
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let dir = app
            .path()
            .app_data_dir()
            .map_err(|e| format!("Could not resolve app data dir: {e}"))?;
        fs::create_dir_all(&dir).map_err(|e| format!("Could not create app data dir: {e}"))?;
        let path = dir.join("trusted_peers.json");
        let peers = match fs::read_to_string(&path) {
            Ok(raw) => serde_json::from_str::<Vec<TrustedPeer>>(&raw)
                .unwrap_or_default()
                .into_iter()
                .map(|peer| (peer.device_id.clone(), peer))
                .collect(),
            Err(_) => HashMap::new(),
        };
        Ok(Self { path, peers })
    }

    pub fn get(&self, device_id: &str) -> Option<&TrustedPeer> {
        self.peers.get(device_id)
    }

    pub fn all(&self) -> Vec<TrustedPeer> {
        self.peers.values().cloned().collect()
    }

    pub fn upsert(&mut self, peer: TrustedPeer) -> Result<(), String> {
        self.peers.insert(peer.device_id.clone(), peer);
        self.save()
    }

    pub fn touch(&mut self, device_id: &str) -> Result<(), String> {
        if let Some(peer) = self.peers.get_mut(device_id) {
            peer.last_seen = now_unix();
            self.save()?;
        }
        Ok(())
    }

    pub fn remove(&mut self, device_id: &str) -> Result<(), String> {
        self.peers.remove(device_id);
        self.save()
    }

    fn save(&self) -> Result<(), String> {
        let peers: Vec<_> = self.peers.values().cloned().collect();
        let raw = serde_json::to_string_pretty(&peers)
            .map_err(|e| format!("Could not serialize trusted peers: {e}"))?;
        fs::write(&self.path, raw).map_err(|e| format!("Could not save trusted peers: {e}"))
    }
}

pub fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

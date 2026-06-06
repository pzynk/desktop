//! Commands that expose information about the local device to the UI.

use crate::system;
use crate::app::AppState;
use tauri::{State, Emitter};

/// Returns the local IPv4 address used to reach the LAN.
#[tauri::command]
pub async fn get_device_ip() -> String {
    system::get_local_ip().unwrap_or_else(|| "127.0.0.1".to_string())
}

/// Returns this device's hostname.
#[tauri::command]
pub async fn get_device_name() -> String {
    system::get_system_name().unwrap_or_else(|| "Unknown".to_string())
}

#[tauri::command]
pub async fn set_device_incoming_files(
    device_id: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut peers = state.trusted_peers.lock().unwrap();
    if let Some(peer) = peers.get(&device_id) {
        let mut updated_peer = peer.clone();
        updated_peer.incoming_files_enabled = enabled;
        peers.upsert(updated_peer)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn set_device_terminal_access(
    device_id: String,
    enabled: bool,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    {
        let mut peers = state.trusted_peers.lock().unwrap();
        if let Some(peer) = peers.get(&device_id) {
            let mut updated_peer = peer.clone();
            updated_peer.terminal_access_enabled = enabled;
            peers.upsert(updated_peer)?;
        }
    }
    let _ = app.emit("terminal-access-changed", device_id);
    Ok(())
}

use crate::app::TransferProgress;

#[tauri::command]
pub async fn get_active_transfer(
    state: State<'_, AppState>,
) -> Result<Option<TransferProgress>, String> {
    Ok(state.active_transfer.lock().unwrap().clone())
}

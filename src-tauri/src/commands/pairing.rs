use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use crate::app::{refresh_tray_menu, AppState, PEERS_CHANGED_EVENT};
use crate::network::pairing::{PairDecision, PairRequestEvent};

#[derive(Serialize)]
pub struct TrustedPeerDto {
    pub device_id: String,
    pub name: String,
    pub last_seen: i64,
    pub connected: bool,
    pub clipboard_sync_enabled: bool,
    pub media_controls_enabled: bool,
    pub volume_sync_enabled: bool,
    pub incoming_files_enabled: bool,
    pub terminal_access_enabled: bool,
}

#[tauri::command]
pub async fn list_pending_pair_requests(state: State<'_, AppState>) -> Result<Vec<PairRequestEvent>, String> {
    Ok(state.pairing.list())
}

#[tauri::command]
pub async fn accept_pair_request(device_id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.pairing.resolve(&device_id, PairDecision::Accept)
}

#[tauri::command]
pub async fn reject_pair_request(device_id: String, state: State<'_, AppState>) -> Result<(), String> {
    state.pairing.resolve(&device_id, PairDecision::Reject)
}

#[tauri::command]
pub async fn list_trusted_peers(state: State<'_, AppState>) -> Result<Vec<TrustedPeerDto>, String> {
    let active = state
        .active_connections
        .lock()
        .expect("active connections mutex poisoned");
    let peers = state
        .trusted_peers
        .lock()
        .expect("trusted peers mutex poisoned")
        .all()
        .into_iter()
        .map(|peer| TrustedPeerDto {
            connected: active.contains(&peer.device_id),
            device_id: peer.device_id,
            name: peer.name,
            last_seen: peer.last_seen,
            clipboard_sync_enabled: peer.clipboard_sync_enabled,
            media_controls_enabled: peer.media_controls_enabled,
            volume_sync_enabled: peer.volume_sync_enabled,
            incoming_files_enabled: peer.incoming_files_enabled,
            terminal_access_enabled: peer.terminal_access_enabled,
        })
        .collect();
    Ok(peers)
}

#[tauri::command]
pub async fn unpair_peer(
    device_id: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .trusted_peers
        .lock()
        .expect("trusted peers mutex poisoned")
        .remove(&device_id)?;
    state
        .active_connections
        .lock()
        .expect("active connections mutex poisoned")
        .remove(&device_id);
    app.emit(PEERS_CHANGED_EVENT, ()).ok();
    refresh_tray_menu(&app).ok();
    Ok(())
}

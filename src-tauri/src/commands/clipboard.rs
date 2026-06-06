use crate::app::AppState;
use tauri::State;

#[tauri::command]
pub async fn set_device_clipboard_sync(
    device_id: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut peers = state.trusted_peers.lock().unwrap();
    if let Some(peer) = peers.get(&device_id) {
        let mut updated_peer = peer.clone();
        updated_peer.clipboard_sync_enabled = enabled;
        peers.upsert(updated_peer)?;
    }
    Ok(())
}

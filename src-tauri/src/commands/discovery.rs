//! Commands that control UDP broadcast visibility.

use tauri::State;

use crate::app::AppState;

/// Enable or disable the UDP discovery broadcast.
/// When disabled this device stops advertising itself on the LAN;
/// already-paired connections are unaffected.
#[tauri::command]
pub async fn set_broadcasting(enabled: bool, state: State<'_, AppState>) -> Result<(), String> {
    state.set_broadcasting(enabled);
    println!("[discovery] broadcasting set to {enabled}");
    Ok(())
}

/// Returns whether the UDP discovery broadcast is currently active.
#[tauri::command]
pub async fn get_broadcasting(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.is_broadcasting())
}

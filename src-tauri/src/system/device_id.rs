//! Persistent desktop device identity.

use std::fs;

use tauri::{AppHandle, Manager};
use uuid::Uuid;

pub fn get_or_create_device_id(app: &AppHandle) -> Result<String, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("Could not resolve app data dir: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("Could not create app data dir: {e}"))?;

    let path = dir.join("device_id.txt");
    if let Ok(existing) = fs::read_to_string(&path) {
        let trimmed = existing.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    let id = Uuid::new_v4().to_string();
    fs::write(&path, &id).map_err(|e| format!("Could not persist device id: {e}"))?;
    Ok(id)
}

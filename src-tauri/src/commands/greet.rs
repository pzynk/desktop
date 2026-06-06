//! The original sample command, kept as a smoke test for the
//! frontend ↔ backend bridge.

#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

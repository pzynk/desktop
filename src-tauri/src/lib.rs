//! `tauri_app_lib` – backend library for the desktop side of the
//! Android ↔ Desktop sync app.
//!
//! Module layout:
//!
//! ```text
//! src/
//! ├── app.rs        // Tauri bootstrap + service wiring
//! ├── commands/     // #[tauri::command] handlers exposed to the UI
//! ├── network/      // Discovery (UDP) + TCP server + wire types
//! └── system/       // Host info helpers (hostname, local IP, ...)
//! ```
//!
//! Keep cross-module dependencies pointing "downward":
//! `app` → `commands` → `network` / `system`.

mod app;
mod commands;
mod network;
mod system;

pub use app::run;

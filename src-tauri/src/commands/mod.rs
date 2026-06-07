//! Tauri commands exposed to the frontend.
//!
//! Keep this module thin: each command should delegate to the
//! appropriate domain module (`network`, `system`, ...) rather than
//! containing business logic itself.

pub mod clipboard;
pub mod device;
pub mod discovery;
pub mod greet;
pub mod media;
pub mod pairing;
pub mod updater;
pub mod camera;

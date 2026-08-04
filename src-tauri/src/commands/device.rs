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

#[tauri::command]
pub async fn set_device_audio_streaming(
    device_id: String,
    enabled: bool,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let peer_name = {
        let mut peers = state.trusted_peers.lock().unwrap();
        let name = peers.get(&device_id).map(|p| p.name.clone()).unwrap_or_default();
        if let Some(peer) = peers.get(&device_id) {
            let mut updated_peer = peer.clone();
            updated_peer.audio_streaming_enabled = enabled;
            let _ = peers.upsert(updated_peer);
        }
        name
    };

    let mut active_streams = state.active_streams.lock().unwrap();
    if let Some(stream) = active_streams.get_mut(&device_id) {
        let mut audio_server = state.audio_stream_server.lock().unwrap();
        if enabled {
            if audio_server.is_none() {
                match crate::system::audio::AudioStreamServer::start() {
                    Ok(server) => {
                        let port = server.port;
                        *audio_server = Some(server);
                        let msg = crate::network::protocol::ServerMessage::AudioStreamInfo { enabled: true, port };
                        let _ = crate::network::protocol::write_line_json(stream, &msg);
                        
                        let _ = notify_rust::Notification::new()
                            .summary("Listen Through Mobile")
                            .body(&format!("Audio is now streaming to {}", peer_name))
                            .show();
                    }
                    Err(e) => {
                        eprintln!("[audio] Failed to start audio server: {}", e);
                    }
                }
            } else {
                let port = audio_server.as_ref().unwrap().port;
                let msg = crate::network::protocol::ServerMessage::AudioStreamInfo { enabled: true, port };
                let _ = crate::network::protocol::write_line_json(stream, &msg);
                
                let _ = notify_rust::Notification::new()
                    .summary("Listen Through Mobile")
                    .body(&format!("Audio is now streaming to {}", peer_name))
                    .show();
            }
        } else {
            *audio_server = None;
            let msg = crate::network::protocol::ServerMessage::AudioStreamInfo { enabled: false, port: 0 };
            let _ = crate::network::protocol::write_line_json(stream, &msg);
        }
    }
    Ok(())
}

#[derive(Debug, serde::Serialize)]
pub struct MissingDependency {
    pub feature: String,
    pub title: String,
    pub package: String,
    pub command: String,
    pub description: String,
}

#[tauri::command]
pub async fn check_system_deps(feature: String) -> Result<Option<MissingDependency>, String> {
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        if feature == "mic" {
            let pactl_ok = Command::new("pactl").arg("--version").output().map(|o| o.status.success()).unwrap_or(false);
            let pacat_ok = Command::new("pacat").arg("--version").output().map(|o| o.status.success()).unwrap_or(false);
            if !pactl_ok || !pacat_ok {
                return Ok(Some(MissingDependency {
                    feature: "mic".to_string(),
                    title: "PulseAudio Utilities Required".to_string(),
                    package: "pulseaudio-utils".to_string(),
                    command: "sudo apt install pulseaudio-utils".to_string(),
                    description: "Required to create the system 'Sync Microphone' virtual input device on Linux.".to_string(),
                }));
            }
        } else if feature == "camera" {
            let v4l2_ok = Command::new("v4l2-ctl").arg("--version").output().map(|o| o.status.success()).unwrap_or(false);
            let mod_ok = std::path::Path::new("/dev/video9").exists() || Command::new("modinfo").arg("v4l2loopback").output().map(|o| o.status.success()).unwrap_or(false);
            if !v4l2_ok || !mod_ok {
                return Ok(Some(MissingDependency {
                    feature: "camera".to_string(),
                    title: "V4L2 Loopback Module Required".to_string(),
                    package: "v4l2loopback-dkms v4l-utils".to_string(),
                    command: "sudo apt install v4l2loopback-dkms v4l-utils && sudo modprobe v4l2loopback exclusive_caps=1 card_label=\"Sync Camera\" video_nr=9".to_string(),
                    description: "Required to create the system '/dev/video9' virtual webcam device on Linux.".to_string(),
                }));
            }
        }
    }
    let _ = feature;
    Ok(None)
}

#[cfg(target_os = "linux")]
use dbus::blocking::Connection;
#[cfg(target_os = "linux")]
use dbus::arg::{Variant, RefArg};
#[cfg(target_os = "linux")]
use dbus::blocking::stdintf::org_freedesktop_dbus::Properties;

use serde::Serialize;
#[cfg(target_os = "linux")]
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct MediaState {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub is_playing: bool,
    pub volume: f64,
    pub position_us: u64,
    pub length_us: u64,
    pub player: String,
}

#[cfg(target_os = "linux")]
pub fn get_media_state() -> Option<MediaState> {
    let conn = Connection::new_session().ok()?;
    let proxy = conn.with_proxy("org.freedesktop.DBus", "/", Duration::from_millis(500));

    let (names,): (Vec<String>,) = proxy.method_call("org.freedesktop.DBus", "ListNames", ()).ok()?;
    
    let mpris_name = names.into_iter().find(|name| name.starts_with("org.mpris.MediaPlayer2."))?;
    let player_name = mpris_name.strip_prefix("org.mpris.MediaPlayer2.")?.to_string();

    let player_proxy = conn.with_proxy(&mpris_name, "/org/mpris/MediaPlayer2", Duration::from_millis(500));

    let playback_status: String = player_proxy.get("org.mpris.MediaPlayer2.Player", "PlaybackStatus").ok()?;
    let is_playing = playback_status == "Playing";

    let volume: f64 = player_proxy.get("org.mpris.MediaPlayer2.Player", "Volume").unwrap_or(1.0);
    
    let position: i64 = player_proxy.get("org.mpris.MediaPlayer2.Player", "Position").unwrap_or(0);

    let metadata: std::collections::HashMap<String, Variant<Box<dyn RefArg>>> = 
        player_proxy.get("org.mpris.MediaPlayer2.Player", "Metadata").ok()?;

    let mut title = String::new();
    let mut artist = String::new();
    let mut album = String::new();
    let mut length_us = 0u64;

    if let Some(v) = metadata.get("xesam:title") {
        if let Some(s) = v.0.as_str() {
            title = s.to_string();
        }
    }

    if let Some(v) = metadata.get("xesam:artist") {
        if let Some(arr) = v.0.as_iter() {
            let artists: Vec<String> = arr.filter_map(|i| i.as_str().map(|s| s.to_string())).collect();
            artist = artists.join(", ");
        } else if let Some(s) = v.0.as_str() {
            artist = s.to_string();
        }
    }

    if let Some(v) = metadata.get("xesam:album") {
        if let Some(s) = v.0.as_str() {
            album = s.to_string();
        }
    }

    if let Some(v) = metadata.get("mpris:length") {
        if let Some(l) = v.0.as_i64() {
            length_us = l.max(0) as u64;
        }
    }

    Some(MediaState {
        title,
        artist,
        album,
        is_playing,
        volume,
        position_us: position.max(0) as u64,
        length_us,
        player: player_name,
    })
}

#[cfg(not(target_os = "linux"))]
pub fn get_media_state() -> Option<MediaState> {
    None
}

#[cfg(target_os = "linux")]
pub fn send_media_command(command: &str, value: Option<f64>) -> Result<(), String> {
    let conn = Connection::new_session().map_err(|e| e.to_string())?;
    let proxy = conn.with_proxy("org.freedesktop.DBus", "/", Duration::from_millis(500));

    let (names,): (Vec<String>,) = proxy.method_call("org.freedesktop.DBus", "ListNames", ()).map_err(|e| e.to_string())?;
    let mpris_name = names.into_iter().find(|name| name.starts_with("org.mpris.MediaPlayer2."))
        .ok_or_else(|| "No media player found".to_string())?;

    let player_proxy = conn.with_proxy(&mpris_name, "/org/mpris/MediaPlayer2", Duration::from_millis(500));

    match command {
        "PlayPause" => {
            player_proxy.method_call::<(), _, _, _>("org.mpris.MediaPlayer2.Player", "PlayPause", ()).map_err(|e| e.to_string())?;
        }
        "Next" => {
            player_proxy.method_call::<(), _, _, _>("org.mpris.MediaPlayer2.Player", "Next", ()).map_err(|e| e.to_string())?;
        }
        "Prev" => {
            player_proxy.method_call::<(), _, _, _>("org.mpris.MediaPlayer2.Player", "Previous", ()).map_err(|e| e.to_string())?;
        }
        "VolumeUp" => {
            let mut volume: f64 = player_proxy.get("org.mpris.MediaPlayer2.Player", "Volume").unwrap_or(1.0);
            volume = (volume + 0.1).min(1.0);
            player_proxy.set("org.mpris.MediaPlayer2.Player", "Volume", volume).map_err(|e| e.to_string())?;
        }
        "VolumeDown" => {
            let mut volume: f64 = player_proxy.get("org.mpris.MediaPlayer2.Player", "Volume").unwrap_or(1.0);
            volume = (volume - 0.1).max(0.0);
            player_proxy.set("org.mpris.MediaPlayer2.Player", "Volume", volume).map_err(|e| e.to_string())?;
        }
        "SetVolume" => {
            if let Some(v) = value {
                player_proxy.set("org.mpris.MediaPlayer2.Player", "Volume", v).map_err(|e| e.to_string())?;
            }
        }
        "SetPosition" => {
            let metadata: std::collections::HashMap<String, Variant<Box<dyn RefArg>>> = 
                player_proxy.get("org.mpris.MediaPlayer2.Player", "Metadata").map_err(|e| e.to_string())?;
            let track_id_str = if let Some(v) = metadata.get("mpris:trackid") {
                v.0.as_str().ok_or_else(|| "trackid is not a string".to_string())?
            } else {
                return Err("No trackid found in metadata".to_string());
            };
            if let Some(pos_sec) = value {
                let position_us = (pos_sec * 1_000_000.0) as i64;
                let track_id_path = dbus::Path::new(track_id_str).map_err(|e| e.to_string())?;
                player_proxy.method_call::<(), _, _, _>(
                    "org.mpris.MediaPlayer2.Player",
                    "SetPosition",
                    (track_id_path, position_us),
                ).map_err(|e| e.to_string())?;
            }
        }
        "SetSystemVolume" => {
            if let Some(v) = value {
                set_system_volume(v)?;
            }
        }
        "SystemVolumeUp" => {
            adjust_system_volume(true)?;
        }
        "SystemVolumeDown" => {
            adjust_system_volume(false)?;
        }
        _ => return Err(format!("Unknown media command: {}", command)),
    }

    Ok(())
}

#[cfg(not(target_os = "linux"))]
pub fn send_media_command(_command: &str, _value: Option<f64>) -> Result<(), String> {
    Err("Media commands are not supported on this platform".to_string())
}

#[cfg(target_os = "linux")]
pub fn get_system_volume() -> Option<(f64, bool)> {
    use std::process::Command;
    let output = Command::new("amixer")
        .args(&["sget", "Master"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    let mut volume_pct = 1.0;
    let mut muted = false;
    let mut parsed_volume = false;
    
    for line in stdout.lines() {
        if line.contains("Playback") && line.contains("[") && line.contains("]") {
            if let Some(start) = line.find('[') {
                if let Some(end) = line[start..].find('%') {
                    let pct_str = &line[start + 1..start + end];
                    if let Ok(pct) = pct_str.parse::<f64>() {
                        volume_pct = pct / 100.0;
                        parsed_volume = true;
                    }
                }
            }
            if line.contains("[off]") {
                muted = true;
            }
        }
    }
    
    if parsed_volume {
        Some((volume_pct, muted))
    } else {
        None
    }
}

#[cfg(not(target_os = "linux"))]
pub fn get_system_volume() -> Option<(f64, bool)> {
    None
}

#[cfg(target_os = "linux")]
pub fn set_system_volume(volume: f64) -> Result<(), String> {
    use std::process::Command;
    let pct = (volume * 100.0).round() as i32;
    let status = Command::new("amixer")
        .args(&["sset", "Master", &format!("{}%", pct), "unmute"])
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("Failed to set system volume".to_string())
    }
}

#[cfg(target_os = "linux")]
pub fn adjust_system_volume(up: bool) -> Result<(), String> {
    use std::process::Command;
    let arg = if up { "5%+" } else { "5%-" };
    let status = Command::new("amixer")
        .args(&["sset", "Master", arg, "unmute"])
        .status()
        .map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err("Failed to adjust system volume".to_string())
    }
}

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

    let volume = get_system_volume().map(|(v, _)| v).unwrap_or(1.0);
    
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

#[cfg(target_os = "windows")]
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSessionManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus,
};

#[cfg(target_os = "windows")]
pub fn get_media_state() -> Option<MediaState> {
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_MULTITHREADED,
        );
    }

    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync().ok()?.get().ok()?;
    let session = manager.GetCurrentSession().ok()?;
    
    let playback_info = session.GetPlaybackInfo().ok()?;
    let playback_status = playback_info.PlaybackStatus().ok()?;
    let is_playing = playback_status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing;

    let properties = session.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
    let title = properties.Title().map(|s| s.to_string()).unwrap_or_default();
    let artist = properties.Artist().map(|s| s.to_string()).unwrap_or_default();
    let album = properties.AlbumTitle().map(|s| s.to_string()).unwrap_or_default();
    let player = session.SourceAppUserModelId().map(|s| s.to_string()).unwrap_or_default();

    let mut position_us = 0;
    let mut length_us = 0;
    if let Ok(timeline) = session.GetTimelineProperties() {
        if let Ok(pos) = timeline.Position() {
            position_us = (pos.Duration / 10).max(0) as u64;
        }
        if let Ok(end) = timeline.EndTime() {
            length_us = (end.Duration / 10).max(0) as u64;
        }
    }

    let volume = get_system_volume().map(|(v, _)| v).unwrap_or(1.0);

    Some(MediaState {
        title,
        artist,
        album,
        is_playing,
        volume,
        position_us,
        length_us,
        player,
    })
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
pub fn get_media_state() -> Option<MediaState> {
    None
}

#[cfg(target_os = "linux")]
pub fn send_media_command(command: &str, value: Option<f64>) -> Result<(), String> {
    match command {
        "SetSystemVolume" => {
            if let Some(v) = value {
                return set_system_volume(v);
            }
            return Ok(());
        }
        "SystemVolumeUp" => {
            return adjust_system_volume(true);
        }
        "SystemVolumeDown" => {
            return adjust_system_volume(false);
        }
        _ => {}
    }

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
            adjust_system_volume(true)?;
        }
        "VolumeDown" => {
            adjust_system_volume(false)?;
        }
        "SetVolume" => {
            if let Some(v) = value {
                set_system_volume(v)?;
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
        _ => return Err(format!("Unknown media command: {}", command)),
    }

    Ok(())
}

#[cfg(target_os = "windows")]
pub fn send_media_command(command: &str, value: Option<f64>) -> Result<(), String> {
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_MULTITHREADED,
        );
    }

    match command {
        "SetSystemVolume" => {
            if let Some(v) = value {
                return set_system_volume(v);
            }
            return Ok(());
        }
        "SystemVolumeUp" => {
            return adjust_system_volume(true);
        }
        "SystemVolumeDown" => {
            return adjust_system_volume(false);
        }
        _ => {}
    }

    let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
        .map_err(|e| e.to_string())?
        .get()
        .map_err(|e| e.to_string())?;
    let session = manager.GetCurrentSession().map_err(|e| e.to_string())?;

    match command {
        "PlayPause" => {
            let playback_info = session.GetPlaybackInfo().map_err(|e| e.to_string())?;
            let status = playback_info.PlaybackStatus().map_err(|e| e.to_string())?;
            if status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing {
                session.TryPauseAsync().map_err(|e| e.to_string())?.get().map_err(|e| e.to_string())?;
            } else {
                session.TryPlayAsync().map_err(|e| e.to_string())?.get().map_err(|e| e.to_string())?;
            }
        }
        "Next" => {
            session.TrySkipNextAsync().map_err(|e| e.to_string())?.get().map_err(|e| e.to_string())?;
        }
        "Prev" => {
            session.TrySkipPreviousAsync().map_err(|e| e.to_string())?.get().map_err(|e| e.to_string())?;
        }
        "VolumeUp" => {
            adjust_system_volume(true)?;
        }
        "VolumeDown" => {
            adjust_system_volume(false)?;
        }
        "SetVolume" => {
            if let Some(v) = value {
                set_system_volume(v)?;
            }
        }
        "SetPosition" => {
            if let Some(pos_sec) = value {
                let ticks = (pos_sec * 10_000_000.0) as i64;
                session.TryChangePlaybackPositionAsync(ticks)
                    .map_err(|e| e.to_string())?
                    .get()
                    .map_err(|e| e.to_string())?;
            }
        }
        _ => return Err(format!("Unknown media command: {}", command)),
    }

    Ok(())
}

#[cfg(target_os = "macos")]
pub fn send_media_command(command: &str, value: Option<f64>) -> Result<(), String> {
    match command {
        "SetSystemVolume" | "SetVolume" => {
            if let Some(v) = value {
                set_system_volume(v)
            } else {
                Ok(())
            }
        }
        "SystemVolumeUp" | "VolumeUp" => {
            adjust_system_volume(true)
        }
        "SystemVolumeDown" | "VolumeDown" => {
            adjust_system_volume(false)
        }
        _ => Err(format!("Media command '{}' is not supported on macOS", command)),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
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

#[cfg(target_os = "windows")]
pub fn get_system_volume() -> Option<(f64, bool)> {
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_MULTITHREADED,
        );
        
        let enumerator: windows::Win32::Media::Audio::IMMDeviceEnumerator = 
            windows::Win32::System::Com::CoCreateInstance(
                &windows::Win32::Media::Audio::MMDeviceEnumerator,
                None,
                windows::Win32::System::Com::CLSCTX_INPROC_SERVER,
            ).ok()?;
            
        let device = enumerator.GetDefaultAudioEndpoint(
            windows::Win32::Media::Audio::eRender,
            windows::Win32::Media::Audio::eMultimedia,
        ).ok()?;
        
        let endpoint_volume: windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume = 
            device.Activate(windows::Win32::System::Com::CLSCTX_INPROC_SERVER, None).ok()?;
            
        let volume = endpoint_volume.GetMasterVolumeLevelScalar().ok()? as f64;
        let muted = endpoint_volume.GetMute().ok()?.0 != 0;
        
        Some((volume, muted))
    }
}

#[cfg(target_os = "macos")]
pub fn get_system_volume() -> Option<(f64, bool)> {
    use std::process::Command;
    let output = Command::new("osascript")
        .args(&[
            "-e",
            "set ovol to output volume of (get volume settings)",
            "-e",
            "set omut to output muted of (get volume settings)",
            "-e",
            "ovol & \"|\" & omut",
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    let parts: Vec<&str> = trimmed.split('|').collect();
    if parts.len() == 2 {
        let vol_pct = parts[0].parse::<f64>().ok()? / 100.0;
        let muted = parts[1].trim().eq_ignore_ascii_case("true");
        Some((vol_pct, muted))
    } else {
        None
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
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

#[cfg(target_os = "windows")]
pub fn set_system_volume(volume: f64) -> Result<(), String> {
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_MULTITHREADED,
        );
        
        let enumerator: windows::Win32::Media::Audio::IMMDeviceEnumerator = 
            windows::Win32::System::Com::CoCreateInstance(
                &windows::Win32::Media::Audio::MMDeviceEnumerator,
                None,
                windows::Win32::System::Com::CLSCTX_INPROC_SERVER,
            ).map_err(|e| e.to_string())?;
            
        let device = enumerator.GetDefaultAudioEndpoint(
            windows::Win32::Media::Audio::eRender,
            windows::Win32::Media::Audio::eMultimedia,
        ).map_err(|e| e.to_string())?;
        
        let endpoint_volume: windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume = 
            device.Activate(windows::Win32::System::Com::CLSCTX_INPROC_SERVER, None).map_err(|e| e.to_string())?;
            
        endpoint_volume.SetMasterVolumeLevelScalar(volume as f32, std::ptr::null()).map_err(|e| e.to_string())?;
        endpoint_volume.SetMute(windows::Win32::Foundation::BOOL::from(false), std::ptr::null()).map_err(|e| e.to_string())?;
        
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub fn set_system_volume(volume: f64) -> Result<(), String> {
    use std::process::Command;
    let pct = (volume * 100.0).round() as i32;
    let status = Command::new("osascript")
        .args(&[
            "-e",
            &format!("set volume output volume {}", pct),
            "-e",
            "set volume without output muted",
        ])
        .status()
        .map_err(|e| e.to_string())?;

    if status.success() {
        Ok(())
    } else {
        Err("Failed to set system volume on macOS".to_string())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub fn set_system_volume(_volume: f64) -> Result<(), String> {
    Err("Volume control is not supported on this platform".to_string())
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

#[cfg(target_os = "windows")]
pub fn adjust_system_volume(up: bool) -> Result<(), String> {
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_MULTITHREADED,
        );
        
        let enumerator: windows::Win32::Media::Audio::IMMDeviceEnumerator = 
            windows::Win32::System::Com::CoCreateInstance(
                &windows::Win32::Media::Audio::MMDeviceEnumerator,
                None,
                windows::Win32::System::Com::CLSCTX_INPROC_SERVER,
            ).map_err(|e| e.to_string())?;
            
        let device = enumerator.GetDefaultAudioEndpoint(
            windows::Win32::Media::Audio::eRender,
            windows::Win32::Media::Audio::eMultimedia,
        ).map_err(|e| e.to_string())?;
        
        let endpoint_volume: windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume = 
            device.Activate(windows::Win32::System::Com::CLSCTX_INPROC_SERVER, None).map_err(|e| e.to_string())?;
            
        let current = endpoint_volume.GetMasterVolumeLevelScalar().map_err(|e| e.to_string())?;
        let delta = if up { 0.05f32 } else { -0.05f32 };
        let new_volume = (current + delta).clamp(0.0, 1.0);
        
        endpoint_volume.SetMasterVolumeLevelScalar(new_volume, std::ptr::null()).map_err(|e| e.to_string())?;
        endpoint_volume.SetMute(windows::Win32::Foundation::BOOL::from(false), std::ptr::null()).map_err(|e| e.to_string())?;
        
        Ok(())
    }
}

#[cfg(target_os = "macos")]
pub fn adjust_system_volume(up: bool) -> Result<(), String> {
    let (current, _) = get_system_volume().ok_or_else(|| "Failed to get current system volume".to_string())?;
    let delta = if up { 0.05 } else { -0.05 };
    let new_volume = (current + delta).clamp(0.0, 1.0);
    set_system_volume(new_volume)
}

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
pub fn adjust_system_volume(_up: bool) -> Result<(), String> {
    Err("Volume control is not supported on this platform".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_volume() {
        let res = get_system_volume();
        println!("Volume result: {:?}", res);
        assert!(res.is_some());
    }
}

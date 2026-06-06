//! Per-connection TCP handshake state machine.

use std::io::{BufReader, Read};
use std::net::TcpStream;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

use crate::app::{refresh_tray_menu, AppState, PEERS_CHANGED_EVENT, update_tray_progress, restore_default_tray_icon};
use crate::network::pairing::{PairDecision, PairRequestEvent, PairingManager};
use crate::network::protocol::{read_line_json, write_line_json, ClientMessage, ServerMessage};
use crate::system::{now_unix, TrustedPeer, TrustedPeers};

#[derive(Clone)]
pub struct SessionContext {
    pub app: AppHandle,
    pub device_id: String,
    pub name: String,
    pub trusted_peers: Arc<Mutex<TrustedPeers>>,
    pub pairing: Arc<PairingManager>,
    pub active_connections: Arc<Mutex<std::collections::HashSet<String>>>,
    pub active_streams: Arc<Mutex<std::collections::HashMap<String, TcpStream>>>,
    pub last_clipboard: Arc<Mutex<String>>,
}

struct ActiveConnectionGuard {
    device_id: String,
    active_connections: Arc<Mutex<std::collections::HashSet<String>>>,
    active_streams: Arc<Mutex<std::collections::HashMap<String, TcpStream>>>,
    app: AppHandle,
}

impl ActiveConnectionGuard {
    fn new(
        device_id: String,
        stream: &TcpStream,
        active_connections: Arc<Mutex<std::collections::HashSet<String>>>,
        active_streams: Arc<Mutex<std::collections::HashMap<String, TcpStream>>>,
        app: AppHandle,
    ) -> Self {
        {
            let mut active = active_connections
                .lock()
                .expect("active connections mutex poisoned");
            active.insert(device_id.clone());
        }
        if let Ok(stream) = stream.try_clone() {
            let mut streams = active_streams
                .lock()
                .expect("active streams mutex poisoned");
            streams.insert(device_id.clone(), stream);
        }
        app.emit("active-connections-changed", ()).ok();
        Self {
            device_id,
            active_connections,
            active_streams,
            app,
        }
    }
}

impl Drop for ActiveConnectionGuard {
    fn drop(&mut self) {
        {
            let mut active = self
                .active_connections
                .lock()
                .expect("active connections mutex poisoned");
            active.remove(&self.device_id);
        }
        {
            let mut streams = self
                .active_streams
                .lock()
                .expect("active streams mutex poisoned");
            streams.remove(&self.device_id);
        }
        self.app.emit("active-connections-changed", ()).ok();
    }
}

pub fn handle_client(stream: TcpStream, ctx: SessionContext) {
    if let Err(e) = run_session(stream, ctx) {
        eprintln!("[tcp] Session ended: {e}");
    }
}

use tauri::Listener;
use tauri_plugin_clipboard_manager::ClipboardExt;

fn run_session(stream: TcpStream, ctx: SessionContext) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(120)))
        .map_err(|e| format!("Could not set read timeout: {e}"))?;
    let mut writer = stream
        .try_clone()
        .map_err(|e| format!("Could not clone TCP stream: {e}"))?;
    let mut reader = BufReader::new(stream);

    let first: ClientMessage = read_line_json(&mut reader)?
        .ok_or_else(|| "Peer closed connection before handshake".to_string())?;

    let (peer_device_id, peer_name) = match first {
        ClientMessage::Hello {
            device_id,
            name,
            token,
        } => {
            if validate_token(&ctx, &device_id, token.as_deref())? {
                write_line_json(
                    &mut writer,
                    &ServerMessage::HelloOk {
                        device_id: ctx.device_id.clone(),
                        name: ctx.name.clone(),
                    },
                )?;
                println!("[tcp] Trusted peer connected: {name} ({device_id})");
                let _guard = ActiveConnectionGuard::new(
                    device_id.clone(),
                    &writer,
                    ctx.active_connections.clone(),
                    ctx.active_streams.clone(),
                    ctx.app.clone(),
                );
                return keep_session_open(&mut reader, &mut writer, &ctx, device_id);
            }

            write_line_json(&mut writer, &ServerMessage::PairRequired)?;
            (device_id, name)
        }
        ClientMessage::PairRequest {
            device_id,
            name,
            nonce,
        } => {
            return finish_pair_request(&mut reader, &mut writer, &ctx, device_id, name, nonce);
        }
        _ => {
            return Err("Expected Hello or PairRequest".to_string());
        }
    };

    let next: ClientMessage = read_line_json(&mut reader)?
        .ok_or_else(|| "Peer closed before pair request".to_string())?;
    match next {
        ClientMessage::PairRequest {
            device_id,
            name,
            nonce,
        } if device_id == peer_device_id && name == peer_name => {
            finish_pair_request(&mut reader, &mut writer, &ctx, device_id, name, nonce)
        }
        _ => Err("Expected PairRequest after PairRequired".to_string()),
    }
}

fn validate_token(
    ctx: &SessionContext,
    device_id: &str,
    token: Option<&str>,
) -> Result<bool, String> {
    let Some(token) = token else {
        return Ok(false);
    };
    let mut peers = ctx
        .trusted_peers
        .lock()
        .expect("trusted peers mutex poisoned");
    if peers
        .get(device_id)
        .map(|peer| peer.token == token)
        .unwrap_or(false)
    {
        peers.touch(device_id)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn finish_pair_request(
    reader: &mut BufReader<TcpStream>,
    writer: &mut TcpStream,
    ctx: &SessionContext,
    device_id: String,
    name: String,
    nonce: String,
) -> Result<(), String> {
    let code = verification_code(&device_id, &ctx.device_id, &nonce);
    let event = PairRequestEvent {
        device_id: device_id.clone(),
        name: name.clone(),
        code,
    };
    let (tx, rx) = mpsc::channel();
    ctx.pairing.insert(event.clone(), tx);
    ctx.app
        .emit("pair-request", &event)
        .map_err(|e| format!("Could not emit pair-request event: {e}"))?;

    let decision = match rx.recv_timeout(Duration::from_secs(120)) {
        Ok(decision) => decision,
        Err(_) => {
            ctx.pairing.remove(&device_id);
            return Err("Pair request timed out".to_string());
        }
    };

    match decision {
        PairDecision::Accept => {
            let token = generate_token();
            {
                let mut peers = ctx
                    .trusted_peers
                    .lock()
                    .expect("trusted peers mutex poisoned");
                peers.upsert(TrustedPeer {
                    device_id: device_id.clone(),
                    name: name.clone(),
                    token: token.clone(),
                    last_seen: now_unix(),
                    clipboard_sync_enabled: true,
                    media_controls_enabled: true,
                    volume_sync_enabled: true,
                    incoming_files_enabled: true,
                    terminal_access_enabled: false,
                })?;
            }
            let _ = ctx.app.emit(PEERS_CHANGED_EVENT, ());
            write_line_json(
                writer,
                &ServerMessage::PairAccepted {
                    device_id: ctx.device_id.clone(),
                    name: ctx.name.clone(),
                    token,
                },
            )?;
            let _guard = ActiveConnectionGuard::new(
                device_id.clone(),
                writer,
                ctx.active_connections.clone(),
                ctx.active_streams.clone(),
                ctx.app.clone(),
            );
            keep_session_open(reader, writer, ctx, device_id)
        }
        PairDecision::Reject => write_line_json(
            writer,
            &ServerMessage::PairRejected {
                reason: "Rejected on desktop".to_string(),
            },
        ),
    }
}

fn keep_session_open(
    mut reader: &mut BufReader<TcpStream>,
    writer: &mut TcpStream,
    ctx: &SessionContext,
    peer_device_id: String,
) -> Result<(), String> {
    let writer_clone = Arc::new(Mutex::new(writer.try_clone().map_err(|e| e.to_string())?));

    // Send initial TerminalServerInfo
    {
        let msg = get_terminal_info(&ctx.app, &peer_device_id);
        if let Ok(mut w) = writer_clone.lock() {
            let _ = write_line_json(&mut *w, &msg);
        }
    }

    let peer_id_for_term = peer_device_id.clone();
    let app_for_term = ctx.app.clone();
    let writer_for_term = writer_clone.clone();
    let term_event_id = ctx.app.listen("terminal-access-changed", move |event| {
        if let Ok(target_device_id) = serde_json::from_str::<String>(event.payload()) {
            if target_device_id == peer_id_for_term {
                let msg = get_terminal_info(&app_for_term, &peer_id_for_term);
                if let Ok(mut w) = writer_for_term.lock() {
                    let _ = write_line_json(&mut *w, &msg);
                }
            }
        }
    });

    let trusted_peers_clone = ctx.trusted_peers.clone();
    let peer_device_id_clone = peer_device_id.clone();
    let closure_writer_clone = writer_clone.clone();
    let event_id = ctx.app.listen("desktop-clipboard-update", move |event| {
        let is_sync_enabled = trusted_peers_clone
            .lock()
            .unwrap()
            .get(&peer_device_id_clone)
            .map(|p| p.clipboard_sync_enabled)
            .unwrap_or(false);
        if is_sync_enabled {
            let payload = event.payload();
            if let Ok(text) = serde_json::from_str::<String>(payload) {
                let msg = ServerMessage::ClipboardUpdate { text };
                if let Ok(mut w) = closure_writer_clone.lock() {
                    let _ = write_line_json(&mut *w, &msg);
                }
            }
        }
    });

    let media_writer_clone = writer_clone.clone();
    let media_running = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let media_running_clone = media_running.clone();
    let media_trusted_peers = ctx.trusted_peers.clone();
    let media_peer_id = peer_device_id.clone();
    std::thread::spawn(move || {
        #[derive(PartialEq)]
        struct MediaKey {
            title: String,
            artist: String,
            album: String,
            is_playing: bool,
            volume_pct: i32,
            player: String,
        }
        let mut last_key: Option<MediaKey> = None;
        let mut last_position_push = std::time::Instant::now();
        let mut last_system_volume: Option<(f64, bool)> = None;

        while media_running_clone.load(std::sync::atomic::Ordering::Relaxed) {
            let volume_enabled = media_trusted_peers
                .lock()
                .unwrap()
                .get(&media_peer_id)
                .map(|p| p.volume_sync_enabled)
                .unwrap_or(false);

            if volume_enabled {
                if let Some((vol, muted)) = crate::system::media::get_system_volume() {
                    let current = Some((vol, muted));
                    if last_system_volume != current {
                        let msg = ServerMessage::SystemVolumeUpdate { volume: vol, muted };
                        if let Ok(mut w) = media_writer_clone.lock() {
                            let _ = write_line_json(&mut *w, &msg);
                        }
                        last_system_volume = current;
                    }
                }
            } else {
                last_system_volume = None;
            }
            // Re-check the feature flag each tick so toggling takes effect immediately
            let media_enabled = media_trusted_peers
                .lock()
                .unwrap()
                .get(&media_peer_id)
                .map(|p| p.media_controls_enabled)
                .unwrap_or(false);

            if !media_enabled {
                // If we had previously sent state, send an empty clear so Android dismisses
                if last_key.is_some() {
                    last_key = None;
                    let empty = crate::system::media::MediaState {
                        title: String::new(),
                        artist: String::new(),
                        album: String::new(),
                        is_playing: false,
                        volume: 1.0,
                        position_us: 0,
                        length_us: 0,
                        player: String::new(),
                    };
                    if let Ok(mut w) = media_writer_clone.lock() {
                        let _ = write_line_json(&mut *w, &ServerMessage::MediaState(empty));
                    }
                }
                std::thread::sleep(Duration::from_millis(1000));
                continue;
            }

            if let Some(state) = crate::system::media::get_media_state() {
                let key = MediaKey {
                    title: state.title.clone(),
                    artist: state.artist.clone(),
                    album: state.album.clone(),
                    is_playing: state.is_playing,
                    // Round to 2 decimal places so minor float drift doesn't trigger sends
                    volume_pct: (state.volume * 100.0).round() as i32,
                    player: state.player.clone(),
                };
                // Send if core metadata changed, or if position is stale (> 3s) during playback
                let meta_changed = last_key.as_ref().map(|k| k != &key).unwrap_or(true);
                let position_due = state.is_playing
                    && last_position_push.elapsed() >= Duration::from_secs(3);

                if meta_changed || position_due {
                    let msg = ServerMessage::MediaState(state);
                    if let Ok(mut w) = media_writer_clone.lock() {
                        let _ = write_line_json(&mut *w, &msg);
                    }
                    last_key = Some(key);
                    last_position_push = std::time::Instant::now();
                }
            } else if last_key.is_some() {
                // Player disappeared — send an empty state to clear the notification
                last_key = None;
                let empty = crate::system::media::MediaState {
                    title: String::new(),
                    artist: String::new(),
                    album: String::new(),
                    is_playing: false,
                    volume: 1.0,
                    position_us: 0,
                    length_us: 0,
                    player: String::new(),
                };
                let msg = ServerMessage::MediaState(empty);
                if let Ok(mut w) = media_writer_clone.lock() {
                    let _ = write_line_json(&mut *w, &msg);
                }
            }
            std::thread::sleep(Duration::from_millis(1000));
        }
    });

    let mut line = String::new();
    let result = loop {
        line.clear();
        let read = match read_line_with_progress(
            &mut reader,
            &mut line,
            &peer_device_id,
            &ctx.app.state::<AppState>(),
            &ctx.app,
        ) {
            Ok(n) => n,
            Err(e) => {
                break Err(format!("Could not read from authenticated session: {e}"));
            }
        };
        if read == 0 {
            break Ok(());
        }

        match serde_json::from_str::<ClientMessage>(line.trim()) {
            Ok(ClientMessage::ClipboardUpdate { text }) => {
                let is_sync_enabled = ctx
                    .trusted_peers
                    .lock()
                    .unwrap()
                    .get(&peer_device_id)
                    .map(|p| p.clipboard_sync_enabled)
                    .unwrap_or(false);
                if is_sync_enabled {
                    println!(
                        "[tcp] Received clipboard update from peer: {} bytes",
                        text.len()
                    );
                    *ctx.last_clipboard.lock().unwrap() = text.clone();
                    let _ = ctx.app.clipboard().write_text(text.clone());
                } else {
                    println!("[tcp] Ignored clipboard update from peer (sync disabled)");
                }
            }
            Ok(ClientMessage::MediaCommand { command, value }) => {
                let media_enabled = ctx
                    .trusted_peers
                    .lock()
                    .unwrap()
                    .get(&peer_device_id)
                    .map(|p| p.media_controls_enabled)
                    .unwrap_or(false);
                let volume_enabled = ctx
                    .trusted_peers
                    .lock()
                    .unwrap()
                    .get(&peer_device_id)
                    .map(|p| p.volume_sync_enabled)
                    .unwrap_or(false);

                let is_volume_cmd = command == "SetSystemVolume" || command == "SystemVolumeUp" || command == "SystemVolumeDown";
                let allowed = if is_volume_cmd { volume_enabled } else { media_enabled };

                if allowed {
                    if let Err(e) = crate::system::media::send_media_command(&command, value) {
                        println!("[tcp] Media/volume command error: {e}");
                    }
                } else {
                    println!("[tcp] Ignored {} command from peer (disabled on desktop)", command);
                }
            }
            Ok(ClientMessage::FileTransferStart { filename, total_bytes }) => {
                println!("[tcp] File transfer started: {filename} ({total_bytes} bytes)");
                {
                    let app_state = ctx.app.state::<AppState>();
                    let mut transfer = app_state.active_transfer.lock().unwrap();
                    *transfer = Some(crate::app::TransferProgress {
                        device_id: peer_device_id.clone(),
                        filename: filename.clone(),
                        bytes_received: 0,
                        total_bytes,
                        speed_bytes_per_sec: 0.0,
                        status: "Receiving".to_string(),
                    });
                }
                let _ = refresh_tray_menu(&ctx.app);
                let _ = ctx.app.emit("file-transfer-started", &peer_device_id);
            }
            Ok(ClientMessage::IncomingFile { filename, base64_data, sha256 }) => {
                let incoming_files_enabled = ctx
                    .trusted_peers
                    .lock()
                    .unwrap()
                    .get(&peer_device_id)
                    .map(|p| p.incoming_files_enabled)
                    .unwrap_or(false);
                if incoming_files_enabled {
                    println!("[tcp] Received file from peer: {} ({} bytes)", filename, base64_data.len());
                    if let Err(e) = save_incoming_file(&ctx.app, &filename, &base64_data, &sha256) {
                        eprintln!("[tcp] Failed to save incoming file: {e}");
                    }
                } else {
                    println!("[tcp] Ignored incoming file from peer (incoming files disabled)");
                }
                {
                    let app_state = ctx.app.state::<AppState>();
                    let mut transfer = app_state.active_transfer.lock().unwrap();
                    *transfer = None;
                }
                let _ = refresh_tray_menu(&ctx.app);
                let _ = ctx.app.emit("file-transfer-finished", &peer_device_id);
                restore_default_tray_icon(&ctx.app);
            }
            Ok(_) => println!("[tcp] Authenticated session payload: {}", line.trim()),
            Err(e) => println!(
                "[tcp] Unknown message from peer: {}; raw={}",
                e,
                line.trim()
            ),
        }
    };

    media_running.store(false, std::sync::atomic::Ordering::Relaxed);
    ctx.app.unlisten(event_id);
    ctx.app.unlisten(term_event_id);
    {
        let app_state = ctx.app.state::<AppState>();
        let mut transfer = app_state.active_transfer.lock().unwrap();
        if let Some(ref progress) = *transfer {
            if progress.device_id == peer_device_id {
                *transfer = None;
                let _ = ctx.app.emit("file-transfer-finished", &peer_device_id);
                let _ = refresh_tray_menu(&ctx.app);
                restore_default_tray_icon(&ctx.app);
            }
        }
    }
    result
}

fn verification_code(phone_id: &str, desktop_id: &str, nonce: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(phone_id.as_bytes());
    hasher.update(desktop_id.as_bytes());
    hasher.update(nonce.as_bytes());
    let digest = hasher.finalize();
    let value = digest
        .iter()
        .take(8)
        .fold(0u64, |acc, byte| (acc << 8) | u64::from(*byte));
    format!("{:06}", value % 1_000_000)
}

fn generate_token() -> String {
    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(Uuid::new_v4().as_bytes());
    bytes.extend_from_slice(Uuid::new_v4().as_bytes());
    STANDARD.encode(bytes)
}

fn save_incoming_file(app: &AppHandle, filename: &str, base64_data: &str, expected_sha256: &str) -> Result<String, String> {
    let app_state = app.state::<AppState>();
    
    // Stage 1: Decoding
    {
        let mut transfer = app_state.active_transfer.lock().unwrap();
        if let Some(ref mut progress) = *transfer {
            progress.status = "Decoding".to_string();
            progress.speed_bytes_per_sec = 0.0;
            let _ = app.emit("file-transfer-progress", progress.clone());
        }
    }
    let _ = refresh_tray_menu(app);
    update_tray_progress(app, 100.0);

    let bytes = STANDARD
        .decode(base64_data)
        .map_err(|e| format!("Invalid base64: {e}"))?;

    // Stage 2: Verifying
    {
        let mut transfer = app_state.active_transfer.lock().unwrap();
        if let Some(ref mut progress) = *transfer {
            progress.status = "Verifying".to_string();
            let _ = app.emit("file-transfer-progress", progress.clone());
        }
    }
    let _ = refresh_tray_menu(app);

    // Compute SHA-256 hash of decoded bytes
    let computed_sha256 = {
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let result = hasher.finalize();
        result.iter().map(|b| format!("{:02x}", b)).collect::<String>()
    };

    if computed_sha256.to_lowercase() != expected_sha256.to_lowercase() {
        return Err(format!(
            "SHA-256 verification failed (hash mismatch): expected {}, got {}",
            expected_sha256, computed_sha256
        ));
    }

    // Stage 3: Saving
    {
        let mut transfer = app_state.active_transfer.lock().unwrap();
        if let Some(ref mut progress) = *transfer {
            progress.status = "Saving".to_string();
            let _ = app.emit("file-transfer-progress", progress.clone());
        }
    }
    let _ = refresh_tray_menu(app);

    let download_dir = app
        .path()
        .download_dir()
        .map_err(|e| format!("Could not resolve download dir: {e}"))?;

    let path = std::path::Path::new(filename);
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let mut file_path = download_dir.join(filename);
    let mut counter = 1;
    while file_path.exists() {
        let new_filename = if extension.is_empty() {
            format!("{stem} ({counter})")
        } else {
            format!("{stem} ({counter}).{extension}")
        };
        file_path = download_dir.join(new_filename);
        counter += 1;
    }

    let final_filename = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(filename)
        .to_string();

    std::fs::write(&file_path, bytes)
        .map_err(|e| format!("Failed to write file to disk: {e}"))?;

    println!("[tcp] Successfully saved incoming file to {:?}", file_path);

    // Show native system notification
    let _ = notify_rust::Notification::new()
        .summary("File Received")
        .body(&format!("Saved to Downloads: {final_filename}"))
        .show();

    Ok(final_filename)
}

fn read_line_with_progress(
    reader: &mut BufReader<TcpStream>,
    line: &mut String,
    peer_device_id: &str,
    state: &AppState,
    app: &AppHandle,
) -> Result<usize, String> {
    line.clear();
    let start_time = std::time::Instant::now();
    let mut bytes_read = 0;

    loop {
        let mut byte = [0u8; 1];
        match reader.read_exact(&mut byte) {
            Ok(_) => {
                bytes_read += 1;
                let c = byte[0];
                if c == b'\n' {
                    break;
                }
                line.push(c as char);

                if bytes_read % 65536 == 0 {
                    update_progress(peer_device_id, bytes_read, start_time, state, app);
                }
            }
            Err(e) => {
                if bytes_read == 0 {
                    return Err(format!("Socket closed: {e}"));
                } else {
                    break;
                }
            }
        }
    }

    update_progress(peer_device_id, bytes_read, start_time, state, app);
    Ok(bytes_read)
}

fn update_progress(
    peer_device_id: &str,
    bytes_received: usize,
    start_time: std::time::Instant,
    state: &AppState,
    app: &AppHandle,
) {
    let mut transfer = state.active_transfer.lock().unwrap();
    if let Some(ref mut progress) = *transfer {
        if progress.device_id == peer_device_id {
            progress.bytes_received = bytes_received as u64;
            let elapsed = start_time.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                progress.speed_bytes_per_sec = (bytes_received as f64) / elapsed;
            }
            progress.status = "Receiving".to_string();
            let _ = app.emit("file-transfer-progress", progress.clone());
            
            let progress_pct = if progress.total_bytes > 0 {
                (progress.bytes_received as f64 / progress.total_bytes as f64 * 100.0).min(100.0)
            } else {
                0.0
            };
            update_tray_progress(app, progress_pct);
        }
    }
}

fn get_terminal_info(app: &tauri::AppHandle, peer_device_id: &str) -> ServerMessage {
    let state = app.state::<AppState>();
    let (enabled, port, password) = {
        let peers = state.trusted_peers.lock().unwrap();
        let enabled = peers.get(peer_device_id)
            .map(|p| p.terminal_access_enabled)
            .unwrap_or(false);
        
        let terminal_server = state.terminal_server.lock().unwrap();
        if let Some(ref server) = *terminal_server {
            (enabled, server.port, Some(server.password.clone()))
        } else {
            (false, 0, None)
        }
    };

    ServerMessage::TerminalServerInfo {
        enabled,
        port,
        username: peer_device_id.to_string(),
        password,
    }
}

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tauri::{Emitter, Manager, State};
use crate::app::AppState;

#[tauri::command]
pub async fn toggle_camera_stream(
    state: State<'_, AppState>,
    device_id: String,
    start: bool,
) -> Result<(), String> {
    let mut active_streams = state.active_streams.lock().unwrap();
    if let Some(stream) = active_streams.get_mut(&device_id) {
        let message = if start {
            crate::network::protocol::ServerMessage::StartCameraStream
        } else {
            crate::network::protocol::ServerMessage::StopCameraStream
        };
        crate::network::protocol::write_line_json(stream, &message)?;
        Ok(())
    } else {
        Err("Device is not connected".into())
    }
}

fn create_command(program: &str) -> std::process::Command {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = std::process::Command::new(program);
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::process::Command::new(program)
    }
}

fn find_adb_path() -> String {
    let adb_name = if cfg!(target_os = "windows") { "adb.exe" } else { "adb" };

    // 1. Try raw "adb" in PATH
    if create_command(adb_name).arg("--version").output().is_ok() {
        return adb_name.to_string();
    }

    // 2. Try standard Android SDK paths
    #[cfg(not(target_os = "windows"))]
    if let Ok(home) = std::env::var("HOME") {
        let sdk_path = format!("{}/Android/Sdk/platform-tools/adb", home);
        if std::path::Path::new(&sdk_path).exists() {
            return sdk_path;
        }
        // Check other common Unix/macOS locations
        for path in &[
            "/usr/bin/adb", 
            "/usr/local/bin/adb", 
            "/opt/android-sdk/platform-tools/adb",
            "/Library/Android/sdk/platform-tools/adb" // macOS default
        ] {
            if std::path::Path::new(path).exists() {
                return path.to_string();
            }
        }
    }

    #[cfg(target_os = "windows")]
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        let sdk_path = format!("{}\\Android\\Sdk\\platform-tools\\adb.exe", local_app_data);
        if std::path::Path::new(&sdk_path).exists() {
            return sdk_path;
        }
    }

    adb_name.to_string()
}

#[tauri::command]
pub async fn start_virtual_camera(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    ip: String,
    port: u16,
    use_adb: bool,
) -> Result<(), String> {
    let mut running_guard = state.virtual_camera_running.lock().unwrap();
    if running_guard.is_some() {
        return Ok(()); // Already running
    }

    if use_adb {
        let adb_cmd = find_adb_path();
        println!("[camera] Setting up ADB port forwarding: {} forward tcp:40000 tcp:{}", adb_cmd, port);
        let output = create_command(&adb_cmd)
            .args(&["forward", "tcp:40000", &format!("tcp:{}", port)])
            .output();
        match output {
            Ok(out) if out.status.success() => {
                println!("[camera] ADB port forwarding set up successfully");
            }
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr).to_string();
                eprintln!("[camera] ADB port forwarding failed: {}", err);
                return Err(format!("ADB forward failed: {}", err));
            }
            Err(e) => {
                eprintln!("[camera] Failed to execute adb command (path: {}): {}", adb_cmd, e);
                return Err(format!("Failed to execute adb (path: {}): {e}. Please ensure ADB is installed.", adb_cmd));
            }
        }
    }

    let (cancel_tx, mut cancel_rx) = tokio::sync::oneshot::channel::<()>();
    *running_guard = Some(cancel_tx);

    let virtual_camera_running = state.virtual_camera_running.clone();
    let app_handle = app.clone();
    let latest_frame = state.latest_camera_frame.clone();
    tokio::spawn(async move {
        println!("[camera] Starting virtual camera background thread...");

        #[cfg(target_os = "linux")]
        {
            let _ = app_handle.emit(
                "virtual-camera-state-changed",
                serde_json::json!({
                    "active": false,
                    "error": "Preparing driver (check for password prompt)...",
                }),
            );
            if let Err(e) = prepare_linux_driver() {
                eprintln!("[camera] Driver preparation failed: {e}");
                let _ = app_handle.emit(
                    "virtual-camera-state-changed",
                    serde_json::json!({
                        "active": false,
                        "error": format!("Driver initialization failed: {e}"),
                    }),
                );
                
                let state = app_handle.state::<AppState>();
                let mut active_streams = state.active_streams.lock().unwrap();
                for stream in active_streams.values_mut() {
                    let message = crate::network::protocol::ServerMessage::StopCameraStream;
                    let _ = crate::network::protocol::write_line_json(stream, &message);
                }
                let mut guard = state.virtual_camera_running.lock().unwrap();
                *guard = None;
                return;
            }
        }

        #[cfg(target_os = "windows")]
        {
            let _ = app_handle.emit(
                "virtual-camera-state-changed",
                serde_json::json!({
                    "active": false,
                    "error": "Preparing driver (check for administrator prompt)...",
                }),
            );
            if let Err(e) = prepare_windows_driver(&app_handle) {
                eprintln!("[camera] Driver preparation failed: {e}");
                let _ = app_handle.emit(
                    "virtual-camera-state-changed",
                    serde_json::json!({
                        "active": false,
                        "error": format!("Driver initialization failed: {e}"),
                    }),
                );
                
                let state = app_handle.state::<AppState>();
                let mut active_streams = state.active_streams.lock().unwrap();
                for stream in active_streams.values_mut() {
                    let message = crate::network::protocol::ServerMessage::StopCameraStream;
                    let _ = crate::network::protocol::write_line_json(stream, &message);
                }
                let mut guard = state.virtual_camera_running.lock().unwrap();
                *guard = None;
                return;
            }
        }

        let target_ip = if use_adb { "127.0.0.1".to_string() } else { ip };
        let target_port = if use_adb { 40000 } else { port };

        let mut retry_count = 0;
        let max_retries = 5;

        loop {
            // Check if cancelled before starting loop iteration
            let is_cancelled = match cancel_rx.try_recv() {
                Ok(_) | Err(tokio::sync::oneshot::error::TryRecvError::Closed) => true,
                Err(tokio::sync::oneshot::error::TryRecvError::Empty) => false,
            };
            if is_cancelled {
                println!("[camera] Virtual camera stream cancelled.");
                break;
            }

            println!("[camera] Connecting to camera stream (attempt {}/{})...", retry_count + 1, max_retries);
            
            match run_virtual_camera_loop(app_handle.clone(), target_ip.clone(), target_port, latest_frame.clone(), &mut cancel_rx).await {
                Ok(_) => {
                    let _ = app_handle.emit(
                        "virtual-camera-state-changed",
                        serde_json::json!({
                            "active": false,
                            "error": null,
                        }),
                    );
                    break;
                }
                Err(e) => {
                    eprintln!("[camera] Virtual camera loop error: {e}");
                    
                    // Check if cancelled during the loop
                    let is_cancelled_during = match cancel_rx.try_recv() {
                        Ok(_) | Err(tokio::sync::oneshot::error::TryRecvError::Closed) => true,
                        Err(tokio::sync::oneshot::error::TryRecvError::Empty) => false,
                    };
                    if is_cancelled_during {
                        println!("[camera] Virtual camera stream cancelled after error.");
                        break;
                    }

                    retry_count += 1;
                    if retry_count >= max_retries {
                        let _ = app_handle.emit(
                            "virtual-camera-state-changed",
                            serde_json::json!({
                                "active": false,
                                "error": format!("Failed after {} attempts: {}", max_retries, e),
                            }),
                        );
                        break;
                    }

                    let _ = app_handle.emit(
                        "virtual-camera-state-changed",
                        serde_json::json!({
                            "active": false,
                            "error": format!("Connection lost. Retrying in 2s ({}/{})...", retry_count, max_retries),
                        }),
                    );

                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
        println!("[camera] Virtual camera background thread stopped.");
        
        let mut guard = virtual_camera_running.lock().unwrap();
        *guard = None;
        drop(guard);

        if use_adb {
            let adb_cmd = find_adb_path();
            println!("[camera] Removing ADB port forwarding: {} forward --remove tcp:40000", adb_cmd);
            let _ = create_command(&adb_cmd)
                .args(&["forward", "--remove", "tcp:40000"])
                .output();
        }


        // Notify frontend that camera streaming has stopped
        let _ = app_handle.emit(
            "camera-stream-state-changed",
            serde_json::json!({
                "streaming": false,
            }),
        );

        // Send stop message to active streams to ensure Android stops streaming
        let state = app_handle.state::<AppState>();
        let mut active_streams = state.active_streams.lock().unwrap();
        for stream in active_streams.values_mut() {
            let message = crate::network::protocol::ServerMessage::StopCameraStream;
            let _ = crate::network::protocol::write_line_json(stream, &message);
        }
    });

    Ok(())
}


#[tauri::command]
pub async fn stop_virtual_camera(state: State<'_, AppState>) -> Result<(), String> {
    let mut running_guard = state.virtual_camera_running.lock().unwrap();
    if let Some(cancel_tx) = running_guard.take() {
        let _ = cancel_tx.send(());
    }
    Ok(())
}

async fn run_virtual_camera_loop(
    app: tauri::AppHandle,
    ip: String,
    port: u16,
    latest_frame: std::sync::Arc<std::sync::Mutex<Option<Vec<u8>>>>,
    cancel_rx: &mut tokio::sync::oneshot::Receiver<()>,
) -> Result<(), String> {
    let url = format!("{}:{}", ip, port);
    
    let stream_future = tokio::net::TcpStream::connect(&url);
    let mut stream = tokio::time::timeout(std::time::Duration::from_secs(5), stream_future)
        .await
        .map_err(|_| format!("Connection timeout to MJPEG server at {url}"))?
        .map_err(|e| format!("Failed to connect to MJPEG server at {url}: {e}"))?;

    let request = format!(
        "GET / HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        url
    );
    let write_future = stream.write_all(request.as_bytes());
    tokio::time::timeout(std::time::Duration::from_secs(5), write_future)
        .await
        .map_err(|_| "Timeout sending HTTP GET request".to_string())?
        .map_err(|e| format!("Failed to send HTTP GET: {e}"))?;

    let mut reader = tokio::io::BufReader::with_capacity(65536, stream);
    let mut buffer = Vec::new();
    let mut chunk = vec![0u8; 65536];

    let mut cam_initialized = false;
    
    #[cfg(target_os = "linux")]
    let mut linux_cam: Option<linux_cam::LinuxVirtualCamera> = None;
    
    #[cfg(target_os = "windows")]
    let mut win_cam: Option<win_cam::WinVirtualCamera> = None;

    loop {
        tokio::select! {
            _ = &mut *cancel_rx => {
                break;
            }
            read_res = tokio::time::timeout(std::time::Duration::from_secs(15), reader.read(&mut chunk)) => {
                let bytes_read = match read_res {
                    Ok(Ok(n)) => n,
                    Ok(Err(e)) => return Err(format!("Read error: {e}")),
                    Err(_) => return Err("Read timeout: no camera data received for 15 seconds".into()),
                };
                if bytes_read == 0 {
                    break; // stream closed
                }
                buffer.extend_from_slice(&chunk[..bytes_read]);

                while let Some(start_idx) = find_subsequence(&buffer, &[0xFF, 0xD8]) {
                    if start_idx > 0 {
                        buffer.drain(0..start_idx);
                        continue;
                    }

                    if let Some(end_idx) = find_subsequence(&buffer, &[0xFF, 0xD9]) {
                        let actual_end_idx = end_idx + 2;
                        let jpeg_bytes = &buffer[0..actual_end_idx];

                        {
                            let mut guard = latest_frame.lock().unwrap();
                            *guard = Some(jpeg_bytes.to_vec());
                        }

                        let mut needs_decode = false;
                        #[cfg(target_os = "linux")]
                        if linux_cam.is_some() || !cam_initialized {
                            needs_decode = true;
                        }
                        #[cfg(target_os = "windows")]
                        if win_cam.is_some() || !cam_initialized {
                            needs_decode = true;
                        }

                        if needs_decode {
                            if let Ok((rgb, w, h)) = decode_jpeg(jpeg_bytes) {
                                if !cam_initialized {
                                    cam_initialized = true;
                                    #[cfg(target_os = "linux")]
                                    {
                                        match linux_cam::LinuxVirtualCamera::create(w, h) {
                                            Ok(c) => {
                                                linux_cam = Some(c);
                                                let _ = app.emit(
                                                    "virtual-camera-state-changed",
                                                    serde_json::json!({
                                                        "active": true,
                                                        "error": null,
                                                    }),
                                                );
                                            }
                                            Err(e) => {
                                                eprintln!("[camera] Failed to create virtual camera (preview will still work): {e}");
                                                let _ = app.emit(
                                                    "virtual-camera-state-changed",
                                                    serde_json::json!({
                                                        "active": false,
                                                        "error": format!("Failed to create virtual camera: {e}"),
                                                    }),
                                                );
                                            }
                                        }
                                    }
                                    #[cfg(target_os = "windows")]
                                    {
                                        match win_cam::WinVirtualCamera::create(w, h) {
                                            Ok(c) => {
                                                win_cam = Some(c);
                                                let _ = app.emit(
                                                    "virtual-camera-state-changed",
                                                    serde_json::json!({
                                                        "active": true,
                                                        "error": null,
                                                    }),
                                                );
                                            }
                                            Err(e) => {
                                                eprintln!("[camera] Failed to create virtual camera (preview will still work): {e}");
                                                let _ = app.emit(
                                                    "virtual-camera-state-changed",
                                                    serde_json::json!({
                                                        "active": false,
                                                        "error": format!("Failed to create virtual camera: {e}"),
                                                    }),
                                                );
                                            }
                                        }
                                    }
                                }

                                #[cfg(target_os = "linux")]
                                {
                                    if let Some(ref mut c) = linux_cam {
                                        let mut yuyv = vec![0u8; (w * h * 2) as usize];
                                        rgb_to_yuyv(&rgb, &mut yuyv);
                                        let _ = c.send(&yuyv);
                                    }
                                }

                                #[cfg(target_os = "windows")]
                                {
                                    if let Some(ref mut c) = win_cam {
                                        let _ = c.send(&rgb);
                                    }
                                }
                            }
                        } else {
                            if !cam_initialized {
                                cam_initialized = true;
                            }
                        }

                        buffer.drain(0..actual_end_idx);
                        tokio::task::yield_now().await;
                    } else {
                        break;
                    }
                }

                if find_subsequence(&buffer, &[0xFF, 0xD8]).is_none() && buffer.len() > 4096 {
                    buffer.drain(0..buffer.len() - 1);
                }
            }
        }
    }

    {
        let mut guard = latest_frame.lock().unwrap();
        *guard = None;
    }
    Ok(())
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

fn decode_jpeg(jpeg_bytes: &[u8]) -> Result<(Vec<u8>, u32, u32), String> {
    let mut decoder = jpeg_decoder::Decoder::new(std::io::Cursor::new(jpeg_bytes));
    let pixels = decoder.decode().map_err(|e| e.to_string())?;
    let info = decoder.info().ok_or_else(|| "Failed to get JPEG info".to_string())?;
    Ok((pixels, info.width as u32, info.height as u32))
}

#[cfg(target_os = "linux")]
fn rgb_to_yuyv(rgb: &[u8], yuyv: &mut [u8]) {
    let num_pixels = rgb.len() / 3;
    for i in (0..num_pixels).step_by(2) {
        if i + 1 >= num_pixels {
            break;
        }
        let r0 = rgb[i * 3] as f32;
        let g0 = rgb[i * 3 + 1] as f32;
        let b0 = rgb[i * 3 + 2] as f32;

        let r1 = rgb[(i + 1) * 3] as f32;
        let g1 = rgb[(i + 1) * 3 + 1] as f32;
        let b1 = rgb[(i + 1) * 3 + 2] as f32;

        let y0 = 0.299 * r0 + 0.587 * g0 + 0.114 * b0;
        let y1 = 0.299 * r1 + 0.587 * g1 + 0.114 * b1;

        let r_avg = (r0 + r1) / 2.0;
        let g_avg = (g0 + g1) / 2.0;
        let b_avg = (b0 + b1) / 2.0;

        let u = -0.169 * r_avg - 0.331 * g_avg + 0.500 * b_avg + 128.0;
        let v = 0.500 * r_avg - 0.419 * g_avg - 0.081 * b_avg + 128.0;

        let dest_idx = i * 2;
        if dest_idx + 3 < yuyv.len() {
            yuyv[dest_idx] = y0.clamp(0.0, 255.0) as u8;
            yuyv[dest_idx + 1] = u.clamp(0.0, 255.0) as u8;
            yuyv[dest_idx + 2] = y1.clamp(0.0, 255.0) as u8;
            yuyv[dest_idx + 3] = v.clamp(0.0, 255.0) as u8;
        }
    }
}

#[cfg(target_os = "linux")]
fn prepare_linux_driver() -> Result<(), String> {
    if !std::path::Path::new("/dev/video9").exists() {
        println!("[camera] /dev/video9 does not exist. Loading v4l2loopback module...");
        let status = create_command("pkexec")
            .args(&["modprobe", "v4l2loopback", "exclusive_caps=1", "card_label=Sync Camera", "video_nr=9"])
            .status()
            .map_err(|e| format!("Failed to run pkexec modprobe: {e}"))?;
        if !status.success() {
            return Err("Failed to load v4l2loopback module".into());
        }
    }

    // Check permissions and fix if needed before running v4l2-ctl
    let test_open = std::fs::OpenOptions::new().write(true).open("/dev/video9");
    if let Err(ref e) = test_open {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            println!("[camera] Permission denied for /dev/video9. Attempting to fix permissions using pkexec chmod...");
            let chmod_status = create_command("pkexec")
                .args(&["chmod", "0666", "/dev/video9"])
                .status()
                .map_err(|e| format!("Failed to run pkexec chmod: {e}"))?;
            if !chmod_status.success() {
                return Err("Failed to set write permissions on /dev/video9".into());
            }
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn is_windows_driver_registered() -> bool {
    let status = create_command("reg")
        .args(&["query", "HKCR\\CLSID\\{A3FCE0F5-3493-419F-958A-ABA1250EC20B}"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();

    match status {
        Ok(s) => s.success(),
        Err(_) => false,
    }
}

#[cfg(target_os = "windows")]
fn is_windows_driver_renamed() -> bool {
    let output = create_command("reg")
        .args(&[
            "query",
            "HKLM\\SOFTWARE\\Classes\\CLSID\\{860BB310-5D01-11d0-BD3B-00A0C911CE86}\\Instance\\{A3FCE0F5-3493-419F-958A-ABA1250EC20B}",
            "/v",
            "FriendlyName",
        ])
        .output();

    if let Ok(out) = output {
        if out.status.success() {
            let stdout = String::from_utf8_lossy(&out.stdout);
            return stdout.contains("Sync Camera");
        }
    }
    false
}

#[cfg(target_os = "windows")]
fn prepare_windows_driver(app: &tauri::AppHandle) -> Result<(), String> {
    let registered = is_windows_driver_registered();
    let renamed = is_windows_driver_renamed();

    if registered && renamed {
        println!("[camera] Sync Camera driver is registered and properly named.");
        return Ok(());
    }

    use tauri::path::BaseDirectory;
    let dll_path = app
        .path()
        .resolve("resources/obs-virtualcam-module64.dll", BaseDirectory::Resource)
        .map_err(|e| format!("Failed to resolve DLL path: {e}"))?;

    if !dll_path.exists() {
        return Err(format!(
            "Bundled virtual camera module not found at resource path: {:?}",
            dll_path
        ));
    }

    if !registered {
        println!("[camera] Virtual camera driver not registered. Registering and naming...");
    } else {
        println!("[camera] Virtual camera driver registered but not named. Naming...");
    }

    let register_cmd = if !registered {
        format!("regsvr32 /s \"{}\"; ", dll_path.to_string_lossy())
    } else {
        "".to_string()
    };

    let script = format!(
        "{}reg add \"HKLM\\SOFTWARE\\Classes\\CLSID\\{{A3FCE0F5-3493-419F-958A-ABA1250EC20B}}\" /ve /d \"Sync Camera\" /f; \
         reg add \"HKLM\\SOFTWARE\\Classes\\CLSID\\{{860BB310-5D01-11d0-BD3B-00A0C911CE86}}\\Instance\\{{A3FCE0F5-3493-419F-958A-ABA1250EC20B}}\" /v \"FriendlyName\" /d \"Sync Camera\" /f; \
         reg add \"HKLM\\SOFTWARE\\Classes\\CLSID\\{{860BB310-5D01-11d0-BD3B-00A0C911CE86}}\\Instance\\{{A3FCE0F5-3493-419F-958A-ABA1250EC20B}}\" /ve /d \"Sync Camera\" /f; \
         reg add \"HKLM\\SOFTWARE\\Classes\\WOW6432Node\\CLSID\\{{A3FCE0F5-3493-419F-958A-ABA1250EC20B}}\" /ve /d \"Sync Camera\" /f; \
         reg add \"HKLM\\SOFTWARE\\Classes\\WOW6432Node\\CLSID\\{{860BB310-5D01-11d0-BD3B-00A0C911CE86}}\\Instance\\{{A3FCE0F5-3493-419F-958A-ABA1250EC20B}}\" /v \"FriendlyName\" /d \"Sync Camera\" /f; \
         reg add \"HKLM\\SOFTWARE\\Classes\\WOW6432Node\\CLSID\\{{860BB310-5D01-11d0-BD3B-00A0C911CE86}}\\Instance\\{{A3FCE0F5-3493-419F-958A-ABA1250EC20B}}\" /ve /d \"Sync Camera\" /f",
        register_cmd
    );

    println!("[camera] Launching elevated script: {}", script);

    let status = create_command("powershell")
        .args(&[
            "-Command",
            &format!(
                "Start-Process powershell -ArgumentList '-NoProfile -Command \"{}\"' -Verb RunAs -Wait",
                script
            ),
        ])
        .status()
        .map_err(|e| format!("Failed to launch elevated registration: {e}"))?;

    if !status.success() {
        return Err("Registration/renaming failed or was cancelled by the user.".into());
    }

    if !is_windows_driver_registered() {
        return Err("DLL registration command completed, but class registry check still failed.".into());
    }

    if !is_windows_driver_renamed() {
        return Err("DLL registered successfully, but naming registry check failed.".into());
    }

    println!("[camera] Virtual camera driver registered and named 'Sync Camera' successfully!");
    Ok(())
}

#[cfg(target_os = "linux")]
mod linux_cam {
    use std::fs::File;
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    use super::create_command;

    pub struct LinuxVirtualCamera(File);

    impl LinuxVirtualCamera {
        pub fn create(w: u32, h: u32) -> Result<Self, String> {
            // Set the video format on the device BEFORE opening the file writer.
            // Setting format fails with EBUSY if a writer is already holding the device descriptor.
            let format_status = create_command("v4l2-ctl")
                .args(&[
                    "-d", "/dev/video9",
                    "--set-fmt-video-out",
                    &format!("width={},height={},pixelformat=YUYV", w, h)
                ])
                .status()
                .map_err(|e| format!("Failed to run v4l2-ctl: {e}"))?;

            if !format_status.success() {
                eprintln!("[camera] Warning: v4l2-ctl returned non-zero status");
            }

            // Open the file descriptor to write MJPEG/YUYV frames.
            let file = std::fs::OpenOptions::new()
                .write(true)
                .custom_flags(0x800) // O_NONBLOCK (2048) on Linux
                .open("/dev/video9")
                .map_err(|e| format!("Failed to open /dev/video9 for writing: {e}"))?;

            Ok(LinuxVirtualCamera(file))
        }

        pub fn send(&mut self, yuyv: &[u8]) -> Result<(), String> {
            match self.0.write_all(yuyv) {
                Ok(_) => {
                    let _ = self.0.flush();
                    Ok(())
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // Consumer is slow, drop the frame to prevent freezing
                    Ok(())
                }
                Err(e) => Err(e.to_string()),
            }
        }
    }
}

#[cfg(target_os = "windows")]
mod win_cam {
    use virtualcam::{Camera, PixelFormat};

    pub struct WinVirtualCamera(Camera);

    impl WinVirtualCamera {
        pub fn create(w: u32, h: u32) -> Result<Self, String> {
            let cam = Camera::builder(w, h, 30.0)
                .format(PixelFormat::RGB)
                .build()
                .map_err(|e| format!("Failed to create Windows virtual camera: {e}"))?;
            Ok(WinVirtualCamera(cam))
        }

        pub fn send(&mut self, rgb: &[u8]) -> Result<(), String> {
            self.0.send(rgb).map_err(|e| e.to_string())
        }
    }
}

#[tauri::command]
pub async fn get_latest_frame(
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let guard = state.latest_camera_frame.lock().unwrap();
    if let Some(ref bytes) = *guard {
        use base64::{engine::general_purpose::STANDARD, Engine as _};
        Ok(Some(STANDARD.encode(bytes)))
    } else {
        Ok(None)
    }
}


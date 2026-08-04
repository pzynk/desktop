use std::io::Write;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{AppHandle, Emitter};

pub struct VirtualMicrophone {
    running: Arc<AtomicBool>,
    muted: Arc<AtomicBool>,
    volume: Arc<Mutex<f64>>,
    null_sink_id: Option<String>,
    remap_source_id: Option<String>,
    pacat_child: Arc<Mutex<Option<Child>>>,
}

fn run_command(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("Failed to execute {program}: {e}"))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

impl VirtualMicrophone {
    pub fn create(
        app: AppHandle,
        ip: String,
        port: u16,
        sample_rate: u32,
        channels: u16,
        use_adb: bool,
    ) -> Result<Self, String> {
        let (null_sink_id, remap_source_id) = Self::setup_pulse_modules()?;

        let connect_addr = if use_adb {
            format!("127.0.0.1:{port}")
        } else {
            format!("{ip}:{port}")
        };

        let mut child = Command::new("pacat")
            .arg("--playback")
            .arg("--device=SyncMicSink")
            .arg(format!("--rate={sample_rate}"))
            .arg(format!("--channels={channels}"))
            .arg("--format=s16le")
            .arg("--latency-msec=20")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed to start pacat: {e}. Is pulseaudio-utils installed?"))?;

        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Failed to capture pacat stdin".to_string())?;

        let pacat_child = Arc::new(Mutex::new(Some(child)));
        let running = Arc::new(AtomicBool::new(true));
        let muted = Arc::new(AtomicBool::new(false));
        let volume = Arc::new(Mutex::new(1.0f64));

        let r_clone = running.clone();
        let m_clone = muted.clone();
        let v_clone = volume.clone();

        thread::spawn(move || {
            use std::io::Read;
            use std::net::TcpStream;
            use std::time::Duration;

            let stream_res = TcpStream::connect(&connect_addr);
            let mut stream = match stream_res {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[virtual_mic] Failed to connect to audio server at {connect_addr}: {e}");
                    app.emit(
                        "virtual-mic-state-changed",
                        serde_json::json!({ "active": false, "error": format!("Connection failed: {e}") }),
                    )
                    .ok();
                    return;
                }
            };

            stream.set_read_timeout(Some(Duration::from_millis(500))).ok();
            stream.set_nodelay(true).ok();

            app.emit(
                "virtual-mic-state-changed",
                serde_json::json!({ "active": true, "error": null }),
            )
            .ok();

            let mut buf = [0u8; 1024];
            let mut sample_count = 0u32;
            let mut sum_sq = 0.0f64;

            while r_clone.load(Ordering::Relaxed) {
                match stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let is_muted = m_clone.load(Ordering::Relaxed);
                        let vol = *v_clone.lock().unwrap();

                        let mut processed_buf = buf[..n].to_vec();

                        for chunk in processed_buf.chunks_exact_mut(2) {
                            let raw_sample = i16::from_le_bytes([chunk[0], chunk[1]]);
                            let sample_val = if is_muted {
                                0f64
                            } else {
                                (raw_sample as f64) * vol
                            };

                            let clamped = sample_val.clamp(-32768.0, 32767.0) as i16;
                            let bytes = clamped.to_le_bytes();
                            chunk[0] = bytes[0];
                            chunk[1] = bytes[1];

                            sum_sq += (clamped as f64) * (clamped as f64);
                            sample_count += 1;
                        }

                        if stdin.write_all(&processed_buf).is_err() {
                            break;
                        }
                        let _ = stdin.flush();

                        if sample_count >= (sample_rate / 20) {
                            let rms = (sum_sq / (sample_count as f64)).sqrt();
                            let level = (rms / 32768.0).clamp(0.0, 1.0);
                            app.emit("mic-audio-level", level).ok();

                            sample_count = 0;
                            sum_sq = 0.0;
                        }
                    }
                    Err(ref e)
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            || e.kind() == std::io::ErrorKind::TimedOut =>
                    {
                        continue;
                    }
                    Err(e) => {
                        eprintln!("[virtual_mic] Read error: {e}");
                        break;
                    }
                }
            }

            app.emit(
                "virtual-mic-state-changed",
                serde_json::json!({ "active": false, "error": null }),
            )
            .ok();
            app.emit("mic-audio-level", 0.0).ok();
        });

        Ok(Self {
            running,
            muted,
            volume,
            null_sink_id,
            remap_source_id,
            pacat_child,
        })
    }

    fn setup_pulse_modules() -> Result<(Option<String>, Option<String>), String> {
        cleanup_pulse_modules();
        let sink_out = run_command(
            "pactl",
            &[
                "load-module",
                "module-null-sink",
                "sink_name=SyncMicSink",
                "sink_properties=device.description=\"Sync_Microphone_Sink\"",
            ],
        );

        let sink_id = match sink_out {
            Ok(id) => Some(id),
            Err(e) => {
                eprintln!("[virtual_mic] Notice: null-sink load result: {e}");
                None
            }
        };

        let source_out = run_command(
            "pactl",
            &[
                "load-module",
                "module-remap-source",
                "source_name=SyncMic",
                "master=SyncMicSink.monitor",
                "source_properties=device.description=\"Sync Microphone\"",
            ],
        );

        let source_id = match source_out {
            Ok(id) => Some(id),
            Err(e) => {
                eprintln!("[virtual_mic] Notice: remap-source load result: {e}");
                None
            }
        };

        Ok((sink_id, source_id))
    }

    pub fn set_muted(&self, muted: bool) {
        self.muted.store(muted, Ordering::Relaxed);
    }

    pub fn set_volume(&self, volume: f64) {
        let mut guard = self.volume.lock().unwrap();
        *guard = volume.clamp(0.0, 2.0);
    }
}

pub fn cleanup_pulse_modules() {
    let output = match run_command("pactl", &["list", "modules", "short"]) {
        Ok(out) => out,
        Err(_) => return,
    };

    let mut sources_to_unload = Vec::new();
    let mut sinks_to_unload = Vec::new();

    for line in output.lines() {
        if line.contains("SyncMic") || line.contains("SyncMicSink") || line.contains("Sync_Microphone_Sink") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if !parts.is_empty() {
                let id = parts[0];
                if line.contains("module-remap-source") || (parts.len() > 1 && parts[1].contains("remap-source")) {
                    sources_to_unload.push(id.to_string());
                } else if line.contains("module-null-sink") || (parts.len() > 1 && parts[1].contains("null-sink")) {
                    sinks_to_unload.push(id.to_string());
                }
            }
        }
    }

    for id in sources_to_unload {
        let _ = run_command("pactl", &["unload-module", &id]);
    }
    for id in sinks_to_unload {
        let _ = run_command("pactl", &["unload-module", &id]);
    }
}

impl Drop for VirtualMicrophone {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);

        if let Ok(mut child_guard) = self.pacat_child.lock() {
            if let Some(mut child) = child_guard.take() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }

        if let Some(ref source_id) = self.remap_source_id {
            let _ = run_command("pactl", &["unload-module", source_id]);
        }

        if let Some(ref sink_id) = self.null_sink_id {
            let _ = run_command("pactl", &["unload-module", sink_id]);
        }

        cleanup_pulse_modules();
    }
}

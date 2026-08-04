use std::io::Write;
use std::net::TcpListener;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct AudioStreamServer {
    running: Arc<AtomicBool>,
    pub port: u16,
}

/// Build a WAV header for an infinite-length stream (data size = 0xFFFFFFFF).
/// 44100 Hz, 16-bit signed LE, stereo.
fn wav_header() -> [u8; 44] {
    let channels: u16 = 2;
    let sample_rate: u32 = 44100;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * (channels as u32) * (bits_per_sample as u32 / 8);
    let block_align = channels * (bits_per_sample / 8);
    let data_size: u32 = 0xFFFF_FFFF; // streaming – unknown length

    let mut h = [0u8; 44];
    // RIFF header
    h[0..4].copy_from_slice(b"RIFF");
    h[4..8].copy_from_slice(&(36_u32.wrapping_add(data_size)).to_le_bytes()); // file size - 8
    h[8..12].copy_from_slice(b"WAVE");
    // fmt sub-chunk
    h[12..16].copy_from_slice(b"fmt ");
    h[16..20].copy_from_slice(&16u32.to_le_bytes()); // sub-chunk size
    h[20..22].copy_from_slice(&1u16.to_le_bytes());  // PCM format
    h[22..24].copy_from_slice(&channels.to_le_bytes());
    h[24..28].copy_from_slice(&sample_rate.to_le_bytes());
    h[28..32].copy_from_slice(&byte_rate.to_le_bytes());
    h[32..34].copy_from_slice(&block_align.to_le_bytes());
    h[34..36].copy_from_slice(&bits_per_sample.to_le_bytes());
    // data sub-chunk
    h[36..40].copy_from_slice(b"data");
    h[40..44].copy_from_slice(&data_size.to_le_bytes());
    h
}

impl AudioStreamServer {
    pub fn start() -> Result<Self, String> {
        let listener = TcpListener::bind("0.0.0.0:0").map_err(|e| format!("Bind failed: {e}"))?;
        let port = listener.local_addr().map_err(|e| e.to_string())?.port();
        listener.set_nonblocking(true).map_err(|e| format!("Set nonblocking failed: {e}"))?;
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        thread::spawn(move || {
            loop {
                if !running_clone.load(Ordering::Relaxed) {
                    break;
                }
                match listener.accept() {
                    Ok((mut socket, addr)) => {
                        socket.set_nonblocking(false).ok();
                        socket.set_nodelay(true).ok(); // Extremely important for low latency
                        println!("[audio] Client connected from {addr}");

                        // Send HTTP response with WAV content type.
                        let headers = "HTTP/1.1 200 OK\r\n\
                                       Content-Type: audio/wav\r\n\
                                       Connection: close\r\n\
                                       Cache-Control: no-cache\r\n\r\n";
                        if let Err(e) = socket.write_all(headers.as_bytes()) {
                            eprintln!("[audio] Failed to write HTTP headers: {e}");
                            continue;
                        }

                        // Write WAV header first
                        if let Err(e) = socket.write_all(&wav_header()) {
                            eprintln!("[audio] Failed to write WAV header: {e}");
                            continue;
                        }

                        let monitor = "@DEFAULT_SINK@.monitor";

                        // parec outputs raw PCM (s16le, stereo, 44100 Hz).
                        // Works on PulseAudio and PipeWire (via PulseAudio compat).
                        let parec = Command::new("parec")
                            .arg("--rate=44100")
                            .arg("--channels=2")
                            .arg("--format=s16le")
                            .arg("--latency-msec=5")
                            .arg("--process-time-msec=5")
                            .arg("-d")
                            .arg(monitor)
                            .stdout(Stdio::piped())
                            .stderr(Stdio::null())
                            .spawn();

                        let mut parec_child = match parec {
                            Ok(c) => c,
                            Err(e) => {
                                eprintln!("[audio] Failed to start parec: {e}. Is pulseaudio-utils installed?");
                                continue;
                            }
                        };

                        let mut parec_out = match parec_child.stdout.take() {
                            Some(out) => out,
                            None => {
                                eprintln!("[audio] parec stdout unavailable");
                                let _ = parec_child.kill();
                                continue;
                            }
                        };

                        let r_clone = running_clone.clone();
                        thread::spawn(move || {
                            let mut buf = [0u8; 512];
                            use std::io::Read;
                            while r_clone.load(Ordering::Relaxed) {
                                match parec_out.read(&mut buf) {
                                    Ok(0) => break,
                                    Ok(n) => {
                                        if socket.write_all(&buf[..n]).is_err() {
                                            break;
                                        }
                                        socket.flush().ok();
                                    }
                                    Err(e) => {
                                        eprintln!("[audio] Read error: {e}");
                                        break;
                                    }
                                }
                            }
                            let _ = parec_child.kill();
                            let _ = parec_child.wait();
                            println!("[audio] Stream ended");
                        });
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(100));
                    }
                    Err(e) => {
                        eprintln!("[audio] Accept failed: {e}");
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        });

        println!("[audio] AudioStreamServer started on port {port}");
        Ok(Self { running, port })
    }
}

impl Drop for AudioStreamServer {
    fn drop(&mut self) {
        println!("[audio] AudioStreamServer stopping");
        self.running.store(false, Ordering::Relaxed);
    }
}

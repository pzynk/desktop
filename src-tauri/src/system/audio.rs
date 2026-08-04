use std::io::Write;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub struct AudioStreamServer {
    running: Arc<AtomicBool>,
    pub port: u16,
}

fn wav_header() -> [u8; 44] {
    let channels: u16 = 2;
    let sample_rate: u32 = 44100;
    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * (channels as u32) * (bits_per_sample as u32 / 8);
    let block_align = channels * (bits_per_sample / 8);
    let data_size: u32 = 0xFFFF_FFFF;

    let mut h = [0u8; 44];
    h[0..4].copy_from_slice(b"RIFF");
    h[4..8].copy_from_slice(&(36_u32.wrapping_add(data_size)).to_le_bytes());
    h[8..12].copy_from_slice(b"WAVE");
    h[12..16].copy_from_slice(b"fmt ");
    h[16..20].copy_from_slice(&16u32.to_le_bytes());
    h[20..22].copy_from_slice(&1u16.to_le_bytes());
    h[22..24].copy_from_slice(&channels.to_le_bytes());
    h[24..28].copy_from_slice(&sample_rate.to_le_bytes());
    h[28..32].copy_from_slice(&byte_rate.to_le_bytes());
    h[32..34].copy_from_slice(&block_align.to_le_bytes());
    h[34..36].copy_from_slice(&bits_per_sample.to_le_bytes());
    h[36..40].copy_from_slice(b"data");
    h[40..44].copy_from_slice(&data_size.to_le_bytes());
    h
}

#[cfg(not(target_os = "windows"))]
fn stream_audio_linux(mut socket: std::net::TcpStream, running: Arc<AtomicBool>) {
    use std::process::{Command, Stdio};
    use std::io::Read;

    let monitor = "@DEFAULT_SINK@.monitor";
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
            eprintln!("[audio] Failed to start parec: {e}");
            return;
        }
    };

    let mut parec_out = match parec_child.stdout.take() {
        Some(out) => out,
        None => {
            let _ = parec_child.kill();
            return;
        }
    };

    let mut buf = [0u8; 512];
    while running.load(Ordering::Relaxed) {
        match parec_out.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if socket.write_all(&buf[..n]).is_err() {
                    break;
                }
                let _ = socket.flush();
            }
            Err(_) => break,
        }
    }
    let _ = parec_child.kill();
    let _ = parec_child.wait();
}

#[cfg(target_os = "windows")]
fn stream_audio_windows(mut socket: std::net::TcpStream, running: Arc<AtomicBool>) {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let host = cpal::default_host();
    let device = match host.default_output_device() {
        Some(d) => d,
        None => {
            eprintln!("[audio] No default output device found");
            return;
        }
    };

    let config = cpal::StreamConfig {
        channels: 2,
        sample_rate: cpal::SampleRate(44100),
        buffer_size: cpal::BufferSize::Default,
    };

    let (tx, rx) = std::sync::mpsc::sync_channel::<Vec<u8>>(100);

    let stream = device.build_input_stream(
        &config,
        move |data: &[f32], _: &_| {
            let mut pcm = Vec::with_capacity(data.len() * 2);
            for &sample in data {
                let s = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
                pcm.extend_from_slice(&s.to_le_bytes());
            }
            let _ = tx.send(pcm);
        },
        |err| eprintln!("[audio] Stream error: {err}"),
        None,
    );

    let stream = match stream {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[audio] Failed to build loopback stream: {e}");
            return;
        }
    };

    if let Err(e) = stream.play() {
        eprintln!("[audio] Failed to play stream: {e}");
        return;
    }

    while running.load(Ordering::Relaxed) {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(bytes) => {
                if socket.write_all(&bytes).is_err() {
                    break;
                }
                let _ = socket.flush();
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
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
                        socket.set_nodelay(true).ok();
                        println!("[audio] Client connected from {addr}");

                        let headers = "HTTP/1.1 200 OK\r\n\
                                       Content-Type: audio/wav\r\n\
                                       Connection: close\r\n\
                                       Cache-Control: no-cache\r\n\r\n";
                        if socket.write_all(headers.as_bytes()).is_err() {
                            continue;
                        }

                        if socket.write_all(&wav_header()).is_err() {
                            continue;
                        }

                        let r_clone = running_clone.clone();
                        thread::spawn(move || {
                            #[cfg(not(target_os = "windows"))]
                            stream_audio_linux(socket, r_clone);

                            #[cfg(target_os = "windows")]
                            stream_audio_windows(socket, r_clone);

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

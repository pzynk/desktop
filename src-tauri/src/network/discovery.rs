//! UDP broadcast based device discovery.
//!
//! Periodically broadcasts a [`BroadcastMessage`] containing the local
//! IP and hostname so that peers on the same LAN can find this device.
//! Broadcasting can be paused at runtime by setting the `visible` flag to
//! `false`.

use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::network::message::BroadcastMessage;
use crate::system::get_local_ip;

/// How often the discovery packet is sent.
const BROADCAST_INTERVAL: Duration = Duration::from_secs(2);

/// Spawn a background thread that keeps broadcasting this device's
/// presence on the given UDP `port`. The advertised `tcp_port` is
/// included in the payload so peers know where to open the sync
/// connection.
///
/// When `visible` is set to `false` the loop keeps running but skips
/// sending — so toggling back on is instant with no re-bind needed.
pub fn start_broadcast(
    port: u16,
    tcp_port: u16,
    device_id: String,
    name: String,
    visible: Arc<AtomicBool>,
) {
    thread::spawn(move || {
        let socket = match UdpSocket::bind("0.0.0.0:0") {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[discovery] Failed to bind UDP socket: {}", e);
                return;
            }
        };

        if let Err(e) = socket.set_broadcast(true) {
            eprintln!("[discovery] Failed to enable UDP broadcast: {}", e);
            return;
        }

        let broadcast_addr = format!("255.255.255.255:{}", port);
        println!("[discovery] Broadcasting on {}", broadcast_addr);

        loop {
            if visible.load(Ordering::Relaxed) {
                let msg = BroadcastMessage {
                    ip: get_local_ip().unwrap_or_else(|| "127.0.0.1".to_string()),
                    name: name.clone(),
                    port: tcp_port,
                    device_id: device_id.clone(),
                    os: std::env::consts::OS.to_string(),
                };

                match serde_json::to_string(&msg) {
                    Ok(serialized) => {
                        if let Err(e) = socket.send_to(serialized.as_bytes(), &broadcast_addr) {
                            eprintln!("[discovery] Failed to send broadcast: {}", e);
                        }
                    }
                    Err(e) => eprintln!("[discovery] Failed to serialize message: {}", e),
                }
            }

            thread::sleep(BROADCAST_INTERVAL);
        }
    });
}

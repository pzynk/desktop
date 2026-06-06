//! TCP server that accepts incoming sync connections from peers.

use std::net::TcpListener;
use std::thread;

use crate::network::session::{handle_client, SessionContext};

/// Spawn a background thread that listens for TCP connections on
/// `0.0.0.0:port`. Each accepted connection is handled in its own
/// thread by [`handle_client`].
pub fn start_server(port: u16, ctx: SessionContext) {
    thread::spawn(move || {
        let addr = format!("0.0.0.0:{}", port);
        let listener = match TcpListener::bind(&addr) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("[tcp] Failed to bind {}: {}", addr, e);
                return;
            }
        };

        println!("[tcp] Listening on {}", addr);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let peer = stream
                        .peer_addr()
                        .map(|a| a.to_string())
                        .unwrap_or_else(|_| "<unknown>".into());
                    println!("[tcp] New connection from {}", peer);
                    let session_ctx = ctx.clone();
                    thread::spawn(move || handle_client(stream, session_ctx));
                }
                Err(e) => eprintln!("[tcp] Accept error: {}", e),
            }
        }
    });
}

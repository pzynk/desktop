//! Helpers for retrieving information about the host system.

use std::net::UdpSocket;

/// Returns the primary local IPv4 address of this machine, if it can be determined.
///
/// Works by opening a UDP socket and "connecting" it to a public address
/// (no packets are actually sent) so the OS chooses the appropriate
/// outbound interface.
pub fn get_local_ip() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let local_addr = socket.local_addr().ok()?;
    Some(local_addr.ip().to_string())
}

/// Returns the system hostname, if it can be read as UTF-8.
pub fn get_system_name() -> Option<String> {
    gethostname::gethostname().into_string().ok()
}

//! Wire-format messages exchanged between the desktop and the Android peer.

use serde::{Deserialize, Serialize};

/// Payload broadcast over UDP so that peers (e.g. the Android app) can
/// discover this desktop on the local network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BroadcastMessage {
    /// Local IPv4 address of the broadcasting device.
    pub ip: String,
    /// Human-readable hostname of the broadcasting device.
    pub name: String,
    /// TCP port peers should connect to for the sync connection.
    pub port: u16,
    /// Stable non-secret UUID for this desktop, used to find stored trust.
    pub device_id: String,
    /// The operating system of this device.
    pub os: String,
}

//! System-level utilities: host info, local network address, etc.

pub mod device_id;
pub mod info;
pub mod trusted_peers;
pub mod media;
pub mod terminal;

pub use device_id::get_or_create_device_id;
pub use info::{get_local_ip, get_system_name};
pub use trusted_peers::{now_unix, TrustedPeer, TrustedPeers};


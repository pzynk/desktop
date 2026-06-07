//! Line-delimited JSON handshake protocol.

use std::io::{BufRead, Write};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    Hello {
        device_id: String,
        name: String,
        token: Option<String>,
    },
    PairRequest {
        device_id: String,
        name: String,
        nonce: String,
    },
    ClipboardUpdate {
        text: String,
    },
    MediaCommand {
        command: String,
        value: Option<f64>,
    },
    IncomingFile {
        filename: String,
        base64_data: String,
        sha256: String,
    },
    FileTransferStart {
        filename: String,
        total_bytes: u64,
    },
    CameraStreamStarted {
        port: u16,
        #[serde(default)]
        use_adb: bool,
    },

    CameraStreamStopped,
    Unpair,
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum ServerMessage {
    HelloOk {
        device_id: String,
        name: String,
    },
    PairRequired,
    PairAccepted {
        device_id: String,
        name: String,
        token: String,
    },
    PairRejected {
        reason: String,
    },
    ClipboardUpdate {
        text: String,
    },
    IncomingFile {
        filename: String,
        base64_data: String,
        sha256: String,
    },
    MediaState(crate::system::media::MediaState),
    SystemVolumeUpdate {
        volume: f64,
        muted: bool,
    },
    TerminalServerInfo {
        enabled: bool,
        port: u16,
        username: String,
        password: Option<String>,
    },
    StartCameraStream,
    StopCameraStream,
    Unpair,
}

pub fn read_line_json<T: DeserializeOwned>(reader: &mut impl BufRead) -> Result<Option<T>, String> {
    let mut line = String::new();
    let read = reader
        .read_line(&mut line)
        .map_err(|e| format!("Could not read protocol line: {e}"))?;
    if read == 0 {
        return Ok(None);
    }
    serde_json::from_str(line.trim())
        .map(Some)
        .map_err(|e| format!("Could not parse protocol message: {e}; raw={}", line.trim()))
}

pub fn write_line_json<T: Serialize>(writer: &mut impl Write, message: &T) -> Result<(), String> {
    let raw = serde_json::to_string(message)
        .map_err(|e| format!("Could not serialize protocol message: {e}"))?;
    writer
        .write_all(raw.as_bytes())
        .and_then(|_| writer.write_all(b"\n"))
        .and_then(|_| writer.flush())
        .map_err(|e| format!("Could not write protocol message: {e}"))
}

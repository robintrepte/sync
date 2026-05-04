use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload")]
pub enum WireMessage {
    PairingRequest(PairingRequest),
    PairingAck(PairingAck),
    InputEvent(InputEvent),
    ClipboardTextUpdate(ClipboardTextUpdate),
    FocusChange(FocusChange),
    Heartbeat(Heartbeat),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingRequest {
    pub device_name: String,
    pub pairing_code: String,
    pub cert_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingAck {
    pub peer_id: Uuid,
    pub accepted: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputEvent {
    pub sequence: u64,
    pub event: InputEventKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum InputEventKind {
    MouseMove { dx: i32, dy: i32 },
    MouseButton { button: u8, down: bool },
    MouseWheel { delta_x: i32, delta_y: i32 },
    Key { code: u32, down: bool },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardTextUpdate {
    pub revision: u64,
    pub source_peer: Uuid,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusChange {
    pub target_peer: Uuid,
    pub has_focus: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Heartbeat {
    pub timestamp_ms: u64,
}

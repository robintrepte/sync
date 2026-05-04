#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod core {
    pub mod network;
    pub mod protocol;
    pub mod session;
}
mod platform {
    pub mod macos;
    pub mod windows;
}

use std::{collections::HashMap, sync::Arc};

use core::{
    network::{NetworkRuntime, NetworkStatus},
    session::{PeerTrustRecord, SessionManager},
};
use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Default)]
struct RuntimeState {
    session: Arc<SessionManager>,
    network: Arc<NetworkRuntime>,
    last_clipboard_revision: Arc<RwLock<u64>>,
    last_clipboard_source: Arc<RwLock<Option<Uuid>>>,
    last_clipboard_hash: Arc<RwLock<Option<u64>>>,
    active_target_peer: Arc<RwLock<Option<Uuid>>>,
    last_heartbeat_ms: Arc<RwLock<u64>>,
    edge_layout_map: Arc<RwLock<HashMap<String, Uuid>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PairingResponse {
    accepted: bool,
    peer_id: Option<Uuid>,
    reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClipboardSyncRequest {
    text: String,
    source_peer: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LayoutSwitchRequest {
    current_x: i32,
    current_y: i32,
    desktop_width: i32,
    desktop_height: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LayoutConfigRequest {
    left: Option<String>,
    right: Option<String>,
    top: Option<String>,
    bottom: Option<String>,
}

#[tauri::command]
async fn generate_pairing_code(state: State<'_, RuntimeState>) -> Result<String, String> {
    Ok(state.session.new_pairing_code().await)
}

#[tauri::command]
async fn submit_pairing(
    state: State<'_, RuntimeState>,
    device_name: String,
    pairing_code: String,
    cert_fingerprint: String,
) -> Result<PairingResponse, String> {
    if !state.session.verify_pairing_code(&pairing_code).await {
        return Ok(PairingResponse {
            accepted: false,
            peer_id: None,
            reason: Some("Invalid pairing code".to_string()),
        });
    }

    let trusted = state
        .session
        .trust_peer(device_name, cert_fingerprint)
        .await
        .map_err(|e| e.to_string())?;

    Ok(PairingResponse {
        accepted: true,
        peer_id: Some(trusted.peer_id),
        reason: None,
    })
}

#[tauri::command]
async fn trusted_peers(state: State<'_, RuntimeState>) -> Result<Vec<PeerTrustRecord>, String> {
    Ok(state.session.list_trusted_peers().await)
}

#[tauri::command]
async fn relay_clipboard_text(
    state: State<'_, RuntimeState>,
    req: ClipboardSyncRequest,
) -> Result<u64, String> {
    let text_hash = fxhash(&req.text);
    let mut last_hash_guard = state.last_clipboard_hash.write().await;
    let mut last_source_guard = state.last_clipboard_source.write().await;

    // Loop prevention: ignore repeated payloads from the same source peer.
    if last_source_guard.as_ref() == Some(&req.source_peer) && last_hash_guard.as_ref() == Some(&text_hash) {
        return Ok(*state.last_clipboard_revision.read().await);
    }

    let mut revision = state.last_clipboard_revision.write().await;
    *revision += 1;
    *last_source_guard = Some(req.source_peer);
    *last_hash_guard = Some(text_hash);

    platform::windows::set_clipboard_text(&req.text).ok();
    platform::macos::set_clipboard_text(&req.text).ok();

    Ok(*revision)
}

#[tauri::command]
async fn evaluate_edge_handoff(
    state: State<'_, RuntimeState>,
    req: LayoutSwitchRequest,
) -> Result<Option<String>, String> {
    let at_left_edge = req.current_x <= 0;
    let at_right_edge = req.current_x >= req.desktop_width - 1;
    let at_top_edge = req.current_y <= 0;
    let at_bottom_edge = req.current_y >= req.desktop_height - 1;

    let direction = if at_left_edge {
        Some("left")
    } else if at_right_edge {
        Some("right")
    } else if at_top_edge {
        Some("top")
    } else if at_bottom_edge {
        Some("bottom")
    } else {
        None
    };

    if let Some(dir) = direction {
        let mut active_target = state.active_target_peer.write().await;
        let layout = state.edge_layout_map.read().await;
        *active_target = layout.get(dir).cloned();
    }

    Ok(direction.map(ToOwned::to_owned))
}

#[tauri::command]
async fn configure_layout(
    state: State<'_, RuntimeState>,
    req: LayoutConfigRequest,
) -> Result<(), String> {
    let peers = state.session.list_trusted_peers().await;
    let mut by_name = HashMap::<String, Uuid>::new();
    for peer in peers {
        by_name.insert(peer.name, peer.peer_id);
    }

    let mut layout = state.edge_layout_map.write().await;
    layout.clear();
    if let Some(name) = req.left {
        if let Some(id) = by_name.get(&name) {
            layout.insert("left".to_string(), *id);
        }
    }
    if let Some(name) = req.right {
        if let Some(id) = by_name.get(&name) {
            layout.insert("right".to_string(), *id);
        }
    }
    if let Some(name) = req.top {
        if let Some(id) = by_name.get(&name) {
            layout.insert("top".to_string(), *id);
        }
    }
    if let Some(name) = req.bottom {
        if let Some(id) = by_name.get(&name) {
            layout.insert("bottom".to_string(), *id);
        }
    }
    Ok(())
}

#[tauri::command]
async fn fail_safe_release(state: State<'_, RuntimeState>) -> Result<(), String> {
    let mut active_target = state.active_target_peer.write().await;
    *active_target = None;
    Ok(())
}

#[tauri::command]
async fn health_check() -> Result<String, String> {
    Ok("ok".to_string())
}

#[tauri::command]
async fn start_host_listener(
    state: State<'_, RuntimeState>,
    bind_addr: String,
) -> Result<NetworkStatus, String> {
    state
        .network
        .start_host(state.session.clone(), bind_addr)
        .await
        .map_err(|e| e.to_string())?;
    Ok(state.network.status().await)
}

#[tauri::command]
async fn connect_to_host(
    state: State<'_, RuntimeState>,
    host_addr: String,
    pairing_code: String,
    device_name: String,
) -> Result<NetworkStatus, String> {
    state
        .network
        .connect_to_host(state.session.clone(), host_addr, pairing_code, device_name)
        .await
        .map_err(|e| e.to_string())?;
    Ok(state.network.status().await)
}

#[tauri::command]
async fn network_status(state: State<'_, RuntimeState>) -> Result<NetworkStatus, String> {
    Ok(state.network.status().await)
}

#[tauri::command]
async fn start_input_capture() -> Result<(), String> {
    platform::windows::start_input_capture().map_err(|e| e.to_string())?;
    platform::macos::start_input_capture().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn inject_test_input(dx: i32, dy: i32, key_code: u32) -> Result<(), String> {
    platform::windows::inject_mouse_move(dx, dy).ok();
    platform::windows::inject_mouse_button(1, true).ok();
    platform::windows::inject_mouse_button(1, false).ok();
    platform::windows::inject_key(key_code, true).ok();
    platform::windows::inject_key(key_code, false).ok();
    platform::macos::inject_mouse_move(dx, dy).ok();
    platform::macos::inject_mouse_button(1, true).ok();
    platform::macos::inject_mouse_button(1, false).ok();
    platform::macos::inject_key(key_code, true).ok();
    platform::macos::inject_key(key_code, false).ok();
    Ok(())
}

#[tauri::command]
async fn heartbeat(state: State<'_, RuntimeState>) -> Result<u64, String> {
    let mut guard = state.last_heartbeat_ms.write().await;
    *guard = now_ms();
    Ok(*guard)
}

fn now_ms() -> u64 {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_millis() as u64
}

fn fxhash(input: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

#[tokio::main]
async fn main() {
    let session = SessionManager::initialize()
        .await
        .expect("failed to initialize session manager");

    let state = RuntimeState {
        session: Arc::new(session),
        network: Arc::new(NetworkRuntime::new()),
        ..Default::default()
    };

    tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            generate_pairing_code,
            submit_pairing,
            trusted_peers,
            relay_clipboard_text,
            configure_layout,
            evaluate_edge_handoff,
            fail_safe_release,
            health_check,
            heartbeat,
            start_host_listener,
            connect_to_host,
            network_status,
            start_input_capture,
            inject_test_input
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

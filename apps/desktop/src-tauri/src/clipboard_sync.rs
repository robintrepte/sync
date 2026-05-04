use std::{
    hash::{Hash, Hasher},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::sync::RwLock;
use uuid::Uuid;

/// Dedupes echo when we apply remote clipboard and keeps revision for wire protocol.
pub struct ClipboardSyncState {
    revision: Arc<RwLock<u64>>,
    last_broadcast_hash: Arc<RwLock<Option<u64>>>,
    suppress_poll_until_ms: Arc<AtomicU64>,
}

fn fxhash(input: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(std::time::Duration::from_secs(0))
        .as_millis() as u64
}

impl ClipboardSyncState {
    pub fn new() -> Self {
        Self {
            revision: Arc::new(RwLock::new(0)),
            last_broadcast_hash: Arc::new(RwLock::new(None)),
            suppress_poll_until_ms: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Called when peer sends clipboard text — updates local OS clipboard.
    pub async fn note_remote_apply(&self, text: &str) {
        let h = fxhash(text);
        *self.last_broadcast_hash.write().await = Some(h);
        self.suppress_poll_until_ms
            .store(now_ms().saturating_add(1200), Ordering::SeqCst);
    }

    pub fn poll_allowed(&self) -> bool {
        now_ms() >= self.suppress_poll_until_ms.load(Ordering::SeqCst)
    }

    /// Build outgoing update if local clipboard text changed since last broadcast.
    pub async fn prepare_outgoing(
        &self,
        device_id: Uuid,
        text: &str,
    ) -> Option<(u64, crate::core::protocol::ClipboardTextUpdate)> {
        if text.is_empty() {
            return None;
        }
        if !self.poll_allowed() {
            return None;
        }
        let h = fxhash(text);
        let guard = self.last_broadcast_hash.read().await;
        if guard.as_ref() == Some(&h) {
            return None;
        }
        drop(guard);

        let mut rev = self.revision.write().await;
        *rev += 1;
        let revision = *rev;
        *self.last_broadcast_hash.write().await = Some(h);

        Some((
            revision,
            crate::core::protocol::ClipboardTextUpdate {
                revision,
                source_peer: device_id,
                text: text.to_string(),
            },
        ))
    }

    /// Legacy manual relay — align broadcast hash so poll does not immediately echo.
    pub async fn note_local_manual_set(&self, text: &str) {
        *self.last_broadcast_hash.write().await = Some(fxhash(text));
    }
}

use std::{
    collections::HashMap,
    fs,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use directories::ProjectDirs;
use rand::{distributions::Alphanumeric, Rng};
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer},
    ClientConfig, RootCertStore, ServerConfig,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use uuid::Uuid;

use super::protocol::WireMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerTrustRecord {
    pub peer_id: Uuid,
    pub name: String,
    pub fingerprint: String,
    pub trusted_at_ms: u64,
    pub cert_der_base64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingState {
    pub current_code: Option<String>,
    pub code_expires_at_ms: u64,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct TrustStoreFile {
    pub peers: Vec<PeerTrustRecord>,
}

#[derive(Clone)]
pub struct SessionManager {
    pub pairing_state: Arc<RwLock<PairingState>>,
    pub trust_map: Arc<RwLock<HashMap<Uuid, PeerTrustRecord>>>,
    pub identity: Arc<RwLock<LocalIdentity>>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self {
            pairing_state: Arc::new(RwLock::new(PairingState::default())),
            trust_map: Arc::new(RwLock::new(HashMap::new())),
            identity: Arc::new(RwLock::new(LocalIdentity {
                cert_der_base64: String::new(),
                key_der_base64: String::new(),
                device_id: None,
            })),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalIdentity {
    pub cert_der_base64: String,
    pub key_der_base64: String,
    #[serde(default)]
    pub device_id: Option<Uuid>,
}

impl Default for PairingState {
    fn default() -> Self {
        Self {
            current_code: None,
            code_expires_at_ms: 0,
        }
    }
}

impl SessionManager {
    pub async fn initialize() -> Result<Self> {
        let mut mgr = SessionManager::default();
        let trust = mgr.load_trust_store()?;
        let identity = mgr.load_or_create_local_identity()?;
        mgr.identity = Arc::new(RwLock::new(identity));
        let mut guard = mgr.trust_map.write().await;
        for peer in trust.peers {
            guard.insert(peer.peer_id, peer);
        }
        drop(guard);
        Ok(mgr)
    }

    pub async fn new_pairing_code(&self) -> String {
        let code: String = rand::thread_rng()
            .sample_iter(Alphanumeric)
            .map(char::from)
            .filter(|c| c.is_ascii_alphanumeric())
            .take(6)
            .collect::<String>()
            .to_uppercase();
        let expires_at = now_ms() + 5 * 60_000;
        let mut guard = self.pairing_state.write().await;
        guard.current_code = Some(code.clone());
        guard.code_expires_at_ms = expires_at;
        code
    }

    pub async fn verify_pairing_code(&self, provided: &str) -> bool {
        let guard = self.pairing_state.read().await;
        guard.code_expires_at_ms >= now_ms()
            && guard
                .current_code
                .as_ref()
                .map(|v| v.eq_ignore_ascii_case(provided))
                .unwrap_or(false)
    }

    pub async fn trust_peer(&self, name: String, fingerprint: String) -> Result<PeerTrustRecord> {
        let record = PeerTrustRecord {
            peer_id: Uuid::new_v4(),
            name,
            fingerprint,
            trusted_at_ms: now_ms(),
            cert_der_base64: None,
        };
        let mut guard = self.trust_map.write().await;
        guard.insert(record.peer_id, record.clone());
        drop(guard);
        self.persist_trust_store().await?;
        Ok(record)
    }

    pub async fn list_trusted_peers(&self) -> Vec<PeerTrustRecord> {
        self.trust_map.read().await.values().cloned().collect()
    }

    pub async fn persist_trust_store(&self) -> Result<()> {
        let peers = self.list_trusted_peers().await;
        let contents = serde_json::to_string_pretty(&TrustStoreFile { peers })?;
        let path = trust_store_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, contents)?;
        Ok(())
    }

    pub fn tls_server_config(&self) -> Result<ServerConfig> {
        let identity = self.identity.blocking_read().clone();
        let cert_raw = STANDARD.decode(identity.cert_der_base64)?;
        let key_raw = STANDARD.decode(identity.key_der_base64)?;
        let cert = CertificateDer::from(cert_raw);
        let key = PrivatePkcs8KeyDer::from(key_raw);
        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert], PrivateKeyDer::Pkcs8(key))
            .context("failed to configure TLS server")?;
        Ok(config)
    }

    pub fn tls_client_config(&self) -> Result<ClientConfig> {
        let identity = self.identity.blocking_read().clone();
        let cert_raw = STANDARD.decode(identity.cert_der_base64)?;
        let cert = CertificateDer::from(cert_raw);
        let mut roots = RootCertStore::empty();
        roots.add(cert)?;
        let client = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        Ok(client)
    }

    pub async fn local_fingerprint(&self) -> Result<String> {
        let identity = self.identity.read().await.clone();
        let cert_raw = STANDARD.decode(identity.cert_der_base64)?;
        Ok(fingerprint_for_certificate(&cert_raw))
    }

    pub async fn process_wire_message(
        &self,
        remote: SocketAddr,
        message: WireMessage,
    ) -> Result<Option<WireMessage>> {
        match message {
            WireMessage::Heartbeat(_) => Ok(Some(WireMessage::Heartbeat(super::protocol::Heartbeat {
                timestamp_ms: now_ms(),
            }))),
            WireMessage::PairingRequest(req) => {
                let accepted = self.verify_pairing_code(&req.pairing_code).await;
                if !accepted {
                    return Ok(Some(WireMessage::PairingAck(super::protocol::PairingAck {
                        peer_id: Uuid::nil(),
                        accepted: false,
                        reason: Some("pairing code expired or invalid".to_string()),
                    })));
                }

                let record = self
                    .trust_peer(format!("{}@{}", req.device_name, remote), req.cert_fingerprint)
                    .await?;
                let mut pairing = self.pairing_state.write().await;
                pairing.current_code = None;
                pairing.code_expires_at_ms = 0;

                Ok(Some(WireMessage::PairingAck(super::protocol::PairingAck {
                    peer_id: record.peer_id,
                    accepted: true,
                    reason: None,
                })))
            }
            _ => Ok(None),
        }
    }

    fn load_trust_store(&self) -> Result<TrustStoreFile> {
        let path = trust_store_path()?;
        if !path.exists() {
            return Ok(TrustStoreFile::default());
        }
        let contents = fs::read_to_string(path)?;
        let store = serde_json::from_str::<TrustStoreFile>(&contents)?;
        Ok(store)
    }

    fn load_or_create_local_identity(&self) -> Result<LocalIdentity> {
        let path = identity_path()?;
        if path.exists() {
            let contents = fs::read_to_string(&path)?;
            let mut identity = serde_json::from_str::<LocalIdentity>(&contents)?;
            if identity.device_id.is_none() {
                identity.device_id = Some(Uuid::new_v4());
                fs::write(path, serde_json::to_string_pretty(&identity)?)?;
            }
            return Ok(identity);
        }

        let key_pair = KeyPair::generate()?;
        let mut params = CertificateParams::new(vec![
            "sync.local".to_string(),
            "lan-input-sync.local".to_string(),
            "localhost".to_string(),
        ])?;
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, "sync");
        params.distinguished_name = dn;
        let cert = params.self_signed(&key_pair)?;
        let identity = LocalIdentity {
            cert_der_base64: STANDARD.encode(cert.der()),
            key_der_base64: STANDARD.encode(key_pair.serialize_der()),
            device_id: Some(Uuid::new_v4()),
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, serde_json::to_string_pretty(&identity)?)?;
        Ok(identity)
    }

    pub async fn device_id(&self) -> Uuid {
        let mut guard = self.identity.write().await;
        if guard.device_id.is_none() {
            guard.device_id = Some(Uuid::new_v4());
            if let Ok(path) = identity_path() {
                let _ = fs::write(
                    path,
                    serde_json::to_string_pretty(&*guard).unwrap_or_default(),
                );
            }
        }
        guard.device_id.unwrap_or_else(Uuid::nil)
    }
}

pub fn fingerprint_for_certificate(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    STANDARD.encode(hasher.finalize())
}

fn trust_store_path() -> Result<PathBuf> {
    let proj = ProjectDirs::from("com", "robin", "sync")
        .context("unable to determine config directory")?;
    Ok(proj.config_dir().join("trusted-peers.json"))
}

fn identity_path() -> Result<PathBuf> {
    let proj = ProjectDirs::from("com", "robin", "sync")
        .context("unable to determine config directory")?;
    Ok(proj.config_dir().join("local-identity.json"))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_millis() as u64
}

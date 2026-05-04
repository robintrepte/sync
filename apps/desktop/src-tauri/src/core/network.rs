use std::{net::SocketAddr, sync::Arc};

use anyhow::{Context, Result};
use rustls::pki_types::ServerName;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::{mpsc, RwLock},
};
use tokio_rustls::{TlsAcceptor, TlsConnector};

use crate::clipboard_sync::ClipboardSyncState;

use super::{protocol::WireMessage, session::SessionManager};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkStatus {
    pub listening_addr: Option<String>,
    pub connected_peer: Option<String>,
    pub last_error: Option<String>,
}

pub type OutSender = mpsc::UnboundedSender<WireMessage>;

#[derive(Clone)]
pub struct NetworkRuntime {
    status: Arc<RwLock<NetworkStatus>>,
    outbound: Arc<RwLock<Option<OutSender>>>,
}

impl NetworkRuntime {
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(NetworkStatus::default())),
            outbound: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn status(&self) -> NetworkStatus {
        self.status.read().await.clone()
    }

    pub async fn set_outbound(&self, sender: Option<OutSender>) {
        *self.outbound.write().await = sender;
    }

    pub async fn has_outbound(&self) -> bool {
        self.outbound.read().await.is_some()
    }

    pub async fn send_wire(&self, msg: WireMessage) -> Result<()> {
        let guard = self.outbound.read().await;
        let Some(tx) = guard.as_ref() else {
            return Ok(());
        };
        tx.send(msg).context("clipboard channel closed")?;
        Ok(())
    }

    pub async fn start_host(
        &self,
        session: Arc<SessionManager>,
        clipboard: Arc<ClipboardSyncState>,
        bind_addr: String,
    ) -> Result<()> {
        let listener = TcpListener::bind(&bind_addr)
            .await
            .with_context(|| format!("failed to bind host listener at {bind_addr}"))?;

        {
            let mut status = self.status.write().await;
            status.listening_addr = Some(bind_addr.clone());
            status.last_error = None;
        }

        let tls_config = Arc::new(session.tls_server_config()?);
        let acceptor = TlsAcceptor::from(tls_config);
        let this = self.clone();

        tokio::spawn(async move {
            loop {
                let (socket, remote) = match listener.accept().await {
                    Ok(v) => v,
                    Err(e) => {
                        let mut status = this.status.write().await;
                        status.last_error = Some(format!("accept failed: {e}"));
                        continue;
                    }
                };

                let acceptor = acceptor.clone();
                let session = session.clone();
                let clipboard = clipboard.clone();
                let net_session = this.clone();
                let net_cleanup = this.clone();
                tokio::spawn(async move {
                    if let Err(e) =
                        run_host_session(acceptor, socket, remote, session, clipboard, net_session)
                            .await
                    {
                        let mut status = net_cleanup.status.write().await;
                        status.last_error = Some(e.to_string());
                    }
                    net_cleanup.set_outbound(None).await;
                });
            }
        });

        Ok(())
    }

    pub async fn connect_to_host(
        &self,
        session: Arc<SessionManager>,
        clipboard: Arc<ClipboardSyncState>,
        host_addr: String,
        pairing_code: String,
        device_name: String,
    ) -> Result<()> {
        let tcp = TcpStream::connect(&host_addr)
            .await
            .with_context(|| format!("failed to connect to host {host_addr}"))?;
        let connector = TlsConnector::from(Arc::new(session.tls_client_config()?));
        let server_name = ServerName::try_from("sync.local")
            .map_err(|_| anyhow::anyhow!("invalid TLS server name"))?;
        let tls = connector.connect(server_name, tcp).await?;

        let (read_half, mut write_half) = tokio::io::split(tls);

        let req = WireMessage::PairingRequest(super::protocol::PairingRequest {
            device_name,
            pairing_code,
            cert_fingerprint: session.local_fingerprint().await?,
        });
        let mut line = serde_json::to_string(&req)?;
        line.push('\n');
        write_half.write_all(line.as_bytes()).await?;
        write_half.flush().await?;

        let mut reader = BufReader::new(read_half);
        let mut incoming = String::new();
        reader.read_line(&mut incoming).await?;
        let ack_msg: WireMessage = serde_json::from_str(&incoming)
            .with_context(|| format!("invalid pairing ack: {incoming}"))?;
        if let WireMessage::PairingAck(ack) = ack_msg {
            if !ack.accepted {
                return Err(anyhow::anyhow!(
                    ack.reason.unwrap_or_else(|| "pairing rejected".to_string())
                ));
            }
        } else {
            return Err(anyhow::anyhow!("expected PairingAck"));
        }

        let (out_tx, out_rx) = mpsc::unbounded_channel::<WireMessage>();
        self.set_outbound(Some(out_tx.clone())).await;

        let mut status = self.status.write().await;
        status.connected_peer = Some(host_addr.clone());
        status.last_error = None;
        drop(status);

        tokio::spawn(writer_task(write_half, out_rx));

        let this = self.clone();
        let clip = clipboard.clone();
        let sess = session.clone();
        tokio::spawn(async move {
            read_clipboard_loop(reader, sess, clip).await;
            this.set_outbound(None).await;
            let mut s = this.status.write().await;
            s.connected_peer = None;
        });

        Ok(())
    }
}

async fn writer_task<W: AsyncWriteExt + Unpin>(
    mut write: W,
    mut rx: mpsc::UnboundedReceiver<WireMessage>,
) {
    while let Some(msg) = rx.recv().await {
        let Ok(mut line) = serde_json::to_string(&msg) else {
            continue;
        };
        line.push('\n');
        if write.write_all(line.as_bytes()).await.is_err() {
            break;
        }
        let _ = write.flush().await;
    }
}

async fn read_clipboard_loop<R: tokio::io::AsyncRead + Unpin>(
    mut reader: BufReader<R>,
    _session: Arc<SessionManager>,
    clipboard: Arc<ClipboardSyncState>,
) {
    let mut buf = String::new();
    loop {
        buf.clear();
        let n = reader.read_line(&mut buf).await.unwrap_or(0);
        if n == 0 {
            break;
        }
        let Ok(msg) = serde_json::from_str::<WireMessage>(&buf) else {
            continue;
        };
        match msg {
            WireMessage::ClipboardTextUpdate(u) => {
                clipboard.note_remote_apply(&u.text).await;
                #[cfg(target_os = "windows")]
                crate::platform::windows::set_clipboard_text(&u.text).ok();
                #[cfg(target_os = "macos")]
                crate::platform::macos::set_clipboard_text(&u.text).ok();
            }
            WireMessage::Heartbeat(_) => {}
            _ => {}
        }
    }
}

async fn run_host_session(
    acceptor: TlsAcceptor,
    socket: TcpStream,
    remote: SocketAddr,
    session: Arc<SessionManager>,
    clipboard: Arc<ClipboardSyncState>,
    net: NetworkRuntime,
) -> Result<()> {
    let tls = acceptor.accept(socket).await?;
    let (read_half, write_half) = tokio::io::split(tls);
    let mut reader = BufReader::new(read_half);

    let mut first = String::new();
    reader.read_line(&mut first).await?;
    let msg: WireMessage =
        serde_json::from_str(&first).with_context(|| format!("invalid first message from {remote}"))?;

    if !matches!(&msg, WireMessage::PairingRequest(_)) {
        return Err(anyhow::anyhow!("expected PairingRequest first"));
    }

    let Some(reply) = session.process_wire_message(remote, msg).await? else {
        return Err(anyhow::anyhow!("pairing produced no reply"));
    };

    let (out_tx, out_rx) = mpsc::unbounded_channel::<WireMessage>();
    net.set_outbound(Some(out_tx.clone())).await;

    let mut enc = serde_json::to_string(&reply)?;
    enc.push('\n');

    let mut write_join = write_half;
    write_join.write_all(enc.as_bytes()).await?;
    write_join.flush().await?;

    tokio::spawn(writer_task(write_join, out_rx));

    let mut status = net.status.write().await;
    status.connected_peer = Some(remote.to_string());
    status.last_error = None;
    drop(status);

    read_clipboard_loop(reader, session.clone(), clipboard.clone()).await;
    net.set_outbound(None).await;
    Ok(())
}

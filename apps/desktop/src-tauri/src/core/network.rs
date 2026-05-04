use std::{net::SocketAddr, sync::Arc};

use anyhow::{Context, Result};
use rustls::pki_types::ServerName;
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::RwLock,
};
use tokio_rustls::{TlsAcceptor, TlsConnector};

use super::{protocol::WireMessage, session::SessionManager};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkStatus {
    pub listening_addr: Option<String>,
    pub connected_peer: Option<String>,
    pub last_error: Option<String>,
}

#[derive(Clone, Default)]
pub struct NetworkRuntime {
    status: Arc<RwLock<NetworkStatus>>,
}

impl NetworkRuntime {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn status(&self) -> NetworkStatus {
        self.status.read().await.clone()
    }

    pub async fn start_host(&self, session: Arc<SessionManager>, bind_addr: String) -> Result<()> {
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
                let this_inner = this.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_host_connection(acceptor, socket, remote, session).await {
                        let mut status = this_inner.status.write().await;
                        status.last_error = Some(e.to_string());
                    } else {
                        let mut status = this_inner.status.write().await;
                        status.connected_peer = Some(remote.to_string());
                    }
                });
            }
        });

        Ok(())
    }

    pub async fn connect_to_host(
        &self,
        session: Arc<SessionManager>,
        host_addr: String,
        pairing_code: String,
        device_name: String,
    ) -> Result<()> {
        let tcp = TcpStream::connect(&host_addr)
            .await
            .with_context(|| format!("failed to connect to host {host_addr}"))?;
        let connector = TlsConnector::from(Arc::new(session.tls_client_config()?));
        let server_name = ServerName::try_from("lan-input-sync.local")
            .map_err(|_| anyhow::anyhow!("invalid TLS server name"))?;
        let tls = connector.connect(server_name, tcp).await?;

        let req = WireMessage::PairingRequest(super::protocol::PairingRequest {
            device_name,
            pairing_code,
            cert_fingerprint: session.local_fingerprint().await?,
        });
        let mut line = serde_json::to_string(&req)?;
        line.push('\n');

        let (read_half, mut write_half) = tokio::io::split(tls);
        write_half.write_all(line.as_bytes()).await?;
        write_half.flush().await?;

        let mut reader = BufReader::new(read_half);
        let mut incoming = String::new();
        reader.read_line(&mut incoming).await?;
        if let Ok(WireMessage::PairingAck(ack)) = serde_json::from_str::<WireMessage>(&incoming) {
            if !ack.accepted {
                return Err(anyhow::anyhow!(ack.reason.unwrap_or_else(|| "pairing rejected".to_string())));
            }
        }

        let mut status = self.status.write().await;
        status.connected_peer = Some(host_addr);
        status.last_error = None;
        Ok(())
    }
}

async fn handle_host_connection(
    acceptor: TlsAcceptor,
    socket: TcpStream,
    remote: SocketAddr,
    session: Arc<SessionManager>,
) -> Result<()> {
    let tls = acceptor.accept(socket).await?;
    let (read_half, mut write_half) = tokio::io::split(tls);
    let mut reader = BufReader::new(read_half);

    loop {
        let mut incoming = String::new();
        let read = reader.read_line(&mut incoming).await?;
        if read == 0 {
            break;
        }
        let message = serde_json::from_str::<WireMessage>(&incoming)
            .with_context(|| format!("invalid wire message from {remote}"))?;
        if let Some(reply) = session.process_wire_message(remote, message).await? {
            let mut encoded = serde_json::to_string(&reply)?;
            encoded.push('\n');
            write_half.write_all(encoded.as_bytes()).await?;
            write_half.flush().await?;
        }
    }
    Ok(())
}

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, RwLock,
    },
};

use anyhow::{Context, Result};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::Serialize;

pub const SERVICE_TYPE: &str = "_sync._tcp.local.";

#[derive(Clone, Serialize)]
pub struct NearbyPeer {
    pub id: String,
    pub name: String,
    pub address: String,
}

pub struct Discovery {
    daemon: ServiceDaemon,
    peers: Arc<RwLock<HashMap<String, NearbyPeer>>>,
    browse_started: AtomicBool,
    registered: AtomicBool,
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn peer_from_info(info: &ServiceInfo) -> Option<NearbyPeer> {
    let id = info.get_fullname().to_string();
    let name = info
        .get_property_val_str("name")
        .unwrap_or("Computer")
        .to_string();
    let ip = info.get_addresses_v4().iter().next()?.to_string();
    let port = info.get_port();
    Some(NearbyPeer {
        id,
        name,
        address: format!("{ip}:{port}"),
    })
}

fn primary_ipv4() -> Option<String> {
    let addrs = if_addrs::get_if_addrs().ok()?;
    for iface in addrs {
        if iface.is_loopback() {
            continue;
        }
        if let if_addrs::IfAddr::V4(v4) = iface.addr {
            return Some(v4.ip.to_string());
        }
    }
    None
}

impl Discovery {
    pub fn new() -> Result<Self> {
        Ok(Self {
            daemon: ServiceDaemon::new().context("mdns ServiceDaemon::new")?,
            peers: Arc::new(RwLock::new(HashMap::new())),
            browse_started: AtomicBool::new(false),
            registered: AtomicBool::new(false),
        })
    }

    pub fn register(&self, friendly_name: &str, port: u16) -> Result<()> {
        if self.registered.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let ip = primary_ipv4().unwrap_or_else(|| "127.0.0.1".to_string());
        let instance = sanitize(friendly_name);
        let host_name = format!("{}.local.", instance);
        let props = [("name", friendly_name)];

        let info =
            ServiceInfo::new(SERVICE_TYPE, &instance, &host_name, ip.as_str(), port, &props[..])
                .context("ServiceInfo::new")?;
        self.daemon.register(info).context("mdns register")?;
        Ok(())
    }

    pub fn start_browser(&self) -> Result<()> {
        if self.browse_started.swap(true, Ordering::SeqCst) {
            return Ok(());
        }
        let receiver = self.daemon.browse(SERVICE_TYPE.into()).context("mdns browse")?;
        let peers = self.peers.clone();

        std::thread::spawn(move || {
            while let Ok(event) = receiver.recv() {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        if let Some(p) = peer_from_info(&info) {
                            let mut g = peers.write().expect("discovery peers lock");
                            g.insert(p.id.clone(), p);
                        }
                    }
                    ServiceEvent::ServiceRemoved(_ty, fullname) => {
                        let mut g = peers.write().expect("discovery peers lock");
                        g.remove(&fullname);
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    pub fn list_peers(&self) -> Vec<NearbyPeer> {
        let my_ip = primary_ipv4();
        let mut out: Vec<NearbyPeer> = self
            .peers
            .read()
            .expect("discovery peers lock")
            .values()
            .filter(|p| {
                if let Some(ref mine) = my_ip {
                    if p.address.starts_with(&format!("{mine}:")) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name).then(a.address.cmp(&b.address)));
        out
    }
}

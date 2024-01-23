use crate::types::Heartbeat;
use dittolive_ditto::transport::{Peer, PresenceGraph};
use local_ip_address::{list_afinet_netifas, local_ip};
use log::info;
use serde_json;
use std::io;
use std::time;

pub fn system_time_msec() -> u64 {
    time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .expect("SystemTime::now")
        .as_millis()
        .try_into()
        .unwrap()
}

impl Heartbeat {
    pub fn update_timestamp(&mut self) {
        self.sent_at_msec = system_time_msec();
    }
}

fn concise_peer(p: &Peer) -> String {
    format!(
        "{}:{}",
        p.device_name,
        p.ditto_sdk_version
            .as_ref()
            .unwrap_or(&"? ver.".to_string())
    )
}

pub fn concise_presence(pg: &PresenceGraph) -> String {
    let mut out = String::new();
    // Only output device_name
    out.push_str("local: ");
    out.push_str(&concise_peer(&pg.local_peer));
    out.push_str(", remote: [");
    for rp in pg.remote_peers.iter() {
        out.push_str(&concise_peer(rp));
        out.push_str(", ");
    }
    out.push(']');
    out
}

pub fn print_cdoc(cbor: &serde_cbor::Value) -> Result<(), io::Error> {
    serde_json::to_writer_pretty(std::io::stdout(), cbor)?;
    Ok(())
}

pub fn resolve_local_ip(config_addr: Option<String>) -> String {
    let default_if_addr = local_ip().unwrap();
    info!(
        "==> Local IP address: {}, config: {:?}",
        default_if_addr, config_addr
    );
    config_addr.unwrap_or(default_if_addr.to_string())
}

#[allow(dead_code)]
pub fn show_local_ips() {
    let network_interfaces = list_afinet_netifas().unwrap();

    for (name, ip) in network_interfaces.iter() {
        info!("{}:\t{:?}", name, ip);
    }
}

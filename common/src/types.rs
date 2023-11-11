use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::Duration;

pub use crate::default;
pub type PeerId = String;

pub fn random_peer_id() -> PeerId {
    format!("{:x}", rand::random::<u64>())
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Peer {
    pub peer_id: PeerId,
    pub peer_ip_addr: std::net::IpAddr,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Heartbeat {
    pub sender: Peer,
    pub sent_at_msec: u64,
}

// Ignore timestamps when storing in a set
impl PartialEq for Heartbeat {
    fn eq(&self, other: &Self) -> bool {
        self.sender == other.sender
    }
}

impl Eq for Heartbeat {}

impl Hash for Heartbeat {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.sender.hash(state);
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HeartbeatsDoc {
    // A vec of latest heartbeat record for each peer
    pub beats: HashMap<PeerId, Heartbeat>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CoordinatorInfo {
    pub heartbeat_collection_name: String,
    pub heartbeat_interval_sec: u32,
    pub execution_plan: Option<ExecutionPlan>,
}


#[derive(Serialize, Deserialize, Debug)]
pub struct ExecutionPlan {
    pub start_time: u64,
    pub test_duration_sec: u32,
    pub report_collection_name: String,
    pub peer_collection_name: String,
    pub min_msg_delay_msec: u32,
    pub max_msg_delay_msec: u32,
    pub peers: Vec<Peer>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LatencyStats {
    // TODO histogram
    num_events: u64,
    min_latency_usec: u64,
    max_latency_usec: u64,
    avg_latency_usec: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AvailabilityStats {
    start_time_usec: u64,
    end_time_usec: u64,
    down_time: Duration,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PeerExecutionReport {
    // After losing connection, how long until no events are older than max_msg_delay?
    resync_latency: LatencyStats,
    message_latency: LatencyStats,
    db_availability: AvailabilityStats,
}

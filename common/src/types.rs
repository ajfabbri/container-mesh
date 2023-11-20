use serde_derive::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::time::Duration;

pub use crate::default;
pub type PeerId = String;

pub fn random_peer_id(prefix: Option<&str>) -> PeerId {
    let pre;
    if prefix.is_none() {
        pre = String::new();
    } else {
        pre = format!("{}_", prefix.unwrap());
    }
    format!("{}{:x}", pre, rand::random::<u64>())
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
#[serde(tag = "type")]
pub enum PeerState {
    Init,
    Running,
    Reporting,
    Shutdown,
}

impl Display for PeerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PeerState::Init => "Init",
            PeerState::Running => "Running",
            PeerState::Reporting => "Reporting",
            PeerState::Shutdown => "Shutdown",
        };
        write!(f, "{}", s)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash, PartialEq, Eq)]
pub struct Peer {
    pub peer_id: PeerId,
    pub peer_ip_addr: String,
    pub state: PeerState,
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ExecutionPlan {
    pub start_time: u64,
    pub test_duration_sec: u32,
    pub report_collection_name: String,
    pub peer_collection_name: String,
    pub min_msg_delay_msec: u32,
    pub max_msg_delay_msec: u32,
    pub peers: Vec<Peer>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LatencyStats {
    // TODO histogram
    pub num_events: u64,
    pub min_latency_usec: u64,
    pub max_latency_usec: u64,
    pub avg_latency_usec: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AvailabilityStats {
    pub start_time_usec: u64,
    pub end_time_usec: u64,
    pub down_time: Duration,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PeerReport {
    // After losing connection, how long until no events are older than max_msg_delay?
    pub resync_latency: LatencyStats,
    pub message_latency: LatencyStats,
    pub db_availability: AvailabilityStats,
}

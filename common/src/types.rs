use clap::ValueEnum;
use dittolive_ditto::prelude::DocumentId;
use serde_derive::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::time::Duration;

pub use crate::default;

pub type PeerId = String;

// Keep serialized types here up to date with ts/src/types.ts
// TODO codegen

pub fn random_peer_id(prefix: Option<&str>) -> PeerId {
    let pre;
    if prefix.is_none() {
        pre = String::new();
    } else {
        pre = format!("{}_", prefix.unwrap());
    }
    format!("{}{:x}", pre, rand::random::<u64>())
}

// Assumes peer_id starts with "peer<num>_"
pub fn short_peer_id(peer_id: &PeerId) -> String {
    short_peer_str(peer_id)
}

fn short_peer_str(peer_name: &str) -> String {
    let num = peer_name.strip_prefix("peer");
    match num {
        Some(n) => n
            .split_once('_')
            .unwrap_or((n.split_at(4).0, ""))
            .0
            .to_string(),
        None => {
            let mut pre = peer_name.to_string();
            pre.truncate(4);
            pre
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerGraph {
    #[serde(flatten)]
    pub nmap: HashMap<PeerId, HashSet<PeerId>>,
}

impl PeerGraph {
    pub fn new() -> Self {
        Self {
            nmap: HashMap::new(),
        }
    }

    // Treat each directed edge as an undirected edge, and return the set of neighbors for vertex
    pub fn undirected_links(&self, vertex: &PeerId) -> Option<HashSet<PeerId>> {
        let v = self.nmap.get(vertex)?;
        let mut peers = v.clone();
        for (u, neighbors) in &self.nmap {
            if neighbors.contains(vertex) {
                peers.insert(u.clone());
            }
        }
        Some(peers)
    }

    pub fn to_dot(&self) -> String {
        let mut dot = String::new();
        dot.push_str("digraph G {\n");
        for (u, v) in &self.nmap {
            for v in v {
                dot.push_str(&format!("  {} -> {};\n", short_peer_str(u), short_peer_str(v)));
            }
        }
        dot.push_str("}\n");
        dot
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
pub enum GraphType {
    Complete,
    SpanningTree,
    LAModel,
}

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum PeerState {
    Init,       // Alive, reporting to coord.
    Ready,      // Have test plan, ready to execute
    Running,    // Executing
    Reporting,  // Finished test, outputting results
    Shutdown,   // Done, exiting
}

impl Display for PeerState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PeerState::Init => "Init",
            PeerState::Ready => "Ready",
            PeerState::Running => "Running",
            PeerState::Reporting => "Reporting",
            PeerState::Shutdown => "Shutdown",
        };
        write!(f, "{}", s)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Peer {
    pub peer_id: PeerId,
    pub peer_ip_addr: String,
    pub peer_port: u16,
    pub state: PeerState,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Heartbeat {
    pub sender: Peer,
    pub sent_at_msec: u64,
}

impl PartialEq for Peer {
    fn eq(&self, other: &Self) -> bool {
        self.peer_id == other.peer_id
    }
}
impl Eq for Peer {}

impl Hash for Peer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.peer_id.hash(state);
    }
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
    pub peer_doc_id: DocumentId,
    pub min_msg_delay_msec: u32,
    pub max_msg_delay_msec: u32,
    pub peers: Vec<Peer>,
    pub connections: PeerGraph,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PeerRecord {
    pub timestamp: u64,
    pub data: String,
}

// Bounded-size log of peer records
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PeerLog {
    pub log: HashMap<String, PeerRecord>,
}

impl PeerLog {
    pub fn new() -> Self {
        Self {
            log: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PeerDoc {
    pub _id: DocumentId,
    pub logs: HashMap<PeerId, PeerLog>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LatencyStats {
    // TODO histogram
    pub num_events: u64,
    pub min_msec: u64,
    pub max_msec: u64,
    pub avg_msec: u64,
    pub distinct_peers: usize,
}

impl LatencyStats {
    pub fn new() -> Self {
        Self {
            num_events: 0,
            min_msec: u64::MAX,
            max_msec: 0,
            avg_msec: 0,
            distinct_peers: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AvailabilityStats {
    pub start_time_msec: u64,
    pub end_time_msec: u64,
    pub down_time: Duration,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PeerReport {
    // After losing connection, how long until no events are older than max_msg_delay?
    // TODO pub resync_latency: LatencyStats,
    pub message_latency: LatencyStats,
    // TODO pub db_availability: AvailabilityStats,
    pub records_produced: u64,
}

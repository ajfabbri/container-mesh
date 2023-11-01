use serde_derive::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug)]
pub struct Peer {
    peer_id: u64,
    peer_ip_addr: std::net::IpAddr,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExecutionPlan {
    start_time: u64,
    test_duration_sec: u32,
    report_collection_name: String,
    peer_collection_name: String,
    heartbeat_collection_name: String,
    heartbeat_interval_sec: u32,
    min_msg_delay_msec: u32,
    max_msg_delay_msec: u32,
    peers: Vec<Peer>,
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
